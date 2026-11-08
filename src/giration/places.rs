//! Test d'une place : pose garée en marche avant, voisines occupées, manœuvre d'entrée depuis
//! l'allée et manœuvre de sortie en marche arrière, planifiées depuis la pose garée.
//!
//! Le modèle bicyclette est réversible : une entrée rejouée à l'envers est un mouvement possible
//! depuis la place. On planifie donc depuis la pose garée, connue exactement, vers l'allée, que
//! l'on atteint avec une tolérance ; l'entrée est cette manœuvre parcourue à rebours.

use super::epure::{Run, Sample};
use super::kinematics::{Pose, Profile, State};
use super::planner::{Costs, Outcome, Piece, Search, Table};
use super::scene::{Obstacle, ObstacleKind, Scene};
use super::sweep::{body_contact, footprint};
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
    pub base: &'a [Obstacle],
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
/// Budgets de la recherche : d'un seul tenant, puis (budget réglable) avec reprises ; la moins
/// mauvaise manœuvre, avec contacts, n'est cherchée que pour l'illustrer.
const NODES_DIRECT: usize = 5_000;
pub const NODES_REPRISES: usize = 40_000;
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
    let a = unit(sub(q[3], q[0])); // du fond vers l'allée
    let lat = [-a[1], a[0]];
    let back = mid(q[0], q[1]);
    let front = mid(q[2], q[3]);
    let edge = unit(sub(q[1], q[0]));
    let mut nb = [-edge[1], edge[0]];
    if dot(sub(front, q[0]), nb) < 0.0 {
        nb = [-nb[0], -nb[1]];
    }
    // Axe de la place : à mi-largeur entre les deux côtés, mesurée perpendiculairement à `a`.
    let (l0, l1) = (dot(sub(q[0], back), lat), dot(sub(q[1], back), lat));
    let c = 0.5 * (l0 + l1);
    let base = [back[0] + lat[0] * c, back[1] + lat[1] * c];
    let t = (gap + v.width / 2.0 * dot(lat, nb).abs()) / dot(a, nb);
    let bumper = [base[0] + a[0] * t, base[1] + a[1] * t];
    let e = v.wheelbase + v.front_overhang;
    Pose { x: bumper[0] + a[0] * e, y: bumper[1] + a[1] * e, heading: (-a[1]).atan2(-a[0]) }
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
    let (mut s, mut t, mut steer) = (0.0, 0.0, 0.0);
    for (k, pc) in pieces.iter().enumerate() {
        let dt = (pc.steer - steer).abs() / profile.steer_rate(v);
        run.stationary_time += dt;
        t += dt;
        steer = pc.steer;
        for (j, w) in pc.poses.windows(2).enumerate() {
            if j == 0 {
                run.samples.push(Sample { state: State { pose: w[0], steer }, section: k, s, t });
            }
            let d = (w[1].x - w[0].x).hypot(w[1].y - w[0].y);
            s += d;
            t += d / speed;
            run.samples.push(Sample { state: State { pose: w[1], steer }, section: k, s, t });
        }
        run.deviation.push(0.0);
        run.reached.push(true);
    }
    run
}

/// La même manœuvre parcourue à rebours : ordre inversé, sens de marche inversé.
fn reversed(pieces: &[Piece]) -> Vec<Piece> {
    pieces
        .iter()
        .rev()
        .map(|pc| Piece { reverse: !pc.reverse, k: pc.k, steer: pc.steer, poses: pc.poses.iter().rev().copied().collect() })
        .collect()
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
    let park = park_pose(v, q, ctx.margin + 0.02);
    let mut obstacles = ctx.base.to_vec();
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
            let p = park_pose(v, r, ctx.margin + 0.02);
            let lat = [-p.heading.sin(), p.heading.cos()];
            let s = if near.contains(&j) { dot(sub(me, [p.x, p.y]), lat).signum() * NEIGHBOUR_SHIFT } else { 0.0 };
            obstacles.push(car_obstacle(v, &Pose { x: p.x + lat[0] * s, y: p.y + lat[1] * s, heading: p.heading }));
        }
    }
    // Voie qui dessert la place : l'axe le plus proche de son ouverture.
    let mouth = mid(q[2], q[3]);
    let lane = ctx
        .lanes
        .iter()
        .filter(|l| l.pts.len() >= 2)
        .min_by(|a, b| nearest(&a.pts, mouth).2.total_cmp(&nearest(&b.pts, mouth).2))
        .filter(|l| nearest(&l.pts, mouth).2 < RADIUS - 4.0);
    Prepared { park, scene: Scene::new(obstacles), lane, mouth }
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
    if body_contact(v, &scene, &park, 0.0).is_some() {
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
    let search = |forward: bool, cusp: f64, collision: Option<f64>, weight: f64, max_nodes: usize| {
        Search {
            v,
            profile: ctx.profile,
            scene: &scene,
            margin: ctx.margin,
            costs: Costs { forward: FORWARD, reverse: REVERSE, cusp, switch: 0.3, margin: 8.0, collision },
            first_reverse: true,
            forward,
            weight,
            centre: mouth,
            radius: RADIUS,
            max_nodes,
        }
        .run(park, goal, h)
    };
    // D'un seul tenant (entrée sans reprise), puis avec reprises. Seule l'exploration complète du
    // domaine sans solution conclut à l'impossibilité ; un budget épuisé laisse la question ouverte.
    let (status, mut reason, mut found) = match search(false, 5.0, None, 1.2, NODES_DIRECT.min(ctx.max_nodes)) {
        Outcome::Found(plan) => (Status::Feasible, None, Some(plan)),
        _ => match search(true, 12.0, None, 1.2, ctx.max_nodes) {
            Outcome::Found(plan) => (Status::Feasible, None, Some(plan)),
            Outcome::Exhausted => (Status::Infeasible, Some("aucune manœuvre dans le domaine exploré"), None),
            Outcome::Budget => (Status::Unknown, Some("budget de recherche épuisé"), None),
        },
    };
    let mut collisions = 0;
    if found.is_none() && illustrate {
        if let Outcome::Found(plan) = search(true, 5.0, Some(5.0), 2.0, NODES_CONTACT) {
            collisions = plan.collisions;
            found = Some(plan);
        }
    }
    let Some(plan) = found else {
        reason = reason.or(Some("aucune manœuvre trouvée"));
        return Tested { park, scene, entry: Vec::new(), exit: Vec::new(), status, collisions: 0, reason };
    };
    Tested { park, entry: reversed(&plan.pieces), exit: plan.pieces, status, collisions, scene, reason }
}
