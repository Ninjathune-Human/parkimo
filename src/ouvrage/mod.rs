//! Parcs de stationnement en ouvrage (silos, puis enterrés) : trame structurelle, plateaux,
//! modèle BIM et export IFC.
//!
//! Lot 1 : silo à un niveau sur un plateau rectangulaire (modules de 16 m pour des places de 5,00 m
//! et des allées de 6,00 m), trame de densité longue (poteaux en façade et en dos à dos), moyenne ou
//! courte (poteaux en tête de place), plancher du système constructif retenu (`systemes`).

pub mod bim;
pub mod giration;
pub mod ifc;
pub mod noyaux;
pub use crate::normes;
pub mod plateau;
pub mod systemes;

use crate::geom::{signed_area, v2, Quad, V2};
use crate::layout::KIND_STD;
use crate::model::{Layout, Metrics, Site, StallOut, P2};
use crate::region;
use geo::{BooleanOps, MultiPolygon};
use normes::{neufert, nf, ps};
use plateau::{Densite, Emprise, Gabarit, Implantation, Plan};
use systemes::Systeme;
use serde::{Deserialize, Serialize};

/// Réglages d'un parc en ouvrage. Les dimensions de places et d'allées sont celles des
/// paramètres généraux (`Params`).
#[derive(Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Ouvrage {
    pub actif: bool,
    /// Épaisseur de rive en façade (garde-corps, nez de dalle), en mètres. Hypothèse d'esquisse.
    pub facade: f64,
    /// Poteau : [le long des allées, en travers], en mètres. Pré-dimensionnement à confirmer (BET).
    pub poteau: [f64; 2],
    /// Places entre deux files de poteaux le long des allées.
    pub travee: u32,
    /// Places de rive le long des façades d'extrémité.
    pub rives: bool,
    /// Densité de la trame en travers : "longue", "moyenne" ou "courte".
    pub densite: Densite,
    /// Poteau en tête de place : du nu du poteau au bord de l'allée, en mètres.
    pub retrait: f64,
    /// Système constructif : "coule", "prefa" ou "mixte". Il fixe le plancher (épaisseur, retombée).
    pub systeme: Systeme,
    /// Direction imposée des allées (degrés) ; absente = optimisée.
    pub angle: Option<f64>,
    /// Emprise dessinée à la main ; absente = plus grand plateau de la zone constructible.
    pub emprise: Option<EmpriseIn>,
    /// Hauteur d'étage (plancher à plancher), en mètres.
    pub hauteur_etage: f64,
    /// Nombre de niveaux (rez-de-chaussée compris). Au-delà d'un, rampes droites.
    pub niveaux: u32,
    /// Implantation des rampes : "auto", "rangee" (le long des allées) ou "pignon".
    pub implantation: Implantation,
    /// Pente des rampes (fraction) : 18 % au plus (NF P 91-100, § 3.4).
    pub pente_rampe: f64,
    /// Parc largement ventilé (PS 3) : distances aux escaliers de 50 et 30 m au lieu de 40 et 25.
    pub largement_ventile: bool,
    /// Demi-niveaux : "deux" ascenseurs (un par demi-plateau) ou un seul et un escalier de "liaison".
    pub ascenseurs_demi: noyaux::AscenseursDemi,
    /// Escaliers et ascenseurs placés à la main (coordonnées du site) ; absents = placement
    /// automatique.
    pub noyaux: Option<Vec<NoyauIn>>,
    /// Altitude NGF du niveau de référence (plancher bas), en mètres.
    pub cote: f64,
    pub nom: String,
}

impl Default for Ouvrage {
    fn default() -> Self {
        Ouvrage {
            actif: false,
            facade: 0.20,
            poteau: [0.30, 0.60],
            travee: 3,
            rives: true,
            densite: Densite::Longue,
            retrait: 0.65,
            systeme: Systeme::Coule,
            angle: None,
            emprise: None,
            hauteur_etage: 2.90,
            niveaux: 1,
            implantation: Implantation::Auto,
            pente_rampe: 0.18,
            largement_ventile: true,
            ascenseurs_demi: noyaux::AscenseursDemi::Deux,
            noyaux: None,
            cote: 0.0,
            nom: "Parc de stationnement".into(),
        }
    }
}

/// Emprise du plateau saisie à la main : centre, longueur le long des allées `w`, largeur en
/// travers `h`, direction des allées `rot` (degrés). Les dimensions sont ramenées aux plus proches
/// réalisables ; `fixe` (−1, 0 ou 1 par axe local) désigne le côté qui ne bouge pas pendant ce
/// recalage, 0 gardant le centre.
/// Noyau placé à la main : nature et point du site, recalé par le moteur sur la place la plus proche.
#[derive(Deserialize, Clone, Debug)]
pub struct NoyauIn {
    pub nature: noyaux::Nature,
    pub p: P2,
}

#[derive(Deserialize, Clone, Debug)]
pub struct EmpriseIn {
    pub cx: f64,
    pub cy: f64,
    pub w: f64,
    pub h: f64,
    pub rot: f64,
    #[serde(default)]
    pub fixe: [f64; 2],
}

fn emprise_manuelle(g: &Gabarit, m: &EmpriseIn) -> Emprise {
    let (larg, long) = plateau::dimensions_proches(g, m.h, m.w);
    let r = m.rot.to_radians();
    let u = v2(r.cos(), r.sin());
    let v = u.left();
    let centre = v2(m.cx, m.cy)
        - u * (m.fixe[0] * (long - m.w) / 2.0)
        - v * (m.fixe[1] * (larg - m.h) / 2.0);
    Emprise {
        o: centre - u * (long / 2.0) - v * (larg / 2.0),
        u,
        v,
        largeur: larg,
        longueur: long,
    }
}

impl Ouvrage {
    /// Pente retenue : entre 5 % et 18 % (NF P 91-100, § 3.4).
    fn pente(&self) -> f64 {
        self.pente_rampe.clamp(0.05, nf::PENTE_RAMPE_MAX.v)
    }
    /// Longueur horizontale d'une rampe d'un étage, raccords R 10 en sommet et R 15 en pied compris.
    pub fn longueur_rampe(&self) -> f64 {
        normes::longueur_rampe(self.hauteur_etage, self.pente(), nf::RAYON_SOMMET_MIN.v, nf::RAYON_PIED_MIN.v)
    }

    pub fn gabarit(&self, site: &Site) -> Gabarit {
        let p = &site.params;
        Gabarit {
            w: p.stall_w,
            d: p.stall_d,
            allee: p.aisle,
            facade: self.facade.max(0.0),
            pu: self.poteau[0].max(0.05),
            pv: self.poteau[1].max(0.05),
            travee: self.travee.max(1),
            rives: self.rives,
            densite: self.densite,
            retrait: self.retrait.max(0.0),
            rampe: (self.niveaux > 1).then(|| self.longueur_rampe()),
            rampe_largeur: neufert::RAMPE_SENS_UNIQUE.v,
            implantation: self.implantation,
            hauteur: self.hauteur_etage,
        }
    }
}

/// Rampe : rectangle englobant, contour, axe dans le sens de la montée (ou de la descente),
/// extrémités de l'axe, pente et longueur mesurées sur l'axe (voie intérieure d'une demi-rampe à
/// double sens, NF P 91-100 § 3.4).
#[derive(Serialize, Clone, Debug, Default)]
pub struct RampeOut {
    pub q: [P2; 4],
    pub contour: Vec<P2>,
    pub axe: Vec<P2>,
    pub de: P2,
    pub a: P2,
    pub montee: bool,
    pub double_sens: bool,
    /// "droite", "u" (demi-rampe des demi-niveaux) ou "helice".
    pub forme: String,
    /// Vide intérieur (noyau d'une tour hélicoïdale).
    pub trou: Vec<P2>,
    pub pente: f64,
    pub longueur: f64,
    pub cle: String,
}

/// Épure de giration (NF P 91-100, § 3.3) : trajectoire, contour de la bande balayée, obstacles.
#[derive(Serialize, Clone, Debug, Default)]
pub struct GirationOut {
    pub cle: String,
    pub libelle: String,
    pub axe: Vec<P2>,
    pub contour: Vec<P2>,
    pub conflits: Vec<String>,
}

/// Escalier ou ascenseur : emprise, porte sur l'axe de l'allée.
#[derive(Serialize, Clone, Debug, Default)]
pub struct NoyauOut {
    pub q: [P2; 4],
    pub porte: P2,
    pub ascenseur: bool,
    pub liaison: bool,
    pub cle: String,
}

/// Bilan d'évacuation (PS 13) : distances admises, plus grande distance, places au-delà.
#[derive(Serialize, Clone, Debug, Default)]
pub struct EvacuationOut {
    pub largement_ventile: bool,
    pub limite_deux: f64,
    pub limite_un: f64,
    pub distance_max: f64,
    pub hors: usize,
    pub escaliers: usize,
    pub ascenseurs: usize,
    pub liaisons: usize,
    pub manuel: bool,
}

/// File de poteaux (axe de trame), prolongée de 3 m au-delà de la dalle.
#[derive(Serialize, Clone, Debug, Default)]
pub struct AxeOut {
    pub nom: String,
    pub a: P2,
    pub b: P2,
}

/// Plancher du système constructif (voir `systemes::Plancher`).
#[derive(Serialize, Clone, Debug, Default)]
pub struct PlancherOut {
    pub code: String,
    pub court: String,
    pub libelle: String,
    pub epaisseur: f64,
    pub retombee: f64,
    /// Hauteur d'étage minimale : 2,20 m sous plafond, 2,00 m sous poutre (NF P 91-100, § 3.2).
    pub hauteur_min: f64,
    pub source: String,
    /// Poutres ou solives : profil, section (m), solives (acier) ou poutres.
    pub ossature: Option<OssatureOut>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct OssatureOut {
    pub profil: String,
    pub b: f64,
    pub h: f64,
    pub solive: bool,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct OuvrageOut {
    /// Contour de la dalle.
    pub dalle: [P2; 4],
    /// Direction des allées (degrés).
    pub angle: f64,
    pub largeur: f64,
    pub longueur: f64,
    pub modules: u32,
    pub demi_module: bool,
    /// Plus grande distance entre deux files de poteaux voisines, en travers des allées.
    pub portee: f64,
    pub systeme: String,
    pub plancher: PlancherOut,
    /// Matériau des poteaux pour les types BIM : "BA" ou "Acier".
    pub poteau_materiau: String,
    pub axes: Vec<AxeOut>,
    pub poteaux: Vec<[P2; 4]>,
    pub poteaux_cles: Vec<String>,
    /// Poutres ou solives (empreinte en plan) et leurs clés BIM.
    pub poutres: Vec<[P2; 4]>,
    pub poutres_cles: Vec<String>,
    pub allees: Vec<[P2; 4]>,
    pub allees_cles: Vec<String>,
    /// Clé BIM de chaque place, dans l'ordre de `Layout::stalls`.
    pub places_cles: Vec<String>,
    pub places_rive: usize,
    pub places_modules: usize,
    pub niveaux: u32,
    /// Places d'un niveau courant, du rez et du dernier niveau.
    pub places_niveau: usize,
    pub places_rdc: usize,
    pub places_dernier: usize,
    /// Implantation des rampes retenue : "rangee", "pignon", "demi", ou vide sans rampe.
    pub implantation: String,
    /// Demi-niveaux décalés : chaque niveau compte deux demi-plateaux.
    pub demi_niveaux: bool,
    /// Plateaux inclinés : les deux modules montent d'un demi-étage chacun.
    pub plateaux_inclines: bool,
    /// Rampes droites : emprise de la voie, axe dans le sens de parcours (de → à), pente, longueur.
    pub rampes: Vec<RampeOut>,
    pub girations: Vec<GirationOut>,
    /// Escaliers et ascenseur (plusieurs niveaux), et le bilan d'évacuation.
    pub noyaux: Vec<NoyauOut>,
    pub evacuation: Option<EvacuationOut>,
    pub surface: f64,
    pub m2_place: f64,
    pub hauteur_etage: f64,
    pub dalle_ep: f64,
    pub cote: f64,
    pub nom: String,
}

/// Longueur en mètres, deux décimales, virgule.
pub(crate) fn fr2(x: f64) -> String {
    format!("{x:.2}").replace('.', ",")
}

fn r3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}
fn p2(p: V2) -> P2 {
    [r3(p.x), r3(p.y)]
}

/// Orientations à essayer (degrés, modulo 90) : directions des limites, puis un balayage tous les 3°.
fn angles_candidats(rings: &[Vec<V2>]) -> Vec<f64> {
    let mut a: Vec<f64> = Vec::new();
    for r in rings {
        for i in 0..r.len() {
            let d = r[(i + 1) % r.len()] - r[i];
            if d.len() > 5.0 {
                a.push((d.y.atan2(d.x).to_degrees().rem_euclid(90.0) * 10.0).round() / 10.0);
            }
        }
    }
    a.extend((0..30).map(|k| k as f64 * 3.0));
    let mut out: Vec<f64> = Vec::new();
    for x in a {
        if !out.iter().any(|y| (x - y).abs() < 0.05 || (x - y).abs() > 89.95) {
            out.push(x);
        }
    }
    out
}

/// Plateau d'un niveau : emprise optimisée dans la zone stationnable, puis construction.
pub fn solve(site: &Site, o: &Ouvrage) -> Layout {
    let p = &site.params;
    let mut out = Layout::default();
    let Some(z) = region::build(site) else {
        out.warnings.push("Parcelle invalide".into());
        return out;
    };
    let g = o.gabarit(site);
    let rings = region::rings_of(&z.b, 15.0);
    let (angles, impose) = match (o.angle, p.quick, p.angle_hint) {
        (Some(a), _, _) => (vec![a], true),
        (None, true, Some(h)) => (vec![h.rem_euclid(90.0)], false),
        _ => (angles_candidats(&rings), false),
    };
    let parcel_area = signed_area(&z.parcel).abs();
    out.setback_line = region::rings_of(&z.setback, 1.0)
        .iter()
        .map(|r| r.iter().map(|q| p2(*q)).collect())
        .collect();
    out.metrics = Metrics {
        parcel_area: r3(parcel_area),
        buildable_area: r3(region::mp_area(&z.b)),
        ..Default::default()
    };
    if p.slant < 89.5 {
        out.warnings.push("Silo : places à 90° seulement dans ce lot".into());
    }
    // Poteaux en tête de place : rangées (densités moyenne et courte, profondeur du poteau en
    // travers) et places de rive (profondeur le long des allées).
    let tetes: Vec<f64> = [(g.densite != Densite::Longue, g.pv), (g.rives, g.pu)]
        .into_iter()
        .filter_map(|(oui, ep)| oui.then_some(ep))
        .collect();
    if !tetes.is_empty() {
        if g.retrait < 0.65 - 1e-9 {
            out.warnings
                .push("Poteaux à moins de 0,65 m de l'allée : places à élargir (NF P 91-100, tab. 3)".into());
        }
        if tetes.iter().any(|ep| g.retrait + ep > 1.25 + 1e-9) {
            out.warnings
                .push("Poteaux au-delà de 1,25 m de l'allée : gêne à l'ouverture des portières (Neufert p. 274)".into());
        }
    }
    let e = if let Some(m) = &o.emprise {
        let e = emprise_manuelle(&g, m);
        let dalle = MultiPolygon::new(vec![region::to_poly(&[
            e.monde(v2(0.0, 0.0)),
            e.monde(v2(e.longueur, 0.0)),
            e.monde(v2(e.longueur, e.largeur)),
            e.monde(v2(0.0, e.largeur)),
        ])]);
        let hors = |zone: &MultiPolygon<f64>| region::mp_area(&dalle.difference(zone)) > 0.5;
        if hors(&MultiPolygon::new(vec![region::to_poly(&z.parcel)])) {
            out.warnings.push("La dalle dépasse la parcelle".into());
        } else if hors(&z.b) {
            out.warnings
                .push("La dalle sort de la zone constructible (reculs, bâtiments, servitudes, espaces verts)".into());
        }
        e
    } else {
        let Some((e, _)) = plateau::chercher(&g, &rings, &angles, 0.5, impose) else {
            out.warnings.push("Emprise trop petite pour une rangée de places et son allée".into());
            return out;
        };
        e
    };
    let e = repere_usuel(e);
    let Some(mut plan) = plateau::construire(&g, e.largeur, e.longueur) else {
        out.warnings.push("Emprise trop petite pour une rangée de places et son allée".into());
        return out;
    };
    // Escaliers et ascenseur au-delà d'un niveau (PS 13).
    let evac = (o.niveaux > 1).then(|| match &o.noyaux {
        Some(saisis) => {
            // Points du site ramenés dans le repère du plateau (u, v orthonormés).
            let local = |p: P2| {
                let d = v2(p[0], p[1]) - e.o;
                v2(d.dot(e.u), d.dot(e.v))
            };
            let s: Vec<(noyaux::Nature, V2)> = saisis.iter().map(|n| (n.nature, local(n.p))).collect();
            noyaux::fixer(&mut plan, &g, o.largement_ventile, &s)
        }
        None => noyaux::placer(&mut plan, &g, o.largement_ventile, o.ascenseurs_demi),
    });
    remplir(&mut out, site, o, &g, &e, &plan, evac.as_ref());
    // Tour hors de la dalle en travers (tour en façade) : la recherche ne voit que la dalle.
    let hors_dalle = |q: &V2| q.y < -1e-6 || q.y > plan.largeur() + 1e-6;
    if plan.rampes.iter().filter(|r| r.contour.iter().any(hors_dalle)).any(|r| {
        let tour = MultiPolygon::new(vec![region::to_poly(&r.contour.iter().map(|&q| e.monde(q)).collect::<Vec<_>>())]);
        region::mp_area(&tour.difference(&z.b)) > 0.5
    }) {
        out.warnings.push("La tour hélicoïdale sort de la zone constructible (reculs, bâtiments, servitudes, espaces verts)".into());
    }
    out
}

/// Oriente le repère pour la lecture du plan : files numérotées de gauche à droite (ou de bas en
/// haut pour des allées verticales), files lettrées de bas en haut (ou de gauche à droite). La dalle
/// garde son centre ; le repère peut devenir indirect, sans effet sur la géométrie.
fn repere_usuel(e: Emprise) -> Emprise {
    let centre = e.o + e.u * (e.longueur / 2.0) + e.v * (e.largeur / 2.0);
    let u = if e.u.x < -1e-9 || (e.u.x.abs() <= 1e-9 && e.u.y < 0.0) {
        -e.u
    } else {
        e.u
    };
    let l = u.left();
    let v = if l.x + l.y >= -1e-9 { l } else { -l };
    Emprise {
        o: centre - u * (e.longueur / 2.0) - v * (e.largeur / 2.0),
        u,
        v,
        ..e
    }
}

fn remplir(out: &mut Layout, site: &Site, o: &Ouvrage, g: &Gabarit, e: &Emprise, plan: &Plan, evac: Option<&noyaux::Evacuation>) {
    let p = &site.params;
    let w = |q: V2| e.monde(q);
    let quad = |q: &[V2; 4]| [p2(w(q[0])), p2(w(q[1])), p2(w(q[2])), p2(w(q[3]))];
    let (larg, long) = (plan.largeur(), plan.longueur());

    out.stalls = plan
        .places
        .iter()
        .map(|pl| StallOut {
            q: quad(&pl.q),
            k: KIND_STD,
            e: false,
        })
        .collect();
    // Places : niveau courant (le plan affiché) ; au rez et au dernier niveau, les places des
    // épures de giration inutiles à ces niveaux en plus.
    let niv = o.niveaux.max(1) as usize;
    // Pente d'une rampe : celle des réglages pour les rampes droites ; un demi-étage sur l'axe de
    // la voie intérieure pour une demi-rampe en U.
    let pente = |r: &plateau::RampeLoc| match r.forme {
        plateau::Forme::Droite => o.pente(),
        plateau::Forme::DemiU => o.hauteur_etage / 2.0 / r.longueur,
        plateau::Forme::Helice => o.hauteur_etage / r.longueur,
        plateau::Forme::Virage => 0.0,
        plateau::Forme::Plateau => o.hauteur_etage / 2.0 / r.longueur,
    };
    let rives = |v: &[plateau::PlaceLoc]| v.iter().filter(|pl| pl.rive).count();
    let n_niv = out.stalls.len();
    let (n_rdc, n_dernier) = (n_niv + plan.places_rdc.len(), n_niv + plan.places_dernier.len());
    let rive_niv = rives(&plan.places);
    let (n, rive) = if niv > 1 {
        (
            n_rdc + (niv - 2) * n_niv + n_dernier,
            niv * rive_niv + rives(&plan.places_rdc) + rives(&plan.places_dernier),
        )
    } else {
        (n_niv, rive_niv)
    };

    // Chaussée : réunion des allées.
    let aq: Vec<Quad> = plan
        .allees
        .iter()
        .map(|(q, _)| Quad([w(q[0]), w(q[1]), w(q[2]), w(q[3])]))
        .collect();
    let paved = crate::mp_of_quads(&aq);
    out.paved = region::rings_of(&paved, 0.5)
        .iter()
        .map(|r| r.iter().map(|q| p2(*q)).collect())
        .collect();

    // Dalle : l'emprise, moins les bouts occupés par les tours quand elles sont hors de la dalle.
    let rt = plan.retrait;
    let corners = [v2(rt, 0.0), v2(long - rt, 0.0), v2(long - rt, larg), v2(rt, larg)];
    let ext = 3.0;
    let mut axes: Vec<AxeOut> = Vec::new();
    for (i, &u) in plan.axes_u.iter().enumerate() {
        axes.push(AxeOut {
            nom: (i + 1).to_string(),
            a: p2(w(v2(u, -ext))),
            b: p2(w(v2(u, larg + ext))),
        });
    }
    for (k, &v) in plan.axes_v.iter().enumerate() {
        axes.push(AxeOut {
            nom: plateau::lettre(k),
            a: p2(w(v2(rt - ext, v))),
            b: p2(w(v2(long - rt + ext, v))),
        });
    }
    let (hu, hv) = (g.pu / 2.0, g.pv / 2.0);
    let surface = larg * (long - 2.0 * rt);
    let angle = e.u.y.atan2(e.u.x).to_degrees().rem_euclid(180.0);
    let need = |ratio: f64| -> usize {
        if ratio <= 0.0 || n == 0 {
            0
        } else {
            ((n as f64 * ratio) - 1e-9).ceil().max(1.0) as usize
        }
    };
    let irve_req = need(p.irve_ratio);
    out.metrics.stalls = n;
    out.metrics.standard = n;
    out.metrics.pmr_required = need(p.pmr_ratio);
    out.metrics.irve_required = irve_req;
    out.metrics.irve_equipped_required = if irve_req > 0 && p.irve_equip_ratio > 0.0 {
        (((n as f64 * p.irve_equip_ratio) + 1e-9).floor() as usize).clamp(1, irve_req)
    } else {
        0
    };
    out.metrics.paved_area = r3(surface * niv as f64);
    out.metrics.perimeter_stalls = rive;
    out.metrics.interior_stalls = n - rive;
    if n > 0 {
        out.metrics.m2_paved_per_stall = r3(surface * niv as f64 / n as f64);
        out.metrics.m2_parcel_per_stall = r3(out.metrics.parcel_area / n as f64);
    }
    // Pas d'espaces verts calculés en ouvrage : la parcelle hors dalle n'est pas qualifiée.
    out.metrics.green_area = 0.0;
    out.angle = r3(angle);
    out.row_aisle = p.aisle;

    // Plancher du système constructif : portée en travers, travées le long des rangées et aux
    // extrémités ; poutres ou solives de poteau à poteau.
    let portee = plan.axes_v.windows(2).map(|p| p[1] - p[0]).fold(0.0, f64::max);
    let (t_rangees, t_bouts) = plateau::travees(plan);
    let pl = systemes::plancher(o.systeme, g.densite, portee, t_rangees, t_bouts);
    let poutres = pl.ossature.as_ref().map_or(Vec::new(), |os| plateau::ossature(plan, os.sens, os.b));
    out.warnings.extend(pl.alertes.iter().cloned());
    let h_min = pl.hauteur_min();
    if o.hauteur_etage < h_min - 1e-9 {
        out.warnings.push(format!(
            "Hauteur d'étage de {} m sous le minimum de {} m (NF P 91-100, § 3.2 : 2,20 m sous plafond, 2,00 m sous poutre ; {})",
            fr2(o.hauteur_etage),
            fr2(h_min),
            pl.court
        ));
    }
    let tw = g.travee as f64 * g.w;
    if o.systeme == Systeme::Prefa && ((tw / systemes::DAP_LARGEUR).round() * systemes::DAP_LARGEUR - tw).abs() > 1e-6 {
        out.warnings.push(format!(
            "Travée de {} m : dalles alvéolées de 1,20 m à démoduler au droit des poteaux (places de 2,40 m : travées multiples de 1,20 m)",
            fr2(tw)
        ));
    }

    // Rampes : implantation possible, épures de giration dégagées, hauteur libre perpendiculaire
    // à la rampe (2,00 m, majorée de la flèche du raccord de sommet, NF P 91-100, § 3.2) sous le
    // plancher du dessus.
    if niv > 1 {
        if plan.rampes.is_empty() {
            out.warnings.extend(plan.refus.iter().cloned());
        }
        for gi in plan.girations.iter().filter(|gi| !gi.conflits.is_empty()) {
            out.warnings.push(format!(
                "Giration : {} gênée par {} (NF P 91-100, § 3.3 : voie de 4,00 m en courbe, rayon extérieur de 8,50 m)",
                gi.libelle.to_lowercase(),
                gi.conflits.join(", ")
            ));
        }
        if o.pente_rampe > nf::PENTE_RAMPE_MAX.v + 1e-9 && plan.rampes.iter().any(|r| r.forme == plateau::Forme::Droite) {
            out.warnings.push("Pente des rampes ramenée à 18 % (NF P 91-100, § 3.4)".into());
        }
        if let Some(r) = plan.rampes.iter().find(|r| r.forme == plateau::Forme::Plateau) {
            if pente(r) > neufert::PLANCHER_INCLINE_MAX.v + 1e-9 {
                out.warnings.push(format!(
                    "Plateaux inclinés à {} % : Neufert conseille 6 % au plus (rangées plus longues)",
                    fr2(pente(r) * 100.0)
                ));
            }
            out.warnings.push(
                "Plateaux inclinés : places PMR sur une partie horizontale, dévers de 2 % au plus (arrêté du 20 avril 2017, art. 3)".into(),
            );
        }
        if let Some(r) = plan.rampes.iter().find(|r| matches!(r.forme, plateau::Forme::DemiU | plateau::Forme::Helice) && pente(r) > nf::PENTE_RAMPE_MAX.v + 1e-9) {
            out.warnings.push(format!(
                "Rampe courbe à {} % dans l'axe de la voie (intérieure), plus de 18 % (NF P 91-100, § 3.4)",
                (pente(r) * 100.0).round()
            ));
        }
        let p_max = plan.rampes.iter().map(&pente).fold(0.0, f64::max);
        let libre = (o.hauteur_etage - pl.epaisseur) * p_max.atan().cos();
        let requis = nf::HAUTEUR_RAMPE.v + normes::fleche(nf::RAYON_SOMMET_MIN.v);
        if libre < requis - 1e-9 {
            out.warnings.push(format!(
                "Hauteur libre sur rampe de {} m sous le minimum de {} m (NF P 91-100, § 3.2 : 2,00 m perpendiculaires à la rampe, plus la flèche du raccord)",
                fr2(libre),
                fr2(requis)
            ));
        }
    }
    if let Some(ev) = evac {
        if ev.hors > 0 {
            out.warnings.push(format!(
                "PS 13 : {} places au-delà de {} m d'un escalier ({} m hors de deux escaliers opposés), mesurés dans l'axe des circulations",
                ev.hors,
                fr2(ev.limites.0),
                fr2(ev.limites.1)
            ));
        }
        if o.largement_ventile && larg >= ps::PSLV_DISTANCE_FACADES.v && long >= ps::PSLV_DISTANCE_FACADES.v {
            out.warnings.push("PS 3 : façades opposées à 75 m ou plus, le parc n'est pas largement ventilé".into());
        }
        if !ev.portes_genees.is_empty() {
            out.warnings.push(format!("Noyaux : porte dans l'épure de giration d'une rampe ({})", ev.portes_genees.iter().map(|c| c.trim_start_matches("NY/")).collect::<Vec<_>>().join(", ")));
        }
        if plan.noyaux.iter().any(|n| n.liaison) {
            out.warnings.push("Demi-niveaux : un seul ascenseur, places PMR sur son demi-plateau ; escalier de liaison vers l'autre".into());
        }
    }
    out.warnings.push(if niv > 1 {
        "Silo : cages d'escalier et d'ascenseur dans l'emprise d'une place, à dimensionner ; PMR et IRVE à placer près de l'ascenseur".into()
    } else {
        "Silo, lot 1 : plateau unique, sans rampes ni noyaux ; PMR et IRVE à placer avec les ascenseurs".into()
    });
    out.ouvrage = Some(OuvrageOut {
        dalle: [p2(w(corners[0])), p2(w(corners[1])), p2(w(corners[2])), p2(w(corners[3]))],
        angle: r3(angle),
        largeur: r3(larg),
        longueur: r3(long),
        modules: plan.travers.modules,
        demi_module: plan.travers.demi,
        portee: r3(portee),
        systeme: o.systeme.nom().into(),
        plancher: PlancherOut {
            code: pl.code.clone(),
            court: pl.court.clone(),
            libelle: pl.libelle.clone(),
            epaisseur: r3(pl.epaisseur),
            retombee: r3(pl.retombee),
            hauteur_min: r3(h_min),
            source: pl.source.into(),
            ossature: pl.ossature.as_ref().map(|os| OssatureOut {
                profil: os.profil.clone(),
                b: r3(os.b),
                h: r3(os.h),
                solive: os.solive,
            }),
        },
        poteau_materiau: o.systeme.materiau_poteau().into(),
        axes,
        poteaux: plan
            .poteaux
            .iter()
            .map(|po| quad(&[po.c + v2(-hu, -hv), po.c + v2(hu, -hv), po.c + v2(hu, hv), po.c + v2(-hu, hv)]))
            .collect(),
        poteaux_cles: plan.poteaux.iter().map(|po| po.cle.clone()).collect(),
        poutres: poutres.iter().map(|(q, _)| quad(q)).collect(),
        poutres_cles: poutres.iter().map(|(_, c)| c.clone()).collect(),
        allees: plan.allees.iter().map(|(q, _)| quad(q)).collect(),
        allees_cles: plan.allees.iter().map(|(_, c)| c.clone()).collect(),
        places_cles: plan.places.iter().map(|pl| pl.cle.clone()).collect(),
        places_rive: rive_niv,
        places_modules: n_niv - rive_niv,
        niveaux: niv as u32,
        places_niveau: n_niv,
        places_rdc: n_rdc,
        places_dernier: n_dernier,
        implantation: plan.implantation.map_or(String::new(), |i| i.nom().into()),
        demi_niveaux: plan.implantation == Some(Implantation::DemiNiveaux),
        plateaux_inclines: plan.implantation == Some(Implantation::Inclines),
        rampes: plan
            .rampes
            .iter()
            .map(|r| {
                let (de, a) = if r.montee { (r.bas, r.haut) } else { (r.haut, r.bas) };
                RampeOut {
                    q: quad(&r.q),
                    contour: r.contour.iter().map(|&q| p2(w(q))).collect(),
                    axe: r.axe.iter().map(|&q| p2(w(q))).collect(),
                    de: p2(w(de)),
                    a: p2(w(a)),
                    montee: r.montee,
                    double_sens: r.double_sens,
                    forme: r.forme.nom().into(),
                    trou: r.trou.iter().map(|&q| p2(w(q))).collect(),
                    pente: r3(pente(r)),
                    longueur: r3(r.longueur),
                    cle: r.cle.clone(),
                }
            })
            .collect(),
        girations: plan
            .girations
            .iter()
            .map(|gi| GirationOut {
                cle: gi.cle.clone(),
                libelle: gi.libelle.clone(),
                axe: gi.axe.iter().map(|&q| p2(w(q))).collect(),
                contour: gi.contour.iter().map(|&q| p2(w(q))).collect(),
                conflits: gi.conflits.clone(),
            })
            .collect(),
        noyaux: plan
            .noyaux
            .iter()
            .map(|n| NoyauOut { q: quad(&n.q), porte: p2(w(n.porte)), ascenseur: n.ascenseur, liaison: n.liaison, cle: n.cle.clone() })
            .collect(),
        evacuation: evac.map(|ev| EvacuationOut {
            largement_ventile: o.largement_ventile,
            limite_deux: ev.limites.0,
            limite_un: ev.limites.1,
            distance_max: r3(ev.distance_max),
            hors: ev.hors,
            escaliers: plan.noyaux.iter().filter(|n| !n.ascenseur && !n.liaison).count(),
            ascenseurs: plan.noyaux.iter().filter(|n| n.ascenseur).count(),
            liaisons: plan.noyaux.iter().filter(|n| n.liaison).count(),
            manuel: ev.manuel,
        }),
        surface: r3(surface),
        m2_place: if n > 0 { r3(surface * niv as f64 / n as f64) } else { 0.0 },
        hauteur_etage: o.hauteur_etage,
        dalle_ep: r3(pl.epaisseur),
        cote: o.cote,
        nom: o.nom.clone(),
    });
}
