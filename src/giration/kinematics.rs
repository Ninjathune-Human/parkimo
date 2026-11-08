//! Cinématique à basse vitesse : modèle bicyclette sans glissement, volant à vitesse finie.
//!
//! La pose est celle du milieu de l'essieu arrière ; le cap se mesure depuis +x dans le sens
//! trigonométrique ; un angle de braquage positif fait tourner à gauche.

use super::vehicle::Vehicle;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub x: f64,
    pub y: f64,
    pub heading: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub pose: Pose,
    /// Angle bicyclette (rad), > 0 à gauche.
    pub steer: f64,
}

/// Profil de conducteur. Vitesse de braquage des roues selon l'échelle du CERTU (Giration 3.3,
/// reprise par Mensura) ; braquage à l'arrêt aux seuls rebroussements, comme AutoTURN et Mensura.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Profile {
    /// 90 % de l'angle, 6°/s, jamais de braquage à l'arrêt.
    Prudent,
    /// Tout l'angle, 10°/s, braquage à l'arrêt aux rebroussements.
    Courant,
    /// Tout l'angle, 15°/s (limite admissible), braquage à l'arrêt aux rebroussements.
    Habile,
}

impl Profile {
    pub(crate) fn steer_limit(self, v: &Vehicle) -> f64 {
        match self {
            Profile::Prudent => 0.9 * v.max_steer,
            Profile::Courant | Profile::Habile => v.max_steer,
        }
    }
    /// Vitesse de rotation des roues (rad/s), plafonnée par la direction du véhicule.
    pub(crate) fn steer_rate(self, v: &Vehicle) -> f64 {
        let deg: f64 = match self {
            Profile::Prudent => 6.0,
            Profile::Courant => 10.0,
            Profile::Habile => 15.0,
        };
        deg.to_radians().min(2.0 * v.max_steer / v.lock_to_lock)
    }
    /// Braquage à l'arrêt permis aux rebroussements.
    pub(crate) fn steers_at_cusps(self) -> bool {
        self != Profile::Prudent
    }
}

/// Avance de `ds` (négatif en marche arrière) à angle de roues constant, sur un arc exact.
pub fn advance(v: &Vehicle, s: State, ds: f64) -> State {
    let k = s.steer.tan() / v.wheelbase;
    let Pose { x, y, heading } = s.pose;
    let pose = if k.abs() < 1e-12 {
        Pose {
            x: x + ds * heading.cos(),
            y: y + ds * heading.sin(),
            heading,
        }
    } else {
        let h = heading + k * ds;
        Pose {
            x: x + (h.sin() - heading.sin()) / k,
            y: y - (h.cos() - heading.cos()) / k,
            heading: h,
        }
    };
    State {
        pose,
        steer: s.steer,
    }
}

/// Parcourt `distance` (négative en marche arrière) à la vitesse `speed` (m/s, > 0) en tournant
/// le volant vers `target`, borné par le profil. Renvoie l'état de départ puis un état par pas.
pub fn drive(
    v: &Vehicle,
    p: Profile,
    start: State,
    speed: f64,
    distance: f64,
    target: f64,
    step: f64,
) -> Vec<State> {
    let limit = p.steer_limit(v);
    let target = target.clamp(-limit, limit);
    let n = (distance.abs() / step).ceil().max(1.0) as usize;
    let ds = distance / n as f64;
    let max_change = p.steer_rate(v) / speed * ds.abs();
    let mut out = Vec::with_capacity(n + 1);
    let mut s = start;
    out.push(s);
    for _ in 0..n {
        let gap = target - s.steer;
        let next = if gap.abs() <= max_change {
            target
        } else {
            s.steer + max_change * gap.signum()
        };
        // Arc à l'angle moyen du pas, puis angle de fin de pas.
        let mid = advance(
            v,
            State {
                pose: s.pose,
                steer: 0.5 * (s.steer + next),
            },
            ds,
        );
        s = State {
            pose: mid.pose,
            steer: next,
        };
        out.push(s);
    }
    out
}

/// Braquage à l'arrêt : nouvel état et durée, ou `None` si le profil l'interdit ici.
pub fn steer_in_place(
    v: &Vehicle,
    p: Profile,
    s: State,
    target: f64,
    at_cusp: bool,
) -> Option<(State, f64)> {
    if !(at_cusp && p.steers_at_cusps()) {
        return None;
    }
    let limit = p.steer_limit(v);
    let target = target.clamp(-limit, limit);
    let time = (target - s.steer).abs() / p.steer_rate(v);
    Some((
        State {
            pose: s.pose,
            steer: target,
        },
        time,
    ))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wheel {
    pub centre: [f64; 2],
    pub heading: f64,
}

/// Roues dans l'ordre : avant gauche, avant droite, arrière gauche, arrière droite.
/// Les roues avant suivent la géométrie d'Ackermann.
pub fn wheels(v: &Vehicle, s: &State) -> [Wheel; 4] {
    let Pose { x, y, heading } = s.pose;
    let (c, n) = (
        (heading.cos(), heading.sin()),
        (-heading.sin(), heading.cos()),
    );
    let half = v.track / 2.0;
    let at = |fwd: f64, left: f64| [x + fwd * c.0 + left * n.0, y + fwd * c.1 + left * n.1];
    let (left, right) = if s.steer == 0.0 {
        (0.0, 0.0)
    } else {
        let (inner, outer) = v.ackermann(s.steer.abs());
        if s.steer > 0.0 {
            (inner, outer)
        } else {
            (-outer, -inner)
        }
    };
    [
        Wheel {
            centre: at(v.wheelbase, half),
            heading: heading + left,
        },
        Wheel {
            centre: at(v.wheelbase, -half),
            heading: heading + right,
        },
        Wheel {
            centre: at(0.0, half),
            heading,
        },
        Wheel {
            centre: at(0.0, -half),
            heading,
        },
    ]
}

/// Coins de caisse : avant gauche, avant droit, arrière droit, arrière gauche.
pub fn body_corners(v: &Vehicle, p: &Pose) -> [[f64; 2]; 4] {
    let (c, n) = (
        (p.heading.cos(), p.heading.sin()),
        (-p.heading.sin(), p.heading.cos()),
    );
    let half = v.width / 2.0;
    let at = |fwd: f64, left: f64| [p.x + fwd * c.0 + left * n.0, p.y + fwd * c.1 + left * n.1];
    let front = v.wheelbase + v.front_overhang;
    [
        at(front, half),
        at(front, -half),
        at(-v.rear_overhang, -half),
        at(-v.rear_overhang, half),
    ]
}
