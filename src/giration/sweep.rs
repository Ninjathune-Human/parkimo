//! Traces des roues, enveloppe balayée et conflits avec les obstacles de la scène.

use super::epure::Sample;
use super::kinematics::{advance, wheels, Pose, State};
use super::scene::{segment_hit, Scene};
use super::vehicle::Vehicle;
use crate::geom::v2;
use crate::model::P2;
use crate::region::to_poly;
use geo::algorithm::bool_ops::unary_union;
use geo::{ConvexHull, Polygon};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Part {
    /// Roue : 0 avant gauche, 1 avant droite, 2 arrière gauche, 3 arrière droite.
    Wheel(usize),
    /// Carrosserie (garde au sol comparée à la hauteur de l'obstacle).
    Body,
    /// Marge de sécurité autour de la carrosserie.
    Margin,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum Severity {
    Conflict,
    Warning,
}

/// Un contact, regroupé sur la distance où il dure.
#[derive(Clone, Debug, Serialize)]
pub struct Conflict {
    pub part: Part,
    pub obstacle: usize,
    pub severity: Severity,
    /// Premier point de contact.
    pub point: P2,
    pub s_start: f64,
    pub s_end: f64,
    pub section: usize,
}

/// Deux contacts du même élément avec le même obstacle séparés de moins de cette distance
/// forment un seul événement (m).
const GAP: f64 = 0.5;
/// Espacement des empreintes réunies pour l'enveloppe (m).
const ENVELOPE_STEP: f64 = 0.2;
/// Calage des sommets de l'enveloppe (m).
const SNAP: f64 = 1e-4;

/// Huit polylignes : pour chaque roue (avant gauche, avant droite, arrière gauche, arrière
/// droite), son bord gauche puis son bord droit.
pub fn tyre_tracks(v: &Vehicle, samples: &[Sample]) -> Vec<Vec<P2>> {
    let mut out: Vec<Vec<P2>> = (0..8).map(|_| Vec::with_capacity(samples.len())).collect();
    let e = v.tyre_width / 2.0;
    for s in samples {
        for (w, wheel) in wheels(v, &s.state).iter().enumerate() {
            let (c, h) = (wheel.centre, wheel.heading);
            out[2 * w].push([c[0] - e * h.sin(), c[1] + e * h.cos()]);
            out[2 * w + 1].push([c[0] + e * h.sin(), c[1] - e * h.cos()]);
        }
    }
    out
}

/// Empreinte de caisse agrandie de `margin` : avant gauche, avant droit, arrière droit, arrière gauche.
pub fn footprint(v: &Vehicle, p: &Pose, margin: f64) -> [P2; 4] {
    let (c, n) = ((p.heading.cos(), p.heading.sin()), (-p.heading.sin(), p.heading.cos()));
    let half = v.width / 2.0 + margin;
    let at = |fwd: f64, left: f64| [p.x + fwd * c.0 + left * n.0, p.y + fwd * c.1 + left * n.1];
    let (front, rear) = (v.wheelbase + v.front_overhang + margin, -v.rear_overhang - margin);
    [at(front, half), at(front, -half), at(rear, -half), at(rear, half)]
}

/// Enveloppe balayée par la caisse (agrandie de `margin`) : polygones, anneau extérieur puis trous.
pub fn envelope(v: &Vehicle, samples: &[Sample], margin: f64) -> Vec<Vec<Vec<P2>>> {
    // Enveloppe convexe de deux empreintes successives : elle couvre les positions
    // intermédiaires, au millimètre près pour un pas de 0,2 m.
    let mut kept: Vec<[P2; 4]> = Vec::new();
    let mut last = f64::NEG_INFINITY;
    for (i, s) in samples.iter().enumerate() {
        if s.s - last >= ENVELOPE_STEP || i + 1 == samples.len() {
            kept.push(footprint(v, &s.state.pose, margin));
            last = s.s;
        }
    }
    let mut polys: Vec<Polygon<f64>> = Vec::new();
    if kept.len() == 1 {
        polys.push(to_poly(&kept[0].iter().map(|p| v2(p[0], p[1])).collect::<Vec<_>>()));
    }
    for w in kept.windows(2) {
        // Coordonnées calées au dixième de millimètre : sans cela, des arêtes presque colinéaires
        // (ligne droite à cap oblique) font exploser la réunion booléenne.
        let snap = |x: f64| (x / SNAP).round() * SNAP;
        let pts: Vec<geo::Point<f64>> = w.iter().flatten().map(|p| geo::Point::new(snap(p[0]), snap(p[1]))).collect();
        polys.push(geo::MultiPoint::new(pts).convex_hull());
    }
    if polys.is_empty() {
        return Vec::new();
    }
    unary_union(polys.iter())
        .0
        .iter()
        .map(|p| {
            std::iter::once(p.exterior())
                .chain(p.interiors())
                .map(|r| r.0.iter().map(|c| [c.x, c.y]).collect())
                .collect()
        })
        .collect()
}

fn inside_quad(p: P2, q: &[P2; 4]) -> bool {
    (0..4).all(|i| {
        let (a, b) = (q[i], q[(i + 1) % 4]);
        (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]) <= 0.0
    })
}

/// Premier point où le segment [c, d] touche le quadrilatère convexe `q` (sens horaire).
fn touches_quad(c: P2, d: P2, q: &[P2; 4]) -> Option<P2> {
    if inside_quad(c, q) {
        return Some(c);
    }
    if inside_quad(d, q) {
        return Some(d);
    }
    (0..4).find_map(|i| segment_hit(q[i], q[(i + 1) % 4], c, d).map(|h| h.1))
}

fn bbox(q: &[P2; 4]) -> (P2, P2) {
    let xs = q.iter().map(|p| p[0]);
    let ys = q.iter().map(|p| p[1]);
    (
        [xs.clone().fold(f64::INFINITY, f64::min), ys.clone().fold(f64::INFINITY, f64::min)],
        [xs.fold(f64::NEG_INFINITY, f64::max), ys.fold(f64::NEG_INFINITY, f64::max)],
    )
}

/// Premier obstacle plus haut que la garde au sol touché par la caisse agrandie de `grow`.
pub(crate) fn body_contact(v: &Vehicle, scene: &Scene, pose: &Pose, grow: f64) -> Option<(usize, P2)> {
    let body = footprint(v, pose, grow);
    let (lo, hi) = bbox(&body);
    scene.near(lo, hi).into_iter().find_map(|(o, k)| {
        let ob = &scene.obstacles[o];
        if ob.height < v.ground_clearance {
            return None;
        }
        touches_quad(ob.pts[k], ob.pts[k + 1], &body).map(|p| (o, p))
    })
}

/// Un pneu touche-t-il une bordure plus basse que la garde au sol (que la caisse survole) ?
pub(crate) fn tyre_on_low_kerb(v: &Vehicle, scene: &Scene, pose: &Pose) -> bool {
    let e = v.tyre_width / 2.0;
    let state = State { pose: *pose, steer: 0.0 };
    wheels(v, &state).iter().any(|w| {
        let (c, h) = (w.centre, w.heading);
        let (u, n) = ([h.cos() * 0.35, h.sin() * 0.35], [-h.sin() * e, h.cos() * e]);
        let tyre = [
            [c[0] + u[0] + n[0], c[1] + u[1] + n[1]],
            [c[0] + u[0] - n[0], c[1] + u[1] - n[1]],
            [c[0] - u[0] - n[0], c[1] - u[1] - n[1]],
            [c[0] - u[0] + n[0], c[1] - u[1] + n[1]],
        ];
        let (lo, hi) = bbox(&tyre);
        scene.near(lo, hi).into_iter().any(|(o, k)| {
            let ob = &scene.obstacles[o];
            ob.height > 0.0 && ob.height < v.ground_clearance && touches_quad(ob.pts[k], ob.pts[k + 1], &tyre).is_some()
        })
    })
}

/// Contact de la caisse, agrandie de `grow`, pendant le mouvement de `a` sur l'arc exact de
/// longueur signée `ds` (angle de roues `a.steer`). Chaque point de la caisse décrit un arc dont la
/// corde relie ses positions au début et à la fin : la corde est dans l'enveloppe convexe des deux
/// empreintes, et l'arc s'en écarte au plus de la flèche ρ (1 − cos(Δθ/2)), ρ étant la distance du
/// point le plus éloigné au centre de rotation. L'enveloppe convexe des deux empreintes agrandies
/// de cette flèche contient donc tout le balayage : contrôle exact par excès, sans pas minimal.
pub(crate) fn swept_contact(v: &Vehicle, scene: &Scene, a: &State, ds: f64, grow: f64) -> Option<(usize, P2)> {
    let kappa = a.steer.tan() / v.wheelbase;
    let b = advance(v, *a, ds);
    let sagitta = if kappa.abs() < 1e-12 {
        0.0
    } else {
        let rho = (1.0 / kappa.abs() + v.width / 2.0 + grow).hypot((v.wheelbase + v.front_overhang).max(v.rear_overhang) + grow);
        rho * (1.0 - (kappa * ds / 2.0).cos())
    };
    let pad = grow + sagitta;
    let (fa, fb) = (footprint(v, &a.pose, pad), footprint(v, &b.pose, pad));
    let hull = convex_hull(&[fa[0], fa[1], fa[2], fa[3], fb[0], fb[1], fb[2], fb[3]]);
    let xs = hull.iter().map(|p| p[0]);
    let ys = hull.iter().map(|p| p[1]);
    let lo = [xs.clone().fold(f64::INFINITY, f64::min), ys.clone().fold(f64::INFINITY, f64::min)];
    let hi = [xs.fold(f64::NEG_INFINITY, f64::max), ys.fold(f64::NEG_INFINITY, f64::max)];
    scene.near(lo, hi).into_iter().find_map(|(o, k)| {
        let ob = &scene.obstacles[o];
        if ob.height < v.ground_clearance {
            return None;
        }
        touches_convex(ob.pts[k], ob.pts[k + 1], &hull).map(|p| (o, p))
    })
}

/// Enveloppe convexe (chaîne monotone) de huit points, sens trigonométrique.
fn convex_hull(pts: &[P2; 8]) -> Vec<P2> {
    let mut p = *pts;
    p.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    let cross = |o: P2, a: P2, b: P2| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let mut h: Vec<P2> = Vec::with_capacity(16);
    for pass in 0..2 {
        let start = h.len();
        for j in 0..8 {
            let q = if pass == 0 { p[j] } else { p[7 - j] };
            while h.len() >= start + 2 && cross(h[h.len() - 2], h[h.len() - 1], q) <= 0.0 {
                h.pop();
            }
            h.push(q);
        }
        h.pop();
    }
    h
}

/// Premier point où le segment [c, d] touche le polygone convexe `poly` (sens trigonométrique).
fn touches_convex(c: P2, d: P2, poly: &[P2]) -> Option<P2> {
    let inside = |p: P2| {
        (0..poly.len()).all(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]) >= 0.0
        })
    };
    if inside(c) {
        return Some(c);
    }
    if inside(d) {
        return Some(d);
    }
    (0..poly.len()).find_map(|i| segment_hit(poly[i], poly[(i + 1) % poly.len()], c, d).map(|h| h.1))
}

/// Distance de la caisse aux obstacles plus hauts que la garde au sol, plafonnée à `cap` (m).
pub(crate) fn clearance(v: &Vehicle, scene: &Scene, pose: &Pose, cap: f64) -> f64 {
    let body = footprint(v, pose, 0.0);
    let (lo, hi) = bbox(&footprint(v, pose, cap));
    let seg = |p: P2, a: P2, b: P2| {
        let d = [b[0] - a[0], b[1] - a[1]];
        let t = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1]).max(1e-18)).clamp(0.0, 1.0);
        (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
    };
    let mut best = cap;
    for (o, k) in scene.near(lo, hi) {
        let ob = &scene.obstacles[o];
        if ob.height < v.ground_clearance {
            continue;
        }
        let (c, d) = (ob.pts[k], ob.pts[k + 1]);
        if touches_quad(c, d, &body).is_some() {
            return 0.0;
        }
        for i in 0..4 {
            let (a, b) = (body[i], body[(i + 1) % 4]);
            best = best.min(seg(c, a, b)).min(seg(d, a, b)).min(seg(a, c, d)).min(seg(b, c, d));
        }
    }
    best
}

/// Sens et longueur signée du mouvement d'un échantillon au suivant, et angle de roues pendant ce
/// mouvement (le suiveur braque à mi-pas).
fn step_between(a: &Sample, b: &Sample) -> (State, f64) {
    let (h, dx, dy) = (a.state.pose.heading, b.state.pose.x - a.state.pose.x, b.state.pose.y - a.state.pose.y);
    let sign = if dx * h.cos() + dy * h.sin() < 0.0 { -1.0 } else { 1.0 };
    (State { pose: a.state.pose, steer: 0.5 * (a.state.steer + b.state.steer) }, sign * (b.s - a.s))
}

struct Events {
    list: Vec<Conflict>,
    open: HashMap<(Part, usize), usize>,
}

impl Events {
    fn record(&mut self, part: Part, obstacle: usize, point: P2, s: f64, section: usize) {
        if let Some(&i) = self.open.get(&(part, obstacle)) {
            if s - self.list[i].s_end <= GAP {
                self.list[i].s_end = s;
                return;
            }
        }
        let severity = if part == Part::Margin { Severity::Warning } else { Severity::Conflict };
        self.open.insert((part, obstacle), self.list.len());
        self.list.push(Conflict { part, obstacle, severity, point, s_start: s, s_end: s, section });
    }
}

/// Conflits le long d'une trajectoire : roues qui franchissent un obstacle, caisse au contact
/// d'un obstacle plus haut que sa garde au sol, marge entamée.
pub fn conflicts(v: &Vehicle, scene: &Scene, samples: &[Sample], margin: f64) -> Vec<Conflict> {
    let mut ev = Events { list: Vec::new(), open: HashMap::new() };
    let tracks = tyre_tracks(v, samples);
    for (i, s) in samples.iter().enumerate() {
        if i > 0 {
            for (t, track) in tracks.iter().enumerate() {
                for hit in scene.crossings(track[i - 1], track[i]) {
                    if scene.obstacles[hit.obstacle].height > 0.0 {
                        ev.record(Part::Wheel(t / 2), hit.obstacle, hit.point, s.s, s.section);
                    }
                }
            }
        }
        let body = footprint(v, &s.state.pose, 0.0);
        let outer = footprint(v, &s.state.pose, margin);
        let (lo, hi) = bbox(&outer);
        let mut touched: Vec<usize> = Vec::new();
        let candidates = scene.near(lo, hi);
        for &(o, k) in &candidates {
            let ob = &scene.obstacles[o];
            if ob.height < v.ground_clearance {
                continue;
            }
            if let Some(p) = touches_quad(ob.pts[k], ob.pts[k + 1], &body) {
                ev.record(Part::Body, o, p, s.s, s.section);
                touched.push(o);
            }
        }
        // Entre l'échantillon précédent et celui-ci : contrôle continu de la caisse. La marge, simple
        // alerte, est contrôlée aux échantillons.
        if i > 0 {
            let (from, ds) = step_between(&samples[i - 1], s);
            if ds != 0.0 {
                if let Some((o, p)) = swept_contact(v, scene, &from, ds, 0.0) {
                    if !touched.contains(&o) {
                        ev.record(Part::Body, o, p, s.s, s.section);
                        touched.push(o);
                    }
                }
            }
        }
        if margin > 0.0 {
            for &(o, k) in &candidates {
                let ob = &scene.obstacles[o];
                if ob.height < v.ground_clearance || touched.contains(&o) {
                    continue;
                }
                if let Some(p) = touches_quad(ob.pts[k], ob.pts[k + 1], &outer) {
                    ev.record(Part::Margin, o, p, s.s, s.section);
                }
            }
        }
    }
    let mut list = ev.list;
    list.sort_by(|a, b| a.s_start.total_cmp(&b.s_start));
    list
}
