//! Parc en ouvrage (lot 1) : plateau de silo sur les scénarios de contrôle, pour les trois densités
//! de trame.
//! Aucune place qui se chevauche ou sort de la zone stationnable, toute place donne sur une allée,
//! aucun poteau dans une allée, identifiants BIM stables, IFC produit.

use parkimo::geom::{v2, Quad, QuadHash, Region};
use parkimo::model::Site;
use parkimo::ouvrage::plateau::{Densite, Implantation};
use parkimo::ouvrage::systemes::Systeme;
use parkimo::ouvrage::noyaux::AscenseursDemi;
use parkimo::ouvrage::{bim, ifc, EmpriseIn, Ouvrage};
use parkimo::region;

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
        let lay = parkimo::solve(&s);
        let ms = t.elapsed().as_secs_f64() * 1e3;
        let Some(o) = lay.ouvrage.as_ref() else {
            panic!("{nom} : pas de plateau ({:?})", lay.warnings);
        };
        // La durée dépend de la machine et des tests voisins : contrôlée sur demande (PARKIMO_PERF=1).
        if std::env::var_os("PARKIMO_PERF").is_some() {
            assert!(ms < 500.0, "{nom} : {ms:.0} ms");
        }
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
        parkimo::solve(&s)
    };
    let centre = |o: &parkimo::ouvrage::OuvrageOut| {
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
    let lay = parkimo::solve(&s);
    let m1 = bim::modele(&lay, "projet", 2.5, 5.0, 6.0).unwrap();
    let m2 = bim::modele(&parkimo::solve(&s), "projet", 2.5, 5.0, 6.0).unwrap();
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
    let m3 = bim::modele(&parkimo::solve(&t), "projet", 2.5, 5.0, 6.0).unwrap();
    let mut a: Vec<&String> = m1.elements.iter().map(|e| &e.guid).collect();
    let mut b: Vec<&String> = m3.elements.iter().map(|e| &e.guid).collect();
    a.sort();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn export_ifc() {
    let s = site("rect");
    let n = parkimo::solve(&s).stalls.len();
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
    let avertit = |lay: &parkimo::model::Layout, mot: &str| lay.warnings.iter().any(|w| w.contains(mot));
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
            let lay = parkimo::solve(&s);
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
            assert!(avertit(&parkimo::solve(&s), "Hauteur d'étage"));
        }
    }
    // Préfabriqué avec des places de 2,50 m : 3 × 2,50 = 7,50 m, pas un multiple de 1,20 m.
    let mut s = site("rect");
    s.ouvrage.as_mut().unwrap().systeme = Systeme::Prefa;
    assert!(avertit(&parkimo::solve(&s), "démoduler"));
    // Acier-béton en portée courte : signalé.
    let o = s.ouvrage.as_mut().unwrap();
    (o.systeme, o.densite) = (Systeme::Mixte, Densite::Courte);
    assert!(avertit(&parkimo::solve(&s), "portée longue seulement"));
}

#[test]
fn niveaux_et_rampes() {
    // 3 niveaux : total = rez + niveau courant + dernier ; deux rampes de 18,36 m à 18 % et leurs
    // quatre épures de giration dégagées ; rien dans les rampes ; hauteur libre sur rampe
    // contrôlée ; dalle trop étroite signalée.
    let mut s = site("rect");
    s.ouvrage.as_mut().unwrap().niveaux = 3;
    let lay = parkimo::solve(&s);
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
    let pas = |lay: &parkimo::model::Layout, mot: &str| !lay.warnings.iter().any(|w| w.contains(mot));
    assert!(pas(&lay, "Rampes") && pas(&lay, "Giration") && pas(&lay, "Hauteur libre sur rampe"), "{:?}", lay.warnings);
    // Étage de 2,40 m : (2,40 − 0,40) × cos(atan 0,18) = 1,97 < 2,00 + 0,11.
    s.ouvrage.as_mut().unwrap().hauteur_etage = 2.4;
    assert!(!pas(&parkimo::solve(&s), "Hauteur libre sur rampe"));
    // Dalle d'un module (16,40 m) : ni le long des allées (pas d'autre allée pour aborder la rampe)
    // ni en pignon.
    s.ouvrage.as_mut().unwrap().hauteur_etage = 2.9;
    s.ouvrage.as_mut().unwrap().emprise = Some(EmpriseIn { cx: 60.0, cy: 40.0, w: 60.0, h: 16.4, rot: 0.0, fixe: [0.0, 0.0] });
    let lay = parkimo::solve(&s);
    assert!(lay.ouvrage.as_ref().unwrap().rampes.is_empty(), "{:?}", lay.warnings);
    assert!(!pas(&lay, "Rampes le long des allées") && !pas(&lay, "Rampes en pignon"), "{:?}", lay.warnings);
}

#[test]
fn implantations_sur_les_scenarios() {
    // Trois niveaux : chaque implantation trouve un plateau où ses deux rampes tiennent, épures
    // dégagées. La recherche fait passer les plateaux sans rampe possible après les autres.
    for nom in ["rect", "triangle", "courbe", "ruisseau"] {
        for imp in [Implantation::Auto, Implantation::Rangee, Implantation::Pignon, Implantation::DemiNiveaux, Implantation::Helice, Implantation::Inclines, Implantation::HeliceExt, Implantation::HeliceDouble, Implantation::HeliceCentre, Implantation::HeliceAnneau, Implantation::HeliceFacade] {
            let mut s = site(nom);
            let o = s.ouvrage.as_mut().unwrap();
            o.niveaux = 3;
            o.implantation = imp;
            let lay = parkimo::solve(&s);
            let ov = lay.ouvrage.as_ref().unwrap();
            let n_rampes = if imp == Implantation::Inclines { 4 } else { 2 };
            assert_eq!(ov.rampes.len(), n_rampes, "{nom} {imp:?} : {:?}", lay.warnings);
            if imp != Implantation::Auto {
                assert_eq!(ov.implantation, imp.nom(), "{nom}");
            }
            assert!(ov.girations.iter().all(|g| g.conflits.is_empty()), "{nom} {imp:?} {:?} {} × {}", ov.girations.iter().map(|g| (g.cle.clone(), g.conflits.clone())).collect::<Vec<_>>(), ov.largeur, ov.longueur);
            if imp == Implantation::Inclines {
                // Deux demi-tours horizontaux et deux modules en pente (6 % au plus ici) ; places
                // PMR de niveau rappelées.
                assert!(ov.plateaux_inclines && ov.rampes.iter().filter(|r| r.forme == "plateau").all(|r| r.pente <= 0.075 + 1e-9));
                assert!(ov.rampes.iter().filter(|r| r.forme == "virage").all(|r| r.pente == 0.0));
                assert!(lay.warnings.iter().any(|w| w.contains("places PMR sur une partie horizontale")));
                assert_eq!(lay.metrics.stalls, 3 * ov.places_niveau, "{nom}");
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
    let lay = parkimo::solve(&s);
    assert_eq!(lay.ouvrage.as_ref().unwrap().rampes.len(), 4, "{:?}", lay.warnings);
    assert!(lay.warnings.iter().any(|w| w.starts_with("Plateaux inclinés à 6,44 %")), "{:?}", lay.warnings);
    // Site étroit : aucune rampe le long des allées possible, le motif est donné.
    let mut s = site("etroit");
    let o = s.ouvrage.as_mut().unwrap();
    o.niveaux = 3;
    o.implantation = Implantation::Rangee;
    let lay = parkimo::solve(&s);
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
        for imp in [Implantation::Rangee, Implantation::Pignon, Implantation::DemiNiveaux, Implantation::Helice, Implantation::Inclines, Implantation::HeliceExt, Implantation::HeliceDouble, Implantation::HeliceCentre, Implantation::HeliceAnneau, Implantation::HeliceFacade] {
            for (pslv, liaison) in [(false, false), (true, false), (false, true)] {
                let mut s = site(nom);
                let o = s.ouvrage.as_mut().unwrap();
                o.niveaux = 3;
                o.implantation = imp;
                o.largement_ventile = pslv;
                if liaison {
                    o.ascenseurs_demi = AscenseursDemi::Liaison;
                }
                let lay = parkimo::solve(&s);
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
    let lay = parkimo::solve(&site("rect"));
    let ov = lay.ouvrage.as_ref().unwrap();
    assert!(ov.noyaux.is_empty() && ov.evacuation.is_none());
}

#[test]
fn noyaux_deplaces_a_la_main() {
    // Les centres des noyaux automatiques, renvoyés comme saisie, redonnent les mêmes cages ; un
    // escalier lâché sur le centre d'une place la prend.
    use parkimo::ouvrage::noyaux::Nature;
    use parkimo::ouvrage::NoyauIn;
    for imp in [Implantation::Rangee, Implantation::DemiNiveaux] {
        let mut s = site("ruisseau");
        let o = s.ouvrage.as_mut().unwrap();
        o.niveaux = 3;
        o.implantation = imp;
        o.ascenseurs_demi = AscenseursDemi::Liaison;
        let lay = parkimo::solve(&s);
        let ov = lay.ouvrage.as_ref().unwrap();
        let centre = |q: &[[f64; 2]; 4]| [(q[0][0] + q[2][0]) / 2.0, (q[0][1] + q[2][1]) / 2.0];
        let nature = |n: &parkimo::ouvrage::NoyauOut| if n.ascenseur { Nature::Ascenseur } else if n.liaison { Nature::Liaison } else { Nature::Escalier };
        let mut saisis: Vec<NoyauIn> = ov.noyaux.iter().map(|n| NoyauIn { nature: nature(n), p: centre(&n.q) }).collect();
        s.ouvrage.as_mut().unwrap().noyaux = Some(saisis.clone());
        let lay2 = parkimo::solve(&s);
        let ov2 = lay2.ouvrage.as_ref().unwrap();
        let ev2 = ov2.evacuation.as_ref().unwrap();
        assert!(ev2.manuel);
        let q = |o: &parkimo::ouvrage::OuvrageOut| {
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
        let lay3 = parkimo::solve(&s);
        let ov3 = lay3.ouvrage.as_ref().unwrap();
        assert!(ov3.noyaux.iter().any(|n| !n.ascenseur && !n.liaison && n.q == cible), "{imp:?}");
        assert_eq!(lay3.stalls.len(), lay.stalls.len());
        assert!(!lay3.stalls.iter().any(|st| st.q == cible));
    }
}

/// RDC, étage courant et dernier niveau : le plan affiché est l'étage courant ; les places propres
/// au rez et au dernier niveau sont rendues à part, avec les épures de giration de chaque extrémité.
#[test]
fn le_rez_et_le_dernier_niveau_ont_leurs_places_en_plus() {
    let mut s = site("rect");
    s.ouvrage.as_mut().unwrap().niveaux = 3;
    let lay = parkimo::solve(&s);
    let o = lay.ouvrage.as_ref().unwrap();
    assert!(!o.girations.is_empty());
    assert_eq!(o.places_niveau, lay.stalls.len());
    assert_eq!(o.places_rdc, lay.stalls.len() + o.places_rdc_seul.len());
    assert_eq!(o.places_dernier, lay.stalls.len() + o.places_dernier_seul.len());
    assert!(!o.places_rdc_seul.is_empty() && !o.places_dernier_seul.is_empty(), "rez {} dernier {}", o.places_rdc_seul.len(), o.places_dernier_seul.len());
    // Ces places ne recouvrent aucune place de l'étage courant.
    let centre = |q: &[[f64; 2]; 4]| [(q[0][0] + q[2][0]) / 2.0, (q[0][1] + q[2][1]) / 2.0];
    for q in o.places_rdc_seul.iter().chain(&o.places_dernier_seul) {
        let c = centre(q);
        assert!(lay.stalls.iter().all(|st| { let d = centre(&st.q); (c[0] - d[0]).hypot(c[1] - d[1]) > 1.0 }));
    }
    // Chaque épure dit à quelle extrémité de rampe elle sert : le bas (absent du dernier niveau) ou le haut (absent du rez).
    assert!(o.girations.iter().any(|g| g.bas) && o.girations.iter().any(|g| !g.bas));
}

fn rez_payant(niveaux: u32, voies: u32) -> parkimo::model::Layout {
    let mut s = site("rect");
    let o = s.ouvrage.as_mut().unwrap();
    (o.niveaux, o.controle, o.voies_controle) = (niveaux, true, voies);
    parkimo::solve(&s)
}

fn cotes(q: &[[f64; 2]; 4]) -> (f64, f64) {
    let d = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
    (d(q[0], q[1]), d(q[1], q[2]))
}

/// Rez payant : trois voies de 2,50 m séparées par des îlots (Neufert p. 274-276), barrières, bornes,
/// local d'accueil de deux places ; les places prises ne le sont qu'au rez.
#[test]
fn le_rez_payant_pose_ses_voies_ses_ilots_et_son_local() {
    let lay = rez_payant(3, 3);
    let o = lay.ouvrage.as_ref().unwrap();
    let c = o.controle.as_ref().expect("contrôle d'accès");
    let sens: Vec<&str> = c.voies.iter().map(|v| v.sens.as_str()).collect();
    assert!(sens == ["entree", "reversible", "sortie"] || sens == ["sortie", "reversible", "entree"], "{sens:?}");
    for v in &c.voies {
        let (a, b) = cotes(&v.q);
        assert!((a.min(b) - 2.5).abs() < 1e-6 && a.max(b) > 4.5, "voie {a} x {b}");
    }
    assert_eq!((c.ilots.len(), c.barrieres.len()), (2, 3));
    assert!(c.ilots.iter().all(|q| { let (a, b) = cotes(q); a.min(b) >= 0.75 - 1e-6 }));
    assert!(c.bornes.len() >= 2);
    // Aucun poteau dans une voie.
    for p in &o.poteaux {
        let m = [(p[0][0] + p[2][0]) / 2.0, (p[0][1] + p[2][1]) / 2.0];
        assert!(!c.voies.iter().any(|v| quad(&v.q).contains(v2(m[0], m[1]))), "poteau dans une voie");
    }
    // Local : deux places, moins les murs de 0,20 m et la surlargeur de la place voisine.
    let l = c.local.as_ref().expect("local d'accueil");
    let (a, b) = cotes(&l.q);
    assert!((a.min(b) - 4.9).abs() < 1e-6 || (a.min(b) - 5.0).abs() < 1e-6, "local {a} x {b}");
    assert!((l.surface - (a.min(b) - 0.4) * (a.max(b) - 0.4)).abs() < 0.01 && l.surface > 19.0 && l.surface < 22.0, "{} m²", l.surface);
    // Les places prises ne le sont qu'au rez : décompte par niveau.
    assert!(o.places_rdc_retirees.len() >= 6, "{} places retirées", o.places_rdc_retirees.len());
    assert_eq!(o.places_rdc, lay.stalls.len() - o.places_rdc_retirees.len() + o.places_rdc_seul.len());
    assert_eq!(o.places_niveau, lay.stalls.len());
    assert_eq!(lay.metrics.stalls, o.places_rdc + o.places_niveau + o.places_dernier);
    // Aucune place du rez sous une voie ou dans le local.
    let au_rez = lay.stalls.iter().enumerate().filter(|(i, _)| !o.places_rdc_retirees.contains(i)).map(|(_, st)| st.q).chain(o.places_rdc_seul.iter().copied());
    for q in au_rez {
        let m = quad(&q).center();
        assert!(!c.voies.iter().any(|v| quad(&v.q).contains(m)) && !quad(&l.q).contains(m), "place sous le contrôle");
    }
}

/// La place voisine du mur du local passe à 2,60 m, pris sur le local ; un local en bout de rangée
/// garde ses 5,00 m.
#[test]
fn la_place_voisine_du_local_est_elargie_aux_depens_du_local() {
    let mut vues = [0, 0];
    for (edge, t) in (0..4).flat_map(|e| [0.2, 0.5, 0.8].map(|t| (e, t))) {
        let mut s = site("rect");
        s.entries = vec![parkimo::model::Entry { edge, t }];
        let o = s.ouvrage.as_mut().unwrap();
        (o.niveaux, o.controle) = (2, true);
        let lay = parkimo::solve(&s);
        let o = lay.ouvrage.as_ref().unwrap();
        let l = o.controle.as_ref().unwrap().local.as_ref().unwrap();
        let (a, b) = cotes(&l.q);
        let elargies = o.places_rdc_seul.iter().filter(|q| { let (a, b) = cotes(q); (a.min(b) - 2.6).abs() < 1e-6 }).count();
        if a.min(b) < 5.0 - 1e-6 {
            assert!((a.min(b) - 4.9).abs() < 1e-6, "local de {} m", a.min(b));
            assert_eq!(elargies, 1, "accès {edge} à {t}");
            assert!((l.surface - 4.5 * 4.6).abs() < 0.01, "{} m²", l.surface);
            vues[0] += 1;
        } else {
            assert!((a.min(b) - 5.0).abs() < 1e-6 && elargies == 0, "accès {edge} à {t} : local de {} m", a.min(b));
            vues[1] += 1;
        }
    }
    assert!(vues[0] > 0, "aucun cas avec place voisine : {vues:?}");
}

#[test]
fn le_rez_payant_suit_le_nombre_de_voies_et_reste_facultatif() {
    let deux = rez_payant(2, 2);
    let c = deux.ouvrage.as_ref().unwrap().controle.clone().unwrap();
    assert_eq!((c.voies.len(), c.ilots.len()), (2, 1));
    assert!(c.voies.iter().all(|v| v.sens != "reversible"));
    // Un seul niveau : le rez est tout le parc.
    let un = rez_payant(1, 3);
    let o = un.ouvrage.as_ref().unwrap();
    assert!(o.controle.is_some());
    assert_eq!(un.metrics.stalls, un.stalls.len() - o.places_rdc_retirees.len() + o.places_rdc_seul.len());
    // Sans rez payant, rien ne change.
    let mut s = site("rect");
    s.ouvrage.as_mut().unwrap().niveaux = 3;
    let libre = parkimo::solve(&s);
    let o = libre.ouvrage.as_ref().unwrap();
    assert!(o.controle.is_none() && o.places_rdc_retirees.is_empty());
}

fn milieu(q: &[[f64; 2]; 4]) -> [f64; 2] {
    [(q[0][0] + q[2][0]) / 2.0, (q[0][1] + q[2][1]) / 2.0]
}

/// Rampe placée à la main : elle vient au point demandé le long de sa rangée, on l'aborde alors en S
/// depuis l'allée, et les places se réorganisent autour d'elle. L'autre rampe ne bouge pas.
#[test]
fn une_rampe_se_place_a_la_main_le_long_de_sa_rangee() {
    let mut s = site("rect");
    let o = s.ouvrage.as_mut().unwrap();
    (o.niveaux, o.implantation) = (3, Implantation::Rangee);
    let auto = parkimo::solve(&s);
    let ra = auto.ouvrage.as_ref().unwrap().rampes.clone();
    assert_eq!(ra.len(), 2);
    // Dix mètres plus loin, dans le sens de la montée.
    let (c0, de, a) = (milieu(&ra[0].q), ra[0].de, ra[0].a);
    let l = (a[0] - de[0]).hypot(a[1] - de[1]);
    let cible = [c0[0] + 10.0 * (a[0] - de[0]) / l, c0[1] + 10.0 * (a[1] - de[1]) / l];
    s.ouvrage.as_mut().unwrap().rampes = Some(vec![Some(cible), None]);
    let lay = parkimo::solve(&s);
    let o = lay.ouvrage.as_ref().unwrap();
    let c1 = milieu(&o.rampes[0].q);
    assert!((c1[0] - cible[0]).hypot(c1[1] - cible[1]) < 0.05, "rampe en {c1:?}, demandée en {cible:?}");
    assert!((o.rampes[0].longueur - ra[0].longueur).abs() < 1e-6 && o.rampes[0].montee);
    let (d0, d1) = (milieu(&ra[1].q), milieu(&o.rampes[1].q));
    assert!((d0[0] - d1[0]).hypot(d0[1] - d1[1]) < 1e-6, "la descente a bougé");
    // Le plateau s'adapte : aucune place sous la rampe, quatre épures, un nombre de places voisin.
    for st in &lay.stalls {
        assert!(!quad(&o.rampes[0].q).contains(quad(&st.q).center()), "place sous la rampe");
    }
    assert_eq!(o.girations.len(), 4);
    assert!((lay.stalls.len() as i64 - auto.stalls.len() as i64).abs() <= 8, "{} places au lieu de {}", lay.stalls.len(), auto.stalls.len());
    // Un point hors du plateau ramène la rampe au plus près, dans la dalle.
    s.ouvrage.as_mut().unwrap().rampes = Some(vec![Some([c0[0] + 500.0 * (a[0] - de[0]) / l, c0[1] + 500.0 * (a[1] - de[1]) / l]), None]);
    let loin = parkimo::solve(&s);
    let o = loin.ouvrage.as_ref().unwrap();
    assert!(o.rampes[0].q.iter().all(|p| quad(&o.dalle).contains(v2(p[0], p[1]))), "rampe hors de la dalle");
}
