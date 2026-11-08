//! Planificateur de manœuvre : A* hybride dans l'espace (x, y, cap, angle des roues) du milieu de
//! l'essieu arrière.
//!
//! Comme AutoTURN et Mensura, le volant tourne en roulant, à la vitesse du profil de conducteur :
//! chaque primitive tourne les roues vers l'une de cinq consignes (butée gauche, mi-gauche, droit,
//! mi-droite, butée droite) sans dépasser cette vitesse, d'où des raccords en clothoïde. Les roues
//! ne tournent à l'arrêt qu'aux rebroussements et au départ, voiture garée, si le profil le permet.

use super::kinematics::{advance, Pose, Profile, State};
use super::scene::Scene;
use super::sweep::{margin_contact_among, near_body, swept_contact_among, tyre_on_low_kerb_among};
use super::vehicle::Vehicle;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

/// Longueur d'une primitive (m).
pub const STEP: f64 = 0.5;
/// Maille du plan (m) et nombre de caps.
const CELL: f64 = 0.25;
const BINS: f64 = 72.0;
/// Pas d'intégration et de contrôle des collisions (m) : arc exact à l'angle moyen du pas, comme
/// `kinematics::drive`.
const CHECK: f64 = 0.25;
/// Cases d'angle de roues de la recherche, par demi-plage (de droit à la butée).
const STEER_BINS: f64 = 6.0;

pub struct Costs {
    /// Coût par mètre en marche avant et en marche arrière.
    pub forward: f64,
    pub reverse: f64,
    /// Rebroussement ; mouvement du volant, par demi-butée ; marge entamée.
    pub cusp: f64,
    pub switch: f64,
    pub margin: f64,
    /// Contact toléré à ce prix (recherche de la moins mauvaise manœuvre), sinon interdit.
    pub collision: Option<f64>,
}

pub struct Search<'a> {
    pub v: &'a Vehicle,
    pub profile: Profile,
    pub scene: &'a Scene,
    pub margin: f64,
    pub costs: Costs,
    /// Vitesse de manœuvre (m/s) : avec la vitesse de braquage, elle fixe la longueur des raccords.
    pub speed: f64,
    /// Premier mouvement imposé en marche arrière (sortie d'une place prise en marche avant).
    pub first_reverse: bool,
    /// Marche avant permise (sinon : manœuvre d'un seul tenant, en marche arrière).
    pub forward: bool,
    /// Rebroussements permis au plus.
    pub max_cusps: usize,
    /// Poids de l'estimation (1 : coût minimal, au-delà : recherche plus directe).
    pub weight: f64,
    /// Centre et rayon de la zone explorée.
    pub centre: [f64; 2],
    pub radius: f64,
    pub max_nodes: usize,
}

/// Un morceau de manœuvre de même sens de marche, échantillonné tous les 0,25 m. Le premier état
/// porte l'angle de roues après un éventuel braquage à l'arrêt (rebroussement, départ).
#[derive(Clone, Debug)]
pub struct Piece {
    pub reverse: bool,
    pub states: Vec<State>,
}

/// Issue d'une recherche : manœuvre trouvée, domaine entièrement exploré sans solution, ou budget
/// épuisé avant la fin (sans conclusion).
pub enum Outcome {
    Found(Plan),
    Exhausted,
    Budget,
}

pub struct Plan {
    pub pieces: Vec<Piece>,
    pub cost: f64,
    /// Contacts tolérés en route (recherche de la moins mauvaise manœuvre).
    pub collisions: usize,
}

struct Node {
    state: State,
    dir: i8,
    cusps: u8,
    /// Consigne de la primitive qui a mené ici.
    target: i8,
    g: f64,
    hits: usize,
    parent: usize,
}

#[derive(PartialEq)]
struct Open {
    f: f64,
    order: usize,
    node: usize,
}
impl Eq for Open {}
impl Ord for Open {
    fn cmp(&self, o: &Self) -> Ordering {
        o.f.total_cmp(&self.f).then(o.order.cmp(&self.order))
    }
}
impl PartialOrd for Open {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

type Key = (i32, i32, i32, i8, i8, u8);

/// Case de la recherche, dans le repère de la pose de départ : deux places de même voisinage
/// donnent la même recherche, à un déplacement près.
fn key(s: &State, o: &Pose, dir: i8, steer_cell: f64, cusps: u8) -> Key {
    let t = std::f64::consts::TAU;
    let p = &s.pose;
    let (c, sn) = (o.heading.cos(), o.heading.sin());
    let (dx, dy) = (p.x - o.x, p.y - o.y);
    let (x, y) = (dx * c + dy * sn, -dx * sn + dy * c);
    let b = (((p.heading - o.heading).rem_euclid(t) / t * BINS).round() as i32) % BINS as i32;
    ((x / CELL).floor() as i32, (y / CELL).floor() as i32, b, dir, (s.steer / steer_cell).round() as i8, cusps)
}

/// Primitive jouée : état final, braquage à l'arrêt au départ et en roulant (rad), contacts,
/// marge entamée.
struct Roll {
    end: State,
    jump: f64,
    turned: f64,
    hits: usize,
    grazed: bool,
}

impl Search<'_> {
    fn limit(&self) -> f64 {
        self.profile.steer_limit(self.v)
    }
    /// Angle de roues de la consigne `k` (−2 à 2) : courbure proportionnelle.
    fn steer(&self, k: i8) -> f64 {
        let kmax = self.limit().tan() / self.v.wheelbase;
        (kmax * k as f64 / 2.0 * self.v.wheelbase).atan()
    }

    /// Joue `STEP` m dans le sens `nd` en tournant les roues vers la consigne `k`. Voiture à l'arrêt
    /// (départ ou rebroussement) : braquage à l'arrêt si le profil le permet. `None` si la primitive
    /// sort de la zone ou heurte un obstacle alors que le contact est interdit. `out` reçoit les états.
    fn roll(&self, from: &State, dir: i8, nd: i8, k: i8, mut out: Option<&mut Vec<State>>) -> Option<Roll> {
        let target = self.steer(k);
        let stopped = dir != nd;
        let start = if stopped && self.profile.steers_at_cusps() { target } else { from.steer };
        let max = self.profile.steer_rate(self.v) / self.speed * CHECK;
        let (mut hits, mut grazed) = (0, false);
        // Obstacles proches, cherchés une fois : en 0,5 m, aucun point de la caisse ne se déplace
        // de plus de 1 m.
        let cands = near_body(self.v, self.scene, &from.pose, 2.0 * STEP + self.margin);
        let mut s = State { pose: from.pose, steer: start };
        if let Some(o) = out.as_deref_mut() {
            o.push(s);
        }
        let n = (STEP / CHECK).round() as usize;
        for _ in 0..n {
            let gap = target - s.steer;
            let next = if gap.abs() <= max { target } else { s.steer + max * gap.signum() };
            // Arc exact à l'angle moyen du pas : contrôle continu de la caisse sur cet arc.
            let mid = State { pose: s.pose, steer: 0.5 * (s.steer + next) };
            let ds = nd as f64 * CHECK;
            let p = advance(self.v, mid, ds).pose;
            if (p.x - self.centre[0]).hypot(p.y - self.centre[1]) > self.radius {
                return None;
            }
            if swept_contact_among(self.v, self.scene, &cands, &mid, ds, 0.0).is_some() || tyre_on_low_kerb_among(self.v, self.scene, &cands, &p) {
                self.costs.collision?;
                hits += 1;
            } else if self.margin > 0.0 && !grazed {
                // La marge n'est qu'un coût ici : contrôle aux poses ; le rejeu la contrôle en continu.
                grazed = margin_contact_among(self.v, self.scene, &cands, &p, self.margin);
            }
            s = State { pose: p, steer: next };
            if let Some(o) = out.as_deref_mut() {
                o.push(s);
            }
        }
        Some(Roll { end: s, jump: (start - from.steer).abs(), turned: (s.steer - start).abs(), hits, grazed })
    }

    /// Manœuvre de `start` (roues droites, voiture arrêtée) jusqu'à une pose qui satisfait `goal`,
    /// guidée par l'estimation `h`. À l'arrivée, les roues sont presque droites, sauf si la voiture
    /// s'y arrête pour repartir en sens inverse en braquant à l'arrêt.
    pub fn run(&self, start: Pose, goal: impl Fn(&Pose) -> bool, h: impl Fn(&Pose) -> f64) -> Outcome {
        let cell = self.limit() / STEER_BINS;
        let origin = State { pose: start, steer: 0.0 };
        let mut nodes = vec![Node { state: origin, dir: 0, cusps: 0, target: 0, g: 0.0, hits: 0, parent: usize::MAX }];
        let mut best: HashMap<Key, f64> = HashMap::new();
        let mut open = BinaryHeap::new();
        open.push(Open { f: h(&start), order: 0, node: 0 });
        let mut order = 1;
        while let Some(Open { node, .. }) = open.pop() {
            let (state, dir, cusps, g, hits) = {
                let n = &nodes[node];
                (n.state, n.dir, n.cusps, n.g, n.hits)
            };
            let settled = (dir > 0 && self.profile.steers_at_cusps()) || state.steer.abs() <= cell + 1e-9;
            if node > 0 && settled && goal(&state.pose) {
                return Outcome::Found(self.trace(&nodes, node));
            }
            if nodes.len() > self.max_nodes {
                return Outcome::Budget;
            }
            for nd in [1i8, -1] {
                if nd == 1 && (!self.forward || (node == 0 && self.first_reverse)) {
                    continue;
                }
                let nc = cusps + u8::from(dir != 0 && dir != nd);
                if nc as usize > self.max_cusps {
                    continue;
                }
                for nk in -2i8..=2 {
                    let Some(r) = self.roll(&state, dir, nd, nk, None) else { continue };
                    let c = &self.costs;
                    let mut ng = g + STEP * if nd > 0 { c.forward } else { c.reverse };
                    if dir != 0 && dir != nd {
                        ng += c.cusp;
                    }
                    // Mouvement du volant, à l'arrêt et en roulant.
                    ng += c.switch * (r.jump + r.turned) / (self.limit() / 2.0);
                    if r.grazed {
                        ng += c.margin;
                    }
                    if let Some(pen) = c.collision {
                        ng += pen * r.hits as f64;
                    }
                    let kk = key(&r.end, &start, nd, cell, nc);
                    if best.get(&kk).is_some_and(|&b| b <= ng) {
                        continue;
                    }
                    best.insert(kk, ng);
                    nodes.push(Node { state: r.end, dir: nd, cusps: nc, target: nk, g: ng, hits: hits + r.hits, parent: node });
                    open.push(Open { f: ng + self.weight * h(&r.end.pose), order, node: nodes.len() - 1 });
                    order += 1;
                }
            }
        }
        Outcome::Exhausted
    }

    /// Manœuvre rejouée primitive par primitive, regroupée par sens de marche.
    fn trace(&self, nodes: &[Node], last: usize) -> Plan {
        let mut chain = Vec::new();
        let mut i = last;
        while nodes[i].parent != usize::MAX {
            chain.push(i);
            i = nodes[i].parent;
        }
        chain.reverse();
        let mut pieces: Vec<Piece> = Vec::new();
        let (mut state, mut dir) = (nodes[0].state, 0i8);
        for &i in &chain {
            let nd = nodes[i].dir;
            let mut states = Vec::new();
            let r = self.roll(&state, dir, nd, nodes[i].target, Some(&mut states)).expect("primitive rejouée");
            if nd == dir {
                pieces.last_mut().unwrap().states.extend(states.into_iter().skip(1));
            } else {
                pieces.push(Piece { reverse: nd < 0, states });
            }
            state = r.end;
            dir = nd;
        }
        Plan { pieces, cost: nodes[last].g, collisions: nodes[last].hits }
    }
}

/// Coût minimal, sans obstacle, pour rejoindre une file : table de l'écart latéral `l` du centre
/// du véhicule à la ligne visée et de son cap `t` relatif à la file, calculée une fois par profil
/// (Dijkstra depuis les poses d'arrivée, mouvements parcourus à rebours). L'abscisse le long de la
/// file ne compte pas : la table sert d'estimation à l'A* hybride.
pub struct Table {
    nl: usize,
    nt: usize,
    cost: Vec<f32>,
}

const T_DL: f64 = 0.1;
const T_LMAX: f64 = 14.0;
const T_NT: usize = 180;

impl Table {
    pub fn build(v: &Vehicle, profile: Profile, forward: f64, reverse: f64, lane_tol: f64, heading_tol: f64) -> Table {
        let nl = (2.0 * T_LMAX / T_DL).round() as usize + 1;
        let nt = T_NT;
        let dt = std::f64::consts::TAU / nt as f64;
        let mut cost = vec![f32::INFINITY; nl * nt];
        // État continu qui a donné le meilleur coût de chaque case (pas de dérive d'arrondi).
        let mut state = vec![(0.0f64, 0.0f64); nl * nt];
        let c = (v.wheelbase + v.front_overhang - v.rear_overhang) / 2.0;
        let kmax = profile.steer_limit(v).tan() / v.wheelbase;
        let mut heap = BinaryHeap::new();
        let mut order = 0;
        for il in 0..nl {
            for it in 0..nt {
                let (l, t) = (-T_LMAX + il as f64 * T_DL, it as f64 * dt);
                let tw = (t + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI;
                if l.abs() <= lane_tol && tw.abs() <= heading_tol {
                    cost[il * nt + it] = 0.0;
                    state[il * nt + it] = (l, t);
                    heap.push(Open { f: 0.0, order, node: il * nt + it });
                    order += 1;
                }
            }
        }
        while let Some(Open { f, node, .. }) = heap.pop() {
            if f > cost[node] as f64 + 1e-9 {
                continue;
            }
            let (l, t) = state[node];
            for d in [1.0f64, -1.0] {
                // Parcouru à rebours : un pas en avant ici est un pas en arrière dans la manœuvre.
                let w = STEP * if d > 0.0 { reverse } else { forward };
                for k in -2i32..=2 {
                    let kappa = kmax * k as f64 / 2.0;
                    let s = d * STEP;
                    let t2 = t + kappa * s;
                    let dl_rear = if kappa.abs() < 1e-12 { s * t.sin() } else { (t.cos() - t2.cos()) / kappa };
                    let l2 = l + dl_rear + c * (t2.sin() - t.sin());
                    if l2.abs() > T_LMAX {
                        continue;
                    }
                    let jl = ((l2 + T_LMAX) / T_DL).round() as usize;
                    let jt = ((t2.rem_euclid(std::f64::consts::TAU) / dt).round() as usize) % nt;
                    let j = jl * nt + jt;
                    let nf = f + w;
                    if nf < cost[j] as f64 - 1e-9 {
                        cost[j] = nf as f32;
                        state[j] = (l2, t2);
                        heap.push(Open { f: nf, order, node: j });
                        order += 1;
                    }
                }
            }
        }
        Table { nl, nt, cost }
    }

    /// Estimation du coût pour un écart latéral `l` (m, à gauche) et un cap relatif `t` (rad).
    pub fn get(&self, l: f64, t: f64) -> f64 {
        if l.abs() > T_LMAX {
            return l.abs() - T_LMAX + self.get(T_LMAX.copysign(l), t);
        }
        let il = (((l + T_LMAX) / T_DL).round() as usize).min(self.nl - 1);
        let it = ((t.rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU * self.nt as f64).round() as usize) % self.nt;
        let c = self.cost[il * self.nt + it];
        if c.is_finite() { c as f64 } else { 0.0 }
    }
}
