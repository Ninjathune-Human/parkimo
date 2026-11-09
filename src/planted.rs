//! Espaces plantés : tout le résiduel de la zone stationnable (angles, têtes de rangée en biais,
//! interstices entre places, îlots d'extrémité et de bordure).
//!
//! Les angles saillants tournés vers la chaussée sont arrondis au rayon `RADIUS`, au rayon
//! `TURN_RADIUS` là où l'on tourne d'une voie dans l'autre dans le sens de la circulation (moins si
//! les côtés sont trop courts) ; la chaussée prend la place de l'arrondi. Un angle contre une place
//! ou un fond de rangée reste vif : une bande plantée entre deux places est rectangulaire.

use crate::circulation::Flow;
use crate::geom::{v2, Quad, QuadHash, Region, V2};
use crate::occupation::{polygon_of, polys_of, PolyRings};
use crate::region;
use geo::algorithm::buffer::{BufferStyle, LineJoin};
use geo::{BooleanOps, Buffer, MultiPolygon};
use std::f64::consts::PI;

/// Rayon des arrondis en bordure de chaussée (m).
pub const RADIUS: f64 = 1.0;
/// Rayon des angles autour desquels on tourne d'une voie dans l'autre (m) : la giration y gagne.
pub const TURN_RADIUS: f64 = 3.0;

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
/// relâchement seulement, pendant un glisser le calcul direct reste léger) ; `flow` : sens de
/// circulation, s'ils sont connus, pour les rayons de giration.
pub fn build(
    b: &MultiPolygon<f64>,
    paved: &MultiPolygon<f64>,
    stalls: &[Quad],
    road: &[Quad],
    walks: &[Quad],
    clean: bool,
    flow: Option<&Flow>,
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
        // Angles où l'on tourne d'une voie dans l'autre, dans le sens de la circulation : raccord au
        // rayon de giration, retiré de l'espace planté.
        let mut fillets = Vec::new();
        for r in rings.iter().filter(|_| flow.is_some()) {
            let n = r.len();
            let sides: Vec<Side> = (0..n).map(|i| side(r[i], r[(i + 1) % n])).collect();
            let corners = rounded_corners(r, &sides);
            for i in 0..n {
                let (p, q) = (r[(i + n - 1) % n], r[(i + 1) % n]);
                let road = sides[(i + n - 1) % n] == Side::Road && sides[i] == Side::Road;
                if corners[i] && road && flow.is_some_and(|f| turns(f, p, r[i], q)) {
                    fillets.extend(fillet(r, i, TURN_RADIUS, &corners).into_iter().flat_map(|m| m.0));
                }
            }
        }
        let outs = match polygon_of(&out) {
            Some(p1) if !fillets.is_empty() => {
                let left = fillets.into_iter().fold(MultiPolygon::new(vec![p1]), |m, f| m.difference(&MultiPolygon::new(vec![f])));
                polys_of(&left, 1.0).0
            }
            _ => vec![out],
        };
        if let Some(p0) = polygon_of(poly) {
            before.push(p0);
            after.extend(outs.iter().filter_map(polygon_of));
        }
        polys.extend(outs);
    }
    let cut = MultiPolygon::new(before).difference(&MultiPolygon::new(after));
    Planted { polys, cut }
}

/// Contour assaini avant l'arrondi : les côtés de moins de `eps` (marches, chanfreins et dents de
/// quelques centimètres laissés par les opérations booléennes) sont résorbés dans l'angle formé par
/// les côtés voisins ; les sommets alignés, les demi-tours et les dents (pointe de moins de 30° dont
/// un côté fait moins de 0,5 m) sont retirés. Une pointe longue, comme l'about d'une bande plantée
/// le long de places à 30°, reste.
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
            let (a, b) = ((v - p).norm(), (q - v).norm());
            let tooth = a.dot(b) < -(30f64.to_radians().cos()) && (v - p).len().min((q - v).len()) < 0.5;
            a.cross(b).abs() < 0.017 || tooth
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

/// L'angle `v` d'un contour (précédé de `p`, suivi de `q`, intérieur à gauche, chaussée à droite)
/// est-il l'intérieur d'un virage permis : arrivée le long d'un côté, départ le long de l'autre, dans
/// le sens de la circulation ? Voie de chaque côté lue à 1,5 m de la bordure, au plus à 3 m de l'angle.
fn turns(flow: &Flow, p: V2, v: V2, q: V2) -> bool {
    let (a, b) = ((v - p).norm(), (q - v).norm());
    let lane = |at: V2, d: V2| flow.lane_at(at + v2(d.y, -d.x) * 1.5).filter(|(u, _)| u.dot(d).abs() > 0.8);
    let (Some((u1, one1)), Some((u2, one2))) = (
        lane(v - a * ((v - p).len() * 0.5).min(3.0), a),
        lane(v + b * ((q - v).len() * 0.5).min(3.0), b),
    ) else {
        return false;
    };
    // Sens `d` permis sur une voie de direction `u`.
    let ok = |u: V2, one: bool, d: V2| !one || u.dot(d) > 0.0;
    (ok(u1, one1, a) && ok(u2, one2, b)) || (ok(u1, one1, a * -1.0) && ok(u2, one2, b * -1.0))
}

/// Angle du contour en `i` (intérieur à gauche), positif à gauche.
fn turn_at(ring: &[V2], i: usize) -> f64 {
    let n = ring.len();
    let (p, v, q) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
    let (a, b) = ((v - p).norm(), (q - v).norm());
    a.cross(b).atan2(a.dot(b))
}

/// Angles arrondis : saillants, dont aucun côté ne touche une place ou un cheminement et dont au
/// moins un côté borde la chaussée.
fn rounded_corners(ring: &[V2], sides: &[Side]) -> Vec<bool> {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (s_in, s_out) = (sides[(i + n - 1) % n], sides[i]);
            turn_at(ring, i) > 15f64.to_radians()
                && s_in != Side::Stall
                && s_out != Side::Stall
                && s_in != Side::Walk
                && s_out != Side::Walk
                && (s_in == Side::Road || s_out == Side::Road)
        })
        .collect()
}

/// Raccord de giration à l'angle `i` du contour (intérieur à gauche) : la partie de l'angle hors du
/// cercle de rayon `r` tangent à ses deux côtés, à rendre à la chaussée. Un côté est suivi tant
/// qu'il garde sa direction à 30° près (bordure courbe faite de petits segments) ; la tangente est
/// bornée à la moitié de sa longueur s'il finit sur un angle arrondi, presque à toute sinon. Sur une
/// bordure courbe, le cercle est tangent aux cordes menées de l'angle aux points de raccord. Aucun
/// raccord s'il ne dépasse pas l'arrondi courant.
fn fillet(ring: &[V2], i: usize, r: f64, corners: &[bool]) -> Option<MultiPolygon<f64>> {
    let n = ring.len();
    let v = ring[i];
    let step = |j: usize, forward: bool| if forward { (j + 1) % n } else { (j + n - 1) % n };
    let (a, b) = ((v - ring[step(i, false)]).norm(), (ring[step(i, true)] - v).norm());
    // Longueur disponible le long du côté de direction `d`, vers l'avant ou vers l'arrière.
    let room = |forward: bool, d: V2| {
        let (mut len, mut j) = (0.0, i);
        for _ in 0..n {
            let k = step(j, forward);
            let seg = if forward { ring[k] - ring[j] } else { ring[j] - ring[k] };
            if seg.norm().dot(d) < 30f64.to_radians().cos() {
                break;
            }
            len += seg.len();
            j = k;
            if corners[j] || j == i {
                break;
            }
        }
        len * if corners[j] { 0.5 } else { 0.95 }
    };
    // Premier point du côté à `t` de `v` à vol d'oiseau (au plus à `len` le long du côté), précédé
    // des sommets passés.
    let reach = |forward: bool, t: f64, len: f64| {
        let (mut walked, mut j, mut pts) = (0.0, i, Vec::new());
        for _ in 0..n {
            let k = step(j, forward);
            let (p, d) = (ring[j], ring[k] - ring[j]);
            let l = d.len().max(1e-12);
            if (ring[k] - v).len() >= t || walked + l >= len {
                // |p + s d − v| = t, plus grande racine dans [0, 1].
                let (w, u) = (p - v, d * (1.0 / l));
                let disc = (w.dot(u)).powi(2) - (w.dot(w) - t * t);
                let s = (-w.dot(u) + disc.max(0.0).sqrt()).clamp(0.0, (len - walked).clamp(0.0, l));
                pts.push(p + u * s);
                return pts;
            }
            walked += l;
            j = k;
            pts.push(ring[j]);
        }
        pts
    };
    let half = (PI - turn_at(ring, i)) * 0.5;
    let (back, ahead) = (room(false, a), room(true, b));
    let t = (r / half.tan()).min(back).min(ahead);
    // Points de raccord à même distance de l'angle.
    let (p1, p2) = (reach(false, t, back), reach(true, t, ahead));
    let tc = (*p1.last()? - v).len().min((*p2.last()? - v).len());
    let (mut p1, p2) = (reach(false, tc, back), reach(true, tc, ahead));
    let (c1, c2) = ((*p1.last()? - v).norm(), (*p2.last()? - v).norm());
    let half = c1.dot(c2).clamp(-1.0, 1.0).acos() * 0.5;
    let rr = tc * half.tan();
    if rr <= RADIUS * 1.05 || half < 1e-3 {
        return None;
    }
    let c = v + (c1 + c2).norm() * (tc / half.cos());
    let disc: Vec<V2> = (0..64).map(|k| c + v2((k as f64 * PI / 32.0).cos(), (k as f64 * PI / 32.0).sin()) * rr).collect();
    // Coin : du point de raccord arrière à l'angle, puis au point de raccord avant, par le contour
    // poussé de 5 cm vers la chaussée (la bordure part entière malgré les arrondis au centimètre).
    p1.reverse();
    let rim: Vec<V2> = p1.into_iter().chain(std::iter::once(v)).chain(p2).collect();
    let m = rim.len();
    let shifted = (0..m).map(|k| {
        let d = (rim[(k + 1).min(m - 1)] - rim[k]).norm() + (rim[k] - rim[k.saturating_sub(1)]).norm();
        rim[k] + v2(d.y, -d.x).norm() * 0.05
    });
    let zone: Vec<V2> = std::iter::once(rim[0]).chain(shifted).chain(std::iter::once(rim[m - 1])).collect();
    Some(MultiPolygon::new(vec![region::to_poly(&zone)]).difference(&MultiPolygon::new(vec![region::to_poly(&disc)])))
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
    let rounded = rounded_corners(ring, &sides);
    let mut out = Vec::with_capacity(n * 2);
    for i in 0..n {
        let (p, v, q) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
        let (la, lb) = ((v - p).len(), (q - v).len());
        let a = (v - p).norm();
        let turn = turn_at(ring, i);
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
    use crate::circulation::{Axis, Flow};
    use crate::model::P2;

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
    fn tidy_removes_a_sharp_spike() {
        // Dent de 26° (côté de 0,34 m) laissée par les opérations booléennes au front d'une rangée
        // (plan rect à 60°) : retirée, le contour ne se recoupe plus une fois arrondi. La pointe
        // longue d'une bande plantée reste.
        let r = vec![v2(3.0, 77.0), v2(3.0, 71.42), v2(8.58, 68.2), v2(8.58, 71.5), v2(10.32, 71.52), v2(10.02, 71.37), v2(13.26, 77.0)];
        let t = tidy(&r, 0.2);
        assert!(!t.iter().any(|p| (*p - v2(10.32, 71.52)).len() < 1e-6), "{t:?}");
        assert_eq!(t.len(), 6, "{t:?}");
        let strip = vec![v2(0.0, 0.0), v2(30.0, 0.0), v2(0.0, 3.0)];
        assert_eq!(tidy(&strip, 0.2).len(), 3);
    }

    /// Îlot 4 × 5 m (x 0 → 4, y 0 → 5), place à l'est ; voies de 5 m à l'ouest (axe x = −2,5,
    /// montante), au sud (axe y = −2,5, vers l'ouest) et au nord (axe y = 7,5, sens `north`).
    fn junction(north: [P2; 2]) -> Flow {
        let axis = |pts: Vec<P2>| Axis { pts, w: 5.0, sens: 1, rule: None };
        Flow::new(&[
            axis(vec![[10.0, -2.5], [-2.5, -2.5]]),
            axis(vec![[-2.5, -2.5], [-2.5, 7.5]]),
            axis(north.to_vec()),
        ])
    }

    #[test]
    fn only_corners_turned_around_in_the_flow_direction_get_the_turning_radius() {
        // Vers l'est au nord : on tourne à droite autour de l'angle nord-ouest, comme autour de
        // l'angle sud-ouest en remontant depuis le sud.
        let east = junction([[-2.5, 7.5], [10.0, 7.5]]);
        assert!(turns(&east, v2(0.0, 5.0), v2(0.0, 0.0), v2(4.0, 0.0)), "angle sud-ouest");
        assert!(turns(&east, v2(4.0, 5.0), v2(0.0, 5.0), v2(0.0, 0.0)), "angle nord-ouest");
        // Vers l'ouest au nord : la voie ouest monte, la voie nord en revient ; personne ne tourne
        // autour de l'angle nord-ouest.
        let west = junction([[10.0, 7.5], [-2.5, 7.5]]);
        assert!(!turns(&west, v2(4.0, 5.0), v2(0.0, 5.0), v2(0.0, 0.0)), "angle nord-ouest");
        assert!(turns(&west, v2(0.0, 5.0), v2(0.0, 0.0), v2(4.0, 0.0)), "angle sud-ouest");
    }

    #[test]
    fn a_turning_corner_is_rounded_wider_within_its_sides() {
        // Îlot 4 × 5 m, place à l'est ; côté ouest en segments de 1 m (bordure courbe). Angle
        // nord-ouest tourné, angle sud-ouest arrondi aussi : tangente bornée à la moitié du côté
        // ouest (2,5 m), rayon 2,5, arc à 2,5 (√2 − 1) du coin. Le côté nord (4 m) finit sur un angle
        // vif.
        let island = vec![v2(0.0, 0.0), v2(4.0, 0.0), v2(4.0, 5.0), v2(0.0, 5.0), v2(0.0, 4.0), v2(0.0, 3.0), v2(0.0, 2.0), v2(0.0, 1.0)];
        let corners = [true, false, false, true, false, false, false, false];
        let cut = fillet(&island, 3, TURN_RADIUS, &corners).expect("raccord");
        let left = MultiPolygon::new(vec![region::to_poly(&island)]).difference(&cut);
        let near = left.0[0].exterior().points().map(|p| (v2(p.x(), p.y()) - v2(0.0, 5.0)).len()).fold(f64::MAX, f64::min);
        assert!((near - 2.5 * (2f64.sqrt() - 1.0)).abs() < 0.02, "{near}");
        // Pas plus que l'arrondi courant : aucun raccord.
        assert!(fillet(&island, 3, RADIUS, &corners).is_none());
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
