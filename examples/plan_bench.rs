// Test de toutes les places d'un plan exporté de l'application (entrée du test des places, JSON) :
// répartition des verdicts, reprises, durée par place. Usage : cargo run --release --example plan_bench -- plan.json
use std::collections::BTreeMap;
use trame::giration::api::{places, PlacesIn};

fn main() {
    let path = std::env::args().nth(1).expect("chemin du JSON");
    let d: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let inp: PlacesIn = serde_json::from_value(d["input"].clone()).unwrap();
    let t = std::time::Instant::now();
    let out = places(&inp);
    let ms = t.elapsed().as_millis() as f64;
    let mut c: BTreeMap<&str, usize> = BTreeMap::new();
    let mut rep: BTreeMap<usize, usize> = BTreeMap::new();
    for p in &out.places {
        *c.entry(p.verdict).or_default() += 1;
        *rep.entry(p.entry.reprises.max(p.exit.reprises)).or_default() += 1;
    }
    println!("{} places en {:.1} s ({:.0} ms par place) : {c:?} ; reprises {rep:?}", out.places.len(), ms / 1000.0, ms / out.places.len() as f64);
}
