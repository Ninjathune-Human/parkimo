//! Commandes : chaque action sur le projet est un objet JSON (`op` et ses paramètres) appliqué par
//! le moteur. Une liste de commandes est une transaction : tout passe, ou le projet ne change pas.

use parkimo::commandes::{catalogue, executer, Demande};
use serde_json::{json, Value};

/// Rectangle de 120 x 80 m, un accès au milieu du côté bas, un bâtiment, une zone de chaque
/// nature, une traverse : de quoi adresser tout objet par son rang.
fn projet() -> Value {
    json!({
        "site": {
            "parcel": [[0, 0], [120, 0], [120, 80], [0, 80]], "setbacks": [5, 3, 3, 3],
            "entries": [{ "edge": 0, "t": 0.5 }, { "edge": 2, "t": 0.25 }],
            "buildings": [{ "cx": 95, "cy": 55, "w": 24, "h": 14, "rot": 0, "shape": "rect" }],
            "exclusions": [[[0, 60], [20, 60], [20, 80], [0, 80]]],
            "landscapes": [[[100, 5], [115, 5], [115, 20], [100, 20]]],
            "basins": [[[5, 5], [20, 5], [20, 15], [5, 15]]],
            "traverses": [[[60, 10], [60, 70]]],
        },
        "params": {},
    })
}

fn jouer(projet: Value, commandes: Value, plan: bool) -> parkimo::commandes::Reponse {
    executer(&Demande { projet, commandes: commandes.as_array().unwrap().clone(), plan })
}

/// Projet après des commandes qui doivent passer.
fn apres(commandes: Value) -> Value {
    let r = jouer(projet(), commandes, false);
    assert!(r.ok, "{:?}", r.erreur);
    r.projet
}

/// Message du refus d'une commande.
fn refus(commande: Value) -> String {
    let r = jouer(projet(), json!([commande]), false);
    assert!(!r.ok, "{commande} aurait dû être refusée");
    r.erreur.unwrap().message
}

fn proche(a: &Value, b: f64) -> bool {
    (a.as_f64().unwrap() - b).abs() < 1e-9
}

#[test]
fn regler_change_un_reglage() {
    let p = apres(json!([{ "op": "regler", "cle": "stall_w", "valeur": 2.3 }, { "op": "regler", "cle": "angle", "valeur": 12.5 }]));
    assert_eq!((p["params"]["stall_w"].as_f64(), p["params"]["angle"].as_f64()), (Some(2.3), Some(12.5)));
    let p = apres(json!([{ "op": "regler", "cle": "angle", "valeur": null }, { "op": "regler", "cle": "silo", "valeur": true }]));
    assert_eq!((&p["params"]["angle"], &p["params"]["silo"]), (&Value::Null, &json!(true)));
}

#[test]
fn regler_refuse_une_valeur_hors_du_domaine_du_reglage() {
    for (cle, valeur) in [
        ("largeur", json!(2.5)),      // clé inconnue
        ("stall_w", json!(1.0)),      // sous le minimum
        ("stall_w", json!("large")),  // pas un nombre
        ("silo_niveaux", json!(2.5)), // pas un entier
        ("slant", json!(50)),         // inclinaison hors liste
        ("programme", json!("mixte")),
        ("silo", json!(1)),
        ("angle", json!("nord")),
    ] {
        let m = refus(json!({ "op": "regler", "cle": cle, "valeur": valeur }));
        assert!(m.contains(cle), "{cle} : {m}");
    }
    assert!(refus(json!({ "op": "regler", "cle": "stall_w", "valeur": 1.0 })).contains("1,8"));
}

#[test]
fn une_liste_de_commandes_passe_en_entier_ou_pas_du_tout() {
    let r = jouer(projet(), json!([{ "op": "regler", "cle": "aisle", "valeur": 5.5 }, { "op": "regler", "cle": "aisle", "valeur": 99 }]), false);
    let e = r.erreur.unwrap();
    assert_eq!((r.ok, e.index), (false, 1));
    assert_eq!(r.projet["params"]["aisle"].as_f64(), Some(6.0));
    assert!(r.plan.is_none());
}

#[test]
fn une_commande_inconnue_ou_mal_formee_est_refusee() {
    assert!(refus(json!({ "op": "teleporter" })).contains("teleporter"));
    assert!(refus(json!({ "cle": "aisle" })).contains("op"));
    assert!(refus(json!({ "op": "regler", "cle": "aisle" })).contains("valeur"));
    assert!(refus(json!({ "op": "deplacer_sommet", "sommet": 0, "vers": [1] })).contains("vers"));
}

#[test]
fn ajouter_un_sommet_coupe_le_cote_et_garde_les_acces_en_place() {
    let p = apres(json!([{ "op": "ajouter_sommet", "cote": 0, "t": 0.25 }]));
    let s = &p["site"];
    assert_eq!(s["parcel"], json!([[0.0, 0.0], [30.0, 0.0], [120.0, 0.0], [120.0, 80.0], [0.0, 80.0]]));
    assert_eq!(s["setbacks"], json!([5.0, 5.0, 3.0, 3.0, 3.0]));
    assert_eq!(s["arcs"].as_array().unwrap().len(), 5);
    // L'accès au milieu du côté 0 (x = 60) est désormais au tiers du côté 1 ; celui du côté 2 passe au côté 3.
    assert_eq!(s["entries"][0]["edge"], json!(1));
    assert!(proche(&s["entries"][0]["t"], 1.0 / 3.0));
    assert_eq!(s["entries"][1], json!({ "edge": 3, "t": 0.25 }));
}

#[test]
fn retirer_le_sommet_ajoute_rend_la_parcelle_d_origine() {
    let p = apres(json!([
        { "op": "courber_cote", "cote": 1, "fleche": 0.2 },
        { "op": "ajouter_sommet", "cote": 1, "t": 0.5 },
    ]));
    // Un arc coupé en deux : deux arcs du même cercle, de flèche tan(atan(0,4) / 2) / 2.
    let k = (0.4f64.atan() / 2.0).tan() / 2.0;
    assert!(proche(&p["site"]["arcs"][1], k) && proche(&p["site"]["arcs"][2], k), "{}", p["site"]["arcs"]);
    let r = jouer(p, json!([{ "op": "retirer_sommet", "sommet": 2 }]), false);
    assert!(r.ok, "{:?}", r.erreur);
    let s = &r.projet["site"];
    assert_eq!(s["parcel"], json!([[0.0, 0.0], [120.0, 0.0], [120.0, 80.0], [0.0, 80.0]]));
    assert_eq!(s["setbacks"], json!([5.0, 3.0, 3.0, 3.0]));
    assert_eq!(s["arcs"], json!([0.0, 0.2, 0.0, 0.0]));
    assert_eq!(s["entries"][1]["edge"], json!(2));
    assert!(proche(&s["entries"][1]["t"], 0.25));
}

#[test]
fn retirer_le_premier_sommet_reporte_les_acces_sur_le_cote_reuni() {
    // Sommet 0 retiré : les côtés 3 (80 m) et 0 (120 m) n'en font plus qu'un, de rang 2.
    let p = apres(json!([{ "op": "retirer_sommet", "sommet": 0 }, ]));
    let s = &p["site"];
    assert_eq!(s["parcel"], json!([[120.0, 0.0], [120.0, 80.0], [0.0, 80.0]]));
    assert_eq!(s["entries"][0]["edge"], json!(2));
    assert!(proche(&s["entries"][0]["t"], (80.0 + 0.5 * 120.0) / 200.0));
    assert_eq!(s["entries"][1], json!({ "edge": 1, "t": 0.25 }));
    // Trois sommets au moins.
    let r = jouer(p, json!([{ "op": "retirer_sommet", "sommet": 0 }]), false);
    assert!(!r.ok);
}

#[test]
fn deplacer_un_sommet_et_regler_les_reculs() {
    let p = apres(json!([
        { "op": "deplacer_sommet", "sommet": 2, "vers": [130, 90.5] },
        { "op": "regler_recul", "cote": 1, "valeur": 8 },
    ]));
    assert_eq!(p["site"]["parcel"][2], json!([130.0, 90.5]));
    assert_eq!(p["site"]["setbacks"], json!([5.0, 8.0, 3.0, 3.0]));
    let p = apres(json!([{ "op": "regler_recul", "valeur": 4 }]));
    assert_eq!(p["site"]["setbacks"], json!([4.0, 4.0, 4.0, 4.0]));
    assert!(refus(json!({ "op": "deplacer_sommet", "sommet": 4, "vers": [0, 0] })).contains("sommet"));
    assert!(refus(json!({ "op": "regler_recul", "cote": 0, "valeur": -1 })).contains("recul"));
    assert!(refus(json!({ "op": "courber_cote", "cote": 0, "fleche": 0.7 })).contains("0,5"));
}

#[test]
fn les_acces_s_ajoutent_se_deplacent_et_se_retirent() {
    let p = apres(json!([
        { "op": "ajouter_acces", "cote": 1, "t": 0.3 },
        { "op": "deplacer_acces", "acces": 0, "cote": 3, "t": 0.8 },
        { "op": "retirer_acces", "acces": 1 },
    ]));
    assert_eq!(p["site"]["entries"], json!([{ "edge": 3, "t": 0.8 }, { "edge": 1, "t": 0.3 }]));
    assert!(refus(json!({ "op": "ajouter_acces", "cote": 4, "t": 0.5 })).contains("côté"));
    assert!(refus(json!({ "op": "ajouter_acces", "cote": 0, "t": 1.5 })).contains("t"));
    assert!(refus(json!({ "op": "retirer_acces", "acces": 2 })).contains("accès"));
}

#[test]
fn les_batiments_s_ajoutent_se_modifient_et_se_retirent() {
    let p = apres(json!([
        { "op": "ajouter_batiment", "centre": [30, 40], "largeur": 36, "profondeur": 30, "forme": "L" },
        { "op": "modifier_batiment", "batiment": 0, "largeur": 20, "rotation": 15 },
    ]));
    let b = &p["site"]["buildings"];
    assert_eq!(b[1], json!({ "cx": 30.0, "cy": 40.0, "w": 36.0, "h": 30.0, "rot": 0.0, "shape": "L" }));
    assert_eq!(b[0], json!({ "cx": 95, "cy": 55, "w": 20.0, "h": 14, "rot": 15.0, "shape": "rect" }));
    let p = apres(json!([{ "op": "retirer_batiment", "batiment": 0 }]));
    assert_eq!(p["site"]["buildings"], json!([]));
    assert!(refus(json!({ "op": "ajouter_batiment", "centre": [0, 0], "largeur": 0, "profondeur": 10 })).contains("largeur"));
    assert!(refus(json!({ "op": "ajouter_batiment", "centre": [0, 0], "largeur": 10, "profondeur": 10, "forme": "rond" })).contains("forme"));
    assert!(refus(json!({ "op": "modifier_batiment", "batiment": 3, "largeur": 10 })).contains("bâtiment"));
}

#[test]
fn les_zones_et_les_traverses_s_ajoutent_et_se_retirent() {
    let carre = json!([[40, 30], [50, 30], [50, 40], [40, 40]]);
    let p = apres(json!([
        { "op": "ajouter_zone", "nature": "espace_vert", "points": carre },
        { "op": "ajouter_zone", "nature": "bassin", "points": carre },
        { "op": "retirer_zone", "nature": "servitude", "zone": 0 },
        { "op": "ajouter_traverse", "de": [20, 10], "a": [20, 70] },
        { "op": "retirer_traverse", "traverse": 0 },
    ]));
    let s = &p["site"];
    assert_eq!((s["landscapes"].as_array().unwrap().len(), s["basins"].as_array().unwrap().len(), s["exclusions"].as_array().unwrap().len()), (2, 2, 0));
    assert_eq!(s["traverses"], json!([[[20.0, 10.0], [20.0, 70.0]]]));
    assert!(refus(json!({ "op": "ajouter_zone", "nature": "lac", "points": carre })).contains("nature"));
    assert!(refus(json!({ "op": "ajouter_zone", "nature": "bassin", "points": [[0, 0], [1, 1]] })).contains("trois"));
    assert!(refus(json!({ "op": "retirer_zone", "nature": "bassin", "zone": 1 })).contains("bassin"));
}

#[test]
fn tracer_une_parcelle_remplace_le_contour_et_ses_acces() {
    let p = apres(json!([{ "op": "tracer_parcelle", "points": [[0, 0], [50, 0], [25, 40]], "recul": 2 }]));
    let s = &p["site"];
    assert_eq!(s["parcel"], json!([[0.0, 0.0], [50.0, 0.0], [25.0, 40.0]]));
    assert_eq!((&s["setbacks"], &s["arcs"], &s["entries"]), (&json!([2.0, 2.0, 2.0]), &json!([0.0, 0.0, 0.0]), &json!([])));
    assert_eq!(s["buildings"].as_array().unwrap().len(), 1);
    assert!(refus(json!({ "op": "tracer_parcelle", "points": [[0, 0], [50, 0]] })).contains("trois"));
}

#[test]
fn la_dalle_du_silo_se_regle_et_se_libere() {
    let p = apres(json!([{ "op": "regler_dalle", "centre": [60, 40], "longueur": 90, "largeur": 32, "rotation": 10 }]));
    assert_eq!(p["site"]["slab"], json!({ "cx": 60.0, "cy": 40.0, "w": 90.0, "h": 32.0, "rot": 10.0, "fixe": [0, 0] }));
    let r = jouer(p, json!([{ "op": "liberer_dalle" }]), false);
    assert!(r.ok && r.projet["site"].get("slab").is_none());
}

#[test]
fn le_plan_et_ses_constats_reviennent_avec_la_reponse() {
    let avant = jouer(projet(), json!([]), true).plan.unwrap();
    let r = jouer(projet(), json!([{ "op": "retirer_batiment", "batiment": 0 }, { "op": "regler", "cle": "aisle", "valeur": 5.5 }]), true);
    let plan = r.plan.unwrap();
    assert!(plan.stalls.len() > avant.stalls.len(), "{} places puis {}", avant.stalls.len(), plan.stalls.len());
    let c = plan.constats.iter().find(|c| c.code == "nf.allee.double_sens").unwrap();
    assert_eq!(c.statut, parkimo::conformite::Statut::Ecart);
}

#[test]
fn le_catalogue_decrit_chaque_commande_par_un_exemple_qui_passe() {
    let cat = catalogue();
    let ops: Vec<&str> = cat.iter().map(|c| c["op"].as_str().unwrap()).collect();
    assert_eq!(ops.len(), 19, "{ops:?}");
    for c in &cat {
        assert!(c["description"].as_str().is_some_and(|d| d.len() > 20), "{}", c["op"]);
        assert!(c["parametres"].is_object(), "{}", c["op"]);
        assert_eq!(c["exemple"]["op"], c["op"]);
        let r = jouer(projet(), json!([c["exemple"]]), false);
        assert!(r.ok, "{} : {:?}", c["op"], r.erreur);
        // Chaque paramètre de l'exemple est décrit.
        for k in c["exemple"].as_object().unwrap().keys().filter(|k| *k != "op") {
            assert!(c["parametres"].get(k).is_some(), "{} : paramètre {k} non décrit", c["op"]);
        }
    }
    // Les réglages admis par `regler` y figurent avec leurs bornes.
    let regler = cat.iter().find(|c| c["op"] == "regler").unwrap();
    assert!(regler["reglages"].as_array().unwrap().iter().any(|r| r["cle"] == "stall_w" && r["min"] == json!(1.8)));
}
