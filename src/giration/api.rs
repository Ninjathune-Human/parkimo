//! Contrat JSON de l'épure : entrée de `giration_json` (WebAssembly) et de l'export DXF.

use super::epure::{follow, Run, Section};
use super::places::{motion, test_stall, Ctx, Lane, Status, NODES_REPRISES};
use super::planner::Piece;
use super::kinematics::{Pose, Profile};
use super::scene::{KerbProfile, ObstacleKind, Scene};
use super::sweep::{body_contact, clearance as sweep_clearance, conflicts, envelope, footprint, tyre_tracks, Part, Severity};
use super::vehicle::Vehicle;
use crate::model::{Layout, Site, StallOut, P2};
use serde::{Deserialize, Serialize};

/// Pas d'intégration (m).
pub const STEP: f64 = 0.02;
/// Espacement des poses et des traces renvoyées à l'interface (m).
const OUT_STEP: f64 = 0.10;

fn habile() -> String {
    "habile".into()
}
fn margin() -> f64 {
    0.20
}

#[derive(Clone, Deserialize)]
pub struct GirIn {
    pub site: Site,
    pub sections: Vec<Section>,
    #[serde(default = "habile")]
    pub profile: String,
    #[serde(default = "margin")]
    pub margin: f64,
    #[serde(default = "KerbProfile::t2")]
    pub kerb: KerbProfile,
    /// Plan déjà affiché par l'interface ; absent, le plan est recalculé depuis le site.
    #[serde(default)]
    pub layout: Option<Drivable>,
}

/// Ce que la scène lit du plan : chaussée, fonds de rangées, places, îlots, accès, allées transversales.
#[derive(Clone, Deserialize)]
pub struct Drivable {
    pub paved: Vec<Vec<P2>>,
    pub pads: Vec<[P2; 4]>,
    pub stalls: Vec<[P2; 4]>,
    pub islands: Vec<[P2; 4]>,
    pub entries: Vec<[P2; 4]>,
    pub cross: Vec<[P2; 4]>,
    /// Axes des voies, pour le test des places.
    #[serde(default)]
    pub axes: Vec<Lane>,
}

impl Drivable {
    fn layout(&self) -> Layout {
        Layout {
            paved: self.paved.clone(),
            pads: self.pads.clone(),
            stalls: self.stalls.iter().map(|&q| StallOut { q, k: 0, e: false }).collect(),
            islands: self.islands.clone(),
            entries: self.entries.clone(),
            cross: self.cross.clone(),
            ..Layout::default()
        }
    }
}

/// L'épure seule, quand le site est fourni à côté (export DXF).
#[derive(Clone, Deserialize)]
pub struct EpureIn {
    pub sections: Vec<Section>,
    #[serde(default = "habile")]
    pub profile: String,
    #[serde(default = "margin")]
    pub margin: f64,
    #[serde(default = "KerbProfile::t2")]
    pub kerb: KerbProfile,
}

impl GirIn {
    pub fn new(site: Site, e: &EpureIn) -> GirIn {
        GirIn { site, sections: e.sections.clone(), profile: e.profile.clone(), margin: e.margin, kerb: e.kerb.clone(), layout: None }
    }
}

#[derive(Serialize)]
pub struct VehicleOut {
    pub name: String,
    pub length: f64,
    pub width: f64,
    pub wheelbase: f64,
    pub front_overhang: f64,
    pub rear_overhang: f64,
    pub track: f64,
    pub tyre_width: f64,
    pub ground_clearance: f64,
    pub min_radius: f64,
    pub steer_deg: f64,
    pub kerb_diameter: f64,
    pub wall_diameter: f64,
}

#[derive(Serialize)]
pub struct ObstacleOut {
    /// "bordure", "mur" ou "limite".
    pub kind: &'static str,
    /// Hauteur vue (m) ; absente pour un mur ou une limite.
    pub height: Option<f64>,
    pub pts: Vec<P2>,
}

#[derive(Debug, Serialize)]
pub struct ConflictOut {
    /// "roue avant gauche", …, "carrosserie", "marge".
    pub part: &'static str,
    /// "conflit" ou "alerte".
    pub severity: &'static str,
    /// "bordure T2", "mur", "limite de parcelle".
    pub obstacle: String,
    pub point: P2,
    pub s_start: f64,
    pub s_end: f64,
    pub section: usize,
}

#[derive(Serialize)]
pub struct SectionOut {
    pub deviation: f64,
    pub reached: bool,
}

#[derive(Serialize)]
pub struct GirOut {
    pub vehicle: VehicleOut,
    pub profile: String,
    pub margin: f64,
    /// [x, y, cap, angle des roues, distance, temps, tronçon], tous les 0,10 m.
    pub poses: Vec<[f64; 7]>,
    /// Huit polylignes : bords gauche et droit des roues AVG, AVD, ARG, ARD.
    pub tracks: Vec<Vec<P2>>,
    pub envelope: Vec<Vec<Vec<P2>>>,
    pub margin_envelope: Vec<Vec<Vec<P2>>>,
    pub obstacles: Vec<ObstacleOut>,
    pub conflicts: Vec<ConflictOut>,
    pub sections: Vec<SectionOut>,
    pub length: f64,
    pub time: f64,
    pub stationary_time: f64,
}

fn profile_of(name: &str) -> Profile {
    match name.to_lowercase().as_str() {
        "prudent" => Profile::Prudent,
        "courant" => Profile::Courant,
        _ => Profile::Habile,
    }
}

fn part_name(p: Part) -> &'static str {
    match p {
        Part::Wheel(0) => "roue avant gauche",
        Part::Wheel(1) => "roue avant droite",
        Part::Wheel(2) => "roue arrière gauche",
        Part::Wheel(_) => "roue arrière droite",
        Part::Body => "carrosserie",
        Part::Margin => "marge",
    }
}

/// Simulation complète d'une épure tracée sur le plan du site.
pub fn simulate(inp: &GirIn) -> GirOut {
    if let Some(d) = &inp.layout {
        return simulate_on(inp, &d.layout());
    }
    let mut site = inp.site.clone();
    site.params.quick = false;
    let lay = crate::solve(&site);
    simulate_on(inp, &lay)
}

/// Même simulation sur un plan déjà calculé.
pub fn simulate_on(inp: &GirIn, lay: &crate::model::Layout) -> GirOut {
    let scene = Scene::from_layout(&inp.site, lay, &inp.kerb);
    let v = Vehicle::vl2026();
    let run: Run = follow(&v, profile_of(&inp.profile), &inp.sections, STEP);
    simulate_run(inp, &v, &scene, &run)
}

/// Mise en forme d'une trajectoire déjà calculée (partagée avec l'export DXF).
fn simulate_run(inp: &GirIn, v: &Vehicle, scene: &Scene, run: &Run) -> GirOut {
    let samples = &run.samples;
    let keep: Vec<usize> = {
        let mut out = Vec::new();
        let mut last = f64::NEG_INFINITY;
        for (i, s) in samples.iter().enumerate() {
            if s.s - last >= OUT_STEP - 1e-9 || i + 1 == samples.len() || (i > 0 && samples[i - 1].section != s.section) {
                out.push(i);
                last = s.s;
            }
        }
        out
    };
    let tracks = tyre_tracks(v, samples);
    let found = conflicts(v, scene, samples, inp.margin);
    let r = v.min_radius();
    GirOut {
        vehicle: VehicleOut {
            name: "VL 2026".into(),
            length: v.front_overhang + v.wheelbase + v.rear_overhang,
            width: v.width,
            wheelbase: v.wheelbase,
            front_overhang: v.front_overhang,
            rear_overhang: v.rear_overhang,
            track: v.track,
            tyre_width: v.tyre_width,
            ground_clearance: v.ground_clearance,
            min_radius: r,
            steer_deg: v.max_steer.to_degrees(),
            kerb_diameter: 2.0 * v.kerb_radius(r),
            wall_diameter: 2.0 * v.wall_radius(r),
        },
        profile: inp.profile.to_lowercase(),
        margin: inp.margin,
        poses: keep
            .iter()
            .map(|&i| {
                let s = &samples[i];
                let p = s.state.pose;
                [p.x, p.y, p.heading, s.state.steer, s.s, s.t, s.section as f64]
            })
            .collect(),
        tracks: tracks.iter().map(|t| keep.iter().map(|&i| t[i]).collect()).collect(),
        envelope: envelope(v, samples, 0.0),
        margin_envelope: if inp.margin > 0.0 { envelope(v, samples, inp.margin) } else { Vec::new() },
        obstacles: scene
            .obstacles
            .iter()
            .map(|o| ObstacleOut {
                kind: match o.kind {
                    ObstacleKind::Kerb => "bordure",
                    ObstacleKind::Wall => "mur",
                    ObstacleKind::Limit => "limite",
                    ObstacleKind::Car => "vehicule",
                },
                height: o.height.is_finite().then_some(o.height),
                pts: o.pts.clone(),
            })
            .collect(),
        conflicts: found
            .iter()
            .map(|c| ConflictOut {
                part: part_name(c.part),
                severity: match c.severity {
                    Severity::Conflict => "conflit",
                    Severity::Warning => "alerte",
                },
                obstacle: match scene.obstacles[c.obstacle].kind {
                    ObstacleKind::Kerb => format!("bordure {}", inp.kerb.name),
                    ObstacleKind::Wall => "mur".into(),
                    ObstacleKind::Limit => "limite de parcelle".into(),
                    ObstacleKind::Car => "véhicule garé".into(),
                },
                point: c.point,
                s_start: c.s_start,
                s_end: c.s_end,
                section: c.section,
            })
            .collect(),
        sections: run
            .deviation
            .iter()
            .zip(&run.reached)
            .map(|(&deviation, &reached)| SectionOut { deviation, reached })
            .collect(),
        length: samples.last().map_or(0.0, |s| s.s),
        time: samples.last().map_or(0.0, |s| s.t),
        stationary_time: run.stationary_time,
    }
}

fn speed() -> f64 {
    5.0
}
fn yes() -> bool {
    true
}

/// Test des places : toutes, ou celles de `stalls` ; `detail` ajoute la géométrie d'une place.
#[derive(Clone, Deserialize)]
pub struct PlacesIn {
    pub site: Site,
    pub layout: Drivable,
    #[serde(default = "habile")]
    pub profile: String,
    #[serde(default = "margin")]
    pub margin: f64,
    #[serde(default = "KerbProfile::t2")]
    pub kerb: KerbProfile,
    #[serde(default = "speed")]
    pub speed: f64,
    #[serde(default = "yes")]
    pub neighbours: bool,
    #[serde(default)]
    pub stalls: Option<Vec<usize>>,
    #[serde(default)]
    pub detail: Option<usize>,
    /// Reprises admises pour une place accessible sans condition.
    #[serde(default = "three")]
    pub max_reversals: usize,
    /// Budget de la recherche avec reprises (états créés) ; épuisé, la place reste non conclue.
    #[serde(default = "nodes")]
    pub max_nodes: usize,
}

fn three() -> usize {
    3
}
fn nodes() -> usize {
    NODES_REPRISES
}

#[derive(Serialize, Default, Clone)]
pub struct MoveOut {
    /// "feasible" (manœuvre trouvée), "infeasible" (aucune dans le domaine exploré), "unknown" (non conclu).
    pub status: &'static str,
    /// Distance minimale de la caisse aux obstacles, plafonnée à 1 m.
    pub clearance: f64,
    pub reprises: usize,
    pub conflicts: usize,
    pub alerts: usize,
    pub length: f64,
    pub reverse: f64,
    pub time: f64,
}

#[derive(Serialize, Clone)]
pub struct PlaceOut {
    pub i: usize,
    /// "vert" (d'un seul tenant), "bleu" (avec reprises), "orange" (sous condition : marge entamée
    /// ou reprises au-delà de la limite), "rouge" (impossible dans le domaine exploré), "gris" (non conclu).
    pub verdict: &'static str,
    /// Pose garée : milieu de l'essieu arrière et cap (nez vers le fond).
    pub park: [f64; 3],
    pub entry: MoveOut,
    pub exit: MoveOut,
    pub reason: Option<&'static str>,
    /// Places orange et rouges seulement : traces des huit flancs de pneus (un point tous les 0,25 m)
    /// et enveloppe balayée par la caisse pendant l'entrée ; voiture garée qui ne tient pas : sa caisse.
    pub tracks: Vec<Vec<P2>>,
    pub envelope: Vec<Vec<Vec<P2>>>,
}

/// Ce qui montre la difficulté d'une place orange ou rouge, au centimètre.
fn drawing(v: &Vehicle, run: &Run, park: &Pose) -> (Vec<Vec<P2>>, Vec<Vec<Vec<P2>>>) {
    let cm = |p: &P2| [(p[0] * 100.0).round() / 100.0, (p[1] * 100.0).round() / 100.0];
    if run.samples.is_empty() {
        return (Vec::new(), vec![vec![footprint(v, park, 0.0).iter().map(cm).collect()]]);
    }
    let mut kept = Vec::new();
    let mut last = f64::NEG_INFINITY;
    for (i, s) in run.samples.iter().enumerate() {
        if s.s - last >= 0.25 || i + 1 == run.samples.len() {
            kept.push(*s);
            last = s.s;
        }
    }
    let tracks = tyre_tracks(v, &kept).iter().map(|t| t.iter().map(cm).collect()).collect();
    let body = envelope(v, &run.samples, 0.0).iter().map(|poly| poly.iter().map(|r| r.iter().map(cm).collect()).collect()).collect();
    (tracks, body)
}

#[derive(Serialize)]
pub struct PlacesOut {
    pub places: Vec<PlaceOut>,
    pub detail: Option<GirOut>,
}

fn execute(v: &Vehicle, p: Profile, scene: &Scene, pieces: &[Piece], speed: f64, margin: f64, status: Status) -> (MoveOut, Run) {
    let run = motion(v, p, pieces, speed);
    // Distance exacte seulement là où un obstacle est plus près que le minimum déjà trouvé.
    let mut clearance = 1.0;
    for s in &run.samples {
        if body_contact(v, scene, &s.state.pose, clearance).is_some() {
            clearance = sweep_clearance(v, scene, &s.state.pose, clearance);
        }
    }
    let found = conflicts(v, scene, &run.samples, margin);
    let mut reverse = 0.0;
    for w in run.samples.windows(2) {
        if pieces[w[1].section].reverse {
            reverse += w[1].s - w[0].s;
        }
    }
    let out = MoveOut {
        status: match status {
            Status::Feasible => "feasible",
            Status::Infeasible => "infeasible",
            Status::Unknown => "unknown",
        },
        clearance: if run.samples.is_empty() { 0.0 } else { clearance },
        reprises: pieces.windows(2).filter(|w| w[0].reverse != w[1].reverse).count(),
        conflicts: found.iter().filter(|c| c.severity == Severity::Conflict).count(),
        alerts: found.iter().filter(|c| c.severity == Severity::Warning).count(),
        length: run.samples.last().map_or(0.0, |s| s.s),
        reverse,
        time: run.samples.last().map_or(0.0, |s| s.t),
    };
    (out, run)
}

pub fn places(inp: &PlacesIn) -> PlacesOut {
    places_with(inp, &Vehicle::vl2026())
}

/// Test des places avec un autre véhicule que le VL 2026.
pub fn places_with(inp: &PlacesIn, v: &Vehicle) -> PlacesOut {
    let v = v.clone();
    let profile = profile_of(&inp.profile);
    let base = Scene::from_layout(&inp.site, &inp.layout.layout(), &inp.kerb);
    let table = super::places::table(&v, profile);
    let ctx = Ctx {
        table: &table,
        v: &v,
        profile,
        margin: inp.margin,
        speed: inp.speed,
        base: &base,
        stalls: &inp.layout.stalls,
        lanes: &inp.layout.axes,
        neighbours: inp.neighbours,
        max_nodes: inp.max_nodes,
    };
    let all: Vec<usize> = (0..inp.layout.stalls.len()).collect();
    let mut list: Vec<usize> = inp.stalls.clone().unwrap_or(all);
    if let Some(d) = inp.detail {
        if !list.contains(&d) {
            list.push(d);
        }
    }
    let mut detail = None;
    let places = list
        .into_iter()
        .filter(|&i| i < inp.layout.stalls.len())
        .map(|i| {
            let t = test_stall(&ctx, i, true);
            let park = [t.park.x, t.park.y, t.park.heading];
            let (entry, run) = execute(&v, profile, &t.scene, &t.entry, inp.speed, inp.margin, t.status);
            let (exit, _) = execute(&v, profile, &t.scene, &t.exit, inp.speed, inp.margin, t.status);
            let reprises = entry.reprises.max(exit.reprises);
            let verdict = match t.status {
                Status::Infeasible => "rouge",
                Status::Unknown => "gris",
                // Manœuvre trouvée sans contact ; un contact au contrôle serait une incohérence : non conclu.
                Status::Feasible if entry.conflicts + exit.conflicts > 0 => "gris",
                Status::Feasible if entry.alerts + exit.alerts > 0 || reprises > inp.max_reversals => "orange",
                Status::Feasible if reprises > 0 => "bleu",
                Status::Feasible => "vert",
            };
            if inp.detail == Some(i) && !t.entry.is_empty() {
                let both: Vec<Piece> = t.entry.iter().chain(&t.exit).cloned().collect();
                let gin = GirIn {
                    site: inp.site.clone(),
                    sections: Vec::new(),
                    profile: inp.profile.clone(),
                    margin: inp.margin,
                    kerb: inp.kerb.clone(),
                    layout: None,
                };
                let run = motion(&v, profile, &both, inp.speed);
                detail = Some(simulate_run(&gin, &v, &t.scene, &run));
            }
            let (tracks, envelope) = if matches!(verdict, "orange" | "rouge") { drawing(&v, &run, &t.park) } else { (Vec::new(), Vec::new()) };
            PlaceOut { i, verdict, park, entry, exit, reason: t.reason, tracks, envelope }
        })
        .collect();
    PlacesOut { places, detail }
}
