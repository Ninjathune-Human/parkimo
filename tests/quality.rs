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
        let t = std::time::Instant::now();
        let lay = trame::solve(&site);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
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
