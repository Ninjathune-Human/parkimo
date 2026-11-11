//! Valeurs normatives et réglementaires communes à la surface et à l'ouvrage, chacune avec sa source.
//!
//! Les fonctions de calcul (longueur de rampe, flèche, pente admissible) sont des dérivations
//! géométriques : elles ne figurent dans aucun texte et sont présentées comme « calculées ».

/// Valeur chiffrée, son unité et sa référence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Val {
    pub v: f64,
    pub unite: &'static str,
    pub source: &'static str,
}

const fn val(v: f64, unite: &'static str, source: &'static str) -> Val {
    Val { v, unite, source }
}

/// NF P 91-100 (mai 1994), parcs de stationnement accessibles au public.
pub mod nf {
    use super::{val, Val};

    pub const HAUTEUR_LIBRE: Val = val(2.20, "m", "NF P 91-100, § 3.2");
    pub const HAUTEUR_SOUS_OBSTACLE: Val = val(2.00, "m", "NF P 91-100, § 3.2");
    pub const HAUTEUR_RAMPE: Val = val(2.00, "m", "NF P 91-100, § 3.2 (perpendiculaire à la rampe)");
    pub const EMPATTEMENT: Val = val(3.00, "m", "NF P 91-100, § 3.2");

    pub const VOIE_DROITE_SENS_UNIQUE: Val = val(3.00, "m", "NF P 91-100, § 3.3");
    pub const VOIE_DROITE_DOUBLE_SENS: Val = val(5.50, "m", "NF P 91-100, § 3.3");
    pub const VOIE_COURBE_SENS_UNIQUE: Val = val(4.00, "m", "NF P 91-100, § 3.3");
    pub const VOIE_COURBE_INTERIEURE: Val = val(3.50, "m", "NF P 91-100, § 3.3");
    pub const VOIE_COURBE_EXTERIEURE: Val = val(3.00, "m", "NF P 91-100, § 3.3");
    pub const RAYON_EXTERIEUR_MIN: Val = val(8.50, "m", "NF P 91-100, § 3.3");

    pub const PENTE_RAMPE_MAX: Val = val(0.18, "", "NF P 91-100, § 3.4");
    pub const PENTE_DEBOUCHE_MAX: Val = val(0.05, "", "NF P 91-100, § 3.4, fig. 8");
    pub const LONGUEUR_DEBOUCHE: Val = val(4.00, "m", "NF P 91-100, § 3.4, fig. 8");
    pub const RAYON_SOMMET_MIN: Val = val(10.0, "m", "NF P 91-100, § 3.4, fig. 9");
    pub const RAYON_PIED_MIN: Val = val(15.0, "m", "NF P 91-100, § 3.4, fig. 9");
    pub const PENTE_PLANCHER_MIN: Val = val(0.01, "", "NF P 91-100, § 3.4");
    pub const PENTE_PLACE_AXE_MAX: Val = val(0.05, "", "NF P 91-100, § 3.4");
    pub const PENTE_PLACE_MAX: Val = val(0.075, "", "NF P 91-100, § 3.4");
    pub const PENTE_PEAGE_MAX: Val = val(0.03, "", "NF P 91-100, § 3.4");

    /// Obstacle contenu dans les 1,10 premiers mètres depuis le fond de place : largeur au droit ≥ 2,00
    /// (lecture de la fig. 2, retenue par le skill NF ; la note 3 du tableau 2 diverge).
    pub const OBSTACLE_FOND: Val = val(1.10, "m", "NF P 91-100, tab. 2 note 3 et fig. 2");
    pub const LARGEUR_AU_DROIT_OBSTACLE: Val = val(2.00, "m", "NF P 91-100, tab. 2 note 3 et fig. 2");
    pub const SURLARGEUR_VOILE_UN_COTE: Val = val(0.20, "m", "NF P 91-100, fig. 1");
    pub const SURLARGEUR_VOILE_DEUX_COTES: Val = val(0.30, "m", "NF P 91-100, fig. 1");

    /// Ligne du tableau 2 : angle, allée, L (perpendiculaire à l'allée), l, L en chevron.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Angle {
        pub a: f64,
        pub allee: f64,
        pub long: f64,
        pub larg: f64,
        pub chevron: Option<f64>,
    }

    pub const TAB2: [Angle; 4] = [
        Angle { a: 90.0, allee: 5.00, long: 5.00, larg: 2.30, chevron: None },
        Angle { a: 75.0, allee: 4.50, long: 5.10, larg: 2.25, chevron: Some(4.80) },
        Angle { a: 60.0, allee: 4.00, long: 5.15, larg: 2.25, chevron: Some(4.60) },
        Angle { a: 45.0, allee: 3.50, long: 4.80, larg: 2.20, chevron: Some(4.00) },
    ];
    pub const SOURCE_TAB2: &str = "NF P 91-100, tab. 2";

    /// Tableau 3 : retrait du poteau par rapport au bord de l'allée → surlargeur de la place.
    pub const TAB3: [(f64, f64); 3] = [(0.65, 0.0), (0.40, 0.10), (0.25, 0.15)];
    pub const SOURCE_TAB3: &str = "NF P 91-100, tab. 3";

    /// Dimensions pour un angle entre 45° et 90°, interpolées entre les lignes du tableau 2 (§ 3.1.2).
    pub fn dims_angle(a: f64) -> Option<Angle> {
        if !(45.0..=90.0).contains(&a) {
            return None;
        }
        if let Some(r) = TAB2.iter().find(|r| (r.a - a).abs() < 1e-9) {
            return Some(*r);
        }
        let i = TAB2.windows(2).position(|w| a <= w[0].a && a >= w[1].a)?;
        let (hi, lo) = (TAB2[i], TAB2[i + 1]);
        let k = (hi.a - a) / (hi.a - lo.a);
        let f = |x: f64, y: f64| ((x + k * (y - x)) * 1000.0).round() / 1000.0;
        Some(Angle {
            a,
            allee: f(hi.allee, lo.allee),
            long: f(hi.long, lo.long),
            larg: f(hi.larg, lo.larg),
            chevron: match (hi.chevron, lo.chevron) {
                (Some(x), Some(y)) => Some(f(x, y)),
                _ => None,
            },
        })
    }

    /// Allée à double sens desservant plus de 20 places : + 20 %, 5,00 m au moins (§ 3.1.2).
    pub fn allee_double_sens(allee: f64, places: usize) -> f64 {
        if places > 20 {
            (allee * 1.2 * 100.0).round().max(500.0) / 100.0
        } else {
            allee
        }
    }

    /// Surlargeur imposée par un poteau entre places selon son retrait (tableau 3, ligne la plus
    /// stricte pour un retrait intermédiaire). `None` : retrait inférieur à 0,25 m, non couvert.
    pub fn surlargeur_poteau(retrait: f64) -> Option<f64> {
        TAB3.iter().find(|r| retrait >= r.0 - 1e-9).map(|r| r.1)
    }
}

/// Neufert, édition française, chapitre Circulation (parkings couverts, p. 274-276).
pub mod neufert {
    use super::{val, Val};

    pub const PENTE_RAMPE_CONSEIL: Val = val(0.15, "", "Neufert p. 274-276");
    pub const PENTE_RAMPE_PETIT_GARAGE: Val = val(0.20, "", "Neufert p. 274-276");
    pub const RAYON_SOMMET: Val = val(15.0, "m", "Neufert p. 274-276 (changement de pente > 8 %)");
    pub const RAYON_PIED: Val = val(20.0, "m", "Neufert p. 274-276 (changement de pente > 8 %)");
    pub const RAMPE_SENS_UNIQUE: Val = val(3.25, "m", "Neufert p. 274-276 (2,75 + 2 × 0,25)");
    pub const RAMPE_DOUBLE_SENS: Val = val(6.50, "m", "Neufert p. 274-276");
    pub const PLANCHER_INCLINE_MAX: Val = val(0.06, "", "Neufert p. 274-276");
    /// Rampe courbe à une voie, rayons minimaux depuis le centre : mur plein intérieur, voie de
    /// 4,00 m (bords intérieur et extérieur), bordure de 0,50 m.
    pub const HELICE_MUR: Val = val(4.00, "m", "Neufert p. 274-276 (rampe courbe une voie)");
    pub const HELICE_VOIE_INT: Val = val(5.00, "m", "Neufert p. 274-276 (rampe courbe une voie)");
    pub const HELICE_VOIE_EXT: Val = val(9.00, "m", "Neufert p. 274-276 (rampe courbe une voie)");
    pub const HELICE_BORDURE: Val = val(9.50, "m", "Neufert p. 274-276 (rampe courbe une voie)");
    // Rampe courbe à deux voies (rayon mini) : parapet intérieur R 4,50 ; voie intérieure de 4,00
    // entre R 5,00 et R 9,00 ; 0,50 ; voie extérieure de 3,10 entre R 9,50 et R 12,60 (axe 11,05) ;
    // 0,50 jusqu'à R 13,10 ; 8,60 m en tout.
    pub const HELICE2_PARAPET: Val = val(4.50, "m", "Neufert p. 274-276 (rampe courbe deux voies)");
    pub const HELICE2_EXT_INT: Val = val(9.50, "m", "Neufert p. 274-276 (rampe courbe deux voies)");
    pub const HELICE2_EXT_EXT: Val = val(12.60, "m", "Neufert p. 274-276 (rampe courbe deux voies)");
    pub const HELICE2_BORDURE: Val = val(13.10, "m", "Neufert p. 274-276 (rampe courbe deux voies)");
    pub const HAUTEUR_SOUS_STRUCTURE: Val = val(2.50, "m", "Neufert p. 274-276 (2,20 + 0,25 + 0,05)");
    pub const CHARGE_PLATEAU: Val = val(3.5, "kN/m²", "Neufert p. 274-276 (à remplacer par NF EN 1991-1-1)");
    pub const CHARGE_RAMPE: Val = val(5.0, "kN/m²", "Neufert p. 274-276 (à remplacer par NF EN 1991-1-1)");
    pub const CHARGE_TOITURE_PLANTEE: Val = val(10.0, "kN/m²", "Neufert p. 274-276 (à remplacer par NF EN 1991-1-1)");
}

/// Règlement de sécurité ERP, chapitre PS (arrêté du 25 juin 1980 modifié), et guide PS de 2018.
pub mod ps {
    use super::{val, Val};

    pub const COMPARTIMENT: Val = val(3000.0, "m²", "PS 12 § 1");
    pub const COMPARTIMENT_NIVEAU: Val = val(3600.0, "m²", "PS 12 § 1 (surface du niveau)");
    pub const COMPARTIMENT_SPRINKLE: Val = val(6000.0, "m²", "PS 12 § 1");
    pub const DISTANCE_DEUX_ISSUES: Val = val(40.0, "m", "PS 13 § 1");
    pub const DISTANCE_AUTRES_CAS: Val = val(25.0, "m", "PS 13 § 1");
    pub const DISTANCE_DEUX_ISSUES_PSLV: Val = val(50.0, "m", "PS 13 § 1");
    pub const DISTANCE_AUTRES_CAS_PSLV: Val = val(30.0, "m", "PS 13 § 1");
    pub const PSLV_OUVERTURE_FACADES: Val = val(0.50, "", "PS 3");
    pub const PSLV_DISTANCE_FACADES: Val = val(75.0, "m", "PS 3");
    pub const PSLV_OUVERTURE_PLANCHER: Val = val(0.05, "", "PS 3");
    /// Débits d'extraction par véhicule et par compartiment, selon la présence de sprinkleurs
    /// (la correspondance n'est pas relevée ici : à lire dans PS 18).
    pub const DESENFUMAGE: [Val; 2] = [
        val(600.0, "m³/h", "PS 18, guide PS 2018"),
        val(900.0, "m³/h", "PS 18, guide PS 2018"),
    ];
    pub const IRVE_POINTS_COMPARTIMENT: Val = val(20.0, "", "Guide PS 2018, 2.3.1 e (PS 12)");
    pub const IRVE_KVA_COMPARTIMENT: Val = val(150.0, "kVA", "Guide PS 2018, 2.3.1 e (PS 12)");
}

/// Arrêté du 31 janvier 1986, parcs de stationnement des bâtiments d'habitation.
pub mod habitation {
    use super::{val, Val};

    pub const DISTANCE_ISSUES: Val = val(40.0, "m", "Arrêté du 31/01/1986, art. 87");
    pub const ESCALIER_LARGEUR: Val = val(0.80, "m", "Arrêté du 31/01/1986, art. 87");
}

/// Accessibilité des places adaptées.
pub mod accessibilite {
    use super::{val, Val};

    pub const PMR_LARGEUR: Val = val(3.30, "m", "Arrêté du 20/04/2017, art. 3");
    pub const PMR_LONGUEUR: Val = val(5.00, "m", "Arrêté du 20/04/2017, art. 3");
    pub const PMR_SURLONGUEUR: Val = val(1.20, "m", "Arrêté du 20/04/2017, art. 3");
    pub const PMR_DEVERS_MAX: Val = val(0.02, "", "Arrêté du 20/04/2017, art. 3");
    pub const PMR_PART_ERP: Val = val(0.02, "", "Arrêté du 20/04/2017, art. 3");
    pub const PMR_MIN_AU_DELA_500: Val = val(10.0, "", "Arrêté du 20/04/2017, art. 3");
    pub const PMR_PART_HABITATION: Val = val(0.05, "", "Arrêté du 24/12/2015, art. 3");

    /// Places adaptées requises en ERP : 2 % arrondis à l'unité supérieure, une au moins ; au-delà
    /// de 500 places, 10 au moins et nombre fixé par arrêté municipal (second terme à vrai).
    pub fn places_pmr_erp(places: usize) -> (usize, bool) {
        if places == 0 {
            return (0, false);
        }
        let n = ((places as f64 * PMR_PART_ERP.v) - 1e-9).ceil().max(1.0) as usize;
        if places > 500 {
            (n.max(PMR_MIN_AU_DELA_500.v as usize), true)
        } else {
            (n, false)
        }
    }
}

/// Recharge des véhicules électriques : code de la construction et de l'habitation.
pub mod irve {
    use super::{val, Val};

    pub const SEUIL_NEUF: Val = val(10.0, "places", "CCH, art. L. 113-12");
    pub const PART_PREEQUIPEE: Val = val(0.20, "", "CCH, art. L. 113-12 (bâtiment non résidentiel neuf)");
    pub const TRANCHE_BORNE: Val = val(20.0, "places", "CCH, art. L. 113-13");

    /// Places à pré-équiper : 20 % arrondis à l'unité supérieure, au-delà de 10 places.
    pub fn pre_equipees(places: usize) -> usize {
        if places as f64 <= SEUIL_NEUF.v {
            return 0;
        }
        ((places as f64 * PART_PREEQUIPEE.v) - 1e-9).ceil() as usize
    }

    /// Points de recharge : un par tranche de 20 places, au-delà de 20 places.
    pub fn bornes(places: usize) -> usize {
        let tranche = TRANCHE_BORNE.v as usize;
        if places <= tranche {
            return 0;
        }
        places / tranche
    }
}

/// Parcs de stationnement extérieurs : ombrage, gestion des eaux pluviales, ombrières.
pub mod urbanisme {
    use super::{val, Val};

    pub const SEUIL_OMBRAGE: Val = val(500.0, "m²", "Code de l'urbanisme, art. L. 111-19-1");
    pub const SEUIL_OMBRIERES: Val = val(1500.0, "m²", "Loi du 10 mars 2023 (APER), art. 40");
    /// Part de la surface du parc à traiter (ombrage, perméabilité, ombrières).
    pub const PART: Val = val(0.5, "", "Code de l'urbanisme, art. L. 111-19-1 ; loi APER, art. 40");
}

/// Longueur horizontale d'une rampe entre deux plateaux de dénivelé `h`, de pente `p`, avec des
/// raccordements circulaires de rayons `rs` (sommet) et `rp` (pied). Calculé :
/// L = h / p + (rs + rp) · p / 2.
pub fn longueur_rampe(h: f64, p: f64, rs: f64, rp: f64) -> f64 {
    h / p + (rs + rp) * p / 2.0
}

/// Flèche sous une corde égale à l'empattement nominal (3,00 m) pour un raccordement de rayon `r`.
/// Calculé : f = r − √(r² − 1,5²).
pub fn fleche(r: f64) -> f64 {
    let c = nf::EMPATTEMENT.v / 2.0;
    r - (r * r - c * c).sqrt()
}

/// Flèche pour une cassure nette de pente `p`. Calculé : f = 1,5 · sin(atan(p) / 2).
pub fn fleche_cassure(p: f64) -> f64 {
    nf::EMPATTEMENT.v / 2.0 * (p.atan() / 2.0).sin()
}

/// Pente maximale d'un plancher incliné le long de l'allée pour des places à `angle` degrés de
/// l'allée (0 = longitudinal) : axe de la place ≤ 5 % et plus grande pente ≤ 7,5 % (NF § 3.4).
/// Calculé : min(7,5 %, 5 % / cos(angle)).
pub fn pente_plancher_max(angle: f64) -> f64 {
    let c = angle.to_radians().cos().abs();
    if c < 1e-9 {
        nf::PENTE_PLACE_MAX.v
    } else {
        (nf::PENTE_PLACE_AXE_MAX.v / c).min(nf::PENTE_PLACE_MAX.v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r2(x: f64) -> f64 {
        (x * 100.0).round() / 100.0
    }

    #[test]
    fn longueurs_de_rampe_du_plan() {
        // Plan de développement, section 3 : valeurs calculées.
        let (nf_s, nf_p) = (nf::RAYON_SOMMET_MIN.v, nf::RAYON_PIED_MIN.v);
        let (nu_s, nu_p) = (neufert::RAYON_SOMMET.v, neufert::RAYON_PIED.v);
        for (h, p, brut, avec_nf, avec_neufert) in [
            (2.75, 0.15, 18.33, 20.21, 20.96),
            (2.90, 0.15, 19.33, 21.21, 21.96),
            (2.90, 0.18, 16.11, 18.36, 19.26),
            (3.00, 0.15, 20.00, 21.88, 22.62),
            (3.30, 0.15, 22.00, 23.88, 24.62),
            (3.30, 0.18, 18.33, 20.58, 21.48),
        ] {
            // Au centimètre près (22,625 s'arrondit à 22,62 ou 22,63 selon la convention).
            let proche = |x: f64, y: f64| (x - y).abs() < 0.006;
            assert!(proche(h / p, brut), "{h} {p}");
            assert!(proche(longueur_rampe(h, p, nf_s, nf_p), avec_nf), "{h} {p}");
            assert!(proche(longueur_rampe(h, p, nu_s, nu_p), avec_neufert), "{h} {p}");
        }
        // Demi-niveaux : 1,45 m à 15 % et 18 %.
        assert_eq!(r2(longueur_rampe(1.45, 0.15, nf_s, nf_p)), 11.54);
        assert_eq!(r2(longueur_rampe(1.45, 0.18, nf_s, nf_p)), 10.31);
        // Rampe enterrée depuis la voirie : 4,00 m de débouché + 3,20 m de dénivelé.
        assert_eq!(r2(nf::LONGUEUR_DEBOUCHE.v + longueur_rampe(3.20, 0.15, nf_s, nf_p)), 27.21);
    }

    #[test]
    fn fleches_et_pentes() {
        assert_eq!((fleche(10.0) * 1000.0).round() / 1000.0, 0.113);
        assert_eq!((fleche(15.0) * 1000.0).round() / 1000.0, 0.075);
        assert_eq!((fleche_cassure(0.18) * 1000.0).round() / 1000.0, 0.133);
        assert_eq!(pente_plancher_max(90.0), 0.075);
        assert_eq!(pente_plancher_max(60.0), 0.075);
        assert_eq!((pente_plancher_max(45.0) * 10000.0).round() / 10000.0, 0.0707);
        assert_eq!(pente_plancher_max(0.0), 0.05);
    }

    #[test]
    fn tableaux_nf() {
        assert_eq!(nf::dims_angle(90.0).unwrap().allee, 5.00);
        let a = nf::dims_angle(67.5).unwrap();
        assert_eq!((a.allee, a.long, a.larg, a.chevron), (4.25, 5.125, 2.25, Some(4.7)));
        assert!(nf::dims_angle(30.0).is_none());
        assert_eq!(nf::allee_double_sens(5.00, 21), 6.00);
        assert_eq!(nf::allee_double_sens(4.50, 21), 5.40);
        assert_eq!(nf::allee_double_sens(3.50, 30), 5.00);
        assert_eq!(nf::allee_double_sens(3.50, 20), 3.50);
        assert_eq!(nf::surlargeur_poteau(0.65), Some(0.0));
        assert_eq!(nf::surlargeur_poteau(0.50), Some(0.10));
        assert_eq!(nf::surlargeur_poteau(0.25), Some(0.15));
        assert_eq!(nf::surlargeur_poteau(0.20), None);
    }

    #[test]
    fn bornes_de_recharge_par_tranche_de_vingt_places() {
        // L. 113-13 : au-delà de 20 places, un point de recharge par tranche de 20 places.
        assert_eq!(irve::bornes(19), 0);
        assert_eq!(irve::bornes(20), 0);
        assert_eq!(irve::bornes(21), 1);
        assert_eq!(irve::bornes(95), 4);
    }

    #[test]
    fn places_pre_equipees_au_dela_de_dix_places() {
        // L. 113-12 : parc de plus de 10 places d'un bâtiment non résidentiel neuf, 20 % des places.
        assert_eq!(irve::pre_equipees(10), 0);
        assert_eq!(irve::pre_equipees(11), 3);
        assert_eq!(irve::pre_equipees(100), 20);
        assert_eq!(irve::pre_equipees(101), 21);
    }

    #[test]
    fn places_adaptees() {
        use accessibilite::places_pmr_erp;
        assert_eq!(places_pmr_erp(0), (0, false));
        assert_eq!(places_pmr_erp(10), (1, false));
        assert_eq!(places_pmr_erp(100), (2, false));
        assert_eq!(places_pmr_erp(101), (3, false));
        assert_eq!(places_pmr_erp(501), (11, true));
        assert_eq!(places_pmr_erp(600), (12, true));
    }
}
