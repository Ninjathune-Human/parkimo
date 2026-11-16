//! Finitions : connectivité depuis les accès, places PMR et IRVE.

use crate::geom::{Quad, V2};
use crate::layout::{Stall, KIND_IRVE, KIND_PMR, KIND_PMR_IRVE, KIND_STD};
use crate::model::Params;
use std::collections::{HashMap, VecDeque};

const CELL: f64 = 0.5;

/// Retire les places dont la zone d'accès n'est pas reliée à une entrée.
/// `drive` : anneaux (pair-impair) de la zone circulable ; `blockers` : emprises à exclure.
pub fn keep_reachable(
    stalls: &mut Vec<Stall>,
    drive: &[Vec<V2>],
    entries: &[Quad],
    blockers: &[Quad],
) -> usize {
    if entries.is_empty() || stalls.is_empty() {
        return 0;
    }
    let (mut lo, mut hi) = (
        V2 {
            x: f64::MAX,
            y: f64::MAX,
        },
        V2 {
            x: f64::MIN,
            y: f64::MIN,
        },
    );
    let mut grow = |p: V2| {
        lo = V2 {
            x: lo.x.min(p.x),
            y: lo.y.min(p.y),
        };
        hi = V2 {
            x: hi.x.max(p.x),
            y: hi.y.max(p.y),
        };
    };
    for r in drive {
        for &p in r {
            grow(p);
        }
    }
    for e in entries {
        for &p in &e.0 {
            grow(p);
        }
    }
    let nx = ((hi.x - lo.x) / CELL).ceil() as i64 + 1;
    let ny = ((hi.y - lo.y) / CELL).ceil() as i64 + 1;
    if nx <= 0 || ny <= 0 || nx * ny > 6_000_000 {
        return 0;
    }
    let (nx, ny) = (nx as usize, ny as usize);
    let mut g = vec![0u8; nx * ny]; // 0 bloqué, 1 circulable, 2 atteint
    let cx = |i: usize| lo.x + (i as f64 + 0.5) * CELL;
    let cy = |j: usize| lo.y + (j as f64 + 0.5) * CELL;

    let mut xs: Vec<f64> = Vec::new();
    for j in 0..ny {
        let y = cy(j);
        xs.clear();
        for r in drive {
            let n = r.len();
            for i in 0..n {
                let (a, b) = (r[i], r[(i + 1) % n]);
                if (a.y > y) != (b.y > y) {
                    xs.push(a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y));
                }
            }
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for pair in xs.chunks(2) {
            if pair.len() < 2 {
                break;
            }
            let i0 = (((pair[0] - lo.x) / CELL) - 0.5).ceil().max(0.0) as usize;
            let i1 = ((((pair[1] - lo.x) / CELL) - 0.5).floor() as i64).min(nx as i64 - 1);
            if i1 < 0 {
                continue;
            }
            for i in i0..=(i1 as usize) {
                g[j * nx + i] = 1;
            }
        }
    }
    let fill = |q: &Quad, val: u8, g: &mut Vec<u8>| {
        let (a, b) = q.bbox();
        let i0 = (((a.x - lo.x) / CELL) - 0.5).floor().max(0.0) as usize;
        let i1 = ((((b.x - lo.x) / CELL) - 0.5).ceil() as usize).min(nx - 1);
        let j0 = (((a.y - lo.y) / CELL) - 0.5).floor().max(0.0) as usize;
        let j1 = ((((b.y - lo.y) / CELL) - 0.5).ceil() as usize).min(ny - 1);
        for j in j0..=j1 {
            for i in i0..=i1 {
                if q.contains(V2 { x: cx(i), y: cy(j) }) {
                    g[j * nx + i] = val;
                }
            }
        }
    };
    for e in entries {
        fill(e, 1, &mut g);
    }
    for b in blockers {
        fill(b, 0, &mut g);
    }
    for s in stalls.iter().filter(|s| s.perimeter) {
        fill(&s.q.shrink(0.05), 0, &mut g);
    }
    let mut queue = VecDeque::new();
    for e in entries {
        let (a, b) = e.bbox();
        let i0 = (((a.x - lo.x) / CELL).floor().max(0.0) as usize).min(nx - 1);
        let i1 = (((b.x - lo.x) / CELL).ceil() as usize).min(nx - 1);
        let j0 = (((a.y - lo.y) / CELL).floor().max(0.0) as usize).min(ny - 1);
        let j1 = (((b.y - lo.y) / CELL).ceil() as usize).min(ny - 1);
        for j in j0..=j1 {
            for i in i0..=i1 {
                let k = j * nx + i;
                if g[k] == 1 && e.contains(V2 { x: cx(i), y: cy(j) }) {
                    g[k] = 2;
                    queue.push_back(k);
                }
            }
        }
    }
    while let Some(k) = queue.pop_front() {
        let (i, j) = (k % nx, k / nx);
        let mut push = |kk: usize, g: &mut Vec<u8>| {
            if g[kk] == 1 {
                g[kk] = 2;
                queue.push_back(kk);
            }
        };
        if i > 0 {
            push(k - 1, &mut g);
        }
        if i + 1 < nx {
            push(k + 1, &mut g);
        }
        if j > 0 {
            push(k - nx, &mut g);
        }
        if j + 1 < ny {
            push(k + nx, &mut g);
        }
    }
    let reached = |p: V2| -> bool {
        let i = ((p.x - lo.x) / CELL).floor();
        let j = ((p.y - lo.y) / CELL).floor();
        if i < 0.0 || j < 0.0 || i as usize >= nx || j as usize >= ny {
            return false;
        }
        g[j as usize * nx + i as usize] == 2
    };
    let before = stalls.len();
    stalls.retain(|s| [1.0, 2.0, 3.0].iter().any(|&t| reached(s.access_point(t))));
    before - stalls.len()
}

fn adjacent(a: &Quad, b: &Quad) -> bool {
    let close = |p: V2, q: V2| (p - q).len() < 0.05;
    let ua = (a.0[1] - a.0[0]).norm();
    let ub = (b.0[1] - b.0[0]).norm();
    ua.dot(ub) > 0.999 && (close(a.0[1], b.0[0]) || close(a.0[0], b.0[1]))
}

fn nearest(p: V2, targets: &[V2]) -> f64 {
    targets
        .iter()
        .map(|t| (*t - p).len())
        .fold(f64::MAX, f64::min)
}

/// Pose les places PMR au plus près des cibles (voir `place_pmr`), puis marque les places
/// pré-équipées IRVE. Renvoie (pmr, irve) requis.
pub fn assign_kinds(stalls: &mut Vec<Stall>, targets: &[V2], p: &Params) -> (usize, usize, usize) {
    let need = |total: usize, ratio: f64| -> usize {
        if ratio <= 0.0 || total == 0 {
            0
        } else {
            ((total as f64 * ratio) - 1e-9).ceil().max(1.0) as usize
        }
    };
    if p.pmr_ratio > 0.0 && p.pmr_w > 0.0 {
        place_pmr(stalls, targets, p, |total| need(total, p.pmr_ratio));
    }

    let total = stalls.len();
    let pmr_req = need(total, p.pmr_ratio);
    let irve_req = need(total, p.irve_ratio);
    if irve_req > 0 {
        let mut order: Vec<usize> = (0..stalls.len()).collect();
        order.sort_by(|&a, &b| {
            nearest(stalls[a].access_point(0.0), targets)
                .partial_cmp(&nearest(stalls[b].access_point(0.0), targets))
                .unwrap()
        });
        let pmr_irve = ((irve_req as f64 * 0.02) - 1e-9).ceil().max(1.0) as usize;
        let mut done = 0usize;
        let mut done_pmr = 0usize;
        for &i in &order {
            if stalls[i].kind == KIND_PMR && done_pmr < pmr_irve {
                stalls[i].kind = KIND_PMR_IRVE;
                done_pmr += 1;
                done += 1;
            }
        }
        for &i in &order {
            if done >= irve_req {
                break;
            }
            if stalls[i].kind == KIND_STD {
                stalls[i].kind = KIND_IRVE;
                done += 1;
            }
        }
    }
    // Bornes installées, prises parmi les places IRVE : la PMR d'abord, puis au plus près des cibles.
    let eq_req = if irve_req > 0 && p.irve_equip_ratio > 0.0 {
        (((total as f64 * p.irve_equip_ratio) + 1e-9).floor() as usize).clamp(1, irve_req)
    } else {
        0
    };
    let mut ids: Vec<usize> = (0..stalls.len())
        .filter(|&i| stalls[i].kind == KIND_IRVE || stalls[i].kind == KIND_PMR_IRVE)
        .collect();
    ids.sort_by(|&a, &b| {
        let ka = (stalls[a].kind != KIND_PMR_IRVE) as u8;
        let kb = (stalls[b].kind != KIND_PMR_IRVE) as u8;
        ka.cmp(&kb).then(
            nearest(stalls[a].access_point(0.0), targets)
                .partial_cmp(&nearest(stalls[b].access_point(0.0), targets))
                .unwrap(),
        )
    });
    for &i in ids.iter().take(eq_req) {
        stalls[i].equipped = true;
    }
    (pmr_req, irve_req, eq_req)
}

/// Places PMR de la largeur réglée exactement (3,30 m), par groupes au plus près des cibles. Un
/// groupe de k places PMR prend la place de n places ordinaires consécutives d'une même rangée
/// (n = k × largeur PMR / largeur de place, arrondi au-dessus) ; le reste de la rangée est
/// redistribué à égalité à ses places ordinaires, jamais sous leur largeur. k est le plus grand
/// groupe qui ne coûte qu'une place (3 avec des places de 2,50 m, 2 avec 2,40 ou 2,30 m).
fn place_pmr(stalls: &mut Vec<Stall>, targets: &[V2], p: &Params, need: impl Fn(usize) -> usize) {
    let (w, wp) = (p.stall_w, p.pmr_w);
    let k_max = if wp <= w { 4 } else { (w / (wp - w) + 1e-9).floor().clamp(1.0, 4.0) as usize };
    let mut pmr = 0;
    loop {
        let req = need(stalls.len());
        if pmr >= req {
            break;
        }
        let k = k_max.min(req - pmr);
        let n = ((k as f64 * wp / w) - 1e-9).ceil().max(k as f64) as usize;
        // D'abord une rangée où il reste des places ordinaires pour prendre le reste ; à défaut, les
        // places PMR d'une suite trop courte se partagent toute sa longueur.
        let mut best: Option<(f64, Vec<usize>, Vec<Stall>)> = None;
        for strict in [true, false] {
            for run in runs(stalls) {
                for s in 0..(run.len() + 1).saturating_sub(n) {
                    let win = &run[s..s + n];
                    if win.iter().any(|&i| stalls[i].kind != KIND_STD) {
                        continue;
                    }
                    let c = win.iter().map(|&i| stalls[i].access_point(0.0)).fold(V2::default(), |a, b| a + b) * (1.0 / n as f64);
                    let d = nearest(c, targets);
                    if best.as_ref().is_some_and(|b| b.0 <= d) {
                        continue;
                    }
                    if let Some(rebuilt) = rebuild(stalls, &run, s, n, k, w, wp, strict) {
                        best = Some((d, run.clone(), rebuilt));
                    }
                }
            }
            if best.is_some() {
                break;
            }
        }
        let Some((_, run, rebuilt)) = best else { break };
        let mut gone = vec![false; stalls.len()];
        for &i in &run {
            gone[i] = true;
        }
        let mut i = 0;
        stalls.retain(|_| {
            i += 1;
            !gone[i - 1]
        });
        stalls.extend(rebuilt);
        pmr += k;
    }
}

/// Suites de places adjacentes d'une même rangée, dans l'ordre de la rangée.
fn runs(stalls: &[Stall]) -> Vec<Vec<usize>> {
    let mut by_row: HashMap<u32, Vec<usize>> = HashMap::new();
    for (i, s) in stalls.iter().enumerate() {
        by_row.entry(s.row).or_default().push(i);
    }
    let mut rows: Vec<(u32, Vec<usize>)> = by_row.into_iter().collect();
    rows.sort_by_key(|r| r.0);
    let mut out = Vec::new();
    for (_, mut ids) in rows {
        ids.sort_by_key(|&i| stalls[i].idx);
        let mut cur: Vec<usize> = Vec::new();
        for i in ids {
            if let Some(&j) = cur.last() {
                if stalls[i].idx != stalls[j].idx + 1 || !adjacent(&stalls[j].q, &stalls[i].q) {
                    out.push(std::mem::take(&mut cur));
                }
            }
            cur.push(i);
        }
        out.push(cur);
    }
    out
}

/// La suite `run` redessinée : les places `s..s + n` deviennent `k` places PMR de largeur `wp`, les
/// places PMR déjà posées gardent `wp`, les places ordinaires se partagent le reste à égalité (au
/// moins `w`, sinon `None`). Sans place ordinaire restante, les places PMR se partagent la suite si
/// `strict` est faux, sinon `None`. Largeurs mesurées perpendiculairement à l'axe des places : le long
/// de la rangée, chaque place prend une part de la suite proportionnelle à sa largeur.
#[allow(clippy::too_many_arguments)]
fn rebuild(stalls: &[Stall], run: &[usize], s: usize, n: usize, k: usize, w: f64, wp: f64, strict: bool) -> Option<Vec<Stall>> {
    let kinds: Vec<u8> = run[..s].iter().map(|&i| stalls[i].kind).chain(std::iter::repeat_n(KIND_PMR, k)).chain(run[s + n..].iter().map(|&i| stalls[i].kind)).collect();
    let total: f64 = run.iter().map(|&i| width(&stalls[i].q)).sum();
    let fixed = kinds.iter().filter(|&&c| c != KIND_STD).count();
    let free = kinds.len() - fixed;
    let (w_pmr, w_std) = if free == 0 { (total / fixed as f64, 0.0) } else { (wp, (total - fixed as f64 * wp) / free as f64) };
    if (free > 0 && w_std < w - 1e-6) || (free == 0 && strict && total - fixed as f64 * wp > 1e-3) {
        return None;
    }
    let (first, last) = (&stalls[run[0]], &stalls[run[run.len() - 1]]);
    let forward = run.len() < 2 || adjacent_dir(&first.q, &stalls[run[1]].q);
    let (b0, f0, b1, f1) = if forward {
        (first.q.0[0], first.q.0[3], last.q.0[1], last.q.0[2])
    } else {
        (first.q.0[1], first.q.0[2], last.q.0[0], last.q.0[3])
    };
    let mut at = 0.0;
    let mut out = Vec::with_capacity(kinds.len());
    for (t, &kind) in kinds.iter().enumerate() {
        let (a, c) = (at / total, (at + if kind == KIND_STD { w_std } else { w_pmr }) / total);
        at += if kind == KIND_STD { w_std } else { w_pmr };
        let (pa, pc) = (b0 + (b1 - b0) * a, b0 + (b1 - b0) * c);
        let (fa, fc) = (f0 + (f1 - f0) * a, f0 + (f1 - f0) * c);
        let q = if forward { Quad([pa, pc, fc, fa]) } else { Quad([pc, pa, fa, fc]) };
        out.push(Stall { q, kind, row: first.row, idx: first.idx + t as i32, perimeter: first.perimeter, equipped: false });
    }
    Some(out)
}

/// Largeur d'une place, perpendiculairement à son axe.
fn width(q: &Quad) -> f64 {
    let axis = (q.0[3] - q.0[0]).norm();
    let across = |v: V2| (v.x * axis.y - v.y * axis.x).abs();
    across(q.0[1] - q.0[0]).min(across(q.0[2] - q.0[3]))
}

fn adjacent_dir(a: &Quad, b: &Quad) -> bool {
    (a.0[1] - b.0[0]).len() < 0.05
}
