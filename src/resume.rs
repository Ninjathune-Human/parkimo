//! Résumé du projet pour une IA : le bilan et les constats du plan, les objets désignés par le
//! rang qu'attendent les commandes, les réglages courants. Assez court pour être relu à chaque
//! action, assez complet pour agir sans voir le plan.

use crate::conformite::{Constat, Statut};
use crate::model::{Layout, Metrics};
use crate::projet::{Projet, REGLAGES};
use serde::Deserialize;
use serde_json::{json, Map, Value};

/// Ce que le résumé lit d'un plan : celui du moteur, ou celui que l'interface affiche (elle
/// passe son plan tel quel, les champs absents ou inconnus sont ignorés).
#[derive(Deserialize, Default, Debug)]
#[serde(default)]
pub struct PlanVu {
    pub metrics: Metrics,
    pub constats: Vec<Constat>,
    pub warnings: Vec<String>,
    pub angle: f64,
    pub ouvrage: Option<Value>,
}

impl From<&Layout> for PlanVu {
    fn from(lay: &Layout) -> PlanVu {
        PlanVu {
            metrics: lay.metrics.clone(),
            constats: lay.constats.clone(),
            warnings: lay.warnings.clone(),
            angle: lay.angle,
            ouvrage: lay.ouvrage.as_ref().and_then(|o| serde_json::to_value(o).ok()),
        }
    }
}

fn mm(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

fn num(v: Option<&Value>) -> f64 {
    v.and_then(Value::as_f64).unwrap_or(0.0)
}

fn liste<'a>(site: &'a Value, cle: &str) -> &'a [Value] {
    site.get(cle).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn point(v: &Value) -> [f64; 2] {
    [mm(num(v.get(0))), mm(num(v.get(1)))]
}

/// Zones d'une nature : rang, nombre de sommets, emprise [x min, y min, x max, y max].
fn zones(site: &Value, cle: &str) -> Vec<Value> {
    liste(site, cle)
        .iter()
        .enumerate()
        .map(|(rang, z)| {
            let pts: Vec<[f64; 2]> = z.as_array().map(|a| a.iter().map(point).collect()).unwrap_or_default();
            let borne = |k: usize, f: fn(f64, f64) -> f64, depart: f64| pts.iter().fold(depart, |m, p| f(m, p[k]));
            let emprise = if pts.is_empty() {
                [0.0; 4]
            } else {
                [borne(0, f64::min, f64::MAX), borne(1, f64::min, f64::MAX), borne(0, f64::max, f64::MIN), borne(1, f64::max, f64::MIN)]
            };
            json!({ "rang": rang, "sommets": pts.len(), "emprise": emprise })
        })
        .collect()
}

pub fn resume(projet: &Projet, plan: &PlanVu) -> Value {
    let site = projet.site();
    let m = &plan.metrics;
    let sommets: Vec<[f64; 2]> = liste(site, "parcel").iter().map(point).collect();
    let n = sommets.len();
    let cotes: Vec<Value> = (0..n)
        .map(|i| {
            json!({
                "rang": i,
                "longueur": mm(projet.limite(i).l),
                "recul": num(liste(site, "setbacks").get(i)),
                "fleche": num(liste(site, "arcs").get(i)),
            })
        })
        .collect();
    let acces: Vec<Value> = liste(site, "entries")
        .iter()
        .enumerate()
        .map(|(rang, e)| json!({ "rang": rang, "cote": e["edge"].as_u64().unwrap_or(0), "t": num(e.get("t")) }))
        .collect();
    let batiments: Vec<Value> = liste(site, "buildings")
        .iter()
        .enumerate()
        .map(|(rang, b)| {
            json!({
                "rang": rang,
                "centre": [num(b.get("cx")), num(b.get("cy"))],
                "largeur": num(b.get("w")),
                "profondeur": num(b.get("h")),
                "rotation": num(b.get("rot")),
                "forme": if b.get("shape").and_then(Value::as_str) == Some("L") { "L" } else { "rectangle" },
            })
        })
        .collect();
    let traverses: Vec<Value> = liste(site, "traverses")
        .iter()
        .enumerate()
        .map(|(rang, t)| json!({ "rang": rang, "de": point(&t[0]), "a": point(&t[1]) }))
        .collect();
    let reglages: Map<String, Value> = REGLAGES.iter().map(|r| (r.cle.to_string(), projet.params().get(r.cle).cloned().unwrap_or(Value::Null))).collect();
    let ecarts = plan.constats.iter().filter(|c| c.statut == Statut::Ecart).count();
    let mut r = json!({
        "places": {
            "total": m.stalls, "pmr": m.pmr, "pmr_requises": m.pmr_required,
            "irve_pre_equipees": m.irve, "irve_requises": m.irve_required, "bornes": m.irve_equipped,
            "en_bordure": m.perimeter_stalls, "au_coeur": m.interior_stalls,
        },
        "surfaces": {
            "parcelle_m2": m.parcel_area, "enrobe_m2": m.paved_area, "espaces_verts_m2": m.green_area,
            "enrobe_par_place_m2": m.m2_paved_per_stall,
        },
        "orientation_des_rangees": plan.angle,
        "ecarts": ecarts,
        "constats": plan.constats,
        "avertissements": plan.warnings,
        "parcelle": { "sommets": sommets, "cotes": cotes },
        "acces": acces,
        "batiments": batiments,
        "zones": {
            "servitude": zones(site, "exclusions"),
            "espace_vert": zones(site, "landscapes"),
            "bassin": zones(site, "basins"),
        },
        "traverses": traverses,
        "reglages": reglages,
    });
    if let Some(o) = &plan.ouvrage {
        let ev = &o["evacuation"];
        r["ouvrage"] = json!({
            "niveaux": o["niveaux"],
            "places_par_niveau": o["places_niveau"],
            "dalle": { "longueur": o["longueur"], "largeur": o["largeur"], "surface_m2": o["surface"], "direction_des_allees": o["angle"] },
            "rampes": o["implantation"],
            "hauteur_etage": o["hauteur_etage"],
            "distance_max_a_un_escalier": ev.get("distance_max").cloned().unwrap_or(Value::Null),
            "places_trop_loin_d_un_escalier": ev.get("hors").cloned().unwrap_or(Value::Null),
        });
    }
    r
}

/// Demande de résumé : le projet, et le plan que l'interface affiche s'il y en a un (sinon le
/// moteur calcule le plan).
#[derive(Deserialize)]
pub struct Demande {
    pub projet: Value,
    #[serde(default)]
    pub plan: Option<PlanVu>,
}

/// Résumé du projet, ou `{ "error": motif }` si le projet est illisible.
pub fn repondre(d: &Demande) -> Value {
    let resultat = Projet::lire(d.projet.clone()).and_then(|projet| match &d.plan {
        Some(vu) => Ok(resume(&projet, vu)),
        None => projet.plan().map(|plan| resume(&projet, &PlanVu::from(&plan))),
    });
    resultat.unwrap_or_else(|e| json!({ "error": e }))
}
