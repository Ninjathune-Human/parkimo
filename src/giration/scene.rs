//! Scène : obstacles typés que les roues et la carrosserie ne doivent pas toucher.
//!
//! Bordures : contour de la surface roulable (chaussée, allées, accès, places), là où il est
//! intérieur à la parcelle ; bordure d'espace planté quand la pleine terre la longe (servitudes
//! comprises), de trottoir le long d'un cheminement piéton ou d'un bâtiment. Murs : bâtiments. Limite : contour de
//! parcelle, sauf les bouches des accès.

use crate::geom::{v2, Region, V2};
use crate::model::{Layout, Site, P2};
use crate::region::{erode, grow, to_poly};
use geo::algorithm::bool_ops::unary_union;
use geo::{BooleanOps, Contains, MultiPolygon, Point, Polygon};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ObstacleKind {
    /// Bordure de trottoir ou de cheminement piéton.
    Kerb,
    /// Bordure d'espace planté (pleine terre, îlot, noue) : la caisse déborde au-dessus, pas les pneus.
    Planted,
    /// Façade de bâtiment.
    Wall,
    /// Limite de parcelle (clôture, mur ou haie : infranchissable).
    Limit,
    /// Véhicule garé dans une place voisine.
    Car,
}

#[derive(Clone, Debug)]
pub struct Obstacle {
    pub kind: ObstacleKind,
    /// Hauteur vue (m) ; infinie pour un mur ou une limite.
    pub height: f64,
    /// Polyligne ; un anneau répète son premier point à la fin.
    pub pts: Vec<P2>,
}

/// Profil de bordure : son nom et sa hauteur vue au-dessus de la chaussée.
#[derive(Clone, Debug, Deserialize)]
pub struct KerbProfile {
    pub name: String,
    pub visible_height: f64,
}

impl KerbProfile {
    /// T2, hauteur vue 0,16 m (trottoir courant, prescriptions de la Métropole Aix-Marseille-Provence).
    pub fn t2() -> KerbProfile {
        KerbProfile { name: "T2".into(), visible_height: 0.16 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crossing {
    pub obstacle: usize,
    pub point: P2,
}

const CELL: f64 = 2.0;
/// Fermeture morphologique de la surface roulable : comble les joints de quelques centimètres.
const CLOSE: f64 = 0.05;
/// Distance au milieu d'une bordure du point qui dit ce qu'elle longe (m).
const SIDE: f64 = 0.15;
/// Pas d'examen de ce que longe une bordure : un côté de chaussée peut longer un trottoir puis la
/// pleine terre (m).
const PIECE: f64 = 0.5;

pub struct Scene {
    pub obstacles: Vec<Obstacle>,
    /// Segments par case de CELL m : tableau direct de nx × ny cases à partir de la case (x0, y0) ;
    /// pour chaque segment, sa case de coin bas gauche.
    grid: Vec<Vec<(u32, u32, i32, i32)>>,
    x0: i32,
    y0: i32,
    nx: i32,
    ny: i32,
}

fn quad_poly(q: &[P2; 4]) -> Polygon<f64> {
    to_poly(&q.iter().map(|p| v2(p[0], p[1])).collect::<Vec<_>>())
}

fn ring_pts(ls: &geo::LineString<f64>) -> Vec<P2> {
    ls.0.iter().map(|c| [c.x, c.y]).collect()
}

fn dist_to_segment(p: V2, a: V2, b: V2) -> f64 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.dot(ab).max(1e-18)).clamp(0.0, 1.0);
    (a + ab * t - p).len()
}

/// Intervalles du segment [a, b] (paramètre 0..1) contenus dans le quadrilatère convexe `q`.
fn clip_in_quad(a: V2, b: V2, q: &[V2; 4]) -> Option<(f64, f64)> {
    let area: f64 = (0..4).map(|i| q[i].cross(q[(i + 1) % 4])).sum();
    let s = area.signum();
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    let d = b - a;
    for i in 0..4 {
        let (e0, e1) = (q[i], q[(i + 1) % 4]);
        let e = e1 - e0;
        // Intérieur : s · cross(e, p − e0) ≥ −ε.
        let num = s * e.cross(a - e0) + 0.01 * e.len();
        let den = s * e.cross(d);
        if den.abs() < 1e-12 {
            if num < 0.0 {
                return None;
            }
        } else {
            let t = -num / den;
            if den > 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    (t1 > t0).then_some((t0, t1))
}

pub(crate) fn segment_hit(a: P2, b: P2, c: P2, d: P2) -> Option<(f64, P2)> {
    let r = [b[0] - a[0], b[1] - a[1]];
    let s = [d[0] - c[0], d[1] - c[1]];
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1e-14 {
        return None;
    }
    let q = [c[0] - a[0], c[1] - a[1]];
    let t = (q[0] * s[1] - q[1] * s[0]) / den;
    let u = (q[0] * r[1] - q[1] * r[0]) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u))
        .then(|| (t, [a[0] + t * r[0], a[1] + t * r[1]]))
}

impl Scene {
    pub fn new(obstacles: Vec<Obstacle>) -> Scene {
        let pts = obstacles.iter().flat_map(|o| o.pts.iter());
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for p in pts {
            let (x, y) = ((p[0] / CELL).floor() as i32, (p[1] / CELL).floor() as i32);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
        let (nx, ny) = if x0 > x1 { (0, 0) } else { (x1 - x0 + 1, y1 - y0 + 1) };
        let mut scene = Scene { obstacles, grid: vec![Vec::new(); (nx * ny) as usize], x0, y0, nx, ny };
        for oi in 0..scene.obstacles.len() {
            for si in 0..scene.obstacles[oi].pts.len().saturating_sub(1) {
                let (a, b) = (scene.obstacles[oi].pts[si], scene.obstacles[oi].pts[si + 1]);
                let (cx, cy) = ((a[0].min(b[0]) / CELL).floor() as i32, (a[1].min(b[1]) / CELL).floor() as i32);
                for (x, y) in cells(a, b) {
                    if let Some(i) = scene.index(x, y) {
                        scene.grid[i].push((oi as u32, si as u32, cx, cy));
                    }
                }
            }
        }
        scene
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        let (i, j) = (x - self.x0, y - self.y0);
        (i >= 0 && j >= 0 && i < self.nx && j < self.ny).then(|| (i * self.ny + j) as usize)
    }

    pub fn from_layout(site: &Site, lay: &Layout, kerb: &KerbProfile) -> Scene {
        let parcel: Vec<V2> = site.parcel.iter().map(|p| v2(p[0], p[1])).collect();
        let inside = Region::new(vec![parcel.clone()]);
        let near_limit = |p: V2| {
            (0..parcel.len())
                .map(|i| dist_to_segment(p, parcel[i], parcel[(i + 1) % parcel.len()]))
                .fold(f64::INFINITY, f64::min)
                < 0.02
        };

        // Surface roulable : chaussée (anneaux pair-impair), allées transversales, accès, places.
        let mut paved = MultiPolygon::new(vec![]);
        for r in &lay.paved {
            let ring = to_poly(&r.iter().map(|p| v2(p[0], p[1])).collect::<Vec<_>>());
            paved = paved.xor(&MultiPolygon::new(vec![ring]));
        }
        let mut polys: Vec<Polygon<f64>> = paved.0;
        let quads = lay.cross.iter().chain(&lay.entries).chain(&lay.pads);
        polys.extend(quads.map(quad_poly));
        polys.extend(lay.stalls.iter().map(|s| quad_poly(&s.q)));
        // Îlots des rangées : posés sur les fonds de rangées, bordés de tous côtés (les noues sont déjà hors chaussée).
        let islands: Vec<Polygon<f64>> = lay.islands.iter().map(quad_poly).collect();
        let drivable = erode(&grow(&unary_union(polys.iter()), CLOSE), CLOSE).difference(&unary_union(islands.iter()));

        // Ce qui n'est pas de la pleine terre hors de la surface roulable : bâtiments et cheminements
        // piétons. Les servitudes restent de la pleine terre, comme dans le calcul des espaces verts.
        let hard: Vec<Region> = site
            .buildings
            .iter()
            .map(|b| b.poly.clone())
            .chain(lay.walks.iter().map(|q| q.to_vec()))
            .map(|r| Region::new(vec![r.iter().map(|p| v2(p[0], p[1])).collect()]))
            .collect();
        // Trottoir si le côté hors de la surface roulable tombe sur l'un d'eux ; sinon pleine terre.
        // Aux angles vifs et aux filets de quelques millimètres, les deux côtés sont examinés.
        let kind_of = |a: P2, b: P2| {
            let (m, d) = (v2(0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])), v2(b[0] - a[0], b[1] - a[1]));
            let n = v2(-d.y, d.x) * (SIDE / d.len().max(1e-12));
            let hard_side = |p: V2| !drivable.contains(&Point::new(p.x, p.y)) && hard.iter().any(|r| r.contains(p));
            if hard_side(m + n) || hard_side(m - n) {
                ObstacleKind::Kerb
            } else {
                ObstacleKind::Planted
            }
        };
        let height = kerb.visible_height;
        let flush = |cur: &mut Vec<P2>, kind: ObstacleKind, obstacles: &mut Vec<Obstacle>| {
            if cur.len() > 1 {
                obstacles.push(Obstacle { kind, height, pts: std::mem::take(cur) });
            } else {
                cur.clear();
            }
        };

        let mut obstacles = Vec::new();
        for poly in &drivable.0 {
            for ring in std::iter::once(poly.exterior()).chain(poly.interiors()) {
                let pts = ring_pts(ring);
                if crate::geom::signed_area(&pts.iter().map(|p| v2(p[0], p[1])).collect::<Vec<_>>()).abs() < 0.2 {
                    continue;
                }
                // Découpe en polylignes : on écarte ce qui longe la limite ou sort de la parcelle, et
                // l'on coupe où la bordure change de nature.
                let mut cur: Vec<P2> = Vec::new();
                let mut kind = ObstacleKind::Kerb;
                for w in pts.windows(2) {
                    let m = v2(0.5 * (w[0][0] + w[1][0]), 0.5 * (w[0][1] + w[1][1]));
                    if inside.contains(m) && !near_limit(m) {
                        // Morceaux de PIECE au plus ; un sommet seulement où la nature change.
                        let n = ((w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]) / PIECE).ceil().max(1.0) as usize;
                        let at = |j: usize| {
                            let t = j as f64 / n as f64;
                            [w[0][0] + (w[1][0] - w[0][0]) * t, w[0][1] + (w[1][1] - w[0][1]) * t]
                        };
                        let ks: Vec<ObstacleKind> = (0..n).map(|j| kind_of(at(j), at(j + 1))).collect();
                        for j in 0..n {
                            if ks[j] != kind {
                                flush(&mut cur, kind, &mut obstacles);
                                kind = ks[j];
                            }
                            if cur.is_empty() {
                                cur.push(at(j));
                            }
                            if j + 1 == n {
                                cur.push(w[1]);
                            } else if ks[j + 1] != kind {
                                cur.push(at(j + 1));
                            }
                        }
                    } else {
                        flush(&mut cur, kind, &mut obstacles);
                    }
                }
                flush(&mut cur, kind, &mut obstacles);
            }
        }

        for b in &site.buildings {
            let mut pts = b.poly.clone();
            if let Some(&first) = pts.first() {
                pts.push(first);
            }
            obstacles.push(Obstacle { kind: ObstacleKind::Wall, height: f64::INFINITY, pts });
        }

        // Limite de parcelle, bouches d'accès retirées.
        let entries: Vec<[V2; 4]> = lay
            .entries
            .iter()
            .map(|q| [v2(q[0][0], q[0][1]), v2(q[1][0], q[1][1]), v2(q[2][0], q[2][1]), v2(q[3][0], q[3][1])])
            .collect();
        for i in 0..parcel.len() {
            let (a, b) = (parcel[i], parcel[(i + 1) % parcel.len()]);
            let mut cuts: Vec<(f64, f64)> = entries.iter().filter_map(|q| clip_in_quad(a, b, q)).collect();
            cuts.sort_by(|x, y| x.0.total_cmp(&y.0));
            let mut t = 0.0;
            let at = |t: f64| {
                let p = a + (b - a) * t;
                [p.x, p.y]
            };
            for (c0, c1) in cuts {
                if c0 > t + 1e-9 {
                    obstacles.push(Obstacle { kind: ObstacleKind::Limit, height: f64::INFINITY, pts: vec![at(t), at(c0)] });
                }
                t = t.max(c1);
            }
            if t < 1.0 - 1e-9 {
                obstacles.push(Obstacle { kind: ObstacleKind::Limit, height: f64::INFINITY, pts: vec![at(t), at(1.0)] });
            }
        }
        Scene::new(obstacles)
    }

    /// Segments d'obstacles (obstacle, segment) dont les cases touchent le rectangle [lo, hi].
    pub fn near(&self, lo: P2, hi: P2) -> Vec<(usize, usize)> {
        let mut out: Vec<(usize, usize)> = Vec::new();
        let (qx, qy) = ((lo[0] / CELL).floor() as i32, (lo[1] / CELL).floor() as i32);
        for (x, y) in cells(lo, hi) {
            let Some(i) = self.index(x, y) else { continue };
            for &(o, s, cx, cy) in &self.grid[i] {
                // Un segment couvre plusieurs cases : on ne le retient que dans la première case
                // commune au segment et au rectangle, sans tri ni dédoublonnage.
                if (cx.max(qx), cy.max(qy)) == (x, y) {
                    out.push((o as usize, s as usize));
                }
            }
        }
        out
    }

    /// Points où le segment [a, b] coupe un obstacle, du plus proche de `a` au plus lointain.
    pub fn crossings(&self, a: P2, b: P2) -> Vec<Crossing> {
        let mut seen: Vec<(u32, u32)> = Vec::new();
        let mut hits: Vec<(f64, Crossing)> = Vec::new();
        for (x, y) in cells(a, b) {
            if let Some(i) = self.index(x, y) {
                for &(oi, si, _, _) in &self.grid[i] {
                    if seen.contains(&(oi, si)) {
                        continue;
                    }
                    seen.push((oi, si));
                    let o = &self.obstacles[oi as usize];
                    let (c, d) = (o.pts[si as usize], o.pts[si as usize + 1]);
                    if let Some((t, point)) = segment_hit(a, b, c, d) {
                        hits.push((t, Crossing { obstacle: oi as usize, point }));
                    }
                }
            }
        }
        hits.sort_by(|x, y| x.0.total_cmp(&y.0));
        hits.into_iter().map(|h| h.1).collect()
    }
}

fn cells(a: P2, b: P2) -> impl Iterator<Item = (i32, i32)> {
    let (x0, x1) = ((a[0].min(b[0]) / CELL).floor() as i32, (a[0].max(b[0]) / CELL).floor() as i32);
    let (y0, y1) = ((a[1].min(b[1]) / CELL).floor() as i32, (a[1].max(b[1]) / CELL).floor() as i32);
    (x0..=x1).flat_map(move |x| (y0..=y1).map(move |y| (x, y)))
}
