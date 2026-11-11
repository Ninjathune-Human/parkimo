//! Ombrage du parc extérieur : superficie du parc, ombre des arbres à maturité, ombrières.

use parkimo::model::Layout;
use serde_json::{json, Value};

fn plan(nom: &str, regle: impl FnOnce(&mut Value)) -> Layout {
    let mut v: Value = serde_json::from_slice(&std::fs::read(format!("scenarios/{nom}.json")).unwrap()).unwrap();
    if v.get("params").is_none() {
        v["params"] = json!({});
    }
    v["params"]["turns"] = json!(false);
    regle(&mut v);
    parkimo::solve(&serde_json::from_value(v).unwrap())
}

fn aire(q: &[[f64; 2]; 4]) -> f64 {
    (0..4).map(|i| q[i][0] * q[(i + 1) % 4][1] - q[(i + 1) % 4][0] * q[i][1]).sum::<f64>().abs() / 2.0
}

#[test]
fn l_ombre_des_arbres_est_mesuree_sur_la_superficie_du_parc() {
    let lay = plan("rect", |_| {});
    let o = &lay.ombrage;
    // Superficie du parc : emplacements et voies, sans les espaces verts.
    assert!((o.surface - lay.metrics.paved_area).abs() < 0.03 * lay.metrics.paved_area, "{} m² pour {} m² de voirie", o.surface, lay.metrics.paved_area);
    assert!(o.surface < lay.metrics.parcel_area);
    assert_eq!((o.nb_arbres, o.houppier), (lay.trees.len(), 8.0));
    // Chaque houppier de 8 m ombrage au plus 50 m² ; une part tombe sur les espaces verts.
    let plein = lay.trees.len() as f64 * std::f64::consts::PI * 16.0;
    assert!(o.arbres > 0.2 * plein && o.arbres < plein, "{} m² d'ombre pour {} arbres", o.arbres, lay.trees.len());
    assert_eq!((o.ombrieres, o.total), (0.0, o.arbres));
}

#[test]
fn un_houppier_plus_large_ombrage_davantage() {
    let ombre = |d: f64| plan("rect", |v| v["params"]["canopy"] = json!(d)).ombrage.arbres;
    let (petit, grand) = (ombre(4.0), ombre(10.0));
    assert!(grand > 3.0 * petit, "{petit} m² puis {grand} m²");
    assert_eq!(ombre(0.0), 0.0);
}

#[test]
fn les_ombrieres_couvrent_les_rangees_et_remplacent_leurs_arbres() {
    let sans = plan("rect", |_| {});
    assert!(sans.ombrieres.is_empty());
    let avec = plan("rect", |v| v["params"]["ombrieres"] = json!(true));
    assert!(!avec.pads.is_empty());
    assert_eq!(avec.ombrieres, avec.pads);
    let emprise: f64 = avec.ombrieres.iter().map(aire).sum();
    assert!((avec.ombrage.ombrieres - emprise).abs() < 0.01 * emprise, "{} m² mesurés, {} m² d'emprise", avec.ombrage.ombrieres, emprise);
    // Plus d'arbre sous une ombrière ; ceux des limites restent.
    assert!(avec.trees.len() < sans.trees.len() && !avec.trees.is_empty());
    assert_eq!(avec.metrics.trees, avec.trees.len());
    assert_eq!(avec.stalls.len(), sans.stalls.len());
    // L'ombre totale ne compte pas deux fois ce qui est sous un arbre et sous une ombrière.
    let o = &avec.ombrage;
    assert!(o.total <= o.arbres + o.ombrieres + 1e-6 && o.total >= o.ombrieres.max(o.arbres) - 1e-6);
    assert!(o.total <= o.surface);
}

#[test]
fn un_parc_en_ouvrage_n_a_ni_ombrage_ni_ombrieres() {
    let lay = plan("silo", |v| v["params"]["ombrieres"] = json!(true));
    assert!(lay.ombrieres.is_empty());
    assert_eq!(lay.ombrage.surface, 0.0);
}
