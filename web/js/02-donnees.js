// ============================================================================
// Données : sites d'exemple (mètres, repère local, y vers le haut).
// ============================================================================
const DEFAULT_PARAMS = { stall_w: 2.5, stall_d: 5, noue_w: 0, basin_h: 1.5, basin_margin: 2, aisle: 6, aisle_one: 5, entry_w: 6, building_margin: 2, door_w: 3, planter_every: 8, cap: 1.5, pmr_ratio: 2, irve_ratio: 20, irve_equip_ratio: 5, angle: null, slant: 90, programme: "public", canopy: 8, ombrieres: false,
  silo: false, silo_etage: 2.9, silo_niveaux: 1, silo_pente: 18, silo_systeme: "coule", silo_pu: 0.3, silo_pv: 0.6, silo_travee: 3, silo_facade: 0.2, silo_rives: true, silo_densite: "longue", silo_retrait: 0.65, silo_rampes: "auto", silo_ventile: true, silo_asc: "deux", silo_controle: false, silo_voies: 3 };

function arc(cx, cy, r, a0, a1, n) {
  const pts = [];
  for (let i = 0; i <= n; i++) { const a = (a0 + (a1 - a0) * i / n) * Math.PI / 180; pts.push([+(cx + r * Math.cos(a)).toFixed(2), +(cy + r * Math.sin(a)).toFixed(2)]); }
  return pts;
}
// Accès posés par l'application au tiers de leur limite : le milieu porte les boutons du segment.
const ENTRY_T = 1 / 3;
const EXAMPLES = {
  ruisseau: () => ({
    parcel: [[0, 0], [160, 0], [160, 110], [0, 110]], setbacks: [5, 3, 5, 3],
    exclusions: [[[-5, 70], [20, 72], [45, 85], [70, 92], [95, 95], [125, 100], [165, 104], [165, 80], [130, 78], [105, 70], [80, 58], [55, 48], [30, 45], [-5, 46]]],
    buildings: [{ cx: 132.5, cy: 29, w: 42, h: 35, rot: 90, shape: "L" }],
    landscapes: [[[8, 8], [30, 8], [30, 40], [8, 40]]],
    entries: [{ edge: 0, t: ENTRY_T }, { edge: 2, t: 0.75 }],
  }),
  courbe: () => {
    const curve = arc(-60, 0, 200, 0, 35, 20).slice(1);
    const parcel = [[0, 0], [140, 0], ...curve, [0, +(200 * Math.sin(35 * Math.PI / 180)).toFixed(2)]];
    return { parcel, setbacks: parcel.map((_, i) => (i === 0 ? 5 : 3)), exclusions: [], buildings: [], landscapes: [], basins: [], entries: [{ edge: 0, t: 0.3 }] };
  },
  triangle: () => ({ parcel: [[0, 0], [150, 0], [30, 170]], setbacks: [5, 3, 3], exclusions: [], buildings: [{ cx: 52, cy: 78, w: 24, h: 40, rot: 80, shape: "rect" }], landscapes: [], basins: [], entries: [{ edge: 0, t: ENTRY_T }] }),
  rectangle: () => ({ parcel: [[0, 0], [120, 0], [120, 80], [0, 80]], setbacks: [5, 3, 3, 3], exclusions: [], buildings: [], landscapes: [], basins: [], entries: [{ edge: 0, t: ENTRY_T }] }),
};

let site = EXAMPLES.ruisseau();
let params = { ...DEFAULT_PARAMS };
let exampleName = "ruisseau";
let layout = null, lastAngle = null, lastMs = 0;
let NSKEL = false; // squelette des noues affiché
let noueBtns = []; // pastilles des noues au dernier dessin (centres écran)
let SKEL = false; // squelette des voies affiché (réglage de présentation, hors historique)
const PILL = { w: 68, h: 32 }; // pastilles du squelette
let pills = []; // pastilles posées au dernier dessin (centres écran), pour le toucher
let hoverEdge = null; // limite de parcelle survolée (ou touchée) : porte les boutons Sommet / Courbe
const EBTN = { r: 9, gap: 22, out: 16 }; // `out` : décalage vers l'extérieur sur la limite de parcelle
try { NSKEL = localStorage.getItem("trame.nskel") === "1"; } catch (e) { /* stockage indisponible */ }
try { SKEL = localStorage.getItem("trame.skel") === "1"; } catch (e) { /* stockage indisponible */ }

// Formes d'objets → polygones.
// Entrée d'un bâtiment : sur une arête (b.door = { e: arête, t: abscisse }), par défaut sur l'arête la plus proche du premier accès véhicules.
function doorGeom(b) {
  const P = rectPoly(b), n = P.length, sgn = Math.sign(polyArea(P)) || 1;
  let d = b.door;
  if (!d || d.e >= n) {
    const ref = site.entries.length && site.entries[0].edge < site.parcel.length ? entryPoint(site.entries[0]) : site.parcel.reduce((m, p) => [m[0] + p[0] / site.parcel.length, m[1] + p[1] / site.parcel.length], [0, 0]);
    let best = 0, bd = Infinity;
    for (let i = 0; i < n; i++) { const a = P[i], c = P[(i + 1) % n], dd = dist2([(a[0] + c[0]) / 2, (a[1] + c[1]) / 2], ref); if (dd < bd) { bd = dd; best = i; } }
    d = { e: best, t: 0.5 };
  }
  const a = P[d.e], c = P[(d.e + 1) % n], L = Math.hypot(c[0] - a[0], c[1] - a[1]) || 1, u = [(c[0] - a[0]) / L, (c[1] - a[1]) / L];
  const half = Math.min(params.door_w, L) / 2 / L, t = Math.max(half, Math.min(1 - half, d.t));
  return { e: d.e, t, u, n: [u[1] * sgn, -u[0] * sgn], L, p: [a[0] + (c[0] - a[0]) * t, a[1] + (c[1] - a[1]) * t] };
}
function rectPoly(o) {
  const c = Math.cos(o.rot * Math.PI / 180), s = Math.sin(o.rot * Math.PI / 180), w = o.w / 2, h = o.h / 2;
  let local;
  if (o.shape === "L") {
    const t = Math.max(6, Math.min(o.w, o.h) * 0.4);
    local = [[-w, -h], [w, -h], [w, -h + t], [-w + t, -h + t], [-w + t, h], [-w, h]];
  } else local = [[-w, -h], [w, -h], [w, h], [-w, h]];
  return local.map(([x, y]) => [o.cx + x * c - y * s, o.cy + x * s + y * c]);
}
// Objets polygonaux (sommets modifiables) : type de sélection → liste du site.
const POLY = { exclusion: "exclusions", landscape: "landscapes", basin: "basins" };
// Projets enregistrés avant que les espaces verts ne soient des polygones : rectangle → sommets.
function normSite(s) { if (s && !s.basins) s.basins = []; if (s && s.gir && s.gir.margin === 0.2) s.gir.margin = 0.1; /* ancienne marge par défaut */ if (s && s.parcel) s.arcs = s.parcel.map((_, i) => (s.arcs && s.arcs[i]) || 0); if (s && s.landscapes) s.landscapes = s.landscapes.map((o) => (Array.isArray(o) ? o : rectPoly(o))); return s; }
function polyArea(p) { let a = 0; for (let i = 0; i < p.length; i++) { const q = p[(i + 1) % p.length]; a += p[i][0] * q[1] - q[0] * p[i][1]; } return a / 2; }

function engineInput(quick, turns = false) {
  const p = params;
  return {
    ...flatSite(), exclusions: site.exclusions.map(polyOutline),
    buildings: site.buildings.map((b) => { const d = doorGeom(b); return { poly: rectPoly(b), entrance: [d.p[0] + d.n[0] * params.building_margin, d.p[1] + d.n[1] * params.building_margin], door: { p: d.p, n: d.n, w: Math.min(params.door_w, d.L) } }; }),
    landscapes: site.landscapes.map(polyOutline), basins: (site.basins || []).map(polyOutline), lanes: site.lanes || [], noue_off: site.noue_off || [], traverses: site.traverses || [],
    params: {
      slant: p.slant, stall_w: p.stall_w, stall_d: p.stall_d, noue_w: p.noue_w, basin_h: p.basin_h, basin_margin: p.basin_margin, aisle: p.aisle, aisle_one: p.aisle_one, entry_w: p.entry_w, building_margin: p.building_margin,
      planter_every: Math.max(0, Math.round(p.planter_every)), cap: p.cap,
      pmr_ratio: p.pmr_ratio / 100, irve_ratio: p.irve_ratio / 100, irve_equip_ratio: p.irve_equip_ratio / 100,
      angle: p.angle, quick: !!quick && lastAngle !== null && p.angle === null, angle_hint: lastAngle, turns: !!turns, programme: p.programme, canopy: p.canopy, ombrieres: !!p.ombrieres,
    },
    ouvrage: p.silo ? { actif: true, facade: p.silo_facade, poteau: [p.silo_pu, p.silo_pv], travee: Math.max(1, Math.round(p.silo_travee)), rives: !!p.silo_rives, densite: p.silo_densite, retrait: p.silo_retrait, systeme: p.silo_systeme, angle: p.angle, hauteur_etage: p.silo_etage, niveaux: Math.max(1, Math.round(p.silo_niveaux)), implantation: p.silo_rampes, largement_ventile: p.silo_ventile !== false, controle: !!p.silo_controle, voies_controle: Math.min(4, Math.max(1, Math.round(p.silo_voies))), ascenseurs_demi: p.silo_asc || "deux", noyaux: site.noyaux || null, pente_rampe: p.silo_pente / 100, emprise: site.slab ? { cx: site.slab.cx, cy: site.slab.cy, w: site.slab.w, h: site.slab.h, rot: site.slab.rot, fixe: site.slab.fixe || [0, 0] } : null } : null,
  };
}

