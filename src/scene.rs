//! Scène 3D : le plan décrit en éléments typés (emprise, cotes, matière, niveau), que la vue
//! three.js se contente d'afficher. La 3D est une vue du plan, pas un second modèle : rien de ce
//! qui est ici ne se saisit, tout se déduit du site et du plan calculé.
//!
//! Cotes en mètres, z = 0 au niveau du terrain fini. Hypothèses de représentation (aucune norme) :
//! hauteur des bâtiments, taille des arbres, hauteur des bordures et profondeur du socle.

use crate::geom::{clean_ring, signed_area, v2, V2};
use crate::ombrage::{a_trous, poly, reunion};
use crate::model::{Layout, Site, P2};
use crate::region;
use geo::{BooleanOps, LineString, MultiPolygon};
use serde::Serialize;

/// Dessus de l'enrobé, juste au-dessus du terrain.
pub const ENROBE: f64 = 0.02;
/// Vue d'une bordure : espaces plantés, îlots et trottoirs sont relevés d'autant.
pub const BORDURE: f64 = 0.15;
/// Hauteur d'un bâtiment, faute de la connaître, et acrotère qui le couronne.
pub const HAUTEUR_BATIMENT: f64 = 7.0;
const ACROTERE: (f64, f64) = (0.30, 0.60);
/// Épaisseur de terrain montrée sous le plan : le socle de la maquette.
const SOCLE: f64 = 1.5;
/// Gradins d'un talus de bassin.
const GRADINS: usize = 6;
const GARDE_CORPS: f64 = 1.10;
/// Édicule d'un escalier ou d'un ascenseur au-dessus du dernier niveau.
const EDICULE: f64 = 2.80;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Matiere {
    Terrain,
    Enrobe,
    Plante,
    Noue,
    Bassin,
    Trottoir,
    Batiment,
    Acrotere,
    Porte,
    Auvent,
    Dalle,
    Poteau,
    Poutre,
    Noyau,
    GardeCorps,
}

/// Prisme droit : emprise (contour extérieur, puis vides) entre deux cotes.
#[derive(Serialize, Clone, Debug)]
pub struct Volume {
    pub matiere: Matiere,
    pub anneaux: Vec<Vec<P2>>,
    pub z0: f64,
    pub z1: f64,
    /// Niveau porteur (0 : sol), pour l'éclaté.
    pub niveau: u32,
    /// Clé BIM de l'élément, s'il en a une.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cle: String,
}

/// Place de stationnement : `q[0]`–`q[1]` au fond, `q[2]`–`q[3]` côté allée.
#[derive(Serialize, Clone, Debug)]
pub struct Place {
    pub q: [P2; 4],
    pub z: f64,
    /// 0 standard, 1 PMR, 2 IRVE, 3 PMR + IRVE.
    pub k: u8,
    pub borne: bool,
    pub niveau: u32,
}

#[derive(Serialize, Clone, Debug)]
pub struct Arbre {
    pub p: P2,
    pub z: f64,
    /// Hauteur totale et rayon du houppier.
    pub h: f64,
    pub r: f64,
}

/// Flèche de circulation peinte au sol ; direction en degrés depuis +x.
#[derive(Serialize, Clone, Debug)]
pub struct Fleche {
    pub p: P2,
    pub a: f64,
    pub z: f64,
}

/// Rampe : axe en trois dimensions dans le sens de parcours, largeur roulable.
#[derive(Serialize, Clone, Debug)]
pub struct Rampe {
    pub axe: Vec<[f64; 3]>,
    pub largeur: f64,
    /// Niveau du bas de la rampe.
    pub niveau: u32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cle: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Niveau {
    pub z: f64,
    pub nom: String,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Scene {
    /// Boîte englobante en plan, et cote la plus haute.
    pub min: P2,
    pub max: P2,
    pub haut: f64,
    pub volumes: Vec<Volume>,
    pub places: Vec<Place>,
    pub arbres: Vec<Arbre>,
    pub fleches: Vec<Fleche>,
    pub rampes: Vec<Rampe>,
    pub niveaux: Vec<Niveau>,
}

fn r3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

fn anneau(ls: &LineString<f64>) -> Vec<P2> {
    clean_ring(&ls.0.iter().map(|c| v2(c.x, c.y)).collect::<Vec<_>>()).iter().map(|p| [r3(p.x), r3(p.y)]).collect()
}

impl Scene {
    fn prismes(&mut self, mp: &MultiPolygon<f64>, matiere: Matiere, z0: f64, z1: f64, niveau: u32) {
        for p in &mp.0 {
            let ext = anneau(p.exterior());
            if ext.len() < 3 || signed_area(&ext.iter().map(|q| v2(q[0], q[1])).collect::<Vec<_>>()).abs() < 0.05 {
                continue;
            }
            let mut anneaux = vec![ext];
            anneaux.extend(p.interiors().iter().map(anneau).filter(|r| r.len() >= 3));
            self.volumes.push(Volume { matiere, anneaux, z0, z1, niveau, cle: String::new() });
        }
    }

    fn prisme(&mut self, ring: &[P2], matiere: Matiere, z0: f64, z1: f64, niveau: u32, cle: &str) {
        self.volumes.push(Volume { matiere, anneaux: vec![ring.to_vec()], z0, z1, niveau, cle: cle.to_string() });
    }
}

/// Nombre entre 0 et 1 tiré de la position : le même d'un calcul à l'autre.
fn alea(p: P2, sel: u64) -> f64 {
    let mut x = ((p[0] * 1000.0).round() as i64 as u64) ^ ((p[1] * 1000.0).round() as i64 as u64).rotate_left(32) ^ sel.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    ((x ^ (x >> 31)) >> 11) as f64 / (1u64 << 53) as f64
}

pub fn construire(site: &Site, lay: &Layout) -> Scene {
    let mut s = Scene::default();
    let p = &site.params;
    let bassins = reunion(site.basins.iter().cloned());
    let profondeur = lay.basins.iter().map(|b| b.run / 3.0).fold(0.0, f64::max).min(p.basin_h.max(0.0));
    let socle = SOCLE.max(profondeur + 0.5);

    // Terrain : la parcelle, ouverte au droit des bassins.
    if site.parcel.len() >= 3 {
        s.prismes(&MultiPolygon::new(vec![poly(&site.parcel)]).difference(&bassins), Matiere::Terrain, -socle, 0.0, 0);
    }

    // Bassins : talus en gradins, du haut de talus jusqu'au fond.
    for (contour, b) in site.basins.iter().zip(&lay.basins) {
        if contour.len() < 3 {
            continue;
        }
        let haut = MultiPolygon::new(vec![poly(contour)]);
        let h = (b.run / 3.0).min(p.basin_h.max(0.0));
        if b.run < 0.05 || b.bottom.is_empty() {
            s.prismes(&haut, Matiere::Bassin, -socle, 0.0, 0);
            continue;
        }
        let fond = reunion(b.bottom.iter().cloned());
        let mut dehors = haut.clone();
        for j in 1..=GRADINS {
            let dedans = if j == GRADINS { fond.clone() } else { region::erode(&haut, b.run * j as f64 / GRADINS as f64) };
            s.prismes(&dehors.difference(&dedans), Matiere::Bassin, -socle, r3(-h * j as f64 / (GRADINS + 1) as f64), 0);
            dehors = dedans;
        }
        s.prismes(&fond, Matiere::Bassin, -socle, -h, 0);
    }

    // Enrobé : voirie, fonds de rangées, allées transversales, accès et places, d'un seul tenant.
    // Sous un parc en ouvrage, la dalle basse le remplace.
    let mut enrobe = crate::ombrage::parc(lay);
    if let Some(o) = &lay.ouvrage {
        enrobe = enrobe.difference(&MultiPolygon::new(vec![poly(&o.dalle)]));
    }
    s.prismes(&enrobe, Matiere::Enrobe, 0.0, ENROBE, 0);

    // Bâtiments, leur acrotère, leur porte et son auvent.
    let batiments = reunion(site.buildings.iter().map(|b| b.poly.clone()));
    for b in &site.buildings {
        if b.poly.len() < 3 {
            continue;
        }
        let emprise = MultiPolygon::new(vec![poly(&b.poly)]);
        s.prismes(&emprise, Matiere::Batiment, 0.0, HAUTEUR_BATIMENT, 0);
        s.prismes(&emprise.difference(&region::erode(&emprise, ACROTERE.0)), Matiere::Acrotere, HAUTEUR_BATIMENT, HAUTEUR_BATIMENT + ACROTERE.1, 0);
        if let Some(d) = b.door {
            let (o, n) = (v2(d.p[0], d.p[1]), v2(d.n[0], d.n[1]));
            let t = v2(-n.y, n.x);
            let boite = |demi: f64, a: f64, b: f64| -> Vec<P2> {
                [(-demi, a), (demi, a), (demi, b), (-demi, b)].iter().map(|&(u, w)| { let q = o + t * u + n * w; [r3(q.x), r3(q.y)] }).collect()
            };
            s.prisme(&boite(d.w / 2.0, -0.05, 0.06), Matiere::Porte, 0.0, 2.5, 0, "");
            s.prisme(&boite(d.w / 2.0 + 0.6, 0.0, 1.6), Matiere::Auvent, 2.7, 2.85, 0, "");
        }
    }

    // Espaces plantés : espaces verts imposés, plantations et îlots, relevés d'une bordure.
    let plante = a_trous(lay.planted.iter().cloned())
        .union(&reunion(lay.islands.iter().map(|q| q.to_vec()).chain(site.landscapes.iter().cloned())))
        .difference(&bassins)
        .difference(&batiments);
    s.prismes(&plante, Matiere::Plante, 0.0, BORDURE, 0);

    // Trottoir autour des bâtiments, cheminements devant les portes.
    if !batiments.0.is_empty() && p.building_margin > 0.05 && site.parcel.len() >= 3 {
        let trottoir = region::grow(&batiments, p.building_margin)
            .intersection(&MultiPolygon::new(vec![poly(&site.parcel)]))
            .difference(&batiments)
            .difference(&plante)
            .difference(&bassins);
        s.prismes(&trottoir, Matiere::Trottoir, 0.0, BORDURE, 0);
    }
    for q in &lay.walks {
        s.prisme(q, Matiere::Trottoir, 0.0, ENROBE + 0.01, 0, "");
    }

    // Noues conservées.
    for (i, q) in lay.noues.iter().enumerate() {
        if !lay.noues_off.get(i).copied().unwrap_or(false) {
            s.prisme(q, Matiere::Noue, 0.0, ENROBE + 0.03, 0, "");
        }
    }

    s.arbres = lay.trees.iter().map(|&t| Arbre { p: t, z: BORDURE, h: r3(5.5 + 3.0 * alea(t, 1)), r: r3(1.8 + 0.9 * alea(t, 2)) }).collect();
    s.fleches = lay.circulation.arrows.iter().map(|a| Fleche { p: a.p, a: a.a, z: ENROBE }).collect();

    match &lay.ouvrage {
        Some(o) => ouvrage(&mut s, lay, o),
        None => {
            s.places = lay.stalls.iter().map(|st| Place { q: st.q, z: ENROBE, k: st.k, borne: st.e, niveau: 0 }).collect();
            s.niveaux = vec![Niveau { z: 0.0, nom: "Sol".into() }];
        }
    }

    let (mut min, mut max) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for q in s.volumes.iter().flat_map(|v| v.anneaux.iter().flatten()) {
        for i in 0..2 {
            min[i] = min[i].min(q[i]);
            max[i] = max[i].max(q[i]);
        }
    }
    if min[0].is_finite() {
        (s.min, s.max) = (min, max);
    }
    s.haut = s.volumes.iter().map(|v| v.z1).fold(0.0, f64::max);
    s
}

/// Parc en ouvrage : les niveaux empilés. Chaque niveau reprend le plateau courant, le rez et le
/// dernier niveau avec leurs places en plus ; le dernier est à ciel ouvert. Demi-niveaux et plateaux inclinés sont montrés en niveaux plans.
fn ouvrage(s: &mut Scene, lay: &Layout, o: &crate::ouvrage::OuvrageOut) {
    let (h, ep, niv) = (o.hauteur_etage, o.dalle_ep, o.niveaux.max(1));
    let dalle = MultiPolygon::new(vec![poly(&o.dalle)]);
    let tremies = reunion(o.rampes.iter().map(|r| r.contour.clone()));
    let cote = |k: u32| if k == 0 { ENROBE } else { r3(k as f64 * h) };
    for k in 0..niv {
        let z = k as f64 * h;
        s.niveaux.push(Niveau { z: r3(z), nom: if k == 0 { "RDC".into() } else { format!("R+{k}") } });
        if k == 0 {
            s.prismes(&dalle, Matiere::Dalle, -0.20, ENROBE, 0);
        } else {
            s.prismes(&dalle.difference(&tremies), Matiere::Dalle, r3(z - ep), r3(z), k);
            s.prismes(&dalle.difference(&region::erode(&dalle, 0.15)), Matiere::GardeCorps, r3(z), r3(z + GARDE_CORPS), k);
        }
        s.places.extend(lay.stalls.iter().map(|st| Place { q: st.q, z: cote(k), k: st.k, borne: st.e, niveau: k }));
        // Places propres au rez et au dernier niveau.
        if niv > 1 {
            let seules = if k == 0 { &o.places_rdc_seul[..] } else if k + 1 == niv { &o.places_dernier_seul[..] } else { &[] };
            s.places.extend(seules.iter().map(|&q| Place { q, z: cote(k), k: 0, borne: false, niveau: k }));
        }
        // Escaliers et ascenseurs : un tronçon par niveau, un édicule au-dessus du dernier.
        if niv > 1 {
            for n in &o.noyaux {
                let z1 = if k + 1 == niv { z + EDICULE } else { z + h };
                s.prisme(&n.q, Matiere::Noyau, r3(z), r3(z1), k, &n.cle);
            }
        }
        if k + 1 == niv {
            break;
        }
        // Structure qui porte la dalle du dessus.
        let sous = r3(z + h - ep);
        for (q, cle) in o.poteaux.iter().zip(&o.poteaux_cles) {
            s.prisme(q, Matiere::Poteau, cote(k), sous, k, cle);
        }
        for (q, cle) in o.poutres.iter().zip(&o.poutres_cles) {
            s.prisme(q, Matiere::Poutre, r3(sous - o.plancher.retombee.max(0.05)), sous, k, cle);
        }
        for r in &o.rampes {
            if r.axe.len() < 2 {
                continue;
            }
            let pts: Vec<V2> = r.axe.iter().map(|q| v2(q[0], q[1])).collect();
            let cumul: Vec<f64> = std::iter::once(0.0).chain(pts.windows(2).scan(0.0, |l, w| { *l += (w[1] - w[0]).len(); Some(*l) })).collect();
            let total = cumul[cumul.len() - 1].max(1e-6);
            let axe = pts
                .iter()
                .zip(&cumul)
                .map(|(q, l)| { let t = l / total; [r3(q.x), r3(q.y), r3(z + h * if r.montee { t } else { 1.0 - t })] })
                .collect();
            let aire = |ring: &[P2]| signed_area(&ring.iter().map(|q| v2(q[0], q[1])).collect::<Vec<_>>()).abs();
            let largeur = ((aire(&r.contour) - aire(&r.trou)) / total).clamp(2.5, 8.0);
            s.rampes.push(Rampe { axe, largeur: r3(largeur), niveau: k, cle: r.cle.clone() });
        }
    }
}
