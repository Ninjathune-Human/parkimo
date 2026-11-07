//! Escaliers et ascenseur d'un silo, placés automatiquement.
//!
//! Règlement de sécurité ERP, arrêté du 9 mai 2006 modifié, article PS 13 § 1 : à chaque niveau,
//! un escalier (ou une sortie) à 40 m au plus quand on se trouve entre deux escaliers opposés,
//! 25 m dans les autres cas ; 50 et 30 m dans un parc largement ventilé (PS 3). Les distances se
//! mesurent dans l'axe des circulations des véhicules, depuis l'axe de la place la plus éloignée.
//!
//! Hypothèses Parkimo : une cage d'escalier (deux volées de 0,90 m au moins, PS 13 § 3) ou
//! d'ascenseur occupe l'emprise d'une place présente à tous les niveaux, porte sur l'allée, au même
//! endroit à chaque niveau, en façade de préférence ; un ascenseur, contre le premier escalier.
//! « Entre deux escaliers opposés » : un escalier dans chaque sens de départ depuis la place, deux
//! escaliers différents, chacun à la distance admise ; sinon (cul-de-sac, un seul escalier), la
//! distance des autres cas. Les escaliers sont ajoutés un à un là où ils laissent le moins de places
//! hors de portée, puis les inutiles sont retirés, deux sont remplacés par un quand c'est possible,
//! et chacun est ramené en façade et au plus près des places sans perte de desserte.

use super::normes::ps;
use super::plateau::{Forme, Gabarit, Plan, PlaceLoc};
use crate::geom::{v2, V2};
use serde::Deserialize;

/// Escalier ou ascenseur : emprise (celle de la place remplacée), porte sur l'axe de l'allée.
#[derive(Clone, Debug)]
pub struct Noyau {
    pub q: [V2; 4],
    pub porte: V2,
    pub ascenseur: bool,
    /// Escalier de liaison entre les deux demi-plateaux d'un niveau (demi-niveaux) : deux places
    /// dos à dos et le joint qui les sépare, une porte sur chaque allée.
    pub liaison: bool,
    pub cle: String,
}

/// Ascenseurs en demi-niveaux : un par demi-plateau (chacun dessert un demi-niveau sur deux), ou
/// un seul et un escalier de liaison vers l'autre demi-plateau.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AscenseursDemi {
    #[default]
    Deux,
    Liaison,
}

/// Bilan d'évacuation (PS 13).
#[derive(Clone, Debug, Default)]
pub struct Evacuation {
    /// Distances admises : entre deux escaliers opposés, autres cas.
    pub limites: (f64, f64),
    /// Plus grande distance d'une place à l'escalier le plus proche.
    pub distance_max: f64,
    /// Places au-delà de la distance admise.
    pub hors: usize,
    /// Noyaux placés à la main.
    pub manuel: bool,
    /// Noyaux dont la porte donne dans l'épure de giration d'une rampe (placés à la main).
    pub portes_genees: Vec<String>,
}

/// Nature d'un noyau saisi à la main.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Nature {
    Escalier,
    Ascenseur,
    Liaison,
}

/// Bout du réseau : position u de la liaison entre allées, allée transversale ou demi-tour
/// (coût fixe d'une allée à la suivante).
#[derive(Clone, Copy)]
struct Bout {
    u: f64,
    demi_tour: Option<f64>,
}

/// Tronçon du réseau : axe d'une allée le long des rangées (indice), ou liaison d'un bout.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Seg {
    Allee(usize),
    Bout(usize),
}

/// Point du réseau : tronçon et abscisse (u sur une allée, v sur une allée transversale).
#[derive(Clone, Copy, Debug)]
struct Pos {
    seg: Seg,
    t: f64,
}

/// Nœud voisin d'un point sur son tronçon : indice et abscisse.
type Voisin = Option<(usize, f64)>;

/// Escalier : sa porte sur le réseau et les nœuds qui l'encadrent.
struct Cible {
    pos: Pos,
    moins: Voisin,
    plus: Voisin,
}

/// Réseau des circulations : nœuds aux croisements allée × bout.
struct Reseau {
    allees: Vec<f64>,
    bouts: [Option<Bout>; 2],
    noeuds: Vec<(usize, usize)>,
}

/// Départ d'une place : sa position, les nœuds qui l'encadrent sur son tronçon et les distances
/// de chacun d'eux à tous les nœuds sans repasser par la place.
struct Depart {
    pos: Pos,
    moins: Voisin,
    plus: Voisin,
    d_moins: Vec<f64>,
    d_plus: Vec<f64>,
}

impl Reseau {
    fn t(&self, n: usize, seg: Seg) -> f64 {
        let (k, e) = self.noeuds[n];
        match seg {
            Seg::Allee(_) => self.bouts[e].map_or(0.0, |b| b.u),
            Seg::Bout(_) => self.allees[k],
        }
    }
    /// Nœuds d'un tronçon, dans l'ordre des abscisses.
    fn sur(&self, seg: Seg) -> Vec<(usize, f64)> {
        let mut v: Vec<(usize, f64)> = (0..self.noeuds.len())
            .filter(|&n| match seg {
                Seg::Allee(k) => self.noeuds[n].0 == k,
                Seg::Bout(e) => self.noeuds[n].1 == e,
            })
            .map(|n| (n, self.t(n, seg)))
            .collect();
        v.sort_by(|a, b| a.1.total_cmp(&b.1));
        v
    }
    /// Nœuds qui encadrent un point de son tronçon.
    fn encadre(&self, p: Pos) -> (Voisin, Voisin) {
        let v = self.sur(p.seg);
        (v.iter().rev().find(|n| n.1 <= p.t + 1e-9).copied(), v.iter().find(|n| n.1 > p.t + 1e-9).copied())
    }
    /// Plus courts chemins entre nœuds, sans le tronçon `coupe`.
    fn distances(&self, coupe: Option<(usize, usize)>) -> Vec<Vec<f64>> {
        let n = self.noeuds.len();
        let mut d = vec![vec![f64::INFINITY; n]; n];
        for (i, row) in d.iter_mut().enumerate() {
            row[i] = 0.0;
        }
        let segs = (0..self.allees.len()).map(Seg::Allee).chain((0..2).filter(|&e| self.bouts[e].is_some()).map(Seg::Bout));
        for seg in segs {
            for w in self.sur(seg).windows(2) {
                let (a, b) = (w[0].0, w[1].0);
                if coupe.is_some_and(|c| c == (a, b) || c == (b, a)) {
                    continue;
                }
                let l = match seg {
                    Seg::Bout(e) => self.bouts[e].and_then(|b| b.demi_tour).unwrap_or(w[1].1 - w[0].1),
                    Seg::Allee(_) => w[1].1 - w[0].1,
                };
                d[a][b] = d[a][b].min(l);
                d[b][a] = d[b][a].min(l);
            }
        }
        for k in 0..n {
            for i in 0..n {
                for j in 0..n {
                    let v = d[i][k] + d[k][j];
                    if v < d[i][j] {
                        d[i][j] = v;
                    }
                }
            }
        }
        d
    }
    fn depart(&self, pos: Pos) -> Depart {
        let (moins, plus) = self.encadre(pos);
        let coupe = moins.zip(plus).map(|(a, b)| (a.0, b.0));
        let d = self.distances(coupe);
        let ligne = |n: Voisin| n.map_or_else(Vec::new, |n| d[n.0].clone());
        Depart { pos, moins, plus, d_moins: ligne(moins), d_plus: ligne(plus) }
    }
    fn cible(&self, pos: Pos) -> Cible {
        let (moins, plus) = self.encadre(pos);
        Cible { pos, moins, plus }
    }
    /// Distance d'une place à un escalier en partant dans un sens (vers les abscisses décroissantes
    /// ou croissantes de son tronçon).
    fn vers(&self, dep: &Depart, c: &Cible, croissant: bool) -> f64 {
        let (p, s, sm, sp) = (dep.pos, c.pos, c.moins, c.plus);
        let meme = s.seg == p.seg && sm.map(|n| n.0) == dep.moins.map(|n| n.0) && sp.map(|n| n.0) == dep.plus.map(|n| n.0);
        let (part, dist, autre) = if croissant { (dep.plus, &dep.d_plus, dep.moins) } else { (dep.moins, &dep.d_moins, dep.plus) };
        if meme && (s.t >= p.t) == croissant {
            return (s.t - p.t).abs();
        }
        let Some((_, tn)) = part else { return f64::INFINITY };
        let base = (tn - p.t).abs();
        if meme {
            // Escalier de l'autre côté de la place, sur le même tronçon : par l'autre nœud.
            return autre.map_or(f64::INFINITY, |(m, tm)| base + dist[m] + (tm - s.t).abs());
        }
        [sm, sp].into_iter().flatten().map(|(m, tm)| base + dist[m] + (tm - s.t).abs()).fold(f64::INFINITY, f64::min)
    }
}

fn dans(anneau: &[V2], p: V2) -> bool {
    let n = anneau.len();
    (0..n).fold(false, |dedans, i| {
        let (a, b) = (anneau[i], anneau[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) > p.x {
            !dedans
        } else {
            dedans
        }
    })
}

/// Seuil d'une cage : porte de 0,90 m au milieu de l'avant de la place, et la bande de 1,00 m
/// devant elle dans l'allée (milieu et bouts).
fn seuil(q: &[V2; 4]) -> [V2; 3] {
    let (m, f) = ((q[2] + q[3]) * 0.5, (q[0] + q[1]) * 0.5);
    let (n, t) = ((m - f).norm(), (q[2] - q[3]).norm());
    [m + n, m + n + t * 0.45, m + n - t * 0.45]
}

/// Porte d'une cage hors des épures de giration des rampes.
fn porte_libre(plan: &Plan, q: &[V2; 4]) -> bool {
    !seuil(q).iter().any(|&p| plan.girations.iter().any(|gi| dans(&gi.contour, p)))
}

fn bornes(q: &[V2; 4]) -> (V2, V2) {
    q.iter().fold((v2(f64::MAX, f64::MAX), v2(f64::MIN, f64::MIN)), |(lo, hi), p| {
        (v2(lo.x.min(p.x), lo.y.min(p.y)), v2(hi.x.max(p.x), hi.y.max(p.y)))
    })
}

/// Réseau des circulations d'un plateau : axes des allées, allées transversales ou demi-tours.
fn reseau(plan: &Plan) -> Reseau {
    let mut allees: Vec<(f64, f64, f64)> = Vec::new();
    let mut x: [Option<f64>; 2] = [None, None];
    for (q, cle) in &plan.allees {
        let (lo, hi) = bornes(q);
        match cle.as_str() {
            "AL/X1" => x[0] = Some((lo.x + hi.x) / 2.0),
            "AL/X2" => x[1] = Some((lo.x + hi.x) / 2.0),
            _ => allees.push(((lo.y + hi.y) / 2.0, lo.x, hi.x)),
        }
    }
    allees.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (u0, u1) = allees.iter().fold((f64::MAX, f64::MIN), |(a, b), al| (a.min(al.1), b.max(al.2)));
    // Sans allée transversale (demi-niveaux, plateaux inclinés) : demi-tours aux bouts des allées.
    let demi_tour = plan
        .rampes
        .iter()
        .find(|r| matches!(r.forme, Forme::DemiU | Forme::Virage))
        .map(|r| r.longueur);
    let bout = |e: usize| match (x[e], demi_tour) {
        (Some(u), _) => Some(Bout { u, demi_tour: None }),
        (None, Some(c)) => Some(Bout { u: if e == 0 { u0 } else { u1 }, demi_tour: Some(c) }),
        _ => None,
    };
    let bouts = [bout(0), bout(1)];
    let noeuds = (0..2).filter(|&e| bouts[e].is_some()).flat_map(|e| (0..allees.len()).map(move |k| (k, e))).collect();
    Reseau { allees: allees.iter().map(|a| a.0).collect(), bouts, noeuds }
}

/// Point du réseau devant une place : sur l'axe de l'allée qu'elle borde, ou sur l'allée
/// transversale pour une place de rive.
fn devant(r: &Reseau, plan: &Plan, pl: &PlaceLoc, allee: f64) -> Option<Pos> {
    let (lo, hi) = bornes(&pl.q);
    if pl.rive {
        let e = if (lo.x + hi.x) / 2.0 < plan.longueur() / 2.0 { 0 } else { 1 };
        return r.bouts[e].filter(|b| b.demi_tour.is_none()).map(|_| Pos { seg: Seg::Bout(e), t: (lo.y + hi.y) / 2.0 });
    }
    let av = pl.q[2].y;
    let k = (0..r.allees.len()).min_by(|&a, &b| (r.allees[a] - av).abs().total_cmp(&(r.allees[b] - av).abs()))?;
    ((r.allees[k] - av).abs() <= allee / 2.0 + 1e-6).then_some(Pos { seg: Seg::Allee(k), t: (lo.x + hi.x) / 2.0 })
}

/// Coordonnées d'un point du réseau.
fn point(r: &Reseau, p: Pos) -> V2 {
    match p.seg {
        Seg::Allee(k) => v2(p.t, r.allees[k]),
        Seg::Bout(e) => v2(r.bouts[e].map_or(0.0, |b| b.u), p.t),
    }
}

/// Desserte d'une place par les escaliers déjà placés : dans chaque sens de départ, les deux
/// escaliers les plus proches (distance, identifiant).
#[derive(Clone, Copy)]
struct Etat {
    moins: [(f64, usize); 2],
    plus: [(f64, usize); 2],
}

impl Etat {
    const VIDE: Etat = Etat { moins: [(f64::INFINITY, usize::MAX); 2], plus: [(f64::INFINITY, usize::MAX); 2] };
    fn ajoute(self, id: usize, m: f64, p: f64) -> Etat {
        let ins = |t: [(f64, usize); 2], d: f64| {
            if d < t[0].0 {
                [(d, id), t[0]]
            } else if d < t[1].0 {
                [t[0], (d, id)]
            } else {
                t
            }
        };
        Etat { moins: ins(self.moins, m), plus: ins(self.plus, p) }
    }
    fn d(&self) -> f64 {
        self.moins[0].0.min(self.plus[0].0)
    }
    /// Écart à la règle (0 si respectée) : escalier à la distance des autres cas, ou place entre
    /// deux escaliers opposés, un dans chaque sens, différents, chacun à la distance admise entre
    /// deux escaliers.
    fn ecart(&self, lim: (f64, f64)) -> f64 {
        let mut e = self.d() - lim.1;
        for a in &self.moins {
            for b in &self.plus {
                if a.1 != b.1 {
                    e = e.min(a.0.max(b.0) - lim.0);
                }
            }
        }
        if e <= 1e-9 {
            0.0
        } else {
            e.min(1000.0)
        }
    }
}

/// Distances admises (PS 13 § 1).
pub fn limites(largement_ventile: bool) -> (f64, f64) {
    if largement_ventile {
        (ps::DISTANCE_DEUX_ISSUES_PSLV.v, ps::DISTANCE_AUTRES_CAS_PSLV.v)
    } else {
        (ps::DISTANCE_DEUX_ISSUES.v, ps::DISTANCE_AUTRES_CAS.v)
    }
}

/// Place les escaliers et l'ascenseur dans les rangées du plateau (de préférence en façade), retire
/// les places qu'ils occupent et rend le bilan d'évacuation.
pub fn placer(plan: &mut Plan, g: &Gabarit, largement_ventile: bool, demi: AscenseursDemi) -> Evacuation {
    let lim = limites(largement_ventile);
    let r = reseau(plan);
    let (w, l) = (plan.largeur(), plan.longueur());
    let sur_facade = |pl: &PlaceLoc| {
        let (f, bords) = if pl.rive { (pl.q[0].x, [g.facade, l - g.facade]) } else { (pl.q[0].y, [g.facade, w - g.facade]) };
        bords.iter().any(|b| (f - b).abs() < 1e-6)
    };
    // Départs de toutes les places du parc (courantes, rendues au rez ou au dernier niveau).
    let toutes: Vec<&PlaceLoc> = plan.places.iter().chain(&plan.places_rdc).chain(&plan.places_dernier).collect();
    let departs: Vec<Option<Depart>> = toutes.iter().map(|pl| devant(&r, plan, pl, g.allee).map(|p| r.depart(p))).collect();
    // Candidates : places présentes à tous les niveaux dont la porte ne débouche pas dans l'épure
    // d'une rampe, une par point du réseau (en façade de préférence, deux places dos à dos ayant
    // la même porte).
    let libre = |pl: &PlaceLoc| porte_libre(plan, &pl.q);
    let mut cands: Vec<(usize, Cible, bool)> = Vec::new();
    for (i, pl) in plan.places.iter().enumerate() {
        if !libre(pl) {
            continue;
        }
        let Some(p) = devant(&r, plan, pl, g.allee) else { continue };
        let f = sur_facade(pl);
        match cands.iter_mut().find(|c| c.1.pos.seg == p.seg && (c.1.pos.t - p.t).abs() < 1e-6) {
            Some(c) => {
                if f && !c.2 {
                    *c = (i, r.cible(p), f);
                }
            }
            None => cands.push((i, r.cible(p), f)),
        }
    }
    // Distances de chaque place à chaque escalier possible, dans les deux sens.
    let m: Vec<Vec<(f64, f64)>> = cands
        .iter()
        .map(|c| {
            departs
                .iter()
                .map(|d| d.as_ref().map_or((f64::INFINITY, f64::INFINITY), |d| (r.vers(d, &c.1, false), r.vers(d, &c.1, true))))
                .collect()
        })
        .collect();
    // Places hors de portée, somme des écarts, plus grande distance, somme des distances (places
    // devenues cages exclues).
    let bilan = |set: &[usize], sauf: &[usize]| -> (usize, f64, f64, f64) {
        let (mut hors, mut ecart, mut dmax, mut somme) = (0, 0.0, 0.0f64, 0.0);
        for (pi, dep) in departs.iter().enumerate() {
            if dep.is_none() || sauf.contains(&pi) || set.iter().any(|&c| cands[c].0 == pi) {
                continue;
            }
            let et = set.iter().fold(Etat::VIDE, |et, &c| et.ajoute(c, m[c][pi].0, m[c][pi].1));
            dmax = dmax.max(et.d());
            somme += et.d().min(1000.0);
            let e = et.ecart(lim);
            if e > 0.0 {
                hors += 1;
                ecart += e;
            }
        }
        (hors, ecart, dmax, somme)
    };
    let mieux = |a: (usize, f64, f64, f64), b: (usize, f64, f64, f64)| a.0 < b.0 || (a.0 == b.0 && a.1 < b.1 - 1e-6);
    // Ajout glouton : à chaque fois l'escalier qui laisse le moins de places hors de portée (puis
    // le plus petit écart, puis en façade).
    let mut set: Vec<usize> = Vec::new();
    let mut cur = bilan(&set, &[]);
    while set.len() < 16 && cur.0 > 0 {
        let mut best: Option<(usize, (usize, f64, f64, f64))> = None;
        for c in 0..cands.len() {
            if set.contains(&c) {
                continue;
            }
            set.push(c);
            let b = bilan(&set, &[]);
            set.pop();
            if best.is_none_or(|(bc, bb)| mieux(b, bb) || (b.0 == bb.0 && (b.1 - bb.1).abs() <= 1e-6 && cands[c].2 && !cands[bc].2)) {
                best = Some((c, b));
            }
        }
        let Some((c, b)) = best else { break };
        if !mieux(b, cur) {
            break;
        }
        set.push(c);
        cur = b;
    }
    // Élagage, puis deux escaliers remplacés par un seul tant que la desserte reste aussi bonne,
    // puis escaliers ramenés en façade quand c'est sans effet.
    let mut i = 0;
    while i < set.len() {
        let mut sans = set.clone();
        sans.remove(i);
        let b = bilan(&sans, &[]);
        if b.0 <= cur.0 {
            (set, cur) = (sans, b);
        } else {
            i += 1;
        }
    }
    'reduit: while set.len() >= 2 {
        for i in 0..set.len() {
            for j in i + 1..set.len() {
                let base: Vec<usize> = set.iter().enumerate().filter(|(k, _)| *k != i && *k != j).map(|(_, &c)| c).collect();
                for c in 0..cands.len() {
                    if set.contains(&c) {
                        continue;
                    }
                    let mut s2 = base.clone();
                    s2.push(c);
                    let b = bilan(&s2, &[]);
                    if b.0 <= cur.0 {
                        (set, cur) = (s2, b);
                        continue 'reduit;
                    }
                }
            }
        }
        break;
    }
    // Échanges un pour un sans perte de desserte : escaliers ramenés en façade, puis plus proches
    // des places en moyenne (ce qui les écarte les uns des autres).
    let hors_facade = |s: &[usize]| s.iter().filter(|&&c| !cands[c].2).count();
    for _ in 0..4 {
        let mut change = false;
        for i in 0..set.len() {
            for c in 0..cands.len() {
                if set.contains(&c) {
                    continue;
                }
                let mut s2 = set.clone();
                s2[i] = c;
                let (nf, nf2) = (hors_facade(&set), hors_facade(&s2));
                if nf2 > nf {
                    continue;
                }
                let b = bilan(&s2, &[]);
                if b.0 <= cur.0 && (nf2 < nf || b.3 < cur.3 - 1.0) {
                    (set, cur) = (s2, b);
                    change = true;
                }
            }
        }
        if !change {
            break;
        }
    }
    // Numérotation le long du plateau.
    set.sort_by(|&a, &b| {
        let (pa, pb) = (point(&r, cands[a].1.pos), point(&r, cands[b].1.pos));
        pa.x.total_cmp(&pb.x).then(pa.y.total_cmp(&pb.y))
    });
    let mut retirees: Vec<usize> = set.iter().map(|&c| cands[c].0).collect();
    let mut noyaux: Vec<Noyau> = set
        .iter()
        .enumerate()
        .map(|(i, &c)| Noyau { q: plan.places[cands[c].0].q, porte: point(&r, cands[c].1.pos), ascenseur: false, liaison: false, cle: format!("NY/E{}", i + 1) })
        .collect();
    // Ascenseur dans une place voisine du premier escalier (côté commun), présente à tous les
    // niveaux : à côté, porte sur la même allée, sinon dos à dos.
    let moitie = |i: usize| devant(&r, plan, &plan.places[i], g.allee).map(|p| p.seg);
    let centre = |q: &[V2; 4]| (q[0] + q[2]) * 0.5;
    if let Some(&c0) = set.first() {
        let q0 = plan.places[cands[c0].0].q;
        let communs = |q: &[V2; 4]| q.iter().filter(|a| q0.iter().any(|b| (**a - *b).len() < 1e-6)).count();
        let voisines: Vec<usize> = (0..plan.places.len())
            .filter(|&i| !retirees.contains(&i) && communs(&plan.places[i].q) == 2 && libre(&plan.places[i]))
            .collect();
        let a_cote = |i: &usize| moitie(*i) == Some(cands[c0].1.pos.seg);
        let voisine = voisines.iter().copied().find(a_cote).or(voisines.first().copied());
        if let Some(a) = voisine {
            let porte = devant(&r, plan, &plan.places[a], g.allee).map_or(noyaux[0].porte, |p| point(&r, p));
            noyaux.push(Noyau { q: plan.places[a].q, porte, ascenseur: true, liaison: false, cle: "NY/A1".into() });
            retirees.push(a);
            // Demi-niveaux : les deux demi-plateaux d'un niveau sont décalés d'une demi-hauteur.
            let en_demi = plan.rampes.iter().any(|rp| matches!(rp.forme, Forme::DemiU));
            let (ca, ma) = (centre(&plan.places[a].q), moitie(a));
            let autre = |i: usize| !retirees.contains(&i) && libre(&plan.places[i]) && moitie(i).is_some() && moitie(i) != ma;
            let pres = |i: usize, j: usize| (centre(&plan.places[i].q) - ca).len().total_cmp(&(centre(&plan.places[j].q) - ca).len());
            if en_demi && demi == AscenseursDemi::Deux {
                // Second ascenseur, sur l'autre demi-plateau : à côté d'un escalier de ce côté, sinon
                // au plus près du premier.
                let contre = |c: usize, i: usize| {
                    let qc = plan.places[cands[c].0].q;
                    plan.places[i].q.iter().filter(|a| qc.iter().any(|b| (**a - *b).len() < 1e-6)).count() == 2 && moitie(i) == Some(cands[c].1.pos.seg)
                };
                let pres_escalier = set
                    .iter()
                    .filter(|&&c| Some(cands[c].1.pos.seg) != ma)
                    .find_map(|&c| (0..plan.places.len()).find(|&i| autre(i) && contre(c, i)));
                if let Some(b) = pres_escalier.or_else(|| (0..plan.places.len()).filter(|&i| autre(i)).min_by(|&i, &j| pres(i, j))) {
                    let porte = devant(&r, plan, &plan.places[b], g.allee).map_or(noyaux[0].porte, |p| point(&r, p));
                    noyaux.push(Noyau { q: plan.places[b].q, porte, ascenseur: true, liaison: false, cle: "NY/A2".into() });
                    retirees.push(b);
                }
            } else if en_demi {
                // Escalier de liaison : deux places dos à dos de part et d'autre du joint (même
                // travée, fonds face à face), au plus près de l'ascenseur.
                let (lo, hi) = (|i: usize| bornes(&plan.places[i].q).0, |i: usize| bornes(&plan.places[i].q).1);
                let dos = |i: usize, j: usize| {
                    (lo(i).x - lo(j).x).abs() < 1e-6 && (hi(i).x - hi(j).x).abs() < 1e-6 && (plan.places[i].q[0].y - plan.places[j].q[0].y).abs() < 2.0
                };
                let paires = (0..plan.places.len())
                    .filter(|&i| !retirees.contains(&i) && libre(&plan.places[i]) && moitie(i) == ma)
                    .flat_map(|i| (0..plan.places.len()).filter(move |&j| autre(j) && dos(i, j)).map(move |j| (i, j)));
                if let Some((i, j)) = paires.min_by(|a, b| pres(a.0, b.0)) {
                    let (yi, yj) = (plan.places[i].q[3].y, plan.places[j].q[3].y);
                    let (x0, x1) = (lo(i).x, hi(i).x);
                    let porte = devant(&r, plan, &plan.places[i], g.allee).map_or(noyaux[0].porte, |p| point(&r, p));
                    let q = [v2(x0, yi), v2(x1, yi), v2(x1, yj), v2(x0, yj)];
                    noyaux.push(Noyau { q, porte, ascenseur: false, liaison: true, cle: "NY/L1".into() });
                    retirees.extend([i, j]);
                }
            }
        }
    }
    let (hors, _, dmax, _) = bilan(&set, &retirees);
    let mut i = 0;
    plan.places.retain(|_| {
        i += 1;
        !retirees.contains(&(i - 1))
    });
    plan.noyaux = noyaux;
    Evacuation { limites: lim, distance_max: dmax, hors, manuel: false, portes_genees: Vec::new() }
}

/// Noyaux placés à la main : chaque point (repère du plateau) est recalé sur la place présente à
/// tous les niveaux la plus proche (deux places dos à dos de part et d'autre du joint pour un
/// escalier de liaison, en demi-niveaux seulement) ; les places prises sont retirées et le bilan
/// d'évacuation est refait avec ces escaliers.
pub fn fixer(plan: &mut Plan, g: &Gabarit, largement_ventile: bool, saisis: &[(Nature, V2)]) -> Evacuation {
    let lim = limites(largement_ventile);
    let r = reseau(plan);
    let centre = |q: &[V2; 4]| (q[0] + q[2]) * 0.5;
    let moitie = |i: usize| devant(&r, plan, &plan.places[i], g.allee).map(|p| p.seg);
    let en_demi = plan.rampes.iter().any(|rp| matches!(rp.forme, Forme::DemiU));
    let mut retirees: Vec<usize> = Vec::new();
    // (nature, cadre, porte sur le réseau, place)
    let mut pris: Vec<(Nature, [V2; 4], Option<Pos>)> = Vec::new();
    for &(nature, p) in saisis {
        let loin = |i: usize| (centre(&plan.places[i].q) - p).len();
        if nature == Nature::Liaison {
            if !en_demi {
                continue;
            }
            let (lo, hi) = (|i: usize| bornes(&plan.places[i].q).0, |i: usize| bornes(&plan.places[i].q).1);
            let paire = (0..plan.places.len())
                .filter(|i| !retirees.contains(i))
                .flat_map(|i| (0..plan.places.len()).map(move |j| (i, j)))
                .filter(|&(i, j)| {
                    i < j
                        && !retirees.contains(&j)
                        && moitie(i).is_some()
                        && moitie(j).is_some()
                        && moitie(i) != moitie(j)
                        && (lo(i).x - lo(j).x).abs() < 1e-6
                        && (hi(i).x - hi(j).x).abs() < 1e-6
                        && (plan.places[i].q[0].y - plan.places[j].q[0].y).abs() < 2.0
                })
                .min_by(|a, b| (loin(a.0) + loin(a.1)).total_cmp(&(loin(b.0) + loin(b.1))));
            if let Some((i, j)) = paire {
                let (yi, yj, x0, x1) = (plan.places[i].q[3].y, plan.places[j].q[3].y, lo(i).x, hi(i).x);
                pris.push((nature, [v2(x0, yi), v2(x1, yi), v2(x1, yj), v2(x0, yj)], devant(&r, plan, &plan.places[i], g.allee)));
                retirees.extend([i, j]);
            }
            continue;
        }
        if let Some(i) = (0..plan.places.len()).filter(|i| !retirees.contains(i)).min_by(|&a, &b| loin(a).total_cmp(&loin(b))) {
            pris.push((nature, plan.places[i].q, devant(&r, plan, &plan.places[i], g.allee)));
            retirees.push(i);
        }
    }
    // Numérotation le long du plateau, par nature.
    let x = |e: &(Nature, [V2; 4], Option<Pos>)| e.2.map_or(centre(&e.1), |p| point(&r, p));
    pris.sort_by(|a, b| (x(a).x, x(a).y).partial_cmp(&(x(b).x, x(b).y)).unwrap_or(std::cmp::Ordering::Equal));
    let mut n = [0usize; 3];
    let mut noyaux = Vec::new();
    let mut genees = Vec::new();
    for e in &pris {
        let k = e.0 as usize;
        n[k] += 1;
        let cle = format!("NY/{}{}", ["E", "A", "L"][k], n[k]);
        let porte_ok = porte_libre(plan, &e.1) && (e.0 != Nature::Liaison || porte_libre(plan, &[e.1[2], e.1[3], e.1[0], e.1[1]]));
        if !porte_ok {
            genees.push(cle.clone());
        }
        noyaux.push(Noyau { q: e.1, porte: x(e), ascenseur: e.0 == Nature::Ascenseur, liaison: e.0 == Nature::Liaison, cle });
    }
    // Bilan avec les escaliers retenus.
    let cibles: Vec<Cible> = pris.iter().filter(|e| e.0 == Nature::Escalier).filter_map(|e| e.2).map(|p| r.cible(p)).collect();
    let toutes: Vec<&PlaceLoc> = plan.places.iter().chain(&plan.places_rdc).chain(&plan.places_dernier).collect();
    let (mut hors, mut dmax) = (0, 0.0f64);
    for (i, pl) in toutes.iter().enumerate() {
        if retirees.contains(&i) {
            continue;
        }
        let Some(dep) = devant(&r, plan, pl, g.allee).map(|p| r.depart(p)) else { continue };
        let et = cibles.iter().enumerate().fold(Etat::VIDE, |et, (k, c)| et.ajoute(k, r.vers(&dep, c, false), r.vers(&dep, c, true)));
        dmax = dmax.max(et.d().min(1000.0));
        if et.ecart(lim) > 0.0 {
            hors += 1;
        }
    }
    let mut i = 0;
    plan.places.retain(|_| {
        i += 1;
        !retirees.contains(&(i - 1))
    });
    plan.noyaux = noyaux;
    Evacuation { limites: lim, distance_max: dmax, hors, manuel: true, portes_genees: genees }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ouvrage::plateau::{construire, Densite, Implantation};

    fn gab() -> Gabarit {
        Gabarit {
            w: 2.5,
            d: 5.0,
            allee: 6.0,
            facade: 0.2,
            pu: 0.3,
            pv: 0.6,
            travee: 3,
            rives: true,
            densite: Densite::Longue,
            retrait: 0.65,
            rampe: Some(18.36),
            rampe_largeur: 3.25,
            implantation: Implantation::Rangee,
            hauteur: 2.9,
        }
    }

    fn pos(seg: Seg, t: f64) -> Pos {
        Pos { seg, t }
    }

    /// 32,40 × 60 (59,90 après calage), rampes en rangée : places de rive de 0,20 à 5,20, allée
    /// transversale de 5,20 à 11,20 (axe 8,20), 15 places de 2,50 jusqu'à 48,70, allée de 48,70 à
    /// 54,70 (axe 51,70) ; deux modules de 16 m, axes des allées à 0,20 + 5 + 3 = 8,20 et 24,20.
    #[test]
    fn distances_a_la_main() {
        let p = construire(&gab(), 32.4, 60.0).unwrap();
        let r = reseau(&p);
        let pr = |a: f64, b: f64| assert!((a - b).abs() < 1e-9, "{a} ≠ {b}");
        pr(r.allees[0], 8.2);
        pr(r.allees[1], 24.2);
        pr(r.bouts[0].unwrap().u, 8.2);
        pr(r.bouts[1].unwrap().u, 51.7);
        let place = r.depart(pos(Seg::Allee(0), 30.0));
        let vers = |dep: &Depart, s: Pos| (r.vers(dep, &r.cible(s), false), r.vers(dep, &r.cible(s), true));
        // Escalier de pignon (allée transversale du bout 1, v = 17,45) : à gauche 21,80 + 9,25 ;
        // à droite le tour par l'autre allée, 21,70 + 16 + 43,50 + 6,75.
        let (m, pl) = vers(&place, pos(Seg::Bout(0), 17.45));
        pr(m, 31.05);
        pr(pl, 87.95);
        // Escalier sur la même allée : direct d'un côté ; de l'autre, le tour par les deux allées
        // transversales et l'autre allée, 21,80 + 16 + 43,50 + 16 + 11,70.
        let (m, pl) = vers(&place, pos(Seg::Allee(0), 40.0));
        pr(pl, 10.0);
        pr(m, 109.0);
        let (m, pl) = vers(&place, pos(Seg::Allee(0), 20.0));
        pr(m, 10.0);
        pr(pl, 21.7 + 16.0 + 43.5 + 16.0 + 11.8);
        // Place de rive d'angle (v = 2,45, sous l'allée) : cul-de-sac vers le bas.
        let angle = r.depart(pos(Seg::Bout(0), 2.45));
        let (m, pl) = vers(&angle, pos(Seg::Allee(0), 40.0));
        assert!(m.is_infinite());
        pr(pl, 5.75 + 31.8);
    }

    #[test]
    fn regle_ps13() {
        let lim = limites(false);
        assert_eq!(lim, (40.0, 25.0));
        assert_eq!(limites(true), (50.0, 30.0));
        let un = Etat::VIDE.ajoute(0, 30.0, f64::INFINITY);
        assert!((un.ecart(lim) - 5.0).abs() < 1e-9, "un seul escalier à 30 m : règle des 25 m");
        assert_eq!(un.ecart(limites(true)), 0.0);
        let deux = un.ajoute(1, f64::INFINITY, 35.0);
        assert_eq!(deux.ecart(lim), 0.0, "deux escaliers opposés à 30 et 35 m");
        let loin = un.ajoute(1, f64::INFINITY, 45.0);
        assert!((loin.ecart(lim) - 5.0).abs() < 1e-9, "le second escalier au-delà de 40 m ne compte pas");
        let meme = Etat::VIDE.ajoute(0, 30.0, 35.0);
        assert!(meme.ecart(lim) > 0.0, "le même escalier dans les deux sens n'en fait pas deux");
    }

    #[test]
    fn noyaux_sur_les_plateaux() {
        for (larg, l, imp) in [
            (32.4, 60.0, Implantation::Rangee),
            (32.4, 100.0, Implantation::Rangee),
            (32.4, 100.0, Implantation::Pignon),
            (33.0, 100.0, Implantation::DemiNiveaux),
            (33.0, 100.0, Implantation::Inclines),
            (48.4, 80.0, Implantation::Helice),
        ] {
            let mut nb = [0; 2];
            for (k, pslv) in [false, true].into_iter().enumerate() {
                let g = Gabarit { implantation: imp, ..gab() };
                let mut p = construire(&g, larg, l).unwrap();
                let avant: Vec<[V2; 4]> = p.places.iter().map(|pl| pl.q).collect();
                let ev = placer(&mut p, &g, pslv, AscenseursDemi::Deux);
                let cas = format!("{larg}×{l} {imp:?} PSLV {pslv}");
                assert_eq!(ev.hors, 0, "{cas}");
                assert!(ev.distance_max <= ev.limites.0 + 1e-9, "{cas}");
                let esc: Vec<&Noyau> = p.noyaux.iter().filter(|n| !n.ascenseur && !n.liaison).collect();
                let asc: Vec<&Noyau> = p.noyaux.iter().filter(|n| n.ascenseur).collect();
                let demi = imp == Implantation::DemiNiveaux;
                assert!(esc.len() >= 2 && asc.len() == if demi { 2 } else { 1 }, "{cas} : {} escaliers, {} ascenseurs", esc.len(), asc.len());
                assert_eq!(p.places.len() + p.noyaux.len(), avant.len(), "{cas}");
                for n in &p.noyaux {
                    assert!(avant.contains(&n.q), "{cas} : {} hors d'une place", n.cle);
                }
                let communs = asc[0].q.iter().filter(|a| esc[0].q.iter().any(|b| (**a - *b).len() < 1e-6)).count();
                assert_eq!(communs, 2, "{cas} : ascenseur contre NY/E1");
                // À côté de l'escalier : même fond, même avant.
                let cote = |q: &[V2; 4]| (q[3] - q[0]).norm();
                assert!((cote(&asc[0].q) - cote(&esc[0].q)).len() < 1e-6, "{cas} : ascenseur dos à dos");
                nb[k] = esc.len();
            }
            assert!(nb[1] <= nb[0], "{larg}×{l} {imp:?} : {nb:?}");
        }
    }

    /// Demi-niveaux sur 33 × 100 : façade 0,20, rangée de 5, allée de 6, rangée de 5 (fond à 16,20),
    /// joint de 13,50 - 3 - 10 = 0,50, rangée de 5 (fond à 16,70), allée, rangée.
    #[test]
    fn demi_niveaux_ascenseurs() {
        let g = Gabarit { implantation: Implantation::DemiNiveaux, ..gab() };
        let moitie = |p: &Plan, n: &Noyau| reseau(p).allees.iter().position(|&v| (v - n.porte.y).abs() < 1e-6);
        // Deux ascenseurs, un par demi-plateau.
        let mut p = construire(&g, 33.0, 100.0).unwrap();
        let avant = p.places.len();
        assert_eq!(placer(&mut p, &g, false, AscenseursDemi::Deux).hors, 0);
        let asc: Vec<&Noyau> = p.noyaux.iter().filter(|n| n.ascenseur).collect();
        assert_eq!(asc.len(), 2);
        assert!(moitie(&p, asc[0]).is_some() && moitie(&p, asc[0]) != moitie(&p, asc[1]));
        // Chaque ascenseur contre un escalier de son demi-plateau.
        for a in &asc {
            let contre = p.noyaux.iter().filter(|n| !n.ascenseur).any(|e| e.q.iter().filter(|x| a.q.iter().any(|y| (**x - *y).len() < 1e-6)).count() == 2);
            assert!(contre, "{} seul", a.cle);
        }
        assert_eq!(p.places.len() + p.noyaux.len(), avant);
        // Un ascenseur et un escalier de liaison : deux places dos à dos et le joint, 5 + 0,50 + 5
        // = 10,50 de l'allée basse (bord à 0,20 + 5 + 6 = 11,20) à l'allée haute (16,70 + 5 = 21,70).
        let mut p = construire(&g, 33.0, 100.0).unwrap();
        assert_eq!(placer(&mut p, &g, false, AscenseursDemi::Liaison).hors, 0);
        assert_eq!(p.noyaux.iter().filter(|n| n.ascenseur).count(), 1);
        let l: Vec<&Noyau> = p.noyaux.iter().filter(|n| n.liaison).collect();
        assert_eq!(l.len(), 1);
        let (mut a, mut b) = (l[0].q[0].y, l[0].q[3].y);
        if a > b {
            (a, b) = (b, a);
        }
        assert!((a - 11.2).abs() < 1e-9 && (b - 21.7).abs() < 1e-9, "{a} {b}");
        assert!(((l[0].q[1] - l[0].q[0]).len() - 2.5).abs() < 1e-9);
        assert_eq!(p.places.len() + p.noyaux.len() + 1, avant);
        // Hors demi-niveaux, le choix est sans effet.
        let g = gab();
        let mut p = construire(&g, 32.4, 60.0).unwrap();
        placer(&mut p, &g, false, AscenseursDemi::Liaison);
        assert!(p.noyaux.iter().filter(|n| n.ascenseur).count() == 1 && !p.noyaux.iter().any(|n| n.liaison));
    }

    /// À la main sur 32,40 × 60 (rampes en rangée, places de rive de 1,20 à 31,20 le long des
    /// pignons). Un escalier recalé dans la place de rive d'angle (0,20 à 5,20 ; 1,20 à 3,70), porte
    /// sur l'allée transversale (8,20 ; 2,45) : la place la plus loin est l'angle opposé, à
    /// 5,75 + 16 + 43,50 + 5,75 = 71,00 m. Un second escalier à l'angle opposé : places de l'allée
    /// basse centrées à 12,45 + 2,50 k ; celle de 37,45 est à 5,75 + 29,25 = 35,00 m d'un côté et
    /// 21,75 + 14,25 = 36,00 m de l'autre, le plus loin. Hors de portée : les deux autres angles,
    /// en cul-de-sac à 21,75 + 5,75 = 27,50 m ; et sur l'allée basse, à u - 2,45 m de l'un et
    /// 73,45 - u m de l'autre, les places à plus de 25 m du plus proche sans l'autre à 40 m, soit
    /// u dans ]27,45 ; 33,45[ ou ]42,45 ; 48,45[ : 29,95 et 32,45 (rangée haute, la basse étant
    /// prise par la rampe jusqu'à 32,06), 44,95 et 47,45 (deux rangées), 6 places ; autant sur
    /// l'allée haute, symétrique ; 14 en tout.
    #[test]
    fn noyaux_a_la_main() {
        let g = gab();
        let mut p = construire(&g, 32.4, 60.0).unwrap();
        let n0 = p.places.len();
        let ev = fixer(&mut p, &g, false, &[(Nature::Ascenseur, v2(15.0, 3.0)), (Nature::Escalier, v2(12.0, 3.0))]);
        assert!(ev.manuel && ev.portes_genees.is_empty());
        assert_eq!(p.places.len(), n0 - 2);
        let e1 = p.noyaux.iter().find(|n| n.cle == "NY/E1").unwrap();
        let (lo, hi) = bornes(&e1.q);
        assert!((lo - v2(0.2, 1.2)).len() < 1e-9 && (hi - v2(5.2, 3.7)).len() < 1e-9, "{lo:?} {hi:?}");
        assert!((e1.porte - v2(8.2, 2.45)).len() < 1e-9);
        assert!(p.noyaux.iter().any(|n| n.cle == "NY/A1" && n.ascenseur));
        assert!((ev.distance_max - 71.0).abs() < 1e-9, "{}", ev.distance_max);
        let mut p = construire(&g, 32.4, 60.0).unwrap();
        let ev = fixer(&mut p, &g, false, &[(Nature::Escalier, v2(2.7, 2.45)), (Nature::Escalier, v2(57.2, 29.95))]);
        assert!((ev.distance_max - 35.0).abs() < 1e-9, "{}", ev.distance_max);
        assert_eq!(ev.hors, 14);
        assert_eq!(p.noyaux.iter().map(|n| n.cle.as_str()).collect::<Vec<_>>(), ["NY/E1", "NY/E2"]);
        // Escalier de liaison hors demi-niveaux : ignoré.
        let mut p = construire(&g, 32.4, 60.0).unwrap();
        fixer(&mut p, &g, false, &[(Nature::Liaison, v2(30.0, 16.0))]);
        assert!(p.noyaux.is_empty() && p.places.len() == n0);
    }
}
