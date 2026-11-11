//! Commandes : chaque action sur le projet est un objet JSON (`op` et ses paramètres) appliqué par
//! le moteur. L'interface, une IA ou un serveur MCP agissent sur l'aménagement par les mêmes
//! commandes. Une liste est une transaction : tout passe, ou le projet ne change pas.
//!
//! `OPS` est la seule table : elle porte l'exécution de chaque commande et sa description
//! (`catalogue`), si bien qu'une commande ne peut pas exister sans être décrite.

use crate::model::Layout;
use crate::projet::{Genre, Projet, ARC_MAX, REGLAGES};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

#[derive(Deserialize, Clone, Debug)]
pub struct Demande {
    pub projet: Value,
    #[serde(default)]
    pub commandes: Vec<Value>,
    /// Calculer le plan du projet obtenu.
    #[serde(default)]
    pub plan: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct Erreur {
    /// Rang de la commande refusée.
    pub index: usize,
    pub message: String,
}

#[derive(Serialize)]
pub struct Reponse {
    pub ok: bool,
    /// Projet obtenu ; le projet reçu, inchangé, si une commande est refusée.
    pub projet: Value,
    pub erreur: Option<Erreur>,
    pub plan: Option<Layout>,
}

type Pt = [f64; 2];
type Res<T = ()> = Result<T, String>;

/// Nombre en français : virgule décimale, trois décimales au plus.
fn fr(x: f64) -> String {
    let s = format!("{:.3}", x);
    s.trim_end_matches('0').trim_end_matches('.').replace('.', ",")
}

fn mm(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

/// Paramètres d'une commande, lus avec un message de refus qui nomme la commande et le paramètre.
struct Args<'a> {
    op: &'a str,
    m: &'a Map<String, Value>,
}

impl Args<'_> {
    fn refus(&self, quoi: &str) -> String {
        format!("{} : {quoi}", self.op)
    }
    fn valeur(&self, cle: &str) -> Res<&Value> {
        self.m.get(cle).ok_or_else(|| self.refus(&format!("paramètre « {cle} » manquant")))
    }
    fn nombre(&self, cle: &str) -> Res<f64> {
        self.valeur(cle)?.as_f64().filter(|x| x.is_finite()).ok_or_else(|| self.refus(&format!("« {cle} » doit être un nombre")))
    }
    fn nombre_opt(&self, cle: &str) -> Res<Option<f64>> {
        self.m.get(cle).map(|_| self.nombre(cle)).transpose()
    }
    /// Nombre dans un intervalle fermé.
    fn borne(&self, cle: &str, min: f64, max: f64) -> Res<f64> {
        let x = self.nombre(cle)?;
        if x < min || x > max {
            return Err(self.refus(&format!("« {cle} » vaut {}, hors de l'intervalle {} à {}", fr(x), fr(min), fr(max))));
        }
        Ok(x)
    }
    /// Rang d'un objet parmi `n` (`quoi` : « sommet », « accès »…).
    fn rang(&self, cle: &str, n: usize, quoi: &str) -> Res<usize> {
        let x = self.nombre(cle)?;
        if x.fract() != 0.0 || x < 0.0 || x >= n as f64 {
            let plage = if n == 0 { "aucun".to_string() } else { format!("0 à {}", n - 1) };
            return Err(self.refus(&format!("{quoi} {} inexistant ({plage})", fr(x))));
        }
        Ok(x as usize)
    }
    fn point(&self, cle: &str) -> Res<Pt> {
        point(self.valeur(cle)?).ok_or_else(|| self.refus(&format!("« {cle} » doit être un point [x, y]")))
    }
    fn points(&self, cle: &str) -> Res<Vec<Pt>> {
        let v = self.valeur(cle)?.as_array().ok_or_else(|| self.refus(&format!("« {cle} » doit être une liste de points [x, y]")))?;
        let pts: Option<Vec<Pt>> = v.iter().map(point).collect();
        let pts = pts.ok_or_else(|| self.refus(&format!("« {cle} » doit être une liste de points [x, y]")))?;
        if pts.len() < 3 {
            return Err(self.refus(&format!("« {cle} » demande trois points au moins")));
        }
        Ok(pts)
    }
    fn texte(&self, cle: &str) -> Res<&str> {
        self.valeur(cle)?.as_str().ok_or_else(|| self.refus(&format!("« {cle} » doit être un texte")))
    }
}

fn point(v: &Value) -> Option<Pt> {
    let a = v.as_array().filter(|a| a.len() >= 2)?;
    let (x, y) = (a[0].as_f64()?, a[1].as_f64()?);
    (x.is_finite() && y.is_finite()).then_some([mm(x), mm(y)])
}

// Lecture et écriture des tableaux du site : un tableau touché par une commande est réécrit en entier.

fn site(p: &mut Projet) -> &mut Map<String, Value> {
    p.0["site"].as_object_mut().expect("site")
}

fn parcelle(p: &Projet) -> Vec<Pt> {
    p.site()["parcel"].as_array().map(|a| a.iter().filter_map(point).collect()).unwrap_or_default()
}

/// Tableau de nombres aligné sur les `n` côtés de la parcelle (reculs, flèches), complété par des zéros.
fn par_cote(p: &Projet, cle: &str, n: usize) -> Vec<f64> {
    let mut v: Vec<f64> = p.site().get(cle).and_then(Value::as_array).map(|a| a.iter().map(|x| x.as_f64().unwrap_or(0.0)).collect()).unwrap_or_default();
    v.resize(n, 0.0);
    v
}

fn acces(p: &Projet) -> Vec<(usize, f64)> {
    let lit = |e: &Value| (e["edge"].as_u64().unwrap_or(0) as usize, e["t"].as_f64().unwrap_or(0.5));
    p.site().get("entries").and_then(Value::as_array).map(|a| a.iter().map(lit).collect()).unwrap_or_default()
}

fn ecrire_acces(p: &mut Projet, acces: &[(usize, f64)]) {
    site(p).insert("entries".into(), acces.iter().map(|&(edge, t)| json!({ "edge": edge, "t": t })).collect());
}

fn ecrire_contour(p: &mut Projet, pts: &[Pt], reculs: &[f64], arcs: &[f64]) {
    let s = site(p);
    s.insert("parcel".into(), json!(pts));
    s.insert("setbacks".into(), json!(reculs));
    s.insert("arcs".into(), json!(arcs));
}

/// Liste d'objets du site (bâtiments, zones, traverses), créée au besoin.
fn objets<'a>(p: &'a mut Projet, cle: &str) -> &'a mut Vec<Value> {
    let v = site(p).entry(cle).or_insert_with(|| json!([]));
    if !v.is_array() {
        *v = json!([]);
    }
    v.as_array_mut().unwrap()
}

fn nombre_d_objets(p: &Projet, cle: &str) -> usize {
    p.site().get(cle).and_then(Value::as_array).map_or(0, Vec::len)
}

// Commandes.

fn regler(p: &mut Projet, a: &Args) -> Res {
    let cle = a.texte("cle")?;
    let r = REGLAGES.iter().find(|r| r.cle == cle).ok_or_else(|| a.refus(&format!("réglage inconnu « {cle} »")))?;
    let v = a.valeur("valeur")?;
    let refus = |quoi: String| Err(format!("regler {cle} : {quoi}"));
    let unite = if r.unite.is_empty() { String::new() } else { format!(" {}", r.unite) };
    match r.genre {
        Genre::Nombre { min, max } | Genre::Entier { min, max } => {
            let Some(x) = v.as_f64().filter(|x| x.is_finite()) else {
                return refus(format!("{v} n'est pas un nombre"));
            };
            if matches!(r.genre, Genre::Entier { .. }) && x.fract() != 0.0 {
                return refus(format!("{} n'est pas un nombre entier", fr(x)));
            }
            if x < min || x > max {
                return refus(format!("{} hors de l'intervalle {} à {}{unite}", fr(x), fr(min), fr(max)));
            }
        }
        Genre::Choix(choix) => {
            if !v.as_str().is_some_and(|s| choix.contains(&s)) {
                return refus(format!("{v} n'est pas l'un de {}", choix.join(", ")));
            }
        }
        Genre::Bool => {
            if !v.is_boolean() {
                return refus(format!("{v} n'est ni vrai ni faux"));
            }
        }
        Genre::Orientation => {
            if !(v.is_null() || v.as_f64().is_some_and(f64::is_finite)) {
                return refus(format!("{v} n'est ni un angle en degrés ni null (orientation laissée au moteur)"));
            }
        }
        Genre::Inclinaison => {
            if !v.as_f64().is_some_and(|x| [30.0, 45.0, 60.0, 75.0, 90.0].contains(&x)) {
                return refus(format!("{v} n'est pas l'une des inclinaisons 30, 45, 60, 75, 90"));
            }
        }
    }
    p.0["params"][cle] = v.clone();
    Ok(())
}

fn tracer_parcelle(p: &mut Projet, a: &Args) -> Res {
    let pts = a.points("points")?;
    let recul = if a.m.contains_key("recul") { a.borne("recul", 0.0, 100.0)? } else { 3.0 };
    ecrire_contour(p, &pts, &vec![recul; pts.len()], &vec![0.0; pts.len()]);
    // Les accès et les repères du squelette désignaient l'ancien contour.
    let s = site(p);
    s.insert("entries".into(), json!([]));
    s.remove("lanes");
    s.remove("noue_off");
    Ok(())
}

fn deplacer_sommet(p: &mut Projet, a: &Args) -> Res {
    let mut pts = parcelle(p);
    let i = a.rang("sommet", pts.len(), "sommet")?;
    pts[i] = a.point("vers")?;
    site(p).insert("parcel".into(), json!(pts));
    Ok(())
}

/// Coupe le côté à la fraction `t` : le recul est repris, un arc devient deux arcs du même
/// cercle, les accès gardent leur position.
fn ajouter_sommet(p: &mut Projet, a: &Args) -> Res {
    let mut pts = parcelle(p);
    let n = pts.len();
    let i = a.rang("cote", n, "côté")?;
    let t = a.nombre("t")?;
    if !(t > 0.0 && t < 1.0) {
        return Err(a.refus("« t » doit être strictement entre 0 et 1"));
    }
    let (mut reculs, mut arcs) = (par_cote(p, "setbacks", n), par_cote(p, "arcs", n));
    let q = p.limite(i).at(t);
    pts.insert(i + 1, [(q[0] * 100.0).round() / 100.0, (q[1] * 100.0).round() / 100.0]);
    reculs.insert(i + 1, reculs[i]);
    let k = arcs[i];
    let angle = 4.0 * (2.0 * k).atan();
    let (k1, k2) = if k == 0.0 { (0.0, 0.0) } else { ((angle * t / 4.0).tan() / 2.0, (angle * (1.0 - t) / 4.0).tan() / 2.0) };
    arcs[i] = k1;
    arcs.insert(i + 1, k2);
    let acces: Vec<(usize, f64)> = acces(p)
        .into_iter()
        .map(|(cote, u)| match cote {
            c if c == i && u > t => (i + 1, (u - t) / (1.0 - t)),
            c if c == i => (i, u / t),
            c if c > i => (c + 1, u),
            c => (c, u),
        })
        .collect();
    ecrire_contour(p, &pts, &reculs, &arcs);
    ecrire_acces(p, &acces);
    Ok(())
}

/// Réunit les deux côtés du sommet : les angles de leurs arcs s'additionnent, les accès gardent
/// leur position sur la limite.
fn retirer_sommet(p: &mut Projet, a: &Args) -> Res {
    let mut pts = parcelle(p);
    let n = pts.len();
    let i = a.rang("sommet", n, "sommet")?;
    if n <= 3 {
        return Err(a.refus("la parcelle garde trois sommets au moins"));
    }
    let (mut reculs, mut arcs) = (par_cote(p, "setbacks", n), par_cote(p, "arcs", n));
    let avant = (i + n - 1) % n;
    let reuni = if i == 0 { n - 2 } else { i - 1 };
    let (lp, li) = (p.limite(avant).l, p.limite(i).l);
    let somme = (4.0 * (2.0 * arcs[avant]).atan() + 4.0 * (2.0 * arcs[i]).atan()) / 4.0;
    arcs[avant] = ((somme.tan() / 2.0).clamp(-ARC_MAX, ARC_MAX) * 1e5).round() / 1e5;
    pts.remove(i);
    reculs.remove(i);
    arcs.remove(i);
    let acces: Vec<(usize, f64)> = acces(p)
        .into_iter()
        .map(|(cote, u)| match cote {
            c if c == avant => (reuni, u * lp / (lp + li)),
            c if c == i => (reuni, (lp + u * li) / (lp + li)),
            c if c > i => (c - 1, u),
            c => (c, u),
        })
        .collect();
    ecrire_contour(p, &pts, &reculs, &arcs);
    ecrire_acces(p, &acces);
    Ok(())
}

fn regler_recul(p: &mut Projet, a: &Args) -> Res {
    let n = parcelle(p).len();
    let valeur = a.borne("valeur", 0.0, 100.0)?;
    let mut reculs = par_cote(p, "setbacks", n);
    if a.m.contains_key("cote") {
        reculs[a.rang("cote", n, "côté")?] = valeur;
    } else {
        reculs.fill(valeur);
    }
    site(p).insert("setbacks".into(), json!(reculs));
    Ok(())
}

fn courber_cote(p: &mut Projet, a: &Args) -> Res {
    let n = parcelle(p).len();
    let i = a.rang("cote", n, "côté")?;
    let mut arcs = par_cote(p, "arcs", n);
    arcs[i] = a.borne("fleche", -ARC_MAX, ARC_MAX)?;
    site(p).insert("arcs".into(), json!(arcs));
    Ok(())
}

/// Position d'un accès : côté de la parcelle et fraction le long de ce côté.
fn position_acces(p: &Projet, a: &Args) -> Res<(usize, f64)> {
    Ok((a.rang("cote", parcelle(p).len(), "côté")?, a.borne("t", 0.0, 1.0)?))
}

fn ajouter_acces(p: &mut Projet, a: &Args) -> Res {
    let mut tous = acces(p);
    tous.push(position_acces(p, a)?);
    ecrire_acces(p, &tous);
    Ok(())
}

fn deplacer_acces(p: &mut Projet, a: &Args) -> Res {
    let mut tous = acces(p);
    let i = a.rang("acces", tous.len(), "accès")?;
    tous[i] = position_acces(p, a)?;
    ecrire_acces(p, &tous);
    Ok(())
}

fn retirer_acces(p: &mut Projet, a: &Args) -> Res {
    let mut tous = acces(p);
    tous.remove(a.rang("acces", tous.len(), "accès")?);
    ecrire_acces(p, &tous);
    Ok(())
}

/// Reporte sur un bâtiment les paramètres présents : centre, largeur, profondeur, rotation, forme.
fn regler_batiment(b: &mut Value, a: &Args) -> Res {
    if a.m.contains_key("centre") {
        let c = a.point("centre")?;
        b["cx"] = json!(c[0]);
        b["cy"] = json!(c[1]);
    }
    for (cle, champ) in [("largeur", "w"), ("profondeur", "h")] {
        if a.m.contains_key(cle) {
            b[champ] = json!(a.borne(cle, 1.0, 1000.0)?);
        }
    }
    if let Some(rot) = a.nombre_opt("rotation")? {
        b["rot"] = json!(rot);
    }
    if a.m.contains_key("forme") {
        b["shape"] = match a.texte("forme")? {
            "rectangle" => json!("rect"),
            "L" => json!("L"),
            autre => return Err(a.refus(&format!("« forme » vaut « {autre} » : rectangle ou L"))),
        };
    }
    Ok(())
}

fn ajouter_batiment(p: &mut Projet, a: &Args) -> Res {
    for cle in ["centre", "largeur", "profondeur"] {
        a.valeur(cle)?;
    }
    let mut b = json!({ "rot": 0.0, "shape": "rect" });
    regler_batiment(&mut b, a)?;
    objets(p, "buildings").push(b);
    Ok(())
}

fn modifier_batiment(p: &mut Projet, a: &Args) -> Res {
    let i = a.rang("batiment", nombre_d_objets(p, "buildings"), "bâtiment")?;
    let mut b = objets(p, "buildings")[i].clone();
    regler_batiment(&mut b, a)?;
    objets(p, "buildings")[i] = b;
    Ok(())
}

fn retirer_batiment(p: &mut Projet, a: &Args) -> Res {
    let i = a.rang("batiment", nombre_d_objets(p, "buildings"), "bâtiment")?;
    objets(p, "buildings").remove(i);
    Ok(())
}

/// Liste du site qui porte les zones d'une nature.
fn zones(a: &Args) -> Res<(&'static str, &'static str)> {
    match a.texte("nature")? {
        "servitude" => Ok(("exclusions", "servitude")),
        "espace_vert" => Ok(("landscapes", "espace vert")),
        "bassin" => Ok(("basins", "bassin")),
        autre => Err(a.refus(&format!("« nature » vaut « {autre} » : servitude, espace_vert ou bassin"))),
    }
}

fn ajouter_zone(p: &mut Projet, a: &Args) -> Res {
    let (cle, _) = zones(a)?;
    let pts = a.points("points")?;
    objets(p, cle).push(json!(pts));
    Ok(())
}

fn retirer_zone(p: &mut Projet, a: &Args) -> Res {
    let (cle, nom) = zones(a)?;
    let i = a.rang("zone", nombre_d_objets(p, cle), nom)?;
    objets(p, cle).remove(i);
    Ok(())
}

fn ajouter_traverse(p: &mut Projet, a: &Args) -> Res {
    let t = json!([a.point("de")?, a.point("a")?]);
    objets(p, "traverses").push(t);
    Ok(())
}

fn retirer_traverse(p: &mut Projet, a: &Args) -> Res {
    let i = a.rang("traverse", nombre_d_objets(p, "traverses"), "traverse")?;
    objets(p, "traverses").remove(i);
    Ok(())
}

fn regler_dalle(p: &mut Projet, a: &Args) -> Res {
    let c = a.point("centre")?;
    let dalle = json!({
        "cx": c[0], "cy": c[1],
        "w": a.borne("longueur", 5.0, 1000.0)?, "h": a.borne("largeur", 5.0, 1000.0)?,
        "rot": a.nombre_opt("rotation")?.unwrap_or(0.0), "fixe": [0, 0],
    });
    site(p).insert("slab".into(), dalle);
    Ok(())
}

fn liberer_dalle(p: &mut Projet, _: &Args) -> Res {
    site(p).remove("slab");
    Ok(())
}

/// Commande : nom, exécution, et ce qu'il faut pour la décrire à une IA.
pub struct Op {
    pub nom: &'static str,
    pub description: &'static str,
    /// Paramètres : nom et description (type, unité, valeurs admises).
    pub parametres: &'static [(&'static str, &'static str)],
    /// Exemple en JSON, rejoué par les tests sur un projet de référence.
    pub exemple: &'static str,
    appliquer: fn(&mut Projet, &Args) -> Res,
}

const COTE: &str = "rang du côté de la parcelle, de son sommet de même rang au suivant (entier à partir de 0)";
const T: &str = "position le long du côté, de 0 (premier sommet) à 1";

pub const OPS: &[Op] = &[
    Op {
        nom: "regler",
        description: "Change un réglage du projet : dimensions des places et des allées, parts de places PMR et IRVE, inclinaison, orientation, programme, parc en ouvrage.",
        parametres: &[("cle", "clé du réglage, voir la liste « reglages »"), ("valeur", "nouvelle valeur, dans les bornes du réglage")],
        exemple: r#"{"op":"regler","cle":"stall_w","valeur":2.4}"#,
        appliquer: regler,
    },
    Op {
        nom: "tracer_parcelle",
        description: "Remplace le contour de la parcelle. Les accès sont retirés (à reposer), les autres objets restent.",
        parametres: &[("points", "sommets [x, y] en mètres, trois au moins, dans l'ordre du contour"), ("recul", "recul appliqué à tous les côtés, en mètres (3 par défaut)")],
        exemple: r#"{"op":"tracer_parcelle","points":[[0,0],[90,0],[90,60],[0,60]],"recul":3}"#,
        appliquer: tracer_parcelle,
    },
    Op {
        nom: "deplacer_sommet",
        description: "Déplace un sommet de la parcelle ; les deux côtés qui s'y rejoignent suivent.",
        parametres: &[("sommet", "rang du sommet (entier à partir de 0)"), ("vers", "nouvelle position [x, y] en mètres")],
        exemple: r#"{"op":"deplacer_sommet","sommet":1,"vers":[125,0]}"#,
        appliquer: deplacer_sommet,
    },
    Op {
        nom: "ajouter_sommet",
        description: "Ajoute un sommet sur un côté de la parcelle, sans changer le contour ; le nouveau sommet prend le rang suivant celui du côté.",
        parametres: &[("cote", COTE), ("t", "position le long du côté, strictement entre 0 et 1")],
        exemple: r#"{"op":"ajouter_sommet","cote":0,"t":0.5}"#,
        appliquer: ajouter_sommet,
    },
    Op {
        nom: "retirer_sommet",
        description: "Retire un sommet de la parcelle : ses deux côtés n'en font plus qu'un. Trois sommets au moins restent.",
        parametres: &[("sommet", "rang du sommet (entier à partir de 0)")],
        exemple: r#"{"op":"retirer_sommet","sommet":3}"#,
        appliquer: retirer_sommet,
    },
    Op {
        nom: "regler_recul",
        description: "Fixe le recul d'un côté de la parcelle, ou de tous les côtés si « cote » est absent.",
        parametres: &[("cote", COTE), ("valeur", "recul en mètres, de 0 à 100")],
        exemple: r#"{"op":"regler_recul","cote":0,"valeur":5}"#,
        appliquer: regler_recul,
    },
    Op {
        nom: "courber_cote",
        description: "Arrondit un côté de la parcelle en arc de cercle, ou le redresse avec une flèche nulle.",
        parametres: &[("cote", COTE), ("fleche", "flèche de l'arc en fraction de la corde, de -0,5 à 0,5, positive vers l'extérieur")],
        exemple: r#"{"op":"courber_cote","cote":1,"fleche":0.15}"#,
        appliquer: courber_cote,
    },
    Op {
        nom: "ajouter_acces",
        description: "Pose un accès véhicules sur un côté de la parcelle.",
        parametres: &[("cote", COTE), ("t", T)],
        exemple: r#"{"op":"ajouter_acces","cote":2,"t":0.5}"#,
        appliquer: ajouter_acces,
    },
    Op {
        nom: "deplacer_acces",
        description: "Déplace un accès véhicules, sur le même côté ou sur un autre.",
        parametres: &[("acces", "rang de l'accès (entier à partir de 0)"), ("cote", COTE), ("t", T)],
        exemple: r#"{"op":"deplacer_acces","acces":0,"cote":0,"t":0.3}"#,
        appliquer: deplacer_acces,
    },
    Op {
        nom: "retirer_acces",
        description: "Retire un accès véhicules. Sans accès, aucune place n'est desservie.",
        parametres: &[("acces", "rang de l'accès (entier à partir de 0)")],
        exemple: r#"{"op":"retirer_acces","acces":0}"#,
        appliquer: retirer_acces,
    },
    Op {
        nom: "ajouter_batiment",
        description: "Ajoute un bâtiment (rectangle ou forme en L) ; un trottoir l'entoure et les places PMR se rapprochent de son entrée.",
        parametres: &[
            ("centre", "centre [x, y] en mètres"),
            ("largeur", "dimension le long de son axe, en mètres"),
            ("profondeur", "dimension en travers, en mètres"),
            ("rotation", "rotation en degrés (0 par défaut)"),
            ("forme", "« rectangle » (par défaut) ou « L »"),
        ],
        exemple: r#"{"op":"ajouter_batiment","centre":[40,40],"largeur":30,"profondeur":18,"rotation":0,"forme":"rectangle"}"#,
        appliquer: ajouter_batiment,
    },
    Op {
        nom: "modifier_batiment",
        description: "Modifie un bâtiment : seuls les paramètres présents changent.",
        parametres: &[
            ("batiment", "rang du bâtiment (entier à partir de 0)"),
            ("centre", "centre [x, y] en mètres"),
            ("largeur", "dimension le long de son axe, en mètres"),
            ("profondeur", "dimension en travers, en mètres"),
            ("rotation", "rotation en degrés"),
            ("forme", "« rectangle » ou « L »"),
        ],
        exemple: r#"{"op":"modifier_batiment","batiment":0,"rotation":30}"#,
        appliquer: modifier_batiment,
    },
    Op {
        nom: "retirer_batiment",
        description: "Retire un bâtiment ; le stationnement reprend son emprise.",
        parametres: &[("batiment", "rang du bâtiment (entier à partir de 0)")],
        exemple: r#"{"op":"retirer_batiment","batiment":0}"#,
        appliquer: retirer_batiment,
    },
    Op {
        nom: "ajouter_zone",
        description: "Ajoute une zone sans stationnement : servitude, espace vert imposé ou bassin de rétention.",
        parametres: &[("nature", "« servitude », « espace_vert » ou « bassin »"), ("points", "sommets [x, y] en mètres, trois au moins")],
        exemple: r#"{"op":"ajouter_zone","nature":"espace_vert","points":[[40,30],[55,30],[55,45],[40,45]]}"#,
        appliquer: ajouter_zone,
    },
    Op {
        nom: "retirer_zone",
        description: "Retire une servitude, un espace vert imposé ou un bassin.",
        parametres: &[("nature", "« servitude », « espace_vert » ou « bassin »"), ("zone", "rang de la zone parmi celles de sa nature (entier à partir de 0)")],
        exemple: r#"{"op":"retirer_zone","nature":"servitude","zone":0}"#,
        appliquer: retirer_zone,
    },
    Op {
        nom: "ajouter_traverse",
        description: "Trace une traverse : un segment qui coupe les rangées qu'il croise par une allée transversale.",
        parametres: &[("de", "première extrémité [x, y] en mètres"), ("a", "seconde extrémité [x, y] en mètres")],
        exemple: r#"{"op":"ajouter_traverse","de":[30,10],"a":[30,70]}"#,
        appliquer: ajouter_traverse,
    },
    Op {
        nom: "retirer_traverse",
        description: "Retire une traverse ; les rangées qu'elle coupait se referment.",
        parametres: &[("traverse", "rang de la traverse (entier à partir de 0)")],
        exemple: r#"{"op":"retirer_traverse","traverse":0}"#,
        appliquer: retirer_traverse,
    },
    Op {
        nom: "regler_dalle",
        description: "Impose l'emprise de la dalle d'un parc en ouvrage (réglage « silo » à vrai) ; le moteur la ramène aux dimensions réalisables les plus proches.",
        parametres: &[
            ("centre", "centre [x, y] en mètres"),
            ("longueur", "dimension le long des allées, en mètres"),
            ("largeur", "dimension en travers des allées, en mètres"),
            ("rotation", "direction des allées en degrés (0 par défaut)"),
        ],
        exemple: r#"{"op":"regler_dalle","centre":[60,40],"longueur":90,"largeur":32,"rotation":0}"#,
        appliquer: regler_dalle,
    },
    Op {
        nom: "liberer_dalle",
        description: "Rend au moteur le choix de l'emprise de la dalle : le plus grand plateau de la zone constructible.",
        parametres: &[],
        exemple: r#"{"op":"liberer_dalle"}"#,
        appliquer: liberer_dalle,
    },
];

/// Applique une commande au projet.
pub fn appliquer(p: &mut Projet, commande: &Value) -> Res {
    let m = commande.as_object().ok_or("une commande est un objet JSON avec un champ « op »")?;
    let nom = m.get("op").and_then(Value::as_str).ok_or("commande sans champ « op »")?;
    let op = OPS.iter().find(|o| o.nom == nom).ok_or_else(|| format!("commande inconnue « {nom} »"))?;
    (op.appliquer)(p, &Args { op: nom, m })
}

/// Applique la liste de commandes : tout passe, ou le projet reçu est rendu inchangé avec le
/// rang et le motif du refus. Le plan n'est calculé que sur demande.
pub fn executer(d: &Demande) -> Reponse {
    let refus = |projet: Value, index: usize, message: String| Reponse { ok: false, projet, erreur: Some(Erreur { index, message }), plan: None };
    let mut projet = match Projet::lire(d.projet.clone()) {
        Ok(p) => p,
        Err(e) => return refus(d.projet.clone(), 0, e),
    };
    let origine = projet.clone();
    for (i, c) in d.commandes.iter().enumerate() {
        if let Err(e) = appliquer(&mut projet, c) {
            return refus(origine.0, i, e);
        }
    }
    let plan = match d.plan.then(|| projet.plan()).transpose() {
        Ok(plan) => plan,
        Err(e) => return refus(origine.0, d.commandes.len(), e),
    };
    Reponse { ok: true, projet: projet.0, erreur: None, plan }
}

/// Catalogue des commandes, pour une IA ou une documentation : nom, description, paramètres,
/// exemple ; la commande `regler` porte la liste des réglages et de leurs bornes.
pub fn catalogue() -> Vec<Value> {
    OPS.iter()
        .map(|o| {
            let parametres: Map<String, Value> = o.parametres.iter().map(|&(k, d)| (k.to_string(), json!(d))).collect();
            let mut c = json!({
                "op": o.nom,
                "description": o.description,
                "parametres": parametres,
                "exemple": serde_json::from_str::<Value>(o.exemple).expect("exemple JSON"),
            });
            if o.nom == "regler" {
                c["reglages"] = REGLAGES.iter().map(reglage).collect();
            }
            c
        })
        .collect()
}

fn reglage(r: &crate::projet::Reglage) -> Value {
    let mut v = json!({ "cle": r.cle, "libelle": r.libelle, "unite": r.unite, "defaut": r.defaut.json() });
    match r.genre {
        Genre::Nombre { min, max } => {
            v["min"] = json!(min);
            v["max"] = json!(max);
        }
        Genre::Entier { min, max } => {
            v["min"] = json!(min);
            v["max"] = json!(max);
            v["entier"] = json!(true);
        }
        Genre::Choix(choix) => v["choix"] = json!(choix),
        Genre::Bool => v["choix"] = json!([true, false]),
        Genre::Orientation => v["valeurs"] = json!("angle en degrés, ou null pour laisser le moteur choisir"),
        Genre::Inclinaison => v["choix"] = json!([30, 45, 60, 75, 90]),
    }
    v
}
