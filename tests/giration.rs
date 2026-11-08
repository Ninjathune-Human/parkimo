//! Simulateur de giration, phase 0 : fiche véhicule, conversion du diamètre de braquage,
//! cinématique. Les valeurs attendues sont calculées à la main à partir des formules
//! analytiques (régime permanent, centre de rotation sur l'axe de l'essieu arrière).

use std::f64::consts::PI;
use trame::giration::kinematics::{
    advance, body_corners, drive, steer_in_place, wheels, Pose, Profile, State,
};
use trame::giration::vehicle::{Convention, Vehicle};

const CM: f64 = 0.01;
const STEP: f64 = 0.02;

fn kmh(v: f64) -> f64 {
    v / 3.6
}

fn origin(steer: f64) -> State {
    State {
        pose: Pose {
            x: 0.0,
            y: 0.0,
            heading: 0.0,
        },
        steer,
    }
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// Voiture de Neufert (p. 271), voie et pneus supposés (Neufert ne les donne pas).
fn neufert(conv: Convention) -> Vehicle {
    Vehicle {
        wheelbase: 2.70,
        front_overhang: 0.94,
        rear_overhang: 1.10,
        width: 1.76,
        track: 1.55,
        tyre_width: 0.205,
        ground_clearance: 0.14,
        max_steer: 0.0,
        lock_to_lock: 4.0,
    }
    .with_turning_diameter(11.70, conv)
}

#[test]
fn vl2026_published_diameter_is_read_between_kerbs() {
    let v = Vehicle::vl2026();
    // Bord extérieur du pneu avant extérieur : 12,10 m de diamètre.
    assert!(close(2.0 * v.kerb_radius(v.min_radius()), 12.10, 0.5 * CM));
    // R = √((D/2 − P/2)² − E²) − V/2 = 4,3495 m.
    assert!(
        close(v.min_radius(), 4.3495, 0.5 * CM),
        "R = {}",
        v.min_radius()
    );
}

#[test]
fn vl2026_wall_diameter_exceeds_the_published_figure() {
    let v = Vehicle::vl2026();
    // Coin avant extérieur de caisse : 2·√((R + l/2)² + (E + A)²) = 13,154 m.
    let d = 2.0 * v.wall_radius(v.min_radius());
    assert!(close(d, 13.154, CM), "diamètre entre murs = {d}");
    // Flanc intérieur de caisse au droit de l'essieu arrière : R − l/2 = 3,36 m.
    assert!(close(v.inner_radius(v.min_radius()), 3.3595, 0.5 * CM));
}

#[test]
fn reading_between_walls_turns_tighter_than_between_kerbs() {
    let kerbs = neufert(Convention::Kerbs);
    let walls = neufert(Convention::Walls);
    assert!(
        close(kerbs.min_radius(), 4.2988, 0.5 * CM),
        "R trottoirs = {}",
        kerbs.min_radius()
    );
    assert!(
        close(walls.min_radius(), 3.6996, 0.5 * CM),
        "R murs = {}",
        walls.min_radius()
    );
    // Chaque lecture restitue le diamètre publié sur le cercle qu'elle désigne.
    assert!(close(
        2.0 * kerbs.kerb_radius(kerbs.min_radius()),
        11.70,
        0.5 * CM
    ));
    assert!(close(
        2.0 * walls.wall_radius(walls.min_radius()),
        11.70,
        0.5 * CM
    ));
}

#[test]
fn ackermann_inner_wheel_steers_more_and_both_axes_meet_at_the_centre() {
    let v = Vehicle::vl2026();
    let r = v.min_radius();
    let (inner, outer) = v.ackermann(v.max_steer);
    assert!(inner > v.max_steer && v.max_steer > outer);
    assert!(close(inner.tan(), v.wheelbase / (r - v.track / 2.0), 1e-9));
    assert!(close(outer.tan(), v.wheelbase / (r + v.track / 2.0), 1e-9));
    // VL 2026 : 39,3° à l'intérieur, 29,2° à l'extérieur.
    assert!(close(inner.to_degrees(), 39.31, 0.05));
    assert!(close(outer.to_degrees(), 29.21, 0.05));
}

// ---------------------------------------------------------------------------
// Cinématique

/// Braquage maximal à gauche tenu sur un tour complet : chaque point du véhicule décrit
/// un cercle autour de O = (0, R), avec les rayons de la fiche à 1 cm près.
#[test]
fn steady_state_circles_match_the_analytical_radii() {
    let v = Vehicle::vl2026();
    let r = v.min_radius();
    let o = [0.0, r];
    let path = drive(
        &v,
        Profile::Habile,
        origin(v.max_steer),
        kmh(5.0),
        2.0 * PI * r,
        v.max_steer,
        STEP,
    );
    assert!(path.len() > 100);
    for s in &path {
        // Coin avant droit = coin extérieur en virage à gauche.
        let fr = body_corners(&v, &s.pose)[1];
        assert!(
            close(dist(fr, o), v.wall_radius(r), CM),
            "coin avant : {}",
            dist(fr, o)
        );
        // Flanc gauche au droit de l'essieu arrière = point le plus proche de O.
        let rear_left = [
            s.pose.x - v.width / 2.0 * s.pose.heading.sin(),
            s.pose.y + v.width / 2.0 * s.pose.heading.cos(),
        ];
        assert!(close(dist(rear_left, o), v.inner_radius(r), CM));
        // Roue avant droite : tangente à son cercle, bord extérieur sur le cercle des trottoirs.
        let w = wheels(&v, s)[1];
        let radial = (w.centre[1] - o[1]).atan2(w.centre[0] - o[0]);
        let skew = (w.heading - radial).sin().abs();
        assert!(
            close(skew, 1.0, 1e-9),
            "la roue doit être perpendiculaire au rayon"
        );
        let edge = [
            w.centre[0] + v.tyre_width / 2.0 * radial.cos(),
            w.centre[1] + v.tyre_width / 2.0 * radial.sin(),
        ];
        assert!(close(dist(edge, o), v.kerb_radius(r), CM));
    }
    let end = path.last().unwrap().pose;
    assert!(
        close(end.x, 0.0, CM) && close(end.y, 0.0, CM),
        "le tour doit se refermer"
    );
}

#[test]
fn left_and_right_turns_are_mirror_images() {
    let v = Vehicle::vl2026();
    let l = drive(
        &v,
        Profile::Habile,
        origin(0.0),
        kmh(10.0),
        12.0,
        v.max_steer,
        STEP,
    );
    let r = drive(
        &v,
        Profile::Habile,
        origin(0.0),
        kmh(10.0),
        12.0,
        -v.max_steer,
        STEP,
    );
    let (a, b) = (l.last().unwrap(), r.last().unwrap());
    assert!(close(a.pose.x, b.pose.x, 1e-9));
    assert!(close(a.pose.y, -b.pose.y, 1e-9));
    assert!(close(a.pose.heading, -b.pose.heading, 1e-9));
    assert!(close(a.steer, -b.steer, 1e-9));
}

/// Pour un angle de roues fixe, reculer de la distance parcourue ramène au départ.
#[test]
fn reversing_retraces_the_same_arc() {
    let v = Vehicle::vl2026();
    let start = State {
        pose: Pose {
            x: 3.0,
            y: -2.0,
            heading: 0.7,
        },
        steer: 0.3,
    };
    let mut s = start;
    for _ in 0..400 {
        s = advance(&v, s, STEP);
    }
    for _ in 0..400 {
        s = advance(&v, s, -STEP);
    }
    assert!(close(s.pose.x, start.pose.x, 1e-6));
    assert!(close(s.pose.y, start.pose.y, 1e-6));
    assert!(close(s.pose.heading, start.pose.heading, 1e-9));
}

/// Volant à vitesse finie, échelle du CERTU (Giration 3.3, repris par Mensura) : le conducteur
/// habile tourne les roues à 15°/s, le courant à 10°/s, le prudent à 6°/s. De roues droites à la
/// butée, la voiture parcourt donc vitesse × butée / 15°/s pour le conducteur habile.
#[test]
fn reaching_full_lock_takes_the_lock_over_the_driver_steering_rate() {
    let v = Vehicle::vl2026();
    let rate = 15f64.to_radians();
    for speed in [kmh(10.0), kmh(5.0)] {
        let path = drive(&v, Profile::Habile, origin(0.0), speed, 10.0, v.max_steer, STEP);
        let reached = path.iter().position(|s| close(s.steer, v.max_steer, 1e-12)).unwrap();
        let travelled = reached as f64 * STEP;
        let expected = speed * v.max_steer / rate;
        assert!(close(travelled, expected, STEP), "{travelled} m au lieu de {expected} m");
    }
}

#[test]
fn each_driver_turns_the_wheels_at_his_own_rate_within_the_vehicle_limit() {
    let v = Vehicle::vl2026();
    let speed = kmh(5.0);
    let change = |v: &Vehicle, p: Profile| {
        let path = drive(v, p, origin(0.0), speed, STEP, v.max_steer, STEP);
        (path[1].steer - path[0].steer) / (STEP / speed)
    };
    for (p, deg) in [(Profile::Habile, 15.0), (Profile::Courant, 10.0), (Profile::Prudent, 6.0)] {
        assert!(close(change(&v, p).to_degrees(), deg, 1e-9), "{p:?}");
    }
    // Direction lente (8 s de butée à butée) : la direction plafonne le conducteur habile.
    let slow = Vehicle { lock_to_lock: 8.0, ..Vehicle::vl2026() };
    assert!(close(change(&slow, Profile::Habile), 2.0 * slow.max_steer / 8.0, 1e-12));
}

#[test]
fn prudent_driver_keeps_ten_percent_of_the_lock_in_reserve() {
    let v = Vehicle::vl2026();
    let path = drive(
        &v,
        Profile::Prudent,
        origin(0.0),
        kmh(5.0),
        10.0,
        v.max_steer,
        STEP,
    );
    let max = path.iter().map(|s| s.steer).fold(0.0, f64::max);
    assert!(close(max, 0.9 * v.max_steer, 1e-12), "{max}");
}

/// Braquage à l'arrêt aux seuls rebroussements, où la voiture s'arrête de toute façon (règle
/// d'AutoTURN et de Mensura), pour les conducteurs habile et courant ; jamais pour le prudent.
#[test]
fn stationary_steering_only_at_cusps_and_never_for_a_careful_driver() {
    let v = Vehicle::vl2026();
    let s = origin(0.0);
    let (turned, time) = steer_in_place(&v, Profile::Habile, s, v.max_steer, true).unwrap();
    assert!(close(turned.steer, v.max_steer, 1e-12));
    assert_eq!(turned.pose.x, s.pose.x);
    assert!(close(time, v.max_steer / 15f64.to_radians(), 1e-9), "{time}");
    assert!(steer_in_place(&v, Profile::Habile, s, v.max_steer, false).is_none());
    assert!(steer_in_place(&v, Profile::Courant, s, v.max_steer, false).is_none());
    assert!(steer_in_place(&v, Profile::Courant, s, v.max_steer, true).is_some());
    assert!(steer_in_place(&v, Profile::Prudent, s, v.max_steer, true).is_none());
}

/// L'intégration par arcs exacts ne dépend presque pas du pas : 2 cm et 2 mm donnent
/// la même pose finale à 1 mm près, même pendant que le volant tourne.
#[test]
fn integration_is_step_independent() {
    let v = Vehicle::vl2026();
    let coarse = drive(
        &v,
        Profile::Habile,
        origin(0.0),
        kmh(10.0),
        15.0,
        v.max_steer,
        0.02,
    );
    let fine = drive(
        &v,
        Profile::Habile,
        origin(0.0),
        kmh(10.0),
        15.0,
        v.max_steer,
        0.002,
    );
    let (a, b) = (coarse.last().unwrap().pose, fine.last().unwrap().pose);
    assert!(
        close(a.x, b.x, 0.001) && close(a.y, b.y, 0.001),
        "{a:?} / {b:?}"
    );
}

// ---------------------------------------------------------------------------
// Scène : obstacles typés extraits du plan

use trame::giration::scene::{KerbProfile, ObstacleKind, Scene};
use trame::model::{Layout, Site};

fn scenario(name: &str) -> (Site, Layout) {
    let path = format!("{}/scenarios/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let site: Site = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let lay = trame::solve(&site);
    (site, lay)
}

fn centroid(q: &[[f64; 2]]) -> [f64; 2] {
    let n = q.len() as f64;
    [q.iter().map(|p| p[0]).sum::<f64>() / n, q.iter().map(|p| p[1]).sum::<f64>() / n]
}

fn nearest_stall(lay: &Layout, p: [f64; 2]) -> [f64; 2] {
    lay.stalls
        .iter()
        .map(|s| centroid(&s.q))
        .min_by(|a, b| dist(*a, p).total_cmp(&dist(*b, p)))
        .unwrap()
}

#[test]
fn kerbs_stay_inside_the_parcel_and_leave_the_entry_open() {
    let (site, lay) = scenario("rect");
    let scene = Scene::from_layout(&site, &lay, &KerbProfile::t2());
    assert!(!scene.obstacles.is_empty());
    for o in &scene.obstacles {
        for p in &o.pts {
            assert!((-0.01..=120.01).contains(&p[0]) && (-0.01..=80.01).contains(&p[1]), "{p:?}");
        }
    }
    // Entrée au milieu du bord bas : on vient de la rue sans franchir de bordure.
    let inner = centroid(&lay.entries[0]);
    assert!(scene.crossings([60.0, -5.0], inner).is_empty());
}

#[test]
fn no_kerb_cuts_through_a_stall() {
    for name in ["rect", "ruisseau"] {
        let (site, lay) = scenario(name);
        let scene = Scene::from_layout(&site, &lay, &KerbProfile::t2());
        for (i, s) in lay.stalls.iter().enumerate() {
            let c = centroid(&s.q);
            // Places rétrécies de 5 cm vers leur centre.
            let q: Vec<[f64; 2]> = s
                .q
                .iter()
                .map(|p| {
                    let d = dist(*p, c);
                    let k = (d - 0.05) / d;
                    [c[0] + (p[0] - c[0]) * k, c[1] + (p[1] - c[1]) * k]
                })
                .collect();
            for e in 0..4 {
                let hit = scene.crossings(q[e], q[(e + 1) % 4]);
                assert!(hit.is_empty(), "{name} : bordure à travers la place {i}");
            }
        }
    }
}

#[test]
fn buildings_are_walls_behind_a_kerbed_planted_strip_and_planted_areas_are_kerbed() {
    let (site, lay) = scenario("ruisseau");
    let scene = Scene::from_layout(&site, &lay, &KerbProfile::t2());
    // Un point intérieur au bâtiment (en L : le barycentre des sommets tombe dans l'encoche).
    let poly = &site.buildings[0].poly;
    let region = trame::geom::Region::new(vec![poly.iter().map(|p| trame::geom::v2(p[0], p[1])).collect()]);
    let b = (0..poly.len())
        .map(|i| centroid(&[poly[i], poly[(i + 1) % poly.len()], poly[(i + 2) % poly.len()]]))
        .find(|c| region.contains(trame::geom::v2(c[0], c[1])))
        .unwrap();
    let hits = scene.crossings(b, nearest_stall(&lay, b));
    let kinds: Vec<ObstacleKind> = hits.iter().map(|h| scene.obstacles[h.obstacle].kind).collect();
    // Sans porte, pas de cheminement : de la pleine terre entre le bâtiment et la chaussée.
    assert!(kinds.contains(&ObstacleKind::Wall), "{kinds:?}");
    assert!(kinds.contains(&ObstacleKind::Planted), "{kinds:?}");
    let mut checked = 0;
    for poly in &lay.planted {
        let c = centroid(&poly[0]);
        if !trame::geom::Region::new(vec![poly[0].iter().map(|p| trame::geom::v2(p[0], p[1])).collect()])
            .contains(trame::geom::v2(c[0], c[1]))
        {
            continue;
        }
        let hits = scene.crossings(c, nearest_stall(&lay, c));
        assert!(hits.iter().any(|h| scene.obstacles[h.obstacle].kind == ObstacleKind::Planted));
        checked += 1;
    }
    assert!(checked > 3, "{checked} espaces plantés vérifiés");
    // Hauteur vue de la T2 par défaut.
    assert!(scene.obstacles.iter().any(|o| o.kind == ObstacleKind::Planted && o.height == 0.16));
}

#[test]
fn row_islands_and_noues_are_kerbed() {
    let mut checked = 0;
    for name in ["rect", "ruisseau"] {
        let (site, lay) = scenario(name);
        let scene = Scene::from_layout(&site, &lay, &KerbProfile::t2());
        for q in lay.islands.iter().chain(&lay.noues) {
            let c = centroid(q);
            let hits = scene.crossings(c, nearest_stall(&lay, c));
            assert!(hits.iter().any(|h| scene.obstacles[h.obstacle].kind == ObstacleKind::Planted), "{name} : îlot {q:?} sans bordure");
            checked += 1;
        }
    }
    assert!(checked > 3, "{checked} îlots vérifiés");
    // Noues entre demi-rangées.
    let (mut site, _) = scenario("rect");
    site.params.noue_w = 2.0;
    let lay = trame::solve(&site);
    let scene = Scene::from_layout(&site, &lay, &KerbProfile::t2());
    assert!(!lay.noues.is_empty());
    for q in &lay.noues {
        let c = centroid(q);
        let hits = scene.crossings(c, nearest_stall(&lay, c));
        assert!(hits.iter().any(|h| scene.obstacles[h.obstacle].kind == ObstacleKind::Planted), "noue {q:?} sans bordure");
    }
}

#[test]
fn planted_ground_kerbs_let_the_overhang_pass_and_sidewalk_kerbs_do_not() {
    // Allée de 10 m sur la parcelle rect, pleine terre autour ; un cheminement piéton de 2 m au nord.
    let (site, _) = scenario("rect");
    let lay = Layout {
        paved: vec![vec![[10.0, 20.0], [110.0, 20.0], [110.0, 30.0], [10.0, 30.0]]],
        walks: vec![[[59.0, 30.0], [61.0, 30.0], [61.0, 34.0], [59.0, 34.0]]],
        ..Layout::default()
    };
    let scene = Scene::from_layout(&site, &lay, &KerbProfile::t2());
    let kinds = |x: f64| scene.crossings([x, 25.0], [x, 31.0]).iter().map(|h| scene.obstacles[h.obstacle].kind).collect::<Vec<_>>();
    assert_eq!(kinds(60.0), vec![ObstacleKind::Kerb]);
    assert_eq!(kinds(30.0), vec![ObstacleKind::Planted]);
}

#[test]
fn sharp_corners_of_slanted_stalls_leave_no_sidewalk_kerb_on_planted_ground() {
    // Places à 45° sur la parcelle rect (ni bâtiment, ni cheminement, ni servitude) : leurs angles
    // vifs laissent des bouts de bordure de quelques millimètres, tous le long de la pleine terre.
    let (mut site, _) = scenario("rect");
    site.params.slant = 45.0;
    let lay = trame::solve(&site);
    let scene = Scene::from_layout(&site, &lay, &KerbProfile::t2());
    let sidewalk: Vec<[f64; 2]> = scene.obstacles.iter().filter(|o| o.kind == ObstacleKind::Kerb).map(|o| o.pts[0]).collect();
    assert!(sidewalk.is_empty(), "{} bordures de trottoir, la première en {:?}", sidewalk.len(), sidewalk.first());
}

// ---------------------------------------------------------------------------
// Épure : le véhicule suit une polyligne tracée, en marche avant ou arrière

use trame::giration::epure::{follow, Section};

fn arc(cx: f64, cy: f64, r: f64, a0: f64, a1: f64, n: usize) -> Vec<[f64; 2]> {
    (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f64 / n as f64;
            [cx + r * a.cos(), cy + r * a.sin()]
        })
        .collect()
}

fn section(pts: Vec<[f64; 2]>, reverse: bool, speed_kmh: f64) -> Section {
    Section { pts, reverse, speed_kmh }
}

#[test]
fn straight_line_is_followed_to_its_end() {
    let v = Vehicle::vl2026();
    let run = follow(&v, Profile::Habile, &[section(vec![[0.0, 0.0], [20.0, 0.0]], false, 5.0)], STEP);
    assert!(run.reached[0]);
    assert!(run.deviation[0] < 0.01, "{}", run.deviation[0]);
    let last = run.samples.last().unwrap().state;
    // L'essieu avant (point guidé en marche avant) arrive au bout de la polyligne.
    assert!(close(last.pose.x + v.wheelbase, 20.0, 0.05), "{:?}", last.pose);
    assert!(close(last.pose.heading, 0.0, 1e-6));
}

#[test]
fn a_ten_metre_arc_is_followed_within_two_centimetres() {
    let v = Vehicle::vl2026();
    let mut pts = vec![[-10.0, -10.0], [0.0, -10.0]];
    pts.extend(arc(0.0, 0.0, 10.0, -PI / 2.0, 0.0, 30).into_iter().skip(1));
    pts.push([10.0, 10.0]);
    let run = follow(&v, Profile::Habile, &[section(pts, false, 5.0)], STEP);
    assert!(run.reached[0]);
    assert!(run.deviation[0] < 0.02, "écart {} m", run.deviation[0]);
}

#[test]
fn reversing_along_an_eight_metre_arc_stays_on_it() {
    let v = Vehicle::vl2026();
    let mut pts = vec![[10.0, -8.0], [0.0, -8.0]];
    pts.extend(arc(0.0, 0.0, 8.0, -PI / 2.0, -PI, 30).into_iter().skip(1));
    let run = follow(&v, Profile::Habile, &[section(pts, true, 3.0)], STEP);
    assert!(run.reached[0]);
    assert!(run.deviation[0] < 0.05, "écart {} m", run.deviation[0]);
}

#[test]
fn reversing_along_a_line_keeps_the_nose_forward() {
    let v = Vehicle::vl2026();
    let run = follow(&v, Profile::Habile, &[section(vec![[0.0, 0.0], [-12.0, 0.0]], true, 3.0)], STEP);
    assert!(run.reached[0]);
    let last = run.samples.last().unwrap().state;
    // En marche arrière, l'essieu arrière est guidé et le capot reste tourné vers +x.
    assert!(close(last.pose.x, -12.0, 0.05), "{:?}", last.pose);
    assert!(close(last.pose.heading.cos(), 1.0, 1e-6));
}

#[test]
fn an_impossible_corner_respects_the_steering_limits_and_is_flagged() {
    let v = Vehicle::vl2026();
    let speed = 10.0;
    let pts = vec![[0.0, 0.0], [15.0, 0.0], [15.0, 15.0]];
    let run = follow(&v, Profile::Habile, &[section(pts, false, speed)], STEP);
    let max_change = 2.0 * v.max_steer / v.lock_to_lock / (speed / 3.6) * STEP;
    for w in run.samples.windows(2) {
        assert!(w[1].state.steer.abs() <= v.max_steer + 1e-12);
        assert!((w[1].state.steer - w[0].state.steer).abs() <= max_change + 1e-12);
    }
    assert!(run.deviation[0] > 0.3, "écart {} m", run.deviation[0]);
}

#[test]
fn a_cusp_allows_stationary_steering_only_for_a_skilled_driver() {
    let v = Vehicle::vl2026();
    let sections = [
        section(vec![[0.0, 0.0], [10.0, 0.0]], false, 5.0),
        section(vec![[7.11, 0.0], [4.0, -1.0], [2.0, -4.0]], true, 3.0),
    ];
    let habile = follow(&v, Profile::Habile, &sections, STEP);
    let prudent = follow(&v, Profile::Prudent, &sections, STEP);
    assert!(habile.stationary_time > 0.0);
    assert_eq!(prudent.stationary_time, 0.0);
    // Le second tronçon roule bien en marche arrière.
    let rev: Vec<_> = habile.samples.iter().filter(|s| s.section == 1).collect();
    let (a, b) = (rev.first().unwrap().state.pose, rev.last().unwrap().state.pose);
    let travel = [b.x - a.x, b.y - a.y];
    assert!(travel[0] * a.heading.cos() + travel[1] * a.heading.sin() < 0.0);
}

// ---------------------------------------------------------------------------
// Traces, enveloppe et conflits

use trame::giration::epure::Sample;
use trame::giration::scene::Obstacle;
use trame::giration::sweep::{conflicts, envelope, tyre_tracks, Part, Severity};

fn line_obstacle(kind: ObstacleKind, height: f64, a: [f64; 2], b: [f64; 2]) -> Obstacle {
    Obstacle { kind, height, pts: vec![a, b] }
}

fn samples(path: &[State]) -> Vec<Sample> {
    path.iter()
        .enumerate()
        .map(|(i, s)| Sample { state: *s, section: 0, s: i as f64 * STEP, t: i as f64 * STEP })
        .collect()
}

fn straight(len: f64) -> Vec<Sample> {
    let v = Vehicle::vl2026();
    samples(&drive(&v, Profile::Habile, origin(0.0), kmh(5.0), len, 0.0, STEP))
}

#[test]
fn tyre_tracks_are_eight_edges_one_tyre_width_apart() {
    let v = Vehicle::vl2026();
    let tracks = tyre_tracks(&v, &straight(5.0));
    assert_eq!(tracks.len(), 8);
    // Roue avant gauche : bord extérieur à V/2 + P/2, bord intérieur à V/2 − P/2.
    let (outer, inner) = (&tracks[0], &tracks[1]);
    assert!(close(outer[0][1], v.track / 2.0 + v.tyre_width / 2.0, 1e-9));
    assert!(close(inner[0][1], v.track / 2.0 - v.tyre_width / 2.0, 1e-9));
    assert!(close(outer[0][0], v.wheelbase, 1e-9));
}

#[test]
fn an_empty_lane_raises_no_conflict() {
    let v = Vehicle::vl2026();
    let scene = Scene::new(vec![
        line_obstacle(ObstacleKind::Kerb, 0.16, [-5.0, 3.0], [30.0, 3.0]),
        line_obstacle(ObstacleKind::Kerb, 0.16, [-5.0, -3.0], [30.0, -3.0]),
    ]);
    assert!(conflicts(&v, &scene, &straight(15.0), 0.20).is_empty());
}

#[test]
fn a_kerb_across_the_path_stops_the_bumper_then_the_wheels() {
    let v = Vehicle::vl2026();
    let scene = Scene::new(vec![line_obstacle(ObstacleKind::Kerb, 0.16, [10.0, -5.0], [10.0, 5.0])]);
    let found = conflicts(&v, &scene, &straight(15.0), 0.20);
    let body = found.iter().find(|c| c.part == Part::Body).expect("carrosserie");
    // Le pare-chocs (E + A = 3,84 m devant l'essieu arrière) atteint la bordure à s = 6,16 m.
    assert!(close(body.s_start, 10.0 - v.wheelbase - v.front_overhang, 0.05), "{}", body.s_start);
    assert_eq!(body.severity, Severity::Conflict);
    for w in 0..4 {
        let c = found.iter().find(|c| c.part == Part::Wheel(w)).expect("roue");
        assert!(close(c.point[0], 10.0, 1e-6));
        assert_eq!(c.severity, Severity::Conflict);
    }
    // La marge (0,20 m) prévient avant le contact.
    let margin = found.iter().find(|c| c.part == Part::Margin).expect("marge");
    assert!(margin.s_start < body.s_start);
    assert_eq!(margin.severity, Severity::Warning);
}

#[test]
fn a_kerb_lower_than_the_ground_clearance_lets_the_overhang_pass() {
    let v = Vehicle::vl2026();
    let scene = Scene::new(vec![line_obstacle(ObstacleKind::Kerb, 0.10, [10.0, -5.0], [10.0, 5.0])]);
    let found = conflicts(&v, &scene, &straight(15.0), 0.20);
    assert!(found.iter().all(|c| c.part != Part::Body && c.part != Part::Margin));
    assert!(found.iter().any(|c| c.part == Part::Wheel(0)));
}

#[test]
fn the_overhang_passes_over_a_planted_kerb_but_not_the_wheels() {
    // Bordure T2 d'un espace planté : la caisse déborde au-dessus, sans alerte de marge ; les pneus butent.
    let v = Vehicle::vl2026();
    let scene = Scene::new(vec![line_obstacle(ObstacleKind::Planted, 0.16, [10.0, -5.0], [10.0, 5.0])]);
    let found = conflicts(&v, &scene, &straight(15.0), 0.20);
    assert!(found.iter().all(|c| c.part != Part::Body && c.part != Part::Margin), "{found:?}");
    assert!(found.iter().any(|c| c.part == Part::Wheel(0) && c.severity == Severity::Conflict));
}

#[test]
fn a_planted_kerb_higher_than_twenty_centimetres_stops_the_overhang() {
    // Neufert p. 272 : débord de caisse au-dessus d'un muret de 0,20 m au plus.
    let v = Vehicle::vl2026();
    let scene = Scene::new(vec![line_obstacle(ObstacleKind::Planted, 0.25, [10.0, -5.0], [10.0, 5.0])]);
    assert!(conflicts(&v, &scene, &straight(15.0), 0.20).iter().any(|c| c.part == Part::Body));
}

#[test]
fn a_wall_inside_the_margin_warns_and_a_wall_beyond_it_does_not() {
    let v = Vehicle::vl2026();
    let near = Scene::new(vec![line_obstacle(ObstacleKind::Wall, f64::INFINITY, [-5.0, 1.09], [30.0, 1.09])]);
    let found = conflicts(&v, &near, &straight(10.0), 0.20);
    assert!(!found.is_empty() && found.iter().all(|c| c.part == Part::Margin && c.severity == Severity::Warning));
    let far = Scene::new(vec![line_obstacle(ObstacleKind::Wall, f64::INFINITY, [-5.0, 1.30], [30.0, 1.30])]);
    assert!(conflicts(&v, &far, &straight(10.0), 0.20).is_empty());
}

#[test]
fn a_parked_car_inside_the_margin_does_not_warn_but_still_collides() {
    // Places jointes : la marge de 0,20 m ne vaut pas contre une voiture garée, seul le contact compte.
    let v = Vehicle::vl2026();
    let near = Scene::new(vec![line_obstacle(ObstacleKind::Car, 1.5, [-5.0, 1.09], [30.0, 1.09])]);
    assert!(conflicts(&v, &near, &straight(10.0), 0.20).is_empty());
    let hit = Scene::new(vec![line_obstacle(ObstacleKind::Car, 1.5, [-5.0, 0.90], [30.0, 0.90])]);
    assert!(conflicts(&v, &hit, &straight(10.0), 0.20).iter().any(|c| c.part == Part::Body && c.severity == Severity::Conflict));
}

#[test]
fn the_envelope_covers_the_swept_body() {
    let v = Vehicle::vl2026();
    let run = samples(&drive(&v, Profile::Habile, origin(0.0), kmh(5.0), 12.0, v.max_steer, STEP));
    let rings = envelope(&v, &run, 0.0);
    let region = trame::geom::Region::new(
        rings.iter().flatten().map(|r| r.iter().map(|p| trame::geom::v2(p[0], p[1])).collect()).collect(),
    );
    for s in run.iter().step_by(25) {
        for c in body_corners(&v, &s.state.pose) {
            // Coins ramenés de 1 cm vers le centre de la caisse.
            let mid = [s.state.pose.x + 1.42 * s.state.pose.heading.cos(), s.state.pose.y + 1.42 * s.state.pose.heading.sin()];
            let d = dist(c, mid);
            let p = [mid[0] + (c[0] - mid[0]) * (d - 0.01) / d, mid[1] + (c[1] - mid[1]) * (d - 0.01) / d];
            assert!(region.contains(trame::geom::v2(p[0], p[1])), "{p:?}");
        }
    }
}

// ---------------------------------------------------------------------------
// Porte de la phase 1 : références publiées

/// Voiture de dimensionnement AASHTO « P » (Green Book) : empattement 11 ft, porte-à-faux
/// 3 ft et 5 ft, largeur 7 ft, rayon de l'axe de l'essieu avant (CTR) 21 ft. Les gabarits
/// AASHTO placent les roues au bord de caisse.
fn aashto_p() -> Vehicle {
    let ft = 0.3048;
    let mut v = Vehicle {
        wheelbase: 11.0 * ft,
        front_overhang: 3.0 * ft,
        rear_overhang: 5.0 * ft,
        width: 7.0 * ft,
        track: 7.0 * ft,
        tyre_width: 0.0,
        ground_clearance: 0.14,
        max_steer: 0.0,
        lock_to_lock: 4.0,
    };
    let r = ((21.0 * ft).powi(2) - v.wheelbase.powi(2)).sqrt();
    v.max_steer = (v.wheelbase / r).atan();
    v
}

/// Demi-tour AASHTO P : rayon intérieur 14,4 ft (4,39 m), roue avant extérieure 24 ft
/// (7,31 m), caisse extérieure 25,5 ft (7,77 m). Les valeurs publiées sont arrondies : 2,5 cm.
#[test]
fn aashto_p_u_turn_matches_the_green_book_template() {
    let v = aashto_p();
    let r = v.min_radius();
    let o = [0.0, r];
    let path = drive(&v, Profile::Habile, origin(v.max_steer), kmh(5.0), PI * r, v.max_steer, STEP);
    let run = samples(&path);
    let tracks = tyre_tracks(&v, &run);
    let outer_front = tracks[2].iter().map(|p| dist(*p, o)).fold(0.0, f64::max);
    let inner_rear = tracks[4].iter().map(|p| dist(*p, o)).fold(f64::INFINITY, f64::min);
    let rings = envelope(&v, &run, 0.0);
    let body_out = rings[0][0].iter().map(|p| dist(*p, o)).fold(0.0, f64::max);
    assert!(close(outer_front, 7.315, 0.025), "roue avant extérieure {outer_front}");
    assert!(close(inner_rear, 4.389, 0.025), "roue arrière intérieure {inner_rear}");
    assert!(close(body_out, 7.772, 0.025), "caisse extérieure {body_out}");
}

/// Couloir courbe : approche droite de 8 m, demi-cercle, sortie droite, entre deux obstacles.
fn curved_lane(r_in: f64, r_out: f64, kind: ObstacleKind, height: f64) -> Vec<Obstacle> {
    let side = |r: f64| {
        let mut pts = vec![[-8.0, -r]];
        pts.extend(arc(0.0, 0.0, r, -PI / 2.0, PI / 2.0, 90));
        pts.push([-8.0, r]);
        Obstacle { kind, height, pts }
    };
    vec![side(r_in), side(r_out)]
}

fn drive_lane_centre(v: &Vehicle, rho: f64) -> Vec<Sample> {
    let mut pts = vec![[-8.0, -rho]];
    pts.extend(arc(0.0, 0.0, rho, -PI / 2.0, PI / 2.0, 90));
    pts.push([-6.0, rho]);
    let run = follow(v, Profile::Habile, &[section(pts, false, 5.0)], STEP);
    assert!(run.reached[0] && run.deviation[0] < 0.05, "écart {}", run.deviation[0]);
    run.samples
}

/// NF P 91-100 (§ 3.3) : courbe à sens unique de 4,00 m, rayon extérieur 8,50 m.
#[test]
fn vl2026_drives_an_nf_p_91_100_one_way_curve_without_conflict() {
    let v = Vehicle::vl2026();
    let scene = Scene::new(curved_lane(4.50, 8.50, ObstacleKind::Wall, f64::INFINITY));
    let found = conflicts(&v, &scene, &drive_lane_centre(&v, 6.50), 0.20);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_three_metre_curve_catches_the_inner_rear_wheel() {
    let v = Vehicle::vl2026();
    let scene = Scene::new(curved_lane(5.50, 8.50, ObstacleKind::Wall, f64::INFINITY));
    let found = conflicts(&v, &scene, &drive_lane_centre(&v, 7.00), 0.20);
    assert!(found.iter().any(|c| c.part == Part::Wheel(2) && c.severity == Severity::Conflict), "{found:?}");
}

/// Neufert p. 274-276 : rampe courbe à une voie, voie de 4,00 m entre R 5,00 et R 9,00,
/// mur plein intérieur à R 4,00.
#[test]
fn vl2026_drives_a_neufert_single_lane_curved_ramp_without_conflict() {
    let v = Vehicle::vl2026();
    let mut obstacles = curved_lane(5.00, 9.00, ObstacleKind::Kerb, 0.16);
    obstacles.push(curved_lane(4.00, 4.00, ObstacleKind::Wall, f64::INFINITY).remove(0));
    let scene = Scene::new(obstacles);
    let found = conflicts(&v, &scene, &drive_lane_centre(&v, 7.00), 0.20);
    assert!(found.is_empty(), "{found:?}");
}

// ---------------------------------------------------------------------------
// Contrat JSON de l'épure (entrée giration_json)

use trame::giration::api::{simulate, GirIn};

fn epure_json(name: &str, pts: &[[f64; 2]]) -> GirIn {
    let path = format!("{}/scenarios/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let site: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    serde_json::from_value(serde_json::json!({
        "site": site,
        "sections": [{ "pts": pts, "speed_kmh": 10.0 }]
    }))
    .unwrap()
}

#[test]
fn entering_from_the_street_is_clean_and_driving_into_a_planted_area_is_not() {
    let (_, lay) = scenario("rect");
    let inner = centroid(&lay.entries[0]);
    let out = simulate(&epure_json("rect", &[[60.0, -12.0], [60.0, -4.0], inner]));
    assert_eq!(out.tracks.len(), 8);
    assert!(out.poses.len() > 50);
    assert!(out.conflicts.is_empty(), "{:?}", out.conflicts);
    assert_eq!(out.vehicle.name, "VL 2026");
    assert_eq!(out.profile, "habile");

    // Vers le centre d'un espace planté depuis la place la plus proche.
    let green = lay
        .planted
        .iter()
        .map(|p| centroid(&p[0]))
        .find(|c| {
            trame::geom::Region::new(vec![lay.planted.iter().flat_map(|p| p[0].iter()).map(|p| trame::geom::v2(p[0], p[1])).collect()])
                .contains(trame::geom::v2(c[0], c[1]))
        })
        .unwrap();
    let from = nearest_stall(&lay, green);
    let out = simulate(&epure_json("rect", &[from, green]));
    assert!(out.conflicts.iter().any(|c| c.severity == "conflit" && c.obstacle == "bordure T2 d'espace planté"), "{:?}", out.conflicts);
}

/// Surface roulable du plan affiché, telle que l'interface la transmet (places réduites à leur quadrilatère).
fn drivable_json(lay: &Layout, islands: bool) -> serde_json::Value {
    serde_json::json!({
        "paved": lay.paved, "pads": lay.pads, "entries": lay.entries, "cross": lay.cross,
        "stalls": lay.stalls.iter().map(|s| s.q).collect::<Vec<_>>(),
        "islands": if islands { lay.islands.clone() } else { Vec::new() },
    })
}

#[test]
fn the_displayed_layout_is_checked_without_solving_again() {
    let (_, lay) = scenario("rect");
    let c = centroid(&lay.islands[0]);
    let path = [nearest_stall(&lay, c), c];
    let solved = simulate(&epure_json("rect", &path));
    let with = |islands: bool| {
        let mut inp = epure_json("rect", &path);
        inp.layout = serde_json::from_value(drivable_json(&lay, islands)).unwrap();
        simulate(&inp)
    };
    let given = with(true);
    let names = |o: &trame::giration::api::GirOut| o.conflicts.iter().map(|k| (k.part, k.obstacle.clone())).collect::<Vec<_>>();
    assert_eq!(given.poses.len(), solved.poses.len());
    assert_eq!(names(&given), names(&solved));
    assert!(names(&given).iter().any(|(_, o)| o == "bordure T2 d'espace planté"));
    // Le plan transmis fait foi : sans ses îlots, plus de bordure à franchir.
    assert!(!names(&with(false)).iter().any(|(_, o)| o.starts_with("bordure T2")));
}

#[test]
fn json_round_trip_names_parts_in_french() {
    let (_, lay) = scenario("rect");
    let inner = centroid(&lay.entries[0]);
    let mut gin = epure_json("rect", &[[60.0, -12.0], inner]);
    gin.margin = 3.5; // marge volontairement énorme : alertes sur les bordures de l'accès
    let out = serde_json::to_value(simulate(&gin)).unwrap();
    for key in ["vehicle", "poses", "tracks", "envelope", "margin_envelope", "obstacles", "conflicts", "sections"] {
        assert!(out.get(key).is_some(), "{key}");
    }
    let c = &out["conflicts"][0];
    assert_eq!(c["part"], "marge");
    assert_eq!(c["severity"], "alerte");
}

/// Régression : en ligne droite à cap oblique, les empreintes successives sont presque
/// colinéaires ; leur réunion doit rester un simple rectangle.
#[test]
fn a_straight_oblique_drive_sweeps_a_plain_rectangle() {
    let v = Vehicle::vl2026();
    let run = follow(&v, Profile::Habile, &[section(vec![[9.25, 7.5], [5.5, 8.5]], false, 10.0)], STEP);
    let rings = envelope(&v, &run.samples, 0.0);
    assert_eq!(rings.len(), 1);
    let ring: Vec<trame::geom::V2> = rings[0][0].iter().map(|p| trame::geom::v2(p[0], p[1])).collect();
    let area = trame::geom::signed_area(&ring).abs();
    let travelled = run.samples.last().unwrap().s;
    let expected = (v.front_overhang + v.wheelbase + v.rear_overhang + travelled) * v.width;
    assert!(close(area, expected, 0.01), "{area} / {expected}");
}

#[test]
fn straight_drives_in_every_direction_sweep_a_single_polygon() {
    let v = Vehicle::vl2026();
    for k in 0..52 {
        let a = k as f64 * 7.0_f64.to_radians();
        let start = [3.3 + 1.7 * k as f64, -2.1 + 0.9 * k as f64];
        let end = [start[0] + 6.0 * a.cos(), start[1] + 6.0 * a.sin()];
        let run = follow(&v, Profile::Habile, &[section(vec![start, end], k % 2 == 1, 7.0)], STEP);
        assert_eq!(envelope(&v, &run.samples, 0.2).len(), 1, "cap {k} × 7°");
    }
}

// ---------------------------------------------------------------------------
// Export DXF de l'épure

#[test]
fn dxf_export_draws_the_epure_on_declared_gir_layers() {
    let (_, lay) = scenario("rect");
    let all: Vec<trame::geom::V2> = lay.planted.iter().flat_map(|p| p[0].iter()).map(|p| trame::geom::v2(p[0], p[1])).collect();
    let green = lay
        .planted
        .iter()
        .map(|p| centroid(&p[0]))
        .find(|c| trame::geom::Region::new(vec![all.clone()]).contains(trame::geom::v2(c[0], c[1])))
        .unwrap();
    let from = nearest_stall(&lay, green);
    let path = format!("{}/scenarios/rect.json", env!("CARGO_MANIFEST_DIR"));
    let site: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let inp: trame::dxf::ExportIn = serde_json::from_value(serde_json::json!({
        "site": site,
        "crs": "LOCAL",
        "giration": { "sections": [{ "pts": [from, green], "speed_kmh": 10.0 }] }
    }))
    .unwrap();
    let dxf = trame::dxf::export(&inp).dxf;
    for layer in ["GIR_TRAJECTOIRE", "GIR_ROUES", "GIR_CARROSSERIE", "GIR_MARGE", "GIR_VEHICULE", "GIR_CONFLITS"] {
        assert!(dxf.contains(&format!("  2\n{layer}\n")), "calque {layer} absent de la table");
        assert!(dxf.contains(&format!("  8\n{layer}\n")), "aucune entité sur {layer}");
    }
    // Sans épure, rien sur ces calques.
    let plain: trame::dxf::ExportIn = serde_json::from_value(serde_json::json!({ "site": serde_json::from_slice::<serde_json::Value>(&std::fs::read(format!("{}/scenarios/rect.json", env!("CARGO_MANIFEST_DIR"))).unwrap()).unwrap(), "crs": "LOCAL" })).unwrap();
    assert!(!trame::dxf::export(&plain).dxf.contains("  8\nGIR_ROUES\n"));
}

// ---------------------------------------------------------------------------
// Phase 2 : chaque place testée en entrée (marche avant) puis en sortie (marche arrière)

use trame::giration::api::{places, PlacesOut};

/// Une rangée de `n` places de `w` × 5 m au-dessus d'une allée de largeur `aisle` à double sens,
/// sur la parcelle du scénario rect (120 × 80 m), en pleine terre. Les places donnent sur l'allée (q2, q3).
fn row_case(n: usize, w: f64, aisle: f64, neighbours: bool, detail: Option<usize>) -> PlacesOut {
    row_with_walks(n, w, aisle, neighbours, detail, false, false)
}

/// La même rangée, avec un trottoir de 2 m en face de l'allée (`facing`) ou derrière les places (`behind`).
fn row_with_walks(n: usize, w: f64, aisle: f64, neighbours: bool, detail: Option<usize>, facing: bool, behind: bool) -> PlacesOut {
    let path = format!("{}/scenarios/rect.json", env!("CARGO_MANIFEST_DIR"));
    let site: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let (y0, y1) = (20.0, 20.0 + aisle);
    let x0 = 60.0 - w * n as f64 / 2.0;
    let stalls: Vec<[[f64; 2]; 4]> = (0..n)
        .map(|i| {
            let a = x0 + w * i as f64;
            [[a, y1 + 5.0], [a + w, y1 + 5.0], [a + w, y1], [a, y1]]
        })
        .collect();
    let mut walks = Vec::new();
    if facing {
        walks.push([[10.0, y0 - 2.0], [110.0, y0 - 2.0], [110.0, y0], [10.0, y0]]);
    }
    if behind {
        walks.push([[10.0, y1 + 5.0], [110.0, y1 + 5.0], [110.0, y1 + 7.0], [10.0, y1 + 7.0]]);
    }
    let input = serde_json::json!({
        "site": site,
        "layout": {
            "paved": [[[10.0, y0], [110.0, y0], [110.0, y1], [10.0, y1]]],
            "pads": [], "islands": [], "entries": [], "cross": [], "stalls": stalls, "walks": walks,
            "axes": [{ "pts": [[10.0, (y0 + y1) / 2.0], [110.0, (y0 + y1) / 2.0]], "w": aisle, "sens": 2 }],
        },
        "neighbours": neighbours,
        "detail": detail,
    });
    places(&serde_json::from_value(input).unwrap())
}

#[test]
fn every_stall_gets_a_verdict_and_a_wide_aisle_is_green() {
    let out = row_case(9, 2.5, 10.0, true, None);
    assert_eq!(out.places.len(), 9);
    let mid = &out.places[4];
    assert_eq!(mid.verdict, "vert", "{:?}", serde_json::to_string(mid).unwrap());
    assert_eq!(mid.entry.reprises, 0);
    assert_eq!(mid.entry.conflicts + mid.exit.conflicts, 0);
}

#[test]
fn a_narrow_aisle_is_not_green() {
    let out = row_case(9, 2.5, 3.5, true, None);
    assert_ne!(out.places[4].verdict, "vert", "{:?}", serde_json::to_string(&out.places[4]).unwrap());
}

#[test]
fn occupied_neighbours_decide_a_tight_stall() {
    // 2,00 m de large pour 1,98 m de caisse : les voisines garées, décalées de 0,10 m, empiètent.
    assert_eq!(row_case(9, 2.0, 10.0, true, None).places[4].verdict, "rouge");
    assert_ne!(row_case(9, 2.0, 10.0, false, None).places[4].verdict, "rouge");
}

#[test]
fn the_entry_ends_parked_nose_in_and_the_test_is_deterministic() {
    let a = row_case(9, 2.5, 10.0, true, Some(4));
    let b = row_case(9, 2.5, 10.0, true, Some(4));
    assert_eq!(serde_json::to_string(&a).unwrap(), serde_json::to_string(&b).unwrap());
    let park = a.places[4].park;
    // Place n° 4 : x de 58,75 à 61,25 ; fond à y = 35.
    assert!((park[0] - 60.0).abs() < 0.01 && (park[2] - std::f64::consts::FRAC_PI_2).abs() < 1e-6, "{park:?}");
    let detail = a.detail.unwrap();
    // Le véhicule passe par la pose garée (fin de l'entrée, début de la sortie).
    assert!(detail.poses.iter().any(|p| dist([p[0], p[1]], [park[0], park[1]]) < 0.15 && (p[2] - park[2]).abs() < 0.05));
    assert!(detail.tracks.len() == 8 && !detail.envelope.is_empty());
}

#[test]
fn a_full_car_park_needs_a_wider_aisle_to_enter_without_reprise() {
    // Deux rangées face à face, toutes les places occupées, voisines garées au pire.
    let path = format!("{}/scenarios/rect.json", env!("CARGO_MANIFEST_DIR"));
    let site: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let verdict = |aisle: f64| {
        let (n, w, y0) = (15usize, 2.5, 25.0);
        let y1 = y0 + aisle;
        let x0 = 60.0 - w * n as f64 / 2.0;
        let mut stalls: Vec<[[f64; 2]; 4]> = (0..n).map(|i| { let a = x0 + w * i as f64; [[a, y1 + 5.0], [a + w, y1 + 5.0], [a + w, y1], [a, y1]] }).collect();
        stalls.extend((0..n).map(|i| { let a = x0 + w * i as f64; [[a + w, y0 - 5.0], [a, y0 - 5.0], [a, y0], [a + w, y0]] }));
        let input = serde_json::json!({ "site": site, "layout": { "paved": [[[5.0, y0], [115.0, y0], [115.0, y1], [5.0, y1]]], "pads": [], "islands": [], "entries": [], "cross": [], "stalls": stalls,
            "axes": [{ "pts": [[5.0, (y0 + y1) / 2.0], [115.0, (y0 + y1) / 2.0]], "w": aisle, "sens": 2 }] }, "stalls": [7] });
        let out = places(&serde_json::from_value(input).unwrap());
        (out.places[0].verdict, out.places[0].entry.reprises)
    };
    // Volant tourné en roulant (15°/s), braquage à l'arrêt aux seuls rebroussements : d'un seul tenant
    // à 10 et 8 m ; avec reprises dans la limite (3) à 6 m : bleu ; au-delà à 5 m : orange.
    assert_eq!(verdict(10.0), ("vert", 0));
    assert_eq!(verdict(8.0), ("vert", 0));
    let (v6, r6) = verdict(6.0);
    let (v5, r5) = verdict(5.0);
    assert_eq!((v6, v5), ("bleu", "orange"), "{r6} et {r5} reprises");
    assert!((1..=3).contains(&r6) && r5 > 3, "{r6} puis {r5} reprises");
}

#[test]
fn a_row_facing_planted_ground_needs_no_alert_on_a_narrow_aisle() {
    // Allée de 5,5 m, pleine terre en face et en fond de place : caisse au-dessus des plantations,
    // pneus sur la chaussée. Pastille verte, sans alerte.
    let p = &row_case(9, 2.5, 5.5, true, None).places[4];
    assert!(matches!(p.verdict, "vert" | "bleu") && p.entry.alerts + p.exit.alerts == 0, "{}", serde_json::to_string(p).unwrap());
}

/// Places d'un scénario résolu à l'inclinaison `slant`, plan transmis tel quel ; seule la place la
/// plus proche de `near` est testée.
fn slanted_place(name: &str, slant: f64, near: [f64; 2]) -> trame::giration::api::PlaceOut {
    let path = format!("{}/scenarios/{name}.json", env!("CARGO_MANIFEST_DIR"));
    let mut raw: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    raw["params"] = serde_json::json!({ "slant": slant });
    let lay = trame::solve(&serde_json::from_value(raw.clone()).unwrap());
    let i = (0..lay.stalls.len()).min_by(|&a, &b| dist(centroid(&lay.stalls[a].q), near).total_cmp(&dist(centroid(&lay.stalls[b].q), near))).unwrap();
    let layout = serde_json::json!({
        "paved": lay.paved, "pads": lay.pads, "islands": lay.islands, "entries": lay.entries, "cross": lay.cross, "walks": lay.walks,
        "stalls": lay.stalls.iter().map(|s| s.q).collect::<Vec<_>>(),
        "axes": lay.circulation.axes.iter().map(|a| serde_json::json!({ "pts": a.pts, "w": a.w, "sens": a.sens })).collect::<Vec<_>>(),
    });
    let input: trame::giration::api::PlacesIn = serde_json::from_value(serde_json::json!({ "site": raw, "layout": layout, "stalls": [i] })).unwrap();
    places(&input).places.remove(0)
}

#[test]
fn a_planned_manoeuvre_keeps_its_tyres_off_the_kerbs_at_the_check() {
    // Place à 45° au bord d'un îlot planté : la recherche doit contrôler les pneus au moins aussi
    // strictement que le rejeu, sinon la place sort grise (« contact au contrôle »).
    let p = slanted_place("rect", 45.0, [100.6, 43.5]);
    assert_eq!(p.entry.conflicts + p.exit.conflicts, 0, "{}", serde_json::to_string(&p).unwrap());
}

/// Voie à sens unique de 6 m qui file vers l'est puis tourne à angle droit vers le sud (coin
/// intérieur en (34, 50)) ; une place à 90° sur le côté ouest de la branche sud, fond à x = 29,
/// entre les ordonnées `y` et `y + 2,5`.
fn corner_case(y: f64) -> trame::giration::api::PlaceOut {
    let path = format!("{}/scenarios/rect.json", env!("CARGO_MANIFEST_DIR"));
    let site: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let input = serde_json::json!({ "site": site, "layout": {
        "paved": [[[10.0, 50.0], [34.0, 50.0], [34.0, 10.0], [40.0, 10.0], [40.0, 56.0], [10.0, 56.0]]],
        "pads": [], "islands": [], "entries": [], "cross": [],
        "stalls": [[[29.0, y + 2.5], [29.0, y], [34.0, y], [34.0, y + 2.5]]],
        "axes": [{ "pts": [[10.0, 53.0], [37.0, 53.0], [37.0, 10.0]], "w": 6.0, "sens": 1 }] } });
    places(&serde_json::from_value(input).unwrap()).places.remove(0)
}

#[test]
fn a_stall_right_after_a_turn_is_entered_from_upstream() {
    // Juste après le virage, côté intérieur : en arrivant de l'amont, il faudrait tourner de 180° en
    // quelques mètres. La voiture ne peut pas partir de la branche sud déjà alignée, au pied de la place.
    let p = corner_case(47.5);
    assert!(!matches!(p.verdict, "vert" | "bleu"), "{}", serde_json::to_string(&p).unwrap());
    // 15 m plus bas, la voie est droite en amont : accessible.
    let p = corner_case(32.5);
    assert!(matches!(p.verdict, "vert" | "bleu"), "{}", serde_json::to_string(&p).unwrap());
}

#[test]
fn every_stall_of_the_reference_scenarios_is_rated() {
    for name in ["rect", "ruisseau"] {
        let path = format!("{}/scenarios/{name}.json", env!("CARGO_MANIFEST_DIR"));
        let raw: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let lay = trame::solve(&serde_json::from_value(raw.clone()).unwrap());
        let layout = serde_json::json!({
            "paved": lay.paved, "pads": lay.pads, "islands": lay.islands, "entries": lay.entries, "cross": lay.cross,
            "stalls": lay.stalls.iter().map(|s| s.q).collect::<Vec<_>>(),
            "axes": lay.circulation.axes.iter().map(|a| serde_json::json!({ "pts": a.pts, "w": a.w, "sens": a.sens })).collect::<Vec<_>>(),
        });
        let input: trame::giration::api::PlacesIn = serde_json::from_value(serde_json::json!({ "site": raw, "layout": layout })).unwrap();
        let t0 = std::time::Instant::now();
        let out = places(&input);
        let per = t0.elapsed().as_secs_f64() * 1000.0 / out.places.len() as f64;
        assert_eq!(out.places.len(), lay.stalls.len());
        assert!(out.places.iter().all(|p| ["vert", "bleu", "orange", "rouge", "gris"].contains(&p.verdict)));
        // Angle des roues dans l'état de la recherche : 35 à 45 ms par place en natif, mesurés pendant
        // que les autres tests tournent ; l'application répartit le calcul sur plusieurs Web Workers.
        assert!(per < 60.0, "{name} : {per:.1} ms par place");
    }
}

#[test]
fn a_pole_between_two_samples_is_caught() {
    // Deux poses à 0,50 m d'écart, braquage à la butée : le coin avant extérieur décrit un arc
    // qui sort des deux empreintes ; un poteau sur cet arc doit être vu.
    let v = Vehicle::vl2026();
    let s0 = State { pose: Pose { x: 0.0, y: 0.0, heading: 0.0 }, steer: v.max_steer };
    let s1 = advance(&v, s0, 0.5);
    let corner = |s: &State| trame::giration::kinematics::body_corners(&v, &s.pose)[1];
    let (a, b) = (corner(&s0), corner(&s1));
    let mid = advance(&v, s0, 0.25);
    let c = corner(&mid);
    // Poteau de 2 mm, 5 mm à l'intérieur de l'arc du coin, à mi-chemin.
    let centre = trame::giration::kinematics::advance(&v, s0, 0.0).pose;
    let r = v.wheelbase / v.max_steer.tan();
    let o = [centre.x, centre.y + r];
    let k = 1.0 - 0.005 / ((c[0] - o[0]).hypot(c[1] - o[1]));
    let pole = [o[0] + (c[0] - o[0]) * k, o[1] + (c[1] - o[1]) * k];
    let ob = Obstacle { kind: ObstacleKind::Wall, height: f64::INFINITY, pts: vec![pole, [pole[0] + 0.002, pole[1]], [pole[0] + 0.002, pole[1] + 0.002], [pole[0], pole[1] + 0.002], pole] };
    let scene = Scene::new(vec![ob]);
    // Précondition : le poteau est hors des deux empreintes extrêmes.
    for q in [&s0, &s1] {
        let f = trame::giration::sweep::footprint(&v, &q.pose, 0.0);
        let inside = (0..4).all(|i| {
            let (p, n) = (f[i], f[(i + 1) % 4]);
            (n[0] - p[0]) * (pole[1] - p[1]) - (n[1] - p[1]) * (pole[0] - p[0]) <= 0.0
        });
        assert!(!inside, "poteau dans une empreinte extrême {a:?} {b:?}");
    }
    let two = vec![Sample { state: s0, section: 0, s: 0.0, t: 0.0 }, Sample { state: s1, section: 0, s: 0.5, t: 0.5 }];
    let found = conflicts(&v, &scene, &two, 0.0);
    assert!(found.iter().any(|c| c.part == Part::Body), "{found:?}");
}

/// Une place de 2,50 × 5,50 m, voisines occupées, dont l'ouverture donne sur une bordure : la
/// voiture ne peut que remuer dans sa place.
#[test]
fn a_stall_closed_by_a_kerb_is_certified_impossible() {
    let path = format!("{}/scenarios/rect.json", env!("CARGO_MANIFEST_DIR"));
    let site: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let stalls: Vec<[[f64; 2]; 4]> = (0..3).map(|i| { let a = 56.25 + 2.5 * i as f64; [[a, 35.5], [a + 2.5, 35.5], [a + 2.5, 30.0], [a, 30.0]] }).collect();
    // Bordure de 0,25 m : plus haute que ce que les porte-à-faux survolent, même en espace planté.
    let input = serde_json::json!({ "site": site, "layout": { "paved": [], "pads": [], "islands": [], "entries": [], "cross": [], "stalls": stalls,
        "axes": [{ "pts": [[10.0, 27.0], [110.0, 27.0]], "w": 6.0, "sens": 2 }] }, "stalls": [1], "kerb": { "name": "haute", "visible_height": 0.25 } });
    let out = places(&serde_json::from_value(input).unwrap());
    let p = &out.places[0];
    assert_eq!((p.verdict, p.entry.status), ("rouge", "infeasible"), "{}", serde_json::to_string(p).unwrap());
    // La moins mauvaise manœuvre (à travers la bordure) est dessinée : caisse et roues.
    assert!(p.tracks.len() == 8 && !p.envelope.is_empty());
}

#[test]
fn a_search_cut_short_by_its_budget_is_unknown_not_impossible() {
    let path = format!("{}/scenarios/rect.json", env!("CARGO_MANIFEST_DIR"));
    let site: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let stalls: Vec<[[f64; 2]; 4]> = (0..9).map(|i| { let a = 48.75 + 2.5 * i as f64; [[a, 35.0], [a + 2.5, 35.0], [a + 2.5, 30.0], [a, 30.0]] }).collect();
    let input = serde_json::json!({ "site": site, "layout": { "paved": [[[10.0, 20.0], [110.0, 20.0], [110.0, 30.0], [10.0, 30.0]]], "pads": [], "islands": [], "entries": [], "cross": [], "stalls": stalls,
        "axes": [{ "pts": [[10.0, 25.0], [110.0, 25.0]], "w": 10.0, "sens": 2 }] }, "stalls": [4], "max_nodes": 40 });
    let p = &places(&serde_json::from_value(input).unwrap()).places[0];
    assert_eq!((p.verdict, p.entry.status), ("gris", "unknown"));
}

#[test]
fn the_minimum_clearance_is_measured() {
    // Rangée sur allée de 10 m, voisines vides, trottoir derrière les places : le pare-chocs garé est
    // à 0,12 m de la bordure de fond (marge de 0,10 m + 2 cm), obstacle le plus proche.
    let p = &row_with_walks(9, 2.5, 10.0, false, None, false, true).places[4];
    assert_eq!(p.verdict, "vert");
    for m in [&p.entry, &p.exit] {
        assert!(m.clearance > 0.09 && m.clearance <= 0.125, "{}", m.clearance);
    }
}

use trame::giration::places::park_pose;

/// Plus petit écart, en mètres, entre la caisse garée et les deux côtés de la place (négatif : elle déborde).
fn side_clearance(v: &Vehicle, q: &[[f64; 2]; 4]) -> f64 {
    let p = park_pose(v, q, 0.22);
    let inside = if (q[1][0] - q[0][0]) * (q[2][1] - q[0][1]) - (q[1][1] - q[0][1]) * (q[2][0] - q[0][0]) > 0.0 { 1.0 } else { -1.0 };
    let signed = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| inside * ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) / dist(a, b);
    body_corners(v, &p).iter().map(|&c| signed(q[1], q[2], c).min(signed(q[3], q[0], c))).fold(f64::INFINITY, f64::min)
}

#[test]
fn a_car_parked_in_a_wedge_stall_stays_between_its_sides() {
    let v = Vehicle::vl2026();
    // Places n° 8 et 9 du plan ruisseau, à la cassure de la limite : la 8 s'ouvre en coin (fond 3,72 m,
    // ouverture 2,50 m), la 9 est un rectangle de 2,50 × 5,00 m.
    let wedge = [[131.173, 78.067], [127.549, 77.216], [129.073, 72.454], [131.496, 73.077]];
    let rect = [[127.549, 77.216], [125.168, 76.454], [126.692, 71.692], [129.073, 72.454]];
    // Voisines garées au pire, décalées de 10 cm vers la place testée : il faut plus de 10 cm de chaque côté.
    for q in [wedge, rect] {
        let c = side_clearance(&v, &q);
        assert!(c > 0.11, "{c:.3}");
    }
    // Place à 60° (parallélogramme) : la caisse reste centrée.
    let (s, c) = (60f64.to_radians().sin(), 60f64.to_radians().cos());
    let w = 2.5 / s;
    let slanted = [[5.0 * c / s, 5.0], [5.0 * c / s + w, 5.0], [w, 0.0], [0.0, 0.0]];
    assert!(close(side_clearance(&v, &slanted), (2.5 - v.width) / 2.0, 1e-6));
}

#[test]
fn only_orange_and_red_places_carry_their_body_and_wheel_tracks() {
    // Vertes et bleues : une pastille suffit, rien à dessiner.
    for aisle in [10.0, 8.0] {
        let p = &row_case(9, 2.5, aisle, true, None).places[4];
        assert!(matches!(p.verdict, "vert" | "bleu"), "{}", p.verdict);
        assert!(p.tracks.is_empty() && p.envelope.is_empty());
    }
    // Orange (allée de 5,5 m entre deux trottoirs) : enveloppe de caisse et traces des huit flancs de pneus.
    let p = &row_with_walks(9, 2.5, 5.5, true, None, true, true).places[4];
    assert_eq!(p.verdict, "orange");
    assert!(p.tracks.len() == 8 && p.tracks.iter().all(|t| t.len() >= 2) && !p.envelope.is_empty());
    // Rouge, voiture garée qui ne tient pas (place de 1,80 m en pleine terre : les pneus, 1,90 m hors
    // tout, touchent les bordures) : sa caisse seule.
    let path = format!("{}/scenarios/rect.json", env!("CARGO_MANIFEST_DIR"));
    let site: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let input = serde_json::json!({ "site": site, "layout": { "paved": [[[10.0, 20.0], [110.0, 20.0], [110.0, 30.0], [10.0, 30.0]]], "pads": [], "islands": [], "entries": [], "cross": [],
        "stalls": [[[59.1, 35.0], [60.9, 35.0], [60.9, 30.0], [59.1, 30.0]]], "axes": [{ "pts": [[10.0, 25.0], [110.0, 25.0]], "w": 10.0, "sens": 2 }] } });
    let p = &places(&serde_json::from_value(input).unwrap()).places[0];
    assert_eq!((p.verdict, p.reason), ("rouge", Some("la voiture garée touche un obstacle")));
    assert!(p.tracks.is_empty() && p.envelope.len() == 1 && p.envelope[0][0].len() >= 4);
}

#[test]
fn a_car_parks_up_to_the_line_of_a_joined_stall_and_short_of_a_kerb() {
    let v = Vehicle::vl2026();
    let path = format!("{}/scenarios/rect.json", env!("CARGO_MANIFEST_DIR"));
    let site: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    // Deux rangées dos à dos (fond commun à y = 35, sans bordure) entre deux allées de 6 m.
    let (y0, y1) = (24.0, 30.0);
    let mut stalls: Vec<[[f64; 2]; 4]> = (0..9).map(|i| { let a = 48.75 + 2.5 * i as f64; [[a, 35.0], [a + 2.5, 35.0], [a + 2.5, 30.0], [a, 30.0]] }).collect();
    stalls.extend((0..9).map(|i| { let a = 48.75 + 2.5 * i as f64; [[a + 2.5, 35.0], [a, 35.0], [a, 40.0], [a + 2.5, 40.0]] }));
    let input = serde_json::json!({ "site": site, "layout": { "paved": [[[10.0, y0], [110.0, y0], [110.0, y1], [10.0, y1]], [[10.0, 40.0], [110.0, 40.0], [110.0, 46.0], [10.0, 46.0]]],
        "pads": [], "islands": [], "entries": [], "cross": [], "stalls": stalls,
        "axes": [{ "pts": [[10.0, 27.0], [110.0, 27.0]], "w": 6.0, "sens": 2 }, { "pts": [[10.0, 43.0], [110.0, 43.0]], "w": 6.0, "sens": 2 }] }, "stalls": [4] });
    let park = places(&serde_json::from_value(input).unwrap()).places[0].park;
    let nose = park[1] + v.wheelbase + v.front_overhang;
    assert!((nose - (35.0 - 0.02)).abs() < 0.005, "{nose}");
    // Fond sur la pleine terre : la caisse peut la survoler, nez à 2 cm de la ligne.
    let park = row_case(9, 2.5, 10.0, true, None).places[4].park;
    assert!((park[1] + v.wheelbase + v.front_overhang - (35.0 - 0.02)).abs() < 0.005);
    // Fond sur un trottoir : marge par défaut de 0,10 m + 2 cm.
    let park = row_with_walks(9, 2.5, 10.0, true, None, false, true).places[4].park;
    assert!((park[1] + v.wheelbase + v.front_overhang - (35.0 - 0.12)).abs() < 0.005);
}

/// Rangée de 9 places sur une allée de `aisle` m, place n° 4 détaillée, profil de conducteur choisi.
fn row_detail(aisle: f64, profile: &str) -> PlacesOut {
    let path = format!("{}/scenarios/rect.json", env!("CARGO_MANIFEST_DIR"));
    let site: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let (y0, y1) = (20.0, 20.0 + aisle);
    let stalls: Vec<[[f64; 2]; 4]> = (0..9).map(|i| { let a = 48.75 + 2.5 * i as f64; [[a, y1 + 5.0], [a + 2.5, y1 + 5.0], [a + 2.5, y1], [a, y1]] }).collect();
    let input = serde_json::json!({ "site": site, "layout": { "paved": [[[10.0, y0], [110.0, y0], [110.0, y1], [10.0, y1]]], "pads": [], "islands": [], "entries": [], "cross": [],
        "stalls": stalls, "axes": [{ "pts": [[10.0, (y0 + y1) / 2.0], [110.0, (y0 + y1) / 2.0]], "w": aisle, "sens": 2 }] }, "stalls": [4], "detail": 4, "profile": profile });
    places(&serde_json::from_value(input).unwrap())
}

/// Volant d'une manœuvre détaillée : plus grand dépassement de la vitesse de braquage en roulant
/// (rad), braquages à l'arrêt hors rebroussement, et braquages à l'arrêt en tout.
fn steering(out: &PlacesOut, rate_deg: f64) -> (f64, usize, usize) {
    let g = out.detail.as_ref().expect("détail");
    let per_m = rate_deg.to_radians() / (5.0 / 3.6);
    let p = &g.poses;
    // Sens de marche de chaque pas : +1, −1, ou 0 à l'arrêt.
    let dir: Vec<i8> = p.windows(2).map(|w| {
        let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
        if dx.hypot(dy) < 1e-9 { 0 } else if dx * w[0][2].cos() + dy * w[0][2].sin() > 0.0 { 1 } else { -1 }
    }).collect();
    let (mut excess, mut outside, mut standing) = (0.0f64, 0, 0);
    for (i, w) in p.windows(2).enumerate() {
        let d = (w[1][3] - w[0][3]).abs();
        if dir[i] != 0 {
            excess = excess.max(d - per_m * (w[1][4] - w[0][4]).abs());
        } else if d > 1e-9 {
            standing += 1;
            let before = dir[..i].iter().rev().find(|&&x| x != 0);
            let after = dir[i + 1..].iter().find(|&&x| x != 0);
            if let (Some(b), Some(a)) = (before, after) {
                if a == b { outside += 1; }
            }
        }
    }
    (excess, outside, standing)
}

#[test]
fn the_wheels_turn_while_rolling_at_the_driver_rate() {
    // Allées de 10 m (d'un seul tenant) et de 8 m (avec reprises), conducteur habile (15°/s, 5 km/h).
    for aisle in [10.0, 8.0] {
        let out = row_detail(aisle, "habile");
        assert!(out.places[0].entry.status == "feasible");
        let (excess, outside, _) = steering(&out, 15.0);
        // Tolérance : les distances du rejeu sont des cordes de 0,25 m, à peine plus courtes que les arcs.
        assert!(excess < 1e-4 && outside == 0, "allée {aisle} m : {:.3}° de trop en roulant, {outside} braquages à l'arrêt hors rebroussement", excess.to_degrees());
    }
}

#[test]
fn a_careful_driver_never_steers_standing_still() {
    let out = row_detail(8.0, "prudent");
    assert!(out.places[0].entry.status == "feasible");
    let (excess, _, standing) = steering(&out, 6.0);
    assert!(excess < 1e-4 && standing == 0, "{:.3}° de trop, {standing} braquages à l'arrêt", excess.to_degrees());
    assert!(out.detail.unwrap().stationary_time.abs() < 1e-9);
}
