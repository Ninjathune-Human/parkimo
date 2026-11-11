// ============================================================================
// Vue : monde (m) ↔ écran (px CSS).
// ============================================================================
const $ = (s) => document.querySelector(s);
const canvas = $("#plan"), ctx = canvas.getContext("2d");
let W = 0, H = 0, DPR = 1;
const view = { s: 4, ox: 0, oy: 0 };
const toS = (p) => [p[0] * view.s + view.ox, -p[1] * view.s + view.oy];
const toW = (x, y) => [(x - view.ox) / view.s, (view.oy - y) / view.s];

// Taille de la trame du canvas. Sur iPhone et iPad, l'événement resize d'une rotation peut arriver
// avant la nouvelle mise en page : la taille est donc suivie par ResizeObserver, qui la mesure après.
function resize(cw, ch) {
  DPR = Math.min(window.devicePixelRatio || 1, 2.5);
  const w0 = W, h0 = H;
  W = cw !== undefined ? cw : canvas.clientWidth; H = ch !== undefined ? ch : canvas.clientHeight;
  // Le point du plan au centre de l'écran y reste (rotation, fenêtre redimensionnée).
  if (w0 && h0) { view.ox += (W - w0) / 2; view.oy += (H - h0) / 2; }
  const w = Math.round(W * DPR), h = Math.round(H * DPR);
  if (canvas.width !== w || canvas.height !== h) { canvas.width = w; canvas.height = h; }
  placeSide();
  redraw();
}
// Barre d'outils centrée dans la zone libre à droite de la colonne ; si elle n'y tient pas, au milieu de
// l'écran, et la colonne s'arrête au-dessus d'elle plutôt que de la chevaucher.
function placeSide() {
  const S = $("#side"), T = $("#toolbar"), root = document.documentElement.style;
  S.classList.remove("lift"); root.removeProperty("--cx");
  T.classList.remove("compact");
  if (W < 760) return;
  if (T.scrollWidth > W - 32) T.classList.add("compact");
  const x0 = S.getBoundingClientRect().right - canvas.getBoundingClientRect().left + 12, c = (x0 + W - 16) / 2;
  if (c - T.offsetWidth / 2 >= x0) { root.setProperty("--cx", c + "px"); return; }
  if (T.getBoundingClientRect().left < x0 - 4) S.classList.add("lift");
}
// Zones travaillées : parcelle d'étude, bâtiments, espaces verts, servitudes, accès.
// Sans parcelle, l'emprise du fond de carte importé.
function workBounds() {
  const pts = outline();
  for (const o of site.buildings) pts.push(...rectPoly(o));
  for (const e of [...site.exclusions, ...site.landscapes, ...(site.basins || [])]) pts.push(...polyOutline(e));
  if (layout && !layout.error && site.parcel.length >= 3) for (const q of layout.entries) pts.push(...q);
  if (!pts.length && baseShown()) { const r = basemap.rect; pts.push([r.x0, r.y0], [r.x1, r.y1]); }
  if (!pts.length) return null;
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
  for (const [x, y] of pts) { x0 = Math.min(x0, x); x1 = Math.max(x1, x); y0 = Math.min(y0, y); y1 = Math.max(y1, y); }
  return { x0, y0, x1, y1 };
}
// Plus grand rectangle de l'écran libre de tout panneau, choisi pour la forme des zones à cadrer.
function freeArea(bw, bh, m) {
  const cr = canvas.getBoundingClientRect();
  const obst = ["#stats", "#layers", "#toolbar", "#selbar", "#girmenu"].map((q) => $(q))
    .filter((el) => el && !el.hidden && el.getClientRects().length)
    .map((el) => { const r = el.getBoundingClientRect(); return [r.left - cr.left - 8, r.top - cr.top - 8, r.right - cr.left + 8, r.bottom - cr.top + 8]; });
  const xs0 = [0, ...obst.map((r) => r[2])], xs1 = [W, ...obst.map((r) => r[0])], ys0 = [0, ...obst.map((r) => r[3])], ys1 = [H, ...obst.map((r) => r[1])];
  let best = null;
  for (const a of xs0) for (const b of xs1) for (const c of ys0) for (const d of ys1) {
    if (b - a < 2 * m + 60 || d - c < 2 * m + 60) continue;
    if (obst.some((r) => r[0] < b && r[2] > a && r[1] < d && r[3] > c)) continue;
    const s = Math.min((b - a - 2 * m) / bw, (d - c - 2 * m) / bh);
    if (!best || s > best.s + 1e-9 || (Math.abs(s - best.s) < 1e-9 && (b - a) * (d - c) > (best.x1 - best.x0) * (best.y1 - best.y0))) best = { x0: a, y0: c, x1: b, y1: d, s };
  }
  // Écran trop encombré : tout l'écran.
  return best || { x0: 0, y0: 0, x1: W, y1: H, s: Math.min((W - 2 * m) / bw, (H - 2 * m) / bh) };
}
let viewAnim = 0;
function fit(animate) {
  const bb = workBounds(); if (!bb || !W || !H) return;
  const m = 28, bw = Math.max(1, bb.x1 - bb.x0), bh = Math.max(1, bb.y1 - bb.y0);
  const fa = freeArea(bw, bh, m), s = Math.max(0.5, Math.min(40, fa.s));
  const target = { s, ox: (fa.x0 + fa.x1) / 2 - ((bb.x0 + bb.x1) / 2) * s, oy: (fa.y0 + fa.y1) / 2 + ((bb.y0 + bb.y1) / 2) * s };
  cancelAnimationFrame(viewAnim);
  if (!animate || matchMedia("(prefers-reduced-motion: reduce)").matches) { Object.assign(view, target); redraw(); return; }
  // Transition : échelle interpolée en logarithme, centre de l'écran interpolé en coordonnées plan.
  const cx = W / 2, cy = H / 2, from = toW(cx, cy), to = [(cx - target.ox) / target.s, (target.oy - cy) / target.s];
  const s0 = Math.log(view.s), s1 = Math.log(target.s), t0 = performance.now(), D = 320;
  const step = (now) => {
    const k = Math.min(1, (now - t0) / D), e = 1 - Math.pow(1 - k, 3), sc = Math.exp(s0 + (s1 - s0) * e);
    const wx = from[0] + (to[0] - from[0]) * e, wy = from[1] + (to[1] - from[1]) * e;
    view.s = sc; view.ox = cx - wx * sc; view.oy = cy + wy * sc; redraw();
    if (k < 1) viewAnim = requestAnimationFrame(step);
  };
  viewAnim = requestAnimationFrame(step);
}

