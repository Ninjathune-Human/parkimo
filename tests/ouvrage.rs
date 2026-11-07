//! Parc en ouvrage (lot 1) : plateau de silo sur les scénarios de contrôle, pour les trois densités
//! de trame.
//! Aucune place qui se chevauche ou sort de la zone stationnable, toute place donne sur une allée,
//! aucun poteau dans une allée, identifiants BIM stables, IFC produit.

use trame::geom::{v2, Quad, QuadHash, Region};
use trame::model::Site;
use trame::ouvrage::plateau::{Densite, Implantation};
use trame::ouvrage::systemes::Systeme;
use trame::ouvrage::noyaux::AscenseursDemi;
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
    let densites = [Densite::Longue, Densite::Moyenne, Densite::Courte];
    for (nom, densite) in ["rect", "triangle", "courbe", "ruisseau", "etroit"]
        .into_iter()
        .flat_map(|n| densites.map(|d| (n, d)))
    {
        let mut s = site(nom);
        s.ouvrage.as_mut().unwrap().densite = densite;
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
            "{nom} {densite:?} : {} places ({} en rive), dalle {:.2} × {:.2} m, {} modules{}, portée {:.2} m, {:.1} m²/place, {ms:.1} ms",
            lay.stalls.len(),
            o.places_rive,
            o.largeur,
            o.longueur,
            o.modules,
            if o.demi_module { " et demi" } else { "" },
            o.portee,
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

#[test]
fn systemes_constructifs() {
    // Chaque système porte un plancher ; épaisseur reprise par la dalle ; hauteur d'étage contrôlée
    // (NF P 91-100, § 3.2) ; module des dalles alvéolées contrôlé en préfabriqué.
    let avertit = |lay: &trame::model::Layout, mot: &str| lay.warnings.iter().any(|w| w.contains(mot));
    for (systeme, densites) in [
        (Systeme::Coule, vec![Densite::Longue, Densite::Moyenne, Densite::Courte]),
        (Systeme::Prefa, vec![Densite::Longue, Densite::Moyenne, Densite::Courte]),
        (Systeme::Mixte, vec![Densite::Longue]),
    ] {
        for densite in densites {
            let mut s = site("rect");
            let o = s.ouvrage.as_mut().unwrap();
            (o.systeme, o.densite) = (systeme, densite);
            if systeme == Systeme::Prefa {
                s.params.stall_w = 2.4;
            }
            if systeme == Systeme::Mixte {
                o.travee = 2;
            }
            let lay = trame::solve(&s);
            let ov = lay.ouvrage.as_ref().expect("plateau");
            assert!(ov.plancher.epaisseur > 0.0 && (ov.dalle_ep - ov.plancher.epaisseur).abs() < 1e-9);
            assert!(ov.plancher.hauteur_min > 2.2 && ov.plancher.hauteur_min < 2.9, "{systeme:?} {densite:?}");
            assert!(!avertit(&lay, "Hauteur d'étage"), "{systeme:?} {densite:?} : {:?}", lay.warnings);
            assert!(!avertit(&lay, "démoduler"), "{systeme:?} {densite:?} : {:?}", lay.warnings);
            let m = bim::modele(&lay, "projet", s.params.stall_w, 5.0, 6.0).unwrap();
            assert!(m.types.iter().any(|t| t.nom == format!("PKM_Plancher_{}", ov.plancher.code)));
            // Poutres : une IfcBeam par poutre du plan, sous la toiture, de la hauteur de la retombée.
            let poutres: Vec<_> = m.elements.iter().filter(|e| e.classe == "IfcBeam").collect();
            assert_eq!(poutres.len(), ov.poutres.len(), "{systeme:?} {densite:?}");
            assert_eq!(ov.poutres.is_empty(), ov.plancher.ossature.is_none(), "{systeme:?} {densite:?}");
            assert!(poutres.iter().all(|e| (e.h - ov.plancher.retombee).abs() < 1e-9));
            eprintln!(
                "{systeme:?} {densite:?} : {} ; étage ≥ {:.2} m ; {} places ; {} poutres",
                ov.plancher.libelle,
                ov.plancher.hauteur_min,
                lay.stalls.len(),
                ov.poutres.len()
            );
            // Étage trop bas : signalé.
            s.ouvrage.as_mut().unwrap().hauteur_etage = 2.3;
            assert!(avertit(&trame::solve(&s), "Hauteur d'étage"));
        }
    }
    // Préfabriqué avec des places de 2,50 m : 3 × 2,50 = 7,50 m, pas un multiple de 1,20 m.
    let mut s = site("rect");
    s.ouvrage.as_mut().unwrap().systeme = Systeme::Prefa;
    assert!(avertit(&trame::solve(&s), "démoduler"));
    // Acier-béton en portée courte : signalé.
    let o = s.ouvrage.as_mut().unwrap();
    (o.systeme, o.densite) = (Systeme::Mixte, Densite::Courte);
    assert!(avertit(&trame::solve(&s), "portée longue seulement"));
}

#[test]
fn niveaux_et_rampes() {
    // 3 niveaux : total = rez + niveau courant + dernier ; deux rampes de 18,36 m à 18 % et leurs
    // quatre épures de giration dégagées ; rien dans les rampes ; hauteur libre sur rampe
    // contrôlée ; dalle trop étroite signalée.
    let mut s = site("rect");
    s.ouvrage.as_mut().unwrap().niveaux = 3;
    let lay = trame::solve(&s);
    let o = lay.ouvrage.as_ref().unwrap();
    assert_eq!(o.niveaux, 3);
    assert_eq!(o.places_niveau, lay.stalls.len());
    assert!(o.places_rdc > o.places_niveau && o.places_dernier > o.places_niveau);
    assert_eq!(lay.metrics.stalls, o.places_rdc + o.places_niveau + o.places_dernier);
    assert_eq!(o.rampes.len(), 2);
    assert!(!o.implantation.is_empty());
    assert!(o.rampes.iter().all(|r| (r.pente - 0.18).abs() < 1e-9 && (r.longueur - 18.361).abs() < 1e-3));
    assert!(o.rampes[0].montee && !o.rampes[1].montee);
    assert_eq!(o.girations.len(), 4);
    assert!(o.girations.iter().all(|gi| gi.conflits.is_empty() && gi.contour.len() > 8 && gi.axe.len() > 2));
    let rampes: Vec<Quad> = o.rampes.iter().map(|r| quad(&r.q)).collect();
    for (i, st) in lay.stalls.iter().enumerate() {
        let q = quad(&st.q).shrink(0.01);
        assert!(!rampes.iter().any(|r| r.overlaps(&q)), "place {i} dans une rampe");
    }
    for (k, po) in o.poteaux.iter().enumerate() {
        let q = quad(po).shrink(0.01);
        assert!(!rampes.iter().any(|r| r.overlaps(&q)), "poteau {} dans une rampe", o.poteaux_cles[k]);
    }
    let pas = |lay: &trame::model::Layout, mot: &str| !lay.warnings.iter().any(|w| w.contains(mot));
    assert!(pas(&lay, "Rampes") && pas(&lay, "Giration") && pas(&lay, "Hauteur libre sur rampe"), "{:?}", lay.warnings);
    // Étage de 2,40 m : (2,40 − 0,40) × cos(atan 0,18) = 1,97 < 2,00 + 0,11.
    s.ouvrage.as_mut().unwrap().hauteur_etage = 2.4;
    assert!(!pas(&trame::solve(&s), "Hauteur libre sur rampe"));
    // Dalle d'un module (16,40 m) : ni le long des allées (pas d'autre allée pour aborder la rampe)
    // ni en pignon.
    s.ouvrage.as_mut().unwrap().hauteur_etage = 2.9;
    s.ouvrage.as_mut().unwrap().emprise = Some(EmpriseIn { cx: 60.0, cy: 40.0, w: 60.0, h: 16.4, rot: 0.0, fixe: [0.0, 0.0] });
    let lay = trame::solve(&s);
    assert!(lay.ouvrage.as_ref().unwrap().rampes.is_empty(), "{:?}", lay.warnings);
    assert!(!pas(&lay, "Rampes le long des allées") && !pas(&lay, "Rampes en pignon"), "{:?}", lay.warnings);
}

#[test]
fn implantations_sur_les_scenarios() {
    // Trois niveaux : chaque implantation trouve un plateau où ses deux rampes tiennent, épures
    // dégagées. La recherche fait passer les plateaux sans rampe possible après les autres.
    for nom in ["rect", "triangle", "courbe", "ruisseau"] {
        for imp in [Implantation::Auto, Implantation::Rangee, Implantation::Pignon, Implantation::DemiNiveaux, Implantation::Helice, Implantation::Inclines, Implantation::Accolees, Implantation::HeliceExt, Implantation::HeliceDouble] {
            let mut s = site(nom);
            let o = s.ouvrage.as_mut().unwrap();
            o.niveaux = 3;
            o.implantation = imp;
            let lay = trame::solve(&s);
            let ov = lay.ouvrage.as_ref().unwrap();
            let n_rampes = if imp == Implantation::Inclines { 4 } else { 2 };
            assert_eq!(ov.rampes.len(), n_rampes, "{nom} {imp:?} : {:?}", lay.warnings);
            if imp != Implantation::Auto {
                assert_eq!(ov.implantation, imp.nom(), "{nom}");
            }
            assert!(ov.girations.iter().all(|g| g.conflits.is_empty()), "{nom} {imp:?}");
            if imp == Implantation::Inclines {
                // Deux demi-tours horizontaux et deux modules en pente (6 % au plus ici) ; places
                // PMR de niveau rappelées.
                assert!(ov.plateaux_inclines && ov.rampes.iter().filter(|r| r.forme == "plateau").all(|r| r.pente <= 0.075 + 1e-9));
                assert!(ov.rampes.iter().filter(|r| r.forme == "virage").all(|r| r.pente == 0.0));
                assert!(lay.warnings.iter().any(|w| w.contains("places PMR sur une partie horizontale")));
                assert_eq!(lay.metrics.stalls, 3 * ov.places_niveau, "{nom}");
            }
            if imp == Implantation::Accolees {
                // Montée et descente côte à côte : un grand côté commun.
                let (a, b) = (&ov.rampes[0].q, &ov.rampes[1].q);
                let communs = a.iter().filter(|p| b.iter().any(|q| (p[0] - q[0]).hypot(p[1] - q[1]) < 1e-6)).count();
                assert_eq!(communs, 2, "{nom}");
            }
            if imp == Implantation::HeliceExt {
                // Dalle plus courte que l'emprise : les tours sont hors de la dalle.
                let d = &ov.dalle;
                let ld = (d[1][0] - d[0][0]).hypot(d[1][1] - d[0][1]);
                assert!(ld < ov.longueur - 20.0, "{nom} : dalle de {ld} m sur {} m", ov.longueur);
                assert!(ov.rampes.iter().all(|r| r.forme == "helice"));
            }
            if imp == Implantation::Helice {
                // Deux tours à un tour par niveau : pente = étage / (2π × 7,00).
                assert!(ov.rampes.iter().all(|r| r.forme == "helice" && (r.pente - 2.9 / (14.0 * std::f64::consts::PI)).abs() < 1e-3 && r.trou.len() > 8));
            }
            if imp == Implantation::DemiNiveaux {
                // Deux demi-rampes en U à double sens, pente dans l'axe de la voie intérieure.
                assert!(ov.demi_niveaux && ov.rampes.iter().all(|r| r.double_sens && (r.pente - 1.45 / r.longueur).abs() < 1e-3));
                assert_eq!(lay.metrics.stalls, 3 * ov.places_niveau, "{nom}");
            }
        }
    }
    // Plateaux inclinés sur une dalle dessinée de 33 × 46 m, ramenée à 32,90 × 46,50 (rangées de
    // round((46 − 24) / 2,50) = 9 places) : 1,45 / 22,50 = 6,44 %, admis par la NF (7,5 %) mais
    // au-delà des 6 % de Neufert.
    let mut s = site("rect");
    let o = s.ouvrage.as_mut().unwrap();
    o.niveaux = 3;
    o.implantation = Implantation::Inclines;
    o.emprise = Some(EmpriseIn { cx: 60.0, cy: 40.0, w: 46.0, h: 33.0, rot: 0.0, fixe: [0.0, 0.0] });
    let lay = trame::solve(&s);
    assert_eq!(lay.ouvrage.as_ref().unwrap().rampes.len(), 4, "{:?}", lay.warnings);
    assert!(lay.warnings.iter().any(|w| w.starts_with("Plateaux inclinés à 6,44 %")), "{:?}", lay.warnings);
    // Site étroit : aucune rampe le long des allées possible, le motif est donné.
    let mut s = site("etroit");
    let o = s.ouvrage.as_mut().unwrap();
    o.niveaux = 3;
    o.implantation = Implantation::Rangee;
    let lay = trame::solve(&s);
    assert!(lay.ouvrage.as_ref().unwrap().rampes.is_empty());
    assert!(lay.warnings.iter().any(|w| w.starts_with("Rampes le long des allées")), "{:?}", lay.warnings);
}

#[test]
fn escaliers_et_ascenseur() {
    // Trois niveaux : escaliers et ascenseur dans des emprises de places, hors des rampes et des
    // autres places (un poteau de façade peut entrer dans le mur d'une cage), porte hors des
    // épures de giration ; toutes les places à portée d'un escalier (PS 13). Un seul niveau :
    // aucun noyau.
    let mut demis = 0;
    for nom in ["rect", "triangle", "courbe", "ruisseau"] {
        for imp in [Implantation::Rangee, Implantation::Pignon, Implantation::DemiNiveaux, Implantation::Helice, Implantation::Inclines, Implantation::Accolees, Implantation::HeliceExt, Implantation::HeliceDouble] {
            for (pslv, liaison) in [(false, false), (true, false), (false, true)] {
                let mut s = site(nom);
                let o = s.ouvrage.as_mut().unwrap();
                o.niveaux = 3;
                o.implantation = imp;
                o.largement_ventile = pslv;
                if liaison {
                    o.ascenseurs_demi = AscenseursDemi::Liaison;
                }
                let lay = trame::solve(&s);
                let ov = lay.ouvrage.as_ref().unwrap();
                let cas = format!("{nom} {imp:?} PSLV {pslv} liaison {liaison}");
                let ev = ov.evacuation.as_ref().expect("bilan d'évacuation");
                assert_eq!(ev.limite_deux, if pslv { 50.0 } else { 40.0 });
                assert_eq!(ev.hors, 0, "{cas} : {:?}", lay.warnings);
                assert!(!lay.warnings.iter().any(|w| w.starts_with("PS 13")), "{cas}");
                // Demi-niveaux : un ascenseur par demi-plateau, ou un seul et un escalier de liaison.
                demis += ov.demi_niveaux as usize;
                let (asc, lia) = match (ov.demi_niveaux, liaison) {
                    (true, false) => (2, 0),
                    (true, true) => (1, 1),
                    _ => (1, 0),
                };
                assert!(ev.escaliers >= 2 && ev.ascenseurs == asc && ev.liaisons == lia, "{cas} : {} escaliers, {} m, {:?}", ev.escaliers, ev.distance_max, ov.dalle);
                assert_eq!(ov.noyaux.len(), ev.escaliers + ev.ascenseurs + ev.liaisons);
                assert_eq!(lay.warnings.iter().any(|w| w.starts_with("Demi-niveaux : un seul ascenseur")), lia == 1, "{cas}");
                let obstacles: Vec<Quad> = ov
                    .rampes
                    .iter()
                    .filter(|r| r.forme != "plateau" && r.forme != "helice")
                    .map(|r| quad(&r.q))
                    .chain(lay.stalls.iter().map(|st| quad(&st.q)))
                    .collect();
                // Tours hélicoïdales : le cercle de la bordure, pas sa boîte.
                let tours: Vec<([f64; 2], f64)> = ov
                    .rampes
                    .iter()
                    .filter(|r| r.forme == "helice")
                    .map(|r| ([(r.q[0][0] + r.q[2][0]) / 2.0, (r.q[0][1] + r.q[2][1]) / 2.0], (r.q[1][0] - r.q[0][0]).hypot(r.q[1][1] - r.q[0][1]) / 2.0))
                    .collect();
                let dans_tour = |q: &[[f64; 2]; 4]| {
                    tours.iter().any(|&(c, rd)| {
                        (0..4).any(|i| {
                            let (a, b) = (q[i], q[(i + 1) % 4]);
                            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                            let t = (((c[0] - a[0]) * dx + (c[1] - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
                            (a[0] + t * dx - c[0]).hypot(a[1] + t * dy - c[1]) < rd - 0.01
                        })
                    })
                };
                for n in &ov.noyaux {
                    let q = quad(&n.q).shrink(0.01);
                    let k = obstacles.iter().position(|o| o.overlaps(&q));
                    assert!(k.is_none(), "{cas} : {} gêné par {k:?} sur {} rampes, {:?} {:?}", n.cle, ov.rampes.len(), n.q, k.map(|k| &obstacles[k]));
                    assert!(!dans_tour(&n.q), "{cas} : {} dans une tour", n.cle);
                    // Porte hors des épures de giration : 1,00 m devant le milieu de l'avant.
                    let (m, f) = ([(n.q[2][0] + n.q[3][0]) / 2.0, (n.q[2][1] + n.q[3][1]) / 2.0], [(n.q[0][0] + n.q[1][0]) / 2.0, (n.q[0][1] + n.q[1][1]) / 2.0]);
                    let l = (m[0] - f[0]).hypot(m[1] - f[1]);
                    let p = [m[0] + (m[0] - f[0]) / l, m[1] + (m[1] - f[1]) / l];
                    for gi in &ov.girations {
                        let c = &gi.contour;
                        let dedans = (0..c.len()).fold(false, |d, i| {
                            let (a, b) = (c[i], c[(i + 1) % c.len()]);
                            d ^ ((a[1] > p[1]) != (b[1] > p[1]) && a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]) > p[0])
                        });
                        assert!(!dedans, "{cas} : porte de {} dans l'épure {}", n.cle, gi.cle);
                    }
                }
            }
        }
    }
    assert!(demis >= 6, "{demis} cas en demi-niveaux");
    let lay = trame::solve(&site("rect"));
    let ov = lay.ouvrage.as_ref().unwrap();
    assert!(ov.noyaux.is_empty() && ov.evacuation.is_none());
}

#[test]
fn noyaux_deplaces_a_la_main() {
    // Les centres des noyaux automatiques, renvoyés comme saisie, redonnent les mêmes cages ; un
    // escalier lâché sur le centre d'une place la prend.
    use trame::ouvrage::noyaux::Nature;
    use trame::ouvrage::NoyauIn;
    for imp in [Implantation::Rangee, Implantation::DemiNiveaux] {
        let mut s = site("ruisseau");
        let o = s.ouvrage.as_mut().unwrap();
        o.niveaux = 3;
        o.implantation = imp;
        o.ascenseurs_demi = AscenseursDemi::Liaison;
        let lay = trame::solve(&s);
        let ov = lay.ouvrage.as_ref().unwrap();
        let centre = |q: &[[f64; 2]; 4]| [(q[0][0] + q[2][0]) / 2.0, (q[0][1] + q[2][1]) / 2.0];
        let nature = |n: &trame::ouvrage::NoyauOut| if n.ascenseur { Nature::Ascenseur } else if n.liaison { Nature::Liaison } else { Nature::Escalier };
        let mut saisis: Vec<NoyauIn> = ov.noyaux.iter().map(|n| NoyauIn { nature: nature(n), p: centre(&n.q) }).collect();
        s.ouvrage.as_mut().unwrap().noyaux = Some(saisis.clone());
        let lay2 = trame::solve(&s);
        let ov2 = lay2.ouvrage.as_ref().unwrap();
        let ev2 = ov2.evacuation.as_ref().unwrap();
        assert!(ev2.manuel);
        let q = |o: &trame::ouvrage::OuvrageOut| {
            let mut v: Vec<String> = o.noyaux.iter().map(|n| format!("{:?}", n.q.map(|p| p.map(|x| (x * 100.0).round() as i64)))).collect();
            v.sort();
            v
        };
        assert_eq!(q(ov), q(ov2), "{imp:?}");
        assert_eq!(lay.stalls.len(), lay2.stalls.len());
        assert_eq!(ov.evacuation.as_ref().unwrap().hors, ev2.hors);
        // Le premier escalier lâché sur une place libre la prend ; sa place d'origine est rendue.
        let cible = lay.stalls.iter().map(|st| st.q).find(|q| !ov.noyaux.iter().any(|n| n.q == *q)).unwrap();
        let k = saisis.iter().position(|n| n.nature == Nature::Escalier).unwrap();
        saisis[k].p = centre(&cible);
        s.ouvrage.as_mut().unwrap().noyaux = Some(saisis);
        let lay3 = trame::solve(&s);
        let ov3 = lay3.ouvrage.as_ref().unwrap();
        assert!(ov3.noyaux.iter().any(|n| !n.ascenseur && !n.liaison && n.q == cible), "{imp:?}");
        assert_eq!(lay3.stalls.len(), lay.stalls.len());
        assert!(!lay3.stalls.iter().any(|st| st.q == cible));
    }
}
