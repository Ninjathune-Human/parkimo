//! Entrées / sorties JSON du moteur.

use serde::{Deserialize, Serialize};

pub type P2 = [f64; 2];

#[derive(Deserialize, Clone)]
pub struct Site {
    /// Limite de parcelle, en mètres (sens quelconque).
    pub parcel: Vec<P2>,
    /// Recul par arête : `setbacks[i]` s'applique à l'arête parcel[i] → parcel[i+1].
    #[serde(default)]
    pub setbacks: Vec<f64>,
    /// Servitudes, zones inondables, emprises interdites.
    #[serde(default)]
    pub exclusions: Vec<Vec<P2>>,
    #[serde(default)]
    pub buildings: Vec<Building>,
    /// Espaces verts imposés.
    #[serde(default)]
    pub landscapes: Vec<Vec<P2>>,
    /// Bassins de rétention : zones vertes en creux, sans stationnement ; le contour est le haut de talus.
    #[serde(default)]
    pub basins: Vec<Vec<P2>>,
    /// Accès véhicules sur la limite de parcelle.
    #[serde(default)]
    pub entries: Vec<Entry>,
    /// Typologies de voie imposées depuis le squelette.
    #[serde(default)]
    pub lanes: Vec<LaneRule>,
    /// Repères (milieux d'axe) des noues supprimées depuis le squelette des noues.
    #[serde(default)]
    pub noue_off: Vec<[f64; 2]>,
    /// Traverses tracées depuis le squelette : segments (monde) qui coupent les rangées en allées transversales.
    #[serde(default)]
    pub traverses: Vec<[[f64; 2]; 2]>,
    #[serde(default)]
    pub params: Params,
}

/// Typologie imposée au tronçon d'axe qui passe par `p` (repère recalé à chaque calcul).
#[derive(Deserialize, Serialize, Clone, Copy, Debug, PartialEq)]
pub struct LaneRule {
    pub p: P2,
    /// 2 : double sens, 1 : sens unique.
    pub sens: u8,
    /// Direction de l'axe en `p` : sens de parcours si `fix`, sinon simple orientation.
    #[serde(default)]
    pub d: Option<P2>,
    /// Sens de parcours imposé (sinon choisi par le moteur).
    #[serde(default)]
    pub fix: bool,
}

#[derive(Deserialize, Clone)]
pub struct Building {
    pub poly: Vec<P2>,
    /// Entrée principale (sert au placement des places PMR).
    #[serde(default)]
    pub entrance: Option<P2>,
    /// Porte : point sur le mur, normale sortante, largeur. Devant elle, un cheminement reste libre
    /// de places et de plantations jusqu'à la chaussée.
    #[serde(default)]
    pub door: Option<Door>,
}

#[derive(Deserialize, Clone, Copy)]
pub struct Door {
    pub p: P2,
    pub n: P2,
    pub w: f64,
}

#[derive(Deserialize, Clone, Copy)]
pub struct Entry {
    pub edge: usize,
    pub t: f64,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Params {
    /// Largeur d'une place standard (m).
    pub stall_w: f64,
    /// Longueur d'une place (m).
    pub stall_d: f64,
    /// Hauteur (profondeur) des bassins de rétention (m) ; talus 3 pour 1.
    pub basin_h: f64,
    /// Bande de verdure autour du haut de talus d'un bassin : accès et entretien (m).
    pub basin_margin: f64,
    /// Largeur de la noue entre les deux demi-rangées d'une rangée double (m, 0 = aucune, sinon 1 à 6).
    pub noue_w: f64,
    /// Largeur d'allée double sens (m).
    pub aisle: f64,
    /// Largeur d'une voie passée à sens unique (m).
    pub aisle_one: f64,
    /// Largeur d'un accès véhicules (m).
    pub entry_w: f64,
    /// Part de places adaptées PMR.
    pub pmr_ratio: f64,
    /// Largeur d'une place PMR (m).
    pub pmr_w: f64,
    /// Inclinaison des places des rangées intérieures par rapport à l'allée (degrés) : 30, 45, 60,
    /// 75 ou 90 (perpendiculaire, valeur par défaut). Les valeurs voisines sont ramenées à la plus proche.
    pub slant: f64,
    /// Part de places pré-équipées IRVE.
    pub irve_ratio: f64,
    /// Part de places IRVE équipées d'une borne (une par tranche de 20 places), prises parmi les pré-équipées.
    pub irve_equip_ratio: f64,
    /// Un îlot planté toutes les N places (0 = aucun).
    pub planter_every: u32,
    /// Longueur des îlots d'extrémité de rangée (m, 0 = aucun).
    pub cap: f64,
    /// Trottoir autour des bâtiments (m).
    pub building_margin: f64,
    /// Places périphériques le long des limites.
    pub perimeter: bool,
    /// Orientation imposée des rangées intérieures (degrés). Absente = optimisée.
    pub angle: Option<f64>,
    /// Décalage imposé de la trame (m). Absent = optimisé.
    pub phase: Option<f64>,
    /// Recherche réduite autour de `angle_hint` (pendant un glisser).
    pub quick: bool,
    /// Contrôle des places au sortir des virages (recherche de giration, plus lent) : l'interface le
    /// lance en arrière-plan, après avoir affiché le plan calculé sans lui.
    pub turns: bool,
    pub angle_hint: Option<f64>,
}

impl Params {
    /// Noue effective entre demi-rangées : 0 (aucune) ou entre 1 et 6 m.
    pub fn noue(&self) -> f64 {
        if self.noue_w < 0.5 {
            0.0
        } else {
            self.noue_w.clamp(1.0, 6.0)
        }
    }
    /// Profondeur d'une rangée double : deux places dos à dos, noue comprise.
    pub fn double_depth(&self) -> f64 {
        2.0 * self.stall_d + self.noue()
    }
    /// Inclinaison retenue des places intérieures : 30, 45, 60, 75 ou 90 degrés.
    pub fn slant_deg(&self) -> f64 {
        [30.0, 45.0, 60.0, 75.0, 90.0]
            .into_iter()
            .min_by(|a, b| (a - self.slant).abs().partial_cmp(&(b - self.slant).abs()).unwrap())
            .unwrap()
    }
    /// Places intérieures en épis (inclinées) plutôt que perpendiculaires à l'allée.
    pub fn oblique(&self) -> bool {
        self.slant_deg() < 89.5
    }
    /// Profondeur d'une place inclinée, mesurée perpendiculairement à l'allée : le rectangle
    /// largeur × longueur de la place, pivoté de l'angle d'inclinaison.
    pub fn row_d(&self) -> f64 {
        if !self.oblique() {
            return self.stall_d;
        }
        let a = self.slant_deg().to_radians();
        self.stall_d * a.sin() + self.stall_w * a.cos()
    }
    /// Pas des places le long de l'allée : la largeur de place est mesurée perpendiculairement à son axe.
    pub fn row_pitch(&self) -> f64 {
        if !self.oblique() {
            return self.stall_w;
        }
        self.stall_w / self.slant_deg().to_radians().sin()
    }
    /// Cotangente de l'inclinaison : décalage latéral du fond d'une place par mètre de profondeur.
    pub fn row_shear(&self) -> f64 {
        if !self.oblique() {
            return 0.0;
        }
        let a = self.slant_deg().to_radians();
        a.cos() / a.sin()
    }
    /// Largeur de l'allée entre rangées : celle du réglage pour des places à 90 degrés, sinon la
    /// largeur de voie de Neufert (p. 272) selon l'angle et la largeur de place, par interpolation.
    pub fn row_aisle(&self) -> f64 {
        if !self.oblique() {
            return self.aisle;
        }
        self.neufert_aisle()
    }
    /// Largeur de voie du tableau de Neufert (p. 272) pour l'inclinaison et la largeur de place
    /// courantes, 90 degrés compris ; interpolée entre les colonnes 2,30 / 2,40 / 2,50 m et bornée
    /// à ces colonnes.
    pub fn neufert_aisle(&self) -> f64 {
        // Colonnes : places de 2,30 m, 2,40 m, 2,50 m.
        let col: [f64; 3] = match self.slant_deg() as i32 {
            90 => [6.50, 6.00, 5.50],
            75 => [5.50, 5.25, 5.00],
            60 => [4.50, 4.25, 4.00],
            45 => [3.50, 3.25, 3.00],
            _ => [3.00, 3.00, 3.00],
        };
        let t = ((self.stall_w - 2.3) / 0.1).clamp(0.0, 2.0);
        let i = (t.floor() as usize).min(1);
        let f = t - i as f64;
        col[i] * (1.0 - f) + col[i + 1] * f
    }
    /// Profondeur d'une rangée double inclinée, noue comprise.
    pub fn row_double_depth(&self) -> f64 {
        2.0 * self.row_d() + self.noue()
    }
}

impl Default for Params {
    fn default() -> Self {
        Params {
            stall_w: 2.5,
            stall_d: 5.0,
            basin_h: 1.5,
            basin_margin: 2.0,
            noue_w: 0.0,
            aisle: 6.0,
            aisle_one: 5.0,
            entry_w: 6.0,
            pmr_ratio: 0.02,
            pmr_w: 3.3,
            slant: 90.0,
            irve_ratio: 0.2,
            irve_equip_ratio: 0.05,
            planter_every: 8,
            cap: 1.5,
            building_margin: 2.0,
            perimeter: true,
            angle: None,
            phase: None,
            quick: false,
            turns: true,
            angle_hint: None,
        }
    }
}

/// Type de place : 0 standard, 1 PMR, 2 IRVE, 3 PMR + IRVE.
#[derive(Serialize, Clone)]
pub struct StallOut {
    pub q: [P2; 4],
    pub k: u8,
    /// Borne installée.
    pub e: bool,
}

/// Pied de talus d'un bassin : emprise horizontale du talus et anneaux du fond.
#[derive(Serialize, Clone, Default)]
pub struct BasinOut {
    pub run: f64,
    pub bottom: Vec<Vec<P2>>,
    /// Limite extérieure de la bande de verdure d'accès.
    pub outer: Vec<Vec<P2>>,
}

#[derive(Serialize, Default)]
pub struct Metrics {
    pub parcel_area: f64,
    pub buildable_area: f64,
    pub paved_area: f64,
    pub green_area: f64,
    pub stalls: usize,
    pub standard: usize,
    pub pmr: usize,
    pub pmr_required: usize,
    pub irve: usize,
    pub irve_required: usize,
    pub irve_equipped: usize,
    pub irve_equipped_required: usize,
    pub trees: usize,
    pub perimeter_stalls: usize,
    pub interior_stalls: usize,
    pub removed_unreachable: usize,
    pub m2_paved_per_stall: f64,
    pub m2_parcel_per_stall: f64,
}

#[derive(Serialize, Default)]
pub struct Layout {
    /// Limite de recul (anneaux).
    pub setback_line: Vec<Vec<P2>>,
    /// Voirie : anneau de circulation et allées (anneaux pair-impair).
    pub paved: Vec<Vec<P2>>,
    /// Fonds de rangées (enrobé sous les places).
    pub pads: Vec<[P2; 4]>,
    pub stalls: Vec<StallOut>,
    pub islands: Vec<[P2; 4]>,
    /// Espaces plantés (polygones à trous : extérieur puis trous), îlots d'extrémité et de bordure
    /// compris, angles arrondis côté chaussée ; `islands` ne garde que les îlots des rangées.
    pub planted: Vec<Vec<Vec<P2>>>,
    pub trees: Vec<P2>,
    pub entries: Vec<[P2; 4]>,
    pub cross: Vec<[P2; 4]>,
    /// Noues entre les demi-rangées d'une rangée double : un quadrilatère par tronçon.
    pub noues: Vec<[P2; 4]>,
    /// Pour chaque noue, vrai si elle a été supprimée (reste listée pour pouvoir la rétablir).
    #[serde(default)]
    pub noues_off: Vec<bool>,
    /// Pour chaque noue : [axe de sa rangée (début, fin), milieu du tronçon] sur la trame de référence.
    #[serde(default)]
    pub noue_ref: Vec<[[f64; 2]; 3]>,
    pub entrances: Vec<P2>,
    /// Cheminements piétons devant les portes des bâtiments.
    pub walks: Vec<[P2; 4]>,
    /// Bassins de rétention : pied de talus de chacun (même ordre que `Site::basins`).
    pub basins: Vec<BasinOut>,
    /// Sens de circulation et flèches au sol.
    pub circulation: crate::circulation::Circulation,
    pub metrics: Metrics,
    pub angle: f64,
    pub phase: f64,
    /// Largeur de l'allée entre rangées intérieures (m) : celle de Neufert si les places sont inclinées.
    pub row_aisle: f64,
    /// Largeur de voie de Neufert (p. 272) pour l'inclinaison et la largeur de place réglées (m).
    pub neufert_aisle: f64,
    pub evals: u32,
    pub warnings: Vec<String>,
}
