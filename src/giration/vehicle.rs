//! Fiche véhicule et géométrie du braquage en régime permanent.
//!
//! Repère : le véhicule tourne autour d'un centre O situé sur l'axe de l'essieu arrière ;
//! `r` désigne le rayon décrit par le milieu de l'essieu arrière (modèle bicyclette).

/// Convention de mesure d'un diamètre de braquage publié.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Convention {
    /// Entre trottoirs : bord extérieur du pneu avant extérieur.
    Kerbs,
    /// Entre murs : coin avant extérieur de la carrosserie.
    Walls,
}

/// Cotes en mètres, angles en radians, temps en secondes.
#[derive(Clone, Debug)]
pub struct Vehicle {
    pub wheelbase: f64,
    pub front_overhang: f64,
    pub rear_overhang: f64,
    /// Largeur de caisse, hors rétroviseurs.
    pub width: f64,
    /// Voie (entraxe des roues d'un même essieu).
    pub track: f64,
    pub tyre_width: f64,
    pub ground_clearance: f64,
    /// Angle de braquage maximal du modèle bicyclette (roue fictive au milieu de l'essieu avant).
    pub max_steer: f64,
    /// Temps pour passer d'une butée à l'autre.
    pub lock_to_lock: f64,
}

impl Vehicle {
    /// Véhicule de référence : Peugeot 3008 (3e génération, 2024). Cotes publiées : longueur
    /// 4,542 m, largeur hors rétroviseurs 1,895 m, empattement 2,739 m, porte-à-faux 0,940 / 0,863 m,
    /// voie avant 1,630 m (arrière 1,636 m), garde au sol 0,198 m, pneus 225 ou 235/55 R19.
    /// Diamètre de braquage publié de 10,6 à 10,9 m selon la version, sans convention : 10,9 m lu
    /// entre trottoirs (lecture prudente), pneus de 235.
    pub fn peugeot_3008() -> Vehicle {
        Vehicle {
            wheelbase: 2.739,
            front_overhang: 0.940,
            rear_overhang: 0.863,
            width: 1.895,
            track: 1.630,
            tyre_width: 0.235,
            ground_clearance: 0.198,
            max_steer: 0.0,
            lock_to_lock: 4.0,
        }
        .with_turning_diameter(10.90, Convention::Kerbs)
    }

    /// Fixe l'angle maximal à partir d'un diamètre de braquage publié.
    pub fn with_turning_diameter(mut self, diameter: f64, conv: Convention) -> Vehicle {
        let half = diameter / 2.0;
        let r = match conv {
            Convention::Kerbs => {
                let rw = half - self.tyre_width / 2.0;
                (rw * rw - self.wheelbase * self.wheelbase).sqrt() - self.track / 2.0
            }
            Convention::Walls => {
                let f = self.wheelbase + self.front_overhang;
                (half * half - f * f).sqrt() - self.width / 2.0
            }
        };
        self.max_steer = (self.wheelbase / r).atan();
        self
    }

    /// Rayon de l'essieu arrière au braquage maximal.
    pub fn min_radius(&self) -> f64 {
        self.wheelbase / self.max_steer.tan()
    }

    /// Bord extérieur du pneu avant extérieur (le pneu est tangent à son cercle).
    pub fn kerb_radius(&self, r: f64) -> f64 {
        (r + self.track / 2.0).hypot(self.wheelbase) + self.tyre_width / 2.0
    }

    /// Coin avant extérieur de la carrosserie.
    pub fn wall_radius(&self, r: f64) -> f64 {
        (r + self.width / 2.0).hypot(self.wheelbase + self.front_overhang)
    }

    /// Flanc intérieur de caisse au droit de l'essieu arrière, point le plus proche de O.
    pub fn inner_radius(&self, r: f64) -> f64 {
        r - self.width / 2.0
    }

    /// Angles des roues avant intérieure et extérieure pour un angle bicyclette `steer` (> 0).
    pub fn ackermann(&self, steer: f64) -> (f64, f64) {
        let r = self.wheelbase / steer.tan();
        let half = self.track / 2.0;
        (
            (self.wheelbase / (r - half)).atan(),
            (self.wheelbase / (r + half)).atan(),
        )
    }
}
