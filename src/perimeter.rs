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

/// Direction locale de la limite à l'abscisse `s` (différence centrée sur une largeur de place).
fn tangent(ch: &Chain, s: f64, h: f64) -> V2 {
    let (a, b) = (ch.at((s - h).max(0.0)), ch.at((s + h).min(ch.len())));
    (b - a).norm()
}

/// Place de largeur `w` posée au plus près de l'abscisse `s` : (abscisse de fin, normale, quadrilatère).
/// Sur une limite qui s'éloigne du centre de courbure (arc bombé vers l'extérieur), les places se
/// resserrent vers l'allée : on élargit la corde du fond jusqu'à ce que l'entrée de la place (face
/// avant) mesure bien la largeur demandée. `Err(true)` : au-delà de `s_end` ; `Err(false)` : corde
/// trop ouverte (angle, fin de chaîne), pas de place ici.
fn build_stall(ch: &Chain, s: f64, w: f64, d: f64, s_end: f64) -> Result<(f64, V2, Quad), bool> {
    let p0 = ch.at(s);
    let mut chord = w;
    for _ in 0..6 {
        // Corde de longueur `chord` (sur une courbe, l'abscisse avance plus que la corde).
        let mut s1 = s + chord;
        for _ in 0..8 {
            let dd = (ch.at(s1) - p0).len();
            if dd >= chord - 1e-6 {
                break;
            }
            s1 += chord - dd;
        }
        if s1 > s_end + 1e-6 {
            return Err(true);
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
        // Le trait qui sépare deux places voisines suit la normale locale de la limite (médiane des
        // deux côtés d'un éventail) : sur une courbe, les places sont des trapèzes qui se touchent.
        let (n0, n1) = (tangent(ch, s, w * 0.5).left(), tangent(ch, s1, w * 0.5).left());
        let (b0, b1) = (p0 + n0 * shift, p1 + n1 * shift);
        let q = Quad([b0, b1, b1 + n1 * d, b0 + n0 * d]);
        let front = (q.0[2] - q.0[3]).len();
        if front >= w - 1e-3 {
            return Ok((s1, n, q));
        }
        chord += w - front;
        if chord > 1.5 * w {
            break;
        }
    }
    Err(false)
}

/// Tronçons libres d'une chaîne entre les obstacles (accès, cheminements d'entrée de bâtiment) :
/// (début de la première place, fin permise). Le reste de place de chaque tronçon est réparti
/// équitablement de part et d'autre de chaque obstacle (même jeu des deux côtés, au plus le reste
/// disponible), sans jamais perdre une place.
fn segments(ch: &Chain, w: f64, d: f64, s0: f64, s_end: f64, entries: &[Quad]) -> Vec<(f64, f64)> {
    const SCAN: f64 = 0.1;
    // Intervalles bloqués : (fin de la dernière place possible, début de la première place libre).
    let mut obs: Vec<(f64, f64)> = Vec::new();
    if !entries.is_empty() {
        let mut cur: Option<f64> = None;
        let mut s = s0;
        while s + w <= s_end + 1e-6 {
            if let Ok((s1, _, q)) = build_stall(ch, s, w, d, s_end) {
                let hit = entries.iter().any(|e| e.overlaps(&q));
                match (hit, cur) {
                    (true, None) => cur = Some(s1 - SCAN),
                    (false, Some(o0)) => {
                        obs.push((o0, s));
                        cur = None;
                    }
                    _ => {}
                }
            }
            s += SCAN;
        }
        if let Some(o0) = cur {
            obs.push((o0, s_end));
        }
    }
    // Tronçons libres et leur reste de place.
    let mut segs: Vec<(f64, f64)> = Vec::new();
    let mut a = s0;
    for &(o0, o1) in &obs {
        segs.push((a, o0.max(a)));
        a = o1;
    }
    segs.push((a, s_end));
    if obs.is_empty() {
        return segs;
    }
    let slack: Vec<f64> = segs
        .iter()
        .map(|&(a, b)| {
            let (mut s, mut e) = (a, a);
            while let Ok((s1, _, _)) = build_stall(ch, s, w, d, b) {
                s = s1;
                e = s1;
            }
            (b - e).max(0.0)
        })
        .collect();
    let k = obs.len();
    let last = segs.len() - 1;
    // Jeu de chaque obstacle : le même des deux côtés, au plus la part disponible de chaque tronçon.
    let gap: Vec<f64> = (0..k)
        .map(|i| {
            let left = if i == 0 { slack[i] } else { slack[i] * 0.5 };
            let right = if i + 1 == last { slack[i + 1] } else { slack[i + 1] * 0.5 };
            left.min(right)
        })
        .collect();
    segs.iter()
        .enumerate()
        .map(|(i, &(a, b))| {
            let before = if i > 0 { Some(gap[i - 1]) } else { None };
            let after = if i < last { Some(gap[i]) } else { None };
            let start = match (before, after) {
                (None, None) => a,
                (None, Some(g)) => a + (slack[i] - g).max(0.0),
                (Some(g), None) => a + g,
                (Some(g0), Some(g1)) => a + g0 + (slack[i] - g0 - g1).max(0.0) * 0.5,
            };
            (start, b)
        })
        .collect()
}

/// `b` : zone stationnable ; `drive` : B érodée de la profondeur de place (zone circulable).
pub fn place(b: &Region, drive: &Region, p: &Params, entries: &[Quad], acc: &mut Acc) {
    let (w, d) = (p.stall_w, p.stall_d);
    let a_chk = (p.aisle - 0.3).max(3.0);
    let mut chains: Vec<Chain> = b.rings.iter().flat_map(|r| chains_of(r)).collect();
    chains.sort_by(|x, y| y.len().partial_cmp(&x.len()).unwrap());

    for ch in &chains {
        let chain_end = ch.len() - corner_trim(ch.turn_end, d, a_chk);
        let chain_start = corner_trim(ch.turn_start, d, a_chk);
        for (seg_start, s_end) in segments(ch, w, d, chain_start, chain_end, entries) {
            let mut s = seg_start;
            let mut row = acc.new_row();
            let mut idx = 0i32;
            let mut run = 0u32;
            let mut last_ok = false;
            while s + w <= s_end + 1e-6 {
                let (s1, n, q) = match build_stall(ch, s, w, d, s_end) {
                    Ok(x) => x,
                    Err(true) => break,
                    Err(false) => {
                        // Corde trop ouverte (angle, fin de chaîne) : pas de place ici, on avance.
                        last_ok = false;
                        s += STEP;
                        continue;
                    }
                };
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
                let ok = ok_geom
                    && drive.contains_quad(&box_, TOL)
                    && !acc.hash.hits(&box_.shrink(TOL));
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
                        equipped: false,
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
}
