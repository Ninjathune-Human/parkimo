// ============================================================================
// Interaction : pointeurs (souris, tactile, stylet).
// ============================================================================
let sel = null;
const pointers = new Map();
let drag = null, pinch = null, lastTap = { t: 0, x: 0, y: 0 };
const HIT = () => 14 / view.s;

function dist2(a, b) { return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2; }
function distSeg(p, a, b) { const dx = b[0] - a[0], dy = b[1] - a[1], L = dx * dx + dy * dy || 1; let t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / L; t = Math.max(0, Math.min(1, t)); return [Math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy), t]; }
function pointInPoly(p, poly) { let c = false; for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) { const a = poly[i], b = poly[j]; if ((a[1] > p[1]) !== (b[1] > p[1]) && p[0] < ((b[0] - a[0]) * (p[1] - a[1])) / (b[1] - a[1]) + a[0]) c = !c; } return c; }
function rot(v, deg) { const c = Math.cos(deg * Math.PI / 180), s = Math.sin(deg * Math.PI / 180); return [v[0] * c - v[1] * s, v[0] * s + v[1] * c]; }
const snap = (x, s) => +(Math.round(x / s) * s).toFixed(3);

function hitTest(w) {
  const h = HIT(), h2 = h * h;
  if (selDelBtn) { const sp = toS(w); if (Math.hypot(sp[0] - selDelBtn.x, sp[1] - selDelBtn.y) <= ICON_R + 8) return { kind: "sdel" }; }
  // Points de la trajectoire de giration : glissés, ils relancent la simulation au lâcher.
  if (girEditable()) { const sp = toS(w); for (let k = 0; k < site.gir.pts.length; k++) { const q = toS(site.gir.pts[k]); if (Math.hypot(sp[0] - q[0], sp[1] - q[1]) <= 12) return { kind: "gvert", k }; } }
  if (hoverEdge) { const part = edgeBtnAt(toS(w)); if (part) return { kind: part === "move" ? "emove" : "ebtn", ref: hoverEdge, part }; }
  if (SKEL) {
    const sp = toS(w), T = site.traverses || [];
    for (let i = 0; i < T.length; i++) {
      const a = toS(T[i][0]), b = toS(T[i][1]);
      for (let k = 0; k < 2; k++) { const q = k ? b : a; if (Math.hypot(sp[0] - q[0], sp[1] - q[1]) <= 14) return { kind: "tend", i, k }; }
      if (Math.hypot(sp[0] - (a[0] + b[0]) / 2, sp[1] - (a[1] + b[1]) / 2) <= 15) return { kind: "tbtn", i };
    }
  }
  if (NSKEL) { const sp = toS(w); for (const b of noueBtns) if (Math.hypot(sp[0] - b.x, sp[1] - b.y) <= b.R + 6) return { kind: "nbtn", i: b.i }; }
  if (SKEL) { const sp = toS(w); for (const b of pills) if (Math.abs(sp[0] - b.x) <= PILL.w / 2 + 4 && Math.abs(sp[1] - b.y) <= PILL.h / 2 + 6) return { kind: "pill", i: b.i, part: sp[0] < b.x ? "dir" : "lanes" }; }
  if (sel && sel.type === "building") {
    const o = site.buildings[sel.i];
    if (dist2(w, rotHandle(o)) < h2 * 1.4) return { kind: "rotate", type: sel.type, i: sel.i };
    const cs = objCorners(o); for (let k = 0; k < 4; k++) if (dist2(w, cs[k]) < h2) return { kind: "resize", type: sel.type, i: sel.i, k };
  }
  if (sel && POLY[sel.type]) {
    const e = site[POLY[sel.type]][sel.i]; for (let k = 0; k < e.length; k++) if (dist2(w, e[k]) < h2) return { kind: "evertex", type: sel.type, i: sel.i, k };
  }
  if (sel && sel.type === "slab" && slabOn()) {
    const o = slabObj();
    if (dist2(w, slabRotHandle()) < h2 * 2) return { kind: "srotate" };
    const m = slabMids(o); for (let k = 0; k < 4; k++) if (dist2(w, m[k]) < h2 * 2) return { kind: "sresize", k };
  }
  // Rampe le long des allées : elle se glisse le long de sa rangée ou vers une autre.
  if (VIS.pvoirie && layout && !layout.error && layout.ouvrage && layout.ouvrage.implantation === "rangee") { const R = layout.ouvrage.rampes; for (let i = 0; i < R.length; i++) if (pointInPoly(w, R[i].q)) return { kind: "ramp", i }; }
  if (VIS.noyaux && coresOn()) { const N = layout.ouvrage.noyaux; for (let i = N.length - 1; i >= 0; i--) if (pointInPoly(w, N[i].q)) return { kind: "core", i }; }
  if (VIS.pbati) for (let i = site.buildings.length - 1; i >= 0; i--) if (dist2(w, doorGeom(site.buildings[i]).p) < h2 * 1.2) return { kind: "door", i };
  for (let i = 0; i < site.entries.length; i++) if (site.entries[i].edge < site.parcel.length && dist2(w, entryPoint(site.entries[i])) < h2 * 1.3) return { kind: "entry", i };
  for (let i = 0; i < site.parcel.length; i++) if (dist2(w, site.parcel[i]) < h2) return { kind: "vertex", i };
  { let hit = null; eachRing((r, P) => { for (let i = 0; i < P.length && !hit; i++) { const ref = { ...r, i }, k = kOf(ref); if (k && dist2(w, arcKnob(ref)) < h2) hit = { kind: "arc", ref, s: Math.sign(k) }; } }, true); if (hit) return hit; }
  if (VIS.recul) for (let i = 0; i < site.parcel.length; i++) { const e = edgeInfo(i); if (e.L * view.s >= 40 && dist2(w, setbackHandle(i)) < h2) return { kind: "setback", i }; }
  // Trait de la trajectoire : sélectionne le segment touché.
  if (girEditable()) { const P = site.gir.pts, nb = nearRing(P, (i) => vk(P[i]), w, true); if (nb && nb.d < h * 0.7) return { kind: "gpath", i: nb.i, t: nb.t }; }
  if (VIS.pbati) for (let i = site.buildings.length - 1; i >= 0; i--) if (pointInPoly(w, rectPoly(site.buildings[i]))) return { kind: "move", type: "building", i };
  if (VIS.pbati) for (let i = site.landscapes.length - 1; i >= 0; i--) { const e = polyOutline(site.landscapes[i]); if (pointInPoly(w, e)) return { kind: "move", type: "landscape", i }; for (let k = 0; k < e.length; k++) if (distSeg(w, e[k], e[(k + 1) % e.length])[0] < h * 0.6) return { kind: "move", type: "landscape", i }; }
  if (VIS.pbati) for (let i = (site.basins || []).length - 1; i >= 0; i--) { const e = polyOutline(site.basins[i]); if (pointInPoly(w, e)) return { kind: "move", type: "basin", i }; for (let k = 0; k < e.length; k++) if (distSeg(w, e[k], e[(k + 1) % e.length])[0] < h * 0.6) return { kind: "move", type: "basin", i }; }
  if (VIS.servitudes) for (let i = site.exclusions.length - 1; i >= 0; i--) { const e = polyOutline(site.exclusions[i]); if (pointInPoly(w, e)) return { kind: "move", type: "exclusion", i }; for (let k = 0; k < e.length; k++) if (distSeg(w, e[k], e[(k + 1) % e.length])[0] < h * 0.6) return { kind: "move", type: "exclusion", i }; }
  { const nb = nearEdge(w); if (nb && nb.d < h * 0.7) return { kind: "pedge", i: nb.i, t: nb.t }; }
  // Dalle du silo : un toucher la sélectionne ; sélectionnée, elle se déplace (sinon le geste fait glisser le plan).
  if (slabOn() && pointInPoly(w, layout.ouvrage.dalle)) return { kind: sel && sel.type === "slab" ? "smove" : "slab" };
  const ci = cadParcelAt(w); if (ci >= 0) return { kind: "cadastre", i: ci };
  return null;
}

function setSel(s) { sel = s; updateSelbar(); redraw(); }
function updateSelbar() {
  const bar = $("#selbar");
  if (sel && sel.type === "core" && !(coresOn() && layout.ouvrage.noyaux[sel.i])) sel = null;
  if (!sel || (sel.type !== "cadastre" && sel.type !== "building" && sel.type !== "core" && !(sel.type === "slab" && slabObj()) && !(sel.type === "gir" && girEditable()))) { bar.hidden = true; $("#hint").style.opacity = ""; return; }
  bar.hidden = false; $("#hint").style.opacity = "0";
  bar.classList.toggle("gir", sel.type === "gir" && sel.e != null); // segment : sur téléphone, les réglages prennent la place du nom
  for (const id of ["#sel-arc", "#sel-rwrap", "#sel-tan"]) $(id).hidden = true;
  if (sel.type === "gir") { for (const id of ["#sel-study", "#sel-merge", "#sel-shape", "#sel-rot"]) $(id).hidden = true; girSelbar(); return; }
  const shape = $("#sel-shape"), rotb = $("#sel-rot");
  shape.hidden = sel.type !== "building"; rotb.hidden = sel.type !== "building" && sel.type !== "slab";
  rotb.textContent = sel.type === "slab" ? "Tourner les allées" : "Pivoter 90°";
  $("#sel-auto").hidden = sel.type === "slab" ? !site.slab : sel.type !== "core" || !site.noyaux;
  $("#sel-auto").textContent = sel.type === "core" ? "Placement automatique" : "Emprise automatique";
  if (sel.type === "core") {
    const n = layout.ouvrage.noyaux[sel.i], ev = layout.ouvrage.evacuation;
    $("#sel-study").hidden = $("#sel-merge").hidden = true;
    $("#sel-what").textContent = `${CORE_NAME[natureOf(n)]} ${n.cle.replace("NY/", "")}`;
    $("#sel-dim").textContent = site.noyaux ? "glissez-le vers une place" : "placement automatique · glissez-le pour le déplacer";
    if (ev.hors) $("#sel-dim").textContent = `${ev.hors} place${ev.hors > 1 ? "s" : ""} trop loin d'un escalier`;
    return;
  }
  if (sel.type === "slab") {
    const o = slabObj();
    $("#sel-what").textContent = site.slab ? "Dalle" : "Dalle · emprise automatique";
    $("#sel-dim").textContent = `${fmt1(o.h)} × ${fmt1(o.w)} m · allées à ${fmt1(((o.rot % 180) + 180) % 180)}°`;
    return;
  }
  $("#sel-study").hidden = sel.type !== "cadastre"; $("#sel-merge").hidden = sel.type !== "cadastre" || site.parcel.length < 3;
  if (sel.type === "cadastre") { const f = basemap.layers.parcelles[sel.i]; $("#sel-what").textContent = "Parcelle " + cadName(f); $("#sel-dim").textContent = [f.p.contenance ? fmtInt(f.p.contenance) + " m²" : "", f.p.nom_com].filter(Boolean).join(" · "); return; }
  let what = "", dim = "";
  if (sel.type === "building") { const b = site.buildings[sel.i]; what = b.shape === "L" ? "Bâtiment en L" : "Bâtiment"; dim = fmtInt(Math.abs(polyArea(rectPoly(b)))) + " m²"; shape.textContent = b.shape === "L" ? "Forme rectangle" : "Forme en L"; }
  if (sel.type === "landscape") { what = "Espace vert"; dim = fmtInt(Math.abs(polyArea(polyOutline(site.landscapes[sel.i])))) + " m²"; }
  if (sel.type === "basin") { what = "Bassin de rétention"; dim = fmtInt(Math.abs(polyArea(polyOutline(site.basins[sel.i])))) + " m² · H " + fmt1(params.basin_h) + " m"; }
  if (sel.type === "exclusion") { what = "Servitude"; dim = fmtInt(Math.abs(polyArea(polyOutline(site.exclusions[sel.i])))) + " m²"; }
  if (sel.type === "entry") { what = "Accès véhicules"; dim = "limite " + (site.entries[sel.i].edge + 1); }
  if (sel.type === "vertex") { what = "Sommet " + (sel.i + 1); const v = site.parcel[sel.i]; dim = `${fmt1(v[0])} ; ${fmt1(v[1])}`; }
  $("#sel-what").textContent = what; $("#sel-dim").textContent = dim;
}

function canvasPos(ev) { const r = canvas.getBoundingClientRect(); return [ev.clientX - r.left, ev.clientY - r.top]; }

canvas.addEventListener("pointerdown", (ev) => {
  canvas.setPointerCapture(ev.pointerId);
  cancelAnimationFrame(viewAnim);
  closeMenus();
  const sp = canvasPos(ev); pointers.set(ev.pointerId, sp);
  if (pointers.size === 2) { drag = null; const [a, b] = [...pointers.values()]; pinch = { d: Math.hypot(a[0] - b[0], a[1] - b[1]), m: [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2], s: view.s, ox: view.ox, oy: view.oy, mz: mapv.z, mx: mapv.x, my: mapv.y }; return; }
  if (mode === "map") { mapDown(sp); return; }
  const w = toW(sp[0], sp[1]);
  if (mode === "draw" || mode === "gir") { drag = { kind: "drawtap", sp }; return; }
  const hit = hitTest(w);
  drag = { kind: hit ? hit.kind : "pan", hit, sp, w0: w, moved: false, orig: JSON.stringify(site) };
  if (hit && (hit.kind === "smove" || hit.kind === "srotate" || hit.kind === "sresize")) { drag.slab0 = slabObj(); drag.noy0 = site.noyaux ? JSON.parse(JSON.stringify(site.noyaux)) : null; }
  if (hit && hit.kind === "core") { coreAnim = null; drag.core = { n: layout.ouvrage.noyaux[hit.i] }; }
  if (hit && hit.kind === "ramp") { const q = layout.ouvrage.rampes[hit.i].q; drag.r0 = [(q[0][0] + q[2][0]) / 2, (q[0][1] + q[2][1]) / 2]; }
  if (hit) {
    if (hit.kind === "move") setSel({ type: hit.type, i: hit.i });
    else if (hit.kind === "vertex") setSel({ type: "vertex", i: hit.i });
    else if (hit.kind === "entry") setSel({ type: "entry", i: hit.i });
    else if (hit.kind === "door") setSel({ type: "building", i: hit.i });
    else if (hit.kind === "core") setSel({ type: "core", i: hit.i });
    else if (hit.kind === "evertex") setSel({ type: hit.type, i: hit.i, v: hit.k });
    else if (hit.kind === "gvert") setSel({ type: "gir", v: hit.k });
    else if (hit.kind === "gpath") setSel({ type: "gir", e: hit.i });
    else if (hit.kind === "setback" || hit.kind === "pedge") { /* conserve la sélection */ }
  }
  canvas.classList.add("dragging");
});

canvas.addEventListener("pointermove", (ev) => {
  const sp = canvasPos(ev);
  if (mode === "draw") { hoverW = snapW(toW(sp[0], sp[1])); redraw(); }
  if (mode === "gir") { hoverW = girSnap(toW(sp[0], sp[1])); redraw(); }
  if (!drag && ev.pointerType !== "touch") setHoverEdge(edgeUnder(sp));
  if (!pointers.has(ev.pointerId)) return;
  pointers.set(ev.pointerId, sp);
  if (pinch && pointers.size === 2 && mode === "map") {
    const [a, b] = [...pointers.values()], d = Math.hypot(a[0] - b[0], a[1] - b[1]), m = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
    const z = Math.max(3, Math.min(20, pinch.mz + Math.log2(d / pinch.d))), r0 = mpp(pinch.mz), r1 = mpp(z);
    const wx = pinch.mx + (pinch.m[0] - W / 2) * r0, wy = pinch.my - (pinch.m[1] - H / 2) * r0;
    mapv.z = z; mapv.x = wx - (m[0] - W / 2) * r1; mapv.y = wy + (m[1] - H / 2) * r1; updateMapUI(); redraw(); return;
  }
  if (mode === "map") { mapMove(sp); return; }
  if (pinch && pointers.size === 2) {
    const [a, b] = [...pointers.values()], d = Math.hypot(a[0] - b[0], a[1] - b[1]), m = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
    const k = d / pinch.d, s = Math.max(0.5, Math.min(60, pinch.s * k));
    const wx = (pinch.m[0] - pinch.ox) / pinch.s, wy = (pinch.oy - pinch.m[1]) / pinch.s;
    view.s = s; view.ox = m[0] - wx * s; view.oy = m[1] + wy * s; redraw(); return;
  }
  // Tracé de giration : glisser dessine un trait libre (toucher pose un point).
  if (drag && mode === "gir" && (drag.kind === "drawtap" || drag.kind === "stroke")) {
    if (drag.kind === "drawtap") { if (Math.hypot(sp[0] - drag.sp[0], sp[1] - drag.sp[1]) < 8) return; drag.kind = "stroke"; drag.stroke = [toW(drag.sp[0], drag.sp[1])]; }
    const w = toW(sp[0], sp[1]), last = drag.stroke[drag.stroke.length - 1];
    if (Math.hypot(w[0] - last[0], w[1] - last[1]) * view.s >= 3) drag.stroke.push(w);
    redraw(); return;
  }
  if (!drag || drag.kind === "drawtap") return;
  const dx = sp[0] - drag.sp[0], dy = sp[1] - drag.sp[1];
  if (!drag.moved && Math.hypot(dx, dy) < 4) return;
  drag.moved = true;
  const w = toW(sp[0], sp[1]), h = drag.hit;
  switch (drag.kind) {
    case "pan": view.ox += dx; view.oy += dy; drag.sp = sp; redraw(); return;
    case "ramp": {
      // Rampe placée à la main : son centre suit le pointeur, le moteur la recale dans une rangée
      // et réorganise les places autour d'elle.
      if (!site.rampes) site.rampes = [null, null];
      site.rampes[h.i] = [drag.r0[0] + w[0] - drag.w0[0], drag.r0[1] + w[1] - drag.w0[1]].map((v) => +v.toFixed(3));
      solve(true); return;
    }
    case "core": {
      const c = drag.core, c0 = qCentre(c.n.q); if (!c.T) c.T = coreTargets(c.n);
      c.cur = [c0[0] + w[0] - drag.w0[0], c0[1] + w[1] - drag.w0[1]];
      let best = null, bd = 3.5 * 3.5; for (const t of c.T) { const d = dist2(t.c, c.cur); if (d < bd) { bd = d; best = t; } }
      if (best !== c.target) { c.target = best; c.tt = performance.now(); }
      redraw(); return;
    }
    case "vertex": site.parcel[h.i] = [snap(w[0], 0.1), snap(w[1], 0.1)]; break;
    case "setback": { const e = edgeInfo(h.i); const s = (w[0] - e.apex[0]) * e.n[0] + (w[1] - e.apex[1]) * e.n[1]; site.setbacks[h.i] = Math.max(0, Math.min(40, snap(s, 0.5))); break; }
    case "arc": { const e = edgeRef(h.ref); let sg = -((w[0] - e.mid[0]) * e.n[0] + (w[1] - e.mid[1]) * e.n[1]); sg -= h.s * KNOB / view.s; let k = Math.max(-ARC_MAX, Math.min(ARC_MAX, snap(sg / e.L, 0.005))); if (Math.abs(k) < 0.01) k = 0; setK(h.ref, k); break; }
    case "entry": { const best = nearEdge(w); site.entries[h.i] = { edge: best.i, t: Math.max(0.02, Math.min(0.98, best.t)) }; break; }
    case "move": {
      const o0 = JSON.parse(drag.orig), ddx = w[0] - drag.w0[0], ddy = w[1] - drag.w0[1];
      if (POLY[h.type]) site[POLY[h.type]][h.i] = o0[POLY[h.type]][h.i].map(([x, y, ...r]) => [x + ddx, y + ddy, ...r]);
      else { const src = o0.buildings[h.i]; site.buildings[h.i] = { ...src, cx: snap(src.cx + ddx, 0.25), cy: snap(src.cy + ddy, 0.25) }; }
      break;
    }
    case "door": { const b = site.buildings[h.i], P = rectPoly(b); let best = null; for (let i = 0; i < P.length; i++) { const [dd, t] = distSeg(w, P[i], P[(i + 1) % P.length]); if (!best || dd < best.d) best = { d: dd, e: i, t }; } b.door = { e: best.e, t: +best.t.toFixed(4) }; break; }
    case "evertex": { const P = site[POLY[h.type]][h.i]; P[h.k] = withK(snap(w[0], 0.1), snap(w[1], 0.1), vk(P[h.k])); break; }
    case "rotate": { const o = site.buildings[h.i]; o.rot = snapRot(Math.atan2(w[1] - o.cy, w[0] - o.cx) * 180 / Math.PI - 90); break; }
    case "smove": case "srotate": case "sresize": {
      const o0 = drag.slab0, o = site.slab = { ...o0, fixe: [0, 0] };
      if (drag.kind === "smove") { o.cx = snap(o0.cx + w[0] - drag.w0[0], 0.1); o.cy = snap(o0.cy + w[1] - drag.w0[1], 0.1); }
      else if (drag.kind === "srotate") o.rot = snapRot(o0.rot + (Math.atan2(w[1] - o0.cy, w[0] - o0.cx) - Math.atan2(drag.w0[1] - o0.cy, drag.w0[0] - o0.cx)) * 180 / Math.PI);
      else {
        // Côté étiré, côté opposé fixe : le moteur ramène la dimension à des modules (en travers) et des places (en long) entiers.
        const d = rot([w[0] - o0.cx, w[1] - o0.cy], -o0.rot), long = h.k % 2 === 1, s = h.k === 1 || h.k === 2 ? 1 : -1;
        const half = (long ? o0.w : o0.h) / 2, size = Math.max(1, s * d[long ? 0 : 1] + half), c = -s * half + s * size / 2;
        const cw = rot(long ? [c, 0] : [0, c], o0.rot);
        o.cx = o0.cx + cw[0]; o.cy = o0.cy + cw[1]; o[long ? "w" : "h"] = size; o.fixe = long ? [-s, 0] : [0, -s];
      }
      // Les escaliers placés à la main suivent la dalle déplacée ou tournée.
      if (drag.noy0 && drag.kind !== "sresize") site.noyaux = drag.noy0.map((e) => { const r = rot([e.p[0] - o0.cx, e.p[1] - o0.cy], o.rot - o0.rot); return { ...e, p: [+(o.cx + r[0]).toFixed(3), +(o.cy + r[1]).toFixed(3)] }; });
      break;
    }
    case "resize": {
      const o = site.buildings[h.i];
      const signs = [[-1, -1], [1, -1], [1, 1], [-1, 1]], [sx, sy] = signs[h.k];
      const cs = objCorners(JSON.parse(drag.orig).buildings[h.i]), opp = cs[(h.k + 2) % 4];
      const d = rot([w[0] - opp[0], w[1] - opp[1]], -o.rot);
      const nw = Math.max(6, snap(sx * d[0], 0.5)), nh = Math.max(6, snap(sy * d[1], 0.5));
      const c = rot([sx * nw / 2, sy * nh / 2], o.rot);
      Object.assign(o, { w: nw, h: nh, cx: opp[0] + c[0], cy: opp[1] + c[1] });
      break;
    }
    case "emove": {
      // Déplace le segment : ses deux sommets suivent le doigt (les autres sommets ne bougent pas).
      const o0 = JSON.parse(drag.orig), R0 = h.ref.g === "parcel" ? o0.parcel : h.ref.g === "gir" ? o0.gir.pts : o0[POLY[h.ref.g]][h.ref.o], R = ringOf(h.ref), n = R0.length;
      const ddx = snap(w[0] - drag.w0[0], 0.1), ddy = snap(w[1] - drag.w0[1], 0.1);
      for (const k of [h.ref.i, (h.ref.i + 1) % n]) R[k] = [R0[k][0] + ddx, R0[k][1] + ddy, ...R0[k].slice(2)];
      break;
    }
    case "tend": site.traverses[h.i][h.k] = [snap(w[0], 0.1), snap(w[1], 0.1)]; break;
    case "gvert": { const P = site.gir.pts, q = girSnap(w); P[h.k] = withK(q[0], q[1], vk(P[h.k])); break; }
    case "pedge": case "cadastre": case "pill": case "ebtn": case "nbtn": case "tbtn": case "sdel": case "gpath": case "slab": drag.kind = "pan"; return;
  }
  // La trajectoire ne touche pas au plan : simulation relancée au lâcher seulement.
  if (girDrag(drag)) { updateSelbar(); redraw(); return; }
  updateSelbar(); requestSolve(true);
});

function endPointer(ev) {
  const sp = pointers.get(ev.pointerId) || canvasPos(ev);
  pointers.delete(ev.pointerId);
  if (pinch) { if (pointers.size < 2) pinch = null; return; }
  canvas.classList.remove("dragging");
  const d = drag; drag = null; if (!d) return;
  if (mode === "map") return;
  if (d.kind === "core") { if (d.moved) dropCore(d.core); return; }
  const now = performance.now(), dbl = now - lastTap.t < 350 && Math.hypot(sp[0] - lastTap.x, sp[1] - lastTap.y) < 16;
  if (d.kind === "drawtap") { (mode === "gir" ? onGirTap : onDrawTap)(toW(sp[0], sp[1]), dbl, sp); lastTap = { t: now, x: sp[0], y: sp[1] }; return; }
  if (d.kind === "stroke") { if (girLen(d.stroke) >= 1) girStroke(d.stroke); else onGirTap(toW(sp[0], sp[1]), false, sp); return; }
  if (d.moved) { if (girDrag(d)) girEdited(); else if (d.kind !== "pan") { solve(false); if (d.slab0) { syncSlab(); updateSelbar(); } commit(); markMine(); } return; }
  // Toucher simple
  lastTap = { t: dbl ? 0 : now, x: sp[0], y: sp[1] };
  // Double appui sur une rampe placée à la main : elle reprend sa place d'office.
  if (d.kind === "ramp") {
    if (dbl && site.rampes && site.rampes[d.hit.i]) { site.rampes[d.hit.i] = null; if (!site.rampes.some(Boolean)) delete site.rampes; solve(false); commit(); markMine(); toast("Rampe replacée d'office"); }
    else toast(site.rampes && site.rampes[d.hit.i] ? "Rampe placée à la main · double appui pour la replacer d'office" : "Glissez la rampe pour la déplacer");
    return;
  }
  const h = d.hit;
  if (h && h.kind === "sdel") { deleteHandle(); return; }
  if (h && h.kind === "nbtn") { toggleNoue(h.i); return; }
  if (h && h.kind === "tbtn") { site.traverses.splice(h.i, 1); solve(false); commit(); markMine(); toast("Traverse supprimée"); return; }
  if (h && h.kind === "pill") { (h.part === "dir" ? reverseLane : toggleLanes)(h.i); return; }
  if (h && h.kind === "ebtn") { (h.part === "add" ? insertVertexRef(h.ref, 0.5) : curveRef(h.ref)); return; }
  if (dbl && h) {
    if (h.kind === "gvert") { removeGirPoint(h.k); return; }
    if (h.kind === "gpath") { insertGirPoint(h.i, h.t); return; }
    if (h.kind === "vertex" && site.parcel.length > 3) { removeParcelVertex(h.i); return; }
    if (h.kind === "pedge") { insertParcelVertex(h.i, h.t); return; }
    if (h.kind === "evertex" && site[POLY[h.type]][h.i].length > 3) { removePolyVertex(h.type, h.i, h.k); return; }
    if (h.kind === "move" && POLY[h.type]) { const P = site[POLY[h.type]][h.i], nb = nearRing(P, (i) => vk(P[i]), toW(sp[0], sp[1])); if (nb && nb.d < HIT()) { insertPolyVertex(h.type, h.i, nb.i, nb.t); return; } }
  }
  if (h && h.kind === "cadastre") { setSel({ type: "cadastre", i: h.i }); return; }
  if (h && h.kind === "slab") { setSel({ type: "slab" }); return; }
  { const nb = nearAnyEdge(toW(sp[0], sp[1])); setHoverEdge(nb && nb.d * view.s < 12 && edgeRef(nb.ref).L * view.s >= 100 ? nb.ref : null); }
  if ((!h || h.kind === "pedge" || h.kind === "setback") && places.res && VIS.girplaces) { const k = stallAt(toW(sp[0], sp[1])); if (k >= 0) { openPlace(k); return; } }
  if (!h || h.kind === "pedge" || h.kind === "setback") setSel(null);
}
canvas.addEventListener("pointerup", endPointer);
canvas.addEventListener("pointercancel", endPointer);

canvas.addEventListener("wheel", (ev) => {
  ev.preventDefault();
  if (mode === "map") { mapZoomAt(canvasPos(ev), mapv.z - ev.deltaY * (ev.ctrlKey ? 0.015 : 0.0022)); return; }
  const sp = canvasPos(ev), k = Math.exp(-ev.deltaY * (ev.ctrlKey ? 0.01 : 0.0015));
  const s = Math.max(0.5, Math.min(60, view.s * k)), [wx, wy] = toW(sp[0], sp[1]);
  view.s = s; view.ox = sp[0] - wx * s; view.oy = sp[1] + wy * s; redraw();
}, { passive: false });

function ensureArcs() { const n = site.parcel.length; if (!site.arcs) site.arcs = []; while (site.arcs.length < n) site.arcs.push(0); site.arcs.length = n; }
function insertParcelVertex(i, t) {
  ensureArcs(); hoverEdge = null;
  const e = edgeInfo(i), q = e.at(t), p = [snap(q[0], 0.01), snap(q[1], 0.01)], k = site.arcs[i];
  // Un arc se coupe en deux arcs du même cercle ; angles proportionnels à t.
  site.parcel.splice(i + 1, 0, p); site.setbacks.splice(i + 1, 0, site.setbacks[i] || 0);
  const [k1, k2] = k ? arcSplit(k, t) : [0, 0]; site.arcs[i] = k1; site.arcs.splice(i + 1, 0, k2);
  site.entries.forEach((en) => { if (en.edge === i) { if (en.t > t) { en.edge = i + 1; en.t = (en.t - t) / (1 - t); } else en.t = en.t / t; } else if (en.edge > i) en.edge++; });
  setSel({ type: "vertex", i: i + 1 }); solve(false); commit(); markMine(); toast("Sommet ajouté");
}
function insertPolyVertex(g, o, i, t) {
  const P = site[POLY[g]][o], e = edgeG(P, i, vk(P[i])), q = e.at(t), k = vk(P[i]), [k1, k2] = k ? arcSplit(k, t) : [0, 0];
  hoverEdge = null; P[i] = withK(P[i][0], P[i][1], k1); P.splice(i + 1, 0, withK(snap(q[0], 0.01), snap(q[1], 0.01), k2));
  setSel({ type: g, i: o, v: i + 1 }); solve(false); commit(); markMine(); toast("Sommet ajouté");
}
function removePolyVertex(g, o, k) {
  const P = site[POLY[g]][o], n = P.length, pv = (k - 1 + n) % n;
  P[pv] = withK(P[pv][0], P[pv][1], arcJoin(vk(P[pv]), vk(P[k]))); P.splice(k, 1);
  setSel({ type: g, i: o }); solve(false); commit(); markMine();
}
const insertVertexRef = (r, t) => (r.g === "parcel" ? insertParcelVertex(r.i, t) : r.g === "gir" ? insertGirPoint(r.i, t) : insertPolyVertex(r.g, r.o, r.i, t));
function curveRef(r) {
  if (r.g === "parcel") { curveParcelEdge(r.i); return; }
  if (r.g === "gir") { hoverEdge = null; girCurve(r.i, kOf(r) === 0); return; }
  const was = kOf(r) !== 0; setK(r, was ? 0 : 0.15); hoverEdge = null;
  solve(false); commit(); markMine(); toast(was ? "Segment redressé" : "Segment arrondi · glissez le point pour régler l'arc");
}
// Arrondit le segment i (arc de cercle dynamique, flèche de 15 % de la corde vers l'extérieur), ou le redresse.
function curveParcelEdge(i) {
  ensureArcs(); const was = site.arcs[i] !== 0;
  site.arcs[i] = was ? 0 : 0.15;
  setSel(null); solve(false); commit(); markMine(); toast(was ? "Segment redressé" : "Segment arrondi · glissez le point pour régler l'arc");
}
function removeParcelVertex(i) {
  const n = site.parcel.length; ensureArcs();
  const pv = (i - 1 + n) % n, nw = i ? i - 1 : n - 2; // arête précédente avant / après retrait
  const Lp = edgeInfo(pv).L, Li = edgeInfo(i).L;
  // Les deux arêtes n'en font plus qu'une : les angles de leurs arcs s'additionnent (un arc coupé en deux se reforme à l'identique).
  site.arcs[pv] = +arcJoin(site.arcs[pv], site.arcs[i]).toFixed(5);
  site.parcel.splice(i, 1); site.setbacks.splice(i, 1); site.arcs.splice(i, 1);
  // Les accès gardent leur position sur la limite.
  site.entries = site.entries.map((en) => {
    if (en.edge === pv) return { edge: nw, t: en.t * Lp / (Lp + Li) };
    if (en.edge === i) return { edge: nw, t: (Lp + en.t * Li) / (Lp + Li) };
    return { edge: en.edge > i ? en.edge - 1 : en.edge, t: en.t };
  });
  setSel(null); solve(false); commit(); markMine(); toast("Sommet supprimé");
}

// Tracé d'une parcelle
function snapW(w) { return [snap(w[0], 0.5), snap(w[1], 0.5)]; }
function onDrawTap(w, dbl, sp) {
  w = snapW(w);
  if (draft.length >= 3) { const f = toS(draft[0]); if (Math.hypot(f[0] - sp[0], f[1] - sp[1]) < 16 || dbl) { finishDraft(); return; } }
  draft.push(w); redraw();
}
function finishDraft() {
  const P = draft.slice(); if (P.length < 3 || Math.abs(polyArea(P)) < 100) { toast("Parcelle trop petite"); return; }
  let li = 0, lb = 0; for (let i = 0; i < P.length; i++) { const a = P[i], b = P[(i + 1) % P.length], L = Math.hypot(b[0] - a[0], b[1] - a[1]); if (L > lb) { lb = L; li = i; } }
  site = { parcel: P, setbacks: P.map((_, i) => (i === li ? 5 : 3)), exclusions: [], buildings: [], landscapes: [], basins: [], entries: [{ edge: li, t: ENTRY_T }] };
  setMode("edit"); exampleName = "mine"; syncExample(); lastAngle = null; solve(false); commit(); fit(); toast("Parcelle créée · recul 5 m sur la limite la plus longue, 3 m ailleurs");
}
function setMode(m) {
  if (mode === "gir" && m !== "gir") $("#girbar").hidden = true;
  mode = m; draft = []; hoverW = null; canvas.classList.toggle("drawing", m === "draw" || m === "gir");
  if (m === "gir") { gdraft = { pts: [], cusps: [], reverse: false }; $("#girbar").hidden = false; syncGirBar(); }
  gstroke = null;
  updateUndo();
  $("#b-draw").setAttribute("aria-pressed", String(m === "draw")); $("#b-draw").classList.toggle("on", m === "draw");
  $("#hint").textContent = m === "draw" ? "Touchez pour placer les sommets · touchez le premier sommet pour fermer · Échap pour annuler" : "Glissez sommets, reculs, accès et bâtiments · survolez ou touchez un bord pour ajouter un sommet ou l'arrondir";
  setSel(null); redraw();
}

