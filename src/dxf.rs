//! Export DXF AutoCAD 2010 (AC1024, texte UTF-8), géoréférencé en RGF93 / Lambert-93 ou CC.
//!
//! Le squelette (en-tête, tables, calques, types de ligne, blocs d'habillage) est produit par ezdxf
//! (tools/dxf_template.py) ; ce module y insère les entités : LWPOLYLINE, HATCH, TEXT, INSERT.

use crate::geom::{v2, V2};
use crate::model::{Site, P2};
use crate::occupation::{polygon_of, polys_of, PolyRings};
use crate::proj::{Georef, Lcc};
use crate::region::to_poly;
use geo::algorithm::bool_ops::{unary_union, FillRule};
use geo::{BooleanOps, MultiPolygon, OpType};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

const TEMPLATE: &str = include_str!("dxf_template.dxf");

#[derive(Deserialize, Default)]
pub struct Feat {
    pub polys: Vec<PolyRings>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub lp: Option<P2>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct CadIn {
    pub emprise: Option<[f64; 4]>,
    pub parcelles: Vec<Feat>,
    pub feuilles: Vec<Feat>,
    pub communes: Vec<Feat>,
    pub bati: Vec<Feat>,
    pub voirie: Vec<PolyRings>,
    pub vegetation: Vec<PolyRings>,
}

/// Insertion d'un bloc d'habillage, dans le repère local du plan.
#[derive(Deserialize)]
pub struct Ins {
    /// Nom du bloc (src/symbols.json).
    pub b: String,
    pub p: P2,
    /// Rotation en degrés.
    #[serde(default)]
    pub a: f64,
    #[serde(default = "one")]
    pub s: f64,
}

fn one() -> f64 {
    1.0
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct DressIn {
    pub vehicules: Vec<Ins>,
    /// Absent : les arbres du plan, en feuillus.
    pub arbres: Option<Vec<Ins>>,
}

#[derive(Deserialize)]
pub struct ExportIn {
    pub site: Site,
    #[serde(default)]
    pub georef: Option<Georef>,
    /// "L93", "CC42" à "CC50", ou "LOCAL".
    #[serde(default)]
    pub crs: String,
    /// Calques exportés éteints (masqués dans l'application).
    #[serde(default)]
    pub off: Vec<String>,
    #[serde(default)]
    pub cadastre: CadIn,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub dress: DressIn,
}

#[derive(Serialize)]
pub struct ExportOut {
    pub dxf: String,
    pub entities: usize,
    pub crs: String,
    pub epsg: u32,
}

enum Fill {
    Solid,
    /// Motif ANSI31 (hachures à 45°, pas de 3,175 × échelle), rotation supplémentaire en degrés.
    Ansi31 {
        scale: f64,
        angle: f64,
    },
}

struct Writer {
    out: String,
    next: u64,
    owner: String,
    tf: Box<dyn Fn(f64, f64) -> (f64, f64)>,
    lo: (f64, f64),
    hi: (f64, f64),
    count: usize,
}

/// Texte DXF (UTF-8 depuis AutoCAD 2007) : une valeur tient sur une ligne.
fn dxf_text(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}

fn num(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" || s.is_empty() {
        "0.0".into()
    } else if s.contains('.') {
        s.into()
    } else {
        format!("{s}.0")
    }
}

impl Writer {
    fn pair(&mut self, code: i32, val: &str) {
        let _ = write!(self.out, "{code:>3}\n{val}\n");
    }
    fn xy(&mut self, cx: i32, x: f64, y: f64) {
        self.lo = (self.lo.0.min(x), self.lo.1.min(y));
        self.hi = (self.hi.0.max(x), self.hi.1.max(y));
        self.pair(cx, &num(x));
        self.pair(cx + 10, &num(y));
    }
    fn head(&mut self, kind: &str, layer: &str) {
        let h = format!("{:X}", self.next);
        self.next += 1;
        self.count += 1;
        let owner = self.owner.clone();
        self.pair(0, kind);
        self.pair(5, &h);
        self.pair(330, &owner);
        self.pair(100, "AcDbEntity");
        self.pair(8, layer);
    }
    fn map(&self, ring: &[P2]) -> Vec<(f64, f64)> {
        ring.iter().map(|p| (self.tf)(p[0], p[1])).collect()
    }
    fn lwpoly(&mut self, layer: &str, pts: &[(f64, f64)], closed: bool) {
        if pts.len() < 2 {
            return;
        }
        self.head("LWPOLYLINE", layer);
        self.pair(100, "AcDbPolyline");
        self.pair(90, &pts.len().to_string());
        self.pair(70, if closed { "1" } else { "0" });
        for &(x, y) in pts {
            self.xy(10, x, y);
        }
    }
    fn ring(&mut self, layer: &str, ring: &[P2]) {
        let pts = self.map(ring);
        self.lwpoly(layer, &pts, true);
    }
    /// Contour limité à l'emprise (sans emprise : contour entier).
    fn ring_clipped(&mut self, layer: &str, ring: &[P2], clip: Option<[f64; 4]>) {
        let Some(r) = clip else {
            return self.ring(layer, ring);
        };
        for (part, closed) in clip_ring(ring, r) {
            let pts = self.map(&part);
            self.lwpoly(layer, &pts, closed);
        }
    }
    fn hatch(&mut self, layer: &str, loops: &[Vec<(f64, f64)>], fill: &Fill) {
        let loops: Vec<&Vec<(f64, f64)>> = loops.iter().filter(|l| l.len() >= 3).collect();
        if loops.is_empty() {
            return;
        }
        self.head("HATCH", layer);
        self.pair(100, "AcDbHatch");
        for (c, v) in [
            (10, "0.0"),
            (20, "0.0"),
            (30, "0.0"),
            (210, "0.0"),
            (220, "0.0"),
            (230, "1.0"),
        ] {
            self.pair(c, v);
        }
        let solid = matches!(fill, Fill::Solid);
        self.pair(2, if solid { "SOLID" } else { "ANSI31" });
        self.pair(70, if solid { "1" } else { "0" });
        self.pair(71, "0");
        self.pair(91, &loops.len().to_string());
        for (i, l) in loops.iter().enumerate() {
            self.pair(92, if i == 0 { "3" } else { "2" });
            self.pair(72, "0");
            self.pair(73, "1");
            self.pair(93, &l.len().to_string());
            for &(x, y) in l.iter() {
                self.pair(10, &num(x));
                self.pair(20, &num(y));
            }
            self.pair(97, "0");
        }
        self.pair(75, "1");
        self.pair(76, "1");
        if let Fill::Ansi31 { scale, angle } = fill {
            let a = (45.0 + angle).to_radians();
            let d = 3.175 * scale;
            self.pair(52, &num(*angle));
            self.pair(41, &num(*scale));
            self.pair(77, "0");
            self.pair(78, "1");
            self.pair(53, &num(45.0 + angle));
            self.pair(43, "0.0");
            self.pair(44, "0.0");
            self.pair(45, &num(-d * a.sin()));
            self.pair(46, &num(d * a.cos()));
            self.pair(79, "0");
        }
        self.pair(98, "0");
    }
    /// Polygone à trous : contour(s) puis trame.
    fn area(&mut self, layer: &str, poly: &PolyRings, fill: Option<&Fill>) {
        let loops: Vec<Vec<(f64, f64)>> = poly.iter().map(|r| self.map(r)).collect();
        if let Some(f) = fill {
            self.hatch(layer, &loops, f);
        }
        for l in &loops {
            self.lwpoly(layer, l, true);
        }
    }
    fn text(&mut self, layer: &str, x: f64, y: f64, h: f64, s: &str, centered: bool) {
        self.head("TEXT", layer);
        self.pair(100, "AcDbText");
        self.xy(10, x, y);
        self.pair(30, "0.0");
        self.pair(40, &num(h));
        self.pair(1, &dxf_text(s));
        if centered {
            self.pair(72, "1");
            self.pair(11, &num(x));
            self.pair(21, &num(y));
            self.pair(31, "0.0");
        }
        self.pair(100, "AcDbText");
        if centered {
            self.pair(73, "2");
        }
    }
    /// Bloc inséré au point `p` du plan, tourné de `deg` et mis à l'échelle `scale` dans le repère
    /// local : l'orientation et l'échelle projetées suivent la convergence des méridiens.
    fn insert(&mut self, layer: &str, block: &str, p: P2, deg: f64, scale: f64) {
        let (x, y) = (self.tf)(p[0], p[1]);
        let a = deg.to_radians();
        let (ux, uy) = (self.tf)(p[0] + a.cos(), p[1] + a.sin());
        let (dx, dy) = (ux - x, uy - y);
        let k = (dx * dx + dy * dy).sqrt();
        self.head("INSERT", layer);
        self.pair(100, "AcDbBlockReference");
        self.pair(2, block);
        self.xy(10, x, y);
        self.pair(30, "0.0");
        for c in [41, 42, 43] {
            self.pair(c, &num(scale * k));
        }
        self.pair(50, &num(dy.atan2(dx).to_degrees().rem_euclid(360.0)));
    }
}

/// Découpe d'un segment par le rectangle [x0, y0, x1, y1] (Liang-Barsky).
/// Renvoie le segment conservé et si son début, sa fin ont été coupés.
fn clip_seg(a: P2, b: P2, r: [f64; 4]) -> Option<(P2, P2, bool, bool)> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-dx, a[0] - r[0]),
        (dx, r[2] - a[0]),
        (-dy, a[1] - r[1]),
        (dy, r[3] - a[1]),
    ] {
        if p.abs() < 1e-15 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    if t0 > t1 + 1e-12 {
        return None;
    }
    let at = |t: f64| [a[0] + t * dx, a[1] + t * dy];
    Some((at(t0), at(t1), t0 > 1e-12, t1 < 1.0 - 1e-12))
}

/// Contour fermé découpé par l'emprise : soit le contour entier (fermé), soit ses portions
/// intérieures (ouvertes). Les bords de l'emprise ne sont jamais ajoutés.
fn clip_ring(ring: &[P2], r: [f64; 4]) -> Vec<(Vec<P2>, bool)> {
    let n = ring.len();
    if n < 2 {
        return vec![];
    }
    let (mut pieces, mut cur, mut whole) = (Vec::<Vec<P2>>::new(), Vec::<P2>::new(), true);
    for i in 0..n {
        match clip_seg(ring[i], ring[(i + 1) % n], r) {
            None => {
                whole = false;
                if cur.len() >= 2 {
                    pieces.push(std::mem::take(&mut cur));
                }
                cur.clear();
            }
            Some((pa, pb, cut_a, cut_b)) => {
                whole &= !(cut_a || cut_b);
                if cur.is_empty() || cut_a {
                    if cur.len() >= 2 {
                        pieces.push(std::mem::take(&mut cur));
                    }
                    cur = vec![pa];
                }
                cur.push(pb);
                if cut_b {
                    pieces.push(std::mem::take(&mut cur));
                }
            }
        }
    }
    if whole {
        return vec![(ring.to_vec(), true)];
    }
    if cur.len() >= 2 {
        // Le contour reprend au premier sommet : on recolle la dernière portion à la première.
        let same = |p: P2, q: P2| (p[0] - q[0]).abs() < 1e-9 && (p[1] - q[1]).abs() < 1e-9;
        match pieces.first() {
            Some(first) if same(first[0], ring[0]) && same(*cur.last().unwrap(), ring[0]) => {
                cur.extend_from_slice(&first[1..]);
                pieces[0] = cur;
            }
            _ => pieces.push(cur),
        }
    }
    pieces.into_iter().map(|p| (p, false)).collect()
}

fn inside(p: P2, r: [f64; 4]) -> bool {
    p[0] >= r[0] && p[0] <= r[2] && p[1] >= r[1] && p[1] <= r[3]
}

fn quad_poly(q: &[P2; 4]) -> geo::Polygon<f64> {
    to_poly(&q.iter().map(|p| v2(p[0], p[1])).collect::<Vec<V2>>())
}

/// Limite des surfaces enrobées du projet : voirie (anneaux pair-impair) ∪ rangées ∪ places ∪
/// allées transversales ∪ accès, moins les îlots plantés.
fn paved_outline(lay: &crate::model::Layout) -> Vec<PolyRings> {
    let rings = MultiPolygon::new(
        lay.paved
            .iter()
            .map(|r| to_poly(&r.iter().map(|p| v2(p[0], p[1])).collect::<Vec<_>>()))
            .collect(),
    );
    let paved = rings.boolean_op_with_fill_rule(
        &MultiPolygon::new(vec![]),
        OpType::Union,
        FillRule::EvenOdd,
    );
    let mut parts: Vec<geo::Polygon<f64>> = paved.0;
    for q in lay
        .pads
        .iter()
        .chain(lay.cross.iter())
        .chain(lay.entries.iter())
    {
        parts.push(quad_poly(q));
    }
    for s in &lay.stalls {
        parts.push(quad_poly(&s.q));
    }
    if parts.is_empty() {
        return vec![];
    }
    let mut all = unary_union(parts.iter());
    if !lay.islands.is_empty() {
        let isl: Vec<geo::Polygon<f64>> = lay.islands.iter().map(quad_poly).collect();
        all = all.difference(&unary_union(isl.iter()));
    }
    polys_of(&all, 0.5).0
}

fn fix_layers_off(tpl: &str, off: &[String]) -> String {
    let mut s = tpl.to_string();
    for name in off {
        let key = format!("  2\n{name}\n 70\n0\n 62\n");
        if let Some(i) = s.find(&key) {
            let j = i + key.len();
            let end = s[j..].find('\n').map(|e| j + e).unwrap_or(s.len());
            let color: i32 = s[j..end].trim().parse().unwrap_or(7);
            s.replace_range(j..end, &(-color.abs()).to_string());
        }
    }
    s
}

fn set_header(s: &mut String, var: &str, vals: &[(i32, String)]) {
    let key = format!("{var}\n");
    let Some(mut i) = s.find(&key) else { return };
    i += key.len();
    for (code, v) in vals {
        let tag = format!("{code:>3}\n");
        if !s[i..].starts_with(&tag) {
            return;
        }
        i += tag.len();
        let end = s[i..].find('\n').map(|e| i + e).unwrap_or(s.len());
        s.replace_range(i..end, v);
        i += v.len() + 1;
    }
}

fn template_handles(tpl: &str) -> (u64, String) {
    let seed = tpl
        .split("$HANDSEED\n  5\n")
        .nth(1)
        .and_then(|r| r.lines().next())
        .and_then(|h| u64::from_str_radix(h.trim(), 16).ok())
        .unwrap_or(0x1000);
    // Propriétaire des entités : l'enregistrement de bloc *Model_Space.
    let i = tpl.find("BLOCK_RECORD\n  5\n").unwrap_or(0);
    let mut owner = "1F".to_string();
    let mut k = i;
    while let Some(p) = tpl[k..].find("  0\nBLOCK_RECORD\n  5\n") {
        let h0 = k + p + "  0\nBLOCK_RECORD\n  5\n".len();
        let h = tpl[h0..].lines().next().unwrap_or("").to_string();
        let next = tpl[h0..].find("  0\n").map(|e| h0 + e).unwrap_or(tpl.len());
        if tpl[h0..next].contains("\n*Model_Space\n") {
            owner = h;
            break;
        }
        k = h0;
    }
    (seed, owner)
}

pub fn export(inp: &ExportIn) -> ExportOut {
    let mut site = inp.site.clone();
    site.params.quick = false;
    let lay = crate::solve(&site);

    // Système de coordonnées.
    let (crs, epsg, tf): (String, u32, Box<dyn Fn(f64, f64) -> (f64, f64)>) =
        match (inp.georef, inp.crs.as_str()) {
            (Some(g), c) if c != "LOCAL" => {
                let p = if let Some(z) = c.strip_prefix("CC").and_then(|z| z.parse::<u32>().ok()) {
                    Lcc::cc(z)
                } else {
                    Lcc::lambert93()
                };
                let (name, epsg) = (p.name.clone(), p.epsg);
                (
                    name,
                    epsg,
                    Box::new(move |x, y| {
                        let (lon, lat) = g.to_lonlat(x, y);
                        p.forward(lon, lat)
                    }),
                )
            }
            _ => (
                "Coordonnées locales (plan non géoréférencé)".into(),
                0,
                Box::new(|x, y| (x, y)),
            ),
        };

    let (seed, owner) = template_handles(TEMPLATE);
    let mut w = Writer {
        out: String::new(),
        next: seed,
        owner,
        tf,
        lo: (f64::MAX, f64::MAX),
        hi: (f64::MIN, f64::MIN),
        count: 0,
    };
    let c = &inp.cadastre;
    let clip = c.emprise;
    let bati_fill = Fill::Ansi31 {
        scale: 0.25,
        angle: 0.0,
    };
    let veg_fill = Fill::Ansi31 {
        scale: 0.5,
        angle: 90.0,
    };

    // Fond : occupation du sol, puis cadastre.
    for p in &c.voirie {
        w.area("OCC_VOIRIE", p, Some(&Fill::Solid));
    }
    for p in &c.vegetation {
        w.area("OCC_VEGETATION", p, Some(&veg_fill));
    }
    // Bâti découpé à l'emprise (surfaces), contours cadastraux limités à l'emprise (lignes).
    let rect_mp = clip.map(|[x0, y0, x1, y1]| {
        MultiPolygon::new(vec![to_poly(&[
            v2(x0, y0),
            v2(x1, y0),
            v2(x1, y1),
            v2(x0, y1),
        ])])
    });
    for f in &c.bati {
        for p in &f.polys {
            match (&rect_mp, polygon_of(p)) {
                (Some(rm), Some(poly)) => {
                    for q in polys_of(&MultiPolygon::new(vec![poly]).intersection(rm), 0.05).0 {
                        w.area("OCC_BATI", &q, Some(&bati_fill));
                    }
                }
                _ => w.area("OCC_BATI", p, Some(&bati_fill)),
            }
        }
    }
    let label_ok = |lp: P2| clip.map_or(true, |r| inside(lp, r));
    for f in &c.parcelles {
        for p in &f.polys {
            for r in p {
                w.ring_clipped("CAD_PARCELLES", r, clip);
            }
        }
        if let (Some(l), Some(lp)) = (&f.label, f.lp) {
            if label_ok(lp) {
                let (x, y) = (w.tf)(lp[0], lp[1]);
                w.text("CAD_NUMEROS", x, y, 1.2, l, true);
            }
        }
    }
    for f in &c.feuilles {
        for p in &f.polys {
            for r in p {
                w.ring_clipped("CAD_FEUILLES", r, clip);
            }
        }
        if let (Some(l), Some(lp)) = (&f.label, f.lp) {
            if label_ok(lp) {
                let (x, y) = (w.tf)(lp[0], lp[1]);
                w.text("CAD_FEUILLES", x, y, 6.0, l, true);
            }
        }
    }
    for f in &c.communes {
        for p in &f.polys {
            for r in p {
                w.ring_clipped("CAD_COMMUNES", r, clip);
            }
        }
    }

    // Projet.
    for p in paved_outline(&lay) {
        w.area("PRJ_VOIRIE", &p, None);
    }
    for e in &site.exclusions {
        w.area(
            "PRJ_SERVITUDES",
            &vec![e.clone()],
            Some(&Fill::Ansi31 {
                scale: 0.5,
                angle: 0.0,
            }),
        );
    }
    for l in &site.landscapes {
        w.area("PRJ_ESPACES_VERTS", &vec![l.clone()], Some(&veg_fill));
    }
    // Bassins de rétention : haut de talus (contour) et pied de talus.
    for (i, bs) in site.basins.iter().enumerate() {
        w.ring("PRJ_BASSIN_TALUS_HAUT", bs);
        if let Some(o) = lay.basins.get(i) {
            for r in &o.outer {
                w.ring("PRJ_ESPACES_VERTS", r);
            }
            for r in &o.bottom {
                w.ring("PRJ_BASSIN_TALUS_BAS", r);
            }
        }
    }
    for b in &site.buildings {
        w.area("PRJ_BATIMENTS", &vec![b.poly.clone()], Some(&bati_fill));
    }
    // Espaces plantés (contours à trous), puis îlots des rangées intérieures.
    for poly in &lay.planted {
        w.area("PRJ_ILOTS", poly, None);
    }
    for q in &lay.islands {
        w.ring("PRJ_ILOTS", q);
    }
    for q in &lay.noues {
        w.ring("PRJ_NOUES", q);
    }
    for s in &lay.stalls {
        let layer = match s.k {
            1 | 3 => "PRJ_PLACES_PMR",
            2 if s.e => "PRJ_PLACES_IRVE_EQ",
            2 => "PRJ_PLACES_IRVE",
            _ => "PRJ_PLACES",
        };
        if layer == "PRJ_PLACES_PMR" {
            let l = vec![w.map(&s.q)];
            w.hatch(layer, &l, &Fill::Solid);
        }
        w.ring(layer, &s.q);
    }
    // Signalisation horizontale : flèches de sens de circulation.
    for a in &lay.circulation.arrows {
        w.insert("PRJ_SIGNALISATION", "SIG_FLECHE", a.p, a.a, 1.0);
    }
    // Habillage : véhicules sous les arbres ; seuls les blocs définis dans le squelette sont insérés.
    let known = |b: &str| TEMPLATE.contains(&format!("AcDbBlockBegin\n  2\n{b}\n"));
    for v in inp.dress.vehicules.iter().filter(|v| known(&v.b)) {
        w.insert("HAB_VEHICULES", &v.b, v.p, v.a, v.s);
    }
    match &inp.dress.arbres {
        Some(list) => {
            for t in list.iter().filter(|t| known(&t.b)) {
                w.insert("HAB_ARBRES", &t.b, t.p, t.a, t.s);
            }
        }
        // Sans habillage transmis : feuillus au rayon de houppier par défaut (src/symbols.json).
        None => {
            for t in &lay.trees {
                w.insert("HAB_ARBRES", "ARB_FEUILLU", *t, 0.0, 2.5);
            }
        }
    }
    for r in &lay.setback_line {
        w.ring("PRJ_RECUL", r);
    }
    if site.parcel.len() >= 3 {
        w.ring("PRJ_PARCELLE", &site.parcel);
    }

    // Emprise et informations.
    let m = &lay.metrics;
    let mut info = vec![
        format!(
            "{crs}{}",
            if epsg > 0 {
                format!(" (EPSG:{epsg}) - unités : mètres")
            } else {
                " - unités : mètres".into()
            }
        ),
        format!(
            "Trame - {} places dont {} PMR, {} IRVE pré-équipées et {} équipées",
            m.stalls, m.pmr, m.irve, m.irve_equipped
        ),
    ];
    if let Some([x0, y0, x1, y1]) = c.emprise {
        w.ring("EMPRISE", &[[x0, y0], [x1, y0], [x1, y1], [x0, y1]]);
        info.push(format!(
            "Sources : IGN, Géoplateforme, Parcellaire Express{}",
            if inp.date.is_empty() {
                String::new()
            } else {
                format!(" - import du {}", inp.date)
            }
        ));
    }
    let (ix, iy) = (w.lo.0, w.lo.1);
    if ix.is_finite() && ix < f64::MAX {
        for (i, line) in info.iter().enumerate() {
            w.text("INFO", ix, iy - 4.0 - 3.5 * i as f64, 2.0, line, false);
        }
    }

    // Assemblage dans le squelette.
    let mut doc = fix_layers_off(TEMPLATE, &inp.off);
    set_header(&mut doc, "$HANDSEED", &[(5, format!("{:X}", w.next))]);
    if w.lo.0 < f64::MAX {
        set_header(
            &mut doc,
            "$EXTMIN",
            &[(10, num(w.lo.0)), (20, num(w.lo.1)), (30, "0.0".into())],
        );
        set_header(
            &mut doc,
            "$EXTMAX",
            &[(10, num(w.hi.0)), (20, num(w.hi.1)), (30, "0.0".into())],
        );
    }
    let marker = "  2\nENTITIES\n";
    let at = doc
        .find(marker)
        .map(|i| i + marker.len())
        .unwrap_or(doc.len());
    doc.insert_str(at, &w.out);
    let comment = format!(
        "999\n{}\n",
        dxf_text(&format!(
            "Trame - {crs}{}",
            if epsg > 0 {
                format!(" (EPSG:{epsg})")
            } else {
                String::new()
            }
        ))
    );
    doc.insert_str(0, &comment);
    ExportOut {
        dxf: doc,
        entities: w.count,
        crs,
        epsg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(georef: bool) -> ExportIn {
        let site: Site = serde_json::from_str(
            r#"{"parcel":[[0,0],[120,0],[120,80],[0,80]],"setbacks":[5,3,3,3],"entries":[{"edge":0,"t":0.5}],
                "buildings":[{"poly":[[80,40],[110,40],[110,70],[80,70]]}],
                "exclusions":[[[0,60],[40,62],[40,70],[0,68]]]}"#,
        )
        .unwrap();
        ExportIn {
            site,
            georef: if georef {
                Some(Georef {
                    o: [808_399.0, 5_418_700.0],
                    k: (43.7f64).to_radians().cos(),
                })
            } else {
                None
            },
            crs: if georef { "L93".into() } else { "LOCAL".into() },
            off: vec!["CAD_FEUILLES".into()],
            cadastre: CadIn {
                emprise: Some([-20.0, -20.0, 140.0, 100.0]),
                parcelles: vec![Feat {
                    polys: vec![vec![vec![
                        [0.0, 0.0],
                        [120.0, 0.0],
                        [120.0, 80.0],
                        [0.0, 80.0],
                    ]]],
                    label: Some("116".into()),
                    lp: Some([60.0, 40.0]),
                }],
                vegetation: vec![vec![
                    vec![[-20.0, -20.0], [0.0, -20.0], [0.0, 0.0], [-20.0, 0.0]],
                    vec![[-15.0, -15.0], [-15.0, -5.0], [-5.0, -5.0], [-5.0, -15.0]],
                ]],
                ..Default::default()
            },
            date: "23/10/2026".into(),
            dress: serde_json::from_str(
                r#"{"vehicules":[{"b":"VEH_BERLINE","p":[30,20],"a":90},{"b":"INCONNU","p":[0,0]}],
                    "arbres":[{"b":"ARB_PALMIER","p":[40,30],"a":15,"s":2.4}]}"#,
            )
            .unwrap(),
        }
    }

    #[test]
    fn structure_is_consistent() {
        let out = export(&sample(true));
        let d = &out.dxf;
        assert!(d.starts_with("999\n"));
        assert!(d.contains("$ACADVER\n  1\nAC1024\n"));
        assert!(d.trim_end().ends_with("EOF"));
        assert!(out.entities > 100, "{} entités", out.entities);
        // Calque masqué : couleur négative.
        assert!(d.contains("  2\nCAD_FEUILLES\n 70\n0\n 62\n-211\n"));
        // Toutes les poignées sont uniques et inférieures à $HANDSEED.
        // Lecture par paires (code, valeur), comme un lecteur DXF.
        let lines: Vec<&str> = d.lines().collect();
        assert_eq!(lines.len() % 2, 0, "nombre de lignes impair");
        let pairs: Vec<(&str, &str)> = lines.chunks(2).map(|c| (c[0].trim(), c[1])).collect();
        let mut handles = std::collections::HashSet::new();
        let mut max = 0u64;
        for (i, &(code, val)) in pairs.iter().enumerate() {
            assert!(
                code.parse::<i32>().is_ok(),
                "code de groupe invalide : {code:?}"
            );
            if (code == "5" || code == "105") && pairs[i - 1].1 != "$HANDSEED" {
                let h = u64::from_str_radix(val, 16).unwrap();
                assert!(handles.insert(h), "poignée en double {h:X}");
                max = max.max(h);
            }
        }
        let seed = d
            .split("$HANDSEED\n  5\n")
            .nth(1)
            .unwrap()
            .lines()
            .next()
            .unwrap();
        assert!(u64::from_str_radix(seed, 16).unwrap() > max);
        // Tous les calques utilisés existent dans la table.
        for &(code, l) in &pairs {
            if code == "8" {
                assert!(
                    l == "0" || d.contains(&format!("  2\n{l}\n 70\n")),
                    "calque absent : {l}"
                );
            }
        }
        // Coordonnées Lambert-93 plausibles pour Nice.
        let x: f64 = d
            .split("$EXTMIN\n 10\n")
            .nth(1)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(x > 1_000_000.0 && x < 1_100_000.0, "X = {x}");
        assert!(d.contains("unités : mètres"));
        // Habillage : blocs connus insérés, bloc inconnu ignoré, rotation corrigée de la
        // convergence des méridiens (Lambert-93 à Nice : environ 3,1°).
        assert_eq!(
            d.matches("  2\nVEH_BERLINE\n").count(),
            3,
            "table, bloc, insertion"
        );
        assert!(!d.contains("INCONNU"));
        // Flèches au sol : une insertion par flèche du plan.
        assert!(d.matches("  2\nSIG_FLECHE\n 10\n").count() > 4);
        let rot: f64 = d
            .split("  2\nVEH_BERLINE\n 10\n")
            .nth(1)
            .unwrap()
            .split(" 50\n")
            .nth(1)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!((rot - 93.1).abs() < 0.2, "rotation {rot}");
    }

    #[test]
    fn rings_are_clipped_to_the_selection() {
        let r = [0.0, 0.0, 10.0, 10.0];
        // Entièrement dedans : fermé.
        let inner = clip_ring(&[[1.0, 1.0], [5.0, 1.0], [5.0, 5.0]], r);
        assert_eq!(inner.len(), 1);
        assert!(inner[0].1);
        // Carré qui déborde à droite : une seule portion ouverte (le bord de l'emprise n'est pas tracé).
        let out = clip_ring(&[[5.0, 2.0], [15.0, 2.0], [15.0, 8.0], [5.0, 8.0]], r);
        assert_eq!(out.len(), 1, "{out:?}");
        assert!(!out[0].1);
        assert_eq!(
            out[0].0,
            vec![[10.0, 8.0], [5.0, 8.0], [5.0, 2.0], [10.0, 2.0]]
        );
        // Entièrement dehors : rien.
        assert!(clip_ring(&[[20.0, 20.0], [30.0, 20.0], [30.0, 30.0]], r).is_empty());
    }

    #[test]
    fn local_export_without_georeference() {
        let out = export(&sample(false));
        assert_eq!(out.epsg, 0);
        let x: f64 = out
            .dxf
            .split("$EXTMIN\n 10\n")
            .nth(1)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(x > -40.0 && x < 1.0, "X = {x}");
    }
}
