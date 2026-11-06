//! Export IFC4 (ISO 16739-1:2018) du modèle BIM : texte STEP écrit directement, sans dépendance.
//!
//! Structure : projet > site > bâtiment > niveau ; places et allées en `IfcSpace` (PARKING)
//! agrégés au niveau ; dalle, toiture, poteaux et trame contenus dans le niveau ; un type par
//! famille d'éléments ; propriétés standard (Pset_*) et propres au module (PKM_*) ;
//! géoréférencement par `IfcMapConversion` vers Lambert-93 ou la conique conforme de la zone.

use super::bim::{self, Element, Modele, Valeur};
use crate::geom::V2;
use crate::model::Site;
use crate::proj::{Georef, Lcc};
use serde::{Deserialize, Serialize};
use std::fmt::Write;

#[derive(Deserialize)]
pub struct IfcIn {
    pub site: Site,
    #[serde(default)]
    pub georef: Option<Georef>,
    /// "L93", "CC42" à "CC50", ou "LOCAL".
    #[serde(default)]
    pub crs: String,
    /// Horodatage ISO 8601 du fichier.
    #[serde(default)]
    pub date: String,
    /// Graine des identifiants : propre au projet, stable d'un export à l'autre.
    #[serde(default)]
    pub projet: String,
}

#[derive(Serialize, Default)]
pub struct IfcOut {
    pub ifc: String,
    pub elements: usize,
    pub crs: String,
    pub epsg: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub erreur: Option<String>,
}

/// Réel STEP : toujours avec un point décimal.
fn r(x: f64) -> String {
    let x = if x.abs() < 5e-7 { 0.0 } else { x };
    let mut s = format!("{x:.6}");
    while s.ends_with('0') {
        s.pop();
    }
    s
}

/// Chaîne STEP : apostrophes doublées, caractères non ASCII en \X2\…\X0\.
fn s(t: &str) -> String {
    let mut o = String::from("'");
    let mut ext = String::new();
    let flush = |o: &mut String, ext: &mut String| {
        if !ext.is_empty() {
            o.push_str("\\X2\\");
            o.push_str(ext);
            o.push_str("\\X0\\");
            ext.clear();
        }
    };
    for c in t.chars() {
        if c.is_ascii() && !c.is_ascii_control() {
            flush(&mut o, &mut ext);
            match c {
                '\'' => o.push_str("''"),
                '\\' => o.push_str("\\\\"),
                _ => o.push(c),
            }
        } else {
            let mut buf = [0u16; 2];
            for u in c.encode_utf16(&mut buf) {
                let _ = write!(ext, "{:04X}", u);
            }
        }
    }
    flush(&mut o, &mut ext);
    o.push('\'');
    o
}

fn liste(ids: &[usize]) -> String {
    let v: Vec<String> = ids.iter().map(|i| format!("#{i}")).collect();
    format!("({})", v.join(","))
}

struct W {
    lignes: Vec<String>,
}

impl W {
    fn add(&mut self, e: String) -> usize {
        self.lignes.push(e);
        self.lignes.len()
    }
}

fn valeur(v: &Valeur) -> String {
    match v {
        Valeur::Label(t) => format!("IFCLABEL({})", s(t)),
        Valeur::Texte(t) => format!("IFCTEXT({})", s(t)),
        Valeur::Booleen(b) => format!("IFCBOOLEAN(.{}.)", if *b { "T" } else { "F" }),
        Valeur::Entier(i) => format!("IFCINTEGER({i})"),
        Valeur::Compte(x) => format!("IFCCOUNTMEASURE({})", r(*x)),
        Valeur::Longueur(x) => format!("IFCPOSITIVELENGTHMEASURE({})", r(*x)),
    }
}

fn polyligne_2d(w: &mut W, pts: &[V2], fermer: bool) -> usize {
    let mut ids: Vec<usize> = pts
        .iter()
        .map(|p| w.add(format!("IFCCARTESIANPOINT(({},{}))", r(p.x), r(p.y))))
        .collect();
    if fermer {
        ids.push(ids[0]);
    }
    w.add(format!("IFCPOLYLINE({})", liste(&ids)))
}

/// Écrit le modèle en IFC4. `geo` : (nom du système, code EPSG, transformation plan → projection).
pub fn ecrire(m: &Modele, date: &str, geo: Option<(&str, u32, &dyn Fn(f64, f64) -> (f64, f64))>) -> String {
    let mut w = W { lignes: Vec::new() };
    let g = |cle: &str| s(&m.guid(cle));

    let o3 = w.add("IFCCARTESIANPOINT((0.,0.,0.))".into());
    let dz = w.add("IFCDIRECTION((0.,0.,1.))".into());
    let dx = w.add("IFCDIRECTION((1.,0.,0.))".into());
    let wcs = w.add(format!("IFCAXIS2PLACEMENT3D(#{o3},#{dz},#{dx})"));
    let ctx = w.add(format!("IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#{wcs},$)"));
    let body = w.add(format!(
        "IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#{ctx},$,.MODEL_VIEW.,$)"
    ));
    let metre = w.add("IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)".into());
    let m2 = w.add("IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.)".into());
    let m3 = w.add("IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.)".into());
    let rad = w.add("IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.)".into());
    let unites = w.add(format!("IFCUNITASSIGNMENT({})", liste(&[metre, m2, m3, rad])));
    let projet = w.add(format!(
        "IFCPROJECT({},$,{},$,$,$,$,(#{ctx}),#{unites})",
        g("PROJET"),
        s(&m.nom)
    ));

    // Géoréférencement : origine du plan, direction de son axe x et échelle en projection.
    if let Some((nom, epsg, tf)) = geo {
        let (e0, n0) = tf(0.0, 0.0);
        let (e1, n1) = tf(100.0, 0.0);
        let (ex, ey) = (e1 - e0, n1 - n0);
        let l = (ex * ex + ey * ey).sqrt();
        let crs = w.add(format!(
            "IFCPROJECTEDCRS({},{},'RGF93 v1','NGF-IGN69',$,$,#{metre})",
            s(&format!("EPSG:{epsg}")),
            s(nom)
        ));
        w.add(format!(
            "IFCMAPCONVERSION(#{ctx},#{crs},{},{},{},{},{},{})",
            r(e0),
            r(n0),
            r(m.cote),
            r(ex / l),
            r(ey / l),
            r(l / 100.0)
        ));
    }

    let lp_site = w.add(format!("IFCLOCALPLACEMENT($,#{wcs})"));
    let site = w.add(format!(
        "IFCSITE({},$,'Parcelle',$,$,#{lp_site},$,$,.ELEMENT.,$,$,{},$,$)",
        g("SITE"),
        r(m.cote)
    ));
    let lp_bat = w.add(format!("IFCLOCALPLACEMENT(#{lp_site},#{wcs})"));
    let bat = w.add(format!(
        "IFCBUILDING({},$,{},$,$,#{lp_bat},$,$,.ELEMENT.,$,$,$)",
        g("BATIMENT"),
        s(&m.nom)
    ));
    let lp_niv = w.add(format!("IFCLOCALPLACEMENT(#{lp_bat},#{wcs})"));
    let niv = w.add(format!(
        "IFCBUILDINGSTOREY({},$,{},$,$,#{lp_niv},$,'Niveau 0',.ELEMENT.,0.)",
        g(&m.niveau),
        s(&m.niveau)
    ));
    w.add(format!("IFCRELAGGREGATES({},$,$,$,#{projet},(#{site}))", g("REL/PROJET")));
    w.add(format!("IFCRELAGGREGATES({},$,$,$,#{site},(#{bat}))", g("REL/SITE")));
    w.add(format!("IFCRELAGGREGATES({},$,$,$,#{bat},(#{niv}))", g("REL/BATIMENT")));

    // Éléments : une placement locale par niveau, géométrie en coordonnées du plan.
    let lp_el = w.add(format!("IFCLOCALPLACEMENT(#{lp_niv},#{wcs})"));
    let mut ids: Vec<usize> = Vec::with_capacity(m.elements.len());
    for e in &m.elements {
        let forme = extrusion(&mut w, e, body, dz);
        let id = match e.classe {
            "IfcSpace" => w.add(format!(
                "IFCSPACE({},$,{},$,$,#{lp_el},#{forme},$,.ELEMENT.,.{}.,$)",
                s(&e.guid),
                s(&e.nom),
                e.predefini
            )),
            c => w.add(format!(
                "{}({},$,{},$,$,#{lp_el},#{forme},{},.{}.)",
                c.to_uppercase(),
                s(&e.guid),
                s(&e.nom),
                s(&e.cle),
                e.predefini
            )),
        };
        ids.push(id);
    }

    // Trame.
    fn axe_ids(w: &mut W, axes: &[bim::Axe]) -> Vec<usize> {
        axes.iter()
            .map(|a| {
                let c = polyligne_2d(w, &[a.a, a.b], false);
                w.add(format!("IFCGRIDAXIS({},#{c},.T.)", s(&a.nom)))
            })
            .collect()
    }
    let (au, av) = (axe_ids(&mut w, &m.axes_u), axe_ids(&mut w, &m.axes_v));
    let grille = if !au.is_empty() && !av.is_empty() {
        Some(w.add(format!(
            "IFCGRID({},$,'Trame',$,$,#{lp_el},$,{},{},$,.RECTANGULAR.)",
            g("TRAME"),
            liste(&au),
            liste(&av)
        )))
    } else {
        None
    };

    // Contenu du niveau et agrégation des espaces.
    let espaces: Vec<usize> = m
        .elements
        .iter()
        .zip(&ids)
        .filter(|(e, _)| e.classe == "IfcSpace")
        .map(|(_, &i)| i)
        .collect();
    let mut contenus: Vec<usize> = m
        .elements
        .iter()
        .zip(&ids)
        .filter(|(e, _)| e.classe != "IfcSpace")
        .map(|(_, &i)| i)
        .collect();
    contenus.extend(grille);
    if !espaces.is_empty() {
        w.add(format!(
            "IFCRELAGGREGATES({},$,$,$,#{niv},{})",
            g("REL/ESPACES"),
            liste(&espaces)
        ));
    }
    w.add(format!(
        "IFCRELCONTAINEDINSPATIALSTRUCTURE({},$,$,$,{},#{niv})",
        g("REL/CONTENU"),
        liste(&contenus)
    ));

    // Types et leurs occurrences.
    for (ti, t) in m.types.iter().enumerate() {
        let occ: Vec<usize> = m
            .elements
            .iter()
            .zip(&ids)
            .filter(|(e, _)| e.type_i == ti)
            .map(|(_, &i)| i)
            .collect();
        if occ.is_empty() {
            continue;
        }
        let tg = g(&format!("TYPE/{}", t.nom));
        let tid = match t.classe {
            "IfcSpace" => w.add(format!(
                "IFCSPACETYPE({tg},$,{},$,$,$,$,$,$,.{}.,$)",
                s(&t.nom),
                t.predefini
            )),
            c => w.add(format!(
                "{}TYPE({tg},$,{},$,$,$,$,$,$,.{}.)",
                c.to_uppercase(),
                s(&t.nom),
                t.predefini
            )),
        };
        w.add(format!(
            "IFCRELDEFINESBYTYPE({},$,$,$,{},#{tid})",
            g(&format!("REL/TYPE/{}", t.nom)),
            liste(&occ)
        ));
    }

    // Jeux de propriétés : un jeu par combinaison identique de valeurs, partagé par ses éléments.
    let mut jeux: Vec<(&'static str, Vec<(&'static str, String)>, Vec<usize>)> = Vec::new();
    for (e, &id) in m.elements.iter().zip(&ids) {
        let mut noms: Vec<&'static str> = e.props.iter().map(|p| p.pset).collect();
        noms.dedup();
        for pset in noms {
            let vals: Vec<(&'static str, String)> = e
                .props
                .iter()
                .filter(|p| p.pset == pset)
                .map(|p| (p.nom, valeur(&p.val)))
                .collect();
            match jeux.iter_mut().find(|j| j.0 == pset && j.1 == vals) {
                Some(j) => j.2.push(id),
                None => jeux.push((pset, vals, vec![id])),
            }
        }
    }
    for (k, (pset, vals, occ)) in jeux.iter().enumerate() {
        let props: Vec<usize> = vals
            .iter()
            .map(|(nom, v)| w.add(format!("IFCPROPERTYSINGLEVALUE({},$,{v},$)", s(nom))))
            .collect();
        let ps = w.add(format!(
            "IFCPROPERTYSET({},$,{},$,{})",
            g(&format!("PSET/{pset}/{k}")),
            s(pset),
            liste(&props)
        ));
        w.add(format!(
            "IFCRELDEFINESBYPROPERTIES({},$,$,$,{},#{ps})",
            g(&format!("REL/PSET/{pset}/{k}")),
            liste(occ)
        ));
    }

    let mut out = String::new();
    out.push_str("ISO-10303-21;\nHEADER;\n");
    out.push_str("FILE_DESCRIPTION(('ViewDefinition [ReferenceView_V1.2]'),'2;1');\n");
    let _ = writeln!(
        out,
        "FILE_NAME({},{},(''),(''),'Parkimo (moteur Trame, Rust)','Parkimo','');",
        s(&format!("{}.ifc", m.nom)),
        s(date)
    );
    out.push_str("FILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n");
    for (i, l) in w.lignes.iter().enumerate() {
        let _ = writeln!(out, "#{}={};", i + 1, l);
    }
    out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    out
}

fn extrusion(w: &mut W, e: &Element, body: usize, dz: usize) -> usize {
    let pl = polyligne_2d(w, &e.profil, true);
    let prof = w.add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#{pl})"));
    let o = w.add(format!("IFCCARTESIANPOINT((0.,0.,{}))", r(e.z0)));
    let pos = w.add(format!("IFCAXIS2PLACEMENT3D(#{o},$,$)"));
    let solide = w.add(format!("IFCEXTRUDEDAREASOLID(#{prof},#{pos},#{dz},{})", r(e.h)));
    let rep = w.add(format!("IFCSHAPEREPRESENTATION(#{body},'Body','SweptSolid',(#{solide}))"));
    w.add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep}))"))
}

/// Calcule le plan, construit le modèle BIM et l'écrit en IFC4.
pub fn export(inp: &IfcIn) -> IfcOut {
    let mut site = inp.site.clone();
    site.params.quick = false;
    let lay = crate::solve(&site);
    let p = &site.params;
    let graine = if inp.projet.is_empty() { "parkimo" } else { inp.projet.as_str() };
    let Some(m) = bim::modele(&lay, graine, p.stall_w, p.stall_d, p.aisle) else {
        return IfcOut {
            erreur: Some("Activez le parc en ouvrage pour exporter la maquette".into()),
            ..Default::default()
        };
    };
    let (texte, crs, epsg) = match (inp.georef, inp.crs.as_str()) {
        (Some(gr), c) if c != "LOCAL" => {
            let pj = match c.strip_prefix("CC").and_then(|z| z.parse::<u32>().ok()) {
                Some(z) => Lcc::cc(z),
                None => Lcc::lambert93(),
            };
            let (name, epsg) = (pj.name.clone(), pj.epsg);
            let tf = move |x: f64, y: f64| {
                let (lon, lat) = gr.to_lonlat(x, y);
                pj.forward(lon, lat)
            };
            (ecrire(&m, &inp.date, Some((&name, epsg, &tf))), name, epsg)
        }
        _ => (
            ecrire(&m, &inp.date, None),
            "Coordonnées locales (plan non géoréférencé)".into(),
            0,
        ),
    };
    IfcOut {
        ifc: texte,
        elements: m.elements.len(),
        crs,
        epsg,
        erreur: None,
    }
}
