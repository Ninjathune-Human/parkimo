//! Trame : moteur d'étude de capacité de stationnement.
//!
//! Pipeline : parcelle → reculs par arête → servitudes, bâtiments, espaces verts → zone B
//! → places périphériques (bords droits et courbes) → anneau de circulation → rangées doubles
//! optimisées (orientation × décalage) → accès, connectivité, PMR, IRVE → métriques.

pub mod circulation;
pub mod dxf;
pub mod finish;
pub mod geom;
pub mod layout;
pub mod model;
pub mod occupation;
pub mod perimeter;
pub mod planted;
pub mod proj;
pub mod region;
pub mod rows;

use geo::{Area, BooleanOps, MultiPolygon};
use geom::{signed_area, v2, Quad, Region, V2};
use layout::{Acc, KIND_IRVE, KIND_PMR, KIND_PMR_IRVE, KIND_STD};
use model::{BasinOut, Layout, Metrics, Site, StallOut, P2};
use std::cell::RefCell;

fn r3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}
fn p2(p: V2) -> P2 {
    [r3(p.x), r3(p.y)]
}
fn q4(q: &Quad) -> [P2; 4] {
    [p2(q.0[0]), p2(q.0[1]), p2(q.0[2]), p2(q.0[3])]
}
fn quad_area(q: &Quad) -> f64 {
    signed_area(&q.0).abs()
}

/// Rectangles d'accès : de la limite de parcelle jusqu'à l'anneau de circulation.
fn entry_quads(site: &Site, parcel: &[V2], sign: f64) -> Vec<Quad> {
    let p = &site.params;
    let n = parcel.len();
    site.entries
        .iter()
        .filter(|e| e.edge < n)
        .map(|e| {
            let a = parcel[e.edge];
            let b = parcel[(e.edge + 1) % n];
            let u = (b - a).norm();
            let nn = u.left() * sign;
            let t = e.t.clamp(0.0, 1.0);
            let c = a + (b - a) * t;
            let s = site.setbacks.get(e.edge).copied().unwrap_or(0.0);
            let depth = s + if p.perimeter { p.stall_d } else { 0.0 } + p.aisle * 0.6;
            let hw = p.entry_w.clamp(2.5, 15.0) * 0.5;
            Quad::on_base(c - u * hw, c + u * hw, nn, depth)
        })
        .collect()
}

/// Cheminements devant les portes : largeur de la porte, du mur jusqu'à la chaussée.
fn door_walks(site: &Site) -> Vec<Quad> {
    let p = &site.params;
    let depth = p.building_margin + if p.perimeter { p.stall_d } else { 0.0 };
    site.buildings
        .iter()
        .filter_map(|b| b.door)
        .filter(|d| depth > 0.01 && d.w > 0.1)
        .map(|d| {
            let n = v2(d.n[0], d.n[1]).norm();
            let u = n.left();
            let c = v2(d.p[0], d.p[1]);
            Quad::on_base(c - u * (d.w * 0.5), c + u * (d.w * 0.5), n, depth)
        })
        .collect()
}

/// Rayon de giration en entrée de parking (m), de part et d'autre de chaque accès.
pub const ENTRY_RADIUS: f64 = 3.0;

/// Entrée de parking : là où l'accès débouche sur l'allée (front des places en bordure), chaque
/// côté reçoit un arrondi de rayon `ENTRY_RADIUS`. Renvoie, par côté, le carré R × R laissé libre
/// de places (les places sont repoussées au-delà) et la surface gagnée par la chaussée (carré moins
/// le quart de disque centré à l'opposé de l'angle).
pub fn entry_turns(site: &Site, parcel: &[V2], entries: &[Quad]) -> (Vec<Quad>, Vec<Vec<V2>>) {
    let p = &site.params;
    let r = ENTRY_RADIUS;
    let n = parcel.len();
    let (mut keep, mut fillets) = (Vec::new(), Vec::new());
    for (e, q) in site.entries.iter().filter(|e| e.edge < n).zip(entries) {
        let (a, b) = (q.0[0], q.0[1]);
        let u = (b - a).norm();
        let nn = (q.0[3] - q.0[0]).norm();
        let s = site.setbacks.get(e.edge).copied().unwrap_or(0.0);
        let depth = s + if p.perimeter { p.stall_d } else { 0.0 };
        for (corner, side) in [(a, -u), (b, u)] {
            let k = corner + nn * depth;
            keep.push(Quad([k, k + side * r, k + side * r - nn * r, k - nn * r]));
            // Arc de centre o, du front de l'allée (direction nn depuis o) au bord de l'accès (−side).
            let o = k + side * r - nn * r;
            let (t1, t2) = (nn.y.atan2(nn.x), (-side).y.atan2(-side.x));
            let mut dt = t2 - t1;
            while dt > std::f64::consts::PI {
                dt -= std::f64::consts::TAU;
            }
            while dt < -std::f64::consts::PI {
                dt += std::f64::consts::TAU;
            }
            let mut ring = vec![k];
            for i in 0..=8 {
                let th = t1 + dt * i as f64 / 8.0;
                ring.push(o + v2(th.cos(), th.sin()) * r);
            }
            fillets.push(ring);
        }
    }
    (keep, fillets)
}

/// Côtés de l'anneau passés à sens unique. Pour chaque point d'une voie de l'anneau, le côté du
/// cœur le plus proche, s'il est parallèle à la voie : perpendiculaire aux rangées, le cœur y
/// avance de `aisle - one` (les rangées s'allongent) ; parallèle aux rangées et en bord de trame,
/// le cœur avance et la trame s'y cale (`Pin` : point de la limite avancée, normale vers
/// l'anneau). Un côté oblique, ou parallèle mais intérieur, garde sa largeur.
fn narrow_ring(
    band: &MultiPolygon<f64>,
    core: &MultiPolygon<f64>,
    pts: &[(V2, V2)],
    aisle: f64,
    one: f64,
    angle: f64,
    per: f64,
) -> (MultiPolygon<f64>, Option<rows::Pin>) {
    let rings = region::rings_of(core, 0.0);
    let depth = aisle - one;
    let t = angle.to_radians();
    let row = v2(t.cos(), t.sin());
    let tol = 20f64.to_radians().sin();
    // Étendue du cœur perpendiculairement aux rangées.
    let ny = row.left();
    let (lo, hi) = rings
        .iter()
        .flatten()
        .fold((f64::MAX, f64::MIN), |(l, h), q| {
            (l.min(q.dot(ny)), h.max(q.dot(ny)))
        });
    let mut rects: Vec<Quad> = Vec::new();
    let mut pin = None;
    for &(q, d) in pts {
        let mut best: Option<(f64, V2, V2)> = None;
        for r in &rings {
            for i in 0..r.len() {
                let (a, b) = (r[i], r[(i + 1) % r.len()]);
                let ab = b - a;
                if ab.len() < 3.0 {
                    continue;
                }
                let t = ((q - a).dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
                let dd = (a + ab * t - q).len();
                if best.map_or(true, |x| dd < x.0) {
                    best = Some((dd, a, b));
                }
            }
        }
        let Some((dd, a, b)) = best else { continue };
        let u = (b - a).norm();
        if dd > aisle + 0.5 || u.cross(d.norm()).abs() > tol {
            continue;
        }
        let mut n = u.left();
        if (q - a).dot(n) < 0.0 {
            n = n * -1.0;
        }
        // Parallèle aux rangées : seulement un côté extrême du cœur (à une période près).
        let y = a.dot(ny);
        let edge = if n.dot(ny) < 0.0 { y - lo } else { hi - y };
        let parallel = u.cross(row).abs() < 5f64.to_radians().sin() && edge < per;
        if parallel {
            pin = Some((a + n * depth, n));
        }
        if parallel || u.dot(row).abs() < tol {
            rects.push(Quad([
                a - n * 0.05,
                b - n * 0.05,
                b + n * depth,
                a + n * depth,
            ]));
        }
    }
    let core = if rects.is_empty() {
        core.clone()
    } else {
        core.union(&mp_of_quads(&rects).intersection(&region::erode(band, one)))
    };
    (core, pin)
}

pub(crate) fn mp_of_quads(qs: &[Quad]) -> MultiPolygon<f64> {
    let polys: Vec<MultiPolygon<f64>> = qs
        .iter()
        .map(|q| MultiPolygon::new(vec![region::to_poly(&q.0)]))
        .collect();
    geo::algorithm::bool_ops::unary_union(polys.iter())
}

pub fn solve(site: &Site) -> Layout {
    let p = &site.params;
    let mut out = Layout::default();
    let Some(mut z) = region::build(site) else {
        out.warnings.push("Parcelle invalide".into());
        return out;
    };
    let entries = entry_quads(site, &z.parcel, z.parcel_sign);
    let (turn_keep, turn_fillets) = entry_turns(site, &z.parcel, &entries);
    let mut keepout = entries.clone();
    keepout.extend(turn_keep.iter().copied());
    // Devant chaque porte : un cheminement de la largeur de la porte, du mur jusqu'à la chaussée
    // (trottoir + profondeur de place), libre de places et de plantations.
    let walks = door_walks(site);
    keepout.extend(walks.iter().copied());

    let b_rings = region::rings_of(&z.b, 15.0);
    let b_region = Region::new(b_rings.clone());
    // Trame des rangées (orientation, décalage) optimisée sur le cœur, indépendamment des voies
    // passées à sens unique.
    let core_rings0 = region::rings_of(&z.core, 1.0);
    let grid = if core_rings0.is_empty() {
        (p.angle.unwrap_or(0.0), 0.0, 0)
    } else {
        rows::optimize(&core_rings0, &b_rings, p)
    };

    // Voies passées à sens unique : largeur réduite. Entre deux rangées, l'allée se resserre et
    // les rangées voisines se rapprochent ; sur l'anneau, le cœur avance ou la trame se cale.
    let one = p.aisle_one.min(p.aisle);
    let mut narrow: Vec<rows::Narrow> = Vec::new();
    let mut narrow_rule: Vec<usize> = Vec::new();
    let mut narrowed = false;
    let mut pin = None;
    if one < p.aisle - 0.05 {
        let in_core = Region::new(core_rings0.clone());
        let mut ring_pts: Vec<(V2, V2)> = Vec::new();
        for (k, r) in site.lanes.iter().enumerate().filter(|(_, r)| r.sens == 1) {
            let (q, Some(d)) = (v2(r.p[0], r.p[1]), r.d) else {
                continue;
            };
            narrowed = true;
            if in_core.contains(q) {
                narrow.push((q, v2(d[0], d[1])));
                narrow_rule.push(k);
            } else {
                ring_pts.push((q, v2(d[0], d[1])));
            }
        }
        if !ring_pts.is_empty() {
            let per = p.double_depth() + p.aisle;
            let (core, pn) = narrow_ring(&z.band, &z.core, &ring_pts, p.aisle, one, grid.0, per);
            z.core = core;
            pin = pn;
        }
    }
    let core_rings = region::rings_of(&z.core, 1.0);

    let mut acc = Acc::new();
    if p.perimeter {
        let band_region = Region::new(region::rings_of(&z.band, 1.0));
        perimeter::place(&b_region, &band_region, p, &keepout, &mut acc);
    }
    let rows = rows::place(&core_rings, grid, p, &narrow, pin, &mut acc);

    // Voirie : anneau + allées = bande moins (cœur ∩ rangées doubles).
    let paved = if rows.strips.is_empty() {
        z.band.clone()
    } else {
        let strips = mp_of_quads(&rows.strips);
        z.band.difference(&z.core.intersection(&strips))
    };
    // Pointes : la chaussée plus étroite que l'allée (angles aigus, effilements) est retirée,
    // sauf l'aire de manœuvre devant les places périphériques. Au relâchement seulement : pendant
    // un glisser, le calcul direct reste léger.
    let paved = if p.quick {
        paved
    } else {
        let w = if narrowed { one } else { p.aisle };
        let opened = region::open(&paved, (w - 0.3).max(3.0) * 0.5);
        let kept = Region::new(region::rings_of(&opened, 0.0));
        let cut: Vec<Quad> = acc
            .access
            .quads
            .iter()
            .filter(|q| !kept.contains_quad(q, 0.05))
            .copied()
            .collect();
        if cut.is_empty() {
            opened
        } else {
            opened.union(&paved.intersection(&mp_of_quads(&cut)))
        }
    };
    // Arrondis d'entrée : la chaussée gagne l'angle de part et d'autre de chaque accès.
    let paved = if turn_fillets.is_empty() {
        paved
    } else {
        let f: Vec<MultiPolygon<f64>> = turn_fillets
            .iter()
            .map(|r| MultiPolygon::new(vec![region::to_poly(r)]))
            .collect();
        paved.union(&geo::algorithm::bool_ops::unary_union(f.iter()).intersection(&z.b))
    };

    // Connectivité depuis les accès.
    let mut blockers = acc.blockers.clone();
    blockers.extend(acc.islands.iter().copied());
    let mut stalls = acc.stalls;
    let removed = finish::keep_reachable(
        &mut stalls,
        &drive_with_cross(&paved, &acc.cross),
        &entries,
        &blockers,
    );
    if site.entries.is_empty() {
        out.warnings
            .push("Aucun accès : connectivité non vérifiée".into());
    } else if removed > 0 {
        out.warnings
            .push(format!("{removed} places sans accès retirées"));
    }

    // Cibles PMR : entrées des bâtiments, sinon accès véhicules.
    let centroid = if stalls.is_empty() {
        v2(0.0, 0.0)
    } else {
        stalls
            .iter()
            .map(|s| s.q.center())
            .fold(V2::default(), |a, b| a + b)
            * (1.0 / stalls.len() as f64)
    };
    let mut targets: Vec<V2> = Vec::new();
    for bd in &site.buildings {
        if let Some(e) = bd.entrance {
            targets.push(v2(e[0], e[1]));
            continue;
        }
        let r: Vec<V2> = bd.poly.iter().map(|q| v2(q[0], q[1])).collect();
        let n = r.len();
        if n < 3 {
            continue;
        }
        let best = (0..n)
            .map(|i| (r[i] + r[(i + 1) % n]) * 0.5)
            .min_by(|a, b| {
                (*a - centroid)
                    .len()
                    .partial_cmp(&(*b - centroid).len())
                    .unwrap()
            })
            .unwrap();
        targets.push(best);
    }
    out.entrances = targets.iter().map(|t| p2(*t)).collect();
    if targets.is_empty() {
        targets = entries.iter().map(|e| (e.0[0] + e.0[1]) * 0.5).collect();
    }
    if targets.is_empty() {
        targets.push(centroid);
    }
    let (pmr_req, irve_req, eq_req) = finish::assign_kinds(&mut stalls, &targets, p);

    // Espaces plantés : le résiduel de la zone stationnable, arrondi côté chaussée ; la chaussée
    // reprend les arrondis.
    let mut stall_pads: Vec<Quad> = stalls.iter().map(|s| s.q).collect();
    stall_pads.extend(acc.pads.iter().copied());
    let mut road = acc.cross.clone();
    road.extend(entries.iter().copied());
    let pl = planted::build(&z.b, &paved, &stall_pads, &road, &walks, !p.quick);
    let paved = if pl.cut.0.is_empty() {
        paved
    } else {
        paved.union(&pl.cut)
    };
    let paved_rings = region::rings_of(&paved, 0.5);

    // Sorties.
    let parcel_area = signed_area(&z.parcel).abs();
    let paved_area = region::mp_area(&paved)
        + acc.pads.iter().map(quad_area).sum::<f64>()
        + acc.cross.iter().map(quad_area).sum::<f64>()
        + stalls
            .iter()
            .filter(|s| s.perimeter)
            .map(|s| quad_area(&s.q))
            .sum::<f64>()
        + entries.iter().map(quad_area).sum::<f64>();
    let count = |k: u8| stalls.iter().filter(|s| s.kind == k).count();
    let n = stalls.len();
    let perim = stalls.iter().filter(|s| s.perimeter).count();
    out.metrics = Metrics {
        parcel_area: r3(parcel_area),
        buildable_area: r3(region::mp_area(&z.b)),
        paved_area: r3(paved_area),
        green_area: r3((parcel_area - paved_area - z.buildings_area).max(0.0)),
        stalls: n,
        standard: count(KIND_STD),
        pmr: count(KIND_PMR) + count(KIND_PMR_IRVE),
        pmr_required: pmr_req,
        irve: count(KIND_IRVE) + count(KIND_PMR_IRVE),
        irve_required: irve_req,
        irve_equipped: stalls.iter().filter(|s| s.equipped).count(),
        irve_equipped_required: eq_req,
        trees: acc.trees.len(),
        perimeter_stalls: perim,
        interior_stalls: n - perim,
        removed_unreachable: removed,
        m2_paved_per_stall: if n > 0 {
            r3(paved_area / n as f64)
        } else {
            0.0
        },
        m2_parcel_per_stall: if n > 0 {
            r3(parcel_area / n as f64)
        } else {
            0.0
        },
    };
    out.setback_line = region::rings_of(&z.setback, 1.0)
        .iter()
        .map(|r| r.iter().map(|p| p2(*p)).collect())
        .collect();
    out.paved = paved_rings
        .iter()
        .map(|r| r.iter().map(|p| p2(*p)).collect())
        .collect();
    out.pads = acc.pads.iter().map(q4).collect();
    out.stalls = stalls
        .iter()
        .map(|s| StallOut {
            q: q4(&s.q),
            k: s.kind,
            e: s.equipped,
        })
        .collect();
    // Îlots des rangées intérieures ; les îlots d'extrémité et de bordure font partie des espaces plantés.
    let in_planted = Region::new(
        pl.polys
            .iter()
            .flatten()
            .map(|r| r.iter().map(|q| v2(q[0], q[1])).collect())
            .collect(),
    );
    out.islands = acc
        .islands
        .iter()
        .filter(|q| !in_planted.contains(q.center()))
        .map(q4)
        .collect();
    out.planted = pl.polys;
    out.trees = acc.trees.iter().map(|t| p2(*t)).collect();
    out.entries = entries.iter().map(q4).collect();
    out.walks = walks.iter().map(q4).collect();
    // Bassins de rétention : pied de talus à `3 × hauteur` du haut de talus (réduit si le bassin est étroit).
    out.basins = site
        .basins
        .iter()
        .map(|b| {
            let ring = crate::geom::clean_ring(&b.iter().map(|q| v2(q[0], q[1])).collect::<Vec<_>>());
            let mut run = (p.basin_h.clamp(0.0, 10.0)) * 3.0;
            if ring.len() < 3 || run < 0.05 {
                return BasinOut { run: 0.0, bottom: vec![] };
            }
            let mp = MultiPolygon::new(vec![region::to_poly(&ring)]);
            for _ in 0..8 {
                let rings = region::rings_of(&region::erode(&mp, run), 0.5);
                if !rings.is_empty() {
                    let bottom = rings.iter().map(|r| r.iter().map(|q| p2(*q)).collect()).collect();
                    return BasinOut { run, bottom };
                }
                run *= 0.7;
            }
            BasinOut { run, bottom: vec![] }
        })
        .collect();
    {
        let blocked = |polys: &[Vec<P2>]| {
            polys.iter().any(|r| {
                let ring: Vec<V2> = r.iter().map(|q| v2(q[0], q[1])).collect();
                ring.len() >= 3
                    && walks.iter().any(|w| {
                        MultiPolygon::new(vec![region::to_poly(&ring)])
                            .intersection(&MultiPolygon::new(vec![region::to_poly(&w.0)]))
                            .unsigned_area()
                            > 0.5
                    })
            })
        };
        if blocked(&site.landscapes) || blocked(&site.basins) || blocked(&site.exclusions) {
            out.warnings
                .push("Entrée de bâtiment obstruée par un espace vert ou une servitude".into());
        }
    }
    out.cross = acc.cross.iter().map(q4).collect();
    // Circulation : sens des voies et flèches au sol, sur la chaussée (voirie, allées
    // transversales, accès). Calculée au relâchement, pas pendant un glisser.
    if !p.quick {
        let mut lanes = acc.cross.clone();
        lanes.extend(entries.iter().copied());
        // Repères des allées resserrées : portés au milieu effectif de l'allée pour l'analyse,
        // puis rendus sur le milieu de la trame régulière, qui ne dépend pas des autres allées
        // resserrées.
        let t = rows.angle.to_radians();
        let ny = v2(-t.sin(), t.cos());
        let mut rules = site.lanes.clone();
        for &(i, _, actual) in &rows.narrowed {
            let r = &mut rules[narrow_rule[i]];
            let q = v2(r.p[0], r.p[1]);
            let q = q + ny * (actual - q.dot(ny));
            r.p = [q.x, q.y];
        }
        out.circulation =
            circulation::analyse(&paved_rings, &lanes, &entries, p.aisle, one, &rules);
        for &(i, regular, actual) in &rows.narrowed {
            let r = &mut out.circulation.lanes[narrow_rule[i]];
            if r.fix {
                let q = v2(r.p[0], r.p[1]) + ny * (regular - actual);
                r.p = [(q.x * 100.0).round() / 100.0, (q.y * 100.0).round() / 100.0];
            }
        }
    }
    match out.circulation.narrow_two_way {
        0 => {}
        1 => out
            .warnings
            .push("1 voie étroite à double sens (impasse ou passage obligé)".into()),
        n => out.warnings.push(format!(
            "{n} voies étroites à double sens (impasses ou passages obligés)"
        )),
    }
    match out.circulation.refused {
        0 => {}
        1 => out
            .warnings
            .push("Sens unique refusé sur 1 voie : la circulation serait coupée".into()),
        n => out.warnings.push(format!(
            "Sens unique refusé sur {n} voies : la circulation serait coupée"
        )),
    }
    match out.circulation.forced_narrow {
        0 => {}
        1 => out
            .warnings
            .push("1 voie de moins de 5 m imposée à double sens".into()),
        n => out
            .warnings
            .push(format!("{n} voies de moins de 5 m imposées à double sens")),
    }
    out.angle = r3(rows.angle);
    out.phase = r3(rows.phase);
    out.evals = rows.evals;
    out
}

/// Réunion de parcelles cadastrales en une emprise d'étude. Les micro-écarts entre parcelles
/// voisines (quelques centimètres) sont refermés par une dilatation puis une érosion de 5 cm.
/// Renvoie les contours extérieurs, du plus grand au plus petit.
pub fn union_rings(polys: &[Vec<P2>]) -> Vec<Vec<P2>> {
    use geo::algorithm::buffer::{BufferStyle, LineJoin};
    use geo::Buffer;
    let parts: Vec<MultiPolygon<f64>> = polys
        .iter()
        .filter(|r| r.len() >= 3)
        .map(|r| {
            let ring: Vec<V2> = r.iter().map(|p| v2(p[0], p[1])).collect();
            MultiPolygon::new(vec![region::to_poly(&ring)])
                .buffer_with_style(BufferStyle::new(0.05).line_join(LineJoin::Miter(0.2)))
        })
        .collect();
    if parts.is_empty() {
        return vec![];
    }
    let merged = geo::algorithm::bool_ops::unary_union(parts.iter());
    let closed = region::erode(&merged, 0.05);
    let mut rings: Vec<Vec<V2>> = region::rings_of(&closed, 1.0)
        .into_iter()
        .filter(|r| signed_area(r) > 0.0)
        .collect();
    rings.sort_by(|a, b| signed_area(b).partial_cmp(&signed_area(a)).unwrap());
    rings
        .iter()
        .map(|r| r.iter().map(|p| p2(*p)).collect())
        .collect()
}

fn drive_with_cross(paved: &MultiPolygon<f64>, cross: &[Quad]) -> Vec<Vec<V2>> {
    if cross.is_empty() {
        return region::rings_of(paved, 0.5);
    }
    let u = paved.union(&mp_of_quads(cross));
    region::rings_of(&u, 0.5)
}

// ---------------------------------------------------------------------------
// Interface WebAssembly minimale (sans dépendance) : JSON en entrée, JSON en sortie.

thread_local! {
    static OUT: RefCell<Vec<u8>> = RefCell::new(Vec::new());
}

#[no_mangle]
pub extern "C" fn alloc(n: usize) -> *mut u8 {
    let mut v: Vec<u8> = Vec::with_capacity(n.max(1));
    let ptr = v.as_mut_ptr();
    std::mem::forget(v);
    ptr
}

/// # Safety
/// `ptr` doit provenir de `alloc(n)`.
#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, n: usize) {
    drop(Vec::from_raw_parts(ptr, 0, n.max(1)));
}

/// # Safety
/// `ptr` doit pointer vers `n` octets JSON valides alloués par `alloc`.
#[no_mangle]
pub unsafe extern "C" fn solve_json(ptr: *const u8, n: usize) -> *const u8 {
    let input = std::slice::from_raw_parts(ptr, n);
    let bytes = match serde_json::from_slice::<Site>(input) {
        Ok(site) => serde_json::to_vec(&solve(&site)).unwrap_or_default(),
        Err(e) => format!(
            "{{\"error\":{}}}",
            serde_json::to_string(&e.to_string()).unwrap()
        )
        .into_bytes(),
    };
    OUT.with(|o| {
        *o.borrow_mut() = bytes;
        o.borrow().as_ptr()
    })
}

#[derive(serde::Deserialize)]
struct UnionIn {
    polys: Vec<Vec<P2>>,
}

/// # Safety
/// `ptr` doit pointer vers `n` octets JSON valides alloués par `alloc`.
#[no_mangle]
pub unsafe extern "C" fn union_json(ptr: *const u8, n: usize) -> *const u8 {
    let input = std::slice::from_raw_parts(ptr, n);
    let bytes = match serde_json::from_slice::<UnionIn>(input) {
        Ok(u) => serde_json::to_vec(&serde_json::json!({ "rings": union_rings(&u.polys) }))
            .unwrap_or_default(),
        Err(e) => format!(
            "{{\"error\":{}}}",
            serde_json::to_string(&e.to_string()).unwrap()
        )
        .into_bytes(),
    };
    OUT.with(|o| {
        *o.borrow_mut() = bytes;
        o.borrow().as_ptr()
    })
}

/// Désérialise l'entrée, calcule, sérialise la sortie (ou l'erreur) dans le tampon partagé.
unsafe fn json_call<I: serde::de::DeserializeOwned, O: serde::Serialize>(
    ptr: *const u8,
    n: usize,
    f: impl Fn(&I) -> O,
) -> *const u8 {
    let input = std::slice::from_raw_parts(ptr, n);
    let bytes = match serde_json::from_slice::<I>(input) {
        Ok(v) => serde_json::to_vec(&f(&v)).unwrap_or_default(),
        Err(e) => format!(
            "{{\"error\":{}}}",
            serde_json::to_string(&e.to_string()).unwrap()
        )
        .into_bytes(),
    };
    OUT.with(|o| {
        *o.borrow_mut() = bytes;
        o.borrow().as_ptr()
    })
}

/// Occupation du sol (voirie, végétation) dans l'emprise importée.
///
/// # Safety
/// `ptr` doit pointer vers `n` octets JSON valides alloués par `alloc`.
#[no_mangle]
pub unsafe extern "C" fn occupation_json(ptr: *const u8, n: usize) -> *const u8 {
    json_call(ptr, n, occupation::occupation)
}

/// Export DXF AutoCAD géoréférencé.
///
/// # Safety
/// `ptr` doit pointer vers `n` octets JSON valides alloués par `alloc`.
#[no_mangle]
pub unsafe extern "C" fn export_dxf_json(ptr: *const u8, n: usize) -> *const u8 {
    json_call(ptr, n, dxf::export)
}

#[no_mangle]
pub extern "C" fn out_len() -> usize {
    OUT.with(|o| o.borrow().len())
}
