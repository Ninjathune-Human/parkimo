//! Espaces plantés : tout le résiduel de la zone stationnable (angles, têtes de rangée en biais,
//! interstices entre places, îlots d'extrémité et de bordure).
//!
//! Les angles saillants tournés vers la chaussée sont arrondis au rayon `RADIUS` (moins si les côtés
//! sont trop courts) ; la chaussée prend la place de l'arrondi. Un angle contre une place ou un fond
//! de rangée reste vif : une bande plantée entre deux places est rectangulaire.

use crate::geom::{v2, Quad, QuadHash, Region, V2};
use crate::occupation::{polygon_of, polys_of, PolyRings};
use crate::region;
use geo::algorithm::buffer::{BufferStyle, LineJoin};
use geo::{BooleanOps, Buffer, MultiPolygon};
use std::f64::consts::PI;

/// Rayon des arrondis en bordure de chaussée (m).
pub const RADIUS: f64 = 1.0;

pub struct Planted {
    pub polys: Vec<PolyRings>,
    /// Parties retirées par les arrondis, rendues à la chaussée.
    pub cut: MultiPolygon<f64>,
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    /// Place de stationnement ou fond de rangée.
    Stall,
    /// Chaussée : voirie, allée transversale, accès.
    Road,
    /// Limite de la zone stationnable (recul, bâti, servitude, espace vert imposé).
    Edge,
    /// Cheminement devant l'entrée d'un bâtiment : jamais d'arrondi de giration à son contact.
    Walk,
}

/// `b` : zone stationnable ; `paved` : voirie ; `stalls` : places et fonds de rangées ;
/// `road` : allées transversales et accès ; `clean` : élimination des filets dégénérés (au
/// relâchement seulement, pendant un glisser le calcul direct reste léger).
pub fn build(
    b: &MultiPolygon<f64>,
    paved: &MultiPolygon<f64>,
    stalls: &[Quad],
    road: &[Quad],
    walks: &[Quad],
    clean: bool,
) -> Planted {
    let mut hard: Vec<Quad> = stalls.to_vec();
    hard.extend_from_slice(road);
    hard.extend_from_slice(walks);
    let rest = b.difference(&crate::mp_of_quads(&hard).union(paved));
    // Filets dégénérés : quand les rangées sont légèrement en biais, la chaussée et le front des
    // places ne coïncident qu'au millimètre près et laissent des filets d'épaisseur nulle qui
    // soudent les îlots entre eux. Érosion puis dilatation de 3 cm, à angles vifs.
    // Miter(angle) : seuls les angles plus aigus que 6° sont abattus.
    let miter = |d: f64| BufferStyle::new(d).line_join(LineJoin::Miter(0.1));
    let rest = if clean {
        rest.buffer_with_style(miter(-0.03))
            .buffer_with_style(miter(0.03))
    } else {
        rest
    };
    // Miettes de moins de 1 m², filets de moins de 30 cm d'épaisseur moyenne (2 × aire / périmètre).
    let raw: Vec<PolyRings> = polys_of(&rest, 1.0)
        .0
        .into_iter()
        .filter(|poly| {
            let ext = pts(&poly[0]);
            let per: f64 = (0..ext.len())
                .map(|i| (ext[(i + 1) % ext.len()] - ext[i]).len())
                .sum();
            2.0 * crate::geom::signed_area(&ext).abs() / per.max(1e-9) >= 0.3
        })
        .collect();

    let mut hash = QuadHash::new(6.0);
    for q in stalls {
        hash.insert(*q);
    }
    let roads = Region::new(region::rings_of(paved, 0.0));
    let side = |a: V2, b: V2| -> Side {
        // Point juste à l'extérieur du milieu du côté (l'intérieur est à gauche).
        let d = (b - a).norm();
        let p = (a + b) * 0.5 + v2(d.y, -d.x) * 0.1;
        let probe = Quad([p, p + v2(0.01, 0.0), p + v2(0.01, 0.01), p + v2(0.0, 0.01)]);
        if hash.hits(&probe) {
            Side::Stall
        } else if walks.iter().any(|q| q.contains(p)) {
            Side::Walk
        } else if roads.contains(p) || road.iter().any(|q| q.contains(p)) {
            Side::Road
        } else {
            Side::Edge
        }
    };

    let mut polys = Vec::with_capacity(raw.len());
    let (mut before, mut after) = (Vec::new(), Vec::new());
    for poly in &raw {
        let rings: Vec<Vec<V2>> = poly.iter().map(|r| tidy(&pts(r), 0.2)).collect();
        if rings[0].len() < 3 {
            continue;
        }
        let rounded: Vec<Vec<V2>> = rings.iter().map(|r| round_ring(r, RADIUS, &side)).collect();
        let out: PolyRings = rounded
            .iter()
            .map(|r| r.iter().map(|p| [r2(p.x), r2(p.y)]).collect())
            .collect();
        if let (Some(p0), Some(p1)) = (polygon_of(poly), polygon_of(&out)) {
            before.push(p0);
            after.push(p1);
        }
        polys.push(out);
    }
    let cut = MultiPolygon::new(before).difference(&MultiPolygon::new(after));
    Planted { polys, cut }
}

/// Contour assaini avant l'arrondi : les côtés de moins de `eps` (marches, chanfreins et dents de
/// quelques centimètres laissés par les opérations booléennes) sont résorbés dans l'angle formé par
/// les côtés voisins ; les sommets alignés et les demi-tours sont retirés.
fn tidy(ring: &[V2], eps: f64) -> Vec<V2> {
    let mut r = ring.to_vec();
    loop {
        let n = r.len();
        if n < 4 {
            return r;
        }
        if let Some(i) = (0..n).find(|&i| (r[(i + 1) % n] - r[i]).len() < eps) {
            let (a0, a1, b0, b1) = (r[(i + n - 1) % n], r[i], r[(i + 1) % n], r[(i + 2) % n]);
            let mid = (a1 + b0) * 0.5;
            let (u, v) = (a1 - a0, b1 - b0);
            let den = u.cross(v);
            let corner = if den.abs() > 1e-9 {
                let p = a0 + u * ((b0 - a0).cross(v) / den);
                if (p - mid).len() < 3.0 * eps {
                    p
                } else {
                    mid
                }
            } else {
                mid
            };
            r[i] = corner;
            r.remove((i + 1) % n);
            continue;
        }
        let flat = (0..n).find(|&i| {
            let (p, v, q) = (r[(i + n - 1) % n], r[i], r[(i + 1) % n]);
            (v - p).norm().cross((q - v).norm()).abs() < 0.017
        });
        match flat {
            Some(i) => {
                r.remove(i);
            }
            None => return r,
        }
    }
}

fn pts(r: &[[f64; 2]]) -> Vec<V2> {
    r.iter().map(|p| v2(p[0], p[1])).collect()
}

fn r2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Arrondit les angles saillants (intérieur à gauche) dont aucun côté ne touche une place et dont
/// au moins un côté borde la chaussée. Le rayon est réduit si un côté est trop court : la moitié
/// du côté quand l'angle voisin est aussi arrondi, presque tout le côté sinon.
fn round_ring(ring: &[V2], r: f64, side: &dyn Fn(V2, V2) -> Side) -> Vec<V2> {
    let n = ring.len();
    if n < 3 {
        return ring.to_vec();
    }
    let sides: Vec<Side> = (0..n).map(|i| side(ring[i], ring[(i + 1) % n])).collect();
    let turn_at = |i: usize| {
        let (p, v, q) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
        let (a, b) = ((v - p).norm(), (q - v).norm());
        a.cross(b).atan2(a.dot(b))
    };
    let rounded: Vec<bool> = (0..n)
        .map(|i| {
            let (s_in, s_out) = (sides[(i + n - 1) % n], sides[i]);
            turn_at(i) > 15f64.to_radians()
                && s_in != Side::Stall
                && s_out != Side::Stall
                && s_in != Side::Walk
                && s_out != Side::Walk
                && (s_in == Side::Road || s_out == Side::Road)
        })
        .collect();
    let mut out = Vec::with_capacity(n * 2);
    for i in 0..n {
        let (p, v, q) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
        let (la, lb) = ((v - p).len(), (q - v).len());
        let a = (v - p).norm();
        let turn = turn_at(i);
        if !rounded[i] {
            out.push(v);
            continue;
        }
        // Demi-angle intérieur, longueur de tangente, rayon réduit si les côtés sont courts.
        let half = (PI - turn) * 0.5;
        let share = |other: bool| if other { 0.5 } else { 0.95 };
        let tmax = (la * share(rounded[(i + n - 1) % n])).min(lb * share(rounded[(i + 1) % n]));
        let (mut t, mut rr) = (r / half.tan(), r);
        if t > tmax {
            t = tmax;
            rr = t * half.tan();
        }
        if rr < 0.05 {
            out.push(v);
            continue;
        }
        let p1 = v - a * t;
        let c = p1 + a.left() * rr;
        let a0 = (p1 - c).y.atan2((p1 - c).x);
        let k = ((turn / (PI / 12.0)).ceil() as usize).max(2);
        for j in 0..=k {
            let ang = a0 + turn * j as f64 / k as f64;
            out.push(c + v2(ang.cos(), ang.sin()) * rr);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sq(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<V2> {
        vec![v2(x0, y0), v2(x1, y0), v2(x1, y1), v2(x0, y1)]
    }

    #[test]
    fn corners_round_on_the_road_and_stay_square_against_stalls() {
        // Îlot 3 × 5 m : place à l'ouest, chaussée au nord et à l'est, limite au sud.
        let island = sq(0.0, 0.0, 3.0, 5.0);
        let side = |a: V2, b: V2| -> Side {
            let m = (a + b) * 0.5;
            if m.x < 0.01 {
                Side::Stall
            } else if m.y < 0.01 {
                Side::Edge
            } else {
                Side::Road
            }
        };
        let r = round_ring(&island, 1.0, &side);
        // Angle nord-est (chaussée-chaussée) arrondi au rayon 1 : le sommet (3, 5) disparaît,
        // l'arc passe à 1 − 1/√2 du coin sur la bissectrice.
        assert!(!r.iter().any(|p| (*p - v2(3.0, 5.0)).len() < 1e-9));
        let near = r
            .iter()
            .map(|p| (*p - v2(3.0, 5.0)).len())
            .fold(f64::MAX, f64::min);
        assert!((near - (2f64.sqrt() - 1.0)).abs() < 0.02, "{near}");
        // Angle sud-est (chaussée-limite) arrondi ; angles contre la place, vifs.
        assert!(!r.iter().any(|p| (*p - v2(3.0, 0.0)).len() < 1e-9));
        assert!(r.iter().any(|p| (*p - v2(0.0, 0.0)).len() < 1e-9));
        assert!(r.iter().any(|p| (*p - v2(0.0, 5.0)).len() < 1e-9));
    }

    #[test]
    fn short_cap_keeps_a_one_metre_radius_when_the_other_corner_is_square() {
        // Îlot d'extrémité 1,5 × 5 m : place au nord, chaussée à l'est, au sud et à l'ouest.
        let cap = sq(0.0, 0.0, 5.0, 1.5);
        let side = |a: V2, b: V2| -> Side {
            if ((a + b) * 0.5).y > 1.49 {
                Side::Stall
            } else {
                Side::Road
            }
        };
        let r = round_ring(&cap, 1.0, &side);
        let near = r
            .iter()
            .map(|p| (*p - v2(5.0, 0.0)).len())
            .fold(f64::MAX, f64::min);
        assert!(
            (near - (2f64.sqrt() - 1.0)).abs() < 0.02,
            "rayon réduit : {near}"
        );
        assert!(
            r.iter().any(|p| (*p - v2(5.0, 1.5)).len() < 1e-9),
            "angle contre la place vif"
        );
    }

    #[test]
    fn tidy_removes_centimetre_steps_and_chamfers() {
        // Rectangle 10 × 5 avec un chanfrein de 2 cm, une dent de 3 cm et un point aligné.
        let r = vec![
            v2(0.0, 0.0),
            v2(5.0, 0.0),
            v2(9.98, 0.0),
            v2(10.0, 0.02),
            v2(10.0, 5.0),
            v2(0.03, 5.0),
            v2(0.03, 4.97),
            v2(0.0, 4.97),
        ];
        let t = tidy(&r, 0.08);
        assert_eq!(t.len(), 4, "{t:?}");
        assert!(t.iter().any(|p| (*p - v2(10.0, 0.0)).len() < 1e-6), "{t:?}");
    }

    #[test]
    fn radius_shrinks_on_short_sides() {
        // Bande de 1 m de large, tout autour la chaussée : rayon ramené à la moitié du petit côté.
        let strip = sq(0.0, 0.0, 1.0, 6.0);
        let r = round_ring(&strip, 1.0, &|_, _| Side::Road);
        let xs: Vec<f64> = r.iter().map(|p| p.x).collect();
        assert!(xs.iter().all(|&x| (-1e-9..=1.0 + 1e-9).contains(&x)));
        let near = r
            .iter()
            .map(|p| (*p - v2(1.0, 6.0)).len())
            .fold(f64::MAX, f64::min);
        assert!((near - 0.5 * (2f64.sqrt() - 1.0)).abs() < 0.02, "{near}");
    }
}
