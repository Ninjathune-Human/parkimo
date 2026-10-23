//! Exporte un scénario en DXF : `cargo run --release --example dxf -- in.json out.dxf [L93|CC44|LOCAL]`
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let site: trame::model::Site = serde_json::from_slice(&std::fs::read(&a[1]).unwrap()).unwrap();
    let crs = a.get(3).cloned().unwrap_or_else(|| "L93".into());
    let georef = if crs == "LOCAL" {
        None
    } else {
        Some(trame::proj::Georef {
            o: [808_399.0, 5_418_700.0],
            k: (43.7f64).to_radians().cos(),
        })
    };
    let inp = trame::dxf::ExportIn {
        site,
        georef,
        crs,
        off: vec![],
        cadastre: Default::default(),
        date: String::new(),
    };
    let t = std::time::Instant::now();
    let out = trame::dxf::export(&inp);
    eprintln!(
        "{} entités, {} octets, {} (EPSG:{}), {:.1} ms",
        out.entities,
        out.dxf.len(),
        out.crs,
        out.epsg,
        t.elapsed().as_secs_f64() * 1e3
    );
    std::fs::write(&a[2], out.dxf).unwrap();
}
