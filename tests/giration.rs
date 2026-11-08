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

/// Volant à vitesse finie : de roues droites à la butée, la voiture parcourt v · T / 2
/// (5,56 m à 10 km/h avec 4 s de butée à butée), et deux fois moins à 5 km/h.
#[test]
fn reaching_full_lock_takes_speed_times_half_lock_to_lock() {
    let v = Vehicle::vl2026();
    for (speed, expected) in [(kmh(10.0), 5.5556), (kmh(5.0), 2.7778)] {
        let path = drive(
            &v,
            Profile::Habile,
            origin(0.0),
            speed,
            10.0,
            v.max_steer,
            STEP,
        );
        let reached = path
            .iter()
            .position(|s| close(s.steer, v.max_steer, 1e-12))
            .unwrap();
        let travelled = reached as f64 * STEP;
        assert!(
            close(travelled, expected, STEP),
            "{travelled} m au lieu de {expected} m"
        );
    }
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

#[test]
fn stationary_steering_depends_on_the_driver_profile() {
    let v = Vehicle::vl2026();
    let s = origin(0.0);
    let (turned, time) = steer_in_place(&v, Profile::Habile, s, v.max_steer, false).unwrap();
    assert!(close(turned.steer, v.max_steer, 1e-12));
    assert_eq!(turned.pose.x, s.pose.x);
    assert!(close(time, v.lock_to_lock / 2.0, 1e-9), "{time}");
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
fn buildings_are_walls_behind_a_kerbed_sidewalk_and_planted_areas_are_kerbed() {
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
    assert!(kinds.contains(&ObstacleKind::Wall), "{kinds:?}");
    assert!(kinds.contains(&ObstacleKind::Kerb), "{kinds:?}");
    let mut checked = 0;
    for poly in &lay.planted {
        let c = centroid(&poly[0]);
        if !trame::geom::Region::new(vec![poly[0].iter().map(|p| trame::geom::v2(p[0], p[1])).collect()])
            .contains(trame::geom::v2(c[0], c[1]))
        {
            continue;
        }
        let hits = scene.crossings(c, nearest_stall(&lay, c));
        assert!(hits.iter().any(|h| scene.obstacles[h.obstacle].kind == ObstacleKind::Kerb));
        checked += 1;
    }
    assert!(checked > 3, "{checked} espaces plantés vérifiés");
    // Hauteur vue de la T2 par défaut.
    assert!(scene.obstacles.iter().any(|o| o.kind == ObstacleKind::Kerb && o.height == 0.16));
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
            assert!(hits.iter().any(|h| scene.obstacles[h.obstacle].kind == ObstacleKind::Kerb), "{name} : îlot {q:?} sans bordure");
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
        assert!(hits.iter().any(|h| scene.obstacles[h.obstacle].kind == ObstacleKind::Kerb), "noue {q:?} sans bordure");
    }
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
fn a_wall_inside_the_margin_warns_and_a_wall_beyond_it_does_not() {
    let v = Vehicle::vl2026();
    let near = Scene::new(vec![line_obstacle(ObstacleKind::Wall, f64::INFINITY, [-5.0, 1.09], [30.0, 1.09])]);
    let found = conflicts(&v, &near, &straight(10.0), 0.20);
    assert!(!found.is_empty() && found.iter().all(|c| c.part == Part::Margin && c.severity == Severity::Warning));
    let far = Scene::new(vec![line_obstacle(ObstacleKind::Wall, f64::INFINITY, [-5.0, 1.30], [30.0, 1.30])]);
    assert!(conflicts(&v, &far, &straight(10.0), 0.20).is_empty());
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
    assert!(out.conflicts.iter().any(|c| c.severity == "conflit" && c.obstacle == "bordure T2"), "{:?}", out.conflicts);
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
    assert!(names(&given).iter().any(|(_, o)| o == "bordure T2"));
    // Le plan transmis fait foi : sans ses îlots, plus de bordure à franchir.
    assert!(!names(&with(false)).iter().any(|(_, o)| o == "bordure T2"));
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
/// sur la parcelle du scénario rect (120 × 80 m). Les places donnent sur l'allée (q2, q3).
fn row_case(n: usize, w: f64, aisle: f64, neighbours: bool, detail: Option<usize>) -> PlacesOut {
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
    let input = serde_json::json!({
        "site": site,
        "layout": {
            "paved": [[[10.0, y0], [110.0, y0], [110.0, y1], [10.0, y1]]],
            "pads": [], "islands": [], "entries": [], "cross": [], "stalls": stalls,
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
    assert_eq!(verdict(10.0), ("vert", 0));
    // Reprises dans la limite (3) : bleu ; au-delà : orange, accessible sous condition.
    let (v6, r6) = verdict(6.0);
    let (v8, r8) = verdict(8.0);
    assert_eq!((v8, v6), ("bleu", "orange"), "{r8} et {r6} reprises");
    assert!(r8 < r6 && r8 >= 1 && r6 > 3, "{r6} puis {r8} reprises");
    // Allée de 5 m : aucune manœuvre ; impossibilité démontrée ou budget épuisé, jamais un verdict favorable.
    assert!(["rouge", "gris"].contains(&verdict(5.0).0));
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
        // Cible : 50 ms par place en WebAssembly ; en natif, bien en dessous.
        assert!(per < 40.0, "{name} : {per:.1} ms par place");
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
    let input = serde_json::json!({ "site": site, "layout": { "paved": [], "pads": [], "islands": [], "entries": [], "cross": [], "stalls": stalls,
        "axes": [{ "pts": [[10.0, 27.0], [110.0, 27.0]], "w": 6.0, "sens": 2 }] }, "stalls": [1] });
    let out = places(&serde_json::from_value(input).unwrap());
    let p = &out.places[0];
    assert_eq!((p.verdict, p.entry.status), ("rouge", "infeasible"), "{}", serde_json::to_string(p).unwrap());
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
    // Rangée sur allée de 10 m : le pare-chocs garé est à 0,22 m de la bordure de fond (marge + 2 cm).
    let p = &row_case(9, 2.5, 10.0, true, None).places[4];
    assert_eq!(p.verdict, "vert");
    for m in [&p.entry, &p.exit] {
        assert!(m.clearance > 0.19 && m.clearance <= 0.225, "{}", m.clearance);
    }
}
