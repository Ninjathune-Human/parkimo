//! Plateau de silo à grande portée : modules de rangées doubles (place + allée + place) côte à
//! côte, poteaux en façade et sur les lignes de dos à dos, places de rive le long des façades
//! d'extrémité, allées transversales aux deux bouts.
//!
//! Repère local : u le long des allées, v en travers ; origine au coin de la dalle.
//!
//! Poteaux, à cheval sur une séparation de places ou contre l'allée transversale :
//! - au fond de place, dans le premier mètre (≤ 1,10 m, NF P 91-100 fig. 2) ; la largeur au droit
//!   de l'obstacle reste d'au moins 2,00 m pour des places de 2,30 m et plus ;
//! - en tête de place (densités moyenne et courte, places de rive), le nu à `retrait` du bord de
//!   l'allée : 0,65 m au moins sans surlargeur (NF P 91-100, tab. 3), le poteau entier dans les
//!   1,25 m côté allée pour garder libre la zone des portières (Neufert p. 274).

use super::giration::{self, Trace, Troncon};
use super::systemes::Sens;
use crate::geom::{rotate_rings, strip_intervals, v2, V2};
use serde::Deserialize;
use std::f64::consts::{FRAC_PI_2, PI};

/// Densité de la trame en travers des allées.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Densite {
    /// Files en façade et sur les lignes de dos à dos : portée d'un module (16 m).
    #[default]
    Longue,
    /// Files en tête des deux rangées de chaque dos à dos, en retrait de l'allée.
    Moyenne,
    /// Files en tête de chaque rangée, de part et d'autre de chaque allée.
    Courte,
}

/// Implantation des rampes droites entre niveaux (voir `docs/corpus-rampes.md`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Implantation {
    /// Celle qui garde le plus de places par niveau courant.
    #[default]
    Auto,
    /// Le long des allées, dans la rangée de façade : montée depuis le premier bout contre la
    /// façade v bas, descente depuis le second contre la façade opposée.
    Rangee,
    /// En travers des allées, dans les bandes de pignon : montée à un bout, descente à l'autre.
    Pignon,
}

impl Implantation {
    pub fn nom(self) -> &'static str {
        match self {
            Implantation::Auto => "auto",
            Implantation::Rangee => "rangee",
            Implantation::Pignon => "pignon",
        }
    }
}

/// Dimensions du plateau.
#[derive(Clone, Copy, Debug)]
pub struct Gabarit {
    /// Largeur de place.
    pub w: f64,
    /// Longueur de place.
    pub d: f64,
    pub allee: f64,
    /// Épaisseur de rive en façade (garde-corps, nez de dalle), du nu extérieur au fond des places.
    pub facade: f64,
    /// Poteau : dimension le long des allées.
    pub pu: f64,
    /// Poteau : dimension en travers.
    pub pv: f64,
    /// Places entre deux files de poteaux.
    pub travee: u32,
    /// Places de rive le long des façades d'extrémité.
    pub rives: bool,
    pub densite: Densite,
    /// Poteau en tête de place : du nu du poteau au bord de l'allée.
    pub retrait: f64,
    /// Plusieurs niveaux : longueur horizontale des rampes droites (raccords compris).
    pub rampe: Option<f64>,
    /// Largeur de la voie de rampe à sens unique.
    pub rampe_largeur: f64,
    pub implantation: Implantation,
}

impl Gabarit {
    /// Module de rangée double : place + allée + place.
    pub fn module(&self) -> f64 {
        2.0 * self.d + self.allee
    }
    /// Zone d'extrémité depuis le nu intérieur de la façade : rive (places de pignon ou rampe en
    /// pignon) et allée transversale.
    fn bout(&self) -> f64 {
        if self.rives || (self.rampe.is_some() && self.implantation == Implantation::Pignon) {
            self.d + self.allee
        } else {
            self.pu + self.allee
        }
    }
}

/// Découpage en travers : modules entiers, demi-module (rangée + allée) éventuel, largeur utile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Travers {
    pub modules: u32,
    pub demi: bool,
    pub largeur: f64,
}

pub fn travers(g: &Gabarit, larg: f64) -> Option<Travers> {
    let m = g.module();
    let dispo = larg - 2.0 * g.facade;
    if dispo <= 0.0 {
        return None;
    }
    let n = ((dispo + 1e-9) / m).floor() as u32;
    let demi_l = g.d + g.allee + g.pv;
    let demi = dispo - n as f64 * m + 1e-9 >= demi_l;
    if n == 0 && !demi {
        return None;
    }
    let largeur = 2.0 * g.facade + n as f64 * m + if demi { demi_l } else { 0.0 };
    Some(Travers {
        modules: n,
        demi,
        largeur,
    })
}

/// Découpage en long : places par rangée de module, début des rangées, longueur utile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Long {
    pub par_rangee: u32,
    pub debut: f64,
    pub longueur: f64,
}

pub fn long(g: &Gabarit, l: f64) -> Option<Long> {
    let debut = g.facade + g.bout();
    let n = ((l - 2.0 * debut + 1e-9) / g.w).floor();
    if n < 1.0 {
        return None;
    }
    Some(Long {
        par_rangee: n as u32,
        debut,
        longueur: 2.0 * debut + n * g.w,
    })
}

/// Files de poteaux en travers (positions v, croissantes) : les deux façades et, selon la densité,
/// les lignes de dos à dos (longue), la tête des deux rangées de chaque dos à dos (moyenne) ou la
/// tête de chaque rangée (courte).
pub fn lignes_v(g: &Gabarit, t: &Travers) -> Vec<f64> {
    let (m, d) = (g.module(), g.d);
    // Axe d'un poteau en tête de place, depuis le bord de l'allée.
    let r = g.retrait + g.pv / 2.0;
    // Dos à dos : entre deux modules, puis entre le dernier module et le demi-module.
    let dos = (1..t.modules)
        .chain((t.demi && t.modules > 0).then_some(t.modules))
        .map(|k| g.facade + k as f64 * m);
    let mut out = vec![g.facade + g.pv / 2.0];
    match g.densite {
        Densite::Longue => out.extend(dos),
        Densite::Moyenne => {
            for b in dos {
                out.extend([b - (d - r), b + (d - r)]);
            }
        }
        Densite::Courte => {
            for k in 0..t.modules {
                let vb = g.facade + k as f64 * m;
                out.extend([vb + d - r, vb + m - d + r]);
            }
            if t.demi {
                out.push(g.facade + t.modules as f64 * m + d - r);
            }
        }
    }
    out.push(t.largeur - g.facade - g.pv / 2.0);
    out
}

/// Places de rive d'une façade d'extrémité : places entières entre deux files de poteaux.
fn rive_par_bout(g: &Gabarit, t: &Travers) -> usize {
    lignes_v(g, t)
        .windows(2)
        .map(|p| ((p[1] - p[0] + 1e-9) / g.w).floor() as usize)
        .sum()
}

/// Début des `k` places de rive entre les files `a` et `b` : calées contre la file intérieure quand
/// l'intervalle touche une façade (le reste part contre la façade), centrées sinon.
fn debut_rive(a: f64, b: f64, k: usize, w: f64, facade_a: bool, facade_b: bool) -> f64 {
    let reste = b - a - k as f64 * w;
    match (facade_a, facade_b) {
        (true, false) => a + reste,
        (false, true) => a,
        _ => a + reste / 2.0,
    }
}

/// Dimensions réalisables les plus proches de `larg` × `l` (l le long des allées) : en travers,
/// façades + modules entiers (+ demi-module) ; en long, extrémités + places entières. Au moins
/// une rangée de places et son allée.
pub fn dimensions_proches(g: &Gabarit, larg: f64, l: f64) -> (f64, f64) {
    let m = g.module();
    let demi_l = g.d + g.allee + g.pv;
    let n_max = ((larg - 2.0 * g.facade).max(0.0) / m).ceil() as u32 + 1;
    let mut best = 2.0 * g.facade + demi_l;
    for n in 0..=n_max {
        for demi in [false, true] {
            if n == 0 && !demi {
                continue;
            }
            let x = 2.0 * g.facade + n as f64 * m + if demi { demi_l } else { 0.0 };
            if (x - larg).abs() < (best - larg).abs() - 1e-9 {
                best = x;
            }
        }
    }
    let debut = g.facade + g.bout();
    let k = ((l - 2.0 * debut) / g.w).round().max(1.0);
    (best, 2.0 * debut + k * g.w)
}

/// Nombre de places d'un plateau `larg` × `l` (l le long des allées), sans rien construire ; avec
/// des rampes, celles d'un niveau courant (plateau construit).
pub fn compte(g: &Gabarit, larg: f64, l: f64) -> Option<(usize, Travers, Long)> {
    if g.rampe.is_some() {
        return construire(g, larg, l).map(|p| (p.places.len(), p.travers, p.long));
    }
    let t = travers(g, larg)?;
    let lo = long(g, l)?;
    let rangees = 2 * t.modules as usize + t.demi as usize;
    let mut n = rangees * lo.par_rangee as usize;
    if g.rives {
        n += 2 * rive_par_bout(g, &t);
    }
    Some((n, t, lo))
}

type Evaluation = Option<(bool, usize, Travers, Long)>;

/// Pour la recherche : rampes placées (ou inutiles), places d'un niveau courant. Un plateau où les
/// rampes demandées ne tiennent pas passe après tous ceux où elles tiennent. Avec des rampes, le
/// plateau ne dépend que des modules et des places par rangée (des deux implantations) : un même
/// découpage n'est construit qu'une fois.
fn evalue(g: &Gabarit, larg: f64, l: f64, memo: &mut std::collections::HashMap<(u32, bool, u32, u32), Evaluation>) -> Evaluation {
    if g.rampe.is_none() {
        return compte(g, larg, l).map(|(n, t, lo)| (true, n, t, lo));
    }
    let t = travers(g, larg)?;
    let n = |i: Implantation| long(&Gabarit { implantation: i, ..*g }, l).map_or(0, |lo| lo.par_rangee);
    let cle = (t.modules, t.demi, n(Implantation::Rangee), n(Implantation::Pignon));
    *memo
        .entry(cle)
        .or_insert_with(|| construire(g, larg, l).map(|p| (!p.rampes.is_empty(), p.places.len(), p.travers, p.long)))
}

/// Place en repère local : [fond a, fond b, avant b, avant a] et sa clé BIM.
#[derive(Clone, Debug)]
pub struct PlaceLoc {
    pub q: [V2; 4],
    pub cle: String,
    pub rive: bool,
}

/// Rampe droite à sens unique, en repère local : emprise de la voie, extrémités basse et haute de
/// son axe, montée ou descente.
#[derive(Clone, Debug)]
pub struct RampeLoc {
    pub q: [V2; 4],
    pub bas: V2,
    pub haut: V2,
    pub montee: bool,
    pub cle: String,
}

/// Épure de giration d'une entrée ou d'une sortie de rampe, en repère local (voir `giration`).
#[derive(Clone, Debug)]
pub struct Giration {
    pub cle: String,
    /// Clé de la rampe desservie.
    pub rampe: String,
    pub montee: bool,
    pub entree: bool,
    pub axe: Vec<V2>,
    pub contour: Vec<V2>,
    /// Obstacles dans l'épure : poteaux, autre rampe, façade.
    pub conflits: Vec<String>,
}

impl Giration {
    /// À l'extrémité basse de la rampe (entrée de la montée, sortie de la descente) : utile à tous
    /// les niveaux sauf le dernier ; à l'extrémité haute : à tous sauf le rez.
    pub fn bas(&self) -> bool {
        self.entree == self.montee
    }
    pub fn libelle(&self) -> &'static str {
        match (self.entree, self.montee) {
            (true, true) => "Entrée de la montée",
            (false, true) => "Sortie de la montée",
            (true, false) => "Entrée de la descente",
            (false, false) => "Sortie de la descente",
        }
    }
}

#[derive(Clone, Debug)]
pub struct PoteauLoc {
    pub c: V2,
    pub cle: String,
}

/// Plateau construit, en repère local.
#[derive(Clone, Debug)]
pub struct Plan {
    pub travers: Travers,
    pub long: Long,
    /// Places d'un niveau courant.
    pub places: Vec<PlaceLoc>,
    /// Places de plus au rez (épures des extrémités hautes inutiles) et au dernier niveau (épures
    /// des extrémités basses inutiles). Les emprises des rampes restent prises à tous les niveaux :
    /// au dernier, la montée qui arrive laisse une trémie.
    pub places_rdc: Vec<PlaceLoc>,
    pub places_dernier: Vec<PlaceLoc>,
    pub allees: Vec<([V2; 4], String)>,
    pub rampes: Vec<RampeLoc>,
    /// Implantation retenue, s'il y a des rampes.
    pub implantation: Option<Implantation>,
    pub girations: Vec<Giration>,
    /// Implantations impossibles et pourquoi (quand aucune rampe n'a pu être placée).
    pub refus: Vec<String>,
    pub poteaux: Vec<PoteauLoc>,
    /// Files transversales (positions u), numérotées 1, 2, 3…
    pub axes_u: Vec<f64>,
    /// Files longitudinales (positions v), lettrées A, B, C…
    pub axes_v: Vec<f64>,
}

impl Plan {
    pub fn largeur(&self) -> f64 {
        self.travers.largeur
    }
    pub fn longueur(&self) -> f64 {
        self.long.longueur
    }
}

/// Nom d'une file longitudinale : A … Z, puis AA, AB…
pub fn lettre(i: usize) -> String {
    let a = (b'A' + (i % 26) as u8) as char;
    if i < 26 {
        a.to_string()
    } else {
        format!("{}{}", (b'A' + (i / 26 - 1) as u8) as char, a)
    }
}

fn rect(u0: f64, v0: f64, u1: f64, v1: f64) -> [V2; 4] {
    [v2(u0, v0), v2(u1, v0), v2(u1, v1), v2(u0, v1)]
}

/// Deux rectangles alignés sur les axes se chevauchent (au-delà d'un contact).
fn chevauche(a: &[V2; 4], b: &[V2; 4]) -> bool {
    let bornes = |q: &[V2; 4]| {
        q.iter().fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |(x0, y0, x1, y1), p| {
            (x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y))
        })
    };
    let ((ax0, ay0, ax1, ay1), (bx0, by0, bx1, by1)) = (bornes(a), bornes(b));
    ax1.min(bx1) - ax0.max(bx0) > 1e-6 && ay1.min(by1) - ay0.max(by0) > 1e-6
}

/// Le plateau vu depuis le bout d'une rampe : allée transversale de ce bout, fin des rangées,
/// allées et rangées en travers (v croissants). La descente se construit comme la montée, sur le
/// plateau tourné d'un demi-tour.
struct Cadre {
    facade: f64,
    /// Allée transversale : u de `x0` à `x1` (début des rangées) ; bande de pignon avant `x0`.
    x0: f64,
    x1: f64,
    fin: f64,
    allees: Vec<(f64, f64)>,
    /// Rangées : v bas, v haut, allée juste au-dessus.
    rangees: Vec<(f64, f64, bool)>,
}

fn cadres(g: &Gabarit, t: &Travers, lo: &Long) -> [Cadre; 2] {
    let (m, d, a, f) = (g.module(), g.d, g.allee, g.facade);
    let fin = lo.debut + lo.par_rangee as f64 * g.w;
    let (mut allees, mut rangees) = (Vec::new(), Vec::new());
    for k in 0..t.modules {
        let vb = f + k as f64 * m;
        rangees.push((vb, vb + d, true));
        allees.push((vb + d, vb + d + a));
        rangees.push((vb + m - d, vb + m, false));
    }
    if t.demi {
        let vb = f + t.modules as f64 * m;
        rangees.push((vb, vb + d, true));
        allees.push((vb + d, vb + d + a));
    }
    let (l, w) = (lo.longueur, t.largeur);
    let mut allees2: Vec<(f64, f64)> = allees.iter().map(|&(v0, v1)| (w - v1, w - v0)).collect();
    allees2.sort_by(|p, q| p.0.total_cmp(&q.0));
    let mut rangees2: Vec<(f64, f64, bool)> = rangees.iter().map(|&(v0, v1, h)| (w - v1, w - v0, !h)).collect();
    rangees2.sort_by(|p, q| p.0.total_cmp(&q.0));
    [
        Cadre { facade: f, x0: lo.debut - a, x1: lo.debut, fin, allees, rangees },
        Cadre { facade: f, x0: l - fin - a, x1: l - fin, fin: l - lo.debut, allees: allees2, rangees: rangees2 },
    ]
}

/// Rampe et girations dans le repère d'un cadre : emprise, bouche (où l'on entre), bout (où l'on
/// sort), trajectoires d'entrée et de sortie, zone gardée libre de poteaux.
struct Esquisse {
    q: [V2; 4],
    bouche: V2,
    bout: V2,
    entree: Vec<Troncon>,
    sortie: Vec<Troncon>,
    zone: [V2; 4],
}

/// Amorce dessinée dans l'allée, avant l'entrée et après la sortie.
const AMORCE: f64 = 3.0;

/// Rampe le long des allées, dans la plus basse rangée qui a une allée juste au-dessus ; voie contre
/// l'allée. On l'aborde par l'allée transversale en venant d'une allée plus haute (deux quarts de
/// tour), on en sort en S dans l'allée voisine.
fn en_rangee(c: &Cadre, g: &Gabarit, lr: f64) -> Result<Esquisse, String> {
    let (r, mg, hs, wr) = (giration::rayon(), giration::marge(), giration::demi_droite(), g.rampe_largeur);
    let pre = "Rampes le long des allées";
    if c.x1 - c.x0 < mg + hs - 1e-9 {
        return Err(format!("{pre} : allée transversale de {} m, il en faut {} pour tourner", super::fr2(c.x1 - c.x0), super::fr2(mg + hs)));
    }
    let Some(&(r0, r1, _)) = c.rangees.iter().find(|x| x.2) else {
        return Err(format!("{pre} : pas de rangée contre une allée"));
    };
    if r1 - r0 < wr - 1e-9 {
        return Err(format!("{pre} : rangée de {} m, voie de rampe de {} m", super::fr2(r1 - r0), super::fr2(wr)));
    }
    let vl = r1 - wr / 2.0;
    // Trajectoire dans l'allée transversale, à 2,00 m de son bord côté pignon.
    let up = c.x0 + mg;
    let uc = up + r;
    let um = c.x1.max(uc);
    let Some(va) = c
        .allees
        .iter()
        .filter(|a| a.0 >= r1 - 1e-9)
        .map(|a| a.1 - mg)
        .find(|va| va - vl >= 2.0 * r - 1e-9)
    else {
        return Err(format!(
            "{pre} : il faut une autre allée à {} m au moins de la voie de rampe pour l'aborder (deux girations de {} m)",
            super::fr2(2.0 * r),
            super::fr2(r)
        ));
    };
    let entree = Trace::depuis(v2(uc + AMORCE, va), PI)
        .droit(AMORCE)
        .tourne(FRAC_PI_2)
        .droit(va - vl - 2.0 * r)
        .tourne(FRAC_PI_2)
        .droit(um - uc)
        .troncons;
    let haut = um + lr;
    // Sortie en S jusqu'à 1,50 m du bord de l'allée.
    let vt = r1 + hs;
    let phi = (1.0 - (vt - vl) / (2.0 * r)).acos();
    let s = 2.0 * r * phi.sin();
    if haut + s > c.fin + 1e-9 {
        return Err(format!(
            "{pre} : {} m de rangée nécessaires (giration d'entrée, rampe de {} m, giration de sortie), {} m disponibles",
            super::fr2(haut + s - c.x1),
            super::fr2(lr),
            super::fr2(c.fin - c.x1)
        ));
    }
    let sortie = Trace::depuis(v2(haut, vl), 0.0).tourne(phi).tourne(-phi).droit(AMORCE).troncons;
    Ok(Esquisse {
        q: rect(um, r1 - wr, haut, r1),
        bouche: v2(um, vl),
        bout: v2(haut, vl),
        entree,
        sortie,
        zone: rect(c.x1, r1 - wr, haut + s, r1),
    })
}

/// Rampe en pignon, contre l'allée transversale : on y entre depuis la plus haute allée (quart de
/// tour), on en sort dans la plus basse (quart de tour). Entre les deux trajectoires, la rampe et
/// deux rayons de giration.
fn en_pignon(c: &Cadre, g: &Gabarit, lr: f64) -> Result<Esquisse, String> {
    let (r, mg, wr) = (giration::rayon(), giration::marge(), g.rampe_largeur);
    let pre = "Rampes en pignon";
    let ul = c.x0 - wr / 2.0;
    if ul - mg < c.facade - 1e-9 {
        return Err(format!("{pre} : bande de pignon de {} m, il en faut {}", super::fr2(c.x0 - c.facade), super::fr2(wr / 2.0 + mg)));
    }
    let (Some(ha), Some(ba)) = (c.allees.last(), c.allees.first()) else {
        return Err(format!("{pre} : pas d'allée"));
    };
    let (va, vb) = (ha.1 - mg, ba.0 + mg);
    if va - vb < lr + 2.0 * r - 1e-9 {
        return Err(format!(
            "{pre} : {} m nécessaires d'une allée à l'autre (rampe de {} m et deux girations de {} m), {} m disponibles",
            super::fr2(lr + 2.0 * r),
            super::fr2(lr),
            super::fr2(r),
            super::fr2((va - vb).max(0.0))
        ));
    }
    let vm = va - r;
    let ua = c.x1.max(ul + r) + AMORCE;
    let entree = Trace::depuis(v2(ua, va), PI).droit(ua - (ul + r)).tourne(FRAC_PI_2).troncons;
    let sortie = Trace::depuis(v2(ul, vm - lr), 3.0 * FRAC_PI_2)
        .droit(vm - lr - (vb + r))
        .tourne(FRAC_PI_2)
        .droit(ua - (ul + r))
        .troncons;
    Ok(Esquisse {
        q: rect(c.x0 - wr, vm - lr, c.x0, vm),
        bouche: v2(ul, vm),
        bout: v2(ul, vm - lr),
        entree,
        sortie,
        zone: rect(c.x0 - wr, vb - mg, c.x0, va + mg),
    })
}

/// Construit le plateau `larg` × `l` (dimensions utiles, voir `compte`). En implantation
/// automatique, garde celle qui laisse le plus de places par niveau courant.
pub fn construire(g: &Gabarit, larg: f64, l: f64) -> Option<Plan> {
    if g.rampe.is_some() && g.implantation == Implantation::Auto {
        let (mut best, mut refus): (Option<Plan>, Vec<String>) = (None, Vec::new());
        for implantation in [Implantation::Rangee, Implantation::Pignon] {
            let Some(p) = construire(&Gabarit { implantation, ..*g }, larg, l) else { continue };
            refus.extend(p.refus.iter().cloned());
            let cle = |p: &Plan| (!p.rampes.is_empty(), p.places.len());
            if best.as_ref().is_none_or(|b| cle(&p) > cle(b)) {
                best = Some(p);
            }
        }
        return best.map(|mut b| {
            b.refus = if b.rampes.is_empty() { refus } else { Vec::new() };
            b
        });
    }
    let t = travers(g, larg)?;
    let lo = long(g, l)?;
    let (m, w, d) = (g.module(), g.w, g.d);
    let (debut, n) = (lo.debut, lo.par_rangee as usize);
    let fin = debut + n as f64 * w;
    let mut places = Vec::new();
    let mut allees = Vec::new();

    // Rangées de modules : rangée A (fond côté v bas), rangée B (fond côté v haut).
    let rangee = |places: &mut Vec<PlaceLoc>, fond: f64, sens: f64, nom: &str| {
        for j in 0..n {
            let (ua, ub) = (debut + j as f64 * w, debut + (j + 1) as f64 * w);
            let av = fond + sens * d;
            places.push(PlaceLoc {
                q: [v2(ua, fond), v2(ub, fond), v2(ub, av), v2(ua, av)],
                cle: format!("{nom}/{}", j + 1),
                rive: false,
            });
        }
    };
    for k in 0..t.modules {
        let vb = g.facade + k as f64 * m;
        rangee(&mut places, vb, 1.0, &format!("M{}A", k + 1));
        rangee(&mut places, vb + m, -1.0, &format!("M{}B", k + 1));
        allees.push((rect(debut, vb + d, fin, vb + d + g.allee), format!("AL/M{}", k + 1)));
    }
    if t.demi {
        let vb = g.facade + t.modules as f64 * m;
        rangee(&mut places, vb, 1.0, "D");
        allees.push((rect(debut, vb + d, fin, vb + d + g.allee), "AL/D".into()));
    }
    // Allées transversales aux deux bouts, sur toute la largeur intérieure.
    let (v0, v1) = (g.facade, t.largeur - g.facade);
    allees.push((rect(debut - g.allee, v0, debut, v1), "AL/X1".into()));
    allees.push((rect(fin, v0, fin + g.allee, v1), "AL/X2".into()));

    // Files longitudinales.
    let axes_v = lignes_v(g, &t);

    // Rampes droites (plusieurs niveaux), à sens unique : montée depuis le premier bout, descente
    // depuis le second, symétrique par rapport au centre du plateau. Elles se superposent d'un
    // niveau à l'autre ; leurs épures de giration d'entrée et de sortie (NF P 91-100, § 3.3).
    let (mut rampes, mut girations, mut zones, mut refus) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut implantation = None;
    if let Some(lr) = g.rampe {
        let imp = if g.implantation == Implantation::Pignon { Implantation::Pignon } else { Implantation::Rangee };
        let essai: Result<Vec<Esquisse>, String> = cadres(g, &t, &lo)
            .iter()
            .map(|c| if imp == Implantation::Pignon { en_pignon(c, g, lr) } else { en_rangee(c, g, lr) })
            .collect();
        match essai {
            Ok(es) => {
                implantation = Some(imp);
                for (i, e) in es.into_iter().enumerate() {
                    // Descente : demi-tour autour du centre du plateau.
                    let p = |q: V2| if i == 1 { v2(lo.longueur - q.x, t.largeur - q.y) } else { q };
                    let montee = i == 0;
                    let cle = format!("RA/{}", i + 1);
                    let (bouche, bout) = (p(e.bouche), p(e.bout));
                    rampes.push(RampeLoc {
                        q: e.q.map(p),
                        bas: if montee { bouche } else { bout },
                        haut: if montee { bout } else { bouche },
                        montee,
                        cle: cle.clone(),
                    });
                    for (entree, tr) in [(true, &e.entree), (false, &e.sortie)] {
                        girations.push(Giration {
                            cle: format!("GI/{}/{}", i + 1, if entree { "E" } else { "S" }),
                            rampe: cle.clone(),
                            montee,
                            entree,
                            axe: giration::axe(tr).into_iter().map(p).collect(),
                            contour: giration::contour(tr).into_iter().map(p).collect(),
                            conflits: Vec::new(),
                        });
                    }
                    zones.push(e.zone.map(p));
                }
            }
            Err(m) => refus.push(m),
        }
    }

    // Places de rive : fond contre la façade d'extrémité, entre deux files de poteaux.
    if g.rives {
        let n_win = axes_v.len() - 1;
        for (b, (fond, sens)) in [(g.facade, 1.0), (lo.longueur - g.facade, -1.0)].into_iter().enumerate() {
            for (s, pair) in axes_v.windows(2).enumerate() {
                let k = ((pair[1] - pair[0] + 1e-9) / w).floor() as usize;
                let v0 = debut_rive(pair[0], pair[1], k, w, s == 0, s + 1 == n_win);
                for j in 0..k {
                    let (va, vb) = (v0 + j as f64 * w, v0 + (j + 1) as f64 * w);
                    let av = fond + sens * d;
                    places.push(PlaceLoc {
                        q: [v2(fond, va), v2(fond, vb), v2(av, vb), v2(av, va)],
                        cle: format!("R{}/{}/{}", b + 1, lettre(s), j + 1),
                        rive: true,
                    });
                }
            }
        }
    }

    // Places sous une rampe : à aucun niveau (au dernier, trémie de la montée). Dans une épure
    // d'extrémité basse : rendues au dernier niveau ; d'extrémité haute : rendues au rez.
    let boites: Vec<(V2, V2)> = girations.iter().map(|gi| giration::bornes(&gi.contour)).collect();
    let dans = |gi: &Giration, b: &(V2, V2), q: &[V2]| {
        let (lo_, hi_) = giration::bornes(q);
        b.0.x < hi_.x && b.1.x > lo_.x && b.0.y < hi_.y && b.1.y > lo_.y && giration::touche(&gi.contour, lo_, hi_)
    };
    let (mut places_rdc, mut places_dernier) = (Vec::new(), Vec::new());
    let toutes = std::mem::take(&mut places);
    for pl in toutes {
        if rampes.iter().any(|r| chevauche(&pl.q, &r.q)) {
            continue;
        }
        let (mut bas, mut haut) = (false, false);
        for (gi, b) in girations.iter().zip(&boites) {
            if dans(gi, b, &pl.q) {
                if gi.bas() {
                    bas = true;
                } else {
                    haut = true;
                }
            }
        }
        match (bas, haut) {
            (false, false) => places.push(pl),
            (true, false) => places_dernier.push(pl),
            (false, true) => places_rdc.push(pl),
            (true, true) => {}
        }
    }

    // Files transversales : façades d'extrémité, tête des places de rive (coupe la portée entre la
    // façade et le début des rangées), début et fin des rangées (poteaux au nu de l'allée
    // transversale), puis des travées réparties également : `travee` places au plus, des tailles
    // qui diffèrent d'une place au plus, chaque file sur une séparation de places.
    let tete = g.facade + d - g.retrait - g.pu / 2.0;
    let mut axes_u = vec![g.facade + g.pu / 2.0];
    if g.rives {
        axes_u.push(tete);
    }
    axes_u.push(debut + g.pu / 2.0);
    let nb = n.div_ceil(g.travee.max(1) as usize);
    for i in 1..nb {
        axes_u.push(debut + (i as f64 * n as f64 / nb as f64).round() * w);
    }
    axes_u.push(fin - g.pu / 2.0);
    if g.rives {
        axes_u.push(lo.longueur - tete);
    }
    axes_u.push(lo.longueur - g.facade - g.pu / 2.0);

    let mut poteaux = Vec::new();
    for (i, &u) in axes_u.iter().enumerate() {
        for (k, &v) in axes_v.iter().enumerate() {
            let c = v2(u, v);
            let q = rect(c.x - g.pu / 2.0, c.y - g.pv / 2.0, c.x + g.pu / 2.0, c.y + g.pv / 2.0);
            // Rien dans l'emprise d'une rampe ni à ses abords.
            if rampes.iter().any(|r| chevauche(&q, &r.q)) || zones.iter().any(|z| chevauche(&q, z)) {
                continue;
            }
            poteaux.push(PoteauLoc {
                c,
                cle: format!("P/{}{}", lettre(k), i + 1),
            });
        }
    }
    // Contrôle des épures : poteaux restants, autre rampe, façades (nu intérieur).
    let (h_u, h_v) = (g.pu / 2.0, g.pv / 2.0);
    let (umax, vmax) = (lo.longueur - g.facade, t.largeur - g.facade);
    for (gi, b) in girations.iter_mut().zip(&boites) {
        let mut conflits: Vec<String> = Vec::new();
        for po in &poteaux {
            let q = rect(po.c.x - h_u, po.c.y - h_v, po.c.x + h_u, po.c.y + h_v);
            if dans(gi, b, &q) {
                conflits.push(po.cle.clone());
            }
        }
        for r in rampes.iter().filter(|r| r.cle != gi.rampe) {
            if dans(gi, b, &r.q) {
                conflits.push(r.cle.clone());
            }
        }
        let e = 1e-6;
        if gi.contour.iter().any(|p| p.x < g.facade - e || p.x > umax + e || p.y < g.facade - e || p.y > vmax + e) {
            conflits.push("façade".into());
        }
        gi.conflits = conflits;
    }
    Some(Plan {
        travers: t,
        long: lo,
        places,
        places_rdc,
        places_dernier,
        allees,
        rampes,
        implantation,
        girations,
        refus,
        poteaux,
        axes_u,
        axes_v,
    })
}

/// Plus grands entraxes des files transversales : le long des rangées, puis dans les zones
/// d'extrémité (places de pignon et allée transversale).
pub fn travees(plan: &Plan) -> (f64, f64) {
    let (d0, d1) = (plan.long.debut, plan.long.longueur - plan.long.debut);
    let dans = |u: f64| u > d0 - 1e-6 && u < d1 + 1e-6;
    let (mut r, mut b) = (0.0f64, 0.0f64);
    for p in plan.axes_u.windows(2) {
        if dans(p[0]) && dans(p[1]) {
            r = r.max(p[1] - p[0]);
        } else {
            b = b.max(p[1] - p[0]);
        }
    }
    (r, b)
}

/// Poutres ou solives de poteau à poteau, en repère local, largeur `b` rognée au contour de la
/// dalle : sur chaque file transversale entre deux files longitudinales voisines (en travers), ou
/// sur chaque file longitudinale entre deux files transversales voisines (le long des allées).
/// Clé : `PO/{file}/{de}-{à}`.
pub fn ossature(plan: &Plan, sens: Sens, b: f64) -> Vec<([V2; 4], String)> {
    let (larg, long) = (plan.largeur(), plan.longueur());
    let mut out = Vec::new();
    match sens {
        Sens::Travers => {
            for (i, &u) in plan.axes_u.iter().enumerate() {
                let (u0, u1) = ((u - b / 2.0).max(0.0), (u + b / 2.0).min(long));
                for (k, p) in plan.axes_v.windows(2).enumerate() {
                    out.push((rect(u0, p[0], u1, p[1]), format!("PO/{}/{}-{}", i + 1, lettre(k), lettre(k + 1))));
                }
            }
        }
        Sens::Long => {
            for (k, &v) in plan.axes_v.iter().enumerate() {
                let (v0, v1) = ((v - b / 2.0).max(0.0), (v + b / 2.0).min(larg));
                for (i, p) in plan.axes_u.windows(2).enumerate() {
                    out.push((rect(p[0], v0, p[1], v1), format!("PO/{}/{}-{}", lettre(k), i + 1, i + 2)));
                }
            }
        }
    }
    // Rien dans l'emprise d'une rampe.
    out.retain(|(q, _)| !plan.rampes.iter().any(|r| chevauche(q, &r.q)));
    out
}

/// Emprise d'un plateau dans le repère du plan : origine (coin de dalle), direction des allées u,
/// direction en travers v, largeur (selon v) et longueur (selon u).
#[derive(Clone, Copy, Debug)]
pub struct Emprise {
    pub o: V2,
    pub u: V2,
    pub v: V2,
    pub largeur: f64,
    pub longueur: f64,
}

impl Emprise {
    pub fn monde(&self, p: V2) -> V2 {
        self.o + self.u * p.x + self.v * p.y
    }
}

/// Cherche, parmi les orientations `angles` (degrés), le plateau qui contient le plus de places
/// dans la région `rings` (pair-impair). Trame de recherche au pas `pas` (m). Le rectangle retenu
/// est réduit à ses dimensions utiles et centré dans le plus grand rectangle libre trouvé.
/// `sens_impose` : les allées suivent exactement l'angle donné.
pub fn chercher(
    g: &Gabarit,
    rings: &[Vec<V2>],
    angles: &[f64],
    pas: f64,
    sens_impose: bool,
) -> Option<(Emprise, usize)> {
    // (rampes placées, places, surface utile, emprise)
    let mut best: Option<(bool, usize, f64, Emprise)> = None;
    let mut memo = std::collections::HashMap::new();
    let densite = 2.0 / (g.w * g.module());
    let mut iv: Vec<(f64, f64)> = Vec::new();
    for &deg in angles {
        let (c, s) = (deg.to_radians().cos(), deg.to_radians().sin());
        let rr = rotate_rings(rings, c, -s);
        let (mut lo, mut hi) = (v2(f64::MAX, f64::MAX), v2(f64::MIN, f64::MIN));
        for p in rr.iter().flatten() {
            lo = v2(lo.x.min(p.x), lo.y.min(p.y));
            hi = v2(hi.x.max(p.x), hi.y.max(p.y));
        }
        if hi.x <= lo.x || hi.y <= lo.y {
            continue;
        }
        let nx = ((hi.x - lo.x) / pas).ceil() as usize;
        let ny = ((hi.y - lo.y) / pas).ceil() as usize;
        let mut haut = vec![0u32; nx];
        let mut pile: Vec<(usize, u32)> = Vec::with_capacity(nx);
        let mut ligne = vec![false; nx];
        for j in 0..ny {
            let (ya, yb) = (lo.y + j as f64 * pas, lo.y + (j + 1) as f64 * pas);
            strip_intervals(&rr, ya, yb, &mut iv);
            ligne.iter_mut().for_each(|x| *x = false);
            for &(a, b) in &iv {
                let i0 = (((a - lo.x) / pas) - 1e-9).ceil().max(0.0) as usize;
                let i1 = ((((b - lo.x) / pas) + 1e-9).floor() as usize).min(nx);
                for x in ligne.iter_mut().take(i1).skip(i0) {
                    *x = true;
                }
            }
            for i in 0..nx {
                haut[i] = if ligne[i] { haut[i] + 1 } else { 0 };
            }
            // Rectangles maximaux dont le bas est la rangée j (histogramme et pile).
            pile.clear();
            for i in 0..=nx {
                let h = if i < nx { haut[i] } else { 0 };
                let mut debut = i;
                while let Some(&(s0, h0)) = pile.last() {
                    if h0 <= h {
                        break;
                    }
                    pile.pop();
                    let (wx, hy) = ((i - s0) as f64 * pas, h0 as f64 * pas);
                    let borne = (wx * hy * densite) as usize + 1;
                    if best.map_or(true, |b| !b.0 || borne >= b.1) {
                        let cx = lo.x + (s0 as f64 + (i - s0) as f64 / 2.0) * pas;
                        let cy = lo.y + (j as f64 + 1.0 - h0 as f64 / 2.0) * pas;
                        let opts: &[(f64, f64, bool)] = if sens_impose {
                            &[(hy, wx, true)]
                        } else {
                            &[(hy, wx, true), (wx, hy, false)]
                        };
                        for &(larg, l, le_long_de_x) in opts {
                            let Some((ok, n, t, lg)) = evalue(g, larg, l, &mut memo) else { continue };
                            let aire = t.largeur * lg.longueur;
                            if best.map_or(true, |b| (ok, n) > (b.0, b.1) || (ok == b.0 && n == b.1 && aire < b.2 - 1e-6)) {
                                // Repère : u le long des allées, v à sa gauche ; rotation inverse vers le plan.
                                let (ul, vl) = if le_long_de_x {
                                    (v2(1.0, 0.0), v2(0.0, 1.0))
                                } else {
                                    (v2(0.0, 1.0), v2(-1.0, 0.0))
                                };
                                let (u, v) = (ul.rot(c, s), vl.rot(c, s));
                                let centre = v2(cx, cy).rot(c, s);
                                let o = centre - u * (lg.longueur / 2.0) - v * (t.largeur / 2.0);
                                best = Some((
                                    ok,
                                    n,
                                    aire,
                                    Emprise {
                                        o,
                                        u,
                                        v,
                                        largeur: t.largeur,
                                        longueur: lg.longueur,
                                    },
                                ));
                            }
                        }
                    }
                    debut = s0;
                }
                if h > 0 && pile.last().map_or(true, |&(_, h0)| h0 < h) {
                    pile.push((debut, h));
                }
            }
        }
    }
    best.map(|(_, n, _, e)| (e, n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gab() -> Gabarit {
        Gabarit {
            w: 2.5,
            d: 5.0,
            allee: 6.0,
            facade: 0.2,
            pu: 0.3,
            pv: 0.6,
            travee: 3,
            rives: true,
            densite: Densite::Longue,
            retrait: 0.65,
            rampe: None,
            rampe_largeur: 3.25,
            implantation: Implantation::Auto,
        }
    }

    #[test]
    fn calcul_a_la_main() {
        // Plateau 32,40 × 60 m, places 2,50 × 5,00, allées 6,00, façades 0,20 :
        // - en travers, 32,00 m utiles = 2 modules de 16,00 m ;
        // - en long, rangées de 60 − 2 × (0,20 + 5,00 + 6,00) = 37,60 m → 15 places de 2,50 m ;
        // - 4 rangées × 15 = 60 places ;
        // - rives : files de poteaux à 0,50 / 16,20 / 31,90 m → 2 travées de 15,70 m → 6 + 6 places,
        //   soit 12 par façade d'extrémité et 24 en tout ;
        // - total 84 places, plateau utile 32,40 × 59,90 m.
        let g = gab();
        let (n, t, lo) = compte(&g, 32.4, 60.0).unwrap();
        assert_eq!((t.modules, t.demi), (2, false));
        assert_eq!(lo.par_rangee, 15);
        assert!((t.largeur - 32.4).abs() < 1e-9 && (lo.longueur - 59.9).abs() < 1e-9);
        assert_eq!(n, 84);
        let p = construire(&g, 32.4, 60.0).unwrap();
        assert_eq!(p.places.len(), 84);
        assert_eq!(p.places.iter().filter(|q| q.rive).count(), 24);
        assert_eq!(p.axes_v.len(), 3);
        // Files transversales : 2 façades + 2 têtes de rive + début + 4 intermédiaires (pas de
        // 7,50 m) + fin.
        assert_eq!(p.axes_u.len(), 2 + 2 + 2 + 4);
        assert_eq!(p.poteaux.len(), 10 * 3);
    }

    #[test]
    fn demi_module_et_sans_rives() {
        let mut g = gab();
        g.rives = false;
        // 0,20 + 16,00 + (5,00 + 6,00 + 0,60) + 0,20 = 28,00 m : un module et un demi-module.
        let (n, t, lo) = compte(&g, 28.0, 40.0).unwrap();
        assert_eq!((t.modules, t.demi), (1, true));
        // Sans rives : rangées de 40 − 2 × (0,20 + 0,30 + 6,00) = 27,00 m → 10 places, 3 rangées.
        assert_eq!(lo.par_rangee, 10);
        assert_eq!(n, 30);
        assert_eq!(construire(&g, 28.0, 40.0).unwrap().places.len(), 30);
        // Trop étroit pour une rangée et son allée.
        assert!(compte(&g, 11.0, 40.0).is_none());
    }

    #[test]
    fn compte_egale_construction() {
        let g = gab();
        for larg in [16.4, 17.0, 23.0, 28.0, 32.4, 40.0, 49.0, 64.8] {
            for l in [30.0, 41.3, 60.0, 77.7] {
                if let Some((n, _, _)) = compte(&g, larg, l) {
                    assert_eq!(construire(&g, larg, l).unwrap().places.len(), n, "{larg} × {l}");
                }
            }
        }
    }

    #[test]
    fn poteaux_conformes() {
        // Aucun poteau dans une allée. Tout poteau qui touche une place y laisse au moins 2,00 m de
        // largeur et se trouve soit dans le premier 1,10 m du fond (NF P 91-100, fig. 2), soit en
        // tête de place, le nu à 0,65 m au moins de l'allée (tab. 3) et le poteau entier dans les
        // 1,25 m côté allée (Neufert p. 274).
        for densite in [Densite::Longue, Densite::Moyenne, Densite::Courte] {
            let g = Gabarit { densite, ..gab() };
            for (larg, l) in [(32.4, 60.0), (28.0, 41.3), (49.0, 77.7), (12.0, 30.0)] {
                let p = construire(&g, larg, l).unwrap();
                for po in &p.poteaux {
                    let (a, b) = (po.c - v2(g.pu / 2.0, g.pv / 2.0), po.c + v2(g.pu / 2.0, g.pv / 2.0));
                    let inter = |q: &[V2; 4]| {
                        let (qa, qb) = (
                            v2(q.iter().map(|p| p.x).fold(f64::MAX, f64::min), q.iter().map(|p| p.y).fold(f64::MAX, f64::min)),
                            v2(q.iter().map(|p| p.x).fold(f64::MIN, f64::max), q.iter().map(|p| p.y).fold(f64::MIN, f64::max)),
                        );
                        let (ix, iy) = ((b.x.min(qb.x) - a.x.max(qa.x)), (b.y.min(qb.y) - a.y.max(qa.y)));
                        (ix > 1e-6 && iy > 1e-6).then_some((ix, iy))
                    };
                    for (q, cle) in &p.allees {
                        assert!(inter(q).is_none(), "{densite:?} {larg}×{l} : poteau {} dans {cle}", po.cle);
                    }
                    for pl in &p.places {
                        if inter(&pl.q).is_none() {
                            continue;
                        }
                        let (ix, iy) = inter(&pl.q).unwrap();
                        // Axe de la place : du fond (q[0]) vers l'avant (q[3]).
                        let fond_u = (pl.q[0].x - pl.q[1].x).abs() < 1e-9;
                        let (f, av, ca, cb, reste) = if fond_u {
                            (pl.q[0].x, pl.q[3].x, a.x, b.x, g.w - iy)
                        } else {
                            (pl.q[0].y, pl.q[3].y, a.y, b.y, g.w - ix)
                        };
                        let s = (av - f).signum();
                        let (x, y) = ((ca - f) * s, (cb - f) * s);
                        // Faces du poteau comptées depuis le fond, ramenées à la place.
                        let (t0, t1) = (x.min(y).max(0.0), x.max(y).min(g.d));
                        let au_fond = t1 <= 1.10 + 1e-9;
                        let en_tete = g.d - t1 >= 0.65 - 1e-9 && g.d - t0 <= 1.25 + 1e-9;
                        assert!(
                            reste >= 2.0 - 1e-9 && (au_fond || en_tete),
                            "{densite:?} {larg}×{l} : {} dans {} (faces à {t0:.2} et {t1:.2} du fond)",
                            po.cle,
                            pl.cle
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn rampes_le_long_des_allees() {
        // 32,40 × 60 m (2 modules, 15 places par rangée, rangées de 11,20 à 48,70), rampe de
        // 2,90 / 0,18 + (10 + 15) × 0,18 / 2 = 18,36 m, voie de 3,25 m, giration au rayon 6,50.
        // Montée dans la rangée M1A, voie v 1,95 → 5,20 (axe 3,575) contre l'allée M1. Entrée :
        // depuis l'allée M2 (trajectoire à 25,20, 2,00 m sous son bord haut), quart de tour dans
        // l'allée transversale (trajectoire à u 7,20), descente jusqu'à 3,575 + 6,50, quart de tour
        // vers +u : bouche à 7,20 + 6,50 = 13,70. Rampe 13,70 → 32,06. Sortie en S jusqu'à 6,70
        // (1,50 dans l'allée) : écart 3,125, angle acos(1 − 3,125 / 13) = 40,5°, longueur
        // 13 sin 40,5° = 8,46, fin à 40,52 ≤ 48,70.
        // Places de M1A (11,20 + 2,50 j) : j = 1 à 8 sous la rampe ; j = 0 dans l'épure d'entrée
        // (rendue au dernier niveau) ; j = 9, 10, 11 dans le S de sortie (rendues au rez).
        // Descente symétrique dans M2B : 8 sous la rampe, 1 rendue au rez, 3 au dernier.
        // Niveau courant : 84 − 2 × (8 + 1 + 3) = 60 ; rez et dernier : 64.
        let lr = crate::ouvrage::normes::longueur_rampe(2.9, 0.18, 10.0, 15.0);
        assert!((lr - 18.3611).abs() < 1e-3, "{lr}");
        let g = Gabarit { rampe: Some(lr), ..gab() };
        let p = construire(&g, 32.4, 60.0).unwrap();
        assert_eq!(p.implantation, Some(Implantation::Rangee));
        assert_eq!(p.rampes.len(), 2);
        let r = &p.rampes[0];
        let (lo, hi) = giration::bornes(&r.q);
        assert!((lo - v2(13.7, 1.95)).len() < 1e-9 && (hi - v2(13.7 + lr, 5.2)).len() < 1e-9, "{lo:?} {hi:?}");
        let r2 = &p.rampes[1];
        let (lo, hi) = giration::bornes(&r2.q);
        assert!((lo - v2(59.9 - 13.7 - lr, 27.2)).len() < 1e-9 && (hi - v2(46.2, 30.45)).len() < 1e-9, "{lo:?} {hi:?}");
        assert!((r2.haut - v2(46.2, 28.825)).len() < 1e-9 && !r2.montee);
        assert_eq!((p.places.len(), p.places_rdc.len(), p.places_dernier.len()), (60, 4, 4));
        let cles = |v: &[PlaceLoc]| {
            let mut c: Vec<String> = v.iter().map(|pl| pl.cle.clone()).collect();
            c.sort();
            c
        };
        assert_eq!(cles(&p.places_rdc), ["M1A/10", "M1A/11", "M1A/12", "M2B/15"]);
        assert_eq!(cles(&p.places_dernier), ["M1A/1", "M2B/4", "M2B/5", "M2B/6"]);
        // Fin du S de sortie de la montée, puis 3,00 m d'amorce dans l'allée.
        let s = p.girations.iter().find(|gi| gi.montee && !gi.entree).unwrap();
        let fin = *s.axe.last().unwrap();
        assert!((fin - v2(13.7 + lr + 8.4558 + 3.0, 6.7)).len() < 1e-3, "{fin:?}");
        // Aucun poteau retiré (ils sont en façade et en dos à dos), aucune épure gênée, et l'entrée
        // frôle sans les toucher les places de pignon et la rangée M2B.
        assert_eq!(p.poteaux.len(), 30);
        assert_eq!(p.girations.len(), 4);
        assert!(p.girations.iter().all(|gi| gi.conflits.is_empty()), "{:?}", p.girations.iter().map(|gi| &gi.conflits).collect::<Vec<_>>());
        assert_eq!(p.places.iter().filter(|q| q.rive).count(), 24);
        assert_eq!(compte(&g, 32.4, 60.0).unwrap().0, 60);
        // Rien dans les rampes : places, poteaux, poutres.
        let q = |c: V2| rect(c.x - g.pu / 2.0, c.y - g.pv / 2.0, c.x + g.pu / 2.0, c.y + g.pv / 2.0);
        for r in &p.rampes {
            assert!(!p.places.iter().chain(&p.places_rdc).chain(&p.places_dernier).any(|pl| chevauche(&pl.q, &r.q)));
            assert!(!p.poteaux.iter().any(|po| chevauche(&q(po.c), &r.q)));
            assert!(!ossature(&p, Sens::Long, 1.1).iter().any(|(o, _)| chevauche(o, &r.q)));
        }
        // Rangées trop courtes : 40,52 − 11,20 = 29,32 m de rangée nécessaires.
        let p = construire(&g, 32.4, 50.0).unwrap();
        assert!(p.rampes.is_empty() && p.refus.iter().any(|m| m.contains("29,32 m de rangée")), "{:?}", p.refus);
        // Un module : pas d'autre allée pour aborder la rampe, ni de place en pignon.
        let p = construire(&g, 16.4, 60.0).unwrap();
        assert!(p.rampes.is_empty() && p.refus.len() == 2, "{:?}", p.refus);
    }

    #[test]
    fn rampes_en_pignon() {
        // 2 modules : trajectoires dans les allées extrêmes à 7,20 et 25,20, soit 18,00 m pour
        // 18,36 + 2 × 6,50 = 31,36 m : impossible.
        let lr = crate::ouvrage::normes::longueur_rampe(2.9, 0.18, 10.0, 15.0);
        let g = Gabarit { rampe: Some(lr), implantation: Implantation::Pignon, ..gab() };
        let p = construire(&g, 32.4, 60.0).unwrap();
        assert!(p.rampes.is_empty() && p.refus[0].contains("31,36 m nécessaires") && p.refus[0].contains("18,00 m disponibles"), "{:?}", p.refus);
        // 3 modules (48,40 m) : 41,20 − 7,20 = 34,00 m. Montée en pignon 1, voie u 1,95 → 5,20
        // (axe 3,575) : entrée depuis l'allée M3, bouche à 41,20 − 6,50 = 34,70, rampe jusqu'à
        // 16,34, 2,64 m droits, quart de tour vers l'allée M1 (trajectoire 7,20).
        // Places de pignon 1 (fenêtres 1,20 → 16,20, 16,70 → 31,70, 32,20 → 47,20) : sous la rampe
        // les 6 du milieu et 32,20 → 34,70 (7) ; dans l'épure d'entrée 34,70 → 42,20 (3, rendues au
        // dernier niveau) ; dans l'épure de sortie 6,20 → 16,20 (4, rendues au rez).
        // Poteaux : 40 moins les 2 de la file de tête de pignon dans chaque voie (16,20 et 32,20).
        // Niveau courant : 126 − 2 × (7 + 3 + 4) = 98 ; rez et dernier : 105.
        let p = construire(&g, 48.4, 60.0).unwrap();
        assert_eq!(p.implantation, Some(Implantation::Pignon));
        let (lo, hi) = giration::bornes(&p.rampes[0].q);
        assert!((lo - v2(1.95, 34.7 - lr)).len() < 1e-9 && (hi - v2(5.2, 34.7)).len() < 1e-9, "{lo:?} {hi:?}");
        assert_eq!((p.places.len(), p.places_rdc.len(), p.places_dernier.len()), (98, 7, 7));
        assert_eq!(p.poteaux.len(), 36);
        assert!(p.girations.iter().all(|gi| gi.conflits.is_empty()), "{:?}", p.girations.iter().map(|gi| &gi.conflits).collect::<Vec<_>>());
        // Le long des allées sur la même dalle : 126 − 2 × 12 = 102 places ; l'automatique la garde.
        let auto = construire(&Gabarit { implantation: Implantation::Auto, ..g }, 48.4, 60.0).unwrap();
        assert_eq!((auto.implantation, auto.places.len()), (Some(Implantation::Rangee), 102));
    }

    #[test]
    fn epures_genees() {
        // Allées de 3,60 m (2 modules, 27,60 m) : la giration d'entrée de la montée quitte l'allée
        // M2 (trajectoire à 20,40, centre de la courbe en (13,70 ; 13,90)) et balaie le poteau du
        // dos à dos en tête de rangée (u 8,80 → 9,10, v 13,50 → 14,10, à 4,6–4,9 m du centre).
        let lr = crate::ouvrage::normes::longueur_rampe(2.9, 0.18, 10.0, 15.0);
        let g = Gabarit { rampe: Some(lr), allee: 3.6, implantation: Implantation::Rangee, ..gab() };
        let p = construire(&g, 27.6, 60.0).unwrap();
        assert_eq!(p.rampes.len(), 2);
        let e = p.girations.iter().find(|gi| gi.montee && gi.entree).unwrap();
        assert_eq!(e.conflits, ["P/B3"]);
        // Allées de 6,00 m : rien.
        let p = construire(&Gabarit { allee: 6.0, ..g }, 32.4, 60.0).unwrap();
        assert!(p.girations.iter().all(|gi| gi.conflits.is_empty()));
    }

    #[test]
    fn ossature_a_la_main() {
        // 32,40 × 60 m, 15 places par rangée.
        // Moyenne, poutres en travers : 10 files transversales × 3 intervalles (4 files A à D) = 30.
        let p = construire(&Gabarit { densite: Densite::Moyenne, ..gab() }, 32.4, 60.0).unwrap();
        let o = ossature(&p, Sens::Travers, 0.32);
        assert_eq!(o.len(), 30);
        assert_eq!(o[0].1, "PO/1/A-B");
        // Courte, poutres le long des allées : 6 files longitudinales × 9 intervalles = 54 ; la
        // poutre de façade (axe à 0,50, largeur 1,10) est rognée au bord de dalle.
        let p = construire(&Gabarit { densite: Densite::Courte, ..gab() }, 32.4, 60.0).unwrap();
        let o = ossature(&p, Sens::Long, 1.10);
        assert_eq!(o.len(), 54);
        assert!(o[0].0[0].y.abs() < 1e-9 && (o[0].0[2].y - 1.05).abs() < 1e-9);
        // Longue, 2 places au plus entre poteaux : files transversales à round(15 i / 8) = 2, 4, 6,
        // 8, 9, 11, 13 places du début → 13 files (2 façades, 2 têtes de pignon, début, 7, fin) ;
        // 13 × 2 intervalles = 26 solives. Entraxes : rangées 5,00 m au plus (4,85 aux bouts des
        // rangées), extrémités 6,95 m (de 4,40 à 11,35).
        let p = construire(&Gabarit { travee: 2, ..gab() }, 32.4, 60.0).unwrap();
        assert_eq!(ossature(&p, Sens::Travers, 0.21).len(), 26);
        let (r, b) = travees(&p);
        assert!((r - 5.0).abs() < 1e-9 && (b - 6.95).abs() < 1e-9, "{r} {b}");
    }

    #[test]
    fn travees_reparties() {
        // 32,40 × 62,40 m : rangées de (62,40 − 2 × 11,20) / 2,50 = 16 places. Au plus 3 places
        // par travée → 6 travées ; files à round(16 i / 6) = 3, 5, 8, 11, 13 places du début,
        // soit des travées de 3, 2, 3, 3, 2, 3 places (au lieu de 3, 3, 3, 3, 4).
        // Au plus 4 places, 15 places → 4 travées : round(15 i / 4) = 4, 8, 11 → 4, 4, 3, 4.
        for (travee, l, attendu) in [(3, 62.4, vec![3.0, 5.0, 8.0, 11.0, 13.0]), (4, 60.0, vec![4.0, 8.0, 11.0])] {
            let g = Gabarit { travee, ..gab() };
            let p = construire(&g, 32.4, l).unwrap();
            let debut = p.long.debut;
            // Files intérieures : entre la file de début (debut + pu/2) et celle de fin.
            let i0 = p.axes_u.iter().position(|&u| (u - (debut + g.pu / 2.0)).abs() < 1e-9).unwrap();
            let k: Vec<f64> = p.axes_u[i0 + 1..i0 + 1 + attendu.len()].iter().map(|u| (u - debut) / g.w).collect();
            assert_eq!(k.len(), attendu.len());
            for (a, b) in k.iter().zip(&attendu) {
                assert!((a - b).abs() < 1e-9, "travée {travee} : {k:?}, attendu {attendu:?}");
            }
            let fin = (p.axes_u[i0 + 1 + attendu.len()] + g.pu / 2.0 - debut) / g.w;
            assert!((fin - p.long.par_rangee as f64).abs() < 1e-9);
        }
    }

    #[test]
    fn densites_a_la_main() {
        // Plateau 32,40 × 60 m (2 modules), poteaux 0,30 × 0,60, retrait 0,65 : axe d'un poteau en
        // tête de rangée à 0,65 + 0,30 = 0,95 m de l'allée.
        // - longue : files 0,50 / 16,20 / 31,90, portées 15,70 ; rives 6 + 6 par façade ;
        // - moyenne : 0,50 / 12,15 / 20,25 / 31,90 (16,20 ∓ 4,05), portées 11,65 / 8,10 / 11,65 ;
        //   rives 4 + 3 + 4 = 11 par façade ;
        // - courte : 0,50 / 4,25 / 12,15 / 20,25 / 28,15 / 31,90, portées 3,75 / 7,90 / 8,10 / 7,90 /
        //   3,75 ; rives 1 + 3 + 3 + 3 + 1 = 11 par façade.
        // 60 places de modules dans les trois cas ; files transversales : 2 façades + 2 têtes de rive
        // (0,20 + 5,00 − 0,65 − 0,15 = 4,40 m) + début + 4 intermédiaires + fin = 10.
        let cas = [
            (Densite::Longue, vec![0.5, 16.2, 31.9], 84),
            (Densite::Moyenne, vec![0.5, 12.15, 20.25, 31.9], 82),
            (Densite::Courte, vec![0.5, 4.25, 12.15, 20.25, 28.15, 31.9], 82),
        ];
        for (densite, files, n) in cas {
            let g = Gabarit { densite, ..gab() };
            let p = construire(&g, 32.4, 60.0).unwrap();
            assert_eq!(p.axes_v.len(), files.len(), "{densite:?}");
            for (a, b) in p.axes_v.iter().zip(&files) {
                assert!((a - b).abs() < 1e-9, "{densite:?} : file à {a}, attendue à {b}");
            }
            assert_eq!(p.places.len(), n, "{densite:?}");
            assert_eq!(compte(&g, 32.4, 60.0).unwrap().0, n, "{densite:?}");
            assert_eq!(p.axes_u.len(), 10, "{densite:?}");
            assert!((p.axes_u[1] - 4.4).abs() < 1e-9 && (p.axes_u[8] - 55.5).abs() < 1e-9);
            assert_eq!(p.poteaux.len(), 10 * files.len(), "{densite:?}");
        }
        // Places de rive en longue : la file centrale tombe sur une séparation de places et les deux
        // restes de 0,70 m partent contre les façades latérales.
        let p = construire(&gab(), 32.4, 60.0).unwrap();
        let mut bords: Vec<f64> = p
            .places
            .iter()
            .filter(|q| q.rive && q.q[0].x < 1.0)
            .flat_map(|q| [q.q[0].y, q.q[1].y])
            .collect();
        bords.sort_by(f64::total_cmp);
        assert!((bords[0] - 1.2).abs() < 1e-9 && (bords[bords.len() - 1] - 31.2).abs() < 1e-9, "{bords:?}");
        assert!(bords.iter().any(|y| (y - 16.2).abs() < 1e-9));
    }

    #[test]
    fn recherche_sur_un_rectangle() {
        // Rectangle libre de 40 × 70 m orienté à 30° : la recherche retrouve l'orientation et le
        // nombre de places du calcul direct sur le plus grand plateau inscrit.
        let g = gab();
        let (c, s) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
        let ring: Vec<V2> = [v2(0.0, 0.0), v2(70.0, 0.0), v2(70.0, 40.0), v2(0.0, 40.0)]
            .iter()
            .map(|p| p.rot(c, s) + v2(100.0, 50.0))
            .collect();
        let angles: Vec<f64> = (0..30).map(|k| k as f64 * 3.0).collect();
        let (e, n) = chercher(&g, &[ring], &angles, 0.5, false).unwrap();
        let direct = compte(&g, 40.0, 70.0).unwrap().0;
        assert_eq!(n, direct);
        let a = e.u.y.atan2(e.u.x).to_degrees().rem_euclid(180.0);
        assert!((a - 30.0).abs() < 1e-6, "angle {a}");
    }

    #[test]
    fn dimensions_les_plus_proches() {
        // Largeurs réalisables : 12,00 (demi-module) ; 16,40 ; 28,00 ; 32,40 ; 44,00 ; 48,40…
        // Longueurs : 2 × 11,20 + k × 2,50.
        let g = gab();
        let cas = [
            ((30.0, 60.0), (28.0, 59.9)),
            ((31.0, 61.3), (32.4, 62.4)),
            ((3.0, 5.0), (12.0, 24.9)),
            ((47.0, 100.0), (48.4, 99.9)),
        ];
        for ((larg, l), (el, ell)) in cas {
            let (a, b) = dimensions_proches(&g, larg, l);
            assert!((a - el).abs() < 1e-9 && (b - ell).abs() < 1e-9, "{larg} × {l} → {a} × {b}");
            // Les dimensions retenues sont exactement celles que le plateau occupe.
            let (_, t, lo) = compte(&g, a, b).expect("plateau réalisable");
            assert!((t.largeur - a).abs() < 1e-9 && (lo.longueur - b).abs() < 1e-9);
        }
    }
}
