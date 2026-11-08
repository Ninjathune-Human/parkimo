//! Planificateur de manœuvre : A* hybride dans l'espace (x, y, cap) du milieu de l'essieu arrière.
//!
//! Les primitives sont des arcs exacts de longueur fixe, en marche avant ou arrière, à cinq
//! courbures (butée gauche, mi-gauche, droit, mi-droite, butée droite). Un changement de courbure
//! se fait à l'arrêt (conducteur habile) ; le suiveur de trajectoire rejoue ensuite la manœuvre
//! avec le profil de conducteur choisi.

use super::kinematics::{advance, Pose, Profile, State};
use super::scene::Scene;
use super::sweep::{margin_contact, swept_contact, tyre_on_low_kerb};
use super::vehicle::Vehicle;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

/// Longueur d'une primitive (m).
pub const STEP: f64 = 0.5;
/// Maille du plan (m) et nombre de caps.
const CELL: f64 = 0.25;
const BINS: f64 = 72.0;
/// Pas de contrôle des collisions le long d'une primitive (m).
const CHECK: f64 = 0.25;
/// Pas de la trajectoire restituée (m).
const DENSE: f64 = 0.1;

pub struct Costs {
    /// Coût par mètre en marche avant et en marche arrière.
    pub forward: f64,
    pub reverse: f64,
    /// Rebroussement, changement de courbure (braquage à l'arrêt), marge entamée.
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
    /// Premier mouvement imposé en marche arrière (sortie d'une place prise en marche avant).
    pub first_reverse: bool,
    /// Marche avant permise (sinon : manœuvre d'un seul tenant, en marche arrière).
    pub forward: bool,
    /// Poids de l'estimation (1 : coût minimal, au-delà : recherche plus directe).
    pub weight: f64,
    /// Centre et rayon de la zone explorée.
    pub centre: [f64; 2],
    pub radius: f64,
    pub max_nodes: usize,
}

/// Un morceau de manœuvre à sens et courbure constants, échantillonné tous les 0,10 m.
#[derive(Clone, Debug)]
pub struct Piece {
    pub reverse: bool,
    /// Indice de courbure, de −2 (butée droite) à 2 (butée gauche), et angle de roues (rad).
    pub k: i8,
    pub steer: f64,
    pub poses: Vec<Pose>,
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
    pose: Pose,
    dir: i8,
    k: i8,
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

/// Case de la recherche, dans le repère de la pose de départ : deux places de même voisinage
/// donnent la même recherche, à un déplacement près.
fn key(p: &Pose, o: &Pose, dir: i8) -> (i32, i32, i32, i8) {
    let t = std::f64::consts::TAU;
    let (c, s) = (o.heading.cos(), o.heading.sin());
    let (dx, dy) = (p.x - o.x, p.y - o.y);
    let (x, y) = (dx * c + dy * s, -dx * s + dy * c);
    let b = (((p.heading - o.heading).rem_euclid(t) / t * BINS).round() as i32) % BINS as i32;
    ((x / CELL).floor() as i32, (y / CELL).floor() as i32, b, dir)
}

impl Search<'_> {
    fn steer(&self, k: i8) -> f64 {
        let kmax = self.profile.steer_limit(self.v).tan() / self.v.wheelbase;
        (kmax * k as f64 / 2.0 * self.v.wheelbase).atan()
    }

    /// Fin de la primitive, contacts rencontrés et marge entamée ; `None` si elle sort de la zone
    /// ou heurte un obstacle alors que le contact est interdit.
    fn primitive(&self, from: &Pose, dir: i8, k: i8) -> Option<(Pose, usize, bool)> {
        let steer = self.steer(k);
        let (mut hits, mut grazed) = (0, false);
        let n = (STEP / CHECK).round() as usize;
        let mut s = State { pose: *from, steer };
        for _ in 0..n {
            // Contrôle continu sur chaque tronçon : rien ne passe entre deux poses.
            let ds = dir as f64 * CHECK;
            let next = advance(self.v, s, ds);
            let p = next.pose;
            if (p.x - self.centre[0]).hypot(p.y - self.centre[1]) > self.radius {
                return None;
            }
            if swept_contact(self.v, self.scene, &s, ds, 0.0).is_some() || tyre_on_low_kerb(self.v, self.scene, &p) {
                self.costs.collision?;
                hits += 1;
            } else if self.margin > 0.0 && !grazed {
                // La marge n'est qu'un coût ici : contrôle aux poses ; le rejeu la contrôle en continu.
                grazed = margin_contact(self.v, self.scene, &p, self.margin);
            }
            s = next;
        }
        Some((s.pose, hits, grazed))
    }

    /// Manœuvre de `start` jusqu'à une pose qui satisfait `goal`, guidée par l'estimation `h`.
    pub fn run(&self, start: Pose, goal: impl Fn(&Pose) -> bool, h: impl Fn(&Pose) -> f64) -> Outcome {
        let mut nodes = vec![Node { pose: start, dir: 0, k: 0, g: 0.0, hits: 0, parent: usize::MAX }];
        let mut best: HashMap<(i32, i32, i32, i8), f64> = HashMap::new();
        let mut open = BinaryHeap::new();
        open.push(Open { f: h(&start), order: 0, node: 0 });
        let mut order = 1;
        while let Some(Open { node, .. }) = open.pop() {
            let (pose, dir, k, g, hits) = {
                let n = &nodes[node];
                (n.pose, n.dir, n.k, n.g, n.hits)
            };
            if node > 0 && goal(&pose) {
                return Outcome::Found(self.trace(&nodes, node));
            }
            if nodes.len() > self.max_nodes {
                return Outcome::Budget;
            }
            for nd in [1i8, -1] {
                if nd == 1 && (!self.forward || (node == 0 && self.first_reverse)) {
                    continue;
                }
                for nk in -2i8..=2 {
                    let Some((p, nh, grazed)) = self.primitive(&pose, nd, nk) else { continue };
                    let c = &self.costs;
                    let mut ng = g + STEP * if nd > 0 { c.forward } else { c.reverse };
                    if dir != 0 && dir != nd {
                        ng += c.cusp;
                    }
                    // Au départ, roues droites.
                    if nk != k {
                        ng += c.switch * (nk - k).abs() as f64 / 2.0;
                    }
                    if grazed {
                        ng += c.margin;
                    }
                    if let Some(pen) = c.collision {
                        ng += pen * nh as f64;
                    }
                    let kk = key(&p, &start, nd);
                    if best.get(&kk).is_some_and(|&b| b <= ng) {
                        continue;
                    }
                    best.insert(kk, ng);
                    nodes.push(Node { pose: p, dir: nd, k: nk, g: ng, hits: hits + nh, parent: node });
                    open.push(Open { f: ng + self.weight * h(&p), order, node: nodes.len() - 1 });
                    order += 1;
                }
            }
        }
        Outcome::Exhausted
    }

    fn trace(&self, nodes: &[Node], last: usize) -> Plan {
        let mut chain = Vec::new();
        let mut i = last;
        while nodes[i].parent != usize::MAX {
            chain.push(i);
            i = nodes[i].parent;
        }
        chain.reverse();
        let mut pieces: Vec<Piece> = Vec::new();
        let mut pose = nodes[0].pose;
        for &i in &chain {
            let (dir, k) = (nodes[i].dir, nodes[i].k);
            if pieces.last().is_none_or(|p| p.reverse != (dir < 0) || p.k != k) {
                pieces.push(Piece { reverse: dir < 0, k, steer: self.steer(k), poses: vec![pose] });
            }
            let steer = self.steer(k);
            let n = (STEP / DENSE).round() as usize;
            let piece = pieces.last_mut().unwrap();
            let mut s = State { pose, steer };
            for _ in 0..n {
                s = advance(self.v, s, dir as f64 * DENSE);
                piece.poses.push(s.pose);
            }
            pose = s.pose;
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
