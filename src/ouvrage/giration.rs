//! Épures de giration à l'entrée et à la sortie des rampes.
//!
//! Gabarit de la NF P 91-100, § 3.3, voie à sens unique : en courbe, 4,00 m de large et un rayon
//! extérieur d'au moins 8,50 m ; en ligne droite, 3,00 m. Hypothèse : la trajectoire suit le
//! milieu de la voie, soit un rayon de 8,50 − 4,00 / 2 = 6,50 m dans les courbes. L'épure est la
//! bande balayée le long de la trajectoire : 3,00 m de large sur les droites, de 4,50 à 8,50 m du
//! centre sur les courbes.

use super::normes::nf;
use crate::geom::{v2, V2};
use std::f64::consts::PI;

/// Rayon de la trajectoire en courbe (milieu de la voie) : 6,50 m.
pub fn rayon() -> f64 {
    nf::RAYON_EXTERIEUR_MIN.v - nf::VOIE_COURBE_SENS_UNIQUE.v / 2.0
}

/// Écart entre la trajectoire et le bord extérieur de la voie en courbe : 2,00 m.
pub fn marge() -> f64 {
    nf::VOIE_COURBE_SENS_UNIQUE.v / 2.0
}

/// Demi-largeur de la voie droite : 1,50 m.
pub fn demi_droite() -> f64 {
    nf::VOIE_DROITE_SENS_UNIQUE.v / 2.0
}

/// Courbe à double sens (NF P 91-100, § 3.3) : voie intérieure de 3,50 m, rayon extérieur 8,50 m ;
/// voie extérieure de 3,00 m au-delà. Rayons des trajectoires (milieu de chaque voie) et
/// demi-largeurs : (6,75 ; 1,75) à l'intérieur, (10,00 ; 1,50) à l'extérieur.
pub fn double_sens() -> ((f64, f64), (f64, f64)) {
    let (r, vi, ve) = (nf::RAYON_EXTERIEUR_MIN.v, nf::VOIE_COURBE_INTERIEURE.v, nf::VOIE_COURBE_EXTERIEURE.v);
    ((r - vi / 2.0, vi / 2.0), (r + ve / 2.0, ve / 2.0))
}

/// Tronçon de trajectoire et demi-largeur `h` de la voie qu'il balaie.
#[derive(Clone, Debug)]
pub enum Troncon {
    Droite { a: V2, b: V2, h: f64 },
    /// Arc de centre `c` et de rayon `r`, de l'angle `a0` à l'angle `a1` (radians, sens de parcours).
    Arc { c: V2, r: f64, a0: f64, a1: f64, h: f64 },
}

/// Trajectoire en construction : position, cap (radians), tronçons parcourus, et gabarit (rayon
/// des courbes, demi-largeurs en courbe et en ligne droite ; par défaut la voie à sens unique).
#[derive(Clone, Debug)]
pub struct Trace {
    pub p: V2,
    pub cap: f64,
    pub troncons: Vec<Troncon>,
    r: f64,
    h_courbe: f64,
    h_droite: f64,
}

impl Trace {
    pub fn depuis(p: V2, cap: f64) -> Trace {
        Trace { p, cap, troncons: Vec::new(), r: rayon(), h_courbe: marge(), h_droite: demi_droite() }
    }
    /// Gabarit des tronçons suivants.
    pub fn gabarit(mut self, r: f64, h_courbe: f64, h_droite: f64) -> Trace {
        (self.r, self.h_courbe, self.h_droite) = (r, h_courbe, h_droite);
        self
    }
    /// Ligne droite de `l` mètres dans le cap courant.
    pub fn droit(mut self, l: f64) -> Trace {
        if l > 1e-9 {
            let b = self.p + v2(self.cap.cos(), self.cap.sin()) * l;
            self.troncons.push(Troncon::Droite { a: self.p, b, h: self.h_droite });
            self.p = b;
        }
        self
    }
    /// Déplacement latéral sans tronçon (ressaut de quelques centimètres entre une voie droite et
    /// une voie courbe plus large).
    pub fn saute(mut self, p: V2) -> Trace {
        self.p = p;
        self
    }
    /// Courbe au rayon du gabarit : `angle` > 0 à gauche, < 0 à droite (radians).
    pub fn tourne(mut self, angle: f64) -> Trace {
        if angle.abs() < 1e-12 {
            return self;
        }
        let r = self.r;
        let s = angle.signum();
        // Centre à gauche (ou à droite) du cap.
        let n = v2(-self.cap.sin(), self.cap.cos()) * s;
        let c = self.p + n * r;
        let a0 = (self.p - c).y.atan2((self.p - c).x);
        let a1 = a0 + angle;
        self.troncons.push(Troncon::Arc { c, r, a0, a1, h: self.h_courbe });
        self.p = c + v2(a1.cos(), a1.sin()) * r;
        self.cap += angle;
        self
    }
}

/// Pas angulaire des polygones d'arc : 24 segments par quart de tour (flèche < 5 mm à 8,50 m).
fn segments(a0: f64, a1: f64) -> usize {
    (((a1 - a0).abs() / (PI / 2.0)) * 24.0).ceil().max(2.0) as usize
}

impl Troncon {
    /// Points de la trajectoire, chacun avec la normale à gauche du sens de parcours et la
    /// demi-largeur de voie.
    fn points(&self) -> Vec<(V2, V2, f64)> {
        match *self {
            Troncon::Droite { a, b, h } => {
                let d = (b - a).norm();
                let n = v2(-d.y, d.x);
                vec![(a, n, h), (b, n, h)]
            }
            Troncon::Arc { c, r, a0, a1, h } => {
                let k = segments(a0, a1);
                // À gauche du parcours : vers le centre si l'arc tourne à gauche, vers l'extérieur sinon.
                let s = (a1 - a0).signum();
                (0..=k)
                    .map(|i| {
                        let t = a0 + (a1 - a0) * i as f64 / k as f64;
                        let e = v2(t.cos(), t.sin());
                        (c + e * r, e * -s, h)
                    })
                    .collect()
            }
        }
    }
}

fn points(troncons: &[Troncon]) -> Vec<(V2, V2, f64)> {
    let mut out: Vec<(V2, V2, f64)> = Vec::new();
    for t in troncons {
        for q in t.points() {
            // Raccord de deux tronçons : on garde les deux largeurs (petit ressaut).
            if out.last().is_some_and(|l| (l.0 - q.0).len() < 1e-9 && (l.2 - q.2).abs() < 1e-9) {
                continue;
            }
            out.push(q);
        }
    }
    out
}

/// Trajectoire complète d'une suite de tronçons, sans points répétés.
pub fn axe(troncons: &[Troncon]) -> Vec<V2> {
    let mut out: Vec<V2> = Vec::new();
    for (p, _, _) in points(troncons) {
        if out.last().is_none_or(|q| (*q - p).len() > 1e-9) {
            out.push(p);
        }
    }
    out
}

/// Contour de l'épure : bord gauche dans le sens de parcours, puis bord droit à rebours.
pub fn contour(troncons: &[Troncon]) -> Vec<V2> {
    let pts = points(troncons);
    let mut out: Vec<V2> = pts.iter().map(|&(p, n, h)| p + n * h).collect();
    out.extend(pts.iter().rev().map(|&(p, n, h)| p - n * h));
    out
}

fn dans_polygone(p: V2, poly: &[V2]) -> bool {
    let mut dedans = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            dedans = !dedans;
        }
    }
    dedans
}

/// Les segments [a, b] et [c, d] se coupent (intersection propre).
fn coupe(a: V2, b: V2, c: V2, d: V2) -> bool {
    let o = |p: V2, q: V2, r: V2| (q - p).cross(r - p);
    let (d1, d2, d3, d4) = (o(c, d, a), o(c, d, b), o(a, b, c), o(a, b, d));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// Le polygone `poly` et le rectangle aligné `lo`–`hi` se chevauchent au-delà d'un contact.
pub fn touche(poly: &[V2], lo: V2, hi: V2) -> bool {
    let e = 1e-6;
    let (lo, hi) = (lo + v2(e, e), hi - v2(e, e));
    if hi.x <= lo.x || hi.y <= lo.y {
        return false;
    }
    let dans_rect = |p: V2| p.x > lo.x && p.x < hi.x && p.y > lo.y && p.y < hi.y;
    let coins = [lo, v2(hi.x, lo.y), hi, v2(lo.x, hi.y)];
    if poly.iter().any(|&p| dans_rect(p)) || coins.iter().any(|&c| dans_polygone(c, poly)) {
        return true;
    }
    let n = poly.len();
    (0..n).any(|i| (0..4).any(|j| coupe(poly[i], poly[(i + 1) % n], coins[j], coins[(j + 1) % 4])))
}

/// Bornes d'un quadrilatère.
pub fn bornes(q: &[V2]) -> (V2, V2) {
    q.iter().fold((v2(f64::MAX, f64::MAX), v2(f64::MIN, f64::MIN)), |(lo, hi), p| {
        (v2(lo.x.min(p.x), lo.y.min(p.y)), v2(hi.x.max(p.x), hi.y.max(p.y)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gabarit_nf() {
        assert!((rayon() - 6.5).abs() < 1e-12 && (marge() - 2.0).abs() < 1e-12 && (demi_droite() - 1.5).abs() < 1e-12);
        // Quart de tour à gauche depuis l'origine, cap +x : centre (0 ; 6,50), arrivée (6,50 ; 6,50)
        // au cap +y ; l'épure va de 4,50 à 8,50 du centre.
        let t = Trace::depuis(v2(0.0, 0.0), 0.0).tourne(PI / 2.0);
        assert!((t.p - v2(6.5, 6.5)).len() < 1e-9 && (t.cap - PI / 2.0).abs() < 1e-12);
        let b = contour(&t.troncons);
        let c = v2(0.0, 6.5);
        for p in &b {
            let r = (*p - c).len();
            assert!((r - 4.5).abs() < 1e-9 || (r - 8.5).abs() < 1e-9, "{r}");
        }
        // Le point (0 ; −1,90) est dans l'épure (à 8,40 du centre), (0 ; −2,10) dehors.
        assert!(touche(&b, v2(-0.05, -1.95), v2(0.05, -1.85)));
        assert!(!touche(&b, v2(-0.05, -2.15), v2(0.05, -2.05)));
        // Droite puis quart de tour : 3,00 m de large, puis 4,00 m.
        let t = Trace::depuis(v2(-5.0, 0.0), 0.0).droit(5.0).tourne(PI / 2.0);
        let b = contour(&t.troncons);
        assert!(touche(&b, v2(-3.0, 1.4), v2(-2.0, 1.45)));
        assert!(!touche(&b, v2(-3.0, 1.5), v2(-2.0, 3.0)), "contact seulement");
        assert!(touche(&b, v2(-0.05, -1.95), v2(0.05, -1.85)));
        // Courbe à droite : centre à droite du cap ; bord gauche à l'extérieur.
        let t = Trace::depuis(v2(0.0, 0.0), PI / 2.0).tourne(-PI / 2.0);
        assert!((t.p - v2(6.5, 6.5)).len() < 1e-9 && t.cap.abs() < 1e-12);
        let b = contour(&t.troncons);
        assert!((b[0] - v2(-2.0, 0.0)).len() < 1e-9, "{:?}", b[0]);
        assert_eq!(axe(&t.troncons).len(), segments(0.0, PI / 2.0) + 1);
    }
}
