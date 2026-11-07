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

/// Escalier ou ascenseur : emprise (celle de la place remplacée), porte sur l'axe de l'allée.
#[derive(Clone, Debug)]
pub struct Noyau {
    pub q: [V2; 4],
    pub porte: V2,
    pub ascenseur: bool,
    pub cle: String,
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
pub fn placer(plan: &mut Plan, g: &Gabarit, largement_ventile: bool) -> Evacuation {
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
    let libre = |pl: &PlaceLoc| !seuil(&pl.q).iter().any(|&p| plan.girations.iter().any(|gi| dans(&gi.contour, p)));
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
    let bilan = |set: &[usize], sauf: Option<usize>| -> (usize, f64, f64, f64) {
        let (mut hors, mut ecart, mut dmax, mut somme) = (0, 0.0, 0.0f64, 0.0);
        for (pi, dep) in departs.iter().enumerate() {
            if dep.is_none() || sauf == Some(pi) || set.iter().any(|&c| cands[c].0 == pi) {
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
    let mut cur = bilan(&set, None);
    while set.len() < 16 && cur.0 > 0 {
        let mut best: Option<(usize, (usize, f64, f64, f64))> = None;
        for c in 0..cands.len() {
            if set.contains(&c) {
                continue;
            }
            set.push(c);
            let b = bilan(&set, None);
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
        let b = bilan(&sans, None);
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
                    let b = bilan(&s2, None);
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
                let b = bilan(&s2, None);
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
        .map(|(i, &c)| Noyau { q: plan.places[cands[c].0].q, porte: point(&r, cands[c].1.pos), ascenseur: false, cle: format!("NY/E{}", i + 1) })
        .collect();
    // Ascenseur dans une place voisine du premier escalier (côté commun), présente à tous les niveaux.
    let mut lift = None;
    if let Some(&c0) = set.first() {
        let q0 = plan.places[cands[c0].0].q;
        let communs = |q: &[V2; 4]| q.iter().filter(|a| q0.iter().any(|b| (**a - *b).len() < 1e-6)).count();
        let voisine = (0..plan.places.len()).find(|&i| !retirees.contains(&i) && communs(&plan.places[i].q) == 2 && libre(&plan.places[i]));
        if let Some(a) = voisine {
            let porte = devant(&r, plan, &plan.places[a], g.allee).map_or(noyaux[0].porte, |p| point(&r, p));
            noyaux.push(Noyau { q: plan.places[a].q, porte, ascenseur: true, cle: "NY/A1".into() });
            retirees.push(a);
            lift = Some(a);
        }
    }
    let (hors, _, dmax, _) = bilan(&set, lift);
    let mut i = 0;
    plan.places.retain(|_| {
        i += 1;
        !retirees.contains(&(i - 1))
    });
    plan.noyaux = noyaux;
    Evacuation { limites: lim, distance_max: dmax, hors }
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
                let ev = placer(&mut p, &g, pslv);
                let cas = format!("{larg}×{l} {imp:?} PSLV {pslv}");
                assert_eq!(ev.hors, 0, "{cas}");
                assert!(ev.distance_max <= ev.limites.0 + 1e-9, "{cas}");
                let esc: Vec<&Noyau> = p.noyaux.iter().filter(|n| !n.ascenseur).collect();
                let asc: Vec<&Noyau> = p.noyaux.iter().filter(|n| n.ascenseur).collect();
                assert!(esc.len() >= 2 && asc.len() == 1, "{cas} : {} escaliers", esc.len());
                assert_eq!(p.places.len() + p.noyaux.len(), avant.len(), "{cas}");
                for n in &p.noyaux {
                    assert!(avant.contains(&n.q), "{cas} : {} hors d'une place", n.cle);
                }
                let communs = asc[0].q.iter().filter(|a| esc[0].q.iter().any(|b| (**a - *b).len() < 1e-6)).count();
                assert_eq!(communs, 2, "{cas} : ascenseur contre NY/E1");
                nb[k] = esc.len();
            }
            assert!(nb[1] <= nb[0], "{larg}×{l} {imp:?} : {nb:?}");
        }
    }
}
