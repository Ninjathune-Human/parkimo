//! Résumé du projet pour une IA : bilan et constats du plan, objets désignés par leur rang
//! (celui qu'attendent les commandes), réglages courants.

use parkimo::projet::Projet;
use parkimo::resume::{resume, PlanVu};
use serde_json::{json, Value};

fn projet(nom: &str) -> Projet {
    let cas: Vec<Value> = serde_json::from_slice(&std::fs::read("tests/fixtures/projets.json").unwrap()).unwrap();
    Projet::lire(cas.into_iter().find(|c| c["nom"] == nom).unwrap()["projet"].clone()).unwrap()
}

fn resume_de(nom: &str) -> Value {
    let p = projet(nom);
    let plan = p.plan().unwrap();
    resume(&p, &PlanVu::from(&plan))
}

#[test]
fn le_resume_designe_chaque_objet_par_son_rang() {
    let r = resume_de("arcs_et_objets");
    let cotes = r["parcelle"]["cotes"].as_array().unwrap();
    assert_eq!(cotes.len(), 4);
    assert_eq!(cotes[1], json!({ "rang": 1, "longueur": 110.0, "recul": 3.0, "fleche": 0.2 }));
    assert_eq!(r["parcelle"]["sommets"][2], json!([160.0, 110.0]));
    // Le premier accès de l'exemple est au tiers du côté 0.
    assert_eq!(r["acces"][0]["cote"], json!(0));
    assert!((r["acces"][0]["t"].as_f64().unwrap() - 1.0 / 3.0).abs() < 1e-9);
    assert_eq!(r["acces"].as_array().unwrap()[1..], [json!({ "rang": 1, "cote": 2, "t": 0.75 }), json!({ "rang": 2, "cote": 1, "t": 0.4 })]);
    let b = r["batiments"].as_array().unwrap();
    assert_eq!(b[0]["forme"], json!("L"));
    assert_eq!(b[1], json!({ "rang": 1, "centre": [60.0, 60.0], "largeur": 20.0, "profondeur": 12.0, "rotation": 15.0, "forme": "rectangle" }));
    assert_eq!(r["zones"]["espace_vert"], json!([{ "rang": 0, "sommets": 4, "emprise": [8.0, 8.0, 30.0, 40.0] }]));
    assert_eq!(r["zones"]["bassin"][0]["emprise"], json!([60.0, 10.0, 80.0, 30.0]));
    assert_eq!(r["zones"]["servitude"].as_array().unwrap().len(), 1);
    assert_eq!(r["traverses"], json!([{ "rang": 0, "de": [40.0, 20.0], "a": [40.0, 60.0] }]));
    assert_eq!((r["reglages"]["slant"].as_f64(), r["reglages"]["stall_w"].as_f64()), (Some(60.0), Some(2.5)));
}

#[test]
fn le_resume_porte_le_bilan_et_les_constats_du_plan() {
    let p = projet("ruisseau");
    let plan = p.plan().unwrap();
    let r = resume(&p, &PlanVu::from(&plan));
    assert_eq!(r["places"]["total"].as_u64(), Some(plan.metrics.stalls as u64));
    assert_eq!(r["places"]["pmr"].as_u64(), Some(plan.metrics.pmr as u64));
    assert_eq!(r["surfaces"]["parcelle_m2"].as_f64(), Some(plan.metrics.parcel_area));
    let constats = r["constats"].as_array().unwrap();
    assert_eq!(constats.len(), plan.constats.len());
    let c = constats.iter().find(|c| c["code"] == "nf.place.largeur").unwrap();
    assert_eq!((c["statut"].as_str(), c["mesure"].as_f64(), c["exige"].as_f64()), (Some("conforme"), Some(2.5), Some(2.3)));
    // Par défaut, seul l'ombrage du parc extérieur est en écart (ombrage et ombrières).
    let ecarts: Vec<&str> = constats.iter().filter(|c| c["statut"] == "ecart").map(|c| c["code"].as_str().unwrap()).collect();
    assert_eq!(ecarts, ["urba.ombrage", "urba.ombrieres"]);
    assert_eq!(r["ecarts"].as_u64(), Some(2));
    assert!(r.get("ouvrage").is_none());
}

#[test]
fn un_silo_resume_son_ouvrage() {
    let r = resume_de("silo");
    let o = &r["ouvrage"];
    assert_eq!(o["niveaux"].as_u64(), Some(3));
    assert!(o["places_par_niveau"].as_u64().unwrap() > 0 && o["dalle"]["longueur"].as_f64().unwrap() > 20.0, "{o}");
    assert!(r["ecarts"].as_u64().unwrap() >= 1);
}

#[test]
fn le_plan_vu_par_l_interface_fait_foi() {
    // L'interface passe son propre plan (bilan, constats) : le résumé le reprend sans recalcul.
    let vu: PlanVu = serde_json::from_value(json!({
        "metrics": { "stalls": 999, "pmr": 20 }, "angle": 12.5, "warnings": ["à voir"],
        "constats": [{ "code": "x", "libelle": "X", "source": "S", "statut": "ecart", "mesure": 1.0, "exige": 2.0, "unite": "m", "detail": "" }],
        "stalls": [], "paved": [],
    }))
    .unwrap();
    let r = resume(&projet("ruisseau"), &vu);
    assert_eq!((r["places"]["total"].as_u64(), r["ecarts"].as_u64(), r["orientation_des_rangees"].as_f64()), (Some(999), Some(1), Some(12.5)));
    assert_eq!(r["avertissements"], json!(["à voir"]));
}

#[test]
fn une_parcelle_en_cours_de_trace_se_resume_sans_plan() {
    let p = Projet::lire(json!({ "site": { "parcel": [[0, 0], [10, 0]] } })).unwrap();
    let r = resume(&p, &PlanVu::default());
    assert_eq!((r["places"]["total"].as_u64(), r["parcelle"]["sommets"].as_array().unwrap().len()), (Some(0), 2));
}

#[test]
fn la_demande_de_resume_calcule_le_plan_quand_l_interface_n_en_passe_pas() {
    use parkimo::resume::{repondre, Demande};
    let p = projet("ruisseau");
    let r = repondre(&serde_json::from_value::<Demande>(json!({ "projet": p })).unwrap());
    assert_eq!(r["places"]["total"].as_u64(), Some(p.plan().unwrap().metrics.stalls as u64));
    let r = repondre(&serde_json::from_value::<Demande>(json!({ "projet": p, "plan": { "metrics": { "stalls": 7 } } })).unwrap());
    assert_eq!(r["places"]["total"].as_u64(), Some(7));
    let r = repondre(&serde_json::from_value::<Demande>(json!({ "projet": {} })).unwrap());
    assert!(r["error"].as_str().is_some());
}
