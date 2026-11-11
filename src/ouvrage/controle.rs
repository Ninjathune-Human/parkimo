//! Contrôle d'accès du rez d'un parc payant : voies à barrière séparées par des îlots, bornes,
//! local d'accueil. Le dispositif traverse une rangée de façade, sur la profondeur d'une place :
//! il ne prend des places qu'au rez.
//!
//! Neufert (p. 274-276, contrôle d'entrée) : voie de 2,50 m, 0,50 m de garde de chaque côté de
//! l'accès, barrière à 0,50 m du bout de l'îlot, borne 3,00 m avant la barrière. Hypothèses propres :
//! îlot de 0,75 m au moins, élargi pour contenir un poteau ; local de deux places à murs de 0,20 m ;
//! place voisine d'un mur portée à 2,60 m.

use super::plateau::Plan;
use crate::geom::{v2, V2};

pub const VOIE: f64 = 2.50;
pub const GARDE: f64 = 0.50;
pub const ILOT_MIN: f64 = 0.75;
/// Jeu entre une voie et le nu d'un poteau.
const JEU: f64 = 0.20;
const BARRIERE: f64 = 0.50;
const BORNE: f64 = 3.00;
pub const MUR: f64 = 0.20;
/// Largeur d'une place bordée par un mur.
pub const PLACE_AU_MUR: f64 = 2.60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sens {
    Entree,
    Sortie,
    Reversible,
}

#[derive(Clone, Debug)]
pub struct Local {
    pub q: [V2; 4],
    pub interieur: [V2; 4],
    pub surface: f64,
}

/// Dispositif en repère local du plateau.
#[derive(Clone, Debug, Default)]
pub struct Controle {
    pub voies: Vec<([V2; 4], Sens)>,
    pub ilots: Vec<[V2; 4]>,
    pub barrieres: Vec<[V2; 2]>,
    pub bornes: Vec<V2>,
    pub local: Option<Local>,
    /// Places de l'étage courant (indices dans `Plan::places`) absentes du rez.
    pub retirees: Vec<usize>,
    /// Places propres au rez : la voisine du local, élargie.
    pub elargies: Vec<[V2; 4]>,
}

/// Place d'une rangée de façade : indice, étendue le long de la façade, profondeur.
struct Rang {
    i: usize,
    x0: f64,
    x1: f64,
    d: f64,
}

/// Pose le dispositif dans la rangée de façade (long pan ou pignon) la plus proche de `vers`
/// (accès du site, en repère local), au plus près de lui. `None` si aucune rangée ne peut le recevoir.
pub fn placer(plan: &Plan, poteau: [f64; 2], vers: V2, voies: u32) -> Option<Controle> {
    let n = voies.clamp(1, 4) as usize;
    let mut mieux: Option<(f64, Controle)> = None;
    // Quatre côtés : axe de la façade (0 : le long de x, 1 : le long de y) et côté haut ou bas.
    for (axe, haut) in [(0, false), (0, true), (1, false), (1, true)] {
        let facade = if !haut { 0.0 } else if axe == 0 { plan.largeur() } else { plan.longueur() };
        // Coordonnées d'un point : le long de la façade, puis en travers.
        let lt = |p: V2| if axe == 0 { (p.x, p.y) } else { (p.y, p.x) };
        // Places adossées à cette façade (leur fond est derrière le voile de façade).
        let mut rang: Vec<Rang> = plan
            .places
            .iter()
            .enumerate()
            .filter(|(_, p)| (lt(p.q[0]).1 - facade).abs() < 0.6 && (lt(p.q[1]).1 - facade).abs() < 0.6)
            .map(|(i, p)| Rang {
                i,
                x0: p.q.iter().map(|&q| lt(q).0).fold(f64::INFINITY, f64::min),
                x1: p.q.iter().map(|&q| lt(q).0).fold(f64::NEG_INFINITY, f64::max),
                d: (lt(p.q[2]).1 - lt(p.q[0]).1).abs(),
            })
            .collect();
        rang.sort_by(|a, b| a.x0.total_cmp(&b.x0));
        if rang.len() < 4 {
            continue;
        }
        let d = rang[0].d;
        let fond = lt(plan.places[rang[0].i].q[0]).1;
        // Poteaux dans la bande de la rangée, en étendue le long de la façade.
        let poteaux: Vec<(f64, f64)> = plan
            .poteaux
            .iter()
            .filter(|p| (lt(p.c).1 - fond).abs() < d + poteau[1 - axe])
            .map(|p| (lt(p.c).0 - poteau[axe] / 2.0, lt(p.c).0 + poteau[axe] / 2.0))
            .collect();
        // Point à la distance `t` du fond des places, vers l'intérieur.
        let pt = |x: f64, t: f64| {
            let y = if haut { fond - t } else { fond + t };
            if axe == 0 { v2(x, y) } else { v2(y, x) }
        };
        let rect = |x0: f64, x1: f64, t0: f64, t1: f64| [pt(x0, t0), pt(x1, t0), pt(x1, t1), pt(x0, t1)];
        for a in 0..rang.len() {
            // Voies de gauche à droite depuis le bord de la place `a`, décalées hors des poteaux.
            let debut = rang[a].x0;
            let mut curseur = debut + GARDE;
            let mut bandes: Vec<(f64, f64)> = Vec::new();
            for _ in 0..n {
                while let Some(&(_, hi)) = poteaux.iter().find(|&&(lo, hi)| lo - JEU < curseur + VOIE && hi + JEU > curseur) {
                    curseur = hi + JEU;
                }
                bandes.push((curseur, curseur + VOIE));
                curseur += VOIE + ILOT_MIN;
            }
            let fin = bandes[n - 1].1 + GARDE;
            // Places sous le dispositif : d'un seul tenant, et qui le portent en entier.
            let Some(b) = (a..rang.len()).take_while(|&j| rang[j].x0 < fin - 0.01).last() else { continue };
            if rang[b].x1 < fin - 0.01 || (a..b).any(|j| rang[j + 1].x0 - rang[j].x1 > 0.9) {
                continue;
            }
            // Local : deux places d'un côté du dispositif, à gauche de préférence.
            // La voisine est la place accolée au mur du local, sans poteau entre eux.
            let contigu = |i: usize, j: usize| rang[j].x0 - rang[i].x1 < 0.9;
            let accole = |i: usize, j: usize| rang[j].x0 - rang[i].x1 < 0.02;
            let local = if a >= 2 && contigu(a - 2, a - 1) && contigu(a - 1, a) {
                Some((a - 2, a - 1, a.checked_sub(3).filter(|&k| accole(k, a - 2)), true))
            } else if b + 2 < rang.len() && contigu(b, b + 1) && contigu(b + 1, b + 2) {
                Some((b + 1, b + 2, Some(b + 3).filter(|&k| k < rang.len() && accole(b + 2, k)), false))
            } else {
                None
            };
            let Some((l0, l1, voisine, a_gauche)) = local else { continue };
            let ecart = (pt((debut + fin) / 2.0, 0.0) - vers).len();
            if mieux.as_ref().is_some_and(|(e, _)| *e <= ecart) {
                continue;
            }
            let mut c = Controle::default();
            // En France on roule à droite : en regardant vers l'intérieur, l'entrée est à droite.
            let entree_en_fin = (axe == 0) != haut;
            for (k, &(x0, x1)) in bandes.iter().enumerate() {
                let sens = match (n, k) {
                    (1, _) => Sens::Reversible,
                    (_, 0) => if entree_en_fin { Sens::Sortie } else { Sens::Entree },
                    (_, k) if k == n - 1 => if entree_en_fin { Sens::Entree } else { Sens::Sortie },
                    _ => Sens::Reversible,
                };
                c.voies.push((rect(x0, x1, 0.0, d), sens));
                let t = match sens { Sens::Entree => d - BARRIERE, Sens::Sortie => BARRIERE, Sens::Reversible => d / 2.0 };
                c.barrieres.push([pt(x0, t), pt(x1, t)]);
            }
            for w in bandes.windows(2) {
                c.ilots.push(rect(w[0].1, w[1].0, 0.25, d - 0.25));
                // Une borne par sens, à la main gauche du conducteur, 3,00 m avant sa barrière.
                let x = (w[0].1 + w[1].0) / 2.0;
                c.bornes.push(pt(x, (d - BARRIERE - BORNE).max(0.6)));
                c.bornes.push(pt(x, (BARRIERE + BORNE).min(d - 0.6)));
            }
            c.retirees = (a..=b).chain([l0, l1]).map(|j| rang[j].i).collect();
            // Emprise du local : ses deux places, moins la surlargeur rendue à la place voisine.
            let (mut x0, mut x1) = (rang[l0].x0, rang[l1].x1);
            if let Some(k) = voisine {
                let plus = (PLACE_AU_MUR - (rang[k].x1 - rang[k].x0)).max(0.0);
                if plus > 1e-9 {
                    let q = if a_gauche {
                        x0 = rang[k].x0 + PLACE_AU_MUR;
                        rect(rang[k].x0, x0, 0.0, d)
                    } else {
                        x1 = rang[k].x1 - PLACE_AU_MUR;
                        rect(x1, rang[k].x1, 0.0, d)
                    };
                    // Même ordre de sommets qu'une place : fond puis côté allée.
                    c.elargies.push(q);
                    c.retirees.push(rang[k].i);
                }
            }
            let interieur = rect(x0 + MUR, x1 - MUR, MUR, d - MUR);
            c.local = Some(Local { q: rect(x0, x1, 0.0, d), interieur, surface: (x1 - x0 - 2.0 * MUR) * (d - 2.0 * MUR) });
            mieux = Some((ecart, c));
        }
    }
    mieux.map(|(_, c)| c)
}
