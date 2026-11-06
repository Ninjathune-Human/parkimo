//! Plateau de silo à grande portée : modules de rangées doubles (place + allée + place) côte à
//! côte, poteaux en façade et sur les lignes de dos à dos, places de rive le long des façades
//! d'extrémité, allées transversales aux deux bouts.
//!
//! Repère local : u le long des allées, v en travers ; origine au coin de la dalle.
//!
//! Poteaux : toujours dans le premier mètre du fond de place (≤ 1,10 m, NF P 91-100 fig. 2), à
//! cheval sur une séparation de places ou contre l'allée transversale ; la largeur au droit de
//! l'obstacle reste d'au moins 2,00 m pour des places de 2,30 m et plus.

use crate::geom::{rotate_rings, strip_intervals, v2, V2};

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
}

impl Gabarit {
    /// Module de rangée double : place + allée + place.
    pub fn module(&self) -> f64 {
        2.0 * self.d + self.allee
    }
    /// Zone d'extrémité depuis le nu intérieur de la façade : rive et allée transversale.
    fn bout(&self) -> f64 {
        if self.rives {
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

/// Files de poteaux en travers (positions v) : façade, lignes de dos à dos, façade opposée.
fn lignes_v(g: &Gabarit, t: &Travers, mut f: impl FnMut(f64)) {
    let m = g.module();
    f(g.facade + g.pv / 2.0);
    for k in 1..t.modules {
        f(g.facade + k as f64 * m);
    }
    if t.demi && t.modules > 0 {
        f(g.facade + t.modules as f64 * m);
    }
    f(t.largeur - g.facade - g.pv / 2.0);
}

/// Places de rive d'une façade d'extrémité : entre deux files de poteaux, à partir de l'axe.
fn rive_par_bout(g: &Gabarit, t: &Travers) -> usize {
    let (mut prev, mut n) = (None::<f64>, 0usize);
    lignes_v(g, t, |v| {
        if let Some(p) = prev {
            n += ((v - p + 1e-9) / g.w).floor() as usize;
        }
        prev = Some(v);
    });
    n
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

/// Nombre de places d'un plateau `larg` × `l` (l le long des allées), sans rien construire.
pub fn compte(g: &Gabarit, larg: f64, l: f64) -> Option<(usize, Travers, Long)> {
    let t = travers(g, larg)?;
    let lo = long(g, l)?;
    let rangees = 2 * t.modules as usize + t.demi as usize;
    let mut n = rangees * lo.par_rangee as usize;
    if g.rives {
        n += 2 * rive_par_bout(g, &t);
    }
    Some((n, t, lo))
}

/// Place en repère local : [fond a, fond b, avant b, avant a] et sa clé BIM.
#[derive(Clone, Debug)]
pub struct PlaceLoc {
    pub q: [V2; 4],
    pub cle: String,
    pub rive: bool,
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
    pub places: Vec<PlaceLoc>,
    pub allees: Vec<([V2; 4], String)>,
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

/// Construit le plateau `larg` × `l` (dimensions utiles, voir `compte`).
pub fn construire(g: &Gabarit, larg: f64, l: f64) -> Option<Plan> {
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
    let mut axes_v = Vec::new();
    lignes_v(g, &t, |v| axes_v.push(v));

    // Places de rive : fond contre la façade d'extrémité, entre deux files de poteaux.
    if g.rives {
        for (b, (fond, sens)) in [(g.facade, 1.0), (lo.longueur - g.facade, -1.0)].into_iter().enumerate() {
            for (s, pair) in axes_v.windows(2).enumerate() {
                let k = ((pair[1] - pair[0] + 1e-9) / w).floor() as usize;
                for j in 0..k {
                    let (va, vb) = (pair[0] + j as f64 * w, pair[0] + (j + 1) as f64 * w);
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

    // Files transversales : façades d'extrémité, début et fin des rangées (poteaux au nu de l'allée
    // transversale), puis toutes les `travee` places ; une dernière travée courte est absorbée.
    let tw = g.travee.max(1) as f64 * w;
    let mut axes_u = vec![g.facade + g.pu / 2.0, debut + g.pu / 2.0];
    let mut x = debut + tw;
    while x < fin - tw / 2.0 - 1e-9 {
        axes_u.push(x);
        x += tw;
    }
    axes_u.push(fin - g.pu / 2.0);
    axes_u.push(lo.longueur - g.facade - g.pu / 2.0);

    let mut poteaux = Vec::new();
    for (i, &u) in axes_u.iter().enumerate() {
        for (k, &v) in axes_v.iter().enumerate() {
            poteaux.push(PoteauLoc {
                c: v2(u, v),
                cle: format!("P/{}{}", lettre(k), i + 1),
            });
        }
    }
    Some(Plan {
        travers: t,
        long: lo,
        places,
        allees,
        poteaux,
        axes_u,
        axes_v,
    })
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
    // (places, surface utile, emprise)
    let mut best: Option<(usize, f64, Emprise)> = None;
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
                    if best.map_or(true, |b| borne >= b.0) {
                        let cx = lo.x + (s0 as f64 + (i - s0) as f64 / 2.0) * pas;
                        let cy = lo.y + (j as f64 + 1.0 - h0 as f64 / 2.0) * pas;
                        let opts: &[(f64, f64, bool)] = if sens_impose {
                            &[(hy, wx, true)]
                        } else {
                            &[(hy, wx, true), (wx, hy, false)]
                        };
                        for &(larg, l, le_long_de_x) in opts {
                            let Some((n, t, lg)) = compte(g, larg, l) else { continue };
                            let aire = t.largeur * lg.longueur;
                            if best.map_or(true, |b| n > b.0 || (n == b.0 && aire < b.1 - 1e-6)) {
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
    best.map(|(n, _, e)| (e, n))
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
        // Files transversales : 2 façades + début + 4 intermédiaires (pas de 7,50 m) + fin.
        assert_eq!(p.axes_u.len(), 2 + 2 + 4);
        assert_eq!(p.poteaux.len(), 8 * 3);
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
    fn poteaux_dans_le_fond_des_places() {
        // Aucun poteau dans une allée ; tout poteau qui touche une place reste dans son premier mètre
        // et laisse au moins 2,00 m de largeur (NF P 91-100, fig. 2).
        let g = gab();
        for (larg, l) in [(32.4, 60.0), (28.0, 41.3), (49.0, 77.7)] {
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
                    assert!(inter(q).is_none(), "{larg}×{l} : poteau {} dans {cle}", po.cle);
                }
                for pl in &p.places {
                    if let Some((ix, iy)) = inter(&pl.q) {
                        // Profondeur depuis le fond (q[0]-q[1]) et largeur restante.
                        let fond_u = (pl.q[0].x - pl.q[1].x).abs() < 1e-9;
                        let (prof, reste) = if fond_u { (ix, g.w - iy) } else { (iy, g.w - ix) };
                        assert!(prof <= 1.10 + 1e-9 && reste >= 2.0 - 1e-9, "{larg}×{l} : {} dans {}", po.cle, pl.cle);
                    }
                }
            }
        }
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
