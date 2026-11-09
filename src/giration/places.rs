//! Test d'une place : pose garée en marche avant, voisines occupées, manœuvre d'entrée depuis
//! la voie, 10 m en amont, et manœuvre de sortie en marche arrière, planifiées depuis la pose garée.
//!
//! Le modèle bicyclette est réversible : une entrée rejouée à l'envers est un mouvement possible
//! depuis la place. On planifie donc depuis la pose garée, connue exactement, vers la voie, que
//! l'on atteint avec une tolérance : l'entrée est la manœuvre qui rejoint le point d'approche,
//! parcourue à rebours. Les pneus ne passent jamais une bordure : ils restent sur la chaussée et
//! les places, jamais sur la pleine terre.

use super::epure::{Run, Sample};
use super::kinematics::{Pose, Profile, State};
use super::planner::{Costs, Outcome, Piece, Plan, Search, Table};
use super::scene::{Obstacle, ObstacleKind, Scene};
use super::sweep::{body_contact, footprint, margin_contact, near_body, tyre_on_kerb_among};
use super::vehicle::Vehicle;
use crate::model::P2;
use serde::Deserialize;
use std::f64::consts::PI;

/// Axe de voie (sortie `circulation.axes` du moteur de plan).
#[derive(Clone, Deserialize)]
pub struct Lane {
    pub pts: Vec<P2>,
    pub w: f64,
    /// 2 : double sens (circulation à droite), 1 : sens unique dans l'ordre des points.
    pub sens: u8,
}

pub struct Ctx<'a> {
    pub v: &'a Vehicle,
    pub profile: Profile,
    pub margin: f64,
    pub speed: f64,
    /// Obstacles du plan (bordures, murs, limites), sans les véhicules garés.
    pub base: &'a Scene,
    pub stalls: &'a [[P2; 4]],
    pub lanes: &'a [Lane],
    pub neighbours: bool,
    /// Estimation sans obstacle (voir `Table`), construite avec `table`.
    pub table: &'a Table,
    /// Budget de la recherche avec reprises (états créés).
    pub max_nodes: usize,
}

/// Statut d'une manœuvre : trouvée, impossible dans le domaine exploré, ou non conclu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    Feasible,
    Infeasible,
    Unknown,
}

/// Coûts de la recherche : marche avant, marche arrière (la manœuvre part de la place).
const FORWARD: f64 = 2.0;
const REVERSE: f64 = 1.0;

pub fn table(v: &Vehicle, profile: Profile) -> Table {
    Table::build(v, profile, FORWARD, REVERSE, LANE_TOL, HEADING_TOL)
}

pub struct Tested {
    /// Pose garée (milieu de l'essieu arrière), nez vers le fond de la place.
    pub park: Pose,
    pub scene: Scene,
    /// Entrée en marche avant depuis l'allée, puis sortie en marche arrière vers l'allée.
    pub entry: Vec<Piece>,
    pub exit: Vec<Piece>,
    pub status: Status,
    /// Contacts de la manœuvre montrée quand aucune n'est possible (la moins mauvaise trouvée).
    pub collisions: usize,
    pub reason: Option<&'static str>,
}

/// Décalage des voisines garées vers la place testée (m).
const NEIGHBOUR_SHIFT: f64 = 0.10;
/// Arrivée dans l'allée : caisse dans la largeur de l'allée (à 0,10 m près de ses bords), au moins
/// à 0,30 m de l'axe, et cap à 8° près.
const LANE_TOL: f64 = 0.3;
const HEADING_TOL: f64 = 8.0 * PI / 180.0;
/// Rayon de la zone explorée autour de l'entrée de la place (m).
const RADIUS: f64 = 16.0;
/// Approche : la voiture arrive dans le sens de la circulation, alignée sur la voie, en amont, à
/// cette distance à vol d'oiseau de l'ouverture (m), en remontant les voies à travers virages et
/// carrefours. Faute de pouvoir partir de là, la place n'est pas accessible.
const APPROACH: f64 = 10.0;
/// Au-delà de cet écart entre la voie, à la distance d'approche en amont, et l'axe d'entrée de la
/// place, l'entrée revient sur l'approche (place juste après un virage, côté intérieur) : contrôlée
/// dès la conception du plan.
const TURN_BACK: f64 = 100.0 * PI / 180.0;
/// Approche atteinte : centre de la caisse au point d'approche ou en amont, jusqu'à deux fois cette
/// distance, le long de la voie (m).
const APPROACH_TOL: f64 = 1.5;
/// Budgets de la recherche : d'un seul tenant, puis (budget réglable) avec reprises ; la moins
/// mauvaise manœuvre, avec contacts, n'est cherchée que pour l'illustrer.
const NODES_DIRECT: usize = 5_000;
/// Recherche à 1 reprise au plus (le cas le plus fréquent, « en deux fois »), avant la recherche
/// sans limite.
const STAGED_REPRISES: usize = 1;
const NODES_STAGE: usize = 20_000;
pub const NODES_REPRISES: usize = 150_000;
const NODES_CONTACT: usize = 20_000;

fn sub(a: P2, b: P2) -> P2 {
    [a[0] - b[0], a[1] - b[1]]
}
fn mid(a: P2, b: P2) -> P2 {
    [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])]
}
fn dot(a: P2, b: P2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn unit(a: P2) -> P2 {
    let l = a[0].hypot(a[1]).max(1e-12);
    [a[0] / l, a[1] / l]
}
fn wrap(a: f64) -> f64 {
    (a + PI).rem_euclid(2.0 * PI) - PI
}

/// Pose garée dans la place `q` (q0, q1 au fond ; q2, q3 sur l'allée) : caisse centrée, nez vers
/// le fond, coin de pare-chocs le plus avancé à `gap` de la ligne de fond.
pub fn park_pose(v: &Vehicle, q: &[P2; 4], gap: f64) -> Pose {
    let back = mid(q[0], q[1]);
    let front = mid(q[2], q[3]);
    // Axe de la place : du milieu du fond au milieu de l'ouverture. Une place en coin (cassure de
    // la limite) a des côtés non parallèles : l'axe d'un seul côté pousserait la caisse sur l'autre.
    let a = unit(sub(front, back)); // du fond vers l'allée
    let lat = [-a[1], a[0]];
    let edge = unit(sub(q[1], q[0]));
    let mut nb = [-edge[1], edge[0]];
    if dot(sub(front, q[0]), nb) < 0.0 {
        nb = [-nb[0], -nb[1]];
    }
    let t = (gap + v.width / 2.0 * dot(lat, nb).abs()) / dot(a, nb);
    let bumper = [back[0] + a[0] * t, back[1] + a[1] * t];
    let e = v.wheelbase + v.front_overhang;
    Pose { x: bumper[0] + a[0] * e, y: bumper[1] + a[1] * e, heading: (-a[1]).atan2(-a[0]) }
}

/// Voiture garée : nez à 2 cm de la ligne de fond quand la place est jointe à une autre (marquage
/// seul), à la marge + 2 cm d'une bordure, d'un mur ou d'une limite.
fn parked(ctx: &Ctx, q: &[P2; 4]) -> Pose {
    let deep = park_pose(ctx.v, q, 0.02);
    if margin_contact(ctx.v, ctx.base, &deep, ctx.margin) {
        park_pose(ctx.v, q, ctx.margin + 0.02)
    } else {
        deep
    }
}

/// Places voisines : celles qui partagent un côté avec la place `i`.
fn neighbours(stalls: &[[P2; 4]], i: usize) -> Vec<usize> {
    let q = &stalls[i];
    let sides = [(q[0], q[3]), (q[1], q[2])];
    let close = |a: P2, b: P2| (a[0] - b[0]).hypot(a[1] - b[1]) < 0.1;
    (0..stalls.len())
        .filter(|&j| j != i)
        .filter(|&j| {
            let r = &stalls[j];
            [(r[0], r[3]), (r[1], r[2])].iter().any(|&(c, d)| {
                sides.iter().any(|&(a, b)| (close(a, c) && close(b, d)) || (close(a, d) && close(b, c)))
            })
        })
        .collect()
}

fn car_obstacle(v: &Vehicle, p: &Pose) -> Obstacle {
    let f = footprint(v, p, 0.0);
    Obstacle { kind: ObstacleKind::Car, height: f64::INFINITY, pts: vec![f[0], f[1], f[2], f[3], f[0]] }
}

/// Point le plus proche sur une polyligne : (point, direction du segment).
fn nearest(pts: &[P2], p: P2) -> (P2, P2, f64) {
    let mut best = (pts[0], [1.0, 0.0], f64::INFINITY);
    for w in pts.windows(2) {
        let d = sub(w[1], w[0]);
        let l2 = dot(d, d).max(1e-18);
        let t = (dot(sub(p, w[0]), d) / l2).clamp(0.0, 1.0);
        let q = [w[0][0] + t * d[0], w[0][1] + t * d[1]];
        let dist = (p[0] - q[0]).hypot(p[1] - q[1]);
        if dist < best.2 {
            best = (q, unit(d), dist);
        }
    }
    best
}

/// Mouvement planifié, rejoué tel quel : échantillons tous les 0,10 m, braquage à l'arrêt à chaque
/// changement de courbure (conducteur habile), à la vitesse de rotation du volant du profil.
pub fn motion(v: &Vehicle, profile: Profile, pieces: &[Piece], speed_kmh: f64) -> Run {
    let speed = (speed_kmh / 3.6).max(0.1);
    let mut run = Run { samples: Vec::new(), deviation: Vec::new(), reached: Vec::new(), stationary_time: 0.0 };
    // La voiture arrive avec l'angle de roues du premier état ; ensuite, braquage à l'arrêt au début
    // de chaque morceau (rebroussement), le volant tournant en roulant à l'intérieur d'un morceau.
    let (mut s, mut t) = (0.0, 0.0);
    let mut steer = pieces.first().and_then(|p| p.states.first()).map_or(0.0, |st| st.steer);
    for (k, pc) in pieces.iter().enumerate() {
        let Some(first) = pc.states.first() else { continue };
        let dt = (first.steer - steer).abs() / profile.steer_rate(v);
        run.stationary_time += dt;
        t += dt;
        run.samples.push(Sample { state: *first, section: k, s, t });
        for w in pc.states.windows(2) {
            let d = (w[1].pose.x - w[0].pose.x).hypot(w[1].pose.y - w[0].pose.y);
            s += d;
            t += d / speed;
            run.samples.push(Sample { state: w[1], section: k, s, t });
        }
        steer = pc.states.last().map_or(steer, |st| st.steer);
        run.deviation.push(0.0);
        run.reached.push(true);
    }
    run
}

/// Points d'approche de l'ouverture `mouth` desservie par `lane` : premier point de la circulation,
/// en remontant depuis le pied de l'ouverture, à `dist` au moins à vol d'oiseau de l'ouverture et
/// qui le reste sur toute la fenêtre d'arrivée en amont ; au-delà du début de la voie, sur les voies
/// qui arrivent au carrefour (toutes les branches ; les deux sens d'une voie à double sens). Après
/// un virage, l'approche part donc avant le virage. Point, direction de marche, largeur de la voie.
fn approaches(lanes: &[Lane], lane: &Lane, mouth: P2, dist: f64) -> Vec<(P2, P2, f64)> {
    // Pied de l'ouverture sur la voie : abscisse curviligne depuis le premier point.
    let (mut best, mut s_foot, mut acc) = (f64::INFINITY, 0.0, 0.0);
    for w in lane.pts.windows(2) {
        let d = sub(w[1], w[0]);
        let l = d[0].hypot(d[1]);
        let t = (dot(sub(mouth, w[0]), d) / (l * l).max(1e-18)).clamp(0.0, 1.0);
        let q = [w[0][0] + t * d[0], w[0][1] + t * d[1]];
        let e = (mouth[0] - q[0]).hypot(mouth[1] - q[1]);
        if e < best {
            (best, s_foot) = (e, acc + t * l);
        }
        acc += l;
    }
    let mut out = Vec::new();
    // Sens de marche le long des points (`along`), ou à rebours sur une voie à double sens.
    walk(lanes, &lane.pts, lane.w, s_foot, (mouth, dist), true, 0, None, &mut out);
    if lane.sens != 1 {
        walk(lanes, &lane.pts, lane.w, s_foot, (mouth, dist), false, 0, None, &mut out);
    }
    out
}

/// Point d'approche retenu en remontant, et la longueur parcourue depuis.
type Found = Option<((P2, P2, f64), f64)>;

/// Remonte la polyligne `pts` depuis l'abscisse `s`, la circulation allant dans l'ordre des points si
/// `along`, jusqu'au premier point à `reach.1` au moins de `reach.0`, à vol d'oiseau, qui le reste
/// sur `2 × APPROACH_TOL` en amont (`found` : point retenu sur une voie en aval) ; au début de la
/// voie, continue sur les voies qui arrivent à ce carrefour.
#[allow(clippy::too_many_arguments)]
fn walk(lanes: &[Lane], pts: &[P2], w: f64, s: f64, reach: (P2, f64), along: bool, depth: usize, mut found: Found, out: &mut Vec<(P2, P2, f64)>) {
    let total: f64 = pts.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum();
    // Distance disponible en amont, et le carrefour d'où vient la circulation.
    let (room, node) = if along { (s, pts[0]) } else { (total - s, pts[pts.len() - 1]) };
    let (from, dist) = reach;
    let at = |t: f64| {
        let (p, u) = point_at(pts, if along { s - t } else { s + t });
        (p, if along { u } else { [-u[0], -u[1]] }, w)
    };
    // Pas de 0,2 m : bien en deçà de la tolérance d'arrivée.
    let mut t = 0.0;
    while t <= room {
        let g = at(t);
        found = match found {
            _ if (g.0[0] - from[0]).hypot(g.0[1] - from[1]) < dist => None,
            Some((f, run)) => Some((f, run + 0.2)),
            None => Some((g, 0.0)),
        };
        if let Some((f, run)) = found {
            if run >= 2.0 * APPROACH_TOL {
                out.push(f);
                return;
            }
        }
        t += 0.2;
    }
    if depth >= 4 {
        out.push(found.map_or_else(|| at(room), |f| f.0));
        return;
    }
    let close = |a: P2| (a[0] - node[0]).hypot(a[1] - node[1]) < 1.0;
    let mut fed = false;
    for l in lanes.iter().filter(|l| l.pts.len() >= 2 && !std::ptr::eq(l.pts.as_ptr(), pts.as_ptr())) {
        let len: f64 = l.pts.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum();
        // Voie dont la circulation arrive au carrefour : par sa fin, ou par son début à double sens.
        if close(l.pts[l.pts.len() - 1]) {
            fed = true;
            walk(lanes, &l.pts, l.w, len, reach, true, depth + 1, found, out);
        } else if l.sens != 1 && close(l.pts[0]) {
            fed = true;
            walk(lanes, &l.pts, l.w, 0.0, reach, false, depth + 1, found, out);
        }
    }
    if !fed {
        // Rien n'arrive (accès depuis la rue, impasse) : l'approche part du point retenu, sinon du
        // carrefour.
        out.push(found.map_or_else(|| at(room), |f| f.0));
    }
}

/// Point et direction (ordre des points) à l'abscisse curviligne `s`, bornée à la polyligne.
fn point_at(pts: &[P2], s: f64) -> (P2, P2) {
    let (mut acc, n) = (0.0, pts.len() - 1);
    for (k, w) in pts.windows(2).enumerate() {
        let d = sub(w[1], w[0]);
        let l = d[0].hypot(d[1]);
        if acc + l >= s || k + 1 == n {
            let t = ((s - acc) / l.max(1e-12)).clamp(0.0, 1.0);
            return ([w[0][0] + t * d[0], w[0][1] + t * d[1]], unit(d));
        }
        acc += l;
    }
    (pts[0], [1.0, 0.0])
}

/// L'approche `lead` suivie de `rest` : morceaux de même sens de marche réunis à la jonction.
fn joined(mut lead: Vec<Piece>, mut rest: Vec<Piece>) -> Vec<Piece> {
    if let (Some(a), Some(b)) = (lead.last_mut(), rest.first_mut()) {
        if a.reverse == b.reverse {
            a.states.extend(b.states.drain(1..));
            rest.remove(0);
        }
    }
    lead.extend(rest);
    lead
}

/// La même manœuvre parcourue à rebours : ordre inversé, sens de marche inversé.
fn reversed(pieces: &[Piece]) -> Vec<Piece> {
    pieces.iter().rev().map(|pc| Piece { reverse: !pc.reverse, states: pc.states.iter().rev().copied().collect() }).collect()
}

/// Une place prête à être testée : pose garée, scène avec les voisines garées, voie desservante.
struct Prepared<'a> {
    pub park: Pose,
    pub scene: Scene,
    pub lane: Option<&'a Lane>,
    pub mouth: P2,
}

fn prepare<'a>(ctx: &Ctx<'a>, i: usize) -> Prepared<'a> {
    let v = ctx.v;
    let q = &ctx.stalls[i];
    let park = parked(ctx, q);
    let mut obstacles = ctx.base.obstacles.clone();
    if ctx.neighbours {
        // Parking plein : toutes les autres places de la zone explorée sont occupées ; les deux
        // voisines, garées au pire, sont décalées vers la place testée.
        let me = mid(mid(q[0], q[1]), mid(q[2], q[3]));
        let near = neighbours(ctx.stalls, i);
        for (j, r) in ctx.stalls.iter().enumerate() {
            let c = mid(mid(r[0], r[1]), mid(r[2], r[3]));
            if j == i || (c[0] - me[0]).hypot(c[1] - me[1]) > RADIUS + 8.0 {
                continue;
            }
            let p = parked(ctx, r);
            let lat = [-p.heading.sin(), p.heading.cos()];
            let s = if near.contains(&j) { dot(sub(me, [p.x, p.y]), lat).signum() * NEIGHBOUR_SHIFT } else { 0.0 };
            obstacles.push(car_obstacle(v, &Pose { x: p.x + lat[0] * s, y: p.y + lat[1] * s, heading: p.heading }));
        }
    }
    // Voie qui dessert la place : l'axe le plus proche de son ouverture.
    let mouth = mid(q[2], q[3]);
    Prepared { park, scene: Scene::new(obstacles), lane: serving(ctx.lanes, mouth), mouth }
}

/// Voie qui dessert une ouverture : l'axe le plus proche.
fn serving(lanes: &[Lane], mouth: P2) -> Option<&Lane> {
    lanes
        .iter()
        .filter(|l| l.pts.len() >= 2)
        .min_by(|a, b| nearest(&a.pts, mouth).2.total_cmp(&nearest(&b.pts, mouth).2))
        .filter(|l| nearest(&l.pts, mouth).2 < RADIUS - 4.0)
}

/// L'entrée, de cap `heading`, revient-elle sur toutes les approches (plus de `TURN_BACK`) ?
fn turning_back(ups: &[(P2, P2, f64)], heading: P2) -> bool {
    !ups.is_empty() && ups.iter().all(|(_, d, _)| dot(*d, heading).clamp(-1.0, 1.0).acos() > TURN_BACK)
}

/// L'entrée de la place `q` revient-elle sur son approche (place juste après un virage, côté
/// intérieur) ? Géométrie seule : la voie à `APPROACH` en amont, l'axe d'entrée de la place.
pub fn turns_back(q: &[P2; 4], lanes: &[Lane]) -> bool {
    let mouth = mid(q[2], q[3]);
    serving(lanes, mouth).is_some_and(|lane| turning_back(&approaches(lanes, lane, mouth, APPROACH), unit(sub(mid(q[0], q[1]), mouth))))
}

/// Arrivée de l'approche : centre de la caisse près d'un point d'approche, dans la voie, au cap de la
/// circulation ; et l'estimation qui guide la recherche vers lui : coût, sans obstacle, pour rejoindre
/// la bande de la voie d'approche (table), plus la distance restante le long de cette voie.
fn upstream_goal<'a>(v: &'a Vehicle, table: &'a Table, ups: &'a [(P2, P2, f64)]) -> (impl Fn(&Pose) -> bool + 'a, impl Fn(&Pose) -> f64 + 'a) {
    let centre = move |p: &Pose| {
        let c = (v.wheelbase + v.front_overhang - v.rear_overhang) / 2.0;
        [p.x + c * p.heading.cos(), p.y + c * p.heading.sin()]
    };
    let goal = move |p: &Pose| {
        let c = centre(p);
        ups.iter().any(|&(g, d, w)| {
            let r = sub(c, g);
            let band = (w / 2.0 - v.width / 2.0 - 0.1).max(LANE_TOL);
            (dot(r, d) + APPROACH_TOL).abs() <= APPROACH_TOL && (r[1] * d[0] - r[0] * d[1]).abs() <= band && wrap(p.heading - d[1].atan2(d[0])).abs() <= HEADING_TOL
        })
    };
    let h = move |p: &Pose| {
        let c = centre(p);
        ups.iter()
            .map(|&(g, d, w)| {
                let r = sub(c, g);
                let band = (w / 2.0 - v.width / 2.0 - 0.1).max(LANE_TOL);
                let left = d[0] * r[1] - d[1] * r[0];
                let lateral = table.get(left.signum() * (left.abs() - (band - LANE_TOL)).max(0.0), wrap(p.heading - d[1].atan2(d[0])));
                lateral + ((dot(r, d) + APPROACH_TOL).abs() - APPROACH_TOL).max(0.0) * FORWARD.min(REVERSE)
            })
            .fold(f64::INFINITY, f64::min)
    };
    (goal, h)
}

/// La place `i`, dont l'entrée revient sur son approche, se prend-elle d'un seul tenant en arrivant
/// de l'amont ? Recherche courte (budget d'un seul tenant), pour la conception du plan.
pub fn enters_in_one_go(ctx: &Ctx, i: usize) -> bool {
    let Prepared { park, scene, lane, mouth } = prepare(ctx, i);
    let Some(lane) = lane else { return false };
    let ups = approaches(ctx.lanes, lane, mouth, APPROACH);
    let (goal, h) = upstream_goal(ctx.v, ctx.table, &ups);
    let search = Search {
        v: ctx.v,
        profile: ctx.profile,
        scene: &scene,
        margin: ctx.margin,
        costs: Costs { forward: FORWARD, reverse: REVERSE, cusp: 5.0, switch: 0.3, margin: 8.0, collision: None },
        speed: (ctx.speed / 3.6).max(0.1),
        first_reverse: true,
        forward: false,
        max_cusps: 0,
        weight: 1.2,
        centre: mouth,
        radius: RADIUS,
        max_nodes: NODES_DIRECT,
    };
    matches!(search.run(park, goal, h), Outcome::Found(_))
}

/// Teste la place `i` ; `illustrate` cherche en plus, si aucune manœuvre n'est possible, la moins
/// mauvaise pour la montrer (détail d'une place).
pub fn test_stall(ctx: &Ctx, i: usize, illustrate: bool) -> Tested {
    let v = ctx.v;
    let Prepared { park, scene, lane, mouth, .. } = prepare(ctx, i);
    let fail = |scene, status, reason| Tested { park, scene, entry: Vec::new(), exit: Vec::new(), status, collisions: 0, reason: Some(reason) };
    let Some(lane) = lane else {
        return fail(scene, Status::Unknown, "aucune voie ne dessert la place");
    };
    // Impossibilité démontrée : la voiture garée ne tient pas dans la place.
    if body_contact(v, &scene, &park, 0.0).is_some() || tyre_on_kerb_among(v, &scene, &near_body(v, &scene, &park, 0.0), &State { pose: park, steer: 0.0 }) {
        return fail(scene, Status::Infeasible, "la voiture garée touche un obstacle");
    }

    let centre_of = |p: &Pose| {
        let c = (v.wheelbase + v.front_overhang - v.rear_overhang) / 2.0;
        [p.x + c * p.heading.cos(), p.y + c * p.heading.sin()]
    };
    // Caps d'arrivée possibles : sens de la voie, et son contraire à double sens ; une seule
    // recherche vise les deux. Pour chacun : écart à gauche de l'axe et cap relatif. Pendant la
    // manœuvre, le conducteur dispose de toute la largeur de l'allée.
    let band = (lane.w / 2.0 - v.width / 2.0 - 0.1).max(LANE_TOL);
    let offsets = |p: &Pose| {
        let c = centre_of(p);
        let (foot, u, _) = nearest(&lane.pts, c);
        let mut out = [(f64::INFINITY, 0.0); 2];
        for (n, along) in [true, false].into_iter().enumerate() {
            if !along && lane.sens == 1 {
                continue;
            }
            let u = if along { u } else { [-u[0], -u[1]] };
            let left = -(c[0] - foot[0]) * u[1] + (c[1] - foot[1]) * u[0];
            out[n] = (left, wrap(p.heading - u[1].atan2(u[0])));
        }
        out
    };
    let goal = |p: &Pose| offsets(p).iter().any(|&(l, t)| l.abs() <= band && t.abs() <= HEADING_TOL);
    // La table vise une bande de ±LANE_TOL : écart ramené d'autant.
    let h = |p: &Pose| {
        offsets(p)
            .iter()
            .filter(|o| o.0.is_finite())
            .map(|&(l, t)| ctx.table.get(l.signum() * (l.abs() - (band - LANE_TOL)).max(0.0), t))
            .fold(f64::INFINITY, f64::min)
    };
    let search = |from: (State, i8), max_cusps: usize, cusp: f64, collision: Option<f64>, weight: f64, max_nodes: usize, goal: &dyn Fn(&Pose) -> bool, h: &dyn Fn(&Pose) -> f64| {
        Search {
            v,
            profile: ctx.profile,
            scene: &scene,
            margin: ctx.margin,
            costs: Costs { forward: FORWARD, reverse: REVERSE, cusp, switch: 0.3, margin: 8.0, collision },
            speed: (ctx.speed / 3.6).max(0.1),
            first_reverse: true,
            forward: max_cusps > 0,
            max_cusps,
            weight,
            centre: mouth,
            radius: RADIUS,
            max_nodes,
        }
        .run_from(from.0, from.1, goal, h)
    };
    let parked = (State { pose: park, steer: 0.0 }, 0);
    // Le moins de reprises possible : d'un seul tenant, puis 1 reprise au plus (chaque recherche reste
    // petite), puis sans limite. Seule l'exploration complète du domaine sans limite de reprises
    // conclut à l'impossibilité ; un budget épuisé laisse la question ouverte.
    let staged = |goal: &dyn Fn(&Pose) -> bool, h: &dyn Fn(&Pose) -> f64| -> (Option<Plan>, Status, Option<&'static str>) {
        for k in 0..=STAGED_REPRISES {
            let budget = if k == 0 { NODES_DIRECT } else { NODES_STAGE };
            if let Outcome::Found(plan) = search(parked, k, 5.0, None, 1.2, budget.min(ctx.max_nodes), goal, h) {
                return (Some(plan), Status::Feasible, None);
            }
        }
        match search(parked, usize::MAX, 12.0, None, 1.2, ctx.max_nodes, goal, h) {
            Outcome::Found(plan) => (Some(plan), Status::Feasible, None),
            Outcome::Exhausted => (None, Status::Infeasible, Some("aucune manœuvre dans le domaine exploré")),
            Outcome::Budget => (None, Status::Unknown, Some("budget de recherche épuisé")),
        }
    };
    // Sortie : marche arrière, puis dans le sens de la voie.
    let (mut exit, mut status, mut reason) = staged(&goal, &h);
    // Entrée : la voiture part 10 m en amont, alignée sur la voie dans le sens de la circulation. Le
    // plus souvent, elle suit la voie d'un seul tenant jusqu'au départ de la sortie parcourue à
    // rebours ; sinon (virage, carrefour), l'entrée est cherchée depuis l'amont.
    let ups = approaches(ctx.lanes, lane, mouth, APPROACH);
    let (up_goal, up_h) = upstream_goal(v, ctx.table, &ups);
    let mut entry = None;
    if let Some(plan) = &exit {
        let back = reversed(&plan.pieces);
        let first = back[0].states[0];
        entry = match search((first, if back[0].reverse { 0 } else { -1 }), 0, 5.0, None, 1.2, NODES_DIRECT, &up_goal, &up_h) {
            Outcome::Found(lead) => Some(joined(reversed(&lead.pieces), back)),
            _ => None,
        };
        if entry.is_none() {
            let (found, st, _) = staged(&up_goal, &up_h);
            match found {
                Some(plan) => entry = Some(reversed(&plan.pieces)),
                None => {
                    status = st;
                    reason = Some(match st {
                        Status::Infeasible => "entrée impossible en arrivant de l'amont",
                        _ => "entrée depuis l'amont non conclue (budget de recherche épuisé)",
                    });
                }
            }
        }
    }
    let mut collisions = 0;
    if illustrate {
        // Illustration seulement : une reprise au plus suffit à montrer où la place bloque.
        if exit.is_none() {
            if let Outcome::Found(plan) = search(parked, 1, 5.0, Some(5.0), 2.0, NODES_CONTACT, &goal, &h) {
                collisions = plan.collisions;
                entry = Some(reversed(&plan.pieces));
                exit = Some(plan);
            }
        } else if entry.is_none() {
            if let Outcome::Found(plan) = search(parked, 1, 5.0, Some(5.0), 2.0, NODES_CONTACT, &up_goal, &up_h) {
                collisions = plan.collisions;
                entry = Some(reversed(&plan.pieces));
            }
        }
    }
    let Some(exit) = exit else {
        reason = reason.or(Some("aucune manœuvre trouvée"));
        return Tested { park, scene, entry: Vec::new(), exit: Vec::new(), status, collisions: 0, reason };
    };
    let entry = entry.unwrap_or_default();
    Tested { park, entry, exit: exit.pieces, status, collisions, scene, reason }
}

