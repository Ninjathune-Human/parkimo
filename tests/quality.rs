//! Invariants de qualité du plan : aucune place qui se chevauche, aucune place hors zone,
//! toute place donne sur une voie circulable, temps de calcul compatible avec le direct.

use trame::geom::{v2, Quad, QuadHash, Region, V2};
use trame::model::{Layout, Site};
use trame::region;

fn quad(q: &[[f64; 2]; 4]) -> Quad {
    Quad([
        v2(q[0][0], q[0][1]),
        v2(q[1][0], q[1][1]),
        v2(q[2][0], q[2][1]),
        v2(q[3][0], q[3][1]),
    ])
}

fn check(site: &Site, lay: &Layout, name: &str) {
    // 1. Aucun chevauchement entre places.
    let mut h = QuadHash::new(6.0);
    for (i, s) in lay.stalls.iter().enumerate() {
        let q = quad(&s.q).shrink(0.03);
        assert!(!h.hits(&q), "{name}: place {i} chevauche une autre");
        h.insert(q);
    }
    // 2. Toute place est dans la zone stationnable (reculs, servitudes, bâtiments exclus).
    let z = region::build(site).expect("zones");
    let b = Region::new(region::rings_of(&z.b, 15.0));
    for (i, s) in lay.stalls.iter().enumerate() {
        assert!(
            b.contains_quad(&quad(&s.q), 0.05),
            "{name}: place {i} hors zone stationnable"
        );
    }
    // 3. Toute place donne sur la voirie (anneau, allées, allées transversales, accès).
    let mut rings: Vec<Vec<V2>> = lay
        .paved
        .iter()
        .map(|r| r.iter().map(|p| v2(p[0], p[1])).collect())
        .collect();
    for q in lay.cross.iter().chain(lay.entries.iter()) {
        rings.push(quad(q).0.to_vec());
    }
    let drive = Region::new(rings);
    let extra: Vec<Quad> = lay
        .cross
        .iter()
        .chain(lay.entries.iter())
        .map(quad)
        .collect();
    for (i, s) in lay.stalls.iter().enumerate() {
        let q = quad(&s.q);
        let front = (q.0[2] + q.0[3]) * 0.5;
        let n = (q.0[3] - q.0[0]).norm();
        let p = front + n * 1.0;
        let on_road = drive.contains(p) || extra.iter().any(|e| e.contains(p));
        assert!(
            on_road,
            "{name}: place {i} sans accès à la voirie ({:.1},{:.1})",
            p.x, p.y
        );
    }
    // 4. Quotas PMR et IRVE atteints dès qu'il y a des places.
    let m = &lay.metrics;
    if m.stalls > 20 {
        assert!(
            m.pmr >= m.pmr_required,
            "{name}: PMR {} < {}",
            m.pmr,
            m.pmr_required
        );
        assert!(
            m.irve >= m.irve_required,
            "{name}: IRVE {} < {}",
            m.irve,
            m.irve_required
        );
    }
}

fn load(name: &str) -> Site {
    let path = format!("{}/scenarios/{name}.json", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn scenarios_respect_invariants() {
    for name in ["rect", "courbe", "ruisseau", "triangle", "etroit"] {
        let site = load(name);
        // Meilleur de trois calculs : les autres tests tournent en parallèle sur la même machine.
        let mut ms = f64::INFINITY;
        let mut lay = trame::solve(&site);
        for _ in 0..3 {
            let t = std::time::Instant::now();
            lay = trame::solve(&site);
            ms = ms.min(t.elapsed().as_secs_f64() * 1000.0);
        }
        check(&site, &lay, name);
        assert!(
            lay.metrics.stalls > 150,
            "{name}: rendement anormalement bas ({})",
            lay.metrics.stalls
        );
        if !cfg!(debug_assertions) {
            assert!(ms < 60.0, "{name}: {ms:.1} ms");
        }
    }
}

#[test]
fn rectangle_yield_is_dense() {
    // 120 × 80 m, reculs 5/3 m : un bon plan dépasse 300 places à moins de 26 m² d'enrobé par place.
    let lay = trame::solve(&load("rect"));
    assert!(lay.metrics.stalls >= 300, "{} places", lay.metrics.stalls);
    assert!(
        lay.metrics.m2_paved_per_stall < 26.0,
        "{} m²/place",
        lay.metrics.m2_paved_per_stall
    );
}

#[test]
fn random_parcels_never_break_invariants() {
    let mut seed: u64 = 42;
    let mut rnd = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((seed >> 33) as f64) / (1u64 << 31) as f64
    };
    for case in 0..60 {
        let n = 3 + (rnd() * 9.0) as usize;
        let r0 = 40.0 + rnd() * 90.0;
        let mut parcel = Vec::new();
        for i in 0..n {
            let a = i as f64 / n as f64 * std::f64::consts::TAU;
            let r = r0 * (0.6 + 0.4 * rnd());
            parcel.push([r * a.cos(), r * a.sin()]);
        }
        let setbacks: Vec<f64> = (0..n).map(|_| (rnd() * 6.0).floor()).collect();
        let json = serde_json::json!({
            "parcel": parcel, "setbacks": setbacks,
            "entries": [{"edge": 0, "t": 0.5}],
            "buildings": if case % 3 == 0 { serde_json::json!([{"poly": [[-10,-10],[15,-10],[15,12],[-10,12]]}]) } else { serde_json::json!([]) },
            "exclusions": if case % 4 == 1 { serde_json::json!([[[-200,-6],[200,-2],[200,6],[-200,3]]]) } else { serde_json::json!([]) }
        });
        let site: Site = serde_json::from_value(json).unwrap();
        let lay = trame::solve(&site);
        check(&site, &lay, &format!("aléatoire {case}"));
    }
}

#[test]
fn degenerate_inputs_do_not_panic() {
    for json in [
        r#"{"parcel":[[0,0],[1,0]]}"#,
        r#"{"parcel":[[0,0],[100,100],[100,0],[0,100]],"entries":[{"edge":0,"t":0.5}]}"#,
        r#"{"parcel":[[0,0],[10,0],[10,10],[0,10]],"setbacks":[20,20,20,20]}"#,
        r#"{"parcel":[[0,0],[300,0],[300,200],[0,200]],"entries":[{"edge":9,"t":2}]}"#,
        r#"{"parcel":[[0,0],[100,0],[100,60],[0,60]],"exclusions":[[[0,0],[100,0],[100,60],[0,60]]]}"#,
    ] {
        let site: Site = serde_json::from_str(json).unwrap();
        let _ = trame::solve(&site);
    }
}

#[test]
fn adjacent_parcels_merge_into_one_outline() {
    // Deux parcelles voisines avec un micro-écart de 2 cm, et une troisième isolée.
    let a = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 30.0], [0.0, 30.0]];
    let b = vec![[20.02, 0.0], [45.0, 0.0], [45.0, 30.0], [20.02, 30.0]];
    let c = vec![[100.0, 0.0], [110.0, 0.0], [110.0, 10.0], [100.0, 10.0]];
    let rings = trame::union_rings(&[a, b, c]);
    assert_eq!(rings.len(), 2, "{rings:?}");
    let area = |r: &Vec<[f64; 2]>| {
        let n = r.len();
        (0..n)
            .map(|i| r[i][0] * r[(i + 1) % n][1] - r[(i + 1) % n][0] * r[i][1])
            .sum::<f64>()
            .abs()
            / 2.0
    };
    assert!(
        (area(&rings[0]) - 45.0 * 30.0).abs() < 2.0,
        "aire {}",
        area(&rings[0])
    );
}

/// Flèches au sol : dans la chaussée, hors des places ; sens unique fortement connexe.
fn check_circulation(lay: &Layout, name: &str) {
    use trame::circulation::{ARROW_L, ARROW_W};
    let c = &lay.circulation;
    assert!(!c.arrows.is_empty(), "{name}: aucune flèche");
    let mut rings: Vec<Vec<V2>> = lay
        .paved
        .iter()
        .map(|r| r.iter().map(|p| v2(p[0], p[1])).collect())
        .collect();
    for q in lay.cross.iter().chain(lay.entries.iter()) {
        rings.push(quad(q).0.to_vec());
    }
    let paved = Region::new(
        lay.paved
            .iter()
            .map(|r| r.iter().map(|p| v2(p[0], p[1])).collect())
            .collect(),
    );
    let extra: Vec<Quad> = lay
        .cross
        .iter()
        .chain(lay.entries.iter())
        .map(quad)
        .collect();
    let mut stalls = QuadHash::new(6.0);
    for s in &lay.stalls {
        stalls.insert(quad(&s.q));
    }
    for q in &lay.islands {
        stalls.insert(quad(q));
    }
    for (i, a) in c.arrows.iter().enumerate() {
        let t = v2(a.a.to_radians().cos(), a.a.to_radians().sin());
        let n = t.left();
        let p = v2(a.p[0], a.p[1]);
        let (hl, hw) = (ARROW_L / 2.0, ARROW_W / 2.0);
        let fp = Quad([
            p - t * hl - n * hw,
            p + t * hl - n * hw,
            p + t * hl + n * hw,
            p - t * hl + n * hw,
        ]);
        for &k in &fp.0 {
            assert!(
                paved.contains(k) || extra.iter().any(|e| e.contains(k)),
                "{name}: flèche {i} hors chaussée ({:.1},{:.1})",
                k.x,
                k.y
            );
        }
        assert!(
            !stalls.hits(&fp.shrink(0.05)),
            "{name}: flèche {i} sur une place"
        );
    }
    // Réseau orienté : extrémités d'axes regroupées en carrefours, la rue reliée aux accès.
    let mut nodes: Vec<V2> = Vec::new();
    let id = |p: V2, nodes: &mut Vec<V2>| match nodes.iter().position(|q| (*q - p).len() < 1.0) {
        Some(i) => i,
        None => {
            nodes.push(p);
            nodes.len() - 1
        }
    };
    let mut arcs: Vec<(usize, usize)> = Vec::new();
    for a in &c.axes {
        let (p0, p1) = (a.pts[0], a.pts[a.pts.len() - 1]);
        let (u, v) = (
            id(v2(p0[0], p0[1]), &mut nodes),
            id(v2(p1[0], p1[1]), &mut nodes),
        );
        arcs.push((u, v));
        if a.sens == 2 {
            arcs.push((v, u));
        }
    }
    let street = nodes.len();
    for e in lay.entries.iter().map(quad) {
        for (i, n) in nodes.iter().enumerate() {
            if e.contains(*n) {
                arcs.push((street, i));
                arcs.push((i, street));
            }
        }
    }
    for rev in [false, true] {
        let mut seen = vec![false; street + 1];
        let mut st = vec![street];
        seen[street] = true;
        while let Some(v) = st.pop() {
            for &(a, b) in &arcs {
                let (a, b) = if rev { (b, a) } else { (a, b) };
                if a == v && !seen[b] {
                    seen[b] = true;
                    st.push(b);
                }
            }
        }
        // Nœuds desservis : ceux reliés à la rue par le réseau non orienté.
        let mut und = vec![false; street + 1];
        let mut st = vec![street];
        und[street] = true;
        while let Some(v) = st.pop() {
            for &(a, b) in &arcs {
                for (x, y) in [(a, b), (b, a)] {
                    if x == v && !und[y] {
                        und[y] = true;
                        st.push(y);
                    }
                }
            }
        }
        for i in 0..street {
            assert!(
                !und[i] || seen[i],
                "{name}: carrefour {i} {} depuis la rue",
                if rev {
                    "ne permet pas de ressortir"
                } else {
                    "inatteignable"
                }
            );
        }
    }
}

#[test]
fn circulation_marks_every_scenario() {
    for name in ["rect", "courbe", "ruisseau", "triangle", "etroit"] {
        let lay = trame::solve(&load(name));
        assert!(
            lay.circulation.axes.iter().all(|a| a.sens == 2),
            "{name}: allées de 6 m à double sens"
        );
        check_circulation(&lay, name);
    }
}

#[test]
fn narrow_aisles_become_one_way_and_stay_connected() {
    for name in ["rect", "courbe", "ruisseau"] {
        let mut site = load(name);
        site.params.aisle = 4.0;
        let lay = trame::solve(&site);
        let one = lay.circulation.axes.iter().filter(|a| a.sens == 1).count();
        assert!(
            one > 0,
            "{name}: aucune voie à sens unique avec des allées de 4 m"
        );
        check_circulation(&lay, name);
    }
}

#[test]
fn interstitial_spaces_are_planted_without_covering_stalls_or_lanes() {
    for name in ["rect", "courbe", "ruisseau", "triangle", "etroit"] {
        let lay = trame::solve(&load(name));
        let planted = Region::new(
            lay.planted
                .iter()
                .flatten()
                .map(|r| r.iter().map(|p| v2(p[0], p[1])).collect())
                .collect(),
        );
        assert!(!planted.is_empty(), "{name}: aucun espace planté");
        for (i, s) in lay.stalls.iter().enumerate() {
            assert!(
                !planted.contains(quad(&s.q).center()),
                "{name}: place {i} plantée"
            );
        }
        for a in &lay.circulation.arrows {
            assert!(
                !planted.contains(v2(a.p[0], a.p[1])),
                "{name}: flèche sur un espace planté"
            );
        }
    }
}

#[test]
fn entries_get_a_three_metre_turning_radius_free_of_stalls() {
    for name in ["rect", "courbe", "ruisseau", "triangle", "etroit"] {
        let site = load(name);
        let lay = trame::solve(&site);
        let z = region::build(&site).expect("zones");
        let entries: Vec<Quad> = lay.entries.iter().map(quad).collect();
        let (keep, fillets) = trame::entry_turns(&site, &z.parcel, &entries);
        assert_eq!(keep.len(), 2 * entries.len(), "{name}");
        let paved = Region::new(
            lay.paved
                .iter()
                .map(|r| r.iter().map(|p| v2(p[0], p[1])).collect())
                .collect(),
        );
        for (k, f) in keep.iter().zip(&fillets) {
            for (i, s) in lay.stalls.iter().enumerate() {
                assert!(
                    !k.overlaps(&quad(&s.q).shrink(0.05)),
                    "{name}: place {i} dans le rayon d'entrée"
                );
            }
            // Le coin de l'arrondi (près de l'angle de l'accès) est en chaussée.
            let corner = f[0] + (f[5] - f[0]) * 0.5;
            assert!(
                paved.contains(corner),
                "{name}: arrondi d'entrée absent de la chaussée"
            );
        }
    }
}

#[test]
fn skeleton_toggles_lane_typology_and_keeps_the_site_connected() {
    use trame::model::LaneRule;
    fn mid(pts: &[[f64; 2]]) -> [f64; 2] {
        let len: f64 = pts
            .windows(2)
            .map(|s| (v2(s[1][0], s[1][1]) - v2(s[0][0], s[0][1])).len())
            .sum();
        let mut acc = 0.0;
        for s in pts.windows(2) {
            let (a, b) = (v2(s[0][0], s[0][1]), v2(s[1][0], s[1][1]));
            let l = (b - a).len();
            if acc + l >= len / 2.0 && l > 0.0 {
                let p = a + (b - a) * ((len / 2.0 - acc) / l);
                return [p.x, p.y];
            }
            acc += l;
        }
        pts[0]
    }
    for name in ["rect", "ruisseau", "courbe"] {
        let site = load(name);
        let base = trame::solve(&site);
        let mut tried = 0;
        for a in base.circulation.axes.iter().filter(|a| a.pts.len() >= 2) {
            if tried == 6 {
                break;
            }
            tried += 1;
            let target = if a.sens == 2 { 1 } else { 2 };
            let p = mid(&a.pts);
            let mut s = site.clone();
            // Typologie seule : la largeur des voies ne change pas.
            s.params.aisle_one = s.params.aisle;
            s.lanes = vec![LaneRule {
                p,
                sens: target,
                d: None,
                fix: false,
            }];
            let lay = trame::solve(&s);
            let c = &lay.circulation;
            let touched = c
                .axes
                .iter()
                .min_by(|x, y| {
                    let d = |ax: &trame::circulation::Axis| {
                        let q = v2(p[0], p[1]);
                        ax.pts
                            .windows(2)
                            .map(|s| {
                                let (a, b) = (v2(s[0][0], s[0][1]), v2(s[1][0], s[1][1]));
                                let ab = b - a;
                                let t = ((q - a).dot(ab) / ab.dot(ab).max(1e-12)).clamp(0.0, 1.0);
                                (a + ab * t - q).len()
                            })
                            .fold(f64::INFINITY, f64::min)
                    };
                    d(x).partial_cmp(&d(y)).unwrap()
                })
                .unwrap();
            assert!(
                touched.sens == target || (target == 1 && c.refused == 1),
                "{name}: axe {:?} non basculé ({} → {}, refus {})",
                p,
                a.sens,
                touched.sens,
                c.refused
            );
            assert_eq!(
                lay.stalls.len(),
                base.stalls.len(),
                "{name}: la typologie ne change pas la géométrie"
            );
            check_circulation(&lay, name);
        }
    }
}

/// Milieu d'un axe et sa direction (dans l'ordre des points).
fn mid_dir(pts: &[[f64; 2]]) -> ([f64; 2], [f64; 2]) {
    let len: f64 = pts
        .windows(2)
        .map(|s| (v2(s[1][0], s[1][1]) - v2(s[0][0], s[0][1])).len())
        .sum();
    let mut acc = 0.0;
    for s in pts.windows(2) {
        let (a, b) = (v2(s[0][0], s[0][1]), v2(s[1][0], s[1][1]));
        let l = (b - a).len();
        if acc + l >= len / 2.0 && l > 0.0 {
            let p = a + (b - a) * ((len / 2.0 - acc) / l);
            let t = (b - a).norm();
            return ([p.x, p.y], [t.x, t.y]);
        }
        acc += l;
    }
    (pts[0], [1.0, 0.0])
}

/// Traverses verticales (tracées par le concepteur) qui remplacent les anciennes coupures automatiques.
fn with_traverses(site: &mut trame::model::Site, name: &str) {
    let (xs, h): (&[f64], f64) = match name {
        "rect" => (&[40.0, 80.0], 80.0),
        "courbe" => (&[47.0, 93.0], 115.0),
        _ => (&[53.0, 107.0], 110.0),
    };
    site.traverses = xs.iter().map(|&x| [[x, 0.0], [x, h]]).collect();
}

fn one_way(p: [f64; 2], d: [f64; 2], fix: bool) -> trame::model::LaneRule {
    trame::model::LaneRule {
        p,
        sens: 1,
        d: Some(d),
        fix,
    }
}

#[test]
fn one_way_lanes_take_their_width_and_stay_put() {
    for name in ["rect", "courbe", "ruisseau"] {
        let mut site = load(name);
        with_traverses(&mut site, name);
        site.params.aisle_one = 4.0;
        let base = trame::solve(&site);
        let mut narrowed = 0;
        let mut tried = 0;
        for a in base.circulation.axes.iter().filter(|a| a.sens == 2) {
            let (p, d) = mid_dir(&a.pts);
            let mut s = site.clone();
            s.lanes = vec![one_way(p, d, false)];
            let lay = trame::solve(&s);
            if lay.circulation.refused > 0 {
                continue;
            }
            tried += 1;
            let ax = lay
                .circulation
                .axes
                .iter()
                .find(|x| x.rule == Some(0))
                .unwrap_or_else(|| panic!("{name}: repère {p:?} perdu"));
            assert_eq!(ax.sens, 1, "{name}: {p:?}");
            if (ax.w - 4.0).abs() < 0.8 {
                narrowed += 1;
            }
            check(&s, &lay, name);
            check_circulation(&lay, name);
            // Repère recalé : le calcul suivant redonne le même plan.
            let re = lay.circulation.lanes[0];
            assert!(re.fix, "{name}: sens de parcours inscrit");
            s.lanes = vec![re];
            let again = trame::solve(&s);
            assert_eq!(
                again.stalls.len(),
                lay.stalls.len(),
                "{name}: {p:?} instable"
            );
            assert_eq!(
                again.circulation.lanes[0], re,
                "{name}: {p:?} repère instable"
            );
        }
        // Allées entre rangées et côtés droits de l'anneau ; les tronçons obliques, les allées
        // transversales et les raccords gardent leur largeur.
        assert!(
            tried > 4 && narrowed >= 1 && (name == "ruisseau" || narrowed * 3 >= tried),
            "{name}: {narrowed}/{tried} voies resserrées"
        );
    }
}

#[test]
fn successive_one_way_lanes_add_up() {
    let mut site = load("rect");
    with_traverses(&mut site, "rect");
    site.params.aisle_one = 4.0;
    let mut lay = trame::solve(&site);
    let n0 = lay.stalls.len();
    // Trois allées entre rangées, l'une après l'autre, avec le recalage de l'interface.
    for _ in 0..3 {
        let next = lay
            .circulation
            .axes
            .iter()
            .filter(|a| a.sens == 2 && a.rule.is_none() && a.w >= 5.9 && a.w < 9.0)
            .find(|a| {
                let (p, d) = mid_dir(&a.pts);
                d[0].abs() < 0.1 && p[1] > 20.0 && p[1] < 60.0
            })
            .expect("allée verticale à double sens");
        let (p, d) = mid_dir(&next.pts);
        site.lanes = lay.circulation.lanes.clone();
        site.lanes.push(one_way(p, d, false));
        lay = trame::solve(&site);
        site.lanes = lay.circulation.lanes.clone();
    }
    let narrowed: Vec<_> = lay
        .circulation
        .axes
        .iter()
        .filter(|a| a.rule.is_some())
        .collect();
    assert_eq!(narrowed.len(), 3);
    assert!(
        narrowed
            .iter()
            .all(|a| a.sens == 1 && ((a.w - 4.0).abs() < 0.3 || a.w >= 5.9)), // une traverse garde sa largeur
        "{narrowed:?}"
    );
    assert!(lay.stalls.len() >= n0, "{} < {n0}", lay.stalls.len());
    check(&site, &lay, "rect");
    check_circulation(&lay, "rect");
    let again = trame::solve(&site);
    assert_eq!(again.stalls.len(), lay.stalls.len());
    assert_eq!(again.circulation.lanes, lay.circulation.lanes);
}

#[test]
fn imposed_direction_is_kept_or_refused() {
    let site = load("rect");
    let base = trame::solve(&site);
    let mut flipped = 0;
    for a in base.circulation.axes.iter().filter(|a| a.sens == 2).take(8) {
        let (p, d) = mid_dir(&a.pts);
        for sign in [1.0, -1.0] {
            let want = [d[0] * sign, d[1] * sign];
            let mut s = site.clone();
            s.lanes = vec![one_way(p, want, true)];
            let lay = trame::solve(&s);
            if lay.circulation.refused > 0 {
                continue;
            }
            let ax = lay
                .circulation
                .axes
                .iter()
                .find(|x| x.rule == Some(0))
                .unwrap();
            let (_, got) = mid_dir(&ax.pts);
            assert!(
                got[0] * want[0] + got[1] * want[1] > 0.5,
                "sens imposé {want:?}, obtenu {got:?}"
            );
            flipped += 1;
            check_circulation(&lay, "rect");
        }
    }
    assert!(flipped >= 8, "{flipped}");
}

#[test]
fn traverse_cuts_rows_and_removing_it_restores_them() {
    let mut site = load("rect");
    let base = trame::solve(&site);
    assert!(base.cross.is_empty(), "plus de coupure automatique");
    site.traverses = vec![[[60.0, 0.0], [60.0, 80.0]]];
    let cut = trame::solve(&site);
    assert!(!cut.cross.is_empty(), "la traverse crée des allées transversales");
    assert!(cut.stalls.len() < base.stalls.len());
    check(&site, &cut, "rect");
    site.traverses.clear();
    assert_eq!(trame::solve(&site).stalls.len(), base.stalls.len());
}
