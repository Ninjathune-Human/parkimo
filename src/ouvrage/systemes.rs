//! Systèmes constructifs du marché français pour les parcs en ouvrage. À chaque système et à
//! chaque densité de trame correspond un plancher réel (produit, épaisseur, retombée), avec sa
//! source. Valeurs d'esquisse, à confirmer par le bureau d'études et le fabricant.
//!
//! Les portées sont des entraxes de files de poteaux (valeur sûre : la portée entre nus d'appuis
//! est plus courte).

use super::normes::nf;
use super::plateau::Densite;
use serde::Deserialize;

/// Système constructif du parc.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Systeme {
    /// Béton coulé en place : dalle post-contrainte, poutres et dalle pleine, plancher-dalle.
    #[default]
    Coule,
    /// Préfabriqué béton : poteaux à corbeaux, poutres précontraintes, dalles alvéolées.
    Prefa,
    /// Mixte acier-béton : solives IPE et dalle mixte sur bac collaborant (parc largement ventilé).
    Mixte,
}

impl Systeme {
    pub fn nom(self) -> &'static str {
        match self {
            Systeme::Coule => "coule",
            Systeme::Prefa => "prefa",
            Systeme::Mixte => "mixte",
        }
    }
    /// Matériau des poteaux, pour les types BIM.
    pub fn materiau_poteau(self) -> &'static str {
        match self {
            Systeme::Mixte => "Acier",
            _ => "BA",
        }
    }
}

/// Dalles alvéolées précontraintes NERVA : référence, épaisseur, portée maximale (m). Largeur 1,20 m.
pub const DAP: [(&str, f64, f64); 9] = [
    ("NVS12", 0.12, 7.5),
    ("NVS15", 0.15, 8.0),
    ("NVS18", 0.18, 10.0),
    ("NVS20", 0.20, 11.5),
    ("NSC24", 0.24, 14.5),
    ("NSC27", 0.27, 17.0),
    ("NSC32", 0.32, 17.5),
    ("NSC36", 0.36, 17.5),
    ("NSC40", 0.40, 19.0),
];
pub const SOURCE_DAP: &str = "Rector, brochure dalle alvéolée NERVA";
pub const DAP_LARGEUR: f64 = 1.20;
/// Portée courante ≈ portée maximale / 1,4 : rapport des FDES NERVA (SCF24 10,50 / 14,50 ;
/// SCF27 11,50 / 17,20 ; VSF15 5,50 / 8,00).
pub const DAP_COURANTE: f64 = 1.4;
/// Dalle de compression rapportée : 5 cm au moins (Rector), obligatoire en zone sismique.
pub const DAP_COMPRESSION: f64 = 0.05;
/// Poutre précontrainte plate en T renversé : retombée de 15 cm au plus pour 8 à 10 m de portée
/// (Poutres-Dalles, fiche Batiproduits).
pub const POUTRE_PREFA_RETOMBEE: f64 = 0.15;

/// Mixte acier-béton : 16 m de portée, solives tous les 2,50 m, dalle de 140 mm → IPE 400
/// (ArcelorMittal, Car parks in steel ; plancher mixte de 140 mm sur bac collaborant, AFGC 2017).
pub const MIXTE_DALLE: f64 = 0.14;
pub const MIXTE_SOLIVE: f64 = 0.40;
pub const MIXTE_ENTRAXE: f64 = 2.50;
pub const MIXTE_PORTEE: f64 = 16.5;
/// Poteau HEA 180 (h 171 × b 180 mm) : parking CHL à Luxembourg, travées de 16,5 m.
pub const MIXTE_POTEAU: [f64; 2] = [0.180, 0.171];

/// Dalle post-contrainte allégée de 40 cm pour 16 m de portée, sans retombée (silo République,
/// Reims, 2020).
pub const PT_DALLE: f64 = 0.40;
pub const PT_PORTEE: f64 = 16.5;

/// Poutres en béton armé : h ≥ L/15 (BAEL 91) ; dalle pleine sur deux appuis : e ≥ L/30.
pub const BA_POUTRE: f64 = 15.0;
pub const BA_DALLE: f64 = 30.0;

/// Plancher-dalle : poteaux espacés de 8 à 12 m, dalle de 22 à 35 cm (interpolation linéaire).
pub const PD_PORTEES: [f64; 2] = [8.0, 12.0];
pub const PD_EPAISSEURS: [f64; 2] = [0.22, 0.35];

/// Plancher retenu pour un système, une densité et une trame.
#[derive(Clone, Debug, PartialEq)]
pub struct Plancher {
    /// Code des types BIM (`PKM_Plancher_{code}`).
    pub code: String,
    /// Libellé court (bilan).
    pub court: String,
    pub libelle: String,
    /// Épaisseur du plancher (dalle et dalle de compression), m.
    pub epaisseur: f64,
    /// Retombée sous le plancher (poutres, solives), m.
    pub retombee: f64,
    pub source: &'static str,
    /// Avertissements : portée hors gamme, densité non prévue pour le système.
    pub alertes: Vec<String>,
}

impl Plancher {
    /// Hauteur d'étage minimale : 2,20 m sous plafond, 2,00 m sous poutre (NF P 91-100, § 3.2).
    pub fn hauteur_min(&self) -> f64 {
        (nf::HAUTEUR_LIBRE.v + self.epaisseur).max(nf::HAUTEUR_SOUS_OBSTACLE.v + self.epaisseur + self.retombee)
    }
}

/// Arrondi au centimètre supérieur.
fn cm_sup(x: f64) -> f64 {
    (x * 100.0 - 1e-9).ceil() / 100.0
}

fn m(x: f64) -> String {
    format!("{x:.2}").replace('.', ",")
}

/// Plancher de `s` pour la densité `d` : `portee` = plus grand entraxe des files en travers des
/// allées, `travee` = plus grand entraxe le long des allées, `entraxe_places` = largeur de place.
pub fn plancher(s: Systeme, d: Densite, portee: f64, travee: f64, entraxe_places: f64) -> Plancher {
    let mut alertes = Vec::new();
    match s {
        Systeme::Mixte => {
            if d != Densite::Longue {
                alertes.push("Acier-béton : portée longue seulement (solives de 15 à 16 m)".into());
            }
            if portee > MIXTE_PORTEE + 1e-9 {
                alertes.push(format!("Acier-béton : portée de {} m au-delà des solives de 16 m", m(portee)));
            }
            if entraxe_places > MIXTE_ENTRAXE + 1e-9 {
                alertes.push(format!(
                    "Acier-béton : solives à {} m d'entraxe, IPE 400 donnée pour 2,50 m (profil à vérifier)",
                    m(entraxe_places)
                ));
            }
            Plancher {
                code: "Mixte14_IPE400".into(),
                court: "Mixte 14 cm, IPE 400".into(),
                libelle: format!(
                    "Dalle mixte 14 cm sur bac collaborant, solives IPE 400 tous les {} m",
                    m(entraxe_places)
                ),
                epaisseur: MIXTE_DALLE,
                retombee: MIXTE_SOLIVE,
                source: "ArcelorMittal, Car parks in steel ; AFGC 2017 (parcs largement ventilés en acier)",
                alertes,
            }
        }
        Systeme::Prefa => {
            // Plus petite dalle dont la portée courante couvre la portée ; à défaut, la plus épaisse
            // dont la portée maximale la couvre, à valider avec le fabricant.
            let (r, e, _) = DAP
                .iter()
                .copied()
                .find(|&(_, _, l)| l / DAP_COURANTE >= portee - 1e-9)
                .unwrap_or_else(|| {
                    let der = DAP[DAP.len() - 1];
                    if der.2 >= portee - 1e-9 {
                        alertes.push(format!(
                            "Dalles alvéolées : portée de {} m au-delà de la portée courante ({} m), à valider avec le fabricant",
                            m(portee),
                            m(der.2 / DAP_COURANTE)
                        ));
                    } else {
                        alertes.push(format!(
                            "Dalles alvéolées : portée de {} m au-delà de la gamme ({} m)",
                            m(portee),
                            m(der.2)
                        ));
                    }
                    der
                });
            let ep = e * 100.0;
            Plancher {
                code: format!("DAP_{r}+5"),
                court: format!("DAP {ep:.0} + 5"),
                libelle: format!(
                    "Dalles alvéolées {r} ({ep:.0} cm) et dalle de compression 5 cm, sur poutres précontraintes à faible retombée"
                ),
                epaisseur: e + DAP_COMPRESSION,
                retombee: POUTRE_PREFA_RETOMBEE,
                source: SOURCE_DAP,
                alertes,
            }
        }
        Systeme::Coule => match d {
            Densite::Longue => {
                if portee > PT_PORTEE + 1e-9 {
                    alertes.push(format!("Dalle post-contrainte : portée de {} m au-delà de la référence de 16 m", m(portee)));
                }
                Plancher {
                    code: "PT40".into(),
                    court: "Post-contrainte 40 cm".into(),
                    libelle: "Dalle post-contrainte allégée 40 cm, sans retombée".into(),
                    epaisseur: PT_DALLE,
                    retombee: 0.0,
                    source: "Silo République, Reims (2020) : 16 m, 40 cm",
                    alertes,
                }
            }
            Densite::Moyenne => {
                let h = cm_sup(portee / BA_POUTRE);
                let e = cm_sup(travee / BA_DALLE);
                Plancher {
                    code: format!("BA_P{:.0}_D{:.0}", h * 100.0, e * 100.0),
                    court: format!("Poutres {:.0} + dalle {:.0}", h * 100.0, e * 100.0),
                    libelle: format!(
                        "Poutres BA de {:.0} cm en travers (L/15) et dalle pleine de {:.0} cm (L/30)",
                        h * 100.0,
                        e * 100.0
                    ),
                    epaisseur: e,
                    retombee: (h - e).max(0.0),
                    source: "BAEL 91 : poutres L/15 à L/10 ; dalle sur deux appuis L/30 à L/35",
                    alertes,
                }
            }
            Densite::Courte => {
                let l = portee.max(travee);
                if l > PD_PORTEES[1] + 1e-9 {
                    alertes.push(format!("Plancher-dalle : portée de {} m au-delà de 12 m", m(l)));
                }
                let k = ((l - PD_PORTEES[0]) / (PD_PORTEES[1] - PD_PORTEES[0])).clamp(0.0, 1.0);
                let e = cm_sup(PD_EPAISSEURS[0] + k * (PD_EPAISSEURS[1] - PD_EPAISSEURS[0]));
                Plancher {
                    code: format!("PD{:.0}", e * 100.0),
                    court: format!("Plancher-dalle {:.0} cm", e * 100.0),
                    libelle: format!("Plancher-dalle BA de {:.0} cm, sans poutre", e * 100.0),
                    epaisseur: e,
                    retombee: 0.0,
                    source: "Plancher-dalle : poteaux à 8 à 12 m, dalle de 22 à 35 cm",
                    alertes,
                }
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn planchers_a_la_main() {
        // Plateau de deux modules, places de 2,50 m, travée de 3 places (7,50 m) ; portées en
        // travers : longue 15,70, moyenne 11,65, courte 8,10 (voir plateau::densites_a_la_main).
        // Acier-béton : 14 + 40 → étage ≥ max(2,20 + 0,14 ; 2,00 + 0,54) = 2,54.
        let p = plancher(Systeme::Mixte, Densite::Longue, 15.7, 7.5, 2.5);
        assert!(eq(p.epaisseur, 0.14) && eq(p.retombee, 0.40) && eq(p.hauteur_min(), 2.54) && p.alertes.is_empty());
        assert!(!plancher(Systeme::Mixte, Densite::Courte, 8.1, 7.5, 2.5).alertes.is_empty());
        // Préfabriqué, courte : 8,10 × 1,4 = 11,34 ≤ 11,5 → NVS20 + 5 = 0,25 ; étage ≥ max(2,45 ; 2,40).
        let p = plancher(Systeme::Prefa, Densite::Courte, 8.1, 7.2, 2.4);
        assert_eq!(p.code, "DAP_NVS20+5");
        assert!(eq(p.epaisseur, 0.25) && eq(p.hauteur_min(), 2.45) && p.alertes.is_empty());
        // Préfabriqué, moyenne : 11,65 × 1,4 = 16,31 ≤ 17 → NSC27 + 5 = 0,32 ; étage ≥ 2,52.
        let p = plancher(Systeme::Prefa, Densite::Moyenne, 11.65, 7.2, 2.4);
        assert_eq!(p.code, "DAP_NSC27+5");
        assert!(eq(p.hauteur_min(), 2.52));
        // Préfabriqué, longue : 15,70 × 1,4 = 21,98 > 19 → NSC40 + 5, à valider (19 ≥ 15,70).
        let p = plancher(Systeme::Prefa, Densite::Longue, 15.7, 7.2, 2.4);
        assert_eq!(p.code, "DAP_NSC40+5");
        assert!(eq(p.epaisseur, 0.45) && p.alertes.len() == 1);
        // Coulé, longue : post-contrainte 40 cm, étage ≥ 2,60.
        let p = plancher(Systeme::Coule, Densite::Longue, 15.7, 7.5, 2.5);
        assert!(eq(p.hauteur_min(), 2.60) && p.alertes.is_empty());
        // Coulé, moyenne : poutres 11,65 / 15 = 0,777 → 0,78 ; dalle 7,50 / 30 = 0,25 ; étage ≥ 2,78.
        let p = plancher(Systeme::Coule, Densite::Moyenne, 11.65, 7.5, 2.5);
        assert_eq!(p.code, "BA_P78_D25");
        assert!(eq(p.epaisseur, 0.25) && eq(p.retombee, 0.53) && eq(p.hauteur_min(), 2.78));
        // Coulé, courte : 8,10 → 0,22 + 0,1 / 4 × 0,13 = 0,2233 → 0,23 ; étage ≥ 2,43.
        let p = plancher(Systeme::Coule, Densite::Courte, 8.1, 7.5, 2.5);
        assert_eq!(p.code, "PD23");
        assert!(eq(p.epaisseur, 0.23) && eq(p.hauteur_min(), 2.43));
    }
}
