// ============================================================================
// Giration (moteur Rust) : épure tracée à la main, comme dans Covadis. Le véhicule suit la trajectoire
// (marche avant puis, après un point de rebroussement, marche arrière) ; le moteur rend les traces des
// huit flancs de pneus, l'enveloppe de carrosserie, la marge et les conflits typés (bordure, mur, limite).
// ============================================================================
const GIR0 = { profile: "habile", speed: 5, margin: 0.1, kerb: { name: "T2", visible_height: 0.16 } };
const GIR_PROFILES = {
  prudent: "Prudent : 90 % de l'angle, roues tournées à 6°/s en roulant, jamais à l'arrêt.",
  courant: "Courant : tout l'angle, roues tournées à 10°/s en roulant, à l'arrêt aux rebroussements.",
  habile: "Habile : tout l'angle, roues tournées à 15°/s en roulant (limite admissible), à l'arrêt aux rebroussements.",
};
let gir = null, girT = 0, girSel = -1, girPlay = 0;
let gdraft = { pts: [], cusps: [], reverse: false };
let gstroke = null, girSketchSeq = 0; // trait libre en attente de la trajectoire proposée
const girSnap = (w) => [snap(w[0], 0.1), snap(w[1], 0.1)];
const girSet = () => ({ ...GIR0, ...(site.gir || {}) });
function ensureGir() { if (!site.gir) site.gir = { ...GIR0, kerb: { ...GIR0.kerb }, pts: [], cusps: [], reverse: false }; return site.gir; }
// Points de rebroussement → tronçons, marche avant et arrière en alternance.
function girSections(g) {
  const n = g.pts.length, out = [];
  if (n < 2) return out;
  const cuts = [0, ...g.cusps.filter((i) => i > 0 && i < n - 1), n - 1];
  for (let i = 0; i + 1 < cuts.length; i++) out.push({ pts: girDense(g.pts, cuts[i], cuts[i + 1]), reverse: (i % 2 === 1) !== !!g.reverse, speed_kmh: g.speed });
  return out;
}
// Le point [x, y, k] porte la flèche k du segment qui en part (0 : droit ; positive à droite du sens
// de tracé), comme les sommets des servitudes ; le dernier point n'en porte pas. Points des segments
// i0 à i1, arcs discrétisés (un côté tous les 4° environ) : moteur, tracé, export.
function girDense(P, i0, i1) {
  const out = [[P[i0][0], P[i0][1]]];
  for (let i = i0; i < i1; i++) { const e = edgeG(P, i, vk(P[i]), 1); for (let j = 1; j <= e.N; j++) { const q = e.at(j / e.N); out.push([q[0], q[1]]); } }
  return out;
}
// Sens de marche de chaque segment ↔ points de rebroussement : un point ajouté ou retiré ne change
// le sens d'aucun autre segment.
function girDirs(g) { const out = []; let r = !!g.reverse; for (let i = 0; i < g.pts.length - 1; i++) { if (i > 0 && g.cusps.includes(i)) r = !r; out.push(r); } return out; }
function setGirDirs(g, d) { g.reverse = !!d[0]; g.cusps = []; for (let i = 1; i < d.length; i++) if (d[i] !== d[i - 1]) g.cusps.push(i); }
function girEpure(g) { const c = { ...GIR0, ...g }; return { sections: girSections(c), profile: c.profile, margin: c.margin, kerb: c.kerb }; }
function girRun() {
  places.open = -1;
  const g = site.gir, sec = g ? girSections(g) : [];
  gir = null;
  if (Engine.ex && sec.length && layout && !layout.error) {
    let out;
    try { out = Engine.call("giration_json", { site: engineInput(false), ...girEpure(g), layout: girLayout() }); } catch (e) { out = { error: String(e) }; }
    if (out.error) toast("Giration impossible : " + out.error); else gir = out;
  }
  if (gir) { girT = Math.min(girT, gir.time); if (girSel >= gir.conflicts.length) girSel = -1; } else stopGirPlay();
  updateGirUI(); updateLayersUI(); redraw();
}
// Le plan affiché est transmis tel quel : le moteur le contrôle sans le recalculer.
function girLayout() { return { paved: layout.paved, pads: layout.pads, stalls: layout.stalls.map((st) => st.q), islands: layout.islands, entries: layout.entries, cross: layout.cross, walks: layout.walks || [] }; }
// Pose interpolée au temps t : [x, y, cap, angle bicyclette, distance, temps, tronçon].
function girPoseAt(t) {
  const P = gir.poses; let lo = 0, hi = P.length - 1;
  if (t <= P[0][5]) return P[0];
  if (t >= P[hi][5]) return P[hi];
  while (hi - lo > 1) { const m = (lo + hi) >> 1; if (P[m][5] <= t) lo = m; else hi = m; }
  const a = P[lo], b = P[hi], k = (t - a[5]) / ((b[5] - a[5]) || 1), dh = Math.atan2(Math.sin(b[2] - a[2]), Math.cos(b[2] - a[2]));
  return [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + dh * k, a[3] + (b[3] - a[3]) * k, a[4] + (b[4] - a[4]) * k, t, a[6]];
}
const girTimeAt = (s) => { const q = gir.poses.find((p) => p[4] >= s - 1e-6); return q ? q[5] : gir.time; };

// ---- Rendu
function girRings(polys, fill) {
  ctx.beginPath(); for (const poly of polys) for (const r of poly) pathPoly(r);
  if (fill) { ctx.fillStyle = fill; ctx.fill("evenodd"); }
}
function strokeLine(pts) { ctx.beginPath(); pts.forEach((v, i) => { const q = toS(v); i ? ctx.lineTo(q[0], q[1]) : ctx.moveTo(q[0], q[1]); }); ctx.stroke(); }
// Trajectoire guide : trait plein en marche avant, tirets en marche arrière, rond aux rebroussements.
function drawGirPath(g, hover) {
  const secs = girSections(hover ? { ...g, pts: [...g.pts, hover] } : g);
  ctx.lineWidth = 1.4; ctx.strokeStyle = C.label; ctx.lineCap = "round"; ctx.lineJoin = "round";
  for (const sc of secs) { ctx.setLineDash(sc.reverse ? [6, 5] : []); strokeLine(sc.pts); }
  ctx.setLineDash([]);
  if (secs.length) {
    const s0 = secs[0].pts, a = toS(s0[0]), b = s0.map(toS).find((q) => Math.hypot(q[0] - a[0], q[1] - a[1]) > 24);
    if (b) { const L = Math.hypot(b[0] - a[0], b[1] - a[1]); arrow(a[0] + (b[0] - a[0]) * 12 / L, a[1] + (b[1] - a[1]) * 12 / L, (b[0] - a[0]) / L, (b[1] - a[1]) / L, 6, C.label, 1.6); }
  }
  g.pts.forEach((v, k) => { const q = toS(v); if (g.cusps.includes(k)) { ctx.beginPath(); ctx.arc(q[0], q[1], 5, 0, Math.PI * 2); ctx.fillStyle = C.ground; ctx.fill(); ctx.strokeStyle = C.label; ctx.lineWidth = 1.6; ctx.stroke(); } else squareHandle(q, false, 6); });
}
// Véhicule à la pose p : caisse (seule prise en compte), quatre roues (épure de Jeantaud pour les roues avant).
function drawGirCar(p) {
  const v = gir.vehicle, c = Math.cos(p[2]), s = Math.sin(p[2]);
  const at = (f, l) => toS([p[0] + f * c - l * s, p[1] + f * s + l * c]);
  const quad = (pts) => { ctx.beginPath(); pts.forEach(([f, l], i) => { const q = at(f, l); i ? ctx.lineTo(q[0], q[1]) : ctx.moveTo(q[0], q[1]); }); ctx.closePath(); };
  const F = v.wheelbase + v.front_overhang, R = -v.rear_overhang, hw = v.width / 2;
  quad([[F, hw], [F, -hw], [R, -hw], [R, hw]]);
  ctx.fillStyle = C["glass-solid"]; ctx.globalAlpha = 0.9; ctx.fill(); ctx.globalAlpha = 1; ctx.strokeStyle = C.label; ctx.lineWidth = 1.3; ctx.stroke();
  quad([[F - 0.25, hw - 0.12], [F - 0.25, -hw + 0.12], [F - 0.9, -hw + 0.2], [F - 0.9, hw - 0.2]]); ctx.lineWidth = 1; ctx.stroke();
  const d = Math.abs(p[3]), E = v.wheelbase, T = v.track / 2;
  let left = 0, right = 0;
  if (d > 1e-6) { const r = E / Math.tan(d), inner = Math.atan(E / (r - T)), outer = Math.atan(E / (r + T)); [left, right] = p[3] > 0 ? [inner, outer] : [-outer, -inner]; }
  ctx.fillStyle = C.label;
  for (const [f, l, a] of [[E, T, left], [E, -T, right], [0, T, 0], [0, -T, 0]]) {
    const cx = p[0] + f * c - l * s, cy = p[1] + f * s + l * c, ca = Math.cos(p[2] + a), sa = Math.sin(p[2] + a), hl = 0.35, ht = v.tyre_width / 2;
    ctx.beginPath(); [[hl, ht], [hl, -ht], [-hl, -ht], [-hl, ht]].forEach(([u, w], i) => { const q = toS([cx + u * ca - w * sa, cy + u * sa + w * ca]); i ? ctx.lineTo(q[0], q[1]) : ctx.moveTo(q[0], q[1]); }); ctx.closePath(); ctx.fill();
  }
}
function drawGir() {
  if (mode === "gir") return;
  // Trajectoire tracée sans simulation (moteur en erreur) : toujours visible, donc éditable.
  if (!gir) { if (girShown()) drawGirPath(site.gir); return; }
  if (VIS.girenv) { girRings(gir.envelope, C["gir-body"]); ctx.strokeStyle = C["gir-body-edge"]; ctx.lineWidth = 1; ctx.stroke(); }
  if (VIS.girmarge && gir.margin_envelope.length) { ctx.setLineDash([5, 4]); girRings(gir.margin_envelope); ctx.strokeStyle = C.warn; ctx.lineWidth = 1.2; ctx.stroke(); ctx.setLineDash([]); }
  if (VIS.girroues) { ctx.strokeStyle = C["gir-wheel"]; ctx.lineWidth = 1; ctx.lineJoin = "round"; for (const t of gir.tracks) strokeLine(t); if (girShown()) drawGirPath(site.gir); }
  if (VIS.girenv) drawGirCar(girPoseAt(girT));
  if (VIS.girconf) gir.conflicts.forEach((k, i) => {
    const q = toS(k.point), r = Math.max(7, 0.3 * view.s) + (i === girSel ? 3 : 0), col = k.severity === "conflit" ? C.bad : C.warn;
    ctx.beginPath(); ctx.arc(q[0], q[1], r, 0, Math.PI * 2); ctx.fillStyle = col; ctx.globalAlpha = 0.18; ctx.fill(); ctx.globalAlpha = 1;
    ctx.strokeStyle = col; ctx.lineWidth = i === girSel ? 2.6 : 1.8; ctx.stroke();
  });
}
function drawGirDraft() {
  if (mode !== "gir") return;
  // Trait libre en cours (plein) ou en attente de la trajectoire proposée (tirets).
  const st = drag && drag.kind === "stroke" ? drag.stroke : gstroke;
  if (st && st.length > 1) { ctx.strokeStyle = C.accent; ctx.lineWidth = 2; ctx.lineCap = "round"; ctx.lineJoin = "round"; ctx.setLineDash(st === gstroke ? [5, 5] : []); strokeLine(st); ctx.setLineDash([]); }
  if (!gdraft.pts.length) return;
  drawGirPath(gdraft, hoverW);
  if (hoverW) { const a = gdraft.pts[gdraft.pts.length - 1], q = toS(hoverW); ctx.font = `600 11px ${getComputedStyle(document.body).fontFamily}`; ctx.fillStyle = C.label; ctx.textAlign = "left"; ctx.fillText(fmt1(Math.hypot(hoverW[0] - a[0], hoverW[1] - a[1])) + " m", q[0] + 12, q[1] - 10); }
}

// ---- Tracé : chaque toucher ajoute un point ; Avant / Arrière pose un point de rebroussement.
const girRev = (g) => (g.cusps.length % 2 === 1) !== g.reverse;
function girLen(pts) { let L = 0; for (let i = 1; i < pts.length; i++) L += Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]); return L; }
function syncGirBar() {
  const g = gdraft, r = girRev(g), n = g.pts.length;
  for (const b of $("#gb-dir").children) b.setAttribute("aria-pressed", String((b.dataset.r === "1") === r));
  $("#gb-dim").textContent = n ? `${n} point${n > 1 ? "s" : ""} · ${fmt1(girLen(g.pts))} m` : "Touchez le départ, ou dessinez d'un trait";
  $("#gb-ok").disabled = n < 2; updateUndo();
}
function onGirTap(w, dbl) {
  if (dbl && gdraft.pts.length >= 2) { finishGir(); return; }
  w = girSnap(w); const last = gdraft.pts[gdraft.pts.length - 1];
  if (last && Math.hypot(w[0] - last[0], w[1] - last[1]) < 0.3) return;
  gdraft.pts.push(w); syncGirBar(); redraw();
}
function girFlip() {
  const g = gdraft, n = g.pts.length, last = g.cusps.length ? g.cusps[g.cusps.length - 1] : 0;
  if (n - 1 > last) g.cusps.push(n - 1); // le tronçon a au moins un segment : rebroussement au dernier point
  else if (g.cusps.length) g.cusps.pop(); // rebroussement pas encore suivi d'un point : on le retire
  else g.reverse = !g.reverse; // aucun segment : sens du premier tronçon
  syncGirBar(); redraw();
}
function girPop() {
  const g = gdraft;
  if (g.cusps.length && g.cusps[g.cusps.length - 1] === g.pts.length - 1) g.cusps.pop(); else g.pts.pop();
  syncGirBar(); redraw();
}
function finishGir() {
  const g = gdraft, n = g.pts.length;
  if (n < 2) { toast("Placez au moins deux points"); return; }
  const set = ensureGir();
  Object.assign(set, { pts: g.pts.slice(), cusps: g.cusps.filter((i) => i < n - 1), reverse: g.reverse });
  setMode("edit"); girT = 0; girSel = -1; girRun(); commit(); markMine();
  if (gir) { const na = gir.conflicts.filter((k) => k.severity !== "conflit").length; toast(`${girCount()}${na ? `, ${na} alerte${na > 1 ? "s" : ""}` : ""} · ${fmt1(gir.length)} m · détail dans Giration`); }
}
// Trait libre : le moteur propose la trajectoire droite / arcs / droite la plus proche du trait,
// rayons faisables par le véhicule, sans contact si une recherche locale en trouve une. Le trait
// remplace les points déjà posés ; le sens de marche est celui du bouton Avant / Arrière.
async function girStroke(stroke) {
  if (!layout || layout.error) return;
  const g = girSet(), rev = girRev(gdraft), seq = ++girSketchSeq;
  gstroke = stroke; redraw(); toast("Recherche de la trajectoire…");
  const input = { site: engineInput(false), layout: girLayout(), stroke, reverse: rev, profile: g.profile, speed_kmh: g.speed, margin: g.margin, kerb: g.kerb };
  const out = await workerCall("giration_sketch_json", input, POOL + 1).catch((e) => ({ error: String(e) }));
  if (seq !== girSketchSeq || mode !== "gir") return; // tracé annulé ou remplacé entre-temps
  gstroke = null;
  if (out.error || !out.pts || out.pts.length < 2) { toast("Trajectoire impossible : " + (out.error || "trait trop court")); redraw(); return; }
  Object.assign(ensureGir(), { pts: out.pts.map(([x, y, k]) => withK(+x.toFixed(3), +y.toFixed(3), k)), cusps: [], reverse: rev });
  setMode("edit"); girT = 0; girSel = -1; girRun(); commit(); markMine(); setSel({ type: "gir" });
  const n = out.contacts;
  toast(`Trajectoire proposée · écart au trait ${fmt1(out.deviation)} m${n ? ` · ${n} contact${n > 1 ? "s" : ""} restant${n > 1 ? "s" : ""}` : out.searched ? " · obstacles évités" : ""}`);
}
$("#gb-dir").addEventListener("click", (ev) => { const b = ev.target.closest("button[data-r]"); if (b && (b.dataset.r === "1") !== girRev(gdraft)) girFlip(); });
$("#gb-ok").addEventListener("click", finishGir);
$("#gb-x").addEventListener("click", () => setMode("edit"));
$("#g-draw").addEventListener("click", () => { closeMenus(); stopGirPlay(); setSel(null); setMode("gir"); toast("Touchez le plan point par point, ou dessinez la trajectoire d'un trait"); });
$("#g-clear").addEventListener("click", girClear);

// ---- Édition de la trajectoire tracée : toucher le trait la sélectionne (segment touché), toucher un
// point le sélectionne ; boutons de segment au survol (point, déplacement, arc), poignée d'arc, panneau.
function girShown() { return places.open < 0 && VIS.girroues && !!site.gir && site.gir.pts.length >= 2; }
function girEditable() { return mode === "edit" && girShown(); }
const girDrag = (d) => d.kind === "gvert" || ((d.kind === "arc" || d.kind === "emove") && d.hit.ref.g === "gir");
const girEdge = (i) => edgeG(site.gir.pts, i, vk(site.gir.pts[i]), 1);
function girEdited() { girRun(); commit(); markMine(); updateSelbar(); }
// Rayon (m) du segment i, 0 s'il est droit ; longueur parcourue.
function girRadius(i) { const e = girEdge(i); if (!e.k) return 0; const s = Math.abs(e.k) * e.L; return (e.L * e.L / 4 + s * s) / (2 * s); }
function girSegLen(i) { const e = girEdge(i); return e.k ? girRadius(i) * 4 * Math.atan(2 * Math.abs(e.k)) : e.L; }
// Direction du tracé au début ou à la fin du segment i : la corde tournée de la demi-ouverture de l'arc.
function girTan(i, end) { const e = girEdge(i), a = (end ? 2 : -2) * Math.atan(2 * e.k), c = Math.cos(a), s = Math.sin(a); return [e.u[0] * c - e.u[1] * s, e.u[0] * s + e.u[1] * c]; }
// Cassure au point i entre les segments i − 1 et i (rad) ; au rebroussement, le tracé repart en sens inverse.
function girKink(i) {
  const d = girDirs(site.gir), f = d[i] !== d[i - 1] ? -1 : 1, a = girTan(i - 1, true), b = girTan(i, false);
  return Math.acos(Math.max(-1, Math.min(1, f * (a[0] * b[0] + a[1] * b[1]))));
}
// Flèche du segment i qui prolonge le segment i − 1 sans cassure (demi-cercle au plus).
function girTangentK(i) {
  const d = girDirs(site.gir), f = d[i] !== d[i - 1] ? -1 : 1, t = girTan(i - 1, true).map((x) => x * f), u = girEdge(i).u;
  const phi = Math.atan2(u[0] * t[1] - u[1] * t[0], u[0] * t[0] + u[1] * t[1]);
  return Math.max(-ARC_MAX, Math.min(ARC_MAX, -Math.tan(phi / 2) / 2));
}
// Rayon mini du point guidé par le moteur : milieu de l'essieu avant en marche avant, de l'essieu arrière en marche arrière.
function girRmin(rev) {
  if (!gir) return 0;
  const v = gir.vehicle, r = v.wheelbase / Math.tan(v.steer_deg * Math.PI / 180 * (girSet().profile === "prudent" ? 0.9 : 1));
  return rev ? r : Math.hypot(r, v.wheelbase);
}
// Segment i courbe (arc tangent au segment précédent, sinon flèche de 15 % de la corde) ou droit.
function girCurve(i, on) {
  const P = site.gir.pts; let k = 0;
  if (on) { k = i > 0 ? girTangentK(i) : 0; if (Math.abs(k) < 0.01) k = 0.15; }
  P[i] = withK(P[i][0], P[i][1], k); sel = { type: "gir", e: i };
  girEdited(); toast(on ? "Segment courbe · glissez le point ou saisissez le rayon" : "Segment droit");
}
function girSetRadius(i, R) {
  const P = site.gir.pts, e = girEdge(i);
  if (!(R > 0)) { updateSelbar(); return; }
  if (R < e.L / 2) toast(`Rayon mini de ce segment : ${fmt1(e.L / 2)} m (demi-cercle)`);
  const k = R <= e.L / 2 ? ARC_MAX : (R - Math.sqrt(R * R - e.L * e.L / 4)) / e.L;
  P[i] = withK(P[i][0], P[i][1], (Math.sign(e.k) || 1) * k); girEdited();
}
function insertGirPoint(i, t) {
  const g = site.gir, P = g.pts, d = girDirs(g), k = vk(P[i]), q = girEdge(i).at(t), [k1, k2] = k ? arcSplit(k, t) : [0, 0];
  hoverEdge = null; P[i] = withK(P[i][0], P[i][1], k1); P.splice(i + 1, 0, withK(snap(q[0], 0.01), snap(q[1], 0.01), k2));
  d.splice(i, 0, d[i]); setGirDirs(g, d);
  sel = { type: "gir", v: i + 1 }; girEdited(); toast("Point ajouté");
}
function removeGirPoint(k) {
  const g = site.gir, P = g.pts, n = P.length;
  if (n <= 2) { toast("Une trajectoire garde au moins deux points"); return; }
  const d = girDirs(g);
  if (k === 0) d.shift();
  else if (k === n - 1) d.pop();
  else { d.splice(k, 1); P[k - 1] = withK(P[k - 1][0], P[k - 1][1], arcJoin(vk(P[k - 1]), vk(P[k]))); } // les deux arcs n'en font qu'un
  P.splice(k, 1); P[n - 2] = [P[n - 2][0], P[n - 2][1]]; setGirDirs(g, d);
  hoverEdge = null; sel = { type: "gir" }; girEdited(); toast("Point supprimé");
}
function girClear() {
  const g = ensureGir(); g.pts = []; g.cusps = []; g.reverse = false;
  hoverEdge = null; sel = null; girEdited(); toast("Trajectoire effacée");
}
// Panneau de sélection : trajectoire, point, ou segment (droit ou courbe, rayon saisi, tangente).
function girSelbar() {
  const g = site.gir, P = g.pts, n = P.length;
  let what = "Trajectoire", dim = `${n} points · ${fmt1(P.slice(1).reduce((a, _, i) => a + girSegLen(i), 0))} m`;
  if (sel.v != null && P[sel.v]) { const v = P[sel.v]; what = `Point ${sel.v + 1}${g.cusps.includes(sel.v) ? " · rebroussement" : ""}`; dim = `${fmt1(v[0])} ; ${fmt1(v[1])}`; }
  else if (sel.e != null && sel.e < n - 1) {
    const i = sel.e, R = girRadius(i), rev = girDirs(g)[i], min = girRmin(rev), tight = R > 0 && R < min, r = $("#sel-r");
    what = `Segment ${i + 1}`; dim = `${fmt1(girSegLen(i))} m${rev ? " · marche arrière" : ""}${tight ? ` · rayon mini ${fmt1(min)} m` : ""}`;
    for (const b of $("#sel-arc").children) b.setAttribute("aria-pressed", String((b.dataset.c === "1") === (R > 0)));
    $("#sel-arc").hidden = false; $("#sel-rwrap").hidden = !R; $("#sel-tan").hidden = !i;
    if (document.activeElement !== r) r.value = R ? fmt1(R) : "";
    r.classList.toggle("bad", tight);
    r.title = tight ? `Trop serré pour le véhicule : ${fmt1(min)} m au moins au milieu de l'essieu ${rev ? "arrière" : "avant"}` : "";
  }
  $("#sel-what").textContent = what; $("#sel-dim").textContent = dim;
}
const girSelSeg = () => sel && sel.type === "gir" && sel.e != null && site.gir && sel.e < site.gir.pts.length - 1;
$("#sel-arc").addEventListener("click", (ev) => { const b = ev.target.closest("button[data-c]"); if (b && girSelSeg() && (b.dataset.c === "1") !== (girRadius(sel.e) > 0)) girCurve(sel.e, b.dataset.c === "1"); });
$("#sel-r").addEventListener("change", (ev) => { if (girSelSeg()) girSetRadius(sel.e, num(ev.target)); });
$("#sel-r").addEventListener("keydown", (ev) => { if (ev.key === "Enter") ev.target.blur(); });
$("#sel-tan").addEventListener("click", () => {
  if (!girSelSeg() || !sel.e) return;
  const P = site.gir.pts, i = sel.e; P[i] = withK(P[i][0], P[i][1], girTangentK(i)); girEdited();
  toast(girKink(i) < 0.01 ? "Arc tangent au segment précédent" : "Tangence impossible : l'arc dépasserait le demi-cercle, déplacez un point");
});

// ---- Lecture du parcours
function stopGirPlay() { cancelAnimationFrame(girPlay); girPlay = 0; $("#g-play-ico").setAttribute("d", "M5 3.5v9l7-4.5z"); $("#g-play").setAttribute("aria-label", "Lire le parcours"); }
function setGirT(t) { girT = Math.max(0, Math.min(gir.time, t)); $("#g-pos").value = girT; $("#g-at").textContent = fmt1(girPoseAt(girT)[4]) + " m"; redraw(); }
$("#g-play").addEventListener("click", () => {
  if (girPlay) { stopGirPlay(); return; }
  if (!gir) return;
  if (girT >= gir.time - 1e-3) girT = 0;
  $("#g-play-ico").setAttribute("d", "M4.5 3.5h2.5v9H4.5zM9 3.5h2.5v9H9z"); $("#g-play").setAttribute("aria-label", "Mettre en pause");
  let t0 = performance.now(), from = girT;
  const step = (now) => { if (!gir) return; setGirT(from + (now - t0) / 1000); if (girT < gir.time) girPlay = requestAnimationFrame(step); else stopGirPlay(); };
  girPlay = requestAnimationFrame(step);
});
$("#g-pos").addEventListener("input", (ev) => { stopGirPlay(); if (gir) setGirT(+ev.target.value); });
$("#g-list").addEventListener("click", (ev) => {
  const b = ev.target.closest("button[data-i]"); if (!b || !gir) return;
  stopGirPlay(); girSel = +b.dataset.i; setGirT(girTimeAt(gir.conflicts[girSel].s_start)); updateGirList(); girFocus(gir.conflicts[girSel].point);
});
// Cadre les 12 m autour d'un conflit dans la place laissée libre par les panneaux et le menu.
function girFocus(p, size = 12) {
  const fa = freeArea(size, size, 16), s = Math.max(4, Math.min(30, fa.s));
  cancelAnimationFrame(viewAnim); view.s = s; view.ox = (fa.x0 + fa.x1) / 2 - p[0] * s; view.oy = (fa.y0 + fa.y1) / 2 + p[1] * s; redraw();
}

// ---- Hypothèses
const num = (el) => parseFloat(el.value.replace(",", "."));
const frNum = (x) => String(x).replace(".", ",");
let girTimer = null;
function girChanged() { clearTimeout(girTimer); girTimer = setTimeout(() => { placesReset(); girRun(); commit(); }, 250); }
$("#g-prof").addEventListener("click", (ev) => { const b = ev.target.closest("button[data-p]"); if (!b) return; ensureGir().profile = b.dataset.p; updateGirUI(); girChanged(); });
$("#g-kerb").addEventListener("click", (ev) => { const b = ev.target.closest("button[data-k]"); if (!b) return; const g = ensureGir(); g.kerb = { ...g.kerb, name: b.dataset.k }; updateGirUI(); girChanged(); });
for (const [id, lo, hi, put] of [["#g-speed", 1, 30, (g, v) => { g.speed = v; }], ["#g-margin", 0, 1, (g, v) => { g.margin = v; }], ["#g-kh", 0, 0.5, (g, v) => { g.kerb = { ...g.kerb, visible_height: v }; }]])
  $(id).addEventListener("input", (ev) => { const v = num(ev.target); if (!isFinite(v) || v < lo || v > hi) return; put(ensureGir(), v); girChanged(); });

function girCount() { const nc = gir.conflicts.filter((k) => k.severity === "conflit").length; return nc ? `${nc} conflit${nc > 1 ? "s" : ""}` : "Aucun conflit"; }
function updateGirList() {
  const ul = $("#g-list"); ul.textContent = "";
  gir.conflicts.forEach((k, i) => {
    const li = document.createElement("li"), b = document.createElement("button");
    b.dataset.i = i; if (i === girSel) b.setAttribute("aria-current", "true");
    const bad = k.severity === "conflit";
    b.innerHTML = `<span class="chip ${bad ? "bad" : "warn"}">${bad ? "conflit" : "alerte"}</span><span></span><span class="at"></span>`;
    b.children[1].textContent = (k.part[0].toUpperCase() + k.part.slice(1)) + " · " + k.obstacle;
    b.children[2].textContent = fmt1(k.s_start) + " m";
    li.appendChild(b); ul.appendChild(li);
  });
  ul.hidden = !gir.conflicts.length;
}
function updateGirUI() {
  const g = girSet();
  for (const b of $("#g-prof").children) b.setAttribute("aria-pressed", String(b.dataset.p === g.profile));
  for (const b of $("#g-kerb").children) b.setAttribute("aria-pressed", String(b.dataset.k === g.kerb.name));
  for (const [id, v] of [["#g-speed", g.speed], ["#g-margin", g.margin], ["#g-kh", g.kerb.visible_height]]) if (document.activeElement !== $(id)) $(id).value = frNum(v);
  $("#g-prof-note").textContent = GIR_PROFILES[g.profile] || "";
  $("#g-hyp-sum").textContent = `Hypothèses : conducteur ${g.profile}, ${frNum(g.speed)} km/h, marge ${frNum(g.margin)} m, bordure ${g.kerb.name} de ${frNum(g.kerb.visible_height)} m`;
  $("#g-draw").textContent = site.gir && site.gir.pts.length >= 2 ? "Tracer une nouvelle trajectoire" : "Tracer une trajectoire";
  $("#g-draw").disabled = !layout || !!layout.error;
  $("#g-res").hidden = !gir;
  if (!gir) return;
  const nc = gir.conflicts.filter((k) => k.severity === "conflit").length, na = gir.conflicts.length - nc;
  const pl = places.open >= 0 && places.res && places.res[places.open];
  $("#g-clear").hidden = places.open >= 0;
  $("#g-count").textContent = pl ? `Place ${places.open + 1} : ${pl.verdict}` : girCount();
  const pa = pl ? pl.entry.alerts + pl.exit.alerts : 0;
  $("#g-sum").textContent = pl ? [`entrée ${STATUS[pl.entry.status]}, sortie ${STATUS[pl.exit.status]}`, `reprises : ${pl.entry.reprises} en entrée, ${pl.exit.reprises} en sortie`, pa ? `${pa} alerte${pa > 1 ? "s" : ""} de marge` : "", `marge minimale ${pl.entry.clearance >= 1 ? "1 m ou plus" : Math.round(Math.min(pl.entry.clearance, pl.exit.clearance) * 100) + " cm"}`, `${fmt1(gir.time)} s`].filter(Boolean).join(" · ") : [na ? `${na} alerte${na > 1 ? "s" : ""} de marge` : "", `${fmt1(gir.length)} m`, `${fmt1(gir.time)} s` + (gir.stationary_time > 0.05 ? ` dont ${fmt1(gir.stationary_time)} s de braquage à l'arrêt` : "")].filter(Boolean).join(" · ");
  $("#g-pos").max = gir.time; $("#g-pos").step = 0.05; setGirT(girT);
  updateGirList();
  const miss = gir.sections.map((q, i) => (q.reached ? 0 : i + 1)).filter(Boolean), dev = Math.max(0, ...gir.sections.map((q) => q.deviation));
  $("#g-note").textContent = [
    miss.length ? `Tronçon ${miss.join(", ")} non atteint : trajectoire trop serrée pour le véhicule.` : "",
    dev >= 0.05 ? `Écart maximal à la trajectoire tracée : ${nf2.format(dev)} m.` : "",
    pl ? "" : "Touchez le trait pour l'éditer : points, segments courbes, rayon.",
    pl ? (pl.entry.status === "feasible" ? "Entrée en marche avant depuis la voie, 10 m en amont, puis sortie en marche arrière." : `Aucune manœuvre sans contact (${pl.reason}) : voici la moins mauvaise trouvée.`) : "Trait plein : marche avant ; tirets : marche arrière.",
  ].filter(Boolean).join(" ");
}

// ---- Test de toutes les places (phase 2) : entrée en marche avant, sortie en marche arrière,
// parking plein. Le calcul tourne dans un Web Worker (repli : par lots sur le fil principal).
const places = { res: null, open: -1, run: 0, pool: [] };
const VERDICT = { vert: "ok", bleu: "ok", orange: "warn", rouge: "bad", gris: "label-3" };
const STATUS = { feasible: "possible", infeasible: "impossible", unknown: "non conclue" };
function placesInput(extra) {
  const g = girSet();
  return {
    site: engineInput(false), profile: g.profile, margin: g.margin, kerb: g.kerb, speed: g.speed,
    layout: { paved: layout.paved, pads: layout.pads, stalls: layout.stalls.map((st) => st.q), islands: layout.islands, entries: layout.entries, cross: layout.cross, walks: layout.walks || [],
      axes: ((layout.circulation && layout.circulation.axes) || []).map((a) => ({ pts: a.pts, w: a.w, sens: a.sens })) },
    ...extra,
  };
}
function placesReset() { places.run++; places.res = null; places.open = -1; $("#g-prog").hidden = true; updatePlacesUI(); }
// Moteur dédié dans un Web Worker, à partir des octets du WebAssembly déjà chargé.
// Plusieurs moteurs en parallèle : un par cœur disponible, quatre au plus.
const POOL = Math.max(1, Math.min(4, (navigator.hardwareConcurrency || 2) - 1));
function placesWorker(k) {
  places.pool = places.pool || [];
  if (places.pool[k] !== undefined) return places.pool[k];
  places.pool[k] = null;
  try {
    if (!Engine.bytes || !window.Worker) return null;
    // Les demandes attendent la fin de l'instanciation ; une erreur revient comme réponse.
    const src = `let ready; onmessage = async (e) => { const m = e.data;
      if (m.wasm) { ready = WebAssembly.instantiate(m.wasm, {}).then((r) => r.instance.exports); return; }
      try {
        const ex = await ready, b = new TextEncoder().encode(m.input), p = ex.alloc(b.length); new Uint8Array(ex.memory.buffer, p, b.length).set(b);
        const o = ex[m.fn](p, b.length), out = new TextDecoder().decode(new Uint8Array(ex.memory.buffer, o, ex.out_len()));
        ex.dealloc(p, b.length); postMessage({ id: m.id, out });
      } catch (err) { postMessage({ id: m.id, out: JSON.stringify({ error: String(err) }) }); } };`;
    const w = new Worker(URL.createObjectURL(new Blob([src], { type: "text/javascript" })));
    w.postMessage({ wasm: Engine.bytes });
    places.pool[k] = w;
  } catch (e) { places.pool[k] = null; }
  return places.pool[k];
}
let placesSeq = 0;
function placesCall(input, k = 0) { return workerCall("giration_places_json", input, k); }
// Appel du moteur dans un Web Worker (repli : sur le fil principal, après l'affichage en cours).
function workerCall(fn, input, k = 0) {
  const w = placesWorker(k), json = JSON.stringify(input);
  if (!w) return new Promise((res) => setTimeout(() => res(Engine.call(fn, input)), 0));
  return new Promise((res, rej) => {
    const id = ++placesSeq;
    const on = (e) => { if (e.data.id !== id) return; w.removeEventListener("message", on); w.removeEventListener("error", bad); res(JSON.parse(e.data.out)); };
    const bad = (e) => { w.removeEventListener("message", on); w.removeEventListener("error", bad); places.pool[k] = null; rej(e); };
    w.addEventListener("message", on); w.addEventListener("error", bad);
    w.postMessage({ id, fn, input: json });
  });
}
async function testPlaces() {
  if (!layout || layout.error || !layout.stalls.length) return;
  const run = ++places.run, n = layout.stalls.length, res = new Array(n).fill(null), CHUNK = 12, t0 = performance.now();
  places.res = res; places.open = -1; $("#g-prog").hidden = false; updatePlacesUI();
  const chunks = [];
  for (let a = 0; a < n; a += CHUNK) { const ids = []; for (let i = a; i < Math.min(n, a + CHUNK); i++) ids.push(i); chunks.push(ids); }
  let next = 0, done = 0, failed = null;
  // Chaque moteur prend le lot suivant dès qu'il a fini le sien.
  const lane = async (k) => {
    while (next < chunks.length && run === places.run && !failed) {
      const ids = chunks[next++];
      let out;
      try { out = await placesCall(placesInput({ stalls: ids }), k); } catch (e) { out = await placesCall(placesInput({ stalls: ids }), k); }
      if (run !== places.run) return; // plan modifié pendant le calcul
      if (out.error) { failed = out.error; return; }
      for (const p of out.places) res[p.i] = p;
      done += ids.length;
      $("#g-prog-bar").style.width = (100 * done / n) + "%";
      $("#g-prog-txt").textContent = `${done} / ${n} places`;
      updateLayersUI(); updatePlacesUI(); redraw();
    }
  };
  await Promise.all(Array.from({ length: POOL }, (_, k) => lane(k)));
  if (run !== places.run) return;
  if (failed) { toast("Test impossible : " + failed); placesReset(); return; }
  $("#g-prog").hidden = true; updatePlacesUI();
  const c = placesTally();
  toast(`${c.vert + c.bleu} vertes, ${c.orange} orange, ${c.rouge} rouges, ${c.gris} grises · ${fmt1((performance.now() - t0) / 1000)} s`);
}
function placesTally() { const c = { vert: 0, bleu: 0, orange: 0, rouge: 0, gris: 0 }; for (const p of places.res || []) if (p) c[p.verdict]++; return c; }
function updatePlacesUI() {
  $("#g-test").disabled = !layout || !!layout.error || !layout.stalls.length;
  const r = places.res, t = $("#g-tally");
  t.hidden = !r;
  if (!r) return;
  const c = placesTally();
  t.innerHTML = `<span class="chip ok">${c.vert + c.bleu} vertes</span><span class="chip warn">${c.orange} orange</span><span class="chip bad">${c.rouge} rouges</span><span class="chip grey">${c.gris} grises</span><br>Vert : d'un seul tenant ou avec 3 reprises au plus (${c.bleu} avec reprises) ; orange : marge entamée ou plus de 3 reprises ; rouge : impossible dans le domaine exploré ; gris : non conclu (budget de recherche épuisé). Orange et rouges : caisse et roues dessinées. Parking plein, voisines garées au pire. Touchez une place pour voir sa manœuvre.`;
}
// Place sous le point (plan), ou −1.
function stallAt(w) { if (!layout || !layout.stalls) return -1; for (let k = 0; k < layout.stalls.length; k++) if (pointInPoly(w, layout.stalls[k].q)) return k; return -1; }
async function openPlace(k) {
  const run = places.run;
  let out;
  try { out = await placesCall(placesInput({ stalls: [k], detail: k })); } catch (e) { return; }
  if (run !== places.run || out.error) return;
  const p = out.places[0];
  if (places.res) places.res[k] = p;
  if (!out.detail) { if (places.res) redraw(); toast(`Place ${k + 1} : ${p.verdict}${p.reason ? " · " + p.reason : ""}`); return; }
  gir = out.detail; places.open = k; girT = 0; girSel = -1;
  updateGirUI(); updateLayersUI(); redraw();
  if ($("#girmenu").hidden) toggleMenu("#b-gir", "#girmenu");
  girFocus([p.park[0], p.park[1]], 18);
}
// Repère de verdict au centre de chaque place testée : rond (vert, avec ou sans reprises), triangle
// (orange), carré (rouge), anneau (gris) ; la forme double la couleur. Orange et rouges : caisse
// balayée et traces des roues, pour voir où la place pose problème.
function drawPlaces() {
  if (!places.res || !layout || !layout.stalls) return;
  const shown = places.res.filter((p, k) => p && k !== places.open && p.envelope && p.envelope.length);
  ctx.globalAlpha = places.open >= 0 ? 0.35 : 1;
  if (VIS.girenv) for (const p of shown) { girRings(p.envelope, C["gir-body"]); ctx.strokeStyle = C["gir-body-edge"]; ctx.lineWidth = 1; ctx.stroke(); }
  if (VIS.girroues) { ctx.strokeStyle = C["gir-wheel"]; ctx.lineWidth = 1; ctx.lineJoin = "round"; for (const p of shown) for (const t of p.tracks) strokeLine(t); }
  ctx.globalAlpha = 1;
  if (!VIS.girplaces) return;
  const r = Math.max(3, Math.min(8, 0.45 * view.s));
  places.res.forEach((p, k) => {
    if (!p || !layout.stalls[k]) return;
    const q = layout.stalls[k].q, c = toS([(q[0][0] + q[1][0] + q[2][0] + q[3][0]) / 4, (q[0][1] + q[1][1] + q[2][1] + q[3][1]) / 4]);
    ctx.beginPath();
    if (p.verdict === "vert" || p.verdict === "bleu" || p.verdict === "gris") ctx.arc(c[0], c[1], r, 0, Math.PI * 2);
    else if (p.verdict === "orange") { ctx.moveTo(c[0], c[1] - r * 1.15); ctx.lineTo(c[0] + r, c[1] + r * 0.75); ctx.lineTo(c[0] - r, c[1] + r * 0.75); ctx.closePath(); }
    else ctx.rect(c[0] - r * 0.9, c[1] - r * 0.9, r * 1.8, r * 1.8);
    if (p.verdict === "gris") { ctx.lineWidth = 2; ctx.strokeStyle = C["label-3"]; ctx.stroke(); return; }
    ctx.fillStyle = C[VERDICT[p.verdict]]; ctx.fill();
    ctx.lineWidth = k === places.open ? 2.5 : 1; ctx.strokeStyle = k === places.open ? C.label : C.ground; ctx.stroke();
  });
}
$("#g-test").addEventListener("click", () => { closeMenus(); testPlaces(); });

