//! Projections : repère local du plan → Web Mercator → RGF93 géographique → Lambert-93 ou CC.
//!
//! Les services IGN livrent les données en EPSG:4326 par transformation nulle depuis RGF93 :
//! appliquer la conique conforme sur l'ellipsoïde GRS80 restitue les coordonnées officielles.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

const R_MERC: f64 = 6378137.0;
const A_GRS80: f64 = 6378137.0;
const INV_F_GRS80: f64 = 298.257222101;

/// Conique conforme de Lambert sécante (deux parallèles automécoïques), ellipsoïde GRS80.
pub struct Lcc {
    e: f64,
    n: f64,
    af: f64,
    rho0: f64,
    lon0: f64,
    x0: f64,
    y0: f64,
    pub epsg: u32,
    pub name: String,
}

fn m(e: f64, phi: f64) -> f64 {
    phi.cos() / (1.0 - e * e * phi.sin().powi(2)).sqrt()
}

fn t(e: f64, phi: f64) -> f64 {
    let es = e * phi.sin();
    (FRAC_PI_4 - phi / 2.0).tan() / ((1.0 - es) / (1.0 + es)).powf(e / 2.0)
}

impl Lcc {
    #[allow(clippy::too_many_arguments)]
    fn new(
        lat0: f64,
        lat1: f64,
        lat2: f64,
        lon0: f64,
        x0: f64,
        y0: f64,
        epsg: u32,
        name: &str,
    ) -> Lcc {
        let f = 1.0 / INV_F_GRS80;
        let e = (2.0 * f - f * f).sqrt();
        let (p0, p1, p2) = (lat0.to_radians(), lat1.to_radians(), lat2.to_radians());
        let (m1, m2) = (m(e, p1), m(e, p2));
        let (t0, t1, t2) = (t(e, p0), t(e, p1), t(e, p2));
        let n = (m1.ln() - m2.ln()) / (t1.ln() - t2.ln());
        let af = A_GRS80 * m1 / (n * t1.powf(n));
        Lcc {
            e,
            n,
            af,
            rho0: af * t0.powf(n),
            lon0: lon0.to_radians(),
            x0,
            y0,
            epsg,
            name: name.to_string(),
        }
    }

    /// RGF93 v1 / Lambert-93 (EPSG:2154).
    pub fn lambert93() -> Lcc {
        Lcc::new(
            46.5,
            49.0,
            44.0,
            3.0,
            700_000.0,
            6_600_000.0,
            2154,
            "RGF93 / Lambert-93",
        )
    }

    /// RGF93 v1 / CCxx, zones 42 à 50 (EPSG:3942 à 3950).
    pub fn cc(zone: u32) -> Lcc {
        let z = zone.clamp(42, 50);
        let lat0 = z as f64;
        let y0 = (z as f64 - 41.0) * 1_000_000.0 + 200_000.0;
        Lcc::new(
            lat0,
            lat0 - 0.75,
            lat0 + 0.75,
            3.0,
            1_700_000.0,
            y0,
            3900 + z,
            &format!("RGF93 / CC{z}"),
        )
    }

    /// Zone CC la mieux adaptée à une latitude (zones centrées sur les degrés entiers).
    pub fn cc_zone_for(lat: f64) -> u32 {
        (lat.round() as i64).clamp(42, 50) as u32
    }

    /// Longitude, latitude en degrés → coordonnées planes en mètres.
    pub fn forward(&self, lon: f64, lat: f64) -> (f64, f64) {
        let rho = self.af * t(self.e, lat.to_radians()).powf(self.n);
        let theta = self.n * (lon.to_radians() - self.lon0);
        (
            self.x0 + rho * theta.sin(),
            self.y0 + self.rho0 - rho * theta.cos(),
        )
    }
}

/// Web Mercator (EPSG:3857) → longitude, latitude en degrés.
pub fn merc_to_lonlat(x: f64, y: f64) -> (f64, f64) {
    (
        (x / R_MERC).to_degrees(),
        (2.0 * (y / R_MERC).exp().atan() - FRAC_PI_2).to_degrees(),
    )
}

/// Repère local du plan : mètres = (Mercator − origine) × k, k = cos(latitude de l'origine).
#[derive(serde::Deserialize, Clone, Copy)]
pub struct Georef {
    pub o: [f64; 2],
    pub k: f64,
}

impl Georef {
    pub fn to_lonlat(&self, x: f64, y: f64) -> (f64, f64) {
        merc_to_lonlat(x / self.k + self.o[0], y / self.k + self.o[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Valeurs de référence calculées avec PROJ 9 (pyproj 3.8), EPSG:4171 → EPSG:2154 / 394x.
    const REF: &[(&str, f64, f64, f64, f64)] = &[
        (
            "L93",
            7.261992898848815,
            43.700032530420266,
            1043470.5467,
            6298271.7222,
        ),
        ("L93", 2.3522, 48.8566, 652469.0227, 6862035.2594),
        ("L93", -4.4861, 48.3904, 146632.9785, 6836262.3267),
        ("L93", 7.7521, 48.5734, 1050362.6954, 6840899.6472),
        ("L93", 8.7369, 41.9192, 1176526.6023, 6108263.0226),
        (
            "CC44",
            7.261992898848815,
            43.700032530420266,
            2043378.2972,
            3175547.3895,
        ),
        ("CC44", -4.4861, 48.3904, 1144717.6814, 3713676.5824),
        ("CC49", 2.3522, 48.8566, 1652467.8689, 8184256.8691),
        ("CC49", 8.7369, 41.9192, 2178899.3775, 7429266.6493),
        ("CC42", 7.7521, 48.5734, 2052867.4467, 1941928.7184),
    ];

    #[test]
    fn matches_proj_to_the_millimetre() {
        for &(crs, lon, lat, ex, ey) in REF {
            let p = match crs {
                "L93" => Lcc::lambert93(),
                z => Lcc::cc(z[2..].parse().unwrap()),
            };
            let (x, y) = p.forward(lon, lat);
            assert!(
                (x - ex).abs() < 1e-3 && (y - ey).abs() < 1e-3,
                "{crs} {lon},{lat}: {x:.4},{y:.4} attendu {ex},{ey}"
            );
        }
    }

    #[test]
    fn mercator_round_trip() {
        let (lon, lat): (f64, f64) = (7.2619929, 43.7000325);
        let x = lon.to_radians() * R_MERC;
        let y = R_MERC * (FRAC_PI_4 + lat.to_radians() / 2.0).tan().ln();
        let (lo, la) = merc_to_lonlat(x, y);
        assert!((lo - lon).abs() < 1e-11 && (la - lat).abs() < 1e-11);
        assert_eq!(Lcc::cc_zone_for(43.7), 44);
        assert_eq!(Lcc::cc_zone_for(51.2), 50);
    }
}
