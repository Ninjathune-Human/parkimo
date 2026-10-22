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

/// Remplace des groupes de 4 places alignées par des places PMR au plus près des cibles,
/// puis marque les places pré-équipées IRVE. Renvoie (pmr, irve) requis.
pub fn assign_kinds(stalls: &mut Vec<Stall>, targets: &[V2], p: &Params) -> (usize, usize) {
    let w = p.stall_w;
    let group = 4usize;
    let k_pmr = ((group as f64 * w) / p.pmr_w + 1e-9).floor() as usize;
    let need = |total: usize, ratio: f64| -> usize {
        if ratio <= 0.0 || total == 0 {
            0
        } else {
            ((total as f64 * ratio) - 1e-9).ceil().max(1.0) as usize
        }
    };

    if k_pmr >= 1 && p.pmr_ratio > 0.0 {
        // Fenêtres de 4 places consécutives dans une même rangée.
        let mut by_row: HashMap<u32, Vec<usize>> = HashMap::new();
        for (i, s) in stalls.iter().enumerate() {
            by_row.entry(s.row).or_default().push(i);
        }
        let mut windows: Vec<(f64, [usize; 4])> = Vec::new();
        for ids in by_row.values_mut() {
            ids.sort_by_key(|&i| stalls[i].idx);
            for wdw in ids.windows(group) {
                let ok = (0..group - 1).all(|t| {
                    stalls[wdw[t + 1]].idx == stalls[wdw[t]].idx + 1
                        && adjacent(&stalls[wdw[t]].q, &stalls[wdw[t + 1]].q)
                });
                if ok {
                    let c = wdw
                        .iter()
                        .map(|&i| stalls[i].access_point(0.0))
                        .fold(V2::default(), |a, b| a + b)
                        * 0.25;
                    windows.push((nearest(c, targets), [wdw[0], wdw[1], wdw[2], wdw[3]]));
                }
            }
        }
        windows.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut used = vec![false; stalls.len()];
        let mut removed = vec![false; stalls.len()];
        let mut new_stalls: Vec<Stall> = Vec::new();
        let mut total = stalls.len();
        let mut pmr = 0usize;
        for (_, wdw) in windows {
            if pmr >= need(total, p.pmr_ratio) {
                break;
            }
            if wdw.iter().any(|&i| used[i]) {
                continue;
            }
            for &i in &wdw {
                used[i] = true;
                removed[i] = true;
            }
            let (s0, s3) = (&stalls[wdw[0]], &stalls[wdw[3]]);
            // Extrémités du groupe (fond et avant), dans l'ordre de la rangée.
            let forward = adjacent_dir(&stalls[wdw[0]].q, &stalls[wdw[1]].q);
            let (b0, f0, b1, f1) = if forward {
                (s0.q.0[0], s0.q.0[3], s3.q.0[1], s3.q.0[2])
            } else {
                (s0.q.0[1], s0.q.0[2], s3.q.0[0], s3.q.0[3])
            };
            for t in 0..k_pmr {
                let a = t as f64 / k_pmr as f64;
                let c = (t + 1) as f64 / k_pmr as f64;
                let pa = b0 + (b1 - b0) * a;
                let pc = b0 + (b1 - b0) * c;
                let fa = f0 + (f1 - f0) * a;
                let fc = f0 + (f1 - f0) * c;
                let q = if forward {
                    Quad([pa, pc, fc, fa])
                } else {
                    Quad([pc, pa, fa, fc])
                };
                new_stalls.push(Stall {
                    q,
                    kind: KIND_PMR,
                    row: s0.row,
                    idx: -1,
                    perimeter: s0.perimeter,
                });
            }
            pmr += k_pmr;
            total = total - group + k_pmr;
        }
        let mut kept: Vec<Stall> = stalls
            .drain(..)
            .enumerate()
            .filter(|(i, _)| !removed[*i])
            .map(|(_, s)| s)
            .collect();
        kept.extend(new_stalls);
        *stalls = kept;
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
    (pmr_req, irve_req)
}

fn adjacent_dir(a: &Quad, b: &Quad) -> bool {
    (a.0[1] - b.0[0]).len() < 0.05
}
