//! Parcs de stationnement en ouvrage (silos, puis enterrés) : trame structurelle, plateaux,
//! modèle BIM et export IFC.
//!
//! Lot 1 : silo à un niveau sur un plateau rectangulaire à grande portée (modules de 16 m pour
//! des places de 5,00 m et des allées de 6,00 m), poteaux en façade et en dos à dos.

pub mod bim;
pub mod ifc;
pub mod normes;
pub mod plateau;

use crate::geom::{signed_area, v2, Quad, V2};
use crate::layout::KIND_STD;
use crate::model::{Layout, Metrics, Site, StallOut, P2};
use crate::region;
use geo::{BooleanOps, MultiPolygon};
use plateau::{Emprise, Gabarit, Plan};
use serde::{Deserialize, Serialize};

/// Réglages d'un parc en ouvrage. Les dimensions de places et d'allées sont celles des
/// paramètres généraux (`Params`).
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Ouvrage {
    pub actif: bool,
    /// Épaisseur de rive en façade (garde-corps, nez de dalle), en mètres. Hypothèse d'esquisse.
    pub facade: f64,
    /// Poteau : [le long des allées, en travers], en mètres. Pré-dimensionnement à confirmer (BET).
    pub poteau: [f64; 2],
    /// Places entre deux files de poteaux le long des allées.
    pub travee: u32,
    /// Places de rive le long des façades d'extrémité.
    pub rives: bool,
    /// Direction imposée des allées (degrés) ; absente = optimisée.
    pub angle: Option<f64>,
    /// Emprise dessinée à la main ; absente = plus grand plateau de la zone constructible.
    pub emprise: Option<EmpriseIn>,
    /// Hauteur d'étage (plancher à plancher), en mètres.
    pub hauteur_etage: f64,
    /// Épaisseur de dalle, en mètres.
    pub dalle: f64,
    /// Altitude NGF du niveau de référence (plancher bas), en mètres.
    pub cote: f64,
    pub nom: String,
}

impl Default for Ouvrage {
    fn default() -> Self {
        Ouvrage {
            actif: false,
            facade: 0.20,
            poteau: [0.30, 0.60],
            travee: 3,
            rives: true,
            angle: None,
            emprise: None,
            hauteur_etage: 2.90,
            dalle: 0.40,
            cote: 0.0,
            nom: "Parc de stationnement".into(),
        }
    }
}

/// Emprise du plateau saisie à la main : centre, longueur le long des allées `w`, largeur en
/// travers `h`, direction des allées `rot` (degrés). Les dimensions sont ramenées aux plus proches
/// réalisables ; `fixe` (−1, 0 ou 1 par axe local) désigne le côté qui ne bouge pas pendant ce
/// recalage, 0 gardant le centre.
#[derive(Deserialize, Clone, Debug)]
pub struct EmpriseIn {
    pub cx: f64,
    pub cy: f64,
    pub w: f64,
    pub h: f64,
    pub rot: f64,
    #[serde(default)]
    pub fixe: [f64; 2],
}

fn emprise_manuelle(g: &Gabarit, m: &EmpriseIn) -> Emprise {
    let (larg, long) = plateau::dimensions_proches(g, m.h, m.w);
    let r = m.rot.to_radians();
    let u = v2(r.cos(), r.sin());
    let v = u.left();
    let centre = v2(m.cx, m.cy)
        - u * (m.fixe[0] * (long - m.w) / 2.0)
        - v * (m.fixe[1] * (larg - m.h) / 2.0);
    Emprise {
        o: centre - u * (long / 2.0) - v * (larg / 2.0),
        u,
        v,
        largeur: larg,
        longueur: long,
    }
}

impl Ouvrage {
    pub fn gabarit(&self, site: &Site) -> Gabarit {
        let p = &site.params;
        Gabarit {
            w: p.stall_w,
            d: p.stall_d,
            allee: p.aisle,
            facade: self.facade.max(0.0),
            pu: self.poteau[0].max(0.05),
            pv: self.poteau[1].max(0.05),
            travee: self.travee.max(1),
            rives: self.rives,
        }
    }
}

/// File de poteaux (axe de trame), prolongée de 3 m au-delà de la dalle.
#[derive(Serialize, Clone, Debug, Default)]
pub struct AxeOut {
    pub nom: String,
    pub a: P2,
    pub b: P2,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct OuvrageOut {
    /// Contour de la dalle.
    pub dalle: [P2; 4],
    /// Direction des allées (degrés).
    pub angle: f64,
    pub largeur: f64,
    pub longueur: f64,
    pub modules: u32,
    pub demi_module: bool,
    pub axes: Vec<AxeOut>,
    pub poteaux: Vec<[P2; 4]>,
    pub poteaux_cles: Vec<String>,
    pub allees: Vec<[P2; 4]>,
    pub allees_cles: Vec<String>,
    /// Clé BIM de chaque place, dans l'ordre de `Layout::stalls`.
    pub places_cles: Vec<String>,
    pub places_rive: usize,
    pub places_modules: usize,
    pub surface: f64,
    pub m2_place: f64,
    pub hauteur_etage: f64,
    pub dalle_ep: f64,
    pub cote: f64,
    pub nom: String,
}

fn r3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}
fn p2(p: V2) -> P2 {
    [r3(p.x), r3(p.y)]
}

/// Orientations à essayer (degrés, modulo 90) : directions des limites, puis un balayage tous les 3°.
fn angles_candidats(rings: &[Vec<V2>]) -> Vec<f64> {
    let mut a: Vec<f64> = Vec::new();
    for r in rings {
        for i in 0..r.len() {
            let d = r[(i + 1) % r.len()] - r[i];
            if d.len() > 5.0 {
                a.push((d.y.atan2(d.x).to_degrees().rem_euclid(90.0) * 10.0).round() / 10.0);
            }
        }
    }
    a.extend((0..30).map(|k| k as f64 * 3.0));
    let mut out: Vec<f64> = Vec::new();
    for x in a {
        if !out.iter().any(|y| (x - y).abs() < 0.05 || (x - y).abs() > 89.95) {
            out.push(x);
        }
    }
    out
}

/// Plateau d'un niveau : emprise optimisée dans la zone stationnable, puis construction.
pub fn solve(site: &Site, o: &Ouvrage) -> Layout {
    let p = &site.params;
    let mut out = Layout::default();
    let Some(z) = region::build(site) else {
        out.warnings.push("Parcelle invalide".into());
        return out;
    };
    let g = o.gabarit(site);
    let rings = region::rings_of(&z.b, 15.0);
    let (angles, impose) = match (o.angle, p.quick, p.angle_hint) {
        (Some(a), _, _) => (vec![a], true),
        (None, true, Some(h)) => (vec![h.rem_euclid(90.0)], false),
        _ => (angles_candidats(&rings), false),
    };
    let parcel_area = signed_area(&z.parcel).abs();
    out.setback_line = region::rings_of(&z.setback, 1.0)
        .iter()
        .map(|r| r.iter().map(|q| p2(*q)).collect())
        .collect();
    out.metrics = Metrics {
        parcel_area: r3(parcel_area),
        buildable_area: r3(region::mp_area(&z.b)),
        ..Default::default()
    };
    if p.slant < 89.5 {
        out.warnings.push("Silo : places à 90° seulement dans ce lot".into());
    }
    let e = if let Some(m) = &o.emprise {
        let e = emprise_manuelle(&g, m);
        let dalle = MultiPolygon::new(vec![region::to_poly(&[
            e.monde(v2(0.0, 0.0)),
            e.monde(v2(e.longueur, 0.0)),
            e.monde(v2(e.longueur, e.largeur)),
            e.monde(v2(0.0, e.largeur)),
        ])]);
        let hors = |zone: &MultiPolygon<f64>| region::mp_area(&dalle.difference(zone)) > 0.5;
        if hors(&MultiPolygon::new(vec![region::to_poly(&z.parcel)])) {
            out.warnings.push("La dalle dépasse la parcelle".into());
        } else if hors(&z.b) {
            out.warnings
                .push("La dalle sort de la zone constructible (reculs, bâtiments, servitudes, espaces verts)".into());
        }
        e
    } else {
        let Some((e, _)) = plateau::chercher(&g, &rings, &angles, 0.5, impose) else {
            out.warnings.push("Emprise trop petite pour une rangée de places et son allée".into());
            return out;
        };
        e
    };
    let e = repere_usuel(e);
    let Some(plan) = plateau::construire(&g, e.largeur, e.longueur) else {
        out.warnings.push("Emprise trop petite pour une rangée de places et son allée".into());
        return out;
    };
    remplir(&mut out, site, o, &g, &e, &plan);
    out
}

/// Oriente le repère pour la lecture du plan : files numérotées de gauche à droite (ou de bas en
/// haut pour des allées verticales), files lettrées de bas en haut (ou de gauche à droite). La dalle
/// garde son centre ; le repère peut devenir indirect, sans effet sur la géométrie.
fn repere_usuel(e: Emprise) -> Emprise {
    let centre = e.o + e.u * (e.longueur / 2.0) + e.v * (e.largeur / 2.0);
    let u = if e.u.x < -1e-9 || (e.u.x.abs() <= 1e-9 && e.u.y < 0.0) {
        -e.u
    } else {
        e.u
    };
    let l = u.left();
    let v = if l.x + l.y >= -1e-9 { l } else { -l };
    Emprise {
        o: centre - u * (e.longueur / 2.0) - v * (e.largeur / 2.0),
        u,
        v,
        ..e
    }
}

fn remplir(out: &mut Layout, site: &Site, o: &Ouvrage, g: &Gabarit, e: &Emprise, plan: &Plan) {
    let p = &site.params;
    let w = |q: V2| e.monde(q);
    let quad = |q: &[V2; 4]| [p2(w(q[0])), p2(w(q[1])), p2(w(q[2])), p2(w(q[3]))];
    let (larg, long) = (plan.largeur(), plan.longueur());

    out.stalls = plan
        .places
        .iter()
        .map(|pl| StallOut {
            q: quad(&pl.q),
            k: KIND_STD,
            e: false,
        })
        .collect();
    let n = out.stalls.len();
    let rive = plan.places.iter().filter(|pl| pl.rive).count();

    // Chaussée : réunion des allées.
    let aq: Vec<Quad> = plan
        .allees
        .iter()
        .map(|(q, _)| Quad([w(q[0]), w(q[1]), w(q[2]), w(q[3])]))
        .collect();
    let paved = crate::mp_of_quads(&aq);
    out.paved = region::rings_of(&paved, 0.5)
        .iter()
        .map(|r| r.iter().map(|q| p2(*q)).collect())
        .collect();

    let corners = [v2(0.0, 0.0), v2(long, 0.0), v2(long, larg), v2(0.0, larg)];
    let ext = 3.0;
    let mut axes: Vec<AxeOut> = Vec::new();
    for (i, &u) in plan.axes_u.iter().enumerate() {
        axes.push(AxeOut {
            nom: (i + 1).to_string(),
            a: p2(w(v2(u, -ext))),
            b: p2(w(v2(u, larg + ext))),
        });
    }
    for (k, &v) in plan.axes_v.iter().enumerate() {
        axes.push(AxeOut {
            nom: plateau::lettre(k),
            a: p2(w(v2(-ext, v))),
            b: p2(w(v2(long + ext, v))),
        });
    }
    let (hu, hv) = (g.pu / 2.0, g.pv / 2.0);
    let surface = larg * long;
    let angle = e.u.y.atan2(e.u.x).to_degrees().rem_euclid(180.0);
    let need = |ratio: f64| -> usize {
        if ratio <= 0.0 || n == 0 {
            0
        } else {
            ((n as f64 * ratio) - 1e-9).ceil().max(1.0) as usize
        }
    };
    let irve_req = need(p.irve_ratio);
    out.metrics.stalls = n;
    out.metrics.standard = n;
    out.metrics.pmr_required = need(p.pmr_ratio);
    out.metrics.irve_required = irve_req;
    out.metrics.irve_equipped_required = if irve_req > 0 && p.irve_equip_ratio > 0.0 {
        (((n as f64 * p.irve_equip_ratio) + 1e-9).floor() as usize).clamp(1, irve_req)
    } else {
        0
    };
    out.metrics.paved_area = r3(surface);
    out.metrics.perimeter_stalls = rive;
    out.metrics.interior_stalls = n - rive;
    if n > 0 {
        out.metrics.m2_paved_per_stall = r3(surface / n as f64);
        out.metrics.m2_parcel_per_stall = r3(out.metrics.parcel_area / n as f64);
    }
    // Pas d'espaces verts calculés en ouvrage : la parcelle hors dalle n'est pas qualifiée.
    out.metrics.green_area = 0.0;
    out.angle = r3(angle);
    out.row_aisle = p.aisle;
    out.warnings
        .push("Silo, lot 1 : plateau unique, sans rampes ni noyaux ; PMR et IRVE à placer avec les ascenseurs".into());
    out.ouvrage = Some(OuvrageOut {
        dalle: [p2(w(corners[0])), p2(w(corners[1])), p2(w(corners[2])), p2(w(corners[3]))],
        angle: r3(angle),
        largeur: r3(larg),
        longueur: r3(long),
        modules: plan.travers.modules,
        demi_module: plan.travers.demi,
        axes,
        poteaux: plan
            .poteaux
            .iter()
            .map(|po| quad(&[po.c + v2(-hu, -hv), po.c + v2(hu, -hv), po.c + v2(hu, hv), po.c + v2(-hu, hv)]))
            .collect(),
        poteaux_cles: plan.poteaux.iter().map(|po| po.cle.clone()).collect(),
        allees: plan.allees.iter().map(|(q, _)| quad(q)).collect(),
        allees_cles: plan.allees.iter().map(|(_, c)| c.clone()).collect(),
        places_cles: plan.places.iter().map(|pl| pl.cle.clone()).collect(),
        places_rive: rive,
        places_modules: n - rive,
        surface: r3(surface),
        m2_place: if n > 0 { r3(surface / n as f64) } else { 0.0 },
        hauteur_etage: o.hauteur_etage,
        dalle_ep: o.dalle,
        cote: o.cote,
        nom: o.nom.clone(),
    });
}
