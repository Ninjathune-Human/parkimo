//! Accumulateur commun du plan en cours de génération.

use crate::geom::{Quad, QuadHash, V2};

pub const KIND_STD: u8 = 0;
pub const KIND_PMR: u8 = 1;
pub const KIND_IRVE: u8 = 2;
pub const KIND_PMR_IRVE: u8 = 3;

#[derive(Clone)]
pub struct Stall {
    pub q: Quad,
    pub kind: u8,
    /// Identifiant de rangée et rang dans la rangée (consécutifs = adjacents).
    pub row: u32,
    pub idx: i32,
    pub perimeter: bool,
    /// Borne installée (sinon, pour une place IRVE : simple pré-équipement).
    pub equipped: bool,
}

impl Stall {
    /// Point d'accès : milieu de la face avant, décalé de `t` dans l'allée.
    pub fn access_point(&self, t: f64) -> V2 {
        let front = (self.q.0[2] + self.q.0[3]) * 0.5;
        let n = (self.q.0[3] - self.q.0[0]).norm();
        front + n * t
    }
}

pub struct Acc {
    pub stalls: Vec<Stall>,
    pub hash: QuadHash,
    pub access: QuadHash,
    pub islands: Vec<Quad>,
    pub trees: Vec<V2>,
    pub pads: Vec<Quad>,
    pub cross: Vec<Quad>,
    /// Noues entre demi-rangées (un quadrilatère par tronçon continu).
    pub noues: Vec<Quad>,
    /// Emprises bloquant la circulation (rangées intérieures, îlots).
    pub blockers: Vec<Quad>,
    pub next_row: u32,
    /// Repères des noues supprimées (milieux d'axe, repère monde).
    pub noue_off: Vec<[f64; 2]>,
    /// Pour chaque noue : axe de sa rangée et milieu du tronçon sur la trame de référence (noues partout).
    pub noue_ref: Vec<[[f64; 2]; 3]>,
}

impl Acc {
    pub fn new() -> Acc {
        Acc {
            stalls: Vec::new(),
            hash: QuadHash::new(6.0),
            access: QuadHash::new(6.0),
            islands: Vec::new(),
            trees: Vec::new(),
            pads: Vec::new(),
            cross: Vec::new(),
            noues: Vec::new(),
            blockers: Vec::new(),
            next_row: 0,
            noue_off: Vec::new(),
            noue_ref: Vec::new(),
        }
    }
    pub fn new_row(&mut self) -> u32 {
        self.next_row += 1;
        self.next_row
    }
}
