//! Construction des zones : parcelle, reculs par arête, servitudes, bâtiments,
//! zone constructible B, bande périphérique et cœur (booléens robustes via `geo`).

use crate::geom::{clean_ring, signed_area, v2, V2};
use crate::model::Site;
use geo::algorithm::bool_ops::unary_union;
use geo::algorithm::buffer::{BufferStyle, LineCap, LineJoin};
use geo::{BooleanOps, Buffer, Coord, Line, LineString, MultiPolygon, Polygon};

const ROUND: f64 = 0.25;

/// Polygone `geo` toujours orienté CCW : `unary_union` déduit l'orientation des pleins
/// du premier anneau, un anneau CW serait lu comme un trou.
pub fn to_poly(ring: &[V2]) -> Polygon<f64> {
    let mut cs: Vec<Coord<f64>> = ring.iter().map(|p| Coord { x: p.x, y: p.y }).collect();
    if signed_area(ring) < 0.0 {
        cs.reverse();
    }
    if let Some(&f) = cs.first() {
        cs.push(f);
    }
    Polygon::new(LineString::new(cs), vec![])
}

fn ring_of(ls: &LineString<f64>) -> Vec<V2> {
    clean_ring(&ls.0.iter().map(|c| v2(c.x, c.y)).collect::<Vec<_>>())
}

/// Anneaux orientés intérieur-à-gauche : extérieurs CCW, trous CW. Ignore les miettes.
pub fn rings_of(mp: &MultiPolygon<f64>, min_area: f64) -> Vec<Vec<V2>> {
    let mut out = Vec::new();
    for poly in &mp.0 {
        let mut ext = ring_of(poly.exterior());
        if ext.len() < 3 || signed_area(&ext).abs() < min_area {
            continue;
        }
        if signed_area(&ext) < 0.0 {
            ext.reverse();
        }
        out.push(ext);
        for h in poly.interiors() {
            let mut r = ring_of(h);
            if r.len() < 3 || signed_area(&r).abs() < 1e-3 {
                continue;
            }
            if signed_area(&r) > 0.0 {
                r.reverse();
            }
            out.push(r);
        }
    }
    out
}

pub fn mp_area(mp: &MultiPolygon<f64>) -> f64 {
    rings_of(mp, 0.0).iter().map(|r| signed_area(r)).sum()
}

fn drop_small(mp: MultiPolygon<f64>, min_area: f64) -> MultiPolygon<f64> {
    MultiPolygon::new(
        mp.0.into_iter()
            .filter(|p| {
                let r = ring_of(p.exterior());
                signed_area(&r).abs() >= min_area
            })
            .collect(),
    )
}

pub fn grow(mp: &MultiPolygon<f64>, d: f64) -> MultiPolygon<f64> {
    if d <= 0.0 {
        return mp.clone();
    }
    mp.buffer_with_style(BufferStyle::new(d).line_join(LineJoin::Round(ROUND)))
}

pub fn erode(mp: &MultiPolygon<f64>, d: f64) -> MultiPolygon<f64> {
    if d <= 0.0 {
        return mp.clone();
    }
    mp.buffer_with_style(BufferStyle::new(-d).line_join(LineJoin::Round(ROUND)))
}

/// Ouverture morphologique : retire les parties plus étroites que 2r (pointes, effilements)
/// et arrondit les angles saillants au rayon r.
pub fn open(mp: &MultiPolygon<f64>, r: f64) -> MultiPolygon<f64> {
    let e = erode(mp, r);
    if e.0.is_empty() {
        return e;
    }
    e.buffer_with_style(BufferStyle::new(r).line_join(LineJoin::Round(ROUND)))
        .intersection(mp)
}

pub struct Zones {
    /// Parcelle et son signe d'orientation (+1 CCW, -1 CW).
    pub parcel: Vec<V2>,
    pub parcel_sign: f64,
    /// Parcelle moins reculs.
    pub setback: MultiPolygon<f64>,
    /// Zone stationnable B.
    pub b: MultiPolygon<f64>,
    /// B érodée de la profondeur de place : circulation + cœur.
    pub band: MultiPolygon<f64>,
    /// Cœur : B érodée de (place + allée), accueille les rangées doubles.
    pub core: MultiPolygon<f64>,
    pub buildings_area: f64,
}

pub fn build(site: &Site) -> Option<Zones> {
    let p = &site.params;
    let parcel = clean_ring(
        &site
            .parcel
            .iter()
            .map(|q| v2(q[0], q[1]))
            .collect::<Vec<_>>(),
    );
    if parcel.len() < 3 || signed_area(&parcel).abs() < 10.0 {
        return None;
    }
    let parcel_sign = signed_area(&parcel).signum();
    let parcel_poly = MultiPolygon::new(vec![to_poly(&parcel)]);

    // Reculs par arête : on retire, pour chaque arête, les points à moins de s_i de cette arête.
    let n = parcel.len();
    let mut strips: Vec<MultiPolygon<f64>> = Vec::new();
    for i in 0..n {
        let s = site.setbacks.get(i).copied().unwrap_or(0.0);
        if s > 0.01 {
            let a = parcel[i];
            let b = parcel[(i + 1) % n];
            let line = Line::new(Coord { x: a.x, y: a.y }, Coord { x: b.x, y: b.y });
            strips.push(
                line.buffer_with_style(
                    BufferStyle::new(s)
                        .line_cap(LineCap::Round(ROUND))
                        .line_join(LineJoin::Round(ROUND)),
                ),
            );
        }
    }
    let setback = if strips.is_empty() {
        parcel_poly.clone()
    } else {
        parcel_poly.difference(&unary_union(strips.iter()))
    };

    // Obstacles : servitudes, bâtiments + trottoir, espaces verts.
    let mut obstacles: Vec<MultiPolygon<f64>> = Vec::new();
    for e in &site.exclusions {
        let r = clean_ring(&e.iter().map(|q| v2(q[0], q[1])).collect::<Vec<_>>());
        if r.len() >= 3 {
            obstacles.push(MultiPolygon::new(vec![to_poly(&r)]));
        }
    }
    let mut buildings_area = 0.0;
    for bd in &site.buildings {
        let r = clean_ring(&bd.poly.iter().map(|q| v2(q[0], q[1])).collect::<Vec<_>>());
        if r.len() < 3 {
            continue;
        }
        buildings_area += signed_area(&r).abs();
        let poly = MultiPolygon::new(vec![to_poly(&r)]);
        if p.building_margin > 0.01 {
            obstacles.push(poly.buffer_with_style(
                BufferStyle::new(p.building_margin).line_join(LineJoin::Miter(0.5)),
            ));
        } else {
            obstacles.push(poly);
        }
    }
    for l in &site.basins {
        let r = clean_ring(&l.iter().map(|q| v2(q[0], q[1])).collect::<Vec<_>>());
        if r.len() >= 3 {
            obstacles.push(grow(
                &MultiPolygon::new(vec![to_poly(&r)]),
                p.basin_margin.clamp(0.0, 10.0),
            ));
        }
    }
    for l in &site.landscapes {
        let r = clean_ring(&l.iter().map(|q| v2(q[0], q[1])).collect::<Vec<_>>());
        if r.len() >= 3 {
            obstacles.push(MultiPolygon::new(vec![to_poly(&r)]));
        }
    }
    let b = if obstacles.is_empty() {
        setback.clone()
    } else {
        setback.difference(&unary_union(obstacles.iter()))
    };
    let b = drop_small(b, 15.0);

    let d = if p.perimeter { p.stall_d } else { 0.0 };
    let band = drop_small(erode(&b, d), 1.0);
    let core = drop_small(erode(&band, p.aisle), 1.0);

    Some(Zones {
        parcel,
        parcel_sign,
        setback,
        b,
        band,
        core,
        buildings_area,
    })
}
