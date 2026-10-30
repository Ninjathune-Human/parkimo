//! Rangées doubles du cœur : optimisation orientation × décalage, puis découpe exacte
//! (îlots plantés alignés sur les deux demi-rangées, îlots d'extrémité, allées transversales).

use crate::geom::{rotate_rings, strip_intervals, v2, Quad, V2};
use crate::layout::{Acc, Stall, KIND_STD};
use crate::model::Params;

pub struct RowsOut {
    pub angle: f64,
    pub phase: f64,
    pub evals: u32,
    /// Bandes des rangées doubles (repère monde), pour le calcul de la voirie.
    pub strips: Vec<Quad>,
    /// Allées resserrées (ordonnées dans le repère des rangées, tourné de `angle`).
    pub narrowed: Vec<Narrowed>,
}

struct Frame {
    rings: Vec<Vec<V2>>,
    ylo: f64,
    yhi: f64,
    xlo: f64,
    xhi: f64,
    /// Voies à sens unique parallèles aux rangées dans ce repère : ordonnée, indice.
    narrow: Vec<(f64, usize)>,
}

/// Voie à sens unique désignée dans le cœur : un point de son axe et sa direction.
pub type Narrow = (V2, V2);

fn frame(core: &[Vec<V2>], deg: f64, narrow: &[Narrow]) -> Frame {
    let t = -deg.to_radians();
    let (c, s) = (t.cos(), t.sin());
    let narrow = narrow
        .iter()
        .enumerate()
        .filter(|(_, (_, d))| d.rot(c, s).y.abs() < 20f64.to_radians().sin() * d.len())
        .map(|(i, (p, _))| (p.rot(c, s).y, i))
        .collect();
    let rings = rotate_rings(core, c, s);
    let (mut ylo, mut yhi, mut xlo, mut xhi) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for r in &rings {
        for p in r {
            ylo = ylo.min(p.y);
            yhi = yhi.max(p.y);
            xlo = xlo.min(p.x);
            xhi = xhi.max(p.x);
        }
    }
    Frame {
        rings,
        ylo,
        yhi,
        xlo,
        xhi,
        narrow,
    }
}

/// Allée resserrée : indice de la voie désignée, milieu de l'allée sur la trame régulière et
/// milieu effectif (ordonnées dans le repère des rangées).
pub type Narrowed = (usize, f64, f64);

/// Départs des rangées doubles, de période `2 d + allée`. Une allée de la trame régulière qui
/// porte une voie à sens unique est ramenée à `aisle_one` : les rangées de part et d'autre s'en
/// rapprochent chacune de la moitié de l'écart, son milieu ne bouge pas. Renvoie le nombre de
/// voies resserrées.
fn row_starts(
    f: &Frame,
    p: &Params,
    phase: f64,
    out: &mut Vec<f64>,
    mut hits: Option<&mut Vec<Narrowed>>,
) -> usize {
    let d2 = p.double_depth();
    let per = d2 + p.aisle;
    let one = p.aisle_one.min(p.aisle);
    let mut j0 = ((f.ylo - d2 - phase) / per).floor() as i64;
    let mut j1 = ((f.yhi - phase) / per).ceil() as i64;
    out.clear();
    if one >= p.aisle - 1e-6 || f.narrow.is_empty() {
        out.extend((j0..=j1).map(|j| phase + j as f64 * per));
        return 0;
    }
    // Rangées en réserve : en se rapprochant, les rangées libèrent de la place aux extrémités.
    let half = (p.aisle - one) * 0.5;
    let extra = ((half * f.narrow.len() as f64) / per).ceil() as i64 + 1;
    j0 -= extra;
    j1 += extra;
    let n = (j1 - j0 + 1) as usize;
    let mut tight = vec![false; n];
    let mut landed = 0;
    for &(y, i) in &f.narrow {
        let k = ((y - d2 - phase) / per).floor() as i64;
        let g = phase + k as f64 * per + d2;
        if k >= j0 && k < j1 && y >= g && y < g + p.aisle {
            tight[(k - j0) as usize] = true;
            landed += 1;
            if let Some(h) = hits.as_deref_mut() {
                h.push((i, g + p.aisle * 0.5, 0.0));
            }
        }
    }
    let total = tight.iter().filter(|&&t| t).count() as f64;
    let mut below = 0.0;
    for (k, &t) in tight.iter().enumerate() {
        // Au-dessus de `below` allées resserrées, sous les autres.
        let y = phase + (j0 + k as i64) as f64 * per + half * (total - 2.0 * below);
        out.push(y);
        if t {
            below += 1.0;
        }
    }
    if let Some(h) = hits {
        for e in h.iter_mut() {
            let k = ((e.1 - d2 - phase) / per).floor() as i64;
            e.2 = out[(k - j0) as usize] + d2 + one * 0.5;
        }
    }
    landed
}

fn aisle_slots(p: &Params) -> i64 {
    ((p.aisle + 2.0 * p.cap) / p.stall_w - 1e-9).ceil() as i64
}

/// Estimation rapide du nombre de places d'une demi-rangée.
fn quick_count(iv: &[(f64, f64)], p: &Params) -> f64 {
    let w = p.stall_w;
    let k = p.planter_every as f64;
    let mut total = 0.0;
    for &(a, b) in iv {
        let len = b - a - 2.0 * p.cap;
        if len < 2.0 * w {
            continue;
        }
        let mut n = (len / w + 1e-9).floor();
        if p.max_run > 0.0 && n * w > p.max_run {
            let m = (n * w / p.max_run).ceil() - 1.0;
            n -= m * aisle_slots(p) as f64;
        }
        if k > 0.0 {
            n -= (n / (k + 1.0)).floor();
        }
        total += n.max(0.0);
    }
    total
}

fn score(f: &Frame, p: &Params, phase: f64, buf: &mut Vec<(f64, f64)>, ys: &mut Vec<f64>) -> f64 {
    let d = p.stall_d;
    row_starts(f, p, phase, ys, None);
    let mut total = 0.0;
    for &y0 in ys.iter() {
        strip_intervals(&f.rings, y0 + 1e-4, y0 + d - 1e-4, buf);
        total += quick_count(buf, p);
        let y1 = y0 + d + p.noue();
        strip_intervals(&f.rings, y1 + 1e-4, y1 + d - 1e-4, buf);
        total += quick_count(buf, p);
    }
    total
}

fn norm_deg(a: f64) -> f64 {
    a.rem_euclid(180.0)
}

/// Orientations de référence : directions des longues arêtes de B et leurs perpendiculaires.
fn edge_angles(b_rings: &[Vec<V2>]) -> Vec<f64> {
    let mut v: Vec<(f64, f64)> = Vec::new();
    for r in b_rings {
        let n = r.len();
        for i in 0..n {
            let e = r[(i + 1) % n] - r[i];
            let l = e.len();
            if l >= 10.0 {
                let a = norm_deg(e.y.atan2(e.x).to_degrees());
                v.push((a, l));
            }
        }
    }
    v.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap());
    let mut out: Vec<f64> = Vec::new();
    for (a, _) in v.into_iter().take(12) {
        for c in [a, norm_deg(a + 90.0)] {
            if !out.iter().any(|&o| ang_dist(o, c) < 0.5) {
                out.push(c);
            }
        }
    }
    out
}

fn ang_dist(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(180.0);
    d.min(180.0 - d)
}

pub fn optimize(core: &[Vec<V2>], b_rings: &[Vec<V2>], p: &Params) -> (f64, f64, u32) {
    let per = p.double_depth() + p.aisle;
    let aligned = edge_angles(b_rings);
    let bonus = |a: f64| {
        if aligned.iter().any(|&e| ang_dist(e, a) < 0.3) {
            0.4
        } else {
            0.0
        }
    };
    let mut buf = Vec::new();
    let mut ys = Vec::new();
    let mut evals = 0u32;

    let mut angles: Vec<f64> = Vec::new();
    if let Some(a) = p.angle {
        angles.push(norm_deg(a));
    } else if p.quick && p.angle_hint.is_some() {
        let h = p.angle_hint.unwrap();
        for k in -4..=4 {
            angles.push(norm_deg(h + k as f64));
        }
        angles.extend(aligned.iter().take(6));
    } else {
        angles.extend(aligned.iter());
        let mut a = 0.0;
        while a < 180.0 {
            angles.push(a);
            a += 3.0;
        }
    }

    let phase_step = if p.quick { 0.5 } else { 0.25 };
    let phases: Vec<f64> = match p.phase {
        Some(ph) => vec![ph.rem_euclid(per)],
        None => (0..((per / phase_step).ceil() as usize))
            .map(|i| i as f64 * phase_step)
            .collect(),
    };

    let mut ranked: Vec<(f64, f64, f64)> = Vec::new(); // (score, angle, phase)
    for &a in &angles {
        let f = frame(core, a, &[]);
        let mut best = (f64::MIN, 0.0);
        for &ph in &phases {
            let s = score(&f, p, ph, &mut buf, &mut ys);
            evals += 1;
            if s > best.0 {
                best = (s, ph);
            }
        }
        ranked.push((best.0 + bonus(a), a, best.1));
    }
    ranked.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());

    // Affinage autour des meilleures orientations (sauf orientation imposée).
    let mut best = ranked[0];
    if p.angle.is_none() && !p.quick {
        let tops: Vec<(f64, f64, f64)> = ranked.iter().take(4).copied().collect();
        for (_, a0, ph0) in tops {
            for da in [-1.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5] {
                let a = norm_deg(a0 + da);
                let f = frame(core, a, &[]);
                for k in -4..=4 {
                    let ph = if p.phase.is_some() {
                        ph0
                    } else {
                        (ph0 + k as f64 * 0.1).rem_euclid(per)
                    };
                    let s = score(&f, p, ph, &mut buf, &mut ys) + bonus(a);
                    evals += 1;
                    if s > best.0 + 1e-9 {
                        best = (s, a, ph);
                    }
                }
            }
        }
    }
    (best.1, best.2, evals)
}

/// Découpe exacte d'une rangée double. Renvoie le nombre de places posées.
fn pack_double_row(f: &Frame, p: &Params, y0: f64, rot: (f64, f64), acc: &mut Acc) -> usize {
    let (w, d, cap) = (p.stall_w, p.stall_d, p.cap);
    let nw = p.noue();
    let mut halves: [Vec<(f64, f64)>; 2] = [Vec::new(), Vec::new()];
    strip_intervals(&f.rings, y0 + 1e-4, y0 + d - 1e-4, &mut halves[0]);
    strip_intervals(&f.rings, y0 + d + nw + 1e-4, y0 + 2.0 * d + nw - 1e-4, &mut halves[1]);
    for h in halves.iter_mut() {
        *h = h
            .iter()
            .map(|&(a, b)| (a + cap, b - cap))
            .filter(|&(a, b)| b - a >= 2.0 * w - 1e-6)
            .collect();
    }
    if halves[0].is_empty() && halves[1].is_empty() {
        return 0;
    }

    // Validité des emplacements pour un décalage g.
    let valid = |g: f64| -> (i64, [Vec<bool>; 2]) {
        let kmin = ((f.xlo - g) / w).floor() as i64 - 1;
        let kmax = ((f.xhi - g) / w).ceil() as i64 + 1;
        let n = (kmax - kmin + 1) as usize;
        let mut v = [vec![false; n], vec![false; n]];
        for h in 0..2 {
            for &(a, b) in &halves[h] {
                let k0 = ((a - g) / w - 1e-9).ceil() as i64;
                let k1 = ((b - g) / w + 1e-9).floor() as i64 - 1;
                for k in k0..=k1 {
                    let i = (k - kmin) as usize;
                    if i < n {
                        v[h][i] = true;
                    }
                }
            }
        }
        (kmin, v)
    };

    let mut best_g = 0.0;
    let mut best_n = -1i64;
    for j in 0..8 {
        let g = j as f64 * w / 8.0;
        let (kmin, v) = valid(g);
        let cnt =
            (v[0].iter().filter(|&&ok| ok).count() + v[1].iter().filter(|&&ok| ok).count()) as i64;
        let _ = kmin;
        if cnt > best_n {
            best_n = cnt;
            best_g = g;
        }
    }
    let g = best_g;
    let (kmin, mut v) = valid(g);
    let n = v[0].len();

    // Allées transversales sur les longues rangées.
    let mut aisle = vec![false; n];
    if p.max_run > 0.0 {
        let sa = aisle_slots(p) as usize;
        let mut i = 0;
        while i < n {
            if !(v[0][i] || v[1][i]) {
                i += 1;
                continue;
            }
            let start = i;
            while i < n && (v[0][i] || v[1][i]) {
                i += 1;
            }
            let len = i - start;
            if len as f64 * w > p.max_run {
                let m = ((len as f64 * w) / p.max_run).ceil() as usize - 1;
                for t in 1..=m {
                    let c = start + (t * len) / (m + 1);
                    let a0 = c.saturating_sub(sa / 2).max(start);
                    for k in a0..(a0 + sa).min(i) {
                        aisle[k] = true;
                    }
                }
            }
        }
        for i in 0..n {
            if aisle[i] {
                v[0][i] = false;
                v[1][i] = false;
            }
        }
        // Emprise des allées transversales.
        let mut i = 0;
        while i < n {
            if !aisle[i] {
                i += 1;
                continue;
            }
            let s0 = i;
            while i < n && aisle[i] {
                i += 1;
            }
            let x0 = g + (kmin + s0 as i64) as f64 * w + cap;
            let x1 = g + (kmin + i as i64) as f64 * w - cap;
            let q = Quad([
                v2(x0, y0),
                v2(x1, y0),
                v2(x1, y0 + 2.0 * d + nw),
                v2(x0, y0 + 2.0 * d + nw),
            ]);
            acc.cross.push(xf(&q, rot));
        }
    }

    // Îlots plantés répartis régulièrement dans chaque rangée, alignés sur les deux demi-rangées.
    let mut planter = vec![false; n];
    if p.planter_every > 0 {
        let per_run = p.planter_every as usize + 1;
        let mut i = 0;
        while i < n {
            if !(v[0][i] || v[1][i]) {
                i += 1;
                continue;
            }
            let s0 = i;
            while i < n && (v[0][i] || v[1][i]) {
                i += 1;
            }
            let len = i - s0;
            let m = len / per_run;
            for t in 1..=m {
                planter[s0 + (t * len) / (m + 1)] = true;
            }
        }
    }

    // Noue entre les deux demi-rangées : un tronçon par suite continue de places.
    // Intervalles (repère des rangées) des noues supprimées : la demi-rangée opposée s'y recale contre l'autre.
    let mut off_x: Vec<(f64, f64)> = Vec::new();
    if nw > 0.0 {
        let mut i = 0;
        while i < n {
            if !(v[0][i] || v[1][i]) {
                i += 1;
                continue;
            }
            let s0 = i;
            while i < n && (v[0][i] || v[1][i]) {
                i += 1;
            }
            let xa = g + (kmin + s0 as i64) as f64 * w;
            let xb = g + (kmin + i as i64) as f64 * w;
            // La noue s'arrête là où le cœur s'arrête : elle ne déborde jamais sur la chaussée.
            let mut inside = Vec::new();
            strip_intervals(&f.rings, y0 + d + 1e-4, y0 + d + nw - 1e-4, &mut inside);
            for (a, b) in inside {
                let (lo, hi) = (a.max(xa), b.min(xb));
                if hi - lo >= 1.0 {
                    let q = Quad([v2(lo, y0 + d), v2(hi, y0 + d), v2(hi, y0 + d + nw), v2(lo, y0 + d + nw)]);
                    let wq = xf(&q, rot);
                    let m = |a: usize, b: usize| [(wq.0[a].x + wq.0[b].x) / 2.0, (wq.0[a].y + wq.0[b].y) / 2.0];
                    let (a, b) = (m(0, 3), m(1, 2));
                    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                    let l2 = (dx * dx + dy * dy).max(1e-9);
                    if acc.noue_off.iter().any(|p| {
                        let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0);
                        (p[0] - a[0] - dx * t).hypot(p[1] - a[1] - dy * t) < 1.5
                    }) {
                        off_x.push((lo, hi));
                    }
                    acc.noues.push(wq);
                }
            }
        }
    }

    let mut placed = 0usize;
    for h in 0..2 {
        let row = acc.new_row();
        let (yb, yf) = if h == 0 {
            (y0 + d, y0)
        } else {
            (y0 + d + nw, y0 + 2.0 * d + nw)
        };
        let mut i = 0;
        while i < n {
            if !v[h][i] {
                i += 1;
                continue;
            }
            let s0 = i;
            while i < n && v[h][i] {
                i += 1;
            }
            let stalls_in_run = (s0..i).filter(|&k| !planter[k]).count();
            if stalls_in_run < 2 {
                continue;
            }
            let xa = g + (kmin + s0 as i64) as f64 * w;
            let xb = g + (kmin + i as i64) as f64 * w;
            // Emprise par tronçon : une demi-rangée recalée sur une noue supprimée a sa propre emprise.
            let is_off = |k: usize| {
                let xc = g + (kmin + k as i64) as f64 * w + w / 2.0;
                h == 1 && off_x.iter().any(|&(lo, hi)| xc >= lo && xc <= hi)
            };
            let mut k0 = s0;
            while k0 < i {
                let o = is_off(k0);
                let mut k1 = k0;
                while k1 < i && is_off(k1) == o {
                    k1 += 1;
                }
                let (ya, yz) = if o { (y0 + d, y0 + 2.0 * d) } else { (yb, yf) };
                let (xs, xe) = (g + (kmin + k0 as i64) as f64 * w, g + (kmin + k1 as i64) as f64 * w);
                let pad = Quad([v2(xs, ya), v2(xe, ya), v2(xe, yz), v2(xs, yz)]);
                acc.pads.push(xf(&pad, rot));
                acc.blockers.push(xf(&pad, rot));
                if o {
                    let v = Quad([v2(xs, y0 + 2.0 * d), v2(xe, y0 + 2.0 * d), v2(xe, y0 + 2.0 * d + nw), v2(xs, y0 + 2.0 * d + nw)]);
                    acc.voids.push(xf(&v, rot));
                }
                k0 = k1;
            }
            if cap > 0.05 {
                for (c0, c1) in [(xa - cap, xa), (xb, xb + cap)] {
                    let q = Quad([v2(c0, yb), v2(c1, yb), v2(c1, yf), v2(c0, yf)]);
                    let wq = xf(&q, rot);
                    acc.islands.push(wq);
                    acc.blockers.push(wq);
                    acc.trees.push(wq.center());
                }
            }
            for k in s0..i {
                let x = g + (kmin + k as i64) as f64 * w;
                let (yb, yf) = if h == 1 && off_x.iter().any(|&(lo, hi)| x + w / 2.0 >= lo && x + w / 2.0 <= hi) {
                    (y0 + d, y0 + 2.0 * d)
                } else {
                    (yb, yf)
                };
                let q = if h == 0 {
                    Quad([v2(x, yb), v2(x + w, yb), v2(x + w, yf), v2(x, yf)])
                } else {
                    Quad([v2(x + w, yb), v2(x, yb), v2(x, yf), v2(x + w, yf)])
                };
                let wq = xf(&q, rot);
                if planter[k] {
                    acc.islands.push(wq);
                    acc.trees.push(wq.center());
                } else {
                    acc.hash.insert(wq);
                    acc.stalls.push(Stall {
                        q: wq,
                        kind: KIND_STD,
                        row,
                        idx: k as i32,
                        perimeter: false,
                        equipped: false,
                    });
                    placed += 1;
                }
            }
        }
    }
    placed
}

fn xf(q: &Quad, rot: (f64, f64)) -> Quad {
    Quad([
        q.0[0].rot(rot.0, rot.1),
        q.0[1].rot(rot.0, rot.1),
        q.0[2].rot(rot.0, rot.1),
        q.0[3].rot(rot.0, rot.1),
    ])
}

/// Côté de l'anneau parallèle aux rangées passé à sens unique : point de la limite avancée du
/// cœur et normale vers l'anneau.
pub type Pin = (V2, V2);

/// Écart à appliquer aux rangées pour que la plus proche du côté calé l'affleure.
fn flush(ys: &[f64], pin: (f64, bool), d2: f64) -> f64 {
    let (t, above) = pin;
    ys.iter()
        .map(|&y| if above { t - (y + d2) } else { t - y })
        .min_by(|a, b| a.abs().partial_cmp(&b.abs()).unwrap())
        .unwrap_or(0.0)
}

/// Rangées doubles posées sur la trame `grid` (orientation, décalage, évaluations) issue de
/// `optimize`, avec les allées resserrées des voies à sens unique.
pub fn place(
    core: &[Vec<V2>],
    grid: (f64, f64, u32),
    p: &Params,
    narrow: &[Narrow],
    pin: Option<Pin>,
    acc: &mut Acc,
) -> RowsOut {
    let (angle, phase0, evals) = grid;
    if core.is_empty() {
        return RowsOut {
            angle,
            phase: phase0,
            evals,
            strips: vec![],
            narrowed: vec![],
        };
    }
    let mut f = frame(core, angle, narrow);
    let t = angle.to_radians();
    let rot = (t.cos(), t.sin());
    let d2 = p.double_depth();
    let mut ys = Vec::new();
    let mut narrowed = Vec::new();
    // Côté de l'anneau calé : la trame glisse pour que la rangée la plus proche l'affleure.
    let pin = pin.map(|(e, n)| (e.rot(t.cos(), -t.sin()).y, n.rot(t.cos(), -t.sin()).y > 0.0));
    let mut phase = phase0;
    if let Some((t, above)) = pin {
        // Rangées candidates (les plus proches du côté) : on retient la meilleure estimation.
        let saved = std::mem::take(&mut f.narrow);
        row_starts(&f, p, phase0, &mut ys, None);
        f.narrow = saved;
        let per = d2 + p.aisle;
        let mut cands: Vec<f64> = ys
            .iter()
            .map(|&y| if above { t - (y + d2) } else { t - y })
            .filter(|dl| dl.abs() <= per)
            .collect();
        cands.sort_by(|a, b| a.abs().partial_cmp(&b.abs()).unwrap());
        let mut buf = Vec::new();
        let mut best = (f64::MIN, 0.0);
        for &dl in cands.iter().take(3) {
            let sc = score(&f, p, phase0 + dl, &mut buf, &mut ys);
            if sc > best.0 + 1e-9 {
                best = (sc, dl);
            }
        }
        phase += best.1;
    }
    // Voies resserrées : celles qui tombent dans une allée de cette trame. Sans côté calé, la
    // trame glisse ensuite d'au plus 2 m pour tirer parti de la place libérée, sans en perdre.
    let landed = row_starts(&f, p, phase, &mut ys, Some(&mut narrowed));
    if landed > 0 {
        f.narrow.retain(|n| narrowed.iter().any(|h| h.0 == n.1));
        let base = phase;
        if pin.is_none() && !p.quick && p.phase.is_none() {
            let mut buf = Vec::new();
            let mut best = (f64::MIN, 0i32);
            for k in [0i32, -1, 1, -2, 2, -3, 3, -4, 4, -5, 5, -6, 6, -7, 7, -8, 8] {
                let ph = base + k as f64 * 0.25;
                if row_starts(&f, p, ph, &mut ys, None) < landed {
                    continue;
                }
                let s = score(&f, p, ph, &mut buf, &mut ys);
                if s > best.0 + 1e-9 {
                    best = (s, k);
                }
            }
            phase = base + best.1 as f64 * 0.25;
        }
        narrowed.clear();
        row_starts(&f, p, phase, &mut ys, Some(&mut narrowed));
        // Repère de chaque voie : milieu de l'allée sur la trame de base, stable.
        for h in narrowed.iter_mut() {
            h.1 -= phase - base;
        }
    }
    if let Some(pf) = pin {
        let dl = flush(&ys, pf, d2);
        for y in ys.iter_mut() {
            *y += dl;
        }
        for h in narrowed.iter_mut() {
            h.2 += dl;
        }
    }
    let mut strips = Vec::new();
    for y0 in ys {
        pack_double_row(&f, p, y0, rot, acc);
        let (x0, x1, y1) = (f.xlo - 1.0, f.xhi + 1.0, y0 + p.double_depth());
        strips.push(xf(
            &Quad([v2(x0, y0), v2(x1, y0), v2(x1, y1), v2(x0, y1)]),
            rot,
        ));
    }
    RowsOut {
        angle,
        phase,
        evals,
        strips,
        narrowed,
    }
}
