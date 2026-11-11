// Audit : rangées de 15 places à 90° (2,50 × 5,00 m) face à face, allée de 6 m à double sens.
// Place au milieu de la rangée du haut, sous plusieurs hypothèses (véhicule, marge, occupation).
use parkimo::giration::api::places_with;
use parkimo::giration::vehicle::{Convention, Vehicle};

fn main() {
    let path = format!("{}/scenarios/rect.json", env!("CARGO_MANIFEST_DIR"));
    let site: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let aisle: f64 = std::env::var("AISLE").ok().and_then(|a| a.parse().ok()).unwrap_or(6.0);
    let (w, n) = (2.5, 15usize);
    let (y0, y1) = (25.0, 25.0 + aisle);
    let x0 = 60.0 - w * n as f64 / 2.0;
    let top: Vec<[[f64; 2]; 4]> = (0..n).map(|i| { let a = x0 + w * i as f64; [[a, y1 + 5.0], [a + w, y1 + 5.0], [a + w, y1], [a, y1]] }).collect();
    let bottom: Vec<[[f64; 2]; 4]> = (0..n).map(|i| { let a = x0 + w * i as f64; [[a + w, y0 - 5.0], [a, y0 - 5.0], [a, y0], [a + w, y0]] }).collect();
    // Voiture de référence de Neufert (p. 269-271) : 4,74 × 1,76 m, empattement 2,70 m, porte-à-faux
    // 0,94 / 1,10 m, rayon de giration extérieur 5,85 m (lu entre murs). Voie et pneus : supposés.
    let neufert = Vehicle { wheelbase: 2.70, front_overhang: 0.94, rear_overhang: 1.10, width: 1.76, track: 1.50, tyre_width: 0.205, ground_clearance: 0.14, max_steer: 0.0, lock_to_lock: 4.0 }
        .with_turning_diameter(11.70, Convention::Walls);
    let vl = Vehicle::peugeot_3008();
    let vl_walls = Vehicle::peugeot_3008().with_turning_diameter(10.90, Convention::Walls);
    let run = |label: &str, v: &Vehicle, margin: f64, opposite: bool, neighbours: bool| {
        let mut stalls = top.clone();
        if opposite { stalls.extend(bottom.iter().copied()); }
        let input = serde_json::json!({ "site": site, "layout": { "paved": [[[5.0, y0], [115.0, y0], [115.0, y1], [5.0, y1]]], "pads": [], "islands": [], "entries": [], "cross": [], "stalls": stalls,
            "axes": [{ "pts": [[5.0, (y0 + y1) / 2.0], [115.0, (y0 + y1) / 2.0]], "w": aisle, "sens": 2 }] }, "stalls": [7], "margin": margin, "neighbours": neighbours });
        let t = std::time::Instant::now();
        let p = &places_with(&serde_json::from_value(input).unwrap(), v).places[0];
        let ms = t.elapsed().as_millis();
        println!("{label:52} {:6} {} rep. {:2} alertes, marge mini {:.2} m, {:.1} m dont {:.1} en marche arrière, {ms} ms", p.verdict, p.entry.reprises, p.entry.alerts, p.entry.clearance, p.entry.length, p.entry.reverse);
    };
    println!("Allée de {aisle} m");
    run("3008, marge 0,10, parking plein (base)", &vl, 0.1, true, true);
    run("3008, marge 0", &vl, 0.0, true, true);
    run("3008, rangée d'en face vide", &vl, 0.1, false, true);
    run("3008, parking vide", &vl, 0.1, false, false);
    run("3008, 10,90 m lus entre murs", &vl_walls, 0.1, true, true);
    run("Neufert 4,74 × 1,76, parking plein", &neufert, 0.1, true, true);
    run("Neufert, marge 0", &neufert, 0.0, true, true);
}
