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
/// Poutre précontrainte plate en T renversé (Poutres-Dalles, fiche Batiproduits) : semelle
/// 110 × 15 cm (la retombée), âme 60 × 30 cm, portée de 10 m au plus.
pub const POUTRE_PREFA_RETOMBEE: f64 = 0.15;
pub const POUTRE_PREFA_LARGEUR: f64 = 1.10;
pub const POUTRE_PREFA_HAUTEUR: f64 = 0.45;
pub const POUTRE_PREFA_PORTEE: f64 = 10.0;

/// Mixte acier-béton, 16 m de portée, dalle de 140 mm (ArcelorMittal, Car parks in steel ; plancher
/// mixte de 140 mm sur bac collaborant, AFGC 2017) : solives tous les 2,50 m → IPE 400 ; tous les
/// 5,00 m → IPE 550 en S355. Solives sur les files de poteaux. Dimensions : IPE 400 400 × 180 mm,
/// IPE 550 550 × 210 mm.
pub const MIXTE_DALLE: f64 = 0.14;
pub const MIXTE_SOLIVES: [(&str, f64, f64, f64); 2] = [("IPE400", 2.50, 0.40, 0.18), ("IPE550", 5.00, 0.55, 0.21)];
pub const MIXTE_PORTEE: f64 = 16.5;
/// Bac collaborant porteur sans étai jusqu'à 5,00 m (Cofraplus 220, parking Enovos).
pub const MIXTE_BAC: f64 = 5.00;
/// Poteau HEA 180 (h 171 × b 180 mm) : parking CHL à Luxembourg, travées de 16,5 m.
pub const MIXTE_POTEAU: [f64; 2] = [0.180, 0.171];

/// Dalle post-contrainte allégée de 40 cm pour 16 m de portée, sans retombée (silo République,
/// Reims, 2020).
pub const PT_DALLE: f64 = 0.40;
pub const PT_PORTEE: f64 = 16.5;

/// Poutres en béton armé : h ≥ L/15 (BAEL 91), largeur 0,4 h à 0,7 h ; dalle pleine sur deux
/// appuis : e ≥ L/30.
pub const BA_POUTRE: f64 = 15.0;
pub const BA_LARGEUR: f64 = 0.4;
pub const BA_DALLE: f64 = 30.0;

/// Plancher-dalle : poteaux espacés de 8 à 12 m, dalle de 22 à 35 cm (interpolation linéaire).
pub const PD_PORTEES: [f64; 2] = [8.0, 12.0];
pub const PD_EPAISSEURS: [f64; 2] = [0.22, 0.35];

/// Sens des poutres : en travers des allées (sur les files transversales 1, 2, 3…) ou le long des
/// allées (sur les files longitudinales A, B, C…). Elles vont de poteau à poteau.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sens {
    Travers,
    Long,
}

/// Poutres ou solives du plancher.
#[derive(Clone, Debug, PartialEq)]
pub struct Ossature {
    pub sens: Sens,
    /// Solives (acier) ou poutres (béton).
    pub solive: bool,
    /// Section : largeur et hauteur totale, m.
    pub b: f64,
    pub h: f64,
    /// Profil ou section pour les types BIM (`IPE550`, `BA_32x78`…).
    pub profil: String,
}

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
    /// Poutres ou solives ; absentes pour une dalle sans retombée.
    pub ossature: Option<Ossature>,
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

/// Plancher de `s` pour la densité `d`. `portee` : plus grand entraxe des files en travers des allées ;
/// `travee_rangees` : plus grand entraxe des files le long des rangées ; `travee_bouts` : plus grand
/// entraxe dans les zones d'extrémité (places de pignon et allée transversale).
pub fn plancher(s: Systeme, d: Densite, portee: f64, travee_rangees: f64, travee_bouts: f64) -> Plancher {
    let mut alertes = Vec::new();
    let travee = travee_rangees.max(travee_bouts);
    match s {
        Systeme::Mixte => {
            if d != Densite::Longue {
                alertes.push("Acier-béton : portée longue seulement (solives de 15 à 16 m)".into());
            }
            if portee > MIXTE_PORTEE + 1e-9 {
                alertes.push(format!("Acier-béton : portée de {} m au-delà des solives de 16 m", m(portee)));
            }
            // Solives sur les files de poteaux : leur entraxe est la travée des rangées.
            let (profil, _, h, b) = MIXTE_SOLIVES
                .iter()
                .copied()
                .find(|&(_, e, _, _)| travee_rangees <= e + 1e-9)
                .unwrap_or_else(|| {
                    alertes.push(format!(
                        "Acier-béton : solives à {} m d'entraxe, au-delà des 5,00 m du guide ArcelorMittal (profil à dimensionner)",
                        m(travee_rangees)
                    ));
                    MIXTE_SOLIVES[MIXTE_SOLIVES.len() - 1]
                });
            if travee_bouts > MIXTE_BAC + 1e-9 {
                alertes.push(format!(
                    "Acier-béton : solives à {} m d'entraxe aux extrémités, bac collaborant à étayer (5,00 m sans étai)",
                    m(travee_bouts)
                ));
            }
            let ipe = profil.replace("IPE", "IPE ");
            Plancher {
                code: format!("Mixte14_{profil}"),
                court: format!("Mixte 14 cm, {ipe}"),
                libelle: format!(
                    "Dalle mixte 14 cm sur bac collaborant, solives {ipe} sur les files de poteaux (tous les {} m au plus)",
                    m(travee_rangees)
                ),
                epaisseur: MIXTE_DALLE,
                retombee: h,
                source: "ArcelorMittal, Car parks in steel ; AFGC 2017 (parcs largement ventilés en acier)",
                ossature: Some(Ossature { sens: Sens::Travers, solive: true, b, h, profil: profil.into() }),
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
            if travee > POUTRE_PREFA_PORTEE + 1e-9 {
                alertes.push(format!(
                    "Poutres précontraintes : portée de {} m au-delà des 10 m de la gamme à faible retombée",
                    m(travee)
                ));
            }
            Plancher {
                code: format!("DAP_{r}+5"),
                court: format!("DAP {ep:.0} + 5"),
                libelle: format!(
                    "Dalles alvéolées {r} ({ep:.0} cm) et dalle de compression 5 cm, sur poutres précontraintes à faible retombée"
                ),
                epaisseur: e + DAP_COMPRESSION,
                retombee: POUTRE_PREFA_RETOMBEE,
                source: SOURCE_DAP,
                ossature: Some(Ossature {
                    sens: Sens::Long,
                    solive: false,
                    b: POUTRE_PREFA_LARGEUR,
                    h: POUTRE_PREFA_HAUTEUR,
                    profil: "PP_T_110x45".into(),
                }),
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
                    ossature: None,
                    alertes,
                }
            }
            Densite::Moyenne => {
                let h = cm_sup(portee / BA_POUTRE);
                let b = cm_sup(BA_LARGEUR * h);
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
                    source: "BAEL 91 : poutres L/15 à L/10, largeur 0,4 h à 0,7 h ; dalle sur deux appuis L/30 à L/35",
                    ossature: Some(Ossature {
                        sens: Sens::Travers,
                        solive: false,
                        b,
                        h,
                        profil: format!("BA_{:.0}x{:.0}", b * 100.0, h * 100.0),
                    }),
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
                    ossature: None,
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
        // Acier-béton, solives tous les 2,50 m : 14 + IPE 400 → étage ≥ max(2,34 ; 2,00 + 0,54) = 2,54.
        let p = plancher(Systeme::Mixte, Densite::Longue, 15.7, 2.5, 2.5);
        assert!(eq(p.epaisseur, 0.14) && eq(p.retombee, 0.40) && eq(p.hauteur_min(), 2.54) && p.alertes.is_empty());
        // Tous les 5,00 m : IPE 550 (550 × 210) → étage ≥ 2,00 + 0,14 + 0,55 = 2,69 ; extrémités à
        // 6,95 m : bac à étayer.
        let p = plancher(Systeme::Mixte, Densite::Longue, 15.7, 5.0, 6.95);
        let o = p.ossature.as_ref().unwrap();
        assert!(o.profil == "IPE550" && o.sens == Sens::Travers && eq(o.b, 0.21) && eq(p.hauteur_min(), 2.69));
        assert!(p.alertes.len() == 1 && p.alertes[0].contains("étayer"));
        // 7,50 m : hors guide.
        assert!(plancher(Systeme::Mixte, Densite::Longue, 15.7, 7.5, 2.5).alertes[0].contains("au-delà"));
        assert!(!plancher(Systeme::Mixte, Densite::Courte, 8.1, 2.5, 2.5).alertes.is_empty());
        // Préfabriqué, courte : 8,10 × 1,4 = 11,34 ≤ 11,5 → NVS20 + 5 = 0,25 ; étage ≥ max(2,45 ; 2,40).
        let p = plancher(Systeme::Prefa, Densite::Courte, 8.1, 7.2, 2.4);
        assert_eq!(p.code, "DAP_NVS20+5");
        assert!(eq(p.epaisseur, 0.25) && eq(p.hauteur_min(), 2.45) && p.alertes.is_empty());
        let o = p.ossature.as_ref().unwrap();
        assert!(o.sens == Sens::Long && eq(o.b, 1.10) && eq(o.h, 0.45));
        // Préfabriqué, moyenne : 11,65 × 1,4 = 16,31 ≤ 17 → NSC27 + 5 = 0,32 ; étage ≥ 2,52.
        let p = plancher(Systeme::Prefa, Densite::Moyenne, 11.65, 7.2, 2.4);
        assert_eq!(p.code, "DAP_NSC27+5");
        assert!(eq(p.hauteur_min(), 2.52));
        // Préfabriqué, longue : 15,70 × 1,4 = 21,98 > 19 → NSC40 + 5, à valider (19 ≥ 15,70).
        let p = plancher(Systeme::Prefa, Densite::Longue, 15.7, 7.2, 2.4);
        assert_eq!(p.code, "DAP_NSC40+5");
        assert!(eq(p.epaisseur, 0.45) && p.alertes.len() == 1);
        // Coulé, longue : post-contrainte 40 cm, étage ≥ 2,60, sans poutre.
        let p = plancher(Systeme::Coule, Densite::Longue, 15.7, 7.5, 6.95);
        assert!(eq(p.hauteur_min(), 2.60) && p.alertes.is_empty() && p.ossature.is_none());
        // Coulé, moyenne : poutres 11,65 / 15 = 0,777 → 0,78, largeur 0,4 × 0,78 = 0,312 → 0,32 ;
        // dalle 7,50 / 30 = 0,25 ; étage ≥ 2,78.
        let p = plancher(Systeme::Coule, Densite::Moyenne, 11.65, 7.5, 6.95);
        assert_eq!(p.code, "BA_P78_D25");
        assert!(eq(p.epaisseur, 0.25) && eq(p.retombee, 0.53) && eq(p.hauteur_min(), 2.78));
        assert_eq!(p.ossature.as_ref().unwrap().profil, "BA_32x78");
        // Coulé, courte : 8,10 → 0,22 + 0,1 / 4 × 0,13 = 0,2233 → 0,23 ; étage ≥ 2,43.
        let p = plancher(Systeme::Coule, Densite::Courte, 8.1, 7.5, 6.95);
        assert_eq!(p.code, "PD23");
        assert!(eq(p.epaisseur, 0.23) && eq(p.hauteur_min(), 2.43));
    }
}
