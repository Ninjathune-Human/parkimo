//! Parc en ouvrage (lot 1) : plateau de silo sur les scénarios de contrôle.
//! Aucune place qui se chevauche ou sort de la zone stationnable, toute place donne sur une allée,
//! aucun poteau dans une allée, identifiants BIM stables, IFC produit.

use trame::geom::{v2, Quad, QuadHash, Region};
use trame::model::Site;
use trame::ouvrage::{bim, ifc, EmpriseIn, Ouvrage};
use trame::region;

fn quad(q: &[[f64; 2]; 4]) -> Quad {
    Quad([v2(q[0][0], q[0][1]), v2(q[1][0], q[1][1]), v2(q[2][0], q[2][1]), v2(q[3][0], q[3][1])])
}

fn site(nom: &str) -> Site {
    let mut s: Site = serde_json::from_slice(&std::fs::read(format!("scenarios/{nom}.json")).unwrap()).unwrap();
    s.ouvrage = Some(Ouvrage { actif: true, ..Default::default() });
    s
}

#[test]
fn plateaux_sur_les_scenarios() {
    for nom in ["rect", "triangle", "courbe", "ruisseau", "etroit"] {
        let s = site(nom);
        let t = std::time::Instant::now();
        let lay = trame::solve(&s);
        let ms = t.elapsed().as_secs_f64() * 1e3;
        let Some(o) = lay.ouvrage.as_ref() else {
            panic!("{nom} : pas de plateau ({:?})", lay.warnings);
        };
        assert!(ms < 500.0, "{nom} : {ms:.0} ms");
        assert!(!lay.stalls.is_empty(), "{nom}");
        assert_eq!(o.places_cles.len(), lay.stalls.len());
        assert_eq!(lay.metrics.stalls, lay.stalls.len());

        // Aucun chevauchement entre places.
        let mut h = QuadHash::new(6.0);
        for (i, st) in lay.stalls.iter().enumerate() {
            let q = quad(&st.q).shrink(0.03);
            assert!(!h.hits(&q), "{nom} : place {i} chevauche une autre");
            h.insert(q);
        }
        // Dalle et places dans la zone stationnable.
        let z = region::build(&s).unwrap();
        let b = Region::new(region::rings_of(&z.b, 15.0));
        assert!(b.contains_quad(&quad(&o.dalle), 0.05), "{nom} : dalle hors zone");
        // Toute place donne sur une allée (1 m devant sa face avant).
        let allees: Vec<Quad> = o.allees.iter().map(quad).collect();
        for (i, st) in lay.stalls.iter().enumerate() {
            let q = quad(&st.q);
            let front = (q.0[2] + q.0[3]) * 0.5;
            let mut n = (q.0[2] - q.0[3]).norm().left();
            if n.dot(front - q.center()) < 0.0 {
                n = -n;
            }
            let p = front + n * 1.0;
            assert!(allees.iter().any(|a| a.contains(p)), "{nom} : place {i} sans allée");
        }
        // Aucun poteau dans une allée.
        for (k, po) in o.poteaux.iter().enumerate() {
            let pq = quad(po).shrink(0.01);
            assert!(!allees.iter().any(|a| a.overlaps(&pq)), "{nom} : poteau {} dans une allée", o.poteaux_cles[k]);
        }
        eprintln!(
            "{nom} : {} places ({} en rive), dalle {:.2} × {:.2} m, {} modules{}, {:.1} m²/place, {ms:.1} ms",
            lay.stalls.len(),
            o.places_rive,
            o.largeur,
            o.longueur,
            o.modules,
            if o.demi_module { " et demi" } else { "" },
            o.m2_place
        );
    }
}

#[test]
fn emprise_dessinee_a_la_main() {
    let avec = |cx: f64, cy: f64, w: f64, h: f64, rot: f64, fixe: [f64; 2]| {
        let mut s = site("rect");
        s.ouvrage.as_mut().unwrap().emprise = Some(EmpriseIn { cx, cy, w, h, rot, fixe });
        trame::solve(&s)
    };
    let centre = |o: &trame::ouvrage::OuvrageOut| {
        let c = o.dalle.iter().fold([0.0, 0.0], |a, p| [a[0] + p[0] / 4.0, a[1] + p[1] / 4.0]);
        v2(c[0], c[1])
    };
    let zone = |w: &[String]| w.iter().any(|x| x.contains("dalle"));

    // Dimensions ramenées aux plus proches réalisables, centre et angle conservés.
    let lay = avec(60.0, 40.0, 61.3, 31.0, 30.0, [0.0, 0.0]);
    let o = lay.ouvrage.as_ref().expect("plateau");
    assert!((o.longueur - 62.4).abs() < 1e-6 && (o.largeur - 32.4).abs() < 1e-6, "{} × {}", o.longueur, o.largeur);
    assert!((o.angle.rem_euclid(180.0) - 30.0).abs() < 1e-6, "angle {}", o.angle);
    assert!((centre(o) - v2(60.0, 40.0)).len() < 1e-6);
    assert!(!zone(&lay.warnings), "{:?}", lay.warnings);
    assert_eq!(lay.metrics.stalls, lay.stalls.len());
    let d = Region::new(vec![o.dalle.iter().map(|p| v2(p[0], p[1])).collect()]);
    assert!(lay.stalls.iter().all(|st| d.contains_quad(&quad(&st.q), 0.01)), "place hors dalle");

    // Côté fixe : le côté −u reste à 30,65 m du centre demandé.
    let lay = avec(60.0, 40.0, 61.3, 32.4, 0.0, [-1.0, 0.0]);
    let o = lay.ouvrage.as_ref().unwrap();
    let xmin = o.dalle.iter().map(|p| p[0]).fold(f64::MAX, f64::min);
    assert!((xmin - (60.0 - 30.65)).abs() < 1e-6, "xmin {xmin}");

    // Hors parcelle, puis dans les reculs : avertissements distincts.
    let w = avec(5.0, 40.0, 62.4, 32.4, 0.0, [0.0, 0.0]).warnings;
    assert!(w.iter().any(|x| x.contains("dépasse la parcelle")), "{w:?}");
    let w = avec(60.0, 19.0, 62.4, 32.4, 0.0, [0.0, 0.0]).warnings;
    assert!(w.iter().any(|x| x.contains("zone constructible")), "{w:?}");
}

#[test]
fn identifiants_stables() {
    let s = site("rect");
    let lay = trame::solve(&s);
    let m1 = bim::modele(&lay, "projet", 2.5, 5.0, 6.0).unwrap();
    let m2 = bim::modele(&trame::solve(&s), "projet", 2.5, 5.0, 6.0).unwrap();
    let g1: Vec<&String> = m1.elements.iter().map(|e| &e.guid).collect();
    let g2: Vec<&String> = m2.elements.iter().map(|e| &e.guid).collect();
    assert_eq!(g1, g2);
    let mut uniques = g1.clone();
    uniques.sort();
    uniques.dedup();
    assert_eq!(uniques.len(), g1.len(), "identifiants en double");
    // Parcelle déplacée : mêmes éléments, mêmes identifiants.
    let mut t = s.clone();
    for p in t.parcel.iter_mut() {
        p[0] += 13.0;
        p[1] += 7.0;
    }
    let m3 = bim::modele(&trame::solve(&t), "projet", 2.5, 5.0, 6.0).unwrap();
    let mut a: Vec<&String> = m1.elements.iter().map(|e| &e.guid).collect();
    let mut b: Vec<&String> = m3.elements.iter().map(|e| &e.guid).collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn export_ifc() {
    let s = site("rect");
    let n = trame::solve(&s).stalls.len();
    let out = ifc::export(&ifc::IfcIn {
        site: s,
        georef: None,
        crs: "LOCAL".into(),
        date: "2026-11-06T18:00:00".into(),
        projet: "test".into(),
    });
    assert!(out.erreur.is_none());
    assert!(out.ifc.starts_with("ISO-10303-21;") && out.ifc.contains("FILE_SCHEMA(('IFC4'))"));
    assert_eq!(out.ifc.matches("=IFCSPACE(").count(), n + 9);
    assert!(!out.ifc.contains("IFCMAPCONVERSION"));
    // Sans ouvrage actif : export refusé.
    let mut s2 = site("rect");
    s2.ouvrage = None;
    let o2 = ifc::export(&ifc::IfcIn { site: s2, georef: None, crs: "LOCAL".into(), date: String::new(), projet: String::new() });
    assert!(o2.erreur.is_some());
}
