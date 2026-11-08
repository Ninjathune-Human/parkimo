// Audit des manœuvres d'entrée d'un plan exporté : sens du premier tronçon, tronçons, changements
// d'angle de roues à l'arrêt. Usage : cargo run --release --example audit_moves -- plan.json
use trame::giration::api::{places, PlacesIn};

fn main() {
    let path = std::env::args().nth(1).expect("chemin du JSON");
    let d: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let list: Vec<usize> = serde_json::from_value(d["left"].clone()).unwrap();
    let (mut first_rev, mut n) = (0, 0);
    for &i in &list {
        let mut inp = d["input"].clone();
        inp["stalls"] = serde_json::json!([i]);
        inp["detail"] = serde_json::json!(i);
        let inp: PlacesIn = serde_json::from_value(inp).unwrap();
        let out = places(&inp);
        let p = &out.places[0];
        let Some(g) = out.detail else { continue };
        // Entrée seule : poses jusqu'à la pose garée (première arrivée à la pose garée).
        let park = p.park;
        let end = g.poses.iter().position(|q| (q[0] - park[0]).hypot(q[1] - park[1]) < 0.02 && (q[2] - park[2]).abs() < 0.01).unwrap_or(g.poses.len() - 1);
        let poses = &g.poses[..=end];
        let mut moves: Vec<(bool, f64)> = Vec::new();
        let mut steer_changes = 0;
        for w in poses.windows(2) {
            let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
            let ds = dx.hypot(dy);
            if (w[1][3] - w[0][3]).abs() > 1e-6 { steer_changes += 1; }
            if ds < 1e-6 { continue; }
            let rev = dx * w[0][2].cos() + dy * w[0][2].sin() < 0.0;
            match moves.last_mut() { Some(m) if m.0 == rev => m.1 += ds, _ => moves.push((rev, ds)) }
        }
        n += 1;
        if moves.first().is_some_and(|m| m.0) { first_rev += 1; }
        let txt: Vec<String> = moves.iter().map(|(r, l)| format!("{}{:.1}", if *r { "AR " } else { "AV " }, l)).collect();
        println!("place {i:3} {:6} {} rep. : [{}]  roues tournées {} fois", p.verdict, p.entry.reprises, txt.join(", "), steer_changes);
    }
    println!("{first_rev} entrées sur {n} commencent en marche arrière ; braquage à l'arrêt total : voir détail");
}
