//! Exporte un scénario en IFC4 (parc en ouvrage) :
//! `cargo run --release --example ifc -- in.json out.ifc [L93|CC44|LOCAL]`
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mut site: trame::model::Site = serde_json::from_slice(&std::fs::read(&a[1]).unwrap()).unwrap();
    if site.ouvrage.is_none() {
        site.ouvrage = Some(trame::ouvrage::Ouvrage { actif: true, ..Default::default() });
    }
    let crs = a.get(3).cloned().unwrap_or_else(|| "L93".into());
    let georef = if crs == "LOCAL" {
        None
    } else {
        Some(trame::proj::Georef {
            o: [808_399.0, 5_418_700.0],
            k: (43.7f64).to_radians().cos(),
        })
    };
    let inp = trame::ouvrage::ifc::IfcIn {
        site,
        georef,
        crs,
        date: "2026-11-06T18:00:00".into(),
        projet: "exemple".into(),
    };
    let t = std::time::Instant::now();
    let out = trame::ouvrage::ifc::export(&inp);
    if let Some(e) = out.erreur {
        eprintln!("{e}");
        std::process::exit(1);
    }
    eprintln!(
        "{} éléments, {} octets, {} (EPSG:{}), {:.1} ms",
        out.elements,
        out.ifc.len(),
        out.crs,
        out.epsg,
        t.elapsed().as_secs_f64() * 1e3
    );
    std::fs::write(&a[2], out.ifc).unwrap();
}
