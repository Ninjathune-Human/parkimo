//! Trait libre : la trajectoire la plus proche d'un trait dessiné à main levée, faite comme un tracé
//! routier d'alignements droits et d'arcs de cercle tangents (un sommet et un rayon par virage). Elle
//! commence et finit par une droite, part du début du trait, et ses rayons restent au-dessus du rayon
//! mini du point guidé par l'épure. Avec les obstacles, une recherche locale déplace les sommets et
//! les rayons pour supprimer les contacts en s'écartant le moins possible du trait.

use super::epure::{follow, Section};
use super::kinematics::Profile;
use super::scene::Scene;
use super::sweep::{conflicts, Severity};
use super::vehicle::Vehicle;
use crate::model::P2;
use std::f64::consts::PI;

/// Pas de rééchantillonnage du trait et du tracé (m).
const DS: f64 = 0.25;
/// Demi-largeur du lissage du trait, en pas (± 0,75 m).
const SMOOTH: usize = 3;
/// Demi-largeur de l'estimation de la courbure, en pas (± 1 m).
const CURV: usize = 4;
/// Courbure en dessous de laquelle le trait est lu droit (rayon de 40 m).
const STRAIGHT_K: f64 = 1.0 / 40.0;
/// Partie de virage ou d'alignement plus courte que cela : rattachée à sa voisine (pas).
const SHORT: usize = 4;
/// Virage plus faible : ignoré.
const MIN_TURN: f64 = 3.0 * PI / 180.0;
/// Gain d'ajustement en dessous duquel un virage est retiré (m², soit 7 cm d'écart quadratique moyen).
const PRUNE: f64 = 0.005;
/// Un sommet ne porte pas plus de 120° : un demi-tour en prend deux.
const MAX_TURN: f64 = 2.0 * PI / 3.0;
/// Longueur mini des droites de départ et d'arrivée (m).
pub const MIN_END: f64 = 0.5;
const R_MAX: f64 = 500.0;
/// Pas d'intégration pendant la recherche d'évitement (m) ; le résultat est rejoué au pas de l'épure.
const SEARCH_STEP: f64 = 0.05;
/// Évaluations au plus : ajustement au trait, recherche d'évitement.
const FIT_BUDGET: usize = 2000;
/// Essai de retrait d'un virage : ajustement court avant de conclure.
const PRUNE_BUDGET: usize = 300;
const BUDGET: usize = 400;

/// Rayon mini du point guidé : milieu de l'essieu avant en marche avant, de l'essieu arrière en
/// marche arrière.
pub fn rmin(v: &Vehicle, p: Profile, reverse: bool) -> f64 {
    let r = v.wheelbase / p.steer_limit(v).tan();
    if reverse {
        r
    } else {
        r.hypot(v.wheelbase)
    }
}

/// Droite ou arc de cercle ; `delta` > 0 : virage à gauche.
#[derive(Clone, Debug)]
pub enum Elem {
    Line { a: P2, b: P2 },
    Arc { a: P2, b: P2, c: P2, r: f64, delta: f64 },
}

impl Elem {
    pub fn len(&self) -> f64 {
        match *self {
            Elem::Line { a, b } => dist(a, b),
            Elem::Arc { r, delta, .. } => r * delta.abs(),
        }
    }
    pub fn at(&self, t: f64) -> P2 {
        match *self {
            Elem::Line { a, b } => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t],
            Elem::Arc { a, c, r, delta, .. } => {
                let f = (a[1] - c[1]).atan2(a[0] - c[0]) + delta * t;
                [c[0] + r * f.cos(), c[1] + r * f.sin()]
            }
        }
    }
    pub fn mid(&self) -> P2 {
        self.at(0.5)
    }
    fn ends(&self) -> (P2, P2) {
        match *self {
            Elem::Line { a, b } | Elem::Arc { a, b, .. } => (a, b),
        }
    }
}

/// Départ fixe, sommets (point d'intersection des alignements, rayon voulu), arrivée. `side` : sens
/// de chaque virage lu sur le trait (+1 à gauche), que l'ajustement ne renverse pas.
#[derive(Clone, Debug)]
pub struct Path {
    start: P2,
    pis: Vec<(P2, f64)>,
    side: Vec<f64>,
    end: P2,
}

impl Path {
    pub fn start(&self) -> P2 {
        self.start
    }

    /// Droites et arcs, du départ à l'arrivée. Les tangentes d'un sommet sont réduites si les
    /// alignements sont trop courts pour son rayon (le rayon effectif baisse alors).
    pub fn elements(&self) -> Vec<Elem> {
        let pts: Vec<P2> = std::iter::once(self.start).chain(self.pis.iter().map(|p| p.0)).chain(std::iter::once(self.end)).collect();
        let n = self.pis.len();
        let mut turn = vec![0.0; n];
        let mut want = vec![0.0; n];
        for i in 0..n {
            let (u, w) = (unit(sub(pts[i + 1], pts[i])), unit(sub(pts[i + 2], pts[i + 1])));
            turn[i] = cross(u, w).atan2(dot(u, w));
            want[i] = self.pis[i].1 * (turn[i].abs() / 2.0).tan();
        }
        // Réduction par alignement : les deux tangentes qui s'y partagent la place.
        let mut scale = vec![1.0f64; n];
        for j in 0..=n {
            let cap = dist(pts[j], pts[j + 1]) - if j == 0 { MIN_END } else { 0.0 } - if j == n { MIN_END } else { 0.0 };
            let demand = if j > 0 { want[j - 1] } else { 0.0 } + if j < n { want[j] } else { 0.0 };
            if demand > 1e-12 && demand > cap {
                let k = (cap / demand).max(0.0);
                if j > 0 {
                    scale[j - 1] = scale[j - 1].min(k);
                }
                if j < n {
                    scale[j] = scale[j].min(k);
                }
            }
        }
        let mut out = Vec::new();
        let mut from = self.start;
        for i in 0..n {
            let t = want[i] * scale[i];
            let (u, w) = (unit(sub(pts[i + 1], pts[i])), unit(sub(pts[i + 2], pts[i + 1])));
            let (a, b) = (add(pts[i + 1], mul(u, -t)), add(pts[i + 1], mul(w, t)));
            out.push(Elem::Line { a: from, b: a });
            let half = (turn[i].abs() / 2.0).tan();
            if half > 1e-9 {
                let r = t / half; // nul faute de place : cassure, pénalisée par l'ajustement
                let c = add(a, mul([-u[1], u[0]], r * turn[i].signum()));
                out.push(Elem::Arc { a, b, c, r, delta: turn[i] });
            }
            from = b;
        }
        out.push(Elem::Line { a: from, b: self.end });
        out
    }

    /// Points tous les `step` mètres au plus (arcs compris), du départ à l'arrivée.
    pub fn sample(&self, step: f64) -> Vec<P2> {
        let mut out = vec![self.start];
        for e in self.elements() {
            let n = (e.len() / step).ceil() as usize;
            for k in 1..=n {
                out.push(e.at(k as f64 / n as f64));
            }
        }
        out
    }

    /// Points de l'interface : [x, y, k], k flèche / corde du segment qui part du point, positive à
    /// droite du sens de tracé (un virage à gauche a sa flèche à droite de la corde) ; 0 au dernier.
    pub fn points(&self) -> Vec<[f64; 3]> {
        let mut out: Vec<[f64; 3]> = Vec::new();
        for e in self.elements().iter().filter(|e| e.len() > 1e-6) {
            let (a, _) = e.ends();
            let k = match *e {
                Elem::Line { .. } => 0.0,
                Elem::Arc { delta, .. } => delta.signum() * (delta.abs() / 4.0).tan() / 2.0,
            };
            out.push([a[0], a[1], k]);
        }
        out.push([self.end[0], self.end[1], 0.0]);
        out
    }

    fn turns(&self) -> Vec<f64> {
        let pts: Vec<P2> = std::iter::once(self.start).chain(self.pis.iter().map(|p| p.0)).chain(std::iter::once(self.end)).collect();
        pts.windows(3).map(|w| {
            let (u, v) = (unit(sub(w[1], w[0])), unit(sub(w[2], w[1])));
            cross(u, v).atan2(dot(u, v))
        }).collect()
    }

    fn params(&self) -> Vec<f64> {
        let mut x: Vec<f64> = self.pis.iter().flat_map(|&(p, r)| [p[0], p[1], r]).collect();
        x.extend([self.end[0], self.end[1]]);
        x
    }

    fn with(&self, x: &[f64], rmin: f64) -> Path {
        let n = self.pis.len();
        let pis = (0..n).map(|i| ([x[3 * i], x[3 * i + 1]], x[3 * i + 2].clamp(rmin, R_MAX))).collect();
        Path { start: self.start, pis, side: self.side.clone(), end: [x[3 * n], x[3 * n + 1]] }
    }

    fn steps(&self) -> Vec<f64> {
        let mut s: Vec<f64> = self.pis.iter().flat_map(|&(_, r)| [1.0, 1.0, (0.2 * r).max(1.0)]).collect();
        s.extend([1.0, 1.0]);
        s
    }
}

/// Écart maximal du trait au tracé (m).
pub fn deviation(stroke: &[P2], path: &Path) -> f64 {
    let line = path.sample(DS);
    stroke.iter().map(|&q| to_polyline(q, &line)).fold(0.0, f64::max)
}

/// Tracé le plus proche du trait, rayons d'au moins `rmin`.
pub fn fit(stroke: &[P2], rmin: f64) -> Path {
    let s = resample(stroke, DS);
    let (start, end) = (stroke[0], *stroke.last().unwrap());
    let straight = Path { start, pis: Vec::new(), side: Vec::new(), end };
    if s.len() < 2 * SMOOTH + 3 {
        return straight;
    }
    let (pis, side) = initial(&smooth(&s, SMOOTH), rmin).into_iter().map(|(p, r, d)| ((p, r), d)).unzip();
    let first = Path { start, pis, side, end };
    let adjust_in = |p: &Path, budget: usize| {
        let x = search(&p.params(), &p.steps(), &mut |x: &[f64]| closeness(&s, &p.with(x, rmin), rmin), budget).0;
        p.with(&x, rmin)
    };
    let adjust = |p: &Path| adjust_in(p, FIT_BUDGET);
    let mut fitted = adjust(&first);
    // Un virage trop faible après l'ajustement n'en est pas un.
    let turns = fitted.turns();
    if turns.iter().any(|t| t.abs() < MIN_TURN) {
        let keep: Vec<usize> = (0..turns.len()).filter(|&i| turns[i].abs() >= MIN_TURN).collect();
        let p = Path { start, pis: keep.iter().map(|&i| fitted.pis[i]).collect(), side: keep.iter().map(|&i| fitted.side[i]).collect(), end: fitted.end };
        fitted = adjust(&p);
    }
    // Deux virages de même sens sans alignement entre eux n'en font qu'un (tremblement du trait).
    for _ in 0..8 {
        match merged(&fitted, rmin) {
            Some(p) => fitted = adjust(&p),
            None => break,
        }
    }
    // Un virage qui n'améliore pas l'ajustement de plus que le tremblement de la main est retiré,
    // le plus faible d'abord.
    'prune: for _ in 0..8 {
        let base = closeness(&s, &fitted, rmin);
        let turns = fitted.turns();
        let mut order: Vec<usize> = (0..turns.len()).collect();
        order.sort_by(|&a, &b| turns[a].abs().total_cmp(&turns[b].abs()));
        for i in order {
            let mut q = fitted.clone();
            q.pis.remove(i);
            q.side.remove(i);
            let q = adjust_in(&q, PRUNE_BUDGET);
            if closeness(&s, &q, rmin) <= base + PRUNE {
                fitted = adjust(&q);
                continue 'prune;
            }
        }
        break;
    }
    fitted
}

/// Premier couple de virages voisins de même sens séparés de moins d'un mètre, réunis en un seul
/// (120° au plus) : sommet à l'intersection des alignements extérieurs, même longueur d'arc.
fn merged(p: &Path, rmin: f64) -> Option<Path> {
    let arcs: Vec<(P2, P2, f64)> = p.elements().iter().filter_map(|e| if let Elem::Arc { a, b, .. } = e { Some((*a, *b, e.len())) } else { None }).collect();
    let turns = p.turns();
    if arcs.len() != p.pis.len() {
        return None;
    }
    let pts: Vec<P2> = std::iter::once(p.start).chain(p.pis.iter().map(|q| q.0)).chain(std::iter::once(p.end)).collect();
    for i in 0..p.pis.len().saturating_sub(1) {
        let gap = dist(arcs[i].1, arcs[i + 1].0);
        let total = turns[i] + turns[i + 1];
        if p.side[i] != p.side[i + 1] || gap >= 1.0 || total.abs() > MAX_TURN {
            continue;
        }
        let x = intersect((pts[i], unit(sub(pts[i + 1], pts[i]))), (pts[i + 2], unit(sub(pts[i + 3], pts[i + 2]))))?;
        let len = arcs[i].2.abs() + arcs[i + 1].2.abs() + gap;
        let mut q = p.clone();
        q.pis.splice(i..i + 2, [(x, (len / total.abs()).clamp(rmin, R_MAX))]);
        q.side.remove(i + 1);
        return Some(q);
    }
    None
}

/// Résultat du trait libre.
#[derive(Debug)]
pub struct Sketched {
    pub path: Path,
    pub contacts: usize,
    pub alerts: usize,
    /// Écart maximal du trait au tracé (m).
    pub deviation: f64,
    /// La recherche d'évitement a-t-elle tourné ?
    pub searched: bool,
    pub evals: usize,
}

/// Trait libre sur une scène : ajustement, puis, s'il y a contact et si `avoid`, recherche des
/// sommets et des rayons qui suppriment les contacts (puis les alertes de marge) au plus près du trait.
#[allow(clippy::too_many_arguments)]
pub fn sketch_on(v: &Vehicle, p: Profile, scene: &Scene, stroke: &[P2], reverse: bool, speed_kmh: f64, margin: f64, avoid: bool) -> Sketched {
    let r = rmin(v, p, reverse);
    let fitted = fit(stroke, r);
    let s = resample(stroke, DS);
    // Contacts (longueur cumulée, nombre, points) et alertes de marge (longueur, nombre).
    let check = |path: &Path, step: f64| {
        let sec = [Section { pts: path.sample(DS), reverse, speed_kmh }];
        let run = follow(v, p, &sec, step);
        let found = conflicts(v, scene, &run.samples, margin);
        let len = |sev: Severity| -> (f64, usize, Vec<P2>) {
            let k: Vec<_> = found.iter().filter(|c| c.severity == sev).collect();
            (k.iter().map(|c| c.s_end - c.s_start + 0.1).sum(), k.len(), k.iter().map(|c| c.point).collect())
        };
        (len(Severity::Conflict), len(Severity::Warning))
    };
    let mut path = fitted;
    let mut evals = 0;
    let mut searched = false;
    if avoid && check(&path, SEARCH_STEP).0 .1 > 0 {
        searched = true;
        let cost = |q: &Path, step: f64| {
            let ((c, _, _), (a, _, _)) = check(q, step);
            1e4 * c + 1e2 * a + closeness(&s, q, r)
        };
        // Recherche locale au pas grossier ; s'il reste des contacts au pas de l'épure, reprise au pas
        // de l'épure, les sommets voisins d'un contact d'abord écartés de lui (le pas de la recherche
        // ne suffit pas à quitter un contact ponctuel).
        let mut step = SEARCH_STEP;
        let mut best = (cost(&path, step), path.clone());
        let mut from = path.clone();
        for _ in 0..4 {
            let left = BUDGET.saturating_sub(evals);
            if left == 0 {
                break;
            }
            let (x, n) = search(&from.params(), &from.steps(), &mut |x: &[f64]| cost(&from.with(x, r), step), left);
            evals += n;
            let q = from.with(&x, r);
            let cq = cost(&q, step);
            if cq < best.0 {
                best = (cq, q);
            }
            let ((_, k, pts), _) = check(&best.1, super::api::STEP);
            if k == 0 {
                break;
            }
            if step != super::api::STEP {
                step = super::api::STEP;
                best.0 = cost(&best.1, step);
            }
            from = best.1.clone();
            for pi in from.pis.iter_mut() {
                for c in &pts {
                    if dist(pi.0, *c) < 10.0 {
                        pi.0 = add(pi.0, mul(unit(sub(pi.0, *c)), 0.75));
                    }
                }
            }
        }
        path = best.1;
    }
    let ((_, contacts, _), (_, alerts, _)) = check(&path, super::api::STEP);
    let deviation = deviation(stroke, &path);
    Sketched { path, contacts, alerts, deviation, searched, evals }
}

// ---------------------------------------------------------------------------

/// Tracé initial : le cap du trait lissé est lu en alignements droits et en virages ; chaque virage
/// donne un sommet à l'intersection des alignements qui l'encadrent (deux ou plus au-delà de 120°).
fn initial(p: &[P2], rmin: f64) -> Vec<(P2, f64, f64)> {
    let m = p.len() - 1; // caps
    let mut th = Vec::with_capacity(m);
    for i in 0..m {
        let a = (p[i + 1][1] - p[i][1]).atan2(p[i + 1][0] - p[i][0]);
        th.push(match th.last() {
            Some(&prev) => prev + wrap(a - prev),
            None => a,
        });
    }
    let class: Vec<i8> = (0..m)
        .map(|i| {
            let (lo, hi) = (i.saturating_sub(CURV), (i + CURV).min(m - 1));
            let k = (th[hi] - th[lo]) / ((hi - lo).max(1) as f64 * DS);
            if k.abs() < STRAIGHT_K {
                0
            } else {
                k.signum() as i8
            }
        })
        .collect();
    // Parties de même nature ; les trop courtes rejoignent la précédente.
    let mut runs: Vec<(usize, usize, i8)> = Vec::new();
    for (i, &c) in class.iter().enumerate() {
        match runs.last_mut() {
            Some(r) if r.2 == c => r.1 = i + 1,
            _ => runs.push((i, i + 1, c)),
        }
    }
    let mut changed = true;
    while changed && runs.len() > 1 {
        changed = false;
        if let Some(k) = (0..runs.len()).filter(|&k| runs[k].1 - runs[k].0 < SHORT).min_by_key(|&k| runs[k].1 - runs[k].0) {
            let j = if k == 0 { 1 } else { k - 1 };
            let (lo, hi) = (runs[k].0.min(runs[j].0), runs[k].1.max(runs[j].1));
            let c = runs[j].2;
            runs[j.min(k)] = (lo, hi, c);
            runs.remove(j.max(k));
            let mut merged: Vec<(usize, usize, i8)> = Vec::new();
            for r in runs.drain(..) {
                match merged.last_mut() {
                    Some(q) if q.2 == r.2 => q.1 = r.1,
                    _ => merged.push(r),
                }
            }
            runs = merged;
            changed = true;
        }
    }
    // Alignement de chaque partie droite (moindres carrés), tangente au trait ailleurs.
    let line_of = |a: usize, b: usize| -> (P2, P2) {
        let pts = &p[a..=b];
        let n = pts.len() as f64;
        let c = [pts.iter().map(|q| q[0]).sum::<f64>() / n, pts.iter().map(|q| q[1]).sum::<f64>() / n];
        let (mut sxx, mut sxy, mut syy) = (0.0, 0.0, 0.0);
        for q in pts {
            let (dx, dy) = (q[0] - c[0], q[1] - c[1]);
            sxx += dx * dx;
            sxy += dx * dy;
            syy += dy * dy;
        }
        let ang = 0.5 * (2.0 * sxy).atan2(sxx - syy);
        let mut d = [ang.cos(), ang.sin()];
        if dot(d, sub(p[b], p[a])) < 0.0 {
            d = [-d[0], -d[1]];
        }
        (c, d)
    };
    let tangent = |i: usize| -> (P2, P2) {
        let k = i.min(m - 1);
        (p[i], [th[k].cos(), th[k].sin()])
    };
    let mut pis = Vec::new();
    for (k, &(a, b, c)) in runs.iter().enumerate() {
        if c == 0 {
            continue;
        }
        let before = match k {
            0 => tangent(0),
            _ if runs[k - 1].2 == 0 => {
                let l = line_of(runs[k - 1].0, runs[k - 1].1);
                if k == 1 {
                    (p[0], l.1)
                } else {
                    l
                }
            }
            _ => tangent(a),
        };
        let after = match runs.get(k + 1) {
            None => tangent(m),
            Some(r) if r.2 == 0 => {
                let l = line_of(r.0, r.1);
                if k + 2 == runs.len() {
                    (p[m], l.1)
                } else {
                    l
                }
            }
            Some(_) => tangent(b),
        };
        let total = cross(before.1, after.1).atan2(dot(before.1, after.1));
        let total = total + 2.0 * PI * ((th[b.min(m - 1)] - th[a] - total) / (2.0 * PI)).round();
        if total.abs() < MIN_TURN {
            continue;
        }
        // Demi-tour plus serré que deux rayons mini entre ses deux branches : raquette, un
        // contre-virage, le demi-tour élargi, un contre-virage (rayon mini partout).
        let side = total.signum();
        let n = [-before.1[1] * side, before.1[0] * side];
        let w = dot(sub(after.0, before.0), n).max(0.0);
        if (total.abs() - PI).abs() < PI / 6.0 && w < 2.0 * rmin {
            let r = rmin;
            let reach = (a..=b.min(m)).map(|i| dot(sub(p[i], before.0), before.1)).fold(f64::MIN, f64::max);
            let along = (4.0 * r * r - (w / 2.0 + r).powi(2)).sqrt();
            let x1 = reach - r - along;
            let back = if k == 0 || runs[k - 1].2 != 0 { 0.0 } else { dot(sub(p[runs[k - 1].0], before.0), before.1) };
            if x1 > back + MIN_END {
                let alpha = along.atan2(w / 2.0 + r);
                let big = side * (PI + 2.0 * alpha);
                let mut turns = vec![-side * alpha];
                turns.extend([big / 3.0; 3]);
                turns.push(-side * alpha);
                let (mut at, mut h) = (add(before.0, mul(before.1, x1)), before.1);
                for d in turns {
                    let t = r * (d.abs() / 2.0).tan();
                    let pi = add(at, mul(h, t));
                    h = [h[0] * d.cos() - h[1] * d.sin(), h[0] * d.sin() + h[1] * d.cos()];
                    at = add(pi, mul(h, t));
                    pis.push((pi, r, d.signum()));
                }
                continue;
            }
        }
        // Lignes d'appui : avant, intermédiaires (au-delà de 120°), après.
        let parts = (total.abs() / MAX_TURN).ceil() as usize;
        let mut lines = vec![before];
        for j in 1..parts {
            lines.push(tangent(a + (b - a) * j / parts));
        }
        lines.push(after);
        let len = (b - a) as f64 * DS / parts as f64;
        for w in lines.windows(2) {
            let turn = cross(w[0].1, w[1].1).atan2(dot(w[0].1, w[1].1));
            if turn.abs() < MIN_TURN {
                continue;
            }
            if let Some(x) = intersect(w[0], w[1]) {
                pis.push((x, (len / turn.abs()).clamp(rmin, R_MAX), turn.signum()));
            }
        }
    }
    pis
}

/// Proximité du trait et du tracé : moyenne des carrés des écarts dans les deux sens, arrivée,
/// pénalités exactes (linéaires) : rayon sous le mini, droites de départ ou d'arrivée trop courtes,
/// virage renversé.
fn closeness(s: &[P2], path: &Path, rmin: f64) -> f64 {
    let line = path.sample(2.0 * DS);
    let a = along(s, &line);
    let b = along(&line, s);
    let e = dist(path.end, *s.last().unwrap()).powi(2);
    let el = path.elements();
    let mut pen = 0.0;
    for x in &el {
        if let Elem::Arc { r, .. } = x {
            pen += (rmin - r).max(0.0);
        }
    }
    pen += (MIN_END - el[0].len()).max(0.0) + (MIN_END - el[el.len() - 1].len()).max(0.0);
    pen += path.turns().iter().zip(&path.side).map(|(t, d)| (-t * d).max(0.0)).sum::<f64>();
    a + b + e + 1e2 * pen
}

/// Recherche directe de Hooke et Jeeves : explorations coordonnée par coordonnée, mouvements de
/// motif, pas divisé par deux jusqu'à 2 cm. Renvoie le point trouvé et le nombre d'évaluations.
fn search(x0: &[f64], steps: &[f64], f: &mut dyn FnMut(&[f64]) -> f64, budget: usize) -> (Vec<f64>, usize) {
    let mut c = Counted { f, n: 0 };
    let mut x = x0.to_vec();
    let mut fx = c.eval(&x);
    let mut scale = 1.0;
    while scale > 0.02 && c.n < budget {
        let (y, fy) = explore(&mut c, steps, &x, fx, scale);
        if fy < fx {
            let (mut prev, mut cur, mut fcur) = (x, y, fy);
            while c.n < budget {
                let z: Vec<f64> = cur.iter().zip(&prev).map(|(a, b)| 2.0 * a - b).collect();
                let fz = c.eval(&z);
                let (w, fw) = explore(&mut c, steps, &z, fz, scale);
                if fw >= fcur {
                    break;
                }
                prev = std::mem::replace(&mut cur, w);
                fcur = fw;
            }
            x = cur;
            fx = fcur;
        } else {
            scale *= 0.5;
        }
    }
    (x, c.n)
}

struct Counted<'a> {
    f: &'a mut dyn FnMut(&[f64]) -> f64,
    n: usize,
}

impl Counted<'_> {
    fn eval(&mut self, x: &[f64]) -> f64 {
        self.n += 1;
        (self.f)(x)
    }
}

fn explore(c: &mut Counted, steps: &[f64], base: &[f64], fb: f64, scale: f64) -> (Vec<f64>, f64) {
    let mut y = base.to_vec();
    let mut fy = fb;
    for j in 0..y.len() {
        let old = y[j];
        for dir in [1.0, -1.0] {
            y[j] = old + dir * steps[j] * scale;
            let v = c.eval(&y);
            if v < fy {
                fy = v;
                break;
            }
            y[j] = old;
        }
    }
    (y, fy)
}

fn resample(stroke: &[P2], step: f64) -> Vec<P2> {
    let mut out = vec![stroke[0]];
    let mut carry = 0.0;
    for w in stroke.windows(2) {
        let l = dist(w[0], w[1]);
        if l < 1e-12 {
            continue;
        }
        let mut t = step - carry;
        while t <= l + 1e-12 {
            out.push([w[0][0] + (w[1][0] - w[0][0]) * t / l, w[0][1] + (w[1][1] - w[0][1]) * t / l]);
            t += step;
        }
        carry = l - (t - step);
    }
    let last = *stroke.last().unwrap();
    if dist(*out.last().unwrap(), last) > 1e-6 {
        out.push(last);
    }
    out
}

/// Moyenne glissante, fenêtre réduite près des bouts (bouts fixes).
fn smooth(p: &[P2], half: usize) -> Vec<P2> {
    let n = p.len();
    (0..n)
        .map(|i| {
            let h = half.min(i).min(n - 1 - i);
            let w = &p[i - h..=i + h];
            let k = w.len() as f64;
            [w.iter().map(|q| q[0]).sum::<f64>() / k, w.iter().map(|q| q[1]).sum::<f64>() / k]
        })
        .collect()
}

fn intersect(l1: (P2, P2), l2: (P2, P2)) -> Option<P2> {
    let d = cross(l1.1, l2.1);
    if d.abs() < 1e-9 {
        return None;
    }
    let t = cross(sub(l2.0, l1.0), l2.1) / d;
    Some(add(l1.0, mul(l1.1, t)))
}

/// Moyenne des carrés des distances des points `pts` à la polyligne `line`, parcourues dans le même
/// sens : chaque point cherche le segment le plus proche autour de celui du point précédent (le trait
/// et le tracé avancent ensemble, et un aller-retour ne se confond pas avec l'aller).
fn along(pts: &[P2], line: &[P2]) -> f64 {
    if line.len() < 2 {
        return pts.iter().map(|&q| dist(q, line[0]).powi(2)).sum::<f64>() / pts.len() as f64;
    }
    let segs = line.len() - 1;
    let ratio = segs as f64 / pts.len().max(1) as f64;
    let (back, ahead) = (8, 24 + ratio.ceil() as usize);
    let mut j = 0usize;
    let mut sum = 0.0;
    for &q in pts {
        let (lo, hi) = (j.saturating_sub(back), (j + ahead).min(segs));
        let mut best = (f64::INFINITY, j);
        for k in lo..hi {
            let d = to_segment(q, line[k], line[k + 1]);
            if d < best.0 {
                best = (d, k);
            }
        }
        j = best.1;
        sum += best.0 * best.0;
    }
    sum / pts.len() as f64
}

fn to_polyline(q: P2, line: &[P2]) -> f64 {
    if line.len() == 1 {
        return dist(q, line[0]);
    }
    line.windows(2).map(|w| to_segment(q, w[0], w[1])).fold(f64::INFINITY, f64::min)
}

fn to_segment(q: P2, a: P2, b: P2) -> f64 {
    let d = sub(b, a);
    let l2 = dot(d, d);
    let t = if l2 < 1e-18 { 0.0 } else { (dot(sub(q, a), d) / l2).clamp(0.0, 1.0) };
    dist(q, add(a, mul(d, t)))
}

fn wrap(a: f64) -> f64 {
    (a + PI).rem_euclid(2.0 * PI) - PI
}
fn dist(a: P2, b: P2) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
fn sub(a: P2, b: P2) -> P2 {
    [a[0] - b[0], a[1] - b[1]]
}
fn add(a: P2, b: P2) -> P2 {
    [a[0] + b[0], a[1] + b[1]]
}
fn mul(a: P2, k: f64) -> P2 {
    [a[0] * k, a[1] * k]
}
fn dot(a: P2, b: P2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn cross(a: P2, b: P2) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn unit(a: P2) -> P2 {
    let l = a[0].hypot(a[1]);
    if l < 1e-12 {
        [1.0, 0.0]
    } else {
        [a[0] / l, a[1] / l]
    }
}
