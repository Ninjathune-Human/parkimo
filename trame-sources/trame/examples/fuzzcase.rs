//! Régénère le cas aléatoire N du test de qualité : `cargo run --release --example fuzzcase -- 37`
fn main() {
    let target: usize = std::env::args().nth(1).unwrap().parse().unwrap();
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
        if case == target {
            let json = serde_json::json!({
                "parcel": parcel, "setbacks": setbacks, "entries": [{"edge": 0, "t": 0.5}],
                "buildings": if case % 3 == 0 { serde_json::json!([{"poly": [[-10,-10],[15,-10],[15,12],[-10,12]]}]) } else { serde_json::json!([]) },
                "exclusions": if case % 4 == 1 { serde_json::json!([[[-200,-6],[200,-2],[200,6],[-200,3]]]) } else { serde_json::json!([]) }
            });
            println!("{}", json);
        }
    }
}
