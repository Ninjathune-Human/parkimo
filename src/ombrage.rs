//! Ombrage du parc extérieur : superficie du parc, ombre des arbres et des ombrières.
//!
//! Code de l'urbanisme, art. L. 111-19-1 et R. 111-25-7 à R. 111-25-8 ; loi du 10 mars 2023 (APER),
//! art. 40. La superficie du parc réunit les emplacements, les voies et les cheminements ; les
//! espaces verts n'en font pas partie (R. 111-25-7). Les textes ne fixent pas de méthode de mesure
//! de l'ombre : elle est prise ici comme la projection verticale des houppiers à maturité (diamètre
//! réglable) et l'emprise des ombrières, recoupées par la superficie du parc. Hypothèse à justifier.

use crate::geom::{signed_area, v2};
use crate::model::{Layout, Site, P2};
use crate::region;
use geo::algorithm::bool_ops::unary_union;
use geo::{BooleanOps, MultiPolygon, Polygon};
use serde::Serialize;

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
pub struct Ombrage {
    /// Superficie du parc (m²) : emplacements, voies et cheminements.
    pub surface: f64,
    /// Ombre des houppiers à maturité sur cette superficie (m²).
    pub arbres: f64,
    /// Emprise des ombrières sur cette superficie (m²).
    pub ombrieres: f64,
    /// Ombre totale (m²), sans compter deux fois ce qui est sous un arbre et sous une ombrière.
    pub total: f64,
    pub nb_arbres: usize,
    /// Diamètre de houppier retenu (m).
    pub houppier: f64,
}

pub(crate) fn poly(ring: &[P2]) -> Polygon<f64> {
    region::to_poly(&ring.iter().map(|p| v2(p[0], p[1])).collect::<Vec<_>>())
}

pub(crate) fn reunion(rings: impl IntoIterator<Item = Vec<P2>>) -> MultiPolygon<f64> {
    let polys: Vec<Polygon<f64>> = rings.into_iter().filter(|r| r.len() >= 3).map(|r| poly(&r)).collect();
    unary_union(polys.iter())
}

/// Polygones donnés chacun par son contour puis ses trous.
pub(crate) fn a_trous(polys: impl IntoIterator<Item = Vec<Vec<P2>>>) -> MultiPolygon<f64> {
    let mut mp = MultiPolygon::new(Vec::new());
    for p in polys {
        if p.is_empty() || p[0].len() < 3 {
            continue;
        }
        let trous = reunion(p[1..].iter().cloned());
        mp = mp.union(&MultiPolygon::new(vec![poly(&p[0])]).difference(&trous));
    }
    mp
}

/// Polygones d'une liste d'anneaux orientés (extérieur CCW, suivi de ses trous CW).
pub(crate) fn polygones(rings: &[Vec<P2>]) -> MultiPolygon<f64> {
    let mut out: Vec<Vec<Vec<P2>>> = Vec::new();
    for r in rings {
        let aire = signed_area(&r.iter().map(|p| v2(p[0], p[1])).collect::<Vec<_>>());
        if aire >= 0.0 || out.is_empty() {
            out.push(vec![r.clone()]);
        } else {
            out.last_mut().unwrap().push(r.clone());
        }
    }
    a_trous(out)
}

/// Superficie du parc : voirie, fonds de rangées, allées transversales, accès et places, d'un seul tenant.
pub fn parc(lay: &Layout) -> MultiPolygon<f64> {
    let quads = lay.pads.iter().chain(&lay.cross).chain(&lay.entries).chain(lay.stalls.iter().map(|st| &st.q)).map(|q| q.to_vec());
    polygones(&lay.paved).union(&reunion(quads))
}

fn dans(q: &[P2; 4], p: P2) -> bool {
    let cote = |a: P2, b: P2| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
    let s: Vec<f64> = (0..4).map(|i| cote(q[i], q[(i + 1) % 4])).collect();
    s.iter().all(|&x| x >= -1e-9) || s.iter().all(|&x| x <= 1e-9)
}

/// Ombrières : une par rangée (fond de rangée). Les arbres des rangées couvertes disparaissent ;
/// leurs îlots restent.
pub fn couvrir(site: &Site, lay: &mut Layout) {
    if !site.params.ombrieres || lay.ouvrage.is_some() {
        return;
    }
    lay.ombrieres = lay.pads.clone();
    let avant = lay.trees.len();
    let ombrieres = &lay.ombrieres;
    lay.trees.retain(|&t| !ombrieres.iter().any(|q| dans(q, t)));
    lay.metrics.trees -= avant - lay.trees.len();
}

/// Disque en polygone de même aire.
fn disque(c: P2, r: f64) -> Vec<P2> {
    const N: usize = 32;
    let pas = std::f64::consts::TAU / N as f64;
    let rr = r * (pas / pas.sin()).sqrt();
    (0..N).map(|i| [c[0] + rr * (i as f64 * pas).cos(), c[1] + rr * (i as f64 * pas).sin()]).collect()
}

pub fn mesurer(site: &Site, lay: &Layout) -> Ombrage {
    if lay.ouvrage.is_some() {
        return Ombrage::default();
    }
    let r3 = |x: f64| (x * 1000.0).round() / 1000.0 + 0.0;
    let parc = parc(lay);
    let houppier = site.params.canopy.clamp(0.0, 30.0);
    let arbres = if houppier > 0.05 { reunion(lay.trees.iter().map(|&t| disque(t, houppier / 2.0))).intersection(&parc) } else { MultiPolygon::new(Vec::new()) };
    let ombrieres = reunion(lay.ombrieres.iter().map(|q| q.to_vec())).intersection(&parc);
    Ombrage {
        surface: r3(region::mp_area(&parc)),
        arbres: r3(region::mp_area(&arbres)),
        ombrieres: r3(region::mp_area(&ombrieres)),
        total: r3(region::mp_area(&arbres.union(&ombrieres))),
        nb_arbres: lay.trees.len(),
        houppier,
    }
}
