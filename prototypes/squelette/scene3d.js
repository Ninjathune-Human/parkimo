// Scène 3D du squelette : le plateau (plateau.js) empilé en niveaux et décrit en éléments typés, que la vue three.js
// affiche sans rien décider. Fonction pure, sans three.js : elle se teste sous Node. Schéma repris de src/scene.rs
// (volumes prismatiques par matière, rampes, niveaux), complété de rubans (pièces inclinées), de sols peints et de traits.
//
// Cotes en mètres, z = 0 au sol fini. Un plancher par niveau, tous sur le même squelette (charte, section 6).
// Parc largement ventilé : RDC, R+1... au-dessus du sol, puis la toiture qui couvre le dernier niveau. Parc enterré :
// R-1, R-2... sous le terrain, puis la couverture au niveau du sol. Toiture et couverture portent sur les poteaux et
// les poutres du dernier niveau ; la vue les lève, comme le couvercle d'une maquette. Les rampes relient deux planchers
// de stationnement voisins (et le sol, en enterré) et montent dans le sens de leur branche ; la dalle du dessus est
// ouverte sur leur emprise (trémie), à chaque niveau desservi.
// Hypothèses de représentation, sans norme : dallage, radier, allèges, rives des rampes, édicules, terrain.

export const ALLEGE = 1.1, ACROTERE = 0.6, EDICULE = 2.8, DALLAGE = 0.15, RADIER = 0.3, MUR = 0.2, MUR_ENTERRE = 0.3, MARGE = 8, SOCLE = 1.2;
export const RIVE = { h: 1.0, e: 0.15 }, PORTE = { l: 0.9, h: 2.1 }, MARCHE = 0.3, PALIER = 0.2;

const aire = (r) => r.reduce((m, p, i) => { const n = r[(i + 1) % r.length]; return m + p[0] * n[1] - n[0] * p[1]; }, 0) / 2;
const oriente = (r, ccw) => ((aire(r) > 0) === ccw ? r : [...r].reverse());
const ouvert = (r) => (r.length > 1 && r[0][0] === r[r.length - 1][0] && r[0][1] === r[r.length - 1][1] ? r.slice(0, -1) : r);
const milieu = (a, b) => [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
const dist = (a, b) => Math.hypot(b[0] - a[0], b[1] - a[1]);
const unit = (a, b) => { const l = dist(a, b) || 1; return [(b[0] - a[0]) / l, (b[1] - a[1]) / l]; };
export const cercle = (c, R, n = 64) => Array.from({ length: n }, (_, k) => [c[0] + R * Math.cos((2 * Math.PI * k) / n), c[1] + R * Math.sin((2 * Math.PI * k) / n)]);
const boite = (rs) => { let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity; for (const r of rs) for (const p of r) { x0 = Math.min(x0, p[0]); y0 = Math.min(y0, p[1]); x1 = Math.max(x1, p[0]); y1 = Math.max(y1, p[1]); } return [x0, y0, x1, y1]; };
const touche = (a, b) => a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3];
// Rectangle autour du segment a-b, d'épaisseur e : centré, ou tout entier à gauche (cote 1) ou à droite (cote -1).
const bande = (a, b, e, cote = 0, rallonge = 0) => {
  const u = unit(a, b), n = [-u[1], u[0]], g = cote === 0 ? e / 2 : cote > 0 ? e : 0, d = cote === 0 ? e / 2 : cote > 0 ? 0 : e;
  const A = [a[0] - u[0] * rallonge, a[1] - u[1] * rallonge], B = [b[0] + u[0] * rallonge, b[1] + u[1] * rallonge];
  return oriente([[A[0] - n[0] * d, A[1] - n[1] * d], [B[0] - n[0] * d, B[1] - n[1] * d], [B[0] + n[0] * g, B[1] + n[1] * g], [A[0] + n[0] * g, A[1] + n[1] * g]], true);
};

// Opérations sur les polygones (polygon-clipping, passé par l'appelant). Un multipolygone est une liste de polygones,
// chacun une liste d'anneaux : le contour, dans le sens trigonométrique, puis les vides, dans l'autre sens.
// Les entrées sont calées au millimètre : sans cela, polygon-clipping échoue parfois sur des sommets presque confondus.
function outils(clip) {
  const mm = (r) => r.map((p) => [Math.round(p[0] * 1000) / 1000, Math.round(p[1] * 1000) / 1000]);
  const valide = (r) => r.length >= 3 && Math.abs(aire(r)) > 0.01;
  const net = (mp) => mp.map((pg) => pg.map(ouvert).filter(valide).map((r, k) => oriente(r, k === 0))).filter((pg) => pg.length && aire(pg[0]) > 0.01);
  const echecs = [], sur = (f, repli) => { try { return f(); } catch (e) { echecs.push(e.message); return repli; } };
  const polys = (rs) => rs.filter(valide).map((r) => [mm(r)]), multi = (mp) => mp.map((pg) => pg.map(mm));
  return {
    echecs,
    reunion: (rs) => { const v = polys(rs); return v.length ? sur(() => net(clip.union(...v)), v.map((pg) => [oriente(pg[0], true)])) : []; },
    moins: (mp, rs) => { const v = polys(rs); return v.length && mp.length ? sur(() => net(clip.difference(multi(mp), ...v)), mp) : mp; },
    inter: (mp, rs) => { const v = polys(rs); return v.length && mp.length ? sur(() => net(clip.intersection(multi(mp), v)), []) : []; },
    dehors: (a, b) => (b.length && a.length ? sur(() => net(clip.difference(multi(a), multi(b))), a) : a),
    dedans: (a, b) => (b.length && a.length ? sur(() => net(clip.intersection(multi(a), multi(b))), []) : []),
  };
}

export function scene(P, clip) {
  const sc = { min: [0, 0], max: [0, 0], haut: 0, bas: 0, niveaux: [], volumes: [], rubans: [], sols: [], traits: [], rampes: [], constats: [], places: 0 };
  if (!P || !P.chaussees.length) return sc;
  const O = outils(clip), E = P.E, n = P.niveaux, ventile = P.parc === "ventile";
  const vol = (matiere, anneaux, z0, z1, niveau, cle) => { if (z1 > z0 + 1e-6 && anneaux.length && anneaux[0].length >= 3) sc.volumes.push({ matiere, anneaux, z0: +z0.toFixed(4), z1: +z1.toFixed(4), niveau, ...(cle ? { cle } : {}) }); };
  const vols = (matiere, mp, z0, z1, niveau, cle) => { for (const pg of mp) vol(matiere, pg, z0, z1, niveau, cle); };

  // ---- Planchers. Le dernier (T) est la toiture du silo, ou la couverture du parc enterré. H : le plus haut plancher
  // que rampes et escaliers desservent, le dernier niveau de stationnement en silo, le sol en enterré.
  const F = ventile ? [...Array.from({ length: n }, (_, f) => ({ z: f * E, nom: f ? `R+${f}` : "RDC", places: true })), { z: n * E, nom: "Toiture", places: false }]
    : [...Array.from({ length: n }, (_, f) => ({ z: -(n - f) * E, nom: `R-${n - f}`, places: true })), { z: 0, nom: "Couverture", places: false }];
  const T = F.length - 1, H = ventile ? T - 1 : T;

  // ---- Escaliers : le plan du noyau tel que le squelette le dessine (zones, volées, portes, baie du sas), dans le
  // repère local de la rangée (a le long de la rangée, v depuis l'allée). La cage (paliers et volées) et les gaines
  // (ascenseurs, gaine technique) sont ouvertes à chaque plancher.
  const M = P.marches, boiteR = (rs) => (rs.length ? [Math.min(...rs.map((r) => r[0])), Math.min(...rs.map((r) => r[1])), Math.max(...rs.map((r) => r[2])), Math.max(...rs.map((r) => r[3]))] : null);
  const cages = P.escaliers.filter((x) => !x.hors).map((x) => {
    const W = (a, v) => [x.O[0] + x.ua[0] * a + x.uv[0] * v, x.O[1] + x.ua[1] * a + x.uv[1] * v];
    const R = (r) => oriente([W(r[0], r[1]), W(r[2], r[1]), W(r[2], r[3]), W(r[0], r[3])], true);
    const pl = x.plan || { zones: [], volees: [], portes: [x.L / 2], baie: null }, paliers = pl.zones.filter((z) => z.nom === "palier").map((z) => z.r);
    const cage = boiteR([...paliers, ...pl.volees.map((v) => v.r)]), puits = pl.zones.filter((z) => z.nom === "ascenseur" || z.nom === "gaine").map((z) => z.r);
    return { x, R, pl, paliers, cage, puits, tremie: [...(cage ? [R(cage)] : []), ...puits.map(R)] };
  });

  // ---- Enveloppe : réunion des chaussées, des rangées, des disques d'hélice et des noyaux. Trémies.
  const env = O.reunion([...P.rangees, ...P.chaussees.map((c) => c.q), ...P.helices.map((h) => cercle(h.p, h.ext + RIVE.e)), ...cages.map((c) => c.x.corps)]);
  if (!env.length) return sc;
  const tremies = [...P.rampes.filter((r) => !r.hors).map((r) => r.q), ...P.rampesVoie.map((r) => r.q), ...P.helices.map((h) => cercle(h.p, h.ext)), ...cages.flatMap((c) => c.tremie)];
  const bTrem = tremies.map((r) => boite([r]));
  const [x0, y0, x1, y1] = boite(env.map((pg) => pg[0]));
  sc.min = [x0, y0]; sc.max = [x1, y1];

  // ---- Dalles : au plancher bas, dallage (ventilé) ou radier (enterré), plein ; au-dessus, dalle percée des trémies.
  vols(ventile ? "dallage" : "radier", env, F[0].z - (ventile ? DALLAGE : RADIER), F[0].z, 0);
  const percee = O.moins(env, tremies);
  for (let f = 1; f <= T; f++) vols(f === T ? "couverture" : "dalle", f <= H ? percee : env, F[f].z - P.dalle, F[f].z, f);

  // ---- Sols peints : chaussées, places, piétons. Au-dessus du plancher bas, ils s'arrêtent aux trémies.
  const decoupe = (r) => { const b = boite([r]), t = tremies.filter((_, k) => touche(b, bTrem[k])); return t.length ? O.moins([[oriente(r, true)]], t) : [[oriente(r, true)]]; };
  const peints = (etage) => {
    const l = [], pose = (rs, matiere, dz) => { for (const r of rs) for (const pg of etage ? decoupe(r) : [[oriente(r, true)]]) l.push({ anneaux: pg, matiere, dz }); };
    pose(P.chaussees.filter((c) => !c.rampe).map((c) => c.q), "enrobe", 0.004);
    pose(P.places.filter((p) => !p.rouge).map((p) => p.q), "place", 0.006);
    pose(P.places.filter((p) => p.rouge).map((p) => p.q), "constat", 0.006);
    pose(P.pietons, "pieton", 0.008);
    pose(P.zebres, "zebre", 0.01);
    return l;
  };
  const sols = [peints(false), T > 0 ? peints(true) : []];
  for (let f = 0; f <= T; f++) {
    if (!F[f].places) continue;
    for (const s of sols[f ? 1 : 0]) sc.sols.push({ anneaux: s.anneaux, z: +(F[f].z + s.dz).toFixed(4), matiere: s.matiere, niveau: f });
    for (const p of P.places) { const q = p.q; for (const [a, b] of [[q[3], q[0]], [q[0], q[1]], [q[1], q[2]]]) sc.traits.push({ a, b, z: +(F[f].z + 0.02).toFixed(4), matiere: "marquage", niveau: f }); }
    sc.places += P.places.length;
  }
  sc.niveaux = F.map((x, f) => ({ z: x.z, nom: x.nom, places: x.places ? P.places.length : 0 }));

  // ---- Ossature, étage par étage : poteaux sous les poutres, poutres sous la dalle, voiles et remplissages.
  const cleP = (p) => `${p[0].toFixed(2)},${p[1].toFixed(2)}`, dirs = new Map();
  for (const b of P.poutres) { const u = unit(b.a, b.b); for (const p of [b.a, b.b]) if (!dirs.has(cleP(p))) dirs.set(cleP(p), u); }
  const carre = (p, s) => { const u = dirs.get(cleP(p)) || [1, 0], v = [-u[1], u[0]], h = s / 2; return oriente([[-1, -1], [1, -1], [1, 1], [-1, 1]].map(([i, j]) => [p[0] + (u[0] * i + v[0] * j) * h, p[1] + (u[1] * i + v[1] * j) * h]), true); };
  for (let f = 0; f < T; f++) {
    const z = F[f].z, sous = F[f + 1].z - P.dalle, sp = sous - P.retombee;
    for (const o of P.poteaux) vol(o.conflit ? "poteau_constat" : "poteau", [carre(o.p, P.section)], z, sp, f, o.cle);
    for (const b of P.poutres) {
      if (b.noyau) continue; // les murs des noyaux viennent des escaliers
      if (b.voile) { vol("voile", [bande(b.a, b.b, P.section)], z, sous, f); continue; }
      if (P.retombee > 0) vol("poutre", [bande(b.a, b.b, Math.max(P.poutre, 0.2))], sp, sous, f);
      if (b.plein) vol("maconnerie", [bande(b.a, b.b, MUR)], z, sp, f);
    }
  }

  // ---- Rampes : une dalle inclinée et ses deux rives par étage ; au dernier plancher desservi, un garde-corps ferme le bas de la trémie.
  const rampe = (q, H, genre, raide, cle, f) => {
    const a = milieu(q[0], q[3]), b = milieu(q[1], q[2]), w = dist(q[0], q[3]), z = F[f].z, axe = [[a[0], a[1], z], [b[0], b[1], z + H]];
    sc.rubans.push({ axe, largeur: w, decalage: 0, z0: -P.dalle, z1: 0, matiere: raide ? "rampe_constat" : "rampe", niveau: f, cle });
    for (const s of [1, -1]) sc.rubans.push({ axe, largeur: RIVE.e, decalage: s * (w / 2 - RIVE.e / 2), z0: 0, z1: RIVE.h, matiere: "rive", niveau: f, cle });
    sc.rampes.push({ axe, largeur: w, niveau: f, cle, genre, raide });
  };
  const rampes = [...P.rampes.filter((r) => !r.hors).map((r) => ({ ...r, genre: "posee" })), ...P.rampesVoie.map((r) => ({ ...r, genre: "branchee" }))];
  for (const r of rampes) {
    for (let f = 0; f < H; f++) rampe(r.q, r.hauteur, r.genre, r.raide, r.cle, f);
    if (H > 0) vol("allege", [bande(r.q[0], r.q[3], RIVE.e)], F[H].z, F[H].z + ALLEGE, H, r.cle);
    if (r.hauteur < E - 1e-6) sc.constats.push(`${r.cle} : rampe de demi-niveau, les plateaux décalés ne sont pas représentés`);
  }
  if (P.rampes.some((r) => r.hors)) sc.constats.push("Rampe à replacer : non représentée");

  // ---- Rampes hélicoïdales : un tour par étage, dans le sens trigonométrique vu du dessus. L'entrée est du côté de la
  // voie que le disque recouvre. Mur intérieur, rive extérieure ouverte au droit de l'entrée, garde-corps au bord de chaque plancher.
  for (const h of P.helices) {
    const disque = cercle(h.p, h.ext);
    let cx = 0, cy = 0, s = 0;
    for (const pg of O.inter([[disque]], P.chaussees.filter((c) => !c.rampe).map((c) => c.q))) { const r = pg[0], a = Math.abs(aire(r)), c = r.reduce((m, p) => [m[0] + p[0] / r.length, m[1] + p[1] / r.length], [0, 0]); cx += c[0] * a; cy += c[1] * a; s += a; }
    const t0 = s > 0 ? Math.atan2(cy / s - h.p[1], cx / s - h.p[0]) : -Math.PI / 2, rm = (h.int + h.ext) / 2, w = h.ext - h.int, gap = Math.min(1.2, 7 / h.ext);
    const spirale = (z, a0, a1, k = 48) => Array.from({ length: k + 1 }, (_, i) => { const t = a0 + ((a1 - a0) * i) / k; return [h.p[0] + rm * Math.cos(t), h.p[1] + rm * Math.sin(t), z + (E * (t - t0)) / (2 * Math.PI)]; });
    const anneau = (r0, r1, a0, a1, k = 48) => { const ext = Array.from({ length: k + 1 }, (_, i) => a0 + ((a1 - a0) * i) / k); return oriente([...ext.map((t) => [h.p[0] + r1 * Math.cos(t), h.p[1] + r1 * Math.sin(t)]), ...ext.reverse().map((t) => [h.p[0] + r0 * Math.cos(t), h.p[1] + r0 * Math.sin(t)])], true); };
    for (let f = 0; f < H; f++) {
      const z = F[f].z, axe = spirale(z, t0, t0 + 2 * Math.PI);
      sc.rubans.push({ axe, largeur: w, decalage: 0, z0: -P.dalle, z1: 0, matiere: h.raide ? "rampe_constat" : "rampe", niveau: f, cle: h.cle });
      sc.rubans.push({ axe: spirale(z, t0 + gap, t0 + 2 * Math.PI - gap), largeur: RIVE.e, decalage: -(w / 2 - RIVE.e / 2), z0: 0, z1: RIVE.h, matiere: "rive", niveau: f, cle: h.cle });
      vol("noyau", [oriente(cercle(h.p, h.int), true), oriente(cercle(h.p, h.int - MUR), false)], z, F[f + 1].z, f, h.cle);
      sc.rampes.push({ axe, largeur: w, niveau: f, cle: h.cle, genre: "helice", raide: h.raide });
    }
    for (let f = 1; f <= H; f++) vol("allege", [anneau(h.ext, h.ext + RIVE.e, t0 + gap, t0 + 2 * Math.PI - gap)], F[f].z, F[f].z + ALLEGE, f, h.cle);
    vol("noyau", [oriente(cercle(h.p, h.int), true), oriente(cercle(h.p, h.int - MUR), false)], F[H].z, F[H].z + ALLEGE, H, h.cle);
    if (!h.raccord) sc.constats.push(`${h.cle} : rampe hélicoïdale sans voie à raccorder`);
  }

  // ---- Escaliers : murs du noyau (le noyau moins ses espaces libres), portes et baie du sas percées sur 2,10 m ; à
  // chaque étage, la première volée part du palier d'étage, la seconde du palier intermédiaire, une marche par giron ;
  // ce qui sépare une volée d'un palier, dans sa largeur, est palier. En silo, la cage monte jusqu'à la toiture ; en
  // enterré, elle sort du sol en édicule.
  for (const c of cages) {
    const { x, R, pl, paliers, cage, puits } = c, e = M.mur, g = M.giron;
    const murs = O.moins([[R([0, 0, x.L, x.prof])]], [...pl.zones.map((z) => R(z.r)), ...pl.volees.map((v) => R(v.r)), ...(cage ? [R(cage)] : [])]);
    const baies = pl.portes.map((a) => R([a - PORTE.l / 2, -0.01, a + PORTE.l / 2, e + 0.01]));
    if (pl.baie) { const [a0, v0, a1, v1] = pl.baie; baies.push(R(Math.abs(a1 - a0) < 1e-6 ? [a0 - e / 2 - 0.01, Math.min(v0, v1), a0 + e / 2 + 0.01, Math.max(v0, v1)] : [Math.min(a0, a1), v0 - e / 2 - 0.01, Math.max(a0, a1), v0 + e / 2 + 0.01])); }
    const bas = O.moins(murs, baies);
    const enceinte = (z0, z1, f) => { const zp = Math.min(z1, z0 + PORTE.h); vols("noyau", bas, z0, zp, f, x.cle); vols("noyau", murs, zp, z1, f, x.cle); };
    const [V1, V2] = pl.volees, axe = (v) => (v.sens[0] ? 0 : 1), signe = (v) => v.sens[0] + v.sens[1];
    const debut = (v) => (signe(v) > 0 ? v.r[axe(v)] : v.r[axe(v) + 2]), bout = (v) => (signe(v) > 0 ? v.r[axe(v) + 2] : v.r[axe(v)]);
    const ecart = (p, k, t) => Math.max(0, p[k] - t, t - p[k + 2]);
    const joint = (v, t, p) => { if (!v || !p) return null; const k = axe(v), lo = t < p[k] ? t : p[k + 2], hi = t < p[k] ? p[k] : t; if (hi - lo < 0.01) return null; return k ? [v.r[0], lo, v.r[2], hi] : [lo, v.r[1], hi, v.r[3]]; };
    const pEt = V1 ? [...paliers].sort((p, q) => ecart(p, axe(V1), debut(V1)) - ecart(q, axe(V1), debut(V1)))[0] : null, pIn = paliers.find((p) => p !== pEt);
    const marches = (v, z0, f) => { const k = axe(v), s = signe(v), d = debut(v); for (let j = 1; j < v.n; j++) { const a = d + s * (j - 1) * g, b = d + s * j * g, r = k ? [v.r[0], Math.min(a, b), v.r[2], Math.max(a, b)] : [Math.min(a, b), v.r[1], Math.max(a, b), v.r[3]], t = z0 + j * M.h; vol("marche", [R(r)], Math.max(z0, t - MARCHE), t, f, x.cle); } };
    const dalle = (r, z, f) => { if (r) vol("palier", [R(r)], z - PALIER, z, f, x.cle); };
    for (let f = 0; f < T; f++) {
      const z = F[f].z, zh = F[f + 1].z, zi = z + M.n1 * M.h;
      enceinte(z, zh, f);
      if (f >= H) continue; // sous la toiture du silo : la cage, sans escalier
      if (f > 0) dalle(joint(V1, debut(V1), pEt), z, f);
      if (V1) marches(V1, z, f);
      dalle(pIn, zi, f); dalle(joint(V1, bout(V1), pIn), zi, f); dalle(joint(V2, debut(V2), pIn), zi, f);
      if (V2) marches(V2, zi, f);
      dalle(pEt, zh, f + 1); dalle(joint(V2, bout(V2), pEt), zh, f + 1); // palier d'étage du plancher du dessus, ouvert sur la cage
    }
    if (!ventile) { enceinte(F[T].z, F[T].z + EDICULE, T); vol("noyau", [R([0, 0, x.L, x.prof])], F[T].z + EDICULE, F[T].z + EDICULE + PALIER, T, x.cle); }
    if (!x.plan) sc.constats.push(`${x.cle} : plan du noyau introuvable, cage montée sans escalier`);
  }

  // ---- Façades et terrain. Le long de l'enveloppe : allèges à chaque plancher de stationnement hors rez et acrotère
  // de la toiture (ventilé), voiles contre terre (enterré). Une bande par côté, rallongée aux deux bouts puis rognée par l'enveloppe : les angles rentrants se ferment.
  const bord = (e, dehors) => {
    const rs = [];
    for (const pg of env) for (const r of pg) r.forEach((a, k) => { const b = r[(k + 1) % r.length]; if (dist(a, b) > 0.05) rs.push(bande(a, b, e, dehors ? -1 : 1, e)); });
    const U = O.reunion(rs);
    return dehors ? O.dehors(U, env) : O.dedans(U, env);
  };
  const cadre = (m) => oriente([[x0 - m, y0 - m], [x1 + m, y0 - m], [x1 + m, y1 + m], [x0 - m, y1 + m]], true);
  if (ventile) {
    const al = bord(MUR, false);
    for (let f = 1; f <= H; f++) vols("allege", al, F[f].z, F[f].z + ALLEGE, f);
    vols("acrotere", al, F[T].z, F[T].z + ACROTERE, T);
    vol("terrain", [cadre(MARGE)], -DALLAGE - SOCLE, -DALLAGE, -1);
  } else {
    const mu = bord(MUR_ENTERRE, true);
    for (let f = 0; f < T; f++) vols("voile", mu, F[f].z - (f ? 0 : RADIER), F[f + 1].z, f);
    vols("terrain", O.dehors([[cadre(MARGE)]], O.reunion([...env.map((pg) => pg[0]), ...mu.map((pg) => pg[0])])), F[0].z - RADIER - SOCLE, 0, -1);
  }

  if (O.echecs.length) sc.constats.push(`Polygones : ${O.echecs.length} opération(s) en échec, remplacée(s) par une forme approchée`);
  let haut = -Infinity, bas = Infinity;
  for (const v of sc.volumes) { haut = Math.max(haut, v.z1); bas = Math.min(bas, v.z0); }
  for (const r of sc.rubans) for (const p of r.axe) { haut = Math.max(haut, p[2] + r.z1); bas = Math.min(bas, p[2] + r.z0); }
  sc.haut = haut; sc.bas = bas;
  return sc;
}
