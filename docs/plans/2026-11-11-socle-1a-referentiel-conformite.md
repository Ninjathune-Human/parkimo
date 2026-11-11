# Socle 1a : Parkimo, référentiel de règles commun et conformité

**But.** La caisse s'appelle `parkimo`, les règles vivent dans un référentiel commun à la surface et à l'ouvrage, et le moteur rend des constats de conformité que l'interface affiche.

**Architecture.** `src/normes.rs` (déplacé de `src/ouvrage/normes.rs`, complété) porte les valeurs sourcées. `src/conformite.rs` lit `Site` et `Layout` et rend `Vec<Constat>`, ajouté à `Layout::constats` en fin de `solve` (hors calcul rapide). L'interface n'a aucune règle : elle affiche les constats.

**Spécification.** `docs/2026-11-11-audit-et-direction.md`, § 4 et § 5 (décisions 2 et 3).

## Contraintes

- Aucun plan existant ne change : les constats observent, ils ne modifient ni les places ni la circulation.
- Chaque valeur exigée vient de `normes.rs` avec sa source ; aucune valeur en dur dans `conformite.rs`.
- NF P 91-120 n'est pas intégrée (texte non disponible) : le programme privatif ne rend pas les constats NF P 91-100 et le dit.
- Clés de stockage du navigateur (`trame.*`) inchangées : elles portent les projets des utilisateurs.
- Français, virgule décimale dans les libellés.

## Points de vigilance (tests ajoutés aux tâches)

1. Plan vide ou parcelle invalide : `constats` vide, pas de panique (tâche 3).
2. Places à 30° : hors tableau 2, constat d'information et non écart (tâche 3).
3. 20 places ou moins : pas d'exigence de double sens majoré (tâche 3).
4. Silo sans PMR posée : écart sur le nombre de places adaptées (tâche 4).
5. Calcul rapide pendant un glisser : `constats` vide, l'interface garde les précédents (tâches 3 et 5).

## Tâche 1 : renommage

Fichiers : `Cargo.toml`, `Cargo.lock`, `build.sh`, `README.md`, `tests/*.rs`, `examples/*.rs`, `web/app.html` (titre, message sans JavaScript, noms des fichiers exportés).

- `name = "parkimo"` ; `use trame::` devient `use parkimo::` ; `target/.../parkimo.wasm`.
- Vérification : `cargo test --release` vert ; `grep -rn "trame::" tests examples src` vide.

## Tâche 2 : référentiel commun

Fichiers : `src/normes.rs` (déplacé), `src/lib.rs`, `src/ouvrage/*.rs` (chemins), `src/model.rs`.

- `pub mod normes;` à la racine ; `ouvrage` importe `crate::normes`.
- Ajouts : `normes::irve` (`PART_PREEQUIPEE` 0,20 et `SEUIL_NEUF` 10 places, L. 113-12 ; `TRANCHE_BORNE` 20 places, L. 113-13 ; `fn bornes(places) -> usize`), `normes::urbanisme` (`SEUIL_OMBRAGE` 500 m², `SEUIL_OMBRIERES` 1 500 m², `PART` 0,5).
- `Params::programme: Programme` (`public` par défaut, `privatif`), sérialisé en minuscules.
- Tests (dans `normes.rs`) : `bornes(95) == 4`, `bornes(20) == 1`, `bornes(19) == 0`.

## Tâche 3 : constats de surface

Fichiers : `src/conformite.rs` (nouveau), `src/model.rs` (`Layout::constats`), `src/lib.rs` (appel), `tests/conformite.rs` (nouveau).

Interface produite :

```rust
pub enum Statut { Conforme, Ecart, Info }            // "conforme", "ecart", "info"
pub struct Constat { pub code: String, pub libelle: String, pub source: String, pub statut: Statut,
                     pub mesure: Option<f64>, pub exige: Option<f64>, pub unite: String, pub detail: String }
pub fn controler(site: &Site, lay: &Layout) -> Vec<Constat>
```

Codes et règles (programme public) :

| Code | Mesuré | Exigé |
| --- | --- | --- |
| `nf.place.largeur` | `stall_w` | `nf::dims_angle(inclinaison).larg` ; information à 30° |
| `nf.demi_travee` | `row_d + row_aisle` | `long + allee` du tableau 2 (note 2) |
| `nf.allee.double_sens` | plus petite largeur des axes à double sens | `nf::allee_double_sens(allee, places)` si plus de 20 places |
| `acc.pmr.nombre` | `metrics.pmr` | `accessibilite::places_pmr_erp` ; information au-delà de 500 |
| `acc.pmr.largeur` | `pmr_w` | 3,30 |
| `acc.pmr.longueur` | `stall_d` | 5,00 |
| `irve.preequipement` | `metrics.irve` | 20 % arrondi au supérieur, au-delà de 10 places |
| `irve.bornes` | `metrics.irve_equipped` | `irve::bornes(places)` |
| `urba.ombrage` | `metrics.paved_area` | information si plus de 500 m² |
| `urba.ombrieres` | `metrics.paved_area` | information si plus de 1 500 m² |

Tests (écrits d'abord, sur `scenarios/rect.json`) : défauts tous conformes ou information ; `stall_w = 2.2` donne un écart `nf.place.largeur` (2,20 pour 2,30) ; `aisle = 5.5` donne un écart `nf.allee.double_sens` (5,50 pour 6,00) ; `pmr_ratio = 0` donne un écart `acc.pmr.nombre` ; `slant = 30` donne une information ; parcelle vide et `quick` donnent une liste vide ; programme privatif : aucun code `nf.`.

## Tâche 4 : constats de l'ouvrage

Fichiers : `src/conformite.rs`, `tests/conformite.rs`.

| Code | Mesuré | Exigé |
| --- | --- | --- |
| `nf.poteau.retrait` | `stall_w` | largeur du tableau 2 plus `nf::surlargeur_poteau(retrait)` ; écart si retrait sous 0,25 |
| `nf.hauteur` | `hauteur_etage` | `plancher.hauteur_min` |
| `ps.evacuation` | `evacuation.distance_max` | `limite_deux` ; écart si `hors > 0` |
| `ps.compartiment` | `surface` du niveau | information au-delà de 3 000 m² (PS 12) |

Tests (sur `scenarios/silo.json`) : silo par défaut à trois niveaux, écart `acc.pmr.nombre` ; `retrait = 0.40` avec places de 2,30 donne un écart `nf.poteau.retrait` (2,30 pour 2,40) ; `hauteur_etage = 2.4` donne un écart `nf.hauteur`.

## Tâche 5 : panneau Conformité

Fichier : `web/app.html`.

- Carte `#conf` entre le bilan et les couches, même gabarit que `#layers` (en-tête, encoche, corps défilant) : pastille de statut, libellé, mesuré et exigé, source en petit ; écarts d'abord. En-tête : nombre d'écarts ou « conforme ».
- Paramètres : commande segmentée Programme (Public, Privatif), liée à `params.programme`.
- Pendant un calcul rapide, la liste précédente reste affichée.
- Vérification : contrôle des identifiants, page compilée par l'intégration continue, essai dans Chrome en formats PC et iPhone (défauts sans écart ; allée à 5,50 m, un écart affiché).
