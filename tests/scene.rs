//! Scène 3D rendue par le moteur : le plan décrit en éléments typés (emprise, cotes, matière), que
//! la vue three.js se contente d'afficher.

use parkimo::model::Layout;
use parkimo::scene::{self, Matiere, Scene, Volume};
use serde_json::{json, Value};

fn plan(nom: &str, regle: impl FnOnce(&mut Value)) -> Layout {
    let mut v: Value = serde_json::from_slice(&std::fs::read(format!("scenarios/{nom}.json")).unwrap()).unwrap();
    if v.get("params").is_none() {
        v["params"] = json!({});
    }
    v["params"]["turns"] = json!(false);
    v["params"]["scene"] = json!(true);
    regle(&mut v);
    parkimo::solve(&serde_json::from_value(v).unwrap())
}

fn scene_de(nom: &str, regle: impl FnOnce(&mut Value)) -> (Layout, Scene) {
    let lay = plan(nom, regle);
    let s = lay.scene.clone().expect("scène demandée");
    (lay, s)
}

fn de(s: &Scene, m: Matiere) -> Vec<&Volume> {
    s.volumes.iter().filter(|v| v.matiere == m).collect()
}

/// Aire d'un volume en plan : extérieur moins trous.
fn aire(v: &Volume) -> f64 {
    let a = |r: &Vec<[f64; 2]>| {
        (0..r.len()).map(|i| r[i][0] * r[(i + 1) % r.len()][1] - r[(i + 1) % r.len()][0] * r[i][1]).sum::<f64>().abs() / 2.0
    };
    a(&v.anneaux[0]) - v.anneaux[1..].iter().map(a).sum::<f64>()
}

#[test]
fn sans_demande_le_plan_ne_porte_pas_de_scene() {
    let lay = plan("rect", |v| v["params"]["scene"] = json!(false));
    assert!(lay.scene.is_none());
}

#[test]
fn chaque_place_du_plan_est_dans_la_scene() {
    let (lay, s) = scene_de("ruisseau", |_| {});
    assert!(lay.stalls.len() > 20);
    assert_eq!(s.places.len(), lay.stalls.len());
    for (p, st) in s.places.iter().zip(&lay.stalls) {
        assert_eq!((p.q, p.k, p.borne, p.niveau), (st.q, st.k, st.e, 0));
        assert_eq!(p.z, scene::ENROBE);
    }
}

#[test]
fn chaque_arbre_du_plan_est_dans_la_scene_avec_sa_taille() {
    let (lay, s) = scene_de("ruisseau", |_| {});
    assert!(!lay.trees.is_empty());
    assert_eq!(s.arbres.len(), lay.trees.len());
    for (a, t) in s.arbres.iter().zip(&lay.trees) {
        assert_eq!(a.p, *t);
        assert!((5.0..=9.0).contains(&a.h) && (1.5..=3.0).contains(&a.r), "arbre {a:?}");
    }
    // Les tailles varient d'un arbre à l'autre, et restent les mêmes d'un calcul à l'autre.
    assert!(s.arbres.iter().any(|a| (a.h - s.arbres[0].h).abs() > 0.05));
    let (_, s2) = scene_de("ruisseau", |_| {});
    assert_eq!(s.arbres.iter().map(|a| a.h).collect::<Vec<_>>(), s2.arbres.iter().map(|a| a.h).collect::<Vec<_>>());
}

#[test]
fn les_espaces_plantes_sont_releves_d_une_bordure() {
    let (lay, s) = scene_de("rect", |_| {});
    assert!(!lay.planted.is_empty());
    let plantes = de(&s, Matiere::Plante);
    assert!(!plantes.is_empty());
    for v in &plantes {
        assert_eq!((v.z0, v.z1), (0.0, scene::BORDURE));
    }
}

#[test]
fn l_enrobe_couvre_la_voirie_du_plan_d_un_seul_tenant() {
    let (lay, s) = scene_de("rect", |_| {});
    let enrobe = de(&s, Matiere::Enrobe);
    let a: f64 = enrobe.iter().map(|v| aire(v)).sum();
    assert!((a - lay.metrics.paved_area).abs() < 0.03 * lay.metrics.paved_area, "enrobé {a:.0} m², voirie {:.0} m²", lay.metrics.paved_area);
    // Voirie, fonds de rangées, places et accès réunis : bien moins de morceaux que de rangées.
    assert!(enrobe.len() < lay.pads.len().max(2), "{} morceaux pour {} fonds de rangée", enrobe.len(), lay.pads.len());
    for v in &enrobe {
        assert_eq!(v.z1, scene::ENROBE);
    }
}

#[test]
fn un_batiment_est_un_volume_couronne_d_un_acrotere() {
    let (_, s) = scene_de("ruisseau", |v| v["buildings"][0]["door"] = json!({"p": [115.0, 14.0], "n": [-1.0, 0.0], "w": 2.0}));
    let b = de(&s, Matiere::Batiment);
    assert_eq!(b.len(), 1);
    assert_eq!((b[0].z0, b[0].z1), (0.0, scene::HAUTEUR_BATIMENT));
    let a = de(&s, Matiere::Acrotere);
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].anneaux.len(), 2, "couronne : contour et vide intérieur");
    assert!(a[0].z0 == scene::HAUTEUR_BATIMENT && a[0].z1 > a[0].z0);
    assert_eq!(de(&s, Matiere::Porte).len(), 1);
    assert_eq!(s.haut, a[0].z1);
}

#[test]
fn un_bassin_descend_en_gradins_jusqu_a_sa_profondeur() {
    let bassin = json!([[30.0, 30.0], [60.0, 30.0], [60.0, 55.0], [30.0, 55.0]]);
    let (lay, s) = scene_de("rect", |v| {
        v["basins"] = json!([bassin]);
        v["params"]["basin_h"] = json!(1.5);
    });
    let g = de(&s, Matiere::Bassin);
    assert!(g.len() >= 4, "{} gradins", g.len());
    let fond = g.iter().map(|v| v.z1).fold(f64::INFINITY, f64::min);
    assert!((fond + 1.5).abs() < 1e-9, "fond à {fond}");
    assert!(g.iter().all(|v| v.z1 <= 0.0 && v.z0 < v.z1));
    // Le terrain est ouvert au droit du bassin.
    let terrain: f64 = de(&s, Matiere::Terrain).iter().map(|v| aire(v)).sum();
    assert!((terrain - (lay.metrics.parcel_area - 30.0 * 25.0)).abs() < 1.0, "terrain {terrain:.0} m²");
}

#[test]
fn la_boite_englobante_contient_tous_les_elements() {
    let (_, s) = scene_de("triangle", |_| {});
    assert!(s.min[0] < s.max[0] && s.min[1] < s.max[1]);
    for v in &s.volumes {
        for p in v.anneaux.iter().flatten() {
            assert!(p[0] >= s.min[0] - 1e-6 && p[0] <= s.max[0] + 1e-6 && p[1] >= s.min[1] - 1e-6 && p[1] <= s.max[1] + 1e-6);
        }
    }
    assert!(de(&s, Matiere::Terrain).iter().all(|v| v.z1 == 0.0 && v.z0 < 0.0));
}

#[test]
fn les_fleches_de_circulation_sont_dans_la_scene() {
    let (lay, s) = scene_de("rect", |_| {});
    assert!(!lay.circulation.arrows.is_empty());
    assert_eq!(s.fleches.len(), lay.circulation.arrows.len());
}

fn silo(niveaux: u32) -> (Layout, Scene) {
    scene_de("silo", |v| {
        v["ouvrage"]["niveaux"] = json!(niveaux);
    })
}

#[test]
fn le_silo_empile_ses_niveaux() {
    let (lay, s) = silo(3);
    let o = lay.ouvrage.as_ref().unwrap();
    let h = o.hauteur_etage;
    assert_eq!(s.niveaux.iter().map(|n| n.z).collect::<Vec<_>>(), vec![0.0, h, 2.0 * h]);
    assert_eq!(s.places.len(), o.places_rdc + o.places_niveau + o.places_dernier);
    for (k, n) in [(0u32, o.places_rdc), (1, o.places_niveau), (2, o.places_dernier)] {
        assert_eq!(s.places.iter().filter(|p| p.niveau == k).count(), n, "niveau {k}");
        assert!(s.places.iter().filter(|p| p.niveau == k).all(|p| (p.z - k as f64 * h).abs() < 0.05));
    }
    // Une dalle par niveau ; poteaux et poutres sous chaque dalle d'étage.
    assert_eq!(de(&s, Matiere::Dalle).iter().map(|v| v.niveau).collect::<std::collections::BTreeSet<_>>().len(), 3);
    assert_eq!(de(&s, Matiere::Poteau).len(), 2 * o.poteaux.len());
    assert_eq!(de(&s, Matiere::Poutre).len(), 2 * o.poutres.len());
    for v in de(&s, Matiere::Poteau) {
        let z = v.niveau as f64 * h;
        assert!((v.z0 - z).abs() < 0.05 && (v.z1 - (z + h - o.dalle_ep)).abs() < 1e-6, "poteau {v:?}");
    }
    assert!(s.haut >= 2.0 * h + 1.0);
}

#[test]
fn un_silo_d_un_niveau_est_une_dalle_sans_structure() {
    let (_, s) = silo(1);
    assert_eq!(de(&s, Matiere::Dalle).len(), 1);
    assert!(de(&s, Matiere::Poteau).is_empty() && s.rampes.is_empty());
}

#[test]
fn les_rampes_relient_deux_niveaux_voisins() {
    let (lay, s) = silo(3);
    let o = lay.ouvrage.as_ref().unwrap();
    assert!(!o.rampes.is_empty());
    assert_eq!(s.rampes.len(), 2 * o.rampes.len());
    for r in &s.rampes {
        let (z0, z1) = (r.axe[0][2], r.axe[r.axe.len() - 1][2]);
        assert!(((z1 - z0).abs() - o.hauteur_etage).abs() < 1e-6, "rampe de {z0} à {z1}");
        assert!(z0.min(z1) >= r.niveau as f64 * o.hauteur_etage - 1e-6);
        assert!(r.largeur >= 2.5 && r.largeur <= 8.0, "largeur {}", r.largeur);
    }
}

#[test]
fn les_dalles_d_etage_sont_ouvertes_au_droit_des_rampes() {
    let (lay, s) = silo(2);
    let o = lay.ouvrage.as_ref().unwrap();
    let dalle = |k: u32| de(&s, Matiere::Dalle).iter().filter(|v| v.niveau == k).map(|v| aire(v)).sum::<f64>();
    assert!((dalle(0) - o.largeur * o.longueur).abs() < 0.5, "dalle basse {:.0} m²", dalle(0));
    assert!(dalle(1) < dalle(0) - 10.0, "dalle d'étage {:.0} m², dalle basse {:.0} m²", dalle(1), dalle(0));
}

#[test]
fn les_etages_sont_bordes_d_un_garde_corps() {
    let (_, s) = silo(3);
    let g = de(&s, Matiere::GardeCorps);
    assert!(!g.is_empty() && g.iter().all(|v| v.niveau >= 1 && v.z1 - v.z0 >= 1.0));
}
