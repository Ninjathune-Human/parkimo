//! Occupation du sol dans l'emprise importée.
//!
//! Surfaces définies : bâti (cadastre) et voirie (domaine non cadastré : l'emprise moins les
//! parcelles). Tout le reste de l'emprise est de la végétation. Résultat en polygones à trous.

use crate::geom::{clean_ring, signed_area, v2, V2};
use crate::model::P2;
use crate::region::{erode, to_poly};
use geo::algorithm::bool_ops::unary_union;
use geo::algorithm::buffer::{BufferStyle, LineJoin};
use geo::{BooleanOps, Buffer, Coord, LineString, MultiPolygon, Polygon};
use serde::{Deserialize, Serialize};

/// Polygone : premier anneau extérieur, suivants trous.
pub type PolyRings = Vec<Vec<P2>>;

#[derive(Deserialize)]
pub struct OccIn {
    /// Emprise [x0, y0, x1, y1] en mètres, repère local.
    pub rect: [f64; 4],
    pub parcels: Vec<PolyRings>,
    pub bati: Vec<PolyRings>,
}

#[derive(Serialize, Default)]
pub struct OccOut {
    pub voirie: Vec<PolyRings>,
    pub vegetation: Vec<PolyRings>,
    pub voirie_area: f64,
    pub vegetation_area: f64,
    pub bati_area: f64,
}

fn ring_ls(r: &[P2]) -> Option<LineString<f64>> {
    let pts = clean_ring(&r.iter().map(|p| v2(p[0], p[1])).collect::<Vec<_>>());
    if pts.len() < 3 || signed_area(&pts).abs() < 1e-6 {
        return None;
    }
    let mut cs: Vec<Coord<f64>> = pts.iter().map(|p| Coord { x: p.x, y: p.y }).collect();
    cs.push(cs[0]);
    Some(LineString::new(cs))
}

pub fn polygon_of(rings: &PolyRings) -> Option<Polygon<f64>> {
    let mut it = rings.iter();
    let ext = ring_ls(it.next()?)?;
    let holes: Vec<LineString<f64>> = it.filter_map(|r| ring_ls(r)).collect();
    let mut p = Polygon::new(ext, holes);
    use geo::orient::{Direction, Orient};
    p = p.orient(Direction::Default);
    Some(p)
}

fn union_of(polys: &[PolyRings]) -> MultiPolygon<f64> {
    let ps: Vec<Polygon<f64>> = polys.iter().filter_map(polygon_of).collect();
    if ps.is_empty() {
        return MultiPolygon::new(vec![]);
    }
    unary_union(ps.iter())
}

fn ring_pts(ls: &LineString<f64>) -> Vec<V2> {
    clean_ring(&ls.0.iter().map(|c| v2(c.x, c.y)).collect::<Vec<_>>())
}

fn r2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Polygones à trous, extérieurs CCW et trous CW, sans les miettes.
pub fn polys_of(mp: &MultiPolygon<f64>, min_area: f64) -> (Vec<PolyRings>, f64) {
    let mut out = Vec::new();
    let mut total = 0.0;
    for p in &mp.0 {
        let mut ext = ring_pts(p.exterior());
        if ext.len() < 3 {
            continue;
        }
        if signed_area(&ext) < 0.0 {
            ext.reverse();
        }
        let mut area = signed_area(&ext);
        let mut rings = vec![ext];
        for h in p.interiors() {
            let mut r = ring_pts(h);
            if r.len() < 3 {
                continue;
            }
            if signed_area(&r) > 0.0 {
                r.reverse();
            }
            area += signed_area(&r);
            rings.push(r);
        }
        if area < min_area {
            continue;
        }
        total += area;
        out.push(
            rings
                .iter()
                .map(|r| r.iter().map(|p| [r2(p.x), r2(p.y)]).collect())
                .collect(),
        );
    }
    (out, total)
}

pub fn occupation(inp: &OccIn) -> OccOut {
    let [x0, y0, x1, y1] = inp.rect;
    if !(x1 > x0 && y1 > y0) {
        return OccOut::default();
    }
    let rect = MultiPolygon::new(vec![to_poly(&[
        v2(x0, y0),
        v2(x1, y0),
        v2(x1, y1),
        v2(x0, y1),
    ])]);

    // Parcelles réunies ; les interstices de quelques centimètres entre parcelles voisines
    // (défauts de jointure du plan cadastral) sont refermés par une fermeture de 5 cm.
    let parcels = union_of(&inp.parcels);
    let closed = if parcels.0.is_empty() {
        parcels
    } else {
        let grown =
            parcels.buffer_with_style(BufferStyle::new(0.05).line_join(LineJoin::Miter(0.2)));
        erode(&grown, 0.05)
    };
    let bati = union_of(&inp.bati).intersection(&rect);
    let parcels_in = closed.intersection(&rect);

    let voirie = rect.difference(&parcels_in).difference(&bati);
    let vegetation = parcels_in.difference(&bati);

    let (voirie, voirie_area) = polys_of(&voirie, 2.0);
    let (vegetation, vegetation_area) = polys_of(&vegetation, 1.0);
    let (_, bati_area) = polys_of(&bati, 0.0);
    OccOut {
        voirie,
        vegetation,
        voirie_area,
        vegetation_area,
        bati_area,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sq(x0: f64, y0: f64, x1: f64, y1: f64) -> PolyRings {
        vec![vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]]
    }

    #[test]
    fn surfaces_partition_the_selection() {
        // Emprise 100 × 60 ; deux îlots de parcelles séparés par une rue de 10 m ;
        // un micro-interstice de 3 cm entre deux parcelles ; un bâtiment à cour intérieure.
        let parcels = vec![
            sq(0.0, 0.0, 45.0, 60.0),
            sq(55.0, 0.0, 80.0, 60.0),
            sq(80.03, 0.0, 100.0, 60.0),
        ];
        let court = vec![
            vec![[10.0, 10.0], [30.0, 10.0], [30.0, 30.0], [10.0, 30.0]],
            vec![[15.0, 15.0], [15.0, 25.0], [25.0, 25.0], [25.0, 15.0]],
        ];
        let out = occupation(&OccIn {
            rect: [0.0, 0.0, 100.0, 60.0],
            parcels,
            bati: vec![court],
        });
        assert!(
            (out.voirie_area - 600.0).abs() < 1.0,
            "voirie {}",
            out.voirie_area
        );
        assert!(
            (out.bati_area - 300.0).abs() < 0.5,
            "bâti {}",
            out.bati_area
        );
        assert!(
            (out.vegetation_area - (6000.0 - 600.0 - 300.0)).abs() < 1.0,
            "végétation {}",
            out.vegetation_area
        );
        assert_eq!(
            out.voirie.len(),
            1,
            "l'interstice de 3 cm ne doit pas devenir de la voirie"
        );
        // La cour du bâtiment est de la végétation : un polygone de végétation de 100 m².
        assert!(out.vegetation.iter().any(|p| p.len() == 1
            && (signed_area(&p[0].iter().map(|q| v2(q[0], q[1])).collect::<Vec<_>>()) - 100.0)
                .abs()
                < 0.5));
    }

    #[test]
    fn empty_cadastre_is_all_road() {
        let out = occupation(&OccIn {
            rect: [0.0, 0.0, 10.0, 10.0],
            parcels: vec![],
            bati: vec![],
        });
        assert!((out.voirie_area - 100.0).abs() < 1e-6 && out.vegetation.is_empty());
    }
}
