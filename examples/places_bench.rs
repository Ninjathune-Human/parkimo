// Mesure : test de toutes les places des scénarios, temps par place et répartition des verdicts.
use std::time::Instant;
fn main() {
    for name in std::env::args().skip(1) {
        let path = format!("{}/scenarios/{name}.json", env!("CARGO_MANIFEST_DIR"));
        let mut raw: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        if let Ok(a) = std::env::var("AISLE") { raw["params"]["aisle"] = serde_json::json!(a.parse::<f64>().unwrap()); }
        let site: parkimo::model::Site = serde_json::from_value(raw.clone()).unwrap();
        let lay = parkimo::solve(&site);
        let layout = serde_json::json!({
            "paved": lay.paved, "pads": lay.pads, "islands": lay.islands, "entries": lay.entries, "cross": lay.cross,
            "stalls": lay.stalls.iter().map(|s| s.q).collect::<Vec<_>>(),
            "axes": lay.circulation.axes.iter().map(|a| serde_json::json!({ "pts": a.pts, "w": a.w, "sens": a.sens })).collect::<Vec<_>>(),
        });
        let inp: parkimo::giration::api::PlacesIn = serde_json::from_value(serde_json::json!({ "site": raw, "layout": layout })).unwrap();
        let t0 = Instant::now();
        let out = parkimo::giration::api::places(&inp);
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        let n = out.places.len();
        let count = |v: &str| out.places.iter().filter(|p| p.verdict == v).count();
        let reasons: std::collections::BTreeMap<&str, usize> = out.places.iter().filter_map(|p| p.reason).fold(Default::default(), |mut m, r| { *m.entry(r).or_default() += 1; m });
        println!("{name}: {n} places, {ms:.0} ms ({:.1} ms/place) vert {} bleu {} orange {} rouge {} gris {} {:?}", ms / n as f64, count("vert"), count("bleu"), count("orange"), count("rouge"), count("gris"), reasons);
    }
}
