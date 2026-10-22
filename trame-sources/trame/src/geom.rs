//! Primitives géométriques : vecteurs, quadrilatères orientés, régions (anneaux pair-impair),
//! intervalles de bande et index spatiaux.

use std::collections::HashMap;
use std::ops::{Add, Mul, Neg, Sub};

pub const EPS: f64 = 1e-9;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V2 {
    pub x: f64,
    pub y: f64,
}

pub const fn v2(x: f64, y: f64) -> V2 {
    V2 { x, y }
}

impl Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        v2(self.x + o.x, self.y + o.y)
    }
}
impl Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        v2(self.x - o.x, self.y - o.y)
    }
}
impl Mul<f64> for V2 {
    type Output = V2;
    fn mul(self, k: f64) -> V2 {
        v2(self.x * k, self.y * k)
    }
}
impl Neg for V2 {
    type Output = V2;
    fn neg(self) -> V2 {
        v2(-self.x, -self.y)
    }
}

impl V2 {
    pub fn dot(self, o: V2) -> f64 {
        self.x * o.x + self.y * o.y
    }
    pub fn cross(self, o: V2) -> f64 {
        self.x * o.y - self.y * o.x
    }
    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn norm(self) -> V2 {
        let l = self.len();
        if l < EPS {
            v2(0.0, 0.0)
        } else {
            self * (1.0 / l)
        }
    }
    /// Normale gauche (rotation +90°).
    pub fn left(self) -> V2 {
        v2(-self.y, self.x)
    }
    /// Rotation par (cos, sin).
    pub fn rot(self, c: f64, s: f64) -> V2 {
        v2(self.x * c - self.y * s, self.x * s + self.y * c)
    }
}

pub fn signed_area(ring: &[V2]) -> f64 {
    let n = ring.len();
    let mut a = 0.0;
    for i in 0..n {
        a += ring[i].cross(ring[(i + 1) % n]);
    }
    a * 0.5
}

/// Supprime la fermeture explicite et les points quasi confondus.
pub fn clean_ring(ring: &[V2]) -> Vec<V2> {
    let mut out: Vec<V2> = Vec::with_capacity(ring.len());
    for &p in ring {
        if out.last().map_or(true, |&q| (p - q).len() > 1e-6) {
            out.push(p);
        }
    }
    while out.len() > 1 && (out[0] - out[out.len() - 1]).len() <= 1e-6 {
        out.pop();
    }
    out
}

/// Quadrilatère : [arrière-gauche, arrière-droit, avant-droit, avant-gauche].
/// Pour une place, l'arrière est le fond, l'avant donne sur l'allée.
#[derive(Clone, Copy, Debug)]
pub struct Quad(pub [V2; 4]);

impl Quad {
    /// Rectangle construit sur un segment de base `a→b` et une profondeur `depth` vers `n`.
    pub fn on_base(a: V2, b: V2, n: V2, depth: f64) -> Quad {
        Quad([a, b, b + n * depth, a + n * depth])
    }
    pub fn center(&self) -> V2 {
        (self.0[0] + self.0[1] + self.0[2] + self.0[3]) * 0.25
    }
    /// Rétrécit vers le centre de `t` mètres sur chaque côté (rectangles).
    pub fn shrink(&self, t: f64) -> Quad {
        let u = (self.0[1] - self.0[0]).norm();
        let n = (self.0[3] - self.0[0]).norm();
        Quad([
            self.0[0] + u * t + n * t,
            self.0[1] - u * t + n * t,
            self.0[2] - u * t - n * t,
            self.0[3] + u * t - n * t,
        ])
    }
    pub fn bbox(&self) -> (V2, V2) {
        let mut lo = self.0[0];
        let mut hi = self.0[0];
        for p in &self.0[1..] {
            lo = v2(lo.x.min(p.x), lo.y.min(p.y));
            hi = v2(hi.x.max(p.x), hi.y.max(p.y));
        }
        (lo, hi)
    }
    pub fn contains(&self, p: V2) -> bool {
        // Convexe : même signe de produit vectoriel sur les 4 côtés.
        let mut sign = 0.0;
        for i in 0..4 {
            let a = self.0[i];
            let b = self.0[(i + 1) % 4];
            let c = (b - a).cross(p - a);
            if c.abs() < 1e-12 {
                continue;
            }
            if sign == 0.0 {
                sign = c.signum();
            } else if c.signum() != sign {
                return false;
            }
        }
        true
    }
    /// Chevauchement strict par séparation d'axes (quadrilatères convexes).
    pub fn overlaps(&self, o: &Quad) -> bool {
        for q in [self, o] {
            for i in 0..4 {
                let e = q.0[(i + 1) % 4] - q.0[i];
                let ax = e.left();
                let (mut a0, mut a1) = (f64::MAX, f64::MIN);
                let (mut b0, mut b1) = (f64::MAX, f64::MIN);
                for p in &self.0 {
                    let d = ax.dot(*p);
                    a0 = a0.min(d);
                    a1 = a1.max(d);
                }
                for p in &o.0 {
                    let d = ax.dot(*p);
                    b0 = b0.min(d);
                    b1 = b1.max(d);
                }
                if a1 <= b0 + 1e-9 || b1 <= a0 + 1e-9 {
                    return false;
                }
            }
        }
        true
    }
}

fn seg_intersect(p1: V2, p2: V2, q1: V2, q2: V2) -> bool {
    let d1 = (p2 - p1).cross(q1 - p1);
    let d2 = (p2 - p1).cross(q2 - p1);
    let d3 = (q2 - q1).cross(p1 - q1);
    let d4 = (q2 - q1).cross(p2 - q1);
    ((d1 > 0.0) != (d2 > 0.0))
        && ((d3 > 0.0) != (d4 > 0.0))
        && d1 != 0.0
        && d2 != 0.0
        && d3 != 0.0
        && d4 != 0.0
}

/// Région plane définie par des anneaux (règle pair-impair), avec index d'arêtes.
/// Les anneaux sont orientés de sorte que l'intérieur soit à gauche.
pub struct Region {
    pub rings: Vec<Vec<V2>>,
    edges: Vec<(V2, V2)>,
    grid: HashMap<(i32, i32), Vec<u32>>,
    cell: f64,
}

impl Region {
    /// `rings` doivent déjà être orientés intérieur-à-gauche (extérieurs CCW, trous CW).
    pub fn new(rings: Vec<Vec<V2>>) -> Region {
        let cell = 4.0;
        let mut edges = Vec::new();
        let mut grid: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for r in &rings {
            let n = r.len();
            for i in 0..n {
                let a = r[i];
                let b = r[(i + 1) % n];
                let id = edges.len() as u32;
                edges.push((a, b));
                let (x0, x1) = (
                    (a.x.min(b.x) / cell).floor() as i32,
                    (a.x.max(b.x) / cell).floor() as i32,
                );
                let (y0, y1) = (
                    (a.y.min(b.y) / cell).floor() as i32,
                    (a.y.max(b.y) / cell).floor() as i32,
                );
                for gx in x0..=x1 {
                    for gy in y0..=y1 {
                        grid.entry((gx, gy)).or_default().push(id);
                    }
                }
            }
        }
        Region {
            rings,
            edges,
            grid,
            cell,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.rings.is_empty()
    }

    pub fn area(&self) -> f64 {
        self.rings.iter().map(|r| signed_area(r)).sum::<f64>().abs()
    }

    pub fn contains(&self, p: V2) -> bool {
        let mut inside = false;
        for &(a, b) in &self.edges {
            if (a.y > p.y) != (b.y > p.y) {
                let x = a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y);
                if x > p.x {
                    inside = !inside;
                }
            }
        }
        inside
    }

    /// Le quadrilatère est-il entièrement dans la région (bord rétréci de `tol`) ?
    pub fn contains_quad(&self, q: &Quad, tol: f64) -> bool {
        let q = q.shrink(tol);
        for p in &q.0 {
            if !self.contains(*p) {
                return false;
            }
        }
        let (lo, hi) = q.bbox();
        let (x0, x1) = (
            (lo.x / self.cell).floor() as i32,
            (hi.x / self.cell).floor() as i32,
        );
        let (y0, y1) = (
            (lo.y / self.cell).floor() as i32,
            (hi.y / self.cell).floor() as i32,
        );
        for gx in x0..=x1 {
            for gy in y0..=y1 {
                if let Some(ids) = self.grid.get(&(gx, gy)) {
                    for &id in ids {
                        let (a, b) = self.edges[id as usize];
                        if q.contains(a) {
                            return false;
                        }
                        for i in 0..4 {
                            if seg_intersect(a, b, q.0[i], q.0[(i + 1) % 4]) {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
    }
}

/// Rotation d'anneaux par (cos, sin).
pub fn rotate_rings(rings: &[Vec<V2>], c: f64, s: f64) -> Vec<Vec<V2>> {
    rings
        .iter()
        .map(|r| r.iter().map(|p| p.rot(c, s)).collect())
        .collect()
}

/// Intervalles en x où le segment vertical {x}×[ya,yb] est entièrement dans la région
/// (anneaux pair-impair). Exact : intérieur au milieu de bande, moins l'ombre des arêtes.
pub fn strip_intervals(rings: &[Vec<V2>], ya: f64, yb: f64, out: &mut Vec<(f64, f64)>) {
    out.clear();
    let ym = 0.5 * (ya + yb);
    let mut xs: Vec<f64> = Vec::new();
    let mut blocked: Vec<(f64, f64)> = Vec::new();
    for r in rings {
        let n = r.len();
        for i in 0..n {
            let p = r[i];
            let q = r[(i + 1) % n];
            if (p.y > ym) != (q.y > ym) {
                xs.push(p.x + (ym - p.y) * (q.x - p.x) / (q.y - p.y));
            }
            let (lo, hi) = (p.y.min(q.y), p.y.max(q.y));
            if hi < ya || lo > yb {
                continue;
            }
            let dy = q.y - p.y;
            if dy.abs() < 1e-12 {
                blocked.push((p.x.min(q.x), p.x.max(q.x)));
            } else {
                let t0 = (ya.max(lo) - p.y) / dy;
                let t1 = (yb.min(hi) - p.y) / dy;
                let xa = p.x + t0 * (q.x - p.x);
                let xb = p.x + t1 * (q.x - p.x);
                blocked.push((xa.min(xb), xa.max(xb)));
            }
        }
    }
    if xs.len() < 2 {
        return;
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    blocked.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut merged: Vec<(f64, f64)> = Vec::with_capacity(blocked.len());
    for b in blocked {
        if let Some(last) = merged.last_mut() {
            if b.0 <= last.1 {
                last.1 = last.1.max(b.1);
                continue;
            }
        }
        merged.push(b);
    }
    let mut bi = 0;
    for pair in xs.chunks(2) {
        if pair.len() < 2 {
            break;
        }
        let (mut a, b) = (pair[0], pair[1]);
        while bi < merged.len() && merged[bi].1 <= a {
            bi += 1;
        }
        let mut k = bi;
        while k < merged.len() && merged[k].0 < b {
            if merged[k].0 > a {
                out.push((a, merged[k].0));
            }
            a = a.max(merged[k].1);
            k += 1;
        }
        if b > a {
            out.push((a, b));
        }
    }
}

/// Index spatial simple pour quadrilatères acceptés.
pub struct QuadHash {
    cell: f64,
    map: HashMap<(i32, i32), Vec<u32>>,
    pub quads: Vec<Quad>,
}

impl QuadHash {
    pub fn new(cell: f64) -> QuadHash {
        QuadHash {
            cell,
            map: HashMap::new(),
            quads: Vec::new(),
        }
    }
    fn cells(&self, q: &Quad) -> (i32, i32, i32, i32) {
        let (lo, hi) = q.bbox();
        (
            (lo.x / self.cell).floor() as i32,
            (hi.x / self.cell).floor() as i32,
            (lo.y / self.cell).floor() as i32,
            (hi.y / self.cell).floor() as i32,
        )
    }
    pub fn insert(&mut self, q: Quad) {
        let id = self.quads.len() as u32;
        let (x0, x1, y0, y1) = self.cells(&q);
        for gx in x0..=x1 {
            for gy in y0..=y1 {
                self.map.entry((gx, gy)).or_default().push(id);
            }
        }
        self.quads.push(q);
    }
    pub fn hits(&self, q: &Quad) -> bool {
        let (x0, x1, y0, y1) = self.cells(q);
        for gx in x0..=x1 {
            for gy in y0..=y1 {
                if let Some(ids) = self.map.get(&(gx, gy)) {
                    for &id in ids {
                        if self.quads[id as usize].overlaps(q) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intervals_square_with_hole() {
        let outer = vec![v2(0., 0.), v2(10., 0.), v2(10., 10.), v2(0., 10.)];
        let hole = vec![v2(4., 4.), v2(4., 6.), v2(6., 6.), v2(6., 4.)];
        let mut out = Vec::new();
        strip_intervals(&[outer.clone(), hole.clone()], 1.0, 3.0, &mut out);
        assert_eq!(out.len(), 1);
        assert!((out[0].0 - 0.0).abs() < 1e-9 && (out[0].1 - 10.0).abs() < 1e-9);
        strip_intervals(&[outer, hole], 3.0, 5.0, &mut out);
        assert_eq!(out.len(), 2);
        assert!((out[0].1 - 4.0).abs() < 1e-9 && (out[1].0 - 6.0).abs() < 1e-9);
    }

    #[test]
    fn quad_inside_region() {
        let r = Region::new(vec![vec![
            v2(0., 0.),
            v2(10., 0.),
            v2(10., 10.),
            v2(0., 10.),
        ]]);
        let q = Quad::on_base(v2(0., 0.), v2(2.5, 0.), v2(0., 1.), 5.0);
        assert!(r.contains_quad(&q, 0.01));
        let q2 = Quad::on_base(v2(8., 0.), v2(10.5, 0.), v2(0., 1.), 5.0);
        assert!(!r.contains_quad(&q2, 0.01));
    }

    #[test]
    fn quad_overlap() {
        let a = Quad::on_base(v2(0., 0.), v2(2.5, 0.), v2(0., 1.), 5.0);
        let b = Quad::on_base(v2(2.5, 0.), v2(5.0, 0.), v2(0., 1.), 5.0);
        let c = Quad::on_base(v2(2.0, 0.), v2(4.5, 0.), v2(0., 1.), 5.0);
        assert!(!a.overlaps(&b));
        assert!(a.overlaps(&c));
    }
}
