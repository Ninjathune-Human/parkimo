//! Circulation : réseau des voies, sens de circulation et flèches au sol.
//!
//! 1. La chaussée (voirie, allées transversales, accès) est tramée au pas `RES` ; la carte des
//!    distances au bord donne la largeur locale, l'amincissement (Zhang-Suen) les axes.
//! 2. Les axes forment un graphe de carrefours, d'impasses et d'accès. Les éperons courts (angles,
//!    élargissements) sont élagués, les carrefours voisins fusionnés.
//! 3. Logique des automobilistes : on entre par un accès, on cherche une place, on ressort par un
//!    accès. Une voie assez large est à double sens ; une impasse aussi, puisqu'il faut en ressortir.
//!    Une voie plus étroite est à sens unique, orientée pour que chaque voie reste atteignable
//!    depuis la rue et que l'on puisse toujours regagner la rue (orientation fortement connexe),
//!    en tournant de préférence dans le sens anti-horaire, comme sur un giratoire.
//! 4. Flèches au sol : en sortie de carrefour, puis tous les `SPACING` mètres au plus ; flèche
//!    double (une par file, circulation à droite) ou flèche simple au centre de la voie.

use crate::geom::{v2, Quad, V2};
use crate::model::{LaneRule, P2};
use serde::Serialize;
use std::collections::VecDeque;

/// Pas de la trame (m).
const RES: f64 = 0.5;
/// Largeur minimale d'une voie à double sens (NF P 91-120 : 5,00 m ; NF P 91-100 : 5,50 m).
pub const TWO_WAY_MIN: f64 = 5.0;
/// Flèche directionnelle, longueur et largeur hors tout (= src/symbols.json, marquage « fleche »).
pub const ARROW_L: f64 = 3.0;
pub const ARROW_W: f64 = 0.9;
/// Intervalle maximal entre deux flèches sur une même voie (m).
const SPACING: f64 = 30.0;

#[derive(Serialize, Clone, Debug)]
pub struct Axis {
    /// Axe de la voie ; à sens unique, dans le sens de circulation.
    pub pts: Vec<P2>,
    /// Largeur de la voie (m).
    pub w: f64,
    /// 2 : double sens, 1 : sens unique.
    pub sens: u8,
}

#[derive(Serialize, Clone, Copy, Debug)]
pub struct Arrow {
    pub p: P2,
    /// Direction en degrés (sens trigonométrique depuis +x).
    pub a: f64,
}

#[derive(Serialize, Default, Clone, Debug)]
pub struct Circulation {
    pub axes: Vec<Axis>,
    pub arrows: Vec<Arrow>,
    /// Voies étroites laissées à double sens (impasses, passages obligés).
    pub narrow_two_way: usize,
    /// Sens uniques imposés mais refusés : la circulation serait coupée.
    pub refused: usize,
    /// Voies étroites imposées à double sens.
    pub forced_narrow: usize,
}

// ---------------------------------------------------------------------------
// Trame de la chaussée et carte des distances.

struct Grid {
    nx: usize,
    ny: usize,
    lo: V2,
    m: Vec<bool>,
    /// Distance au bord de la chaussée (m), au centre de chaque maille.
    d: Vec<f64>,
}

impl Grid {
    fn pos(&self, k: usize) -> V2 {
        v2(
            self.lo.x + ((k % self.nx) as f64 + 0.5) * RES,
            self.lo.y + ((k / self.nx) as f64 + 0.5) * RES,
        )
    }
    fn cell(&self, p: V2) -> Option<usize> {
        let i = ((p.x - self.lo.x) / RES).floor();
        let j = ((p.y - self.lo.y) / RES).floor();
        if i < 0.0 || j < 0.0 || i >= self.nx as f64 || j >= self.ny as f64 {
            return None;
        }
        Some(j as usize * self.nx + i as usize)
    }
    /// Distance au bord au point p (0 hors chaussée).
    fn dist(&self, p: V2) -> f64 {
        self.cell(p)
            .map_or(0.0, |k| if self.m[k] { self.d[k] } else { 0.0 })
    }
}

fn build_grid(rings: &[Vec<V2>], quads: &[Quad]) -> Option<Grid> {
    let mut lo = v2(f64::MAX, f64::MAX);
    let mut hi = v2(f64::MIN, f64::MIN);
    for p in rings
        .iter()
        .flatten()
        .chain(quads.iter().flat_map(|q| q.0.iter()))
    {
        lo = v2(lo.x.min(p.x), lo.y.min(p.y));
        hi = v2(hi.x.max(p.x), hi.y.max(p.y));
    }
    if lo.x > hi.x {
        return None;
    }
    let lo = lo - v2(3.0 * RES, 3.0 * RES);
    let nx = ((hi.x - lo.x) / RES).ceil() as usize + 4;
    let ny = ((hi.y - lo.y) / RES).ceil() as usize + 4;
    if nx * ny > 6_000_000 {
        return None;
    }
    let mut m = vec![false; nx * ny];
    // Anneaux pair-impair, par balayage horizontal : chaque côté inscrit ses intersections
    // dans les lignes qu'il traverse.
    let mut rows: Vec<Vec<f64>> = vec![Vec::new(); ny];
    for r in rings {
        let n = r.len();
        for i in 0..n {
            let (a, b) = (r[i], r[(i + 1) % n]);
            let (ya, yb) = (a.y.min(b.y), a.y.max(b.y));
            let j0 = ((ya - lo.y) / RES - 0.5).ceil().max(0.0) as usize;
            let j1 = ((yb - lo.y) / RES - 0.5).floor();
            if j1 < 0.0 {
                continue;
            }
            let j1 = (j1 as usize).min(ny - 1);
            if j0 > j1 {
                continue;
            }
            for (j, row) in rows.iter_mut().enumerate().take(j1 + 1).skip(j0) {
                let y = lo.y + (j as f64 + 0.5) * RES;
                if (a.y > y) != (b.y > y) {
                    row.push(a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y));
                }
            }
        }
    }
    for (j, xs) in rows.iter_mut().enumerate() {
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for pair in xs.chunks(2) {
            if pair.len() < 2 {
                break;
            }
            let i0 = ((pair[0] - lo.x) / RES - 0.5).ceil().max(0.0) as usize;
            let i1 = ((pair[1] - lo.x) / RES - 0.5).floor();
            if i1 < 0.0 {
                continue;
            }
            for i in i0..=(i1 as usize).min(nx - 1) {
                m[j * nx + i] = true;
            }
        }
    }
    for q in quads {
        let (a, b) = q.bbox();
        let i0 = ((a.x - lo.x) / RES).floor().max(0.0) as usize;
        let i1 = (((b.x - lo.x) / RES).ceil() as usize).min(nx - 1);
        let j0 = ((a.y - lo.y) / RES).floor().max(0.0) as usize;
        let j1 = (((b.y - lo.y) / RES).ceil() as usize).min(ny - 1);
        for j in j0..=j1 {
            for i in i0..=i1 {
                let c = v2(lo.x + (i as f64 + 0.5) * RES, lo.y + (j as f64 + 0.5) * RES);
                if q.contains(c) {
                    m[j * nx + i] = true;
                }
            }
        }
    }
    let d = distance_map(&m, nx, ny);
    Some(Grid { nx, ny, lo, m, d })
}

/// Transformée en distance euclidienne exacte, en mètres : distance du centre de chaque maille au
/// bord de la chaussée. Colonnes : distance verticale par deux balayages ligne à ligne (accès
/// mémoire contigus) ; lignes : enveloppe inférieure de paraboles (Felzenszwalb et Huttenlocher).
fn distance_map(m: &[bool], nx: usize, ny: usize) -> Vec<f64> {
    let far = (nx + ny) as f64;
    let mut g: Vec<f64> = vec![0.0; nx * ny];
    for j in 0..ny {
        for i in 0..nx {
            let k = j * nx + i;
            g[k] = if !m[k] {
                0.0
            } else if j == 0 {
                far
            } else {
                g[k - nx] + 1.0
            };
        }
    }
    for j in (0..ny.saturating_sub(1)).rev() {
        for i in 0..nx {
            let k = j * nx + i;
            let below = g[k + nx] + 1.0;
            if below < g[k] {
                g[k] = below;
            }
        }
    }
    let (mut f, mut out) = (vec![0.0; nx], vec![0.0; nx]);
    let (mut v, mut z) = (vec![0usize; nx], vec![0.0; nx + 1]);
    for j in 0..ny {
        for i in 0..nx {
            let x = g[j * nx + i];
            f[i] = x * x;
        }
        edt_1d(&f, &mut out, &mut v, &mut z);
        for i in 0..nx {
            // Distance entre centres de mailles, ramenée au bord : d − RES/2.
            g[j * nx + i] = (out[i].sqrt() * RES - RES * 0.5).max(0.0);
        }
    }
    g
}

fn edt_1d(f: &[f64], d: &mut [f64], v: &mut [usize], z: &mut [f64]) {
    let n = f.len();
    let sect = |q: usize, p: usize| {
        let (qf, pf) = (q as f64, p as f64);
        ((f[q] + qf * qf) - (f[p] + pf * pf)) / (2.0 * qf - 2.0 * pf)
    };
    let mut k = 0usize;
    v[0] = 0;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;
    for q in 1..n {
        let mut s = sect(q, v[k]);
        while s <= z[k] {
            k -= 1;
            s = sect(q, v[k]);
        }
        k += 1;
        v[k] = q;
        z[k] = s;
        z[k + 1] = f64::INFINITY;
    }
    k = 0;
    for (q, dq) in d.iter_mut().enumerate().take(n) {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        let dx = q as f64 - v[k] as f64;
        *dq = dx * dx + f[v[k]];
    }
}

// ---------------------------------------------------------------------------
// Axes : amincissement et graphe.

/// Voisins dans l'ordre N, NE, E, SE, S, SO, O, NO.
const NB: [(i64, i64); 8] = [
    (0, 1),
    (1, 1),
    (1, 0),
    (1, -1),
    (0, -1),
    (-1, -1),
    (-1, 0),
    (-1, 1),
];

/// Voisins d'un pixel intérieur (jamais au bord de la trame, qui est vide).
fn neighbours(k: usize, nx: usize) -> [usize; 8] {
    NB.map(|(di, dj)| (k as i64 + dj * nx as i64 + di) as usize)
}

/// Amincissement de Zhang-Suen puis suppression des marches d'escalier (axe 8-connexe d'un pixel).
fn skeleton(g: &Grid) -> Vec<bool> {
    let (nx, ny) = (g.nx, g.ny);
    let mut s = g.m.clone();
    let inner = |k: usize| {
        let (i, j) = (k % nx, k / nx);
        i > 0 && j > 0 && i + 1 < nx && j + 1 < ny
    };
    let mut del = Vec::new();
    // Pixels encore pleins : la liste rétrécit à chaque passe.
    let mut live: Vec<usize> = (0..s.len()).filter(|&k| s[k] && inner(k)).collect();
    loop {
        let mut changed = false;
        for step in 0..2 {
            del.clear();
            for &k in &live {
                if !s[k] {
                    continue;
                }
                let p: [bool; 8] = neighbours(k, nx).map(|n| s[n]);
                let b = p.iter().filter(|&&x| x).count();
                if !(2..=6).contains(&b) {
                    continue;
                }
                let a = (0..8).filter(|&t| !p[t] && p[(t + 1) % 8]).count();
                if a != 1 {
                    continue;
                }
                let (n_, e_, s_, w_) = (p[0], p[2], p[4], p[6]);
                let ok = if step == 0 {
                    !(n_ && e_ && s_) && !(e_ && s_ && w_)
                } else {
                    !(n_ && e_ && w_) && !(n_ && s_ && w_)
                };
                if ok {
                    del.push(k);
                }
            }
            for &k in &del {
                s[k] = false;
            }
            changed |= !del.is_empty();
        }
        live.retain(|&k| s[k]);
        if !changed {
            break;
        }
    }
    // Marches d'escalier : un pixel dont les deux seuls voisins se touchent est superflu.
    for &k in &live {
        if !s[k] {
            continue;
        }
        let nb: Vec<usize> = neighbours(k, nx).into_iter().filter(|&n| s[n]).collect();
        if nb.len() == 2 {
            let (a, b) = (nb[0], nb[1]);
            let (ai, aj, bi, bj) = (
                (a % nx) as i64,
                (a / nx) as i64,
                (b % nx) as i64,
                (b / nx) as i64,
            );
            if (ai - bi).abs() <= 1 && (aj - bj).abs() <= 1 {
                s[k] = false;
            }
        }
    }
    s
}

struct Node {
    p: V2,
    /// Rayon du carrefour : distance au bord de la chaussée en son centre.
    r: f64,
    entry: Option<usize>,
    alive: bool,
}

struct Edge {
    a: usize,
    b: usize,
    pts: Vec<V2>,
    /// Distance au bord en chaque point.
    ds: Vec<f64>,
    alive: bool,
}

/// Largeur de voie mesurée sur l'axe tramé : l'axe s'écarte d'au plus RES/2 du vrai milieu,
/// d'où une sous-estimation moyenne de RES/2, compensée ici.
fn lane_width(d: f64) -> f64 {
    2.0 * d + RES * 0.5
}

/// Distance d'un point à une polyligne.
fn dist_to_poly(p: V2, poly: &[V2]) -> f64 {
    poly.windows(2)
        .map(|s| {
            let (a, b) = (s[0], s[1]);
            let ab = b - a;
            let l2 = ab.dot(ab);
            let t = if l2 > 0.0 {
                ((p - a).dot(ab) / l2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (a + ab * t - p).len()
        })
        .fold(f64::INFINITY, f64::min)
}

fn poly_len(p: &[V2]) -> f64 {
    p.windows(2).map(|w| (w[1] - w[0]).len()).sum()
}

struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

impl Graph {
    fn degree(&self, n: usize) -> usize {
        self.edges
            .iter()
            .filter(|e| e.alive)
            .map(|e| (e.a == n) as usize + (e.b == n) as usize)
            .sum()
    }
    fn incident(&self, n: usize) -> Vec<usize> {
        (0..self.edges.len())
            .filter(|&i| self.edges[i].alive && (self.edges[i].a == n || self.edges[i].b == n))
            .collect()
    }
}

fn trace_graph(g: &Grid, s: &[bool]) -> Graph {
    let nx = g.nx;
    let nbs = |k: usize| -> Vec<usize> {
        neighbours(k, nx)
            .into_iter()
            .filter(|&n| n < s.len() && s[n])
            .collect()
    };
    let is_node: Vec<bool> = (0..s.len()).map(|k| s[k] && nbs(k).len() != 2).collect();
    // Groupes de pixels-nœuds contigus → un nœud au point le plus éloigné des bords.
    let mut cluster = vec![usize::MAX; s.len()];
    let mut nodes: Vec<Node> = Vec::new();
    for k in 0..s.len() {
        if !is_node[k] || cluster[k] != usize::MAX || nbs(k).is_empty() {
            continue;
        }
        let id = nodes.len();
        let mut best = k;
        let mut q = VecDeque::from([k]);
        cluster[k] = id;
        while let Some(c) = q.pop_front() {
            if g.d[c] > g.d[best] {
                best = c;
            }
            for n in nbs(c) {
                if is_node[n] && cluster[n] == usize::MAX {
                    cluster[n] = id;
                    q.push_back(n);
                }
            }
        }
        nodes.push(Node {
            p: g.pos(best),
            r: g.d[best],
            entry: None,
            alive: true,
        });
    }
    let mut edges: Vec<Edge> = Vec::new();
    let mut seen = vec![false; s.len()];
    let walk = |start: usize, first: usize, seen: &mut Vec<bool>| -> (Vec<usize>, usize) {
        let mut chain = vec![start, first];
        let (mut prev, mut cur) = (start, first);
        while !is_node[cur] {
            seen[cur] = true;
            let next = nbs(cur).into_iter().find(|&n| {
                n != prev
                    && (is_node[n] || !seen[n])
                    && !(chain.len() > 2 && n == chain[chain.len() - 3])
            });
            match next {
                Some(n) => {
                    prev = cur;
                    cur = n;
                    chain.push(n);
                }
                None => break,
            }
            if cur == start {
                break;
            }
        }
        (chain, cur)
    };
    for k in 0..s.len() {
        if cluster[k] == usize::MAX {
            continue;
        }
        for n in nbs(k) {
            if is_node[n] || seen[n] {
                continue;
            }
            let (chain, end) = walk(k, n, &mut seen);
            if cluster[end] == usize::MAX {
                continue;
            }
            edges.push(Edge {
                a: cluster[k],
                b: cluster[end],
                pts: chain.iter().map(|&c| g.pos(c)).collect(),
                ds: chain.iter().map(|&c| g.d[c]).collect(),
                alive: true,
            });
        }
    }
    // Boucles sans carrefour : un nœud artificiel sur la boucle.
    for k in 0..s.len() {
        if !s[k] || is_node[k] || seen[k] {
            continue;
        }
        let id = nodes.len();
        nodes.push(Node {
            p: g.pos(k),
            r: g.d[k],
            entry: None,
            alive: true,
        });
        seen[k] = true;
        let first = nbs(k)[0];
        let mut chain = vec![k, first];
        let (mut prev, mut cur) = (k, first);
        while cur != k {
            seen[cur] = true;
            match nbs(cur)
                .into_iter()
                .find(|&n| n != prev && (n == k || !seen[n]))
            {
                Some(n) => {
                    prev = cur;
                    cur = n;
                    chain.push(n);
                }
                None => break,
            }
        }
        if chain.len() > 8 {
            edges.push(Edge {
                a: id,
                b: id,
                pts: chain.iter().map(|&c| g.pos(c)).collect(),
                ds: chain.iter().map(|&c| g.d[c]).collect(),
                alive: true,
            });
        } else {
            nodes[id].alive = false;
        }
    }
    Graph { nodes, edges }
}

/// Élagage des éperons, fusion des carrefours voisins, réunion des tronçons consécutifs.
fn simplify_graph(gr: &mut Graph) {
    loop {
        let mut changed = false;
        // Éperons : branche courte qui part d'un carrefour vers un bord (angle, élargissement).
        for e in 0..gr.edges.len() {
            if !gr.edges[e].alive {
                continue;
            }
            let (a, b) = (gr.edges[e].a, gr.edges[e].b);
            if a == b {
                continue;
            }
            let len = poly_len(&gr.edges[e].pts);
            for (leaf, other) in [(a, b), (b, a)] {
                if gr.nodes[leaf].entry.is_some() || gr.degree(leaf) != 1 {
                    continue;
                }
                let od = gr.degree(other);
                let spur = od >= 3 && len < 1.6 * gr.nodes[other].r + 1.0;
                let crumb = od == 1 && gr.nodes[other].entry.is_none() && len < 3.0;
                if spur || crumb {
                    gr.edges[e].alive = false;
                    gr.nodes[leaf].alive = false;
                    if crumb {
                        gr.nodes[other].alive = false;
                    }
                    changed = true;
                    break;
                }
            }
        }
        // Carrefours voisins : deux nœuds reliés par un tronçon très court n'en font qu'un.
        for e in 0..gr.edges.len() {
            if !gr.edges[e].alive {
                continue;
            }
            let (a, b) = (gr.edges[e].a, gr.edges[e].b);
            if a == b || gr.degree(a) < 3 || gr.degree(b) < 3 {
                continue;
            }
            if poly_len(&gr.edges[e].pts) < 2.0 + 0.3 * gr.nodes[a].r.min(gr.nodes[b].r) {
                gr.edges[e].alive = false;
                if gr.nodes[b].r > gr.nodes[a].r {
                    gr.nodes[a].p = gr.nodes[b].p;
                    gr.nodes[a].r = gr.nodes[b].r;
                }
                for f in gr.edges.iter_mut().filter(|f| f.alive) {
                    if f.a == b {
                        f.a = a;
                    }
                    if f.b == b {
                        f.b = a;
                    }
                }
                gr.nodes[b].alive = false;
                changed = true;
            }
        }
        // Nœuds de degré 2 : les deux tronçons n'en font qu'un.
        for n in 0..gr.nodes.len() {
            if !gr.nodes[n].alive || gr.nodes[n].entry.is_some() || gr.degree(n) != 2 {
                continue;
            }
            let inc = gr.incident(n);
            if inc.len() != 2 {
                continue;
            }
            let (e1, e2) = (inc[0], inc[1]);
            let oriented = |gr: &Graph, e: usize, from: usize| -> (Vec<V2>, Vec<f64>, usize) {
                let ed = &gr.edges[e];
                if ed.a == from {
                    (ed.pts.clone(), ed.ds.clone(), ed.b)
                } else {
                    let mut p = ed.pts.clone();
                    let mut d = ed.ds.clone();
                    p.reverse();
                    d.reverse();
                    (p, d, ed.a)
                }
            };
            // e1 parcouru vers n, puis e2 depuis n.
            let (mut p1, mut d1, start) = oriented(gr, e1, n);
            p1.reverse();
            d1.reverse();
            let (p2, d2, end) = oriented(gr, e2, n);
            p1.extend_from_slice(&p2[1..]);
            d1.extend_from_slice(&d2[1..]);
            gr.edges[e1] = Edge {
                a: start,
                b: end,
                pts: p1,
                ds: d1,
                alive: true,
            };
            gr.edges[e2].alive = false;
            gr.nodes[n].alive = false;
            changed = true;
        }
        if !changed {
            break;
        }
    }
}

/// Douglas-Peucker.
fn simplify(p: &[V2], tol: f64) -> Vec<V2> {
    if p.len() < 3 {
        return p.to_vec();
    }
    let mut keep = vec![false; p.len()];
    keep[0] = true;
    keep[p.len() - 1] = true;
    let mut stack = vec![(0usize, p.len() - 1)];
    while let Some((i, j)) = stack.pop() {
        let (a, b) = (p[i], p[j]);
        let ab = b - a;
        let l = ab.len();
        let mut best = (0.0, i);
        for (k, &q) in p.iter().enumerate().take(j).skip(i + 1) {
            let d = if l < 1e-9 {
                (q - a).len()
            } else {
                (q - a).cross(ab).abs() / l
            };
            if d > best.0 {
                best = (d, k);
            }
        }
        if best.0 > tol {
            keep[best.1] = true;
            stack.push((i, best.1));
            stack.push((best.1, j));
        }
    }
    p.iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(q, _)| *q)
        .collect()
}

/// Point et tangente à l'abscisse curviligne s.
fn at(p: &[V2], s: f64) -> (V2, V2) {
    let mut acc = 0.0;
    let last = p.len().saturating_sub(2);
    for (i, w) in p.windows(2).enumerate() {
        let l = (w[1] - w[0]).len();
        if acc + l >= s || i == last {
            let t = if l > 1e-9 {
                ((s - acc) / l).clamp(0.0, 1.0)
            } else {
                0.0
            };
            return (w[0] + (w[1] - w[0]) * t, (w[1] - w[0]).norm());
        }
        acc += l;
    }
    (p[0], v2(1.0, 0.0))
}

fn direction(p: &[V2], s: f64, len: f64) -> V2 {
    let (a, _) = at(p, (s - 1.5).max(0.0));
    let (b, _) = at(p, (s + 1.5).min(len));
    let d = (b - a).norm();
    if d.len() < 0.5 {
        at(p, s).1
    } else {
        d
    }
}

// ---------------------------------------------------------------------------
// Sens de circulation.

/// Ponts (arêtes dont la suppression déconnecte) du multigraphe non orienté.
fn bridges(n: usize, ed: &[(usize, usize)]) -> Vec<bool> {
    let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    for (i, &(a, b)) in ed.iter().enumerate() {
        adj[a].push((b, i));
        if a != b {
            adj[b].push((a, i));
        }
    }
    let mut tin = vec![usize::MAX; n];
    let mut low = vec![0usize; n];
    let mut out = vec![false; ed.len()];
    let mut timer = 0;
    for root in 0..n {
        if tin[root] != usize::MAX {
            continue;
        }
        // Parcours en profondeur itératif : (nœud, arête d'arrivée, rang du voisin suivant).
        let mut st: Vec<(usize, usize, usize)> = vec![(root, usize::MAX, 0)];
        tin[root] = timer;
        low[root] = timer;
        timer += 1;
        while let Some(&mut (v, pe, ref mut it)) = st.last_mut() {
            if *it < adj[v].len() {
                let (w, ei) = adj[v][*it];
                *it += 1;
                if ei == pe {
                    continue;
                }
                if tin[w] == usize::MAX {
                    tin[w] = timer;
                    low[w] = timer;
                    timer += 1;
                    st.push((w, ei, 0));
                } else {
                    low[v] = low[v].min(tin[w]);
                }
            } else {
                st.pop();
                if let Some(&(u, _, _)) = st.last() {
                    low[u] = low[u].min(low[v]);
                    if low[v] > tin[u] {
                        out[pe] = true;
                    }
                }
            }
        }
    }
    out
}

/// Tout nœud de `comp` est atteignable depuis `root` et l'atteint. `dir[i]` : 0 double sens,
/// +1 de a vers b, −1 de b vers a.
fn strongly_connected(
    n: usize,
    ed: &[(usize, usize)],
    dir: &[i8],
    root: usize,
    comp: &[bool],
) -> bool {
    for reverse in [false, true] {
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, &(a, b)) in ed.iter().enumerate() {
            let d = if reverse { -dir[i] } else { dir[i] };
            if d >= 0 {
                adj[a].push(b);
            }
            if d <= 0 {
                adj[b].push(a);
            }
        }
        let mut seen = vec![false; n];
        let mut q = VecDeque::from([root]);
        seen[root] = true;
        while let Some(v) = q.pop_front() {
            for &w in &adj[v] {
                if !seen[w] {
                    seen[w] = true;
                    q.push_back(w);
                }
            }
        }
        if (0..n).any(|v| comp[v] && !seen[v]) {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------

/// `rings` : chaussée (anneaux pair-impair) ; `quads` : allées transversales et accès ;
/// `entries` : accès depuis la rue ; `aisle` : largeur d'allée retenue par le plan.
pub fn analyse(
    rings: &[Vec<V2>],
    quads: &[Quad],
    entries: &[Quad],
    aisle: f64,
    rules: &[LaneRule],
) -> Circulation {
    let mut out = Circulation::default();
    let Some(g) = build_grid(rings, quads) else {
        return out;
    };
    let sk = skeleton(&g);
    let mut gr = trace_graph(&g, &sk);
    // Accès : l'extrémité d'axe la plus proche de la rue dans chaque rectangle d'accès, recalée au
    // milieu de son entrée ; les autres extrémités (fourches de bout d'axe) seront élaguées.
    for (i, e) in entries.iter().enumerate() {
        let gate = (e.0[0] + e.0[1]) * 0.5;
        let inward = (e.0[3] - e.0[0]).norm();
        let best = (0..gr.nodes.len())
            .filter(|&n| gr.nodes[n].alive && gr.degree(n) == 1 && e.contains(gr.nodes[n].p))
            .min_by(|&a, &b| {
                (gr.nodes[a].p - gate)
                    .len()
                    .partial_cmp(&(gr.nodes[b].p - gate).len())
                    .unwrap()
            });
        if let Some(n) = best {
            gr.nodes[n].entry = Some(i);
            gr.nodes[n].p = gate + inward * 0.5;
        }
    }
    simplify_graph(&mut gr);

    // Tronçons retenus : axe simplifié, largeur médiane hors carrefours.
    let ids: Vec<usize> = (0..gr.edges.len()).filter(|&e| gr.edges[e].alive).collect();
    let nn = gr.nodes.len();
    let xnode = nn; // la rue, reliée à chaque accès
    let mut ed: Vec<(usize, usize)> = Vec::new();
    let mut width: Vec<f64> = Vec::new();
    let mut polys: Vec<Vec<V2>> = Vec::new();
    let mut raws: Vec<Vec<V2>> = Vec::new();
    for &e in &ids {
        let edg = &gr.edges[e];
        let len = poly_len(&edg.pts);
        let (ra, rb) = (gr.nodes[edg.a].r, gr.nodes[edg.b].r);
        let mut acc = 0.0;
        let mut ws: Vec<f64> = Vec::new();
        for i in 0..edg.pts.len() {
            if i > 0 {
                acc += (edg.pts[i] - edg.pts[i - 1]).len();
            }
            if acc > ra + 0.5 && acc < len - rb - 0.5 {
                ws.push(lane_width(edg.ds[i]));
            }
        }
        if ws.is_empty() {
            ws = edg.ds.iter().map(|&d| lane_width(d)).collect();
        }
        ws.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut w = ws[ws.len() / 2];
        // Voie au gabarit de l'allée : on retient la largeur réglée, au lieu de la mesure tramée.
        if (w - aisle).abs() < 0.5 {
            w = aisle;
        }
        // Axe lissé (les marches de la trame disparaissent sous la tolérance) ; l'axe publié est
        // raccordé aux centres des carrefours, les flèches suivent l'axe mesuré.
        let mut raw = simplify(&edg.pts, 0.6);
        // Côté rue, l'axe mesuré s'arrête avant la limite : il est prolongé jusqu'à l'entrée.
        if gr.nodes[edg.a].entry.is_some() {
            raw[0] = gr.nodes[edg.a].p;
        }
        if gr.nodes[edg.b].entry.is_some() {
            let k = raw.len() - 1;
            raw[k] = gr.nodes[edg.b].p;
        }
        let mut pts = raw.clone();
        pts[0] = gr.nodes[edg.a].p;
        let last = pts.len() - 1;
        pts[last] = gr.nodes[edg.b].p;
        ed.push((edg.a, edg.b));
        width.push(w);
        polys.push(pts);
        raws.push(raw);
    }
    let real = ed.len();
    for (n, node) in gr.nodes.iter().enumerate() {
        if node.alive && node.entry.is_some() {
            ed.push((n, xnode));
        }
    }
    let nv = nn + 1;
    // Composante desservie : depuis la rue, ou à défaut la plus grande.
    let has_entry = ed.len() > real;
    let mut comp = vec![false; nv];
    {
        let root = if has_entry {
            xnode
        } else {
            match (0..real).max_by(|&a, &b| {
                poly_len(&polys[a])
                    .partial_cmp(&poly_len(&polys[b]))
                    .unwrap()
            }) {
                Some(e) => ed[e].0,
                None => return out,
            }
        };
        let mut q = VecDeque::from([root]);
        comp[root] = true;
        while let Some(v) = q.pop_front() {
            for &(a, b) in &ed {
                let w = if a == v {
                    b
                } else if b == v {
                    a
                } else {
                    continue;
                };
                if !comp[w] {
                    comp[w] = true;
                    q.push_back(w);
                }
            }
        }
    }
    let root = if has_entry {
        xnode
    } else {
        (0..nv).find(|&v| comp[v]).unwrap_or(0)
    };
    let deg = |n: usize| {
        ed.iter()
            .map(|&(a, b)| (a == n) as usize + (b == n) as usize)
            .sum::<usize>()
    };
    let br = bridges(nv, &ed);
    // Typologies imposées : chaque repère s'applique au tronçon d'axe le plus proche, s'il passe
    // sur sa chaussée ; à repères multiples, le dernier l'emporte.
    let mut imposed: Vec<Option<u8>> = vec![None; real];
    for r in rules {
        let p = v2(r.p[0], r.p[1]);
        let best = (0..real)
            .map(|e| (e, dist_to_poly(p, &polys[e])))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        if let Some((e, d)) = best {
            if d <= (width[e] * 0.5).max(1.5) {
                imposed[e] = Some(r.sens);
            }
        }
    }
    let mut dir = vec![0i8; ed.len()];
    let mut narrow: Vec<usize> = Vec::new();
    let mut forced: Vec<usize> = Vec::new();
    for e in 0..real {
        if !comp[ed[e].0] {
            continue;
        }
        match imposed[e] {
            Some(1) => {
                forced.push(e);
                continue;
            }
            Some(_) => {
                if width[e] < TWO_WAY_MIN {
                    out.forced_narrow += 1;
                }
                continue;
            }
            None => {}
        }
        if width[e] >= TWO_WAY_MIN {
            continue;
        }
        let dead_end = deg(ed[e].0) == 1 || deg(ed[e].1) == 1;
        if dead_end || br[e] {
            out.narrow_two_way += 1;
        } else {
            narrow.push(e);
        }
    }
    // Sens préféré : anti-horaire autour du centre de la partie desservie.
    let pts_c: Vec<V2> = (0..real)
        .filter(|&e| comp[ed[e].0])
        .flat_map(|e| polys[e].clone())
        .collect();
    let centre = if pts_c.is_empty() {
        v2(0.0, 0.0)
    } else {
        pts_c.iter().fold(v2(0.0, 0.0), |s, &p| s + p) * (1.0 / pts_c.len() as f64)
    };
    let by_len = |a: &usize, b: &usize| {
        poly_len(&polys[*b])
            .partial_cmp(&poly_len(&polys[*a]))
            .unwrap()
    };
    narrow.sort_by(by_len);
    forced.sort_by(by_len);
    // Sens uniques imposés d'abord, puis ceux des voies étroites, sans couper la circulation.
    let nf = forced.len();
    for (k, e) in forced.into_iter().chain(narrow).enumerate() {
        let len = poly_len(&polys[e]);
        let (mid, _) = at(&polys[e], len / 2.0);
        let t = direction(&polys[e], len / 2.0, len);
        let pref: i8 = if (mid - centre).cross(t) >= 0.0 {
            1
        } else {
            -1
        };
        for d in [pref, -pref] {
            dir[e] = d;
            if strongly_connected(nv, &ed, &dir, root, &comp) {
                break;
            }
            dir[e] = 0;
        }
        if dir[e] == 0 {
            if k < nf {
                out.refused += 1;
            } else {
                out.narrow_two_way += 1;
            }
        }
    }

    // Axes et flèches.
    let mut placed: Vec<Quad> = Vec::new();
    let fits = |c: V2, t: V2, placed: &[Quad]| -> Option<Quad> {
        let n = t.left();
        let (hl, hw) = (ARROW_L * 0.5, ARROW_W * 0.5);
        let q = Quad([
            c - t * hl - n * hw,
            c + t * hl - n * hw,
            c + t * hl + n * hw,
            c - t * hl + n * hw,
        ]);
        let samples = [
            q.0[0],
            q.0[1],
            q.0[2],
            q.0[3],
            c,
            c + t * hl,
            c - t * hl,
            c + n * hw,
            c - n * hw,
        ];
        if samples.iter().any(|&p| g.dist(p) < 0.25) {
            return None;
        }
        let big = q.shrink(-0.4);
        if placed.iter().any(|o| o.overlaps(&big)) {
            return None;
        }
        Some(q)
    };
    for e in 0..real {
        let mut axis = polys[e].clone();
        let mut pts = raws[e].clone();
        let (mut na, mut nb) = ed[e];
        if dir[e] < 0 {
            axis.reverse();
            pts.reverse();
            std::mem::swap(&mut na, &mut nb);
        }
        let len = poly_len(&pts);
        out.axes.push(Axis {
            pts: axis.iter().map(|p| [r3(p.x), r3(p.y)]).collect(),
            w: r3(width[e]),
            sens: if dir[e] == 0 { 2 } else { 1 },
        });
        if !comp[ed[e].0] || len < ARROW_L + 1.0 {
            continue;
        }
        // Tronçons droits : l'axe est coupé aux virages marqués ; chaque tronçon reçoit une flèche
        // à sa sortie de carrefour ou de virage, puis tous les SPACING mètres.
        // Dégagement : hors du carrefour, à 2 m d'un accès ou d'un fond d'impasse, hors du virage.
        let clear = |n: usize| {
            if deg(n) >= 3 {
                gr.nodes[n].r + 1.0
            } else {
                2.0
            }
        };
        let corner = width[e] * 0.5 + 1.0;
        let mut cuts: Vec<(f64, f64)> = vec![(0.0, clear(na))];
        let mut acc = 0.0;
        for i in 1..pts.len().saturating_sub(1) {
            acc += (pts[i] - pts[i - 1]).len();
            let (u, v) = ((pts[i] - pts[i - 1]).norm(), (pts[i + 1] - pts[i]).norm());
            if u.cross(v).atan2(u.dot(v)).abs() > 25f64.to_radians() {
                cuts.push((acc, corner));
            }
        }
        cuts.push((len, clear(nb)));
        let two = dir[e] == 0;
        let mut stations: Vec<f64> = Vec::new();
        for run in cuts.windows(2) {
            let (a, ca) = run[0];
            let (b, cb) = run[1];
            let (s0, s1) = (a + ca + ARROW_L * 0.5, b - cb - ARROW_L * 0.5);
            if s1 < s0 {
                if b - a - ca - cb >= ARROW_L * 0.8 {
                    stations.push((a + ca + b - cb) * 0.5);
                }
            } else if s1 - s0 < 8.0 {
                stations.push((s0 + s1) * 0.5);
            } else if two {
                // Double sens : une paire à chaque bout (entrée de file de chaque sens), et entre
                // les deux au-delà de deux intervalles.
                let k = ((s1 - s0) / (2.0 * SPACING)).ceil() as usize + 1;
                for i in 0..k {
                    stations.push(s0 + (s1 - s0) * i as f64 / (k - 1) as f64);
                }
            } else {
                // Sens unique : en entrée du tronçon, puis tous les SPACING mètres.
                let mut x = s0;
                while x <= s1 + 1e-9 {
                    stations.push(x);
                    x += SPACING;
                }
            }
        }
        let off = (width[e] * 0.25).min(1.6);
        for s in stations {
            for shift in [0.0, 0.6, -0.6, 1.2, -1.2, 1.8, -1.8, 2.4, -2.4, 3.0, -3.0] {
                let ss = s + shift;
                if ss < ARROW_L * 0.5 || ss > len - ARROW_L * 0.5 {
                    continue;
                }
                let (p, _) = at(&pts, ss);
                let t = direction(&pts, ss, len);
                // La flèche se pose sur une partie droite de l'axe.
                let (ta, tb) = (
                    direction(&pts, (ss - ARROW_L * 0.5).max(0.0), len),
                    direction(&pts, (ss + ARROW_L * 0.5).min(len), len),
                );
                if ta.dot(tb) < 12f64.to_radians().cos() {
                    continue;
                }
                let right = v2(t.y, -t.x);
                // Circulation à droite : la flèche d'un sens occupe la file de droite de ce sens.
                let group: Vec<(V2, V2)> = if two {
                    vec![(p + right * off, t), (p - right * off, -t)]
                } else {
                    vec![(p, t)]
                };
                let mut qs = Vec::new();
                for &(c, d) in &group {
                    let mut tmp = placed.clone();
                    tmp.extend(qs.iter().copied());
                    match fits(c, d, &tmp) {
                        Some(q) => qs.push(q),
                        None => break,
                    }
                }
                if qs.len() == group.len() {
                    for (&(c, d), q) in group.iter().zip(qs) {
                        placed.push(q);
                        out.arrows.push(Arrow {
                            p: [r3(c.x), r3(c.y)],
                            a: r3(d.y.atan2(d.x).to_degrees()),
                        });
                    }
                    break;
                }
            }
        }
    }
    out
}

fn r3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<V2> {
        vec![v2(x0, y0), v2(x1, y0), v2(x1, y1), v2(x0, y1)]
    }

    /// Accès : base sur la limite (a → b), profondeur vers n, comme dans le moteur.
    fn entry(a: V2, b: V2, n: V2, depth: f64) -> Quad {
        Quad::on_base(a, b, n, depth)
    }

    #[test]
    fn distance_map_measures_lane_width() {
        let g = build_grid(&[rect(0.0, 0.0, 40.0, 6.0)], &[]).unwrap();
        let w = lane_width(g.dist(v2(20.0, 3.0)));
        assert!((w - 6.0).abs() < 0.4, "largeur {w}");
    }

    #[test]
    fn wide_corridor_is_two_way_with_paired_arrows_on_the_right() {
        // Allée de 50 × 6 m, accès à l'ouest.
        let e = entry(v2(-8.0, 6.0), v2(-8.0, 0.0), v2(1.0, 0.0), 8.5);
        let c = analyse(&[rect(0.0, 0.0, 50.0, 6.0)], &[e], &[e], 6.0, &[]);
        assert!(
            !c.axes.is_empty() && c.axes.iter().all(|a| a.sens == 2),
            "{:?}",
            c.axes
        );
        assert!(
            c.arrows.len() >= 4 && c.arrows.len() % 2 == 0,
            "{} flèches",
            c.arrows.len()
        );
        // Circulation à droite : vers l'est (a ≈ 0°) au sud de l'axe, vers l'ouest au nord.
        for a in &c.arrows {
            let east = a.a.to_radians().cos() > 0.9;
            let west = a.a.to_radians().cos() < -0.9;
            assert!(east || west, "flèche oblique {}", a.a);
            if east {
                assert!(a.p[1] < 3.0, "flèche vers l'est au nord : {:?}", a.p);
            } else {
                assert!(a.p[1] > 3.0, "flèche vers l'ouest au sud : {:?}", a.p);
            }
        }
    }

    #[test]
    fn narrow_ring_turns_counterclockwise_and_entry_stays_two_way() {
        // Anneau de 4 m de large autour d'un îlot, un seul accès au sud.
        let outer = rect(0.0, 0.0, 60.0, 40.0);
        let mut inner = rect(4.0, 4.0, 56.0, 36.0);
        inner.reverse();
        let e = entry(v2(28.0, -10.0), v2(32.0, -10.0), v2(0.0, 1.0), 10.5);
        let c = analyse(&[outer, inner], &[e], &[e], 4.0, &[]);
        let one_way: Vec<&Axis> = c.axes.iter().filter(|a| a.sens == 1).collect();
        assert!(!one_way.is_empty(), "{:?}", c.axes);
        assert_eq!(c.narrow_two_way, 1, "l'accès unique reste à double sens");
        // Sens anti-horaire : sur le côté sud de l'anneau on roule vers l'est.
        let south: Vec<&Arrow> = c
            .arrows
            .iter()
            .filter(|a| a.p[1] > 0.0 && a.p[1] < 4.0)
            .collect();
        assert!(!south.is_empty(), "{:?}", c.arrows);
        assert!(
            south.iter().all(|a| a.a.to_radians().cos() > 0.9),
            "{south:?}"
        );
        // Un seul accès : flèches doubles dans l'accès, flèches simples sur l'anneau.
        let in_entry = c.arrows.iter().filter(|a| a.p[1] < 0.0).count();
        assert_eq!(in_entry % 2, 0);
    }

    #[test]
    fn two_entries_make_a_one_way_crossing() {
        // Voie étroite traversante : entrée à l'ouest, sortie à l'est, sens unique.
        let w = entry(v2(-8.0, 4.0), v2(-8.0, 0.0), v2(1.0, 0.0), 8.5);
        let east = entry(v2(58.0, 0.0), v2(58.0, 4.0), v2(-1.0, 0.0), 8.5);
        let c = analyse(
            &[rect(0.0, 0.0, 50.0, 4.0)],
            &[w, east],
            &[w, east],
            4.0,
            &[],
        );
        assert!(c.axes.iter().any(|a| a.sens == 1), "{:?}", c.axes);
        let dirs: Vec<f64> = c.arrows.iter().map(|a| a.a.to_radians().cos()).collect();
        assert!(
            !dirs.is_empty() && (dirs.iter().all(|&d| d > 0.9) || dirs.iter().all(|&d| d < -0.9)),
            "{dirs:?}"
        );
    }

    /// Anneau de largeur `w` autour d'un îlot de 60 × 40 m, accès unique au sud.
    fn ring(w: f64, rules: &[LaneRule]) -> Circulation {
        let outer = rect(0.0, 0.0, 60.0, 40.0);
        let mut inner = rect(w, w, 60.0 - w, 40.0 - w);
        inner.reverse();
        let e = entry(v2(28.0, -10.0), v2(32.0, -10.0), v2(0.0, 1.0), 10.5);
        analyse(&[outer, inner], &[e], &[e], w, rules)
    }

    fn rule(x: f64, y: f64, sens: u8) -> LaneRule {
        LaneRule { p: [x, y], sens }
    }

    #[test]
    fn imposed_one_way_applies_to_the_touched_axis_only() {
        let c = ring(6.0, &[]);
        assert!(c.axes.iter().all(|a| a.sens == 2), "{:?}", c.axes);
        let c = ring(6.0, &[rule(30.0, 37.0, 1)]);
        assert_eq!(c.refused, 0);
        let ring_axis = c
            .axes
            .iter()
            .find(|a| a.pts.iter().any(|p| p[1] > 30.0))
            .unwrap();
        assert_eq!(ring_axis.sens, 1, "{:?}", c.axes);
        assert_eq!(
            c.axes.iter().filter(|a| a.sens == 2).count(),
            c.axes.len() - 1
        );
        // Sens unique : flèches simples, anti-horaire (vers l'ouest au nord).
        let north: Vec<&Arrow> = c.arrows.iter().filter(|a| a.p[1] > 34.0).collect();
        assert!(!north.is_empty());
        assert!(
            north.iter().all(|a| a.a.to_radians().cos() < -0.9),
            "{north:?}"
        );
    }

    #[test]
    fn imposed_one_way_that_would_cut_the_site_is_refused() {
        // L'accès unique est un passage obligé : à sens unique, on ne pourrait plus ressortir.
        let c = ring(6.0, &[rule(30.0, -5.0, 1)]);
        assert_eq!(c.refused, 1);
        assert!(c.axes.iter().all(|a| a.sens == 2), "{:?}", c.axes);
    }

    #[test]
    fn imposed_two_way_overrides_a_narrow_lane() {
        assert!(ring(4.0, &[]).axes.iter().any(|a| a.sens == 1));
        let c = ring(4.0, &[rule(30.0, 38.0, 2)]);
        assert!(c.axes.iter().all(|a| a.sens == 2), "{:?}", c.axes);
        assert_eq!(c.forced_narrow, 1);
        // Un repère loin de toute voie est ignoré.
        let c = ring(4.0, &[rule(30.0, 20.0, 2)]);
        assert!(c.axes.iter().any(|a| a.sens == 1));
    }

    #[test]
    fn arrow_matches_shared_symbol() {
        let s = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/symbols.json"))
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        let f = &v["marquages"][0];
        assert_eq!(f["bloc"], "SIG_FLECHE");
        assert!((f["L"].as_f64().unwrap() - ARROW_L).abs() < 1e-9);
        assert!((f["W"].as_f64().unwrap() - ARROW_W).abs() < 1e-9);
    }
}
