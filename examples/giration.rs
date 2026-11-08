//! Rendu SVG de contrôle : la VL 2026 sort d'une ligne droite et braque à fond à 5 km/h.
//! Les traces simulées doivent rejoindre les cercles analytiques (en pointillés) une fois
//! le volant en butée.
//!
//! cargo run --release --example giration -- sortie.svg

use std::fmt::Write as _;
use trame::giration::kinematics::{body_corners, drive, wheels, Pose, Profile, State};
use trame::giration::vehicle::Vehicle;

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "giration.svg".into());
    let v = Vehicle::vl2026();
    let r = v.min_radius();
    let start = State {
        pose: Pose {
            x: 0.0,
            y: 0.0,
            heading: std::f64::consts::FRAC_PI_2,
        },
        steer: 0.0,
    };
    let mut path = drive(&v, Profile::Habile, start, 5.0 / 3.6, 3.0, 0.0, 0.02);
    let turn = drive(
        &v,
        Profile::Habile,
        *path.last().unwrap(),
        5.0 / 3.6,
        1.5 * std::f64::consts::PI * r,
        v.max_steer,
        0.02,
    );
    path.extend_from_slice(&turn[1..]);

    let end = path.last().unwrap().pose;
    let o = [end.x - r * end.heading.sin(), end.y + r * end.heading.cos()];

    // Trois points, chacun comparé à son cercle analytique : coin avant extérieur de caisse,
    // bord extérieur du pneu avant extérieur, flanc intérieur au droit de l'essieu arrière.
    let tyre_edge = |s: &State| {
        let w = wheels(&v, s)[1];
        let e = v.tyre_width / 2.0;
        [
            w.centre[0] + e * w.heading.sin(),
            w.centre[1] - e * w.heading.cos(),
        ]
    };
    let inner_flank = |p: &Pose| {
        let e = v.width / 2.0;
        [p.x - e * p.heading.sin(), p.y + e * p.heading.cos()]
    };
    let traces: Vec<(&str, Vec<[f64; 2]>)> = vec![
        (
            "#D85A30",
            path.iter().map(|s| body_corners(&v, &s.pose)[1]).collect(),
        ),
        ("#1D9E75", path.iter().map(tyre_edge).collect()),
        (
            "#888780",
            path.iter().map(|s| inner_flank(&s.pose)).collect(),
        ),
    ];

    let pad = v.wall_radius(r) + 1.0;
    let (x0, x1, y0, y1) = (o[0] - pad, o[0] + pad, o[1] - pad, (o[1] + pad).max(0.0));
    let (x0, x1, y0) = (x0.min(-2.0), x1.max(2.0), y0.min(-2.0));
    let k = 30.0;
    let px = |p: [f64; 2]| ((p[0] - x0) * k, (y1 - p[1]) * k);
    let (w, h) = ((x1 - x0) * k, (y1 - y0) * k + 70.0);

    let mut svg = String::new();
    let _ = write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w:.0} {h:.0}" font-family="sans-serif" font-size="13"><rect width="100%" height="100%" fill="white"/>"#
    );
    let (cx, cy) = px(o);
    for (rad, colour) in [
        (v.wall_radius(r), "#D85A30"),
        (v.kerb_radius(r), "#1D9E75"),
        (v.inner_radius(r), "#888780"),
    ] {
        let _ = write!(
            svg,
            r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="{:.1}" fill="none" stroke="{colour}" stroke-width="1" stroke-dasharray="5 5"/>"#,
            rad * k
        );
    }
    for (i, s) in path.iter().enumerate().filter(|(i, _)| i % 120 == 0) {
        let c = body_corners(&v, &s.pose);
        let pts: Vec<String> = c
            .iter()
            .map(|p| {
                let (a, b) = px(*p);
                format!("{a:.1},{b:.1}")
            })
            .collect();
        let op = if i == 0 { 0.9 } else { 0.35 };
        let _ = write!(
            svg,
            r##"<polygon points="{}" fill="none" stroke="#444441" stroke-opacity="{op}" stroke-width="1"/>"##,
            pts.join(" ")
        );
    }
    for (colour, t) in &traces {
        let d: Vec<String> = t
            .iter()
            .map(|p| {
                let (a, b) = px(*p);
                format!("{a:.1},{b:.1}")
            })
            .collect();
        let _ = write!(
            svg,
            r#"<polyline points="{}" fill="none" stroke="{colour}" stroke-width="2"/>"#,
            d.join(" ")
        );
    }
    let _ = write!(
        svg,
        r##"<circle cx="{cx:.1}" cy="{cy:.1}" r="3" fill="#2C2C2A"/>"##
    );
    let ly = h - 44.0;
    let fr = |x: f64| format!("{x:.2}").replace('.', ",");
    let legend = [
        (
            "#D85A30",
            format!(
                "coin avant extérieur, cercle entre murs {} m",
                fr(2.0 * v.wall_radius(r))
            ),
        ),
        (
            "#1D9E75",
            format!(
                "pneu avant extérieur, cercle entre trottoirs {} m",
                fr(2.0 * v.kerb_radius(r))
            ),
        ),
        (
            "#888780",
            format!(
                "flanc intérieur de caisse, rayon {} m",
                fr(v.inner_radius(r))
            ),
        ),
    ];
    for (i, (colour, text)) in legend.iter().enumerate() {
        let y = ly + 18.0 * i as f64;
        let _ = write!(
            svg,
            r##"<line x1="12" y1="{:.1}" x2="36" y2="{:.1}" stroke="{colour}" stroke-width="2"/><text x="44" y="{:.1}" fill="#2C2C2A">{text}</text>"##,
            y - 4.0,
            y - 4.0,
            y
        );
    }
    svg.push_str("</svg>");
    std::fs::write(&out, svg).expect("écriture du SVG");
    println!(
        "R = {:.3} m, angle {:.1}°, entre murs {:.3} m -> {out}",
        r,
        v.max_steer.to_degrees(),
        2.0 * v.wall_radius(r)
    );
}
