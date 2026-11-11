//! Le projet tel que l'interface l'enregistre (`site`, `params`) et sa conversion en entrée du
//! moteur. La référence est l'interface elle-même : `tests/fixtures/projets.json` porte, pour
//! chaque projet, l'entrée que la page en tire (`engineInput`) et le nombre de places du plan
//! (produit par le script de test de la page dans Chrome).

use parkimo::projet::Projet;
use serde_json::Value;

fn cas() -> Vec<Value> {
    serde_json::from_slice(&std::fs::read("tests/fixtures/projets.json").unwrap()).unwrap()
}

/// Chemins où deux JSON diffèrent : nombres à 1e-9 près, le reste à l'identique.
fn ecarts(a: &Value, b: &Value, chemin: &str, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            if (x.as_f64().unwrap() - y.as_f64().unwrap()).abs() > 1e-9 {
                out.push(format!("{chemin} : {x} au lieu de {y}"));
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                ecarts(p, q, &format!("{chemin}[{i}]"), out);
            }
        }
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys().filter(|k| !x.contains_key(*k))) {
                ecarts(x.get(k).unwrap_or(&Value::Null), y.get(k).unwrap_or(&Value::Null), &format!("{chemin}.{k}"), out);
            }
        }
        _ if a == b => {}
        _ => out.push(format!("{chemin} : {a} au lieu de {b}")),
    }
}

#[test]
fn l_entree_du_moteur_est_celle_que_l_interface_tire_du_projet() {
    for c in cas() {
        let projet = Projet::lire(c["projet"].clone()).unwrap();
        // Hors projet : l'orientation du dernier calcul, qui ne sert qu'aux calculs rapides.
        let mut attendu = c["entree"].clone();
        attendu["params"]["angle_hint"] = Value::Null;
        let mut out = Vec::new();
        ecarts(&projet.entree_moteur(), &attendu, "", &mut out);
        assert!(out.is_empty(), "{} : {} écarts, dont {:?}", c["nom"], out.len(), &out[..out.len().min(5)]);
    }
}

#[test]
fn le_plan_tire_du_projet_a_le_nombre_de_places_de_l_interface() {
    for c in cas() {
        let plan = Projet::lire(c["projet"].clone()).unwrap().plan().unwrap();
        assert_eq!(plan.stalls.len() as u64, c["places"].as_u64().unwrap(), "{}", c["nom"]);
    }
}

#[test]
fn un_projet_ressort_tel_qu_il_est_entre() {
    for c in cas() {
        let projet = Projet::lire(c["projet"].clone()).unwrap();
        let mut out = Vec::new();
        ecarts(&serde_json::to_value(&projet).unwrap(), &c["projet"], "", &mut out);
        assert!(out.is_empty(), "{} : {:?}", c["nom"], out);
    }
}

#[test]
fn les_reglages_absents_prennent_les_valeurs_par_defaut_de_l_interface() {
    // Le projet « ruisseau » des références porte les réglages par défaut de l'interface.
    let reference = &cas()[0]["projet"];
    let nu = Projet::lire(serde_json::json!({ "site": reference["site"], "params": {} })).unwrap();
    let mut out = Vec::new();
    ecarts(&serde_json::to_value(&nu).unwrap()["params"], &reference["params"], "params", &mut out);
    assert!(out.is_empty(), "{:?}", out);
}

#[test]
fn un_projet_sans_parcelle_est_refuse() {
    for v in [serde_json::json!({}), serde_json::json!({ "site": {}, "params": {} }), serde_json::json!({ "site": { "parcel": [[0, 0], "x"] } })] {
        assert!(Projet::lire(v.clone()).is_err(), "{v}");
    }
    // Une parcelle en cours de tracé (moins de trois sommets) est un projet valide, sans plan.
    let debut = Projet::lire(serde_json::json!({ "site": { "parcel": [[0, 0], [10, 0]] } })).unwrap();
    assert!(debut.plan().unwrap().stalls.is_empty());
}
