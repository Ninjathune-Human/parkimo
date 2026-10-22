//! Résout un scénario JSON et écrit le plan : `cargo run --release --example run -- in.json out.json`
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let site: trame::model::Site =
        serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    let t = Instant::now();
    let reps = 5;
    let mut lay = trame::solve(&site);
    for _ in 1..reps {
        lay = trame::solve(&site);
    }
    let ms = t.elapsed().as_secs_f64() * 1000.0 / reps as f64;
    let m = &lay.metrics;
    println!(
        "{:<28} {:>5} places ({} périph. + {} int.) PMR {}/{} IRVE {}/{} arbres {} | {:.1} m² enrobé/place | angle {:.2}° | {} évals | {:.1} ms {:?}",
        args[1].rsplit('/').next().unwrap(), m.stalls, m.perimeter_stalls, m.interior_stalls, m.pmr, m.pmr_required,
        m.irve, m.irve_required, m.trees, m.m2_paved_per_stall, lay.angle, lay.evals, ms, lay.warnings
    );
    if let Some(o) = args.get(2) {
        std::fs::write(o, serde_json::to_vec(&serde_json::json!({"site_parcel": site.parcel, "exclusions": site.exclusions,
            "buildings": site.buildings.iter().map(|b| b.poly.clone()).collect::<Vec<_>>(), "landscapes": site.landscapes,
            "layout": serde_json::to_value(&lay).unwrap()})).unwrap()).unwrap();
    }
}
