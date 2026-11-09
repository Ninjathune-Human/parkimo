// Audit d'un plan exporté de l'application (entrée du test des places, JSON) : verdicts d'une liste
// de places sous plusieurs hypothèses, et obstacles en cause pour la première.
// Usage : cargo run --release --example audit_plan -- plan.json
use std::collections::BTreeMap;
use trame::giration::api::{places_with, PlacesIn};
use trame::giration::vehicle::{Convention, Vehicle};

fn main() {
    let path = std::env::args().nth(1).expect("chemin du JSON");
    let d: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let list: Vec<usize> = serde_json::from_value(d["left"].clone()).unwrap();
    let neufert = Vehicle { wheelbase: 2.70, front_overhang: 0.94, rear_overhang: 1.10, width: 1.76, track: 1.50, tyre_width: 0.205, ground_clearance: 0.14, max_steer: 0.0, lock_to_lock: 4.0 }
        .with_turning_diameter(11.70, Convention::Walls);
    let vl = Vehicle::peugeot_3008();
    let vl_walls = Vehicle::peugeot_3008().with_turning_diameter(10.90, Convention::Walls);
    println!("R essieu arrière : 3008 lu entre trottoirs {:.2} m, lu entre murs {:.2} m ; Neufert {:.2} m", vl.min_radius(), vl_walls.min_radius(), neufert.min_radius());
    let run = |label: &str, v: &Vehicle, margin: f64, detail: bool| {
        let mut inp = d["input"].clone();
        inp["stalls"] = serde_json::json!(list);
        inp["margin"] = serde_json::json!(margin);
        if detail { inp["detail"] = serde_json::json!(list[0]); }
        let inp: PlacesIn = serde_json::from_value(inp).unwrap();
        let out = places_with(&inp, v);
        let mut c: BTreeMap<&str, usize> = BTreeMap::new();
        for p in &out.places { *c.entry(p.verdict).or_default() += 1; }
        let alerts = out.places.iter().filter(|p| p.entry.alerts + p.exit.alerts > 0).count();
        let many = out.places.iter().filter(|p| p.entry.reprises.max(p.exit.reprises) > 3).count();
        println!("{label:40} {c:?}  alerte de marge {alerts}, plus de 3 reprises {many}");
        if let Some(g) = out.detail {
            for k in g.conflicts.iter().filter(|k| k.severity == "alerte") {
                println!("    place {} : {} {} contre « {} » en ({:.2}, {:.2})", list[0], k.part, k.severity, k.obstacle, k.point[0], k.point[1]);
            }
        }
    };
    // Alerte de chaque place : obstacle et point le plus en cause.
    for &i in &list {
        let mut inp = d["input"].clone();
        inp["stalls"] = serde_json::json!([i]);
        inp["detail"] = serde_json::json!(i);
        let out = places_with(&serde_json::from_value(inp).unwrap(), &vl);
        let p = &out.places[0];
        let q = &d["input"]["layout"]["stalls"][i];
        let a = out.detail.as_ref().and_then(|g| g.conflicts.iter().find(|k| k.severity == "alerte").map(|k| format!("{} ({:.2}, {:.2})", k.obstacle, k.point[0], k.point[1])));
        println!("place {i:3} y {:.1} : {} {} rep., marge mini {:.2} m, alerte {:?}", q[0][1].as_f64().unwrap(), p.verdict, p.entry.reprises, p.entry.clearance, a);
    }
    run("3008, 10,90 m lus entre trottoirs", &vl, 0.1, false);
    run("3008, 10,90 m lus entre murs", &vl_walls, 0.1, false);
    run("3008, marge 0", &vl, 0.0, false);
    run("3008 entre murs, marge 0", &vl_walls, 0.0, false);
    run("Neufert 4,74 × 1,76", &neufert, 0.1, false);
}
