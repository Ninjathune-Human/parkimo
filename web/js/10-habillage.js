// ============================================================================
// Habillage : véhicules dans les places, essences d'arbres. Symboles partagés avec l'export DXF
// (src/symbols.json, blocs AutoCAD du même nom). Réglage de présentation, hors historique.
// ============================================================================
const SYM = JSON.parse($("#symbols").textContent);
for (const s of [...SYM.vehicules, ...SYM.arbres, ...SYM.marquages]) for (const p of s.prims) {
  const path = new Path2D();
  if (p.c) path.arc(p.c[0], p.c[1], p.r, 0, Math.PI * 2);
  else { p.p.forEach(([x, y], i) => (i ? path.lineTo(x, y) : path.moveTo(x, y))); if (!p.o) path.closePath(); }
  p.path = path;
}
// Contour des houppiers : teinte de l'essence assombrie.
for (const s of SYM.arbres) s.edge = "#" + [1, 3, 5].map((i) => Math.round(parseInt(s.couleur.slice(i, i + 2), 16) * 0.55).toString(16).padStart(2, "0")).join("");
const TREE_MIX = { feuillu: 35, pin: 20, olivier: 20, palmier: 15, conifere: 10 };
// Teintes courantes du parc automobile : carrosserie, pavillon, part du tirage.
const CAR_COLORS = [["#F1F1EE", "#E2E2DE", 24], ["#B8BCBF", "#C9CCCE", 16], ["#7B8186", "#8B9196", 16], ["#2B2E31", "#3B3F43", 18],
  ["#33465C", "#40556D", 10], ["#8C3B35", "#9C4A43", 7], ["#C9BCA2", "#D6CBB5", 4], ["#41594B", "#4F6859", 5]];
let DRESS = { cars: 0, tree: "mix", seed: 1 };
try { DRESS = { ...DRESS, ...JSON.parse(localStorage.getItem("trame.dress") || "{}") }; } catch (e) { /* stockage indisponible */ }
function saveDress() { try { localStorage.setItem("trame.dress", JSON.stringify(DRESS)); } catch (e) { /* stockage indisponible */ } }

// Tirage stable : un nombre dans [0, 1[ par place (centre au décimètre), par usage k et par tirage.
function rnd(ix, iy, k) {
  let h = Math.imul(ix ^ 0x9E3779B9, 0x85EBCA6B) ^ Math.imul(iy + 0x632BE5AB, 0xC2B2AE35) ^ Math.imul(DRESS.seed * 16 + k, 0x27D4EB2F);
  h ^= h >>> 15; h = Math.imul(h, 0x2C1B3C6D); h ^= h >>> 12; h = Math.imul(h, 0x297A2D39); h ^= h >>> 15;
  return (h >>> 0) / 4294967296;
}
function pickWeighted(list, weight, u) {
  const total = list.reduce((a, x) => a + weight(x), 0); let t = u * total;
  for (const x of list) { t -= weight(x); if (t < 0) return x; }
  return list[list.length - 1];
}
let dressCache = { layout: null, key: "", v: null };
const NO_DRESS = { cars: [], trees: [], batch: null };
function dressing() {
  if (!layout || layout.error) return NO_DRESS;
  // Parc en ouvrage : véhicules sur les places du niveau affiché.
  const places = layout.ouvrage ? nivStalls() : layout.stalls;
  const key = `${DRESS.cars}|${DRESS.tree}|${DRESS.seed}|${layout.ouvrage ? nivActif() : ""}`;
  if (dressCache.layout === layout && dressCache.key === key) return dressCache.v;
  const cars = [], trees = [], rate = DRESS.cars / 100;
  for (const st of places) {
    const q = st.q, bx = (q[0][0] + q[1][0]) / 2, by = (q[0][1] + q[1][1]) / 2, fx = (q[2][0] + q[3][0]) / 2, fy = (q[2][1] + q[3][1]) / 2;
    const ix = Math.round((bx + fx) * 5), iy = Math.round((by + fy) * 5);
    if (rnd(ix, iy, 0) >= rate) continue;
    const depth = Math.hypot(fx - bx, fy - by), width = Math.hypot(q[1][0] - q[0][0], q[1][1] - q[0][1]);
    const fits = SYM.vehicules.filter((v) => v.L <= depth - 0.1 && v.W <= width - 0.25);
    if (!fits.length) continue;
    const v = pickWeighted(fits, (x) => x.poids, rnd(ix, iy, 1));
    const ux = (fx - bx) / depth, uy = (fy - by) / depth;
    // Entrée en marche avant le plus souvent : l'avant du véhicule vers le fond de la place.
    const dir = rnd(ix, iy, 2) < 0.8 ? -1 : 1;
    // Recul de 0,15 à 0,40 m depuis le fond, sans jamais déborder sur l'allée.
    const free = depth - v.L, back = Math.min(0.15, free / 2);
    const c = back + rnd(ix, iy, 3) * Math.max(0, Math.min(0.25, free - 0.1 - back)) + v.L / 2;
    const lat = (rnd(ix, iy, 4) - 0.5) * Math.min(0.16, width - v.W - 0.3);
    const a = Math.atan2(uy * dir, ux * dir) * 180 / Math.PI + (rnd(ix, iy, 5) - 0.5) * 3;
    cars.push({ v, x: bx + ux * c - uy * lat, y: by + uy * c + ux * lat, a, col: pickWeighted(CAR_COLORS, (x) => x[2], rnd(ix, iy, 6)) });
  }
  const plant = (x, y) => {
    const ix = Math.round(x * 10), iy = Math.round(y * 10);
    const id = DRESS.tree === "mix" ? pickWeighted(Object.keys(TREE_MIX), (k) => TREE_MIX[k], rnd(ix, iy, 7)) : DRESS.tree;
    const sp = SYM.arbres.find((t) => t.id === id) || SYM.arbres[0];
    trees.push({ sp, x, y, a: rnd(ix, iy, 8) * 360, s: sp.r * (0.9 + 0.2 * rnd(ix, iy, 9)) });
  };
  for (const t of layout.trees) plant(t[0], t[1]);
  // Espaces verts imposés : plantation en quinconce, à 1 m au moins du bord, essence de l'habillage.
  const one = SYM.arbres.find((t) => t.id === DRESS.tree), step = one ? Math.max(3, 2.1 * one.r) : 5.5;
  for (const poly of site.landscapes.map(polyOutline)) {
    let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
    for (const [x, y] of poly) { x0 = Math.min(x0, x); x1 = Math.max(x1, x); y0 = Math.min(y0, y); y1 = Math.max(y1, y); }
    const inside = (p) => pointInPoly(p, poly) && poly.every((a, k) => distSeg(p, a, poly[(k + 1) % poly.length])[0] >= 1);
    let n = 0, row = 0;
    for (let y = y0 + step / 2; y < y1; y += step * 0.866, row++) for (let x = x0 + step / 2 + (row % 2) * step / 2; x < x1; x += step) {
      const ix = Math.round(x * 10), iy = Math.round(y * 10);
      const p = [x + (rnd(ix, iy, 10) - 0.5) * step * 0.25, y + (rnd(ix, iy, 11) - 0.5) * step * 0.25];
      if (inside(p)) { plant(p[0], p[1]); n++; }
    }
    if (!n) { const c = [poly.reduce((a, q) => a + q[0], 0) / poly.length, poly.reduce((a, q) => a + q[1], 0) / poly.length]; if (pointInPoly(c, poly)) plant(c[0], c[1]); }
  }
  dressCache = { layout, key, v: { cars, trees, batch: batchOf(cars, trees) } };
  return dressCache.v;
}

// Lot de symboles : un Path2D par teinte et par rôle, en coordonnées du plan. Quelques remplissages
// suffisent à dessiner des centaines de véhicules et d'arbres, à toute échelle.
function batchOf(cars, trees) {
  const b = { shadow: new Path2D(), glass: new Path2D(), line: new Path2D(), colors: new Map(), trees: new Map() };
  const place = (o) => { const a = o.a * Math.PI / 180, k = o.s || 1; return new DOMMatrix([k * Math.cos(a), k * Math.sin(a), -k * Math.sin(a), k * Math.cos(a), o.x, o.y]); };
  for (const c of cars) {
    const m = place(c);
    if (!b.colors.has(c.col)) b.colors.set(c.col, { body: new Path2D(), roof: new Path2D() });
    const g = b.colors.get(c.col);
    for (const p of c.v.prims) {
      if (p.k === "body") b.shadow.addPath(p.path, m);
      (p.k === "body" || p.k === "mirror" ? g.body : p.k === "roof" ? g.roof : p.k === "glass" ? b.glass : b.line).addPath(p.path, m);
    }
  }
  for (const t of trees) {
    const m = place(t);
    if (!b.trees.has(t.sp)) b.trees.set(t.sp, { crown: new Path2D(), line: new Path2D(), trunk: new Path2D() });
    const g = b.trees.get(t.sp);
    for (const p of t.sp.prims) g[p.k].addPath(p.path, m);
  }
  return b;
}
const TRUNK = "#5A4632";
// Flèches au sol : un seul Path2D par plan calculé.
let arrowsCache = { layout: null, path: null };
function arrowsPath(lay) {
  if (arrowsCache.layout === lay) return arrowsCache.path;
  const path = new Path2D(), sym = SYM.marquages[0].prims[0].path;
  for (const a of lay.circulation.arrows) { const r = a.a * Math.PI / 180; path.addPath(sym, new DOMMatrix([Math.cos(r), Math.sin(r), -Math.sin(r), Math.cos(r), a.p[0], a.p[1]])); }
  arrowsCache = { layout: lay, path };
  return path;
}
// px : un pixel écran en unités du plan.
function paintCars(g, b, px) {
  for (const [col, p] of b.colors) {
    g.fillStyle = col[0]; g.fill(p.body); g.strokeStyle = "rgba(0,0,0,.42)"; g.lineWidth = 0.7 * px; g.stroke(p.body);
    g.fillStyle = col[1]; g.fill(p.roof);
  }
  g.fillStyle = "#26323C"; g.fill(b.glass);
  g.strokeStyle = "rgba(0,0,0,.32)"; g.lineWidth = 0.7 * px; g.stroke(b.line);
}
function paintTrees(g, b, px) {
  for (const [sp, p] of b.trees) {
    g.globalAlpha = 0.93; g.fillStyle = sp.couleur; g.fill(p.crown);
    g.globalAlpha = 1; g.strokeStyle = sp.edge; g.lineWidth = 0.8 * px; g.stroke(p.crown);
    g.globalAlpha = 0.6; g.lineWidth = 0.7 * px; g.stroke(p.line);
    g.globalAlpha = 1; g.fillStyle = TRUNK; g.fill(p.trunk);
  }
}
// Repère du plan sur le canvas, décalé de (dx, dy) pixels.
function planTransform(g, r, s, ox, oy) { g.setTransform(r * s, 0, 0, -r * s, r * ox, r * oy); }
function drawDressing() {
  const d = dressing(), b = d.batch;
  if (!b) return;
  if (VIS.vehicules && d.cars.length && view.s > 0.6) {
    // Ombre portée commune (lumière en haut à gauche), puis véhicules.
    const sh = Math.max(0.6, 0.14 * view.s);
    planTransform(ctx, DPR, view.s, view.ox + sh, view.oy + sh); ctx.fillStyle = "rgba(0,0,0,.2)"; ctx.fill(b.shadow);
    planTransform(ctx, DPR, view.s, view.ox, view.oy); paintCars(ctx, b, 1 / view.s);
  }
  if (VIS.arbres && view.s > 1.2 && d.trees.length) { planTransform(ctx, DPR, view.s, view.ox, view.oy); paintTrees(ctx, b, 1 / view.s); }
  ctx.setTransform(DPR, 0, 0, DPR, 0, 0);
}

// ---- Menu Habillage
function thumb(cv, w, h) {
  const r = Math.min(window.devicePixelRatio || 1, 2.5);
  cv.width = Math.round(w * r); cv.height = Math.round(h * r);
  const g = cv.getContext("2d"); g.clearRect(0, 0, cv.width, cv.height);
  return [g, r];
}
function drawThumbs() {
  // Gabarits debout, avant vers le haut, alignés en pied ; 1 unité = 1 m.
  const cv = $("#d-veh"), w = cv.clientWidth || 300, h = 52, [g, r] = thumb(cv, w, h);
  const n = SYM.vehicules.length, cell = w / n, k = Math.min((h - 4) / 4.9, cell / 2.6), tones = [0, 4, 1, 5, 3, 2];
  const cars = SYM.vehicules.map((v, i) => ({ v, x: (cell * (i + 0.5)) / k, y: -(h - 2) / k + v.L / 2, a: 90, col: CAR_COLORS[tones[i]] }));
  planTransform(g, r, k, 0, 0); paintCars(g, batchOf(cars, []), 1 / k);
  const by = (q) => SYM.arbres.find((s) => s.id === q);
  document.querySelectorAll("#d-trees canvas").forEach((c) => {
    const [tg, tr] = thumb(c, 40, 40), id = c.dataset.sp;
    const ts = id === "mix"
      ? [{ sp: by("olivier"), x: 13, y: -14, a: 20, s: 10 }, { sp: by("palmier"), x: 27, y: -17, a: 0, s: 11 }, { sp: by("feuillu"), x: 18, y: -28, a: 40, s: 11 }]
      : [{ sp: by(id), x: 20, y: -20, a: 0, s: 18 }];
    planTransform(tg, tr, 1, 0, 0); paintTrees(tg, batchOf([], ts), 1);
  });
}
function syncDressUI() {
  const n = dressing().cars.length;
  $("#d-cars").value = DRESS.cars;
  $("#d-cars-v").textContent = `${DRESS.cars} %` + (DRESS.cars && layout && !layout.error ? ` · ${fmtInt(n)} véhicule${n > 1 ? "s" : ""}` : "");
  document.querySelectorAll("#d-trees .sp").forEach((b) => b.setAttribute("aria-checked", String(b.dataset.sp === DRESS.tree)));
  requestAnimationFrame(drawThumbs);
}
$("#d-trees").innerHTML = [["mix", "Mélange"], ...SYM.arbres.map((s) => [s.id, s.nom])]
  .map(([id, nom]) => `<button class="sp" role="radio" aria-checked="false" data-sp="${id}"><canvas data-sp="${id}" aria-hidden="true"></canvas>${nom}</button>`).join("");
document.querySelectorAll("#d-trees .sp").forEach((b) => b.addEventListener("click", () => { DRESS.tree = b.dataset.sp; saveDress(); syncDressUI(); redraw(); }));
$("#d-cars").addEventListener("input", (ev) => { DRESS.cars = +ev.target.value; syncDressUI(); redraw(); });
$("#d-cars").addEventListener("change", saveDress);
$("#d-shuffle").addEventListener("click", () => { DRESS.seed = 1 + Math.floor(Math.random() * 99991); saveDress(); syncDressUI(); redraw(); });

// Exemples (menu Nouveau)
function syncExample() {
  $("#new-mine").hidden = exampleName !== "mine" && !mineSite;
  document.querySelectorAll("[data-ex]").forEach((b) => b.setAttribute("aria-checked", String(b.dataset.ex === exampleName)));
}
// Un exemple modifié devient « Mon site », sauf si un site calé sur un fond de carte est déjà mis de côté.
function markMine() { if (exampleName !== "mine" && !basemap) { exampleName = "mine"; mineSite = null; syncExample(); } }
function loadExample(v) {
  closeMenus();
  if (v === exampleName) return;
  if (exampleName === "mine") mineSite = JSON.parse(JSON.stringify(site));
  if (v === "mine") { if (!mineSite) return; site = mineSite; } else site = EXAMPLES[v]();
  exampleName = v; lastAngle = null; setSel(null); syncExample(); updateLayersUI(); solve(false); commit(); fit();
}
document.querySelectorAll("[data-ex]").forEach((b) => b.addEventListener("click", () => loadExample(b.dataset.ex)));

let toastTimer = null;
function toast(msg) { const t = $("#toast"); t.textContent = msg; t.classList.add("show"); clearTimeout(toastTimer); toastTimer = setTimeout(() => t.classList.remove("show"), 2200); }

