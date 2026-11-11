//! Le projet tel que l'interface l'enregistre, et sa conversion en entrée du moteur.
//!
//! Le projet garde le format de l'interface (`site` : parcelle, arcs, reculs, accès, objets ;
//! `params` : réglages, parts en pour cent, silo en clés à plat). La conversion reproduit celle de
//! la page (`engineInput`) : arcs discrétisés, bâtiments en polygones, porte et entrée, silo.
//! `tests/projet.rs` la compare à l'interface sur des projets de référence.

use crate::model::{Layout, Site};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

/// Valeur par défaut d'un réglage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Defaut {
    N(f64),
    B(bool),
    S(&'static str),
    Nul,
}

/// Nature d'un réglage et valeurs admises par une commande.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Genre {
    Nombre { min: f64, max: f64 },
    Entier { min: f64, max: f64 },
    Choix(&'static [&'static str]),
    Bool,
    /// Orientation des rangées en degrés, ou `null` pour la laisser au moteur.
    Orientation,
    /// Inclinaison des places : 30, 45, 60, 75 ou 90 degrés.
    Inclinaison,
}

/// Réglage du projet : clé de l'interface, libellé, unité, valeur par défaut, valeurs admises.
#[derive(Clone, Copy, Debug)]
pub struct Reglage {
    pub cle: &'static str,
    pub libelle: &'static str,
    pub unite: &'static str,
    pub defaut: Defaut,
    pub genre: Genre,
}

const fn nombre(cle: &'static str, libelle: &'static str, unite: &'static str, defaut: f64, min: f64, max: f64) -> Reglage {
    Reglage { cle, libelle, unite, defaut: Defaut::N(defaut), genre: Genre::Nombre { min, max } }
}

pub const IMPLANTATIONS: &[&str] = &[
    "auto", "rangee", "pignon", "demi", "helice", "inclines", "helice_ext", "helice_double", "helice_facade", "helice_centre", "helice_anneau",
];

/// Réglages du projet : valeurs par défaut de l'interface, bornes admises par les commandes.
pub const REGLAGES: &[Reglage] = &[
    nombre("stall_w", "Largeur de place", "m", 2.5, 1.8, 4.0),
    nombre("stall_d", "Longueur de place", "m", 5.0, 3.5, 8.0),
    nombre("noue_w", "Noue entre rangées", "m", 0.0, 0.0, 6.0),
    nombre("basin_h", "Hauteur du bassin de rétention", "m", 1.5, 0.0, 5.0),
    nombre("basin_margin", "Verdure autour du bassin", "m", 2.0, 0.0, 10.0),
    nombre("aisle", "Largeur d'allée", "m", 6.0, 2.5, 12.0),
    nombre("aisle_one", "Largeur de voie en sens unique", "m", 5.0, 2.5, 12.0),
    nombre("entry_w", "Largeur de l'entrée", "m", 6.0, 2.5, 15.0),
    nombre("building_margin", "Trottoir autour du bâti", "m", 2.0, 0.0, 10.0),
    nombre("door_w", "Largeur d'entrée du bâtiment", "m", 3.0, 0.5, 10.0),
    Reglage { cle: "planter_every", libelle: "Îlot planté toutes les", unite: "places", defaut: Defaut::N(8.0), genre: Genre::Entier { min: 0.0, max: 100.0 } },
    nombre("cap", "Îlot en bout de rangée", "m", 1.5, 0.0, 10.0),
    nombre("pmr_ratio", "Places PMR", "%", 2.0, 0.0, 100.0),
    nombre("irve_ratio", "Pré-équipement IRVE", "%", 20.0, 0.0, 100.0),
    nombre("irve_equip_ratio", "Équipement IRVE (bornes)", "%", 5.0, 0.0, 100.0),
    Reglage { cle: "angle", libelle: "Orientation des rangées", unite: "°", defaut: Defaut::Nul, genre: Genre::Orientation },
    Reglage { cle: "slant", libelle: "Inclinaison des places", unite: "°", defaut: Defaut::N(90.0), genre: Genre::Inclinaison },
    Reglage { cle: "programme", libelle: "Programme", unite: "", defaut: Defaut::S("public"), genre: Genre::Choix(&["public", "privatif"]) },
    Reglage { cle: "silo", libelle: "Parc en ouvrage (silo)", unite: "", defaut: Defaut::B(false), genre: Genre::Bool },
    nombre("silo_etage", "Hauteur d'étage", "m", 2.9, 2.2, 4.5),
    Reglage { cle: "silo_niveaux", libelle: "Niveaux", unite: "niv.", defaut: Defaut::N(1.0), genre: Genre::Entier { min: 1.0, max: 12.0 } },
    nombre("silo_pente", "Pente des rampes", "%", 18.0, 5.0, 18.0),
    Reglage { cle: "silo_systeme", libelle: "Système constructif", unite: "", defaut: Defaut::S("coule"), genre: Genre::Choix(&["coule", "prefa", "mixte"]) },
    nombre("silo_pu", "Poteau le long des allées", "m", 0.3, 0.05, 1.5),
    nombre("silo_pv", "Poteau en travers", "m", 0.6, 0.05, 1.5),
    Reglage { cle: "silo_travee", libelle: "Places par travée", unite: "places", defaut: Defaut::N(3.0), genre: Genre::Entier { min: 1.0, max: 6.0 } },
    nombre("silo_facade", "Rive de façade", "m", 0.2, 0.0, 1.0),
    Reglage { cle: "silo_rives", libelle: "Places de rive", unite: "", defaut: Defaut::B(true), genre: Genre::Bool },
    Reglage { cle: "silo_densite", libelle: "Portée de la trame", unite: "", defaut: Defaut::S("longue"), genre: Genre::Choix(&["longue", "moyenne", "courte"]) },
    nombre("silo_retrait", "Retrait des poteaux", "m", 0.65, 0.0, 2.0),
    Reglage { cle: "silo_rampes", libelle: "Implantation des rampes", unite: "", defaut: Defaut::S("auto"), genre: Genre::Choix(IMPLANTATIONS) },
    Reglage { cle: "silo_ventile", libelle: "Parc largement ventilé", unite: "", defaut: Defaut::B(true), genre: Genre::Bool },
    Reglage { cle: "silo_asc", libelle: "Ascenseurs des demi-niveaux", unite: "", defaut: Defaut::S("deux"), genre: Genre::Choix(&["deux", "liaison"]) },
];

impl Defaut {
    pub fn json(self) -> Value {
        match self {
            Defaut::N(x) => json!(x),
            Defaut::B(b) => json!(b),
            Defaut::S(s) => json!(s),
            Defaut::Nul => Value::Null,
        }
    }
}

/// Projet : `{ site, params }`, dans le format de l'interface. Les champs inconnus sont conservés.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(transparent)]
pub struct Projet(pub Value);

type Pt = [f64; 2];

fn num(v: Option<&Value>) -> f64 {
    v.and_then(Value::as_f64).unwrap_or(0.0)
}

/// Sommets d'un contour : `[x, y]` ou `[x, y, k]` (flèche de l'arc qui part du sommet).
fn sommets(v: Option<&Value>) -> Vec<[f64; 3]> {
    v.and_then(Value::as_array)
        .map(|a| a.iter().map(|p| [num(p.get(0)), num(p.get(1)), num(p.get(2))]).collect())
        .unwrap_or_default()
}

fn liste(v: Option<&Value>) -> &[Value] {
    v.and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn aire(p: &[[f64; 3]]) -> f64 {
    let n = p.len();
    (0..n).map(|i| p[i][0] * p[(i + 1) % n][1] - p[(i + 1) % n][0] * p[i][1]).sum::<f64>() / 2.0
}

/// `Math.sign`, zéro compris.
fn signe(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// `Math.round` : au plus proche, les demis vers le haut.
fn arrondi(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// Flèche maximale d'un arc, en fraction de la corde.
pub const ARC_MAX: f64 = 0.5;

/// Arête d'un contour : segment droit, ou arc de cercle de flèche `k` (fraction de la corde,
/// positive vers l'extérieur). `n` : nombre de côtés de l'arc discrétisé, un tous les 4° environ.
pub struct Arete {
    pub a: Pt,
    pub b: Pt,
    pub l: f64,
    pub n: usize,
    arc: Option<Arc>,
}

struct Arc {
    c: Pt,
    r: f64,
    a0: f64,
    balayage: f64,
}

impl Arete {
    /// Arête `i` du contour `p`, de flèche `k0`.
    pub fn de(p: &[[f64; 3]], i: usize, k0: f64) -> Arete {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        let s0 = signe(aire(p));
        let sgn = if s0 == 0.0 { 1.0 } else { s0 };
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let h = dx.hypot(dy);
        let l = if h == 0.0 { 1.0 } else { h };
        let u = [dx / l, dy / l];
        let n = [-u[1] * sgn, u[0] * sgn];
        let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
        let k = k0.clamp(-ARC_MAX, ARC_MAX);
        let mut e = Arete { a: [a[0], a[1]], b: [b[0], b[1]], l, n: 1, arc: None };
        if k == 0.0 {
            return e;
        }
        let s = k * l;
        let o = [-n[0], -n[1]];
        let tot = 4.0 * (2.0 * k.abs()).atan();
        let r = (l * l / 4.0 + s * s) / (2.0 * s.abs());
        let dc = s - signe(s) * r;
        let c = [mid[0] + o[0] * dc, mid[1] + o[1] * dc];
        let a0 = (a[1] - c[1]).atan2(a[0] - c[0]);
        let am = (mid[1] + o[1] * s - c[1]).atan2(mid[0] + o[0] * s - c[0]);
        let d0 = signe((am - a0).sin());
        let dir = if d0 == 0.0 { 1.0 } else { d0 };
        e.n = ((tot * 45.0 / std::f64::consts::PI).ceil() as usize).clamp(2, 48);
        e.arc = Some(Arc { c, r, a0, balayage: dir * tot });
        e
    }

    /// Point à la fraction `t` de l'arête (de l'angle, pour un arc).
    pub fn at(&self, t: f64) -> Pt {
        match &self.arc {
            None => [self.a[0] + (self.b[0] - self.a[0]) * t, self.a[1] + (self.b[1] - self.a[1]) * t],
            Some(_) if t <= 0.0 => self.a,
            Some(_) if t >= 1.0 => self.b,
            Some(arc) => {
                let f = arc.a0 + arc.balayage * t;
                [arc.c[0] + arc.r * f.cos(), arc.c[1] + arc.r * f.sin()]
            }
        }
    }
}

/// Contour discrétisé d'un objet polygonal dont les sommets portent leur flèche.
fn contour(p: &[[f64; 3]]) -> Vec<Pt> {
    let mut out = Vec::new();
    for i in 0..p.len() {
        let e = Arete::de(p, i, p[i][2]);
        for j in 0..e.n {
            out.push(if j == 0 { e.a } else { e.at(j as f64 / e.n as f64) });
        }
    }
    out
}

/// Polygone d'un bâtiment : rectangle ou forme en L, tourné autour de son centre.
fn emprise_batiment(b: &Value) -> Vec<[f64; 3]> {
    let rot = num(b.get("rot")).to_radians();
    let (c, s) = (rot.cos(), rot.sin());
    let (w, h) = (num(b.get("w")) / 2.0, num(b.get("h")) / 2.0);
    let (cx, cy) = (num(b.get("cx")), num(b.get("cy")));
    let local: Vec<Pt> = if b.get("shape").and_then(Value::as_str) == Some("L") {
        let t = (num(b.get("w")).min(num(b.get("h"))) * 0.4).max(6.0);
        vec![[-w, -h], [w, -h], [w, -h + t], [-w + t, -h + t], [-w + t, h], [-w, h]]
    } else {
        vec![[-w, -h], [w, -h], [w, h], [-w, h]]
    };
    local.iter().map(|&[x, y]| [cx + x * c - y * s, cy + x * s + y * c, 0.0]).collect()
}

impl Projet {
    /// Lit un projet : une parcelle (liste de points) est exigée ; les réglages absents prennent
    /// leur valeur par défaut et les flèches des limites sont alignées sur les sommets.
    pub fn lire(mut v: Value) -> Result<Projet, String> {
        let site = v.get("site").and_then(Value::as_object).ok_or("projet sans site")?;
        let parcelle = site.get("parcel").and_then(Value::as_array).ok_or("site sans parcelle")?;
        let point = |p: &Value| p.as_array().is_some_and(|a| a.len() >= 2 && a[0].is_number() && a[1].is_number());
        if !parcelle.iter().all(point) {
            return Err("parcelle : chaque sommet est un couple [x, y]".into());
        }
        let n = parcelle.len();
        let mut arcs: Vec<Value> = liste(site.get("arcs")).to_vec();
        arcs.resize(n, json!(0));
        v["site"]["arcs"] = Value::Array(arcs);
        if !v.get("params").is_some_and(Value::is_object) {
            v["params"] = json!({});
        }
        let params = v["params"].as_object_mut().unwrap();
        for r in REGLAGES {
            params.entry(r.cle).or_insert_with(|| r.defaut.json());
        }
        Ok(Projet(v))
    }

    pub fn site(&self) -> &Value {
        &self.0["site"]
    }

    pub fn params(&self) -> &Map<String, Value> {
        self.0["params"].as_object().expect("réglages")
    }

    fn reglage(&self, cle: &str) -> f64 {
        num(self.params().get(cle))
    }

    fn parcelle(&self) -> Vec<[f64; 3]> {
        sommets(self.site().get("parcel"))
    }

    /// Limite `i` de la parcelle, droite ou courbe.
    pub fn limite(&self, i: usize) -> Arete {
        Arete::de(&self.parcelle(), i, num(self.site()["arcs"].get(i)))
    }

    /// Porte d'un bâtiment : point sur le mur, normale sortante, longueur du mur. Sans porte
    /// saisie, le milieu du mur le plus proche du premier accès (ou du centre de la parcelle).
    fn porte(&self, b: &Value) -> (Pt, Pt, f64) {
        let p = emprise_batiment(b);
        let n = p.len();
        let s0 = signe(aire(&p));
        let sgn = if s0 == 0.0 { 1.0 } else { s0 };
        let parcelle = self.parcelle();
        let saisie = b.get("door").filter(|d| d.is_object() && (num(d.get("e")) as usize) < n);
        let (e, t0) = match saisie {
            Some(d) => (num(d.get("e")) as usize, num(d.get("t"))),
            None => {
                let acces = liste(self.site().get("entries")).first().filter(|en| (num(en.get("edge")) as usize) < parcelle.len());
                let repere = match acces {
                    Some(en) => self.limite(num(en.get("edge")) as usize).at(num(en.get("t"))),
                    None => {
                        let m = parcelle.len().max(1) as f64;
                        parcelle.iter().fold([0.0, 0.0], |c, q| [c[0] + q[0] / m, c[1] + q[1] / m])
                    }
                };
                let d2 = |i: usize| {
                    let (a, c) = (p[i], p[(i + 1) % n]);
                    ((a[0] + c[0]) / 2.0 - repere[0]).powi(2) + ((a[1] + c[1]) / 2.0 - repere[1]).powi(2)
                };
                let mut proche = 0;
                for i in 1..n {
                    if d2(i) < d2(proche) {
                        proche = i;
                    }
                }
                (proche, 0.5)
            }
        };
        let (a, c) = (p[e], p[(e + 1) % n]);
        let h = (c[0] - a[0]).hypot(c[1] - a[1]);
        let l = if h == 0.0 { 1.0 } else { h };
        let u = [(c[0] - a[0]) / l, (c[1] - a[1]) / l];
        let demi = self.reglage("door_w").min(l) / 2.0 / l;
        let t = t0.min(1.0 - demi).max(demi);
        ([a[0] + (c[0] - a[0]) * t, a[1] + (c[1] - a[1]) * t], [u[1] * sgn, -u[0] * sgn], l)
    }

    /// Entrée du moteur (`Site` en JSON), comme l'interface la construit pour un calcul complet.
    pub fn entree_moteur(&self) -> Value {
        let site = self.site();
        let parcelle = self.parcelle();
        let n = parcelle.len();
        // Contour discrétisé : reculs répétés sur chaque côté d'un arc, accès recalés sur leur côté.
        let (mut contour_parcelle, mut reculs, mut debut) = (Vec::new(), Vec::new(), Vec::new());
        for i in 0..n {
            let e = self.limite(i);
            debut.push(contour_parcelle.len());
            for j in 0..e.n {
                contour_parcelle.push(if j == 0 { e.a } else { e.at(j as f64 / e.n as f64) });
                reculs.push(num(liste(site.get("setbacks")).get(i)));
            }
        }
        let acces: Vec<Value> = liste(site.get("entries"))
            .iter()
            .map(|en| {
                let cote = num(en.get("edge")) as usize;
                if cote >= n {
                    return en.clone();
                }
                let cotes = self.limite(cote).n;
                let u = num(en.get("t")) * cotes as f64;
                let j = (u.floor() as usize).min(cotes - 1);
                json!({ "edge": debut[cote] + j, "t": u - j as f64 })
            })
            .collect();
        let contours = |cle: &str| -> Vec<Vec<Pt>> { liste(site.get(cle)).iter().map(|p| contour(&sommets(Some(p)))).collect() };
        let marge = self.reglage("building_margin");
        let batiments: Vec<Value> = liste(site.get("buildings"))
            .iter()
            .map(|b| {
                let (p, nrm, l) = self.porte(b);
                let poly: Vec<Pt> = emprise_batiment(b).iter().map(|q| [q[0], q[1]]).collect();
                json!({
                    "poly": poly,
                    "entrance": [p[0] + nrm[0] * marge, p[1] + nrm[1] * marge],
                    "door": { "p": p, "n": nrm, "w": self.reglage("door_w").min(l) },
                })
            })
            .collect();
        let p = self.params();
        let v = |cle: &str| p.get(cle).cloned().unwrap_or(Value::Null);
        let ouvrage = if p.get("silo").and_then(Value::as_bool) == Some(true) {
            let dalle = site.get("slab").filter(|s| s.is_object());
            json!({
                "actif": true,
                "facade": v("silo_facade"),
                "poteau": [v("silo_pu"), v("silo_pv")],
                "travee": arrondi(self.reglage("silo_travee")).max(1.0) as u64,
                "rives": p.get("silo_rives").and_then(Value::as_bool) == Some(true),
                "densite": v("silo_densite"),
                "retrait": v("silo_retrait"),
                "systeme": v("silo_systeme"),
                "angle": v("angle"),
                "hauteur_etage": v("silo_etage"),
                "niveaux": arrondi(self.reglage("silo_niveaux")).max(1.0) as u64,
                "implantation": v("silo_rampes"),
                "largement_ventile": p.get("silo_ventile").and_then(Value::as_bool) != Some(false),
                "ascenseurs_demi": p.get("silo_asc").and_then(Value::as_str).filter(|s| !s.is_empty()).unwrap_or("deux"),
                "noyaux": site.get("noyaux").filter(|x| x.is_array()).cloned().unwrap_or(Value::Null),
                "pente_rampe": self.reglage("silo_pente") / 100.0,
                "emprise": dalle.map(|s| json!({
                    "cx": s["cx"], "cy": s["cy"], "w": s["w"], "h": s["h"], "rot": s["rot"],
                    "fixe": s.get("fixe").filter(|f| f.is_array()).cloned().unwrap_or(json!([0, 0])),
                })),
            })
        } else {
            Value::Null
        };
        let tableau = |cle: &str| site.get(cle).filter(|x| x.is_array()).cloned().unwrap_or(json!([]));
        json!({
            "parcel": contour_parcelle,
            "setbacks": reculs,
            "entries": acces,
            "exclusions": contours("exclusions"),
            "buildings": batiments,
            "landscapes": contours("landscapes"),
            "basins": contours("basins"),
            "lanes": tableau("lanes"),
            "noue_off": tableau("noue_off"),
            "traverses": tableau("traverses"),
            "params": {
                "slant": v("slant"), "stall_w": v("stall_w"), "stall_d": v("stall_d"), "noue_w": v("noue_w"),
                "basin_h": v("basin_h"), "basin_margin": v("basin_margin"), "aisle": v("aisle"), "aisle_one": v("aisle_one"),
                "entry_w": v("entry_w"), "building_margin": v("building_margin"),
                "planter_every": arrondi(self.reglage("planter_every")).max(0.0) as u64, "cap": v("cap"),
                "pmr_ratio": self.reglage("pmr_ratio") / 100.0,
                "irve_ratio": self.reglage("irve_ratio") / 100.0,
                "irve_equip_ratio": self.reglage("irve_equip_ratio") / 100.0,
                "angle": v("angle"), "quick": false, "angle_hint": null, "turns": false, "programme": v("programme"),
            },
            "ouvrage": ouvrage,
        })
    }

    pub fn site_moteur(&self) -> Result<Site, String> {
        serde_json::from_value(self.entree_moteur()).map_err(|e| format!("projet illisible par le moteur : {e}"))
    }

    /// Plan du projet (calcul complet, sans le contrôle des virages) ; vide tant que la parcelle
    /// a moins de trois sommets.
    pub fn plan(&self) -> Result<Layout, String> {
        if self.parcelle().len() < 3 {
            return Ok(Layout::default());
        }
        Ok(crate::solve(&self.site_moteur()?))
    }
}
