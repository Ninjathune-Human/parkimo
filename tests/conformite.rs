//! Constats de conformité rendus par le moteur : chaque règle du référentiel (`src/normes.rs`)
//! donne un constat conforme, en écart ou d'information, avec la valeur mesurée et la valeur exigée.

use parkimo::conformite::{Constat, Statut};
use serde_json::{json, Value};

/// Plan d'un scénario de référence, sans le contrôle des virages (recherche de giration, lente),
/// après les réglages de `regle`.
fn plan_de(mut v: Value, regle: impl FnOnce(&mut Value)) -> parkimo::model::Layout {
    v["params"]["turns"] = json!(false);
    regle(&mut v);
    parkimo::solve(&serde_json::from_value(v).unwrap())
}

fn plan(nom: &str, regle: impl FnOnce(&mut Value)) -> parkimo::model::Layout {
    let v = serde_json::from_slice(&std::fs::read(format!("scenarios/{nom}.json")).unwrap()).unwrap();
    plan_de(v, regle)
}

fn constats(nom: &str, regle: impl FnOnce(&mut Value)) -> Vec<Constat> {
    plan(nom, regle).constats
}

fn codes(cs: &[Constat]) -> Vec<&str> {
    cs.iter().map(|c| c.code.as_str()).collect()
}

fn trouve<'a>(cs: &'a [Constat], code: &str) -> &'a Constat {
    cs.iter().find(|c| c.code == code).unwrap_or_else(|| panic!("constat {code} absent parmi {:?}", codes(cs)))
}

#[test]
fn les_reglages_par_defaut_ne_donnent_aucun_ecart() {
    let cs = constats("rect", |_| {});
    for code in [
        "nf.place.largeur",
        "nf.demi_travee",
        "nf.allee.double_sens",
        "acc.pmr.nombre",
        "acc.pmr.largeur",
        "acc.pmr.longueur",
        "irve.preequipement",
        "irve.bornes",
    ] {
        assert_eq!(trouve(&cs, code).statut, Statut::Conforme, "{code}");
    }
    // Seul l'ombrage du parc extérieur reste à traiter : il dépend des arbres et des ombrières choisis.
    assert!(cs.iter().all(|c| c.statut != Statut::Ecart || c.code.starts_with("urba.")), "{:?}", cs);
    // Chaque constat cite sa source.
    assert!(cs.iter().all(|c| !c.source.is_empty() && !c.libelle.is_empty()));
}

#[test]
fn une_place_de_2_20_m_est_en_ecart_a_90_degres() {
    let cs = constats("rect", |v| v["params"]["stall_w"] = json!(2.2));
    let c = trouve(&cs, "nf.place.largeur");
    assert_eq!((c.statut, c.mesure, c.exige), (Statut::Ecart, Some(2.2), Some(2.3)));
    assert_eq!(c.source, "NF P 91-100, tab. 2");
}

#[test]
fn la_demi_travee_additionne_la_place_et_l_allee() {
    // Note 2 du tableau 2 : une place de 4,50 m passe avec une allée de 6,00 m (10,50 pour 10,00)...
    let cs = constats("rect", |v| v["params"]["stall_d"] = json!(4.5));
    let c = trouve(&cs, "nf.demi_travee");
    assert_eq!((c.statut, c.mesure, c.exige), (Statut::Conforme, Some(10.5), Some(10.0)));
    // ... mais pas avec une allée de 5,00 m (9,50 pour 10,00).
    let cs = constats("rect", |v| {
        v["params"]["stall_d"] = json!(4.5);
        v["params"]["aisle"] = json!(5.0);
    });
    let c = trouve(&cs, "nf.demi_travee");
    assert_eq!((c.statut, c.mesure, c.exige), (Statut::Ecart, Some(9.5), Some(10.0)));
}

#[test]
fn une_allee_a_double_sens_de_5_50_m_est_en_ecart_au_dela_de_20_places() {
    let cs = constats("rect", |v| v["params"]["aisle"] = json!(5.5));
    let c = trouve(&cs, "nf.allee.double_sens");
    assert_eq!((c.statut, c.mesure, c.exige), (Statut::Ecart, Some(5.5), Some(6.0)));
}

#[test]
fn jusqu_a_20_places_l_allee_a_double_sens_garde_la_largeur_du_tableau() {
    let petit = json!({ "parcel": [[0, 0], [30, 0], [30, 22], [0, 22]], "entries": [{ "edge": 0, "t": 0.5 }], "params": { "aisle": 5.5 } });
    let lay = plan_de(petit, |_| {});
    assert!((1..=20).contains(&lay.metrics.stalls), "{} places", lay.metrics.stalls);
    let c = trouve(&lay.constats, "nf.allee.double_sens");
    assert_eq!((c.statut, c.exige), (Statut::Conforme, Some(5.0)));
}

#[test]
fn des_places_a_30_degres_sont_hors_tableau() {
    let cs = constats("rect", |v| v["params"]["slant"] = json!(30));
    assert_eq!(trouve(&cs, "nf.place.largeur").statut, Statut::Info);
    assert!(!codes(&cs).contains(&"nf.demi_travee"));
}

#[test]
fn sans_place_adaptee_le_nombre_est_en_ecart() {
    let lay = plan("rect", |v| v["params"]["pmr_ratio"] = json!(0));
    let c = trouve(&lay.constats, "acc.pmr.nombre");
    let exige = parkimo::normes::accessibilite::places_pmr_erp(lay.metrics.stalls).0 as f64;
    assert_eq!((c.statut, c.mesure, c.exige), (Statut::Ecart, Some(0.0), Some(exige)));
    assert_eq!(c.source, "Arrêté du 20/04/2017, art. 3");
}

#[test]
fn la_largeur_des_places_adaptees_est_mesuree_sur_le_plan() {
    // Le moteur remplace quatre places par des places adaptées : avec des places de 2,50 m, un
    // réglage à 3,00 m donne tout de même trois places de 3,33 m. Le constat lit le plan, pas le réglage.
    let cs = constats("rect", |v| v["params"]["pmr_w"] = json!(3.0));
    let c = trouve(&cs, "acc.pmr.largeur");
    assert_eq!((c.statut, c.mesure, c.exige), (Statut::Conforme, Some(3.333), Some(3.3)));
    // Avec des places de 2,30 m, les mêmes trois places n'ont plus que 3,07 m.
    let cs = constats("rect", |v| {
        v["params"]["pmr_w"] = json!(3.0);
        v["params"]["stall_w"] = json!(2.3);
    });
    let c = trouve(&cs, "acc.pmr.largeur");
    assert_eq!((c.statut, c.exige), (Statut::Ecart, Some(3.3)));
    // 9,20 m en trois places, au millimètre des sommets près.
    assert!((c.mesure.unwrap() - 3.067).abs() < 0.002, "{:?}", c.mesure);
}

#[test]
fn au_dela_de_500_places_l_arrete_municipal_est_signale() {
    let grand = json!({ "parcel": [[0, 0], [220, 0], [220, 140], [0, 140]], "entries": [{ "edge": 0, "t": 0.5 }] });
    let lay = plan_de(grand, |_| {});
    assert!(lay.metrics.stalls > 500, "{} places", lay.metrics.stalls);
    assert_eq!(trouve(&lay.constats, "acc.pmr.arrete").statut, Statut::Info);
    assert!(!codes(&plan("etroit", |_| {}).constats).contains(&"acc.pmr.arrete"));
}

#[test]
fn sans_pre_equipement_la_recharge_est_en_ecart() {
    let lay = plan("rect", |v| v["params"]["irve_ratio"] = json!(0));
    let n = lay.metrics.stalls;
    let c = trouve(&lay.constats, "irve.preequipement");
    assert_eq!((c.statut, c.mesure, c.exige), (Statut::Ecart, Some(0.0), Some(parkimo::normes::irve::pre_equipees(n) as f64)));
    let c = trouve(&lay.constats, "irve.bornes");
    assert_eq!((c.statut, c.exige), (Statut::Ecart, Some(parkimo::normes::irve::bornes(n) as f64)));
}

/// Plan du rectangle de référence après réglage des plantations, du houppier et des ombrières.
fn ombrage(ilot: u32, houppier: f64, ombrieres: bool) -> parkimo::model::Layout {
    plan("rect", |v| {
        v["params"]["planter_every"] = json!(ilot);
        v["params"]["canopy"] = json!(houppier);
        v["params"]["ombrieres"] = json!(ombrieres);
    })
}

#[test]
fn l_ombrage_est_mesure_et_en_ecart_sous_la_moitie_du_parc() {
    let lay = ombrage(8, 8.0, false);
    let o = &lay.ombrage;
    assert!(o.surface > 1500.0 && o.total < 0.5 * o.surface);
    let c = trouve(&lay.constats, "urba.ombrage");
    assert_eq!((c.statut, c.exige, c.unite.as_str()), (Statut::Ecart, Some(50.0), "%"));
    assert!((c.mesure.unwrap() - 100.0 * o.total / o.surface).abs() < 0.06, "{:?}", c);
    assert!(c.detail.contains("houppiers de 8 m") && c.source.contains("L. 111-19-1"), "{:?}", c);
    // Sans ombrière, l'obligation de la loi APER n'est pas remplie non plus.
    let c = trouve(&lay.constats, "urba.ombrieres");
    assert_eq!((c.statut, c.mesure, c.exige), (Statut::Ecart, Some(0.0), Some(50.0)));
    // La gestion des eaux pluviales reste à justifier.
    assert_eq!(trouve(&lay.constats, "urba.eaux_pluviales").statut, Statut::Info);
}

#[test]
fn des_arbres_ombrageant_la_moitie_du_parc_satisfont_l_ombrage_et_exonerent_des_ombrieres() {
    let lay = ombrage(3, 8.0, false);
    assert!(lay.ombrage.arbres >= 0.5 * lay.ombrage.surface, "{:?}", lay.ombrage);
    assert_eq!(trouve(&lay.constats, "urba.ombrage").statut, Statut::Conforme);
    let c = trouve(&lay.constats, "urba.ombrieres");
    assert_eq!(c.statut, Statut::Conforme);
    assert!(c.detail.contains("Exonération"), "{:?}", c);
}

#[test]
fn un_arbre_pour_trois_places_satisfait_l_ombrage_meme_avec_de_petits_houppiers() {
    let lay = ombrage(2, 3.0, false);
    assert!(lay.ombrage.total < 0.5 * lay.ombrage.surface && 3 * lay.trees.len() >= lay.stalls.len());
    let c = trouve(&lay.constats, "urba.ombrage");
    assert_eq!(c.statut, Statut::Conforme);
    assert!(c.detail.contains("un arbre pour trois") && c.source.contains("R. 111-25-8"), "{:?}", c);
    // La loi APER, elle, demande une ombre effective : pas d'exonération.
    assert_eq!(trouve(&lay.constats, "urba.ombrieres").statut, Statut::Ecart);
}

#[test]
fn des_ombrieres_completees_par_des_arbres_forment_un_procede_mixte() {
    // Ombrières sur les rangées (plus de 17,5 % du parc) : il manque de l'ombre avec de petits arbres.
    let lay = ombrage(8, 8.0, true);
    let o = &lay.ombrage;
    assert!(o.ombrieres > 0.175 * o.surface && o.ombrieres < 0.5 * o.surface && o.total < 0.5 * o.surface);
    assert_eq!(trouve(&lay.constats, "urba.ombrieres").statut, Statut::Ecart);
    assert_eq!(trouve(&lay.constats, "urba.ombrage").statut, Statut::Ecart);
    // Avec des houppiers plus larges, arbres et ombrières ombragent la moitié du parc.
    let lay = ombrage(8, 12.0, true);
    assert!(lay.ombrage.total >= 0.5 * lay.ombrage.surface);
    let c = trouve(&lay.constats, "urba.ombrieres");
    assert_eq!((c.statut, c.exige), (Statut::Conforme, Some(17.5)));
    assert!(c.detail.contains("mixte"), "{:?}", c);
    assert_eq!(trouve(&lay.constats, "urba.ombrage").statut, Statut::Conforme);
}

#[test]
fn un_petit_parc_n_est_pas_soumis_a_l_ombrage() {
    let petit = json!({ "parcel": [[0, 0], [22, 0], [22, 18], [0, 18]], "entries": [{ "edge": 0, "t": 0.5 }] });
    let lay = plan_de(petit, |_| {});
    assert!(lay.ombrage.surface < 500.0, "{} m²", lay.ombrage.surface);
    assert!(!codes(&lay.constats).iter().any(|c| c.starts_with("urba.")));
}

#[test]
fn un_plan_vide_ou_un_calcul_rapide_ne_rend_aucun_constat() {
    let vide = json!({ "parcel": [[0, 0], [1, 0]] });
    assert!(plan_de(vide, |_| {}).constats.is_empty());
    let rapide = constats("rect", |v| {
        v["params"]["quick"] = json!(true);
        v["params"]["angle_hint"] = json!(0);
    });
    assert!(rapide.is_empty());
}

#[test]
fn le_programme_privatif_ne_rend_pas_les_regles_du_parc_public() {
    let cs = constats("rect", |v| {
        v["params"]["programme"] = json!("privatif");
        v["params"]["stall_w"] = json!(2.2);
    });
    assert!(!codes(&cs).iter().any(|c| c.starts_with("nf.") || c.starts_with("acc.") || c.starts_with("ps.")), "{:?}", codes(&cs));
    assert_eq!(trouve(&cs, "programme").statut, Statut::Info);
    assert!(codes(&cs).contains(&"irve.preequipement"));
}

#[test]
fn les_constats_sortent_en_json_avec_un_statut_en_minuscules() {
    let lay = plan("rect", |v| v["params"]["stall_w"] = json!(2.2));
    let v = serde_json::to_value(&lay).unwrap();
    let c = v["constats"].as_array().unwrap().iter().find(|c| c["code"] == "nf.place.largeur").unwrap();
    assert_eq!((c["statut"].as_str(), c["mesure"].as_f64(), c["unite"].as_str()), (Some("ecart"), Some(2.2), Some("m")));
}

// Parc en ouvrage.

fn silo(regle: impl FnOnce(&mut Value)) -> parkimo::model::Layout {
    plan("silo", |v| {
        v["ouvrage"]["niveaux"] = json!(3);
        regle(v);
    })
}

#[test]
fn un_silo_sans_place_adaptee_posee_est_en_ecart() {
    let lay = silo(|_| {});
    assert!(lay.ouvrage.is_some() && lay.metrics.stalls > 100);
    let c = trouve(&lay.constats, "acc.pmr.nombre");
    assert_eq!((c.statut, c.mesure), (Statut::Ecart, Some(0.0)));
    // Les règles des parcs extérieurs ne concernent pas un silo.
    assert!(!codes(&lay.constats).iter().any(|c| c.starts_with("urba.")));
}

#[test]
fn un_poteau_a_0_40_m_de_l_allee_demande_une_place_plus_large() {
    // Tableau 3 : retrait de 0,40 m, surlargeur de 0,10 m, soit 2,40 m à 90°.
    let cs = silo(|v| {
        v["ouvrage"]["retrait"] = json!(0.40);
        v["params"]["stall_w"] = json!(2.3);
    })
    .constats;
    let c = trouve(&cs, "nf.poteau.retrait");
    assert_eq!((c.statut, c.mesure, c.exige), (Statut::Ecart, Some(2.3), Some(2.4)));
    assert_eq!(c.source, "NF P 91-100, tab. 3");
    // Au retrait nominal de 0,65 m, aucune surlargeur.
    let cs = silo(|v| v["params"]["stall_w"] = json!(2.3)).constats;
    let c = trouve(&cs, "nf.poteau.retrait");
    assert_eq!((c.statut, c.exige), (Statut::Conforme, Some(2.3)));
}

#[test]
fn un_poteau_a_moins_de_0_25_m_de_l_allee_n_est_pas_couvert() {
    let cs = silo(|v| v["ouvrage"]["retrait"] = json!(0.20)).constats;
    let c = trouve(&cs, "nf.poteau.retrait");
    assert_eq!((c.statut, c.exige), (Statut::Ecart, None));
}

#[test]
fn une_hauteur_d_etage_de_2_40_m_est_en_ecart() {
    let lay = silo(|v| v["ouvrage"]["hauteur_etage"] = json!(2.4));
    let c = trouve(&lay.constats, "nf.hauteur");
    let exige = lay.ouvrage.as_ref().unwrap().plancher.hauteur_min;
    assert_eq!((c.statut, c.mesure, c.exige), (Statut::Ecart, Some(2.4), Some(exige)));
    assert_eq!(trouve(&silo(|_| {}).constats, "nf.hauteur").statut, Statut::Conforme);
}

#[test]
fn l_evacuation_et_le_compartimentage_d_un_silo_sont_controles() {
    let lay = silo(|_| {});
    let o = lay.ouvrage.as_ref().unwrap();
    let ev = o.evacuation.as_ref().unwrap();
    let c = trouve(&lay.constats, "ps.evacuation");
    assert_eq!((c.statut, c.mesure, c.exige), (Statut::Conforme, Some(ev.distance_max), Some(ev.limite_deux)));
    // Niveau de plus de 3 000 m² : recoupement à prévoir (PS 12).
    assert!(o.surface > 3000.0, "{} m²", o.surface);
    let c = trouve(&lay.constats, "ps.compartiment");
    assert_eq!((c.statut, c.mesure, c.exige), (Statut::Info, Some(o.surface), Some(3000.0)));
}
