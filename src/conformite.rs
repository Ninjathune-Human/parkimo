//! Contrôle de conformité : le plan fini est confronté au référentiel (`normes`) et chaque règle
//! rend un constat. Le contrôle observe : il ne modifie ni les places ni la circulation.

use crate::layout::{KIND_PMR, KIND_PMR_IRVE};
use crate::model::{Layout, Programme, Site, P2};
use crate::normes::{accessibilite, irve, nf, ps, urbanisme, Val};
use crate::ouvrage::plateau::Densite;
use crate::ouvrage::{Ouvrage, OuvrageOut};
use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Statut {
    Conforme,
    Ecart,
    /// Règle applicable que le plan ne permet pas de trancher : à justifier par le concepteur.
    Info,
}

/// Résultat d'une règle : ce qui est mesuré sur le plan, ce que le texte exige, et sa source.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Constat {
    /// Identifiant stable de la règle (`nf.place.largeur`).
    pub code: String,
    pub libelle: String,
    pub source: String,
    pub statut: Statut,
    pub mesure: Option<f64>,
    pub exige: Option<f64>,
    pub unite: String,
    /// Précision en une phrase, vide si les deux valeurs suffisent.
    pub detail: String,
}

fn r3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

/// Constat chiffré : conforme si la mesure atteint l'exigence (`au_moins`) ou ne la dépasse pas.
fn chiffre(code: &str, libelle: &str, v: Val, mesure: f64, exige: f64, au_moins: bool) -> Constat {
    let ok = if au_moins { mesure >= exige - 1e-6 } else { mesure <= exige + 1e-6 };
    Constat {
        code: code.into(),
        libelle: libelle.into(),
        source: v.source.into(),
        statut: if ok { Statut::Conforme } else { Statut::Ecart },
        mesure: Some(r3(mesure)),
        exige: Some(r3(exige)),
        unite: v.unite.into(),
        detail: String::new(),
    }
}

/// Constat d'information : règle applicable, à justifier par le concepteur.
fn info(code: &str, libelle: &str, source: &str, detail: &str) -> Constat {
    Constat {
        code: code.into(),
        libelle: libelle.into(),
        source: source.into(),
        statut: Statut::Info,
        mesure: None,
        exige: None,
        unite: String::new(),
        detail: detail.into(),
    }
}

const METRE: &str = "m";

fn metres(source: &'static str) -> Val {
    Val { v: 0.0, unite: METRE, source }
}

fn nombre(source: &'static str) -> Val {
    Val { v: 0.0, unite: "places", source }
}

/// Largeur d'une place, perpendiculairement à son axe : la plus petite du fond et de l'ouverture.
fn largeur(q: &[P2; 4]) -> f64 {
    let d = |a: P2, b: P2| [b[0] - a[0], b[1] - a[1]];
    let axe = d(q[0], q[3]);
    let l = (axe[0] * axe[0] + axe[1] * axe[1]).sqrt().max(1e-9);
    let travers = |v: [f64; 2]| ((v[0] * axe[1] - v[1] * axe[0]) / l).abs();
    travers(d(q[0], q[1])).min(travers(d(q[3], q[2])))
}

pub fn controler(site: &Site, lay: &Layout) -> Vec<Constat> {
    let p = &site.params;
    let places = lay.metrics.stalls;
    let mut out = Vec::new();
    if places == 0 {
        return out;
    }
    let ouvrage = lay.ouvrage.as_ref().zip(site.ouvrage.as_ref());
    if p.programme == Programme::Public {
        dimensions(&mut out, site, lay, ouvrage.is_some());
        places_adaptees(&mut out, site, lay);
        if let Some((o, reglages)) = ouvrage {
            structure(&mut out, site, o, reglages);
        }
    } else {
        out.push(info(
            "programme",
            "Programme privatif",
            "NF P 91-120",
            "Règles du parc public non contrôlées (NF P 91-100, accessibilité et sécurité des ERP) ; NF P 91-120 non intégrée",
        ));
    }
    recharge(&mut out, lay);
    if ouvrage.is_none() {
        parc_exterieur(&mut out, lay);
    }
    out
}

/// Places et allées : tableau 2 de la NF P 91-100 pour l'inclinaison du plan (90° en ouvrage).
fn dimensions(out: &mut Vec<Constat>, site: &Site, lay: &Layout, ouvrage: bool) {
    let p = &site.params;
    let angle = if ouvrage { 90.0 } else { p.slant_deg() };
    let Some(d) = nf::dims_angle(angle) else {
        let mut c = info(
            "nf.place.largeur",
            "Largeur des places",
            nf::SOURCE_TAB2,
            &format!("Places à {angle:.0}° : hors du tableau 2, qui va de 45° à 90°"),
        );
        c.mesure = Some(r3(p.stall_w));
        c.unite = METRE.into();
        out.push(c);
        return;
    };
    out.push(chiffre("nf.place.largeur", "Largeur des places", metres(nf::SOURCE_TAB2), p.stall_w, d.larg, true));
    let (profondeur, allee) = if ouvrage { (p.stall_d, p.aisle) } else { (p.row_d(), p.row_aisle()) };
    out.push(chiffre(
        "nf.demi_travee",
        "Place et allée (demi-travée)",
        metres("NF P 91-100, tab. 2 note 2"),
        profondeur + allee,
        d.long + d.allee,
        true,
    ));
    let etroite = lay.circulation.axes.iter().filter(|a| a.sens == 2).map(|a| a.w).fold(f64::MAX, f64::min);
    if etroite < f64::MAX {
        out.push(chiffre(
            "nf.allee.double_sens",
            "Allée à double sens",
            metres("NF P 91-100, § 3.1.2"),
            etroite,
            nf::allee_double_sens(d.allee, lay.metrics.stalls),
            true,
        ));
    }
}

/// Places adaptées des ERP : nombre, largeur mesurée sur le plan, longueur.
fn places_adaptees(out: &mut Vec<Constat>, site: &Site, lay: &Layout) {
    let p = &site.params;
    let m = &lay.metrics;
    let (requis, arrete) = accessibilite::places_pmr_erp(m.stalls);
    out.push(chiffre(
        "acc.pmr.nombre",
        "Places adaptées",
        nombre(accessibilite::PMR_PART_ERP.source),
        m.pmr as f64,
        requis as f64,
        true,
    ));
    if arrete {
        out.push(info(
            "acc.pmr.arrete",
            "Places adaptées au-delà de 500 places",
            accessibilite::PMR_MIN_AU_DELA_500.source,
            "Nombre fixé par arrêté municipal, 10 au moins",
        ));
    }
    let etroite = lay
        .stalls
        .iter()
        .filter(|s| s.k == KIND_PMR || s.k == KIND_PMR_IRVE)
        .map(|s| largeur(&s.q))
        .fold(f64::MAX, f64::min);
    if etroite < f64::MAX {
        let l = accessibilite::PMR_LARGEUR;
        out.push(chiffre("acc.pmr.largeur", "Largeur des places adaptées", l, etroite, l.v, true));
        let l = accessibilite::PMR_LONGUEUR;
        out.push(chiffre("acc.pmr.longueur", "Longueur des places adaptées", l, p.stall_d, l.v, true));
    }
}

/// Recharge des véhicules électriques : pré-équipement et points de recharge.
fn recharge(out: &mut Vec<Constat>, lay: &Layout) {
    let m = &lay.metrics;
    if m.stalls as f64 > irve::SEUIL_NEUF.v {
        out.push(chiffre(
            "irve.preequipement",
            "Places pré-équipées pour la recharge",
            nombre(irve::PART_PREEQUIPEE.source),
            m.irve as f64,
            irve::pre_equipees(m.stalls) as f64,
            true,
        ));
    }
    if m.stalls as f64 > irve::TRANCHE_BORNE.v {
        out.push(chiffre(
            "irve.bornes",
            "Points de recharge",
            nombre(irve::TRANCHE_BORNE.source),
            m.irve_equipped as f64,
            irve::bornes(m.stalls) as f64,
            true,
        ));
    }
}

/// Parc extérieur : obligations déclenchées par la surface, que le plan ne permet pas de trancher.
fn parc_exterieur(out: &mut Vec<Constat>, lay: &Layout) {
    let surface = lay.metrics.paved_area;
    for (code, libelle, seuil, detail) in [
        (
            "urba.ombrage",
            "Ombrage et eaux pluviales",
            urbanisme::SEUIL_OMBRAGE,
            "Surface enrobée de plus de 500 m² : ombrage et gestion des eaux pluviales sur la moitié de la surface du parc, à justifier",
        ),
        (
            "urba.ombrieres",
            "Ombrières à énergie renouvelable",
            urbanisme::SEUIL_OMBRIERES,
            "Surface enrobée de plus de 1 500 m² : ombrières à énergie renouvelable sur la moitié de la surface du parc, à justifier",
        ),
    ] {
        if surface > seuil.v {
            let mut c = info(code, libelle, seuil.source, detail);
            c.mesure = Some(r3(surface));
            c.exige = Some(seuil.v);
            c.unite = seuil.unite.into();
            out.push(c);
        }
    }
}

/// Parc en ouvrage : poteaux en tête de place, hauteur d'étage, évacuation, compartimentage.
fn structure(out: &mut Vec<Constat>, site: &Site, o: &OuvrageOut, reglages: &Ouvrage) {
    let p = &site.params;
    let tetes = reglages.densite != Densite::Longue || reglages.rives;
    if let Some(d) = nf::dims_angle(90.0).filter(|_| tetes) {
        let v = metres(nf::SOURCE_TAB3);
        match nf::surlargeur_poteau(reglages.retrait.max(0.0)) {
            Some(plus) => out.push(chiffre("nf.poteau.retrait", "Places au droit des poteaux", v, p.stall_w, d.larg + plus, true)),
            None => {
                let mut c = info(
                    "nf.poteau.retrait",
                    "Places au droit des poteaux",
                    nf::SOURCE_TAB3,
                    "Poteau à moins de 0,25 m de l'allée : cas non couvert par le tableau 3",
                );
                c.statut = Statut::Ecart;
                c.mesure = Some(r3(p.stall_w));
                c.unite = METRE.into();
                out.push(c);
            }
        }
    }
    out.push(chiffre(
        "nf.hauteur",
        "Hauteur d'étage",
        metres(nf::HAUTEUR_LIBRE.source),
        o.hauteur_etage,
        o.plancher.hauteur_min,
        true,
    ));
    if let Some(ev) = &o.evacuation {
        let mut c = chiffre(
            "ps.evacuation",
            "Distance à un escalier",
            metres(ps::DISTANCE_DEUX_ISSUES.source),
            ev.distance_max,
            ev.limite_deux,
            false,
        );
        if ev.hors > 0 {
            c.statut = Statut::Ecart;
            c.detail = format!("{} places au-delà de la distance admise", ev.hors);
        }
        out.push(c);
    }
    let seuil = ps::COMPARTIMENT;
    let mut c = chiffre("ps.compartiment", "Surface d'un niveau", seuil, o.surface, seuil.v, false);
    if c.statut == Statut::Ecart {
        c.statut = Statut::Info;
        c.detail = "Niveau de plus de 3 000 m² : recoupement en compartiments à prévoir".into();
    }
    out.push(c);
}
