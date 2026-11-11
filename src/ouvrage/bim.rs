//! Modèle BIM du plateau : éléments typés (classe IFC, type prédéfini, type, propriétés), géométrie
//! par extrusion verticale et identifiants stables.
//!
//! Identifiant : le `GlobalId` IFC (22 caractères) est dérivé d'une clé métier (niveau, file de
//! trame, rangée, rang) et d'une graine de projet. Un même plan redonne les mêmes identifiants, si
//! bien qu'un nouvel export met à jour les éléments déjà liés au lieu d'en créer d'autres.

use super::OuvrageOut;
use crate::geom::{signed_area, v2, V2};
use crate::model::{Layout, P2};

/// Valeur de propriété, avec son type IFC.
#[derive(Clone, Debug, PartialEq)]
pub enum Valeur {
    Label(String),
    Texte(String),
    Booleen(bool),
    Entier(i64),
    Compte(f64),
    Longueur(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Prop {
    pub pset: &'static str,
    pub nom: &'static str,
    pub val: Valeur,
}

fn prop(pset: &'static str, nom: &'static str, val: Valeur) -> Prop {
    Prop { pset, nom, val }
}

/// Type d'élément : l'équivalent du couple famille et type de Revit.
#[derive(Clone, Debug, PartialEq)]
pub struct TypeBim {
    pub classe: &'static str,
    pub predefini: &'static str,
    pub nom: String,
}

#[derive(Clone, Debug)]
pub struct Element {
    pub cle: String,
    pub guid: String,
    pub classe: &'static str,
    pub predefini: &'static str,
    /// Indice dans `Modele::types`.
    pub type_i: usize,
    pub nom: String,
    /// Profil horizontal (repère du plan), extrudé de `z0` à `z0 + h` par rapport au niveau.
    pub profil: Vec<V2>,
    pub z0: f64,
    pub h: f64,
    pub props: Vec<Prop>,
}

#[derive(Clone, Debug)]
pub struct Axe {
    pub nom: String,
    pub a: V2,
    pub b: V2,
}

#[derive(Clone, Debug)]
pub struct Modele {
    pub graine: String,
    pub nom: String,
    pub cote: f64,
    pub niveau: String,
    pub types: Vec<TypeBim>,
    pub elements: Vec<Element>,
    pub axes_u: Vec<Axe>,
    pub axes_v: Vec<Axe>,
}

impl Modele {
    pub fn guid(&self, cle: &str) -> String {
        guid(&self.graine, cle)
    }
}

// ---------------------------------------------------------------------------
// Identifiants.

const B64: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";

fn fnv1a(seed: u64, data: &[u8]) -> u64 {
    let mut h = seed;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// 128 bits déterministes pour (graine, clé), au format d'un UUID version 4 (bits de version et
/// de variante fixés).
pub fn uuid(graine: &str, cle: &str) -> [u8; 16] {
    let mut data = Vec::with_capacity(graine.len() + cle.len() + 1);
    data.extend_from_slice(graine.as_bytes());
    data.push(0x1f);
    data.extend_from_slice(cle.as_bytes());
    let a = splitmix(fnv1a(0xcbf2_9ce4_8422_2325, &data));
    let b = splitmix(fnv1a(0x8422_2325_cbf2_9ce4 ^ a, &data));
    let mut u = [0u8; 16];
    u[..8].copy_from_slice(&a.to_be_bytes());
    u[8..].copy_from_slice(&b.to_be_bytes());
    u[6] = (u[6] & 0x0f) | 0x40;
    u[8] = (u[8] & 0x3f) | 0x80;
    u
}

/// Compression IFC d'un UUID en 22 caractères (même algorithme qu'IfcOpenShell).
pub fn compresser(u: &[u8; 16]) -> String {
    let mut s = String::with_capacity(22);
    let b64 = |v: u32, n: usize, s: &mut String| {
        for i in (0..n).rev() {
            s.push(B64[((v >> (6 * i)) & 63) as usize] as char);
        }
    };
    b64(u[0] as u32, 2, &mut s);
    for i in (1..16).step_by(3) {
        b64(((u[i] as u32) << 16) | ((u[i + 1] as u32) << 8) | u[i + 2] as u32, 4, &mut s);
    }
    s
}

pub fn guid(graine: &str, cle: &str) -> String {
    compresser(&uuid(graine, cle))
}

// ---------------------------------------------------------------------------
// Construction du modèle à partir du plan calculé.

fn pt(p: &P2) -> V2 {
    v2(p[0], p[1])
}

fn ccw(mut r: Vec<V2>) -> Vec<V2> {
    if signed_area(&r) < 0.0 {
        r.reverse();
    }
    r
}

fn cm(x: f64) -> i64 {
    (x * 100.0).round() as i64
}

/// Modèle du niveau unique du lot 1. `None` si le plan n'est pas un ouvrage.
pub fn modele(lay: &Layout, graine: &str, largeur_place: f64, longueur_place: f64, allee: f64) -> Option<Modele> {
    let o: &OuvrageOut = lay.ouvrage.as_ref()?;
    let niveau = "N00".to_string();
    let h_libre = (o.hauteur_etage - o.dalle_ep).max(0.0);
    let mut types: Vec<TypeBim> = Vec::new();
    let mut ty = |classe: &'static str, predefini: &'static str, nom: String| -> usize {
        if let Some(i) = types.iter().position(|t| t.nom == nom) {
            return i;
        }
        types.push(TypeBim { classe, predefini, nom });
        types.len() - 1
    };
    let t_dalle = ty("IfcSlab", "FLOOR", format!("PKM_Plancher_{}", o.plancher.code));
    let t_toit = ty("IfcSlab", "ROOF", format!("PKM_Toiture_{}", o.plancher.code));
    let t_place = ty(
        "IfcSpace",
        "PARKING",
        format!("PKM_Place_{}x{}", cm(largeur_place), cm(longueur_place)),
    );
    let t_allee = ty("IfcSpace", "PARKING", format!("PKM_Allee_{}", cm(allee)));
    let mut t_poteaux: Vec<usize> = Vec::new();
    for q in &o.poteaux {
        let (a, b) = (pt(&q[0]), pt(&q[1]));
        let (c, d) = (pt(&q[1]), pt(&q[2]));
        let (l1, l2) = ((b - a).len(), (d - c).len());
        t_poteaux.push(ty(
            "IfcColumn",
            "COLUMN",
            format!("PKM_Poteau_{}_{}x{}", o.poteau_materiau, cm(l1.min(l2)), cm(l1.max(l2))),
        ));
    }
    // Poutres ou solives du système constructif : type nommé par le profil.
    let ossature = o.plancher.ossature.as_ref().map(|os| {
        let (pre, quoi) = if os.solive { ("JOIST", "Solive") } else { ("BEAM", "Poutre") };
        (ty("IfcBeam", pre, format!("PKM_{quoi}_{}", os.profil)), pre, quoi)
    });
    let types_final = types;

    let g = |cle: &str| guid(graine, &format!("{niveau}/{cle}"));
    let dalle: Vec<V2> = ccw(o.dalle.iter().map(pt).collect());
    let mut elements = Vec::new();
    let porteur = || {
        vec![
            prop("Pset_SlabCommon", "LoadBearing", Valeur::Booleen(true)),
            prop("Pset_SlabCommon", "IsExternal", Valeur::Booleen(false)),
        ]
    };
    elements.push(Element {
        cle: "DALLE".into(),
        guid: g("DALLE"),
        classe: "IfcSlab",
        predefini: "FLOOR",
        type_i: t_dalle,
        nom: "Dalle N00".into(),
        profil: dalle.clone(),
        z0: -o.dalle_ep,
        h: o.dalle_ep,
        props: porteur(),
    });
    elements.push(Element {
        cle: "TOITURE".into(),
        guid: g("TOITURE"),
        classe: "IfcSlab",
        predefini: "ROOF",
        type_i: t_toit,
        nom: "Toiture".into(),
        profil: dalle,
        z0: h_libre,
        h: o.dalle_ep,
        props: porteur(),
    });
    for ((q, cle), &ti) in o.poteaux.iter().zip(&o.poteaux_cles).zip(&t_poteaux) {
        elements.push(Element {
            cle: cle.clone(),
            guid: g(cle),
            classe: "IfcColumn",
            predefini: "COLUMN",
            type_i: ti,
            nom: format!("Poteau {}", cle.trim_start_matches("P/")),
            profil: ccw(q.iter().map(pt).collect()),
            z0: 0.0,
            h: h_libre,
            props: vec![prop("Pset_ColumnCommon", "LoadBearing", Valeur::Booleen(true))],
        });
    }
    // Poutres : la retombée, sous la toiture (le plancher est déjà la dalle).
    if let Some((ti, pre, quoi)) = ossature {
        let r = o.plancher.retombee;
        for (q, cle) in o.poutres.iter().zip(&o.poutres_cles) {
            elements.push(Element {
                cle: cle.clone(),
                guid: g(cle),
                classe: "IfcBeam",
                predefini: pre,
                type_i: ti,
                nom: format!("{quoi} {}", cle.trim_start_matches("PO/")),
                profil: ccw(q.iter().map(pt).collect()),
                z0: h_libre - r,
                h: r,
                props: vec![prop("Pset_BeamCommon", "LoadBearing", Valeur::Booleen(true))],
            });
        }
    }
    for (s, cle) in lay.stalls.iter().zip(&o.places_cles) {
        let k = format!("PL/{cle}");
        elements.push(Element {
            guid: g(&k),
            cle: k,
            classe: "IfcSpace",
            predefini: "PARKING",
            type_i: t_place,
            nom: format!("Place {cle}"),
            profil: ccw(s.q.iter().map(pt).collect()),
            z0: 0.0,
            h: h_libre,
            props: vec![
                prop("Pset_SpaceParking", "ParkingUse", Valeur::Label("Voiture".into())),
                prop("Pset_SpaceParking", "ParkingUnits", Valeur::Compte(1.0)),
                prop("Pset_SpaceParking", "IsAisle", Valeur::Booleen(false)),
                prop("PKM_Stationnement", "CategorieNF", Valeur::Entier(1)),
                prop("PKM_Stationnement", "Largeur", Valeur::Longueur(largeur_place)),
                prop("PKM_Stationnement", "Longueur", Valeur::Longueur(longueur_place)),
                prop("PKM_Stationnement", "Reference", Valeur::Texte("NF P 91-100, tab. 2".into())),
            ],
        });
    }
    for (q, cle) in o.allees.iter().zip(&o.allees_cles) {
        elements.push(Element {
            cle: cle.clone(),
            guid: g(cle),
            classe: "IfcSpace",
            predefini: "PARKING",
            type_i: t_allee,
            nom: format!("Allée {}", cle.trim_start_matches("AL/")),
            profil: ccw(q.iter().map(pt).collect()),
            z0: 0.0,
            h: h_libre,
            props: vec![
                prop("Pset_SpaceParking", "IsAisle", Valeur::Booleen(true)),
                prop("Pset_SpaceParking", "IsOneWay", Valeur::Booleen(false)),
                prop("PKM_Stationnement", "Largeur", Valeur::Longueur(allee)),
            ],
        });
    }
    let (mut axes_u, mut axes_v) = (Vec::new(), Vec::new());
    for a in &o.axes {
        let axe = Axe {
            nom: a.nom.clone(),
            a: pt(&a.a),
            b: pt(&a.b),
        };
        if a.nom.chars().all(|c| c.is_ascii_digit()) {
            axes_u.push(axe);
        } else {
            axes_v.push(axe);
        }
    }
    Some(Modele {
        graine: graine.to_string(),
        nom: o.nom.clone(),
        cote: o.cote,
        niveau,
        types: types_final,
        elements,
        axes_u,
        axes_v,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compression_ifc() {
        // UUID nul et UUID plein : valeurs connues de la compression IFC.
        assert_eq!(compresser(&[0u8; 16]), "0000000000000000000000");
        assert_eq!(compresser(&[0xffu8; 16]), "3$$$$$$$$$$$$$$$$$$$$$");
        let g = guid("projet", "N00/PL/M1A/1");
        assert_eq!(g.len(), 22);
        assert_eq!(g, guid("projet", "N00/PL/M1A/1"));
        assert_ne!(g, guid("projet", "N00/PL/M1A/2"));
        assert_ne!(g, guid("autre", "N00/PL/M1A/1"));
        assert!(g.as_bytes()[0] <= b'3');
    }
}
