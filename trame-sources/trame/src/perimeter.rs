//! Places périphériques : on longe chaque limite de la zone B (extérieure et trous),
//! y compris les bords courbes, en posant des places perpendiculaires au bord local.

use crate::geom::{Quad, Region, V2};
use crate::layout::{Acc, Stall, KIND_STD};
use crate::model::Params;
use std::f64::consts::PI;

const CORNER: f64 = 0.5; // ~29° : au-delà, on considère un angle et non une courbe
const STEP: f64 = 0.25;
const TOL: f64 = 0.02;

struct Chain {
    pts: Vec<V2>,
    cum: Vec<f64>,
    turn_start: f64,
    turn_end: f64,
}

impl Chain {
    fn new(pts: Vec<V2>, turn_start: f64, turn_end: f64) -> Chain {
        let mut cum = vec![0.0];
        for i in 1..pts.len() {
            let l = cum[i - 1] + (pts[i] - pts[i - 1]).len();
            cum.push(l);
        }
        Chain {
            pts,
            cum,
            turn_start,
            turn_end,
        }
    }
    fn len(&self) -> f64 {
        *self.cum.last().unwrap()
    }
    fn at(&self, s: f64) -> V2 {
        let i = match self.cum.binary_search_by(|c| c.partial_cmp(&s).unwrap()) {
            Ok(i) => return self.pts[i],
            Err(i) => i.clamp(1, self.pts.len() - 1),
        };
        let (a, b) = (self.pts[i - 1], self.pts[i]);
        let seg = self.cum[i] - self.cum[i - 1];
        if seg < 1e-12 {
            return a;
        }
        a + (b - a) * ((s - self.cum[i - 1]) / seg)
    }
}

fn turn_at(prev: V2, cur: V2, next: V2) -> f64 {
    let e0 = cur - prev;
    let e1 = next - cur;
    e0.cross(e1).atan2(e0.dot(e1))
}

fn chains_of(ring: &[V2]) -> Vec<Chain> {
    let n = ring.len();
    if n < 3 {
        return vec![];
    }
    let turns: Vec<f64> = (0..n)
        .map(|i| turn_at(ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]))
        .collect();
    let corners: Vec<usize> = (0..n).filter(|&i| turns[i].abs() > CORNER).collect();
    if corners.is_empty() {
        let mut pts = ring.to_vec();
        pts.push(ring[0]);
        return vec![Chain::new(pts, 0.0, 0.0)];
    }
    let mut out = Vec::new();
    for j in 0..corners.len() {
        let a = corners[j];
        let b = corners[(j + 1) % corners.len()];
        let mut pts = vec![ring[a]];
        let mut k = a;
        loop {
            k = (k + 1) % n;
            pts.push(ring[k]);
            if k == b {
                break;
            }
        }
        out.push(Chain::new(pts, turns[a], turns[b]));
    }
    out
}

/// Longueur à laisser libre au droit d'un angle saillant d'ouverture β pour que les places
/// des deux côtés ne se chevauchent pas (d·cot(β/2)) et que la zone de manœuvre d'un côté
/// n'empiète pas sur les places de l'autre ((d + (d+a)·cos β) / sin β, angle aigu).
fn corner_trim(turn: f64, d: f64, a: f64) -> f64 {
    if turn <= 0.0 {
        return 0.0;
    }
    let beta = (PI - turn).max(0.05);
    let stalls = d / (beta / 2.0).tan();
    let access = (d + (d + a) * beta.cos()) / beta.sin();
    stalls.max(access)
}

/// `b` : zone stationnable ; `drive` : B érodée de la profondeur de place (zone circulable).
pub fn place(b: &Region, drive: &Region, p: &Params, entries: &[Quad], acc: &mut Acc) {
    let (w, d) = (p.stall_w, p.stall_d);
    let a_chk = (p.aisle - 0.3).max(3.0);
    let mut chains: Vec<Chain> = b.rings.iter().flat_map(|r| chains_of(r)).collect();
    chains.sort_by(|x, y| y.len().partial_cmp(&x.len()).unwrap());

    for ch in &chains {
        let s_end = ch.len() - corner_trim(ch.turn_end, d, a_chk);
        let mut s = corner_trim(ch.turn_start, d, a_chk);
        let mut row = acc.new_row();
        let mut idx = 0i32;
        let mut run = 0u32;
        let mut last_ok = false;
        while s + w <= s_end + 1e-6 {
            let p0 = ch.at(s);
            // Corde de longueur w (sur une courbe, l'abscisse avance plus que w).
            let mut s1 = s + w;
            for _ in 0..8 {
                let dd = (ch.at(s1) - p0).len();
                if dd >= w - 1e-6 {
                    break;
                }
                s1 += w - dd;
            }
            if s1 > s_end + 1e-6 {
                break;
            }
            let p1 = ch.at(s1);
            let u = (p1 - p0).norm();
            let n = u.left();
            // Flèche : si le bord rentre entre p0 et p1, on décale la place vers l'intérieur.
            let mut shift: f64 = 0.0;
            for k in 0..ch.pts.len() {
                if ch.cum[k] > s && ch.cum[k] < s1 {
                    shift = shift.max((ch.pts[k] - p0).dot(n));
                }
            }
            let q = Quad::on_base(p0 + n * shift, p1 + n * shift, n, d);
            let planter_turn = p.planter_every > 0 && run >= p.planter_every;
            let ok_geom = b.contains_quad(&q, TOL)
                && !acc.hash.hits(&q.shrink(TOL))
                && !acc.access.hits(&q.shrink(TOL))
                && !entries.iter().any(|e| e.overlaps(&q));
            if ok_geom && planter_turn {
                acc.islands.push(q);
                acc.trees.push(q.center());
                acc.hash.insert(q);
                idx += 1;
                run = 0;
                s = s1;
                last_ok = true;
                continue;
            }
            let box_ = Quad::on_base(q.0[3], q.0[2], n, a_chk);
            let ok =
                ok_geom && drive.contains_quad(&box_, TOL) && !acc.hash.hits(&box_.shrink(TOL));
            if ok {
                if !last_ok && idx > 0 {
                    row = acc.new_row();
                }
                acc.hash.insert(q);
                acc.access.insert(box_);
                acc.stalls.push(Stall {
                    q,
                    kind: KIND_STD,
                    row,
                    idx,
                    perimeter: true,
                });
                idx += 1;
                run += 1;
                s = s1;
                last_ok = true;
            } else {
                if last_ok {
                    run = 0;
                }
                last_ok = false;
                s += STEP;
            }
        }
    }
}
