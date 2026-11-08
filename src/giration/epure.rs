//! Épure manuelle : le véhicule suit une polyligne tracée, tronçon par tronçon.
//!
//! En marche avant, le milieu de l'essieu avant est guidé (loi de Stanley) ; en marche arrière,
//! le milieu de l'essieu arrière (poursuite d'un point situé un peu plus loin sur la polyligne).
//! Le volant reste dans les limites de braquage et de vitesse de volant du profil.

use super::kinematics::{advance, Pose, Profile, State};
use super::vehicle::Vehicle;
use crate::model::P2;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Section {
    pub pts: Vec<P2>,
    #[serde(default)]
    pub reverse: bool,
    pub speed_kmh: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub state: State,
    pub section: usize,
    /// Distance parcourue depuis le départ (m, toujours croissante).
    pub s: f64,
    /// Temps écoulé (s), braquages à l'arrêt compris.
    pub t: f64,
}

#[derive(Debug)]
pub struct Run {
    pub samples: Vec<Sample>,
    /// Écart maximal entre le point guidé et sa polyligne, par tronçon (m).
    pub deviation: Vec<f64>,
    /// Le point guidé a-t-il atteint le bout de la polyligne ?
    pub reached: Vec<bool>,
    /// Temps passé à braquer à l'arrêt (s).
    pub stationary_time: f64,
}

/// Distance de visée en marche arrière (m).
const LOOKAHEAD: f64 = 1.0;
/// Tolérance d'arrivée au bout de la polyligne (m).
const ARRIVAL: f64 = 0.05;

struct Polyline {
    pts: Vec<P2>,
    cum: Vec<f64>,
}

impl Polyline {
    fn new(pts: &[P2]) -> Polyline {
        let mut cum = vec![0.0];
        for w in pts.windows(2) {
            cum.push(cum.last().unwrap() + (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]));
        }
        Polyline { pts: pts.to_vec(), cum }
    }
    fn len(&self) -> f64 {
        *self.cum.last().unwrap()
    }
    /// Abscisse et distance du point de la polyligne le plus proche de `p`.
    fn project(&self, p: P2) -> (f64, f64) {
        let mut best = (0.0, f64::INFINITY);
        for i in 0..self.pts.len() - 1 {
            let (a, b) = (self.pts[i], self.pts[i + 1]);
            let d = [b[0] - a[0], b[1] - a[1]];
            let l2 = (d[0] * d[0] + d[1] * d[1]).max(1e-18);
            let t = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / l2).clamp(0.0, 1.0);
            let q = [a[0] + t * d[0], a[1] + t * d[1]];
            let dist = (p[0] - q[0]).hypot(p[1] - q[1]);
            if dist < best.1 {
                best = (self.cum[i] + t * (self.cum[i + 1] - self.cum[i]), dist);
            }
        }
        best
    }
    fn at(&self, s: f64) -> P2 {
        let s = s.clamp(0.0, self.len());
        let i = self.cum.partition_point(|&c| c <= s).clamp(1, self.pts.len() - 1) - 1;
        let seg = (self.cum[i + 1] - self.cum[i]).max(1e-18);
        let t = (s - self.cum[i]) / seg;
        let (a, b) = (self.pts[i], self.pts[i + 1]);
        [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]
    }
}

fn guided(v: &Vehicle, pose: &Pose, reverse: bool) -> P2 {
    let e = if reverse { 0.0 } else { v.wheelbase };
    [pose.x + e * pose.heading.cos(), pose.y + e * pose.heading.sin()]
}

fn wrap(a: f64) -> f64 {
    let t = std::f64::consts::TAU;
    (a + std::f64::consts::PI).rem_euclid(t) - std::f64::consts::PI
}

/// Gain latéral du suivi en marche avant (1/s) et vitesse d'amortissement (m/s).
const GAIN: f64 = 2.0;
const SOFT: f64 = 0.5;

/// Angle de roues souhaité au point de la polyligne d'abscisse `sp`.
fn aim(v: &Vehicle, pose: &Pose, reverse: bool, line: &Polyline, sp: f64, speed: f64) -> f64 {
    let g = guided(v, pose, reverse);
    if reverse {
        // Poursuite de cible sur l'essieu arrière : sa courbure de marche vaut 2 sin α / d,
        // et en marche arrière cette courbure vaut −tan δ / E.
        let target = line.at(sp + LOOKAHEAD);
        let bearing = (target[1] - g[1]).atan2(target[0] - g[0]);
        let alpha = wrap(bearing - (pose.heading + std::f64::consts::PI));
        let d = (target[0] - g[0]).hypot(target[1] - g[1]).max(1e-6);
        (-v.wheelbase * 2.0 * alpha.sin() / d).atan()
    } else {
        // Loi de Stanley sur l'essieu avant : écart de cap à la tangente, plus correction de
        // l'écart latéral ; exacte en régime permanent sur un cercle.
        let (a, b) = (line.at(sp - 0.5), line.at(sp + 0.5));
        let tangent = (b[1] - a[1]).atan2(b[0] - a[0]);
        let q = line.at(sp);
        let left = -(g[0] - q[0]) * tangent.sin() + (g[1] - q[1]) * tangent.cos();
        wrap(tangent - pose.heading) - (GAIN * left / (speed + SOFT)).atan()
    }
}

pub fn follow(v: &Vehicle, p: Profile, sections: &[Section], step: f64) -> Run {
    let limit = p.steer_limit(v);
    let mut run = Run { samples: Vec::new(), deviation: Vec::new(), reached: Vec::new(), stationary_time: 0.0 };
    let (mut s_total, mut t_total) = (0.0, 0.0);
    let mut state: Option<State> = None;

    for (k, sec) in sections.iter().enumerate() {
        if sec.pts.len() < 2 {
            run.deviation.push(0.0);
            run.reached.push(false);
            continue;
        }
        let line = Polyline::new(&sec.pts);
        let speed = (sec.speed_kmh / 3.6).max(0.1);
        let sign = if sec.reverse { -1.0 } else { 1.0 };
        let mut s = match state {
            Some(prev) => {
                // Rebroussement ou reprise : braquage à l'arrêt si le profil le permet.
                let cusp = sections[k - 1].reverse != sec.reverse;
                let (sp, _) = line.project(guided(v, &prev.pose, sec.reverse));
                let want = aim(v, &prev.pose, sec.reverse, &line, sp, speed).clamp(-limit, limit);
                match super::kinematics::steer_in_place(v, p, prev, want, cusp) {
                    Some((turned, dt)) => {
                        run.stationary_time += dt;
                        t_total += dt;
                        turned
                    }
                    None => prev,
                }
            }
            None => {
                // Départ : véhicule aligné sur le premier segment, point guidé sur le premier point.
                let (a, b) = (sec.pts[0], sec.pts[1]);
                let travel = (b[1] - a[1]).atan2(b[0] - a[0]);
                let heading = if sec.reverse { travel + std::f64::consts::PI } else { travel };
                let e = if sec.reverse { 0.0 } else { v.wheelbase };
                let pose = Pose { x: a[0] - e * heading.cos(), y: a[1] - e * heading.sin(), heading };
                State { pose, steer: 0.0 }
            }
        };
        run.samples.push(Sample { state: s, section: k, s: s_total, t: t_total });

        let max_change = p.steer_rate(v) / speed * step;
        let budget = 2.0 * line.len() + 10.0;
        let (mut travelled, mut worst, mut reached) = (0.0, 0.0f64, false);
        while travelled < budget {
            let g = guided(v, &s.pose, sec.reverse);
            let (sp, dev) = line.project(g);
            worst = worst.max(dev);
            if sp >= line.len() - ARRIVAL {
                reached = true;
                break;
            }
            let want = aim(v, &s.pose, sec.reverse, &line, sp, speed).clamp(-limit, limit);
            let gap = want - s.steer;
            let next = if gap.abs() <= max_change { want } else { s.steer + max_change * gap.signum() };
            let mid = advance(v, State { pose: s.pose, steer: 0.5 * (s.steer + next) }, sign * step);
            s = State { pose: mid.pose, steer: next };
            travelled += step;
            s_total += step;
            t_total += step / speed;
            run.samples.push(Sample { state: s, section: k, s: s_total, t: t_total });
        }
        run.deviation.push(worst);
        run.reached.push(reached);
        state = Some(s);
    }
    run
}
