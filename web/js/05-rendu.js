// ============================================================================
// Rendu.
// ============================================================================
let C = {};
function readColors() {
  const cs = getComputedStyle(document.documentElement);
  for (const k of ["glass-solid", "sep", "ground", "grid", "grid-strong", "parcel", "parcel-edge", "asphalt", "stripe", "island", "tree", "tree-edge", "building", "building-edge", "building-label", "slab", "excl", "setback", "pmr", "irve", "pmr-irve", "accent", "label", "label-2", "cad", "cad-halo", "cad-label", "cad-bati", "cad-bati-edge", "cad-sheet", "cad-commune", "veg", "veg-tint", "occ-voirie", "label-3", "ok", "warn", "bad", "gir-wheel", "gir-body", "gir-body-edge"]) C[k] = cs.getPropertyValue("--" + k).trim();
  hatch = null; vegPat = null;
}
let hatch = null;
function hatchPattern() {
  if (hatch) return hatch;
  const c = document.createElement("canvas"); c.width = c.height = 10 * DPR;
  const g = c.getContext("2d"); g.strokeStyle = C.excl; g.globalAlpha = 0.55; g.lineWidth = 1.2 * DPR;
  g.beginPath(); g.moveTo(0, c.height); g.lineTo(c.width, 0); g.moveTo(-c.width / 2, c.height / 2); g.lineTo(c.width / 2, -c.height / 2); g.moveTo(c.width / 2, c.height * 1.5); g.lineTo(c.width * 1.5, c.height / 2); g.stroke();
  hatch = ctx.createPattern(c, "repeat"); return hatch;
}

function pathPoly(pts) { if (!pts.length) return; const a = toS(pts[0]); ctx.moveTo(a[0], a[1]); for (let i = 1; i < pts.length; i++) { const b = toS(pts[i]); ctx.lineTo(b[0], b[1]); } ctx.closePath(); }
// Noues entre les places : un haricot allongé aux bouts arrondis (haut de talus en trait fort), des barbules
// courtes vers l'intérieur, de longueurs alternées, et le fond figuré par un seul trait (fil d'eau).
// Axe d'une noue (milieu du petit côté au milieu du petit côté opposé). Supprimer une noue supprime celle de
// toute sa rangée ; les repères de site.noue_off sont donnés sur la trame de référence du moteur.
const noueAxis = (q) => [[(q[0][0] + q[3][0]) / 2, (q[0][1] + q[3][1]) / 2], [(q[1][0] + q[2][0]) / 2, (q[1][1] + q[2][1]) / 2]];
function toggleNoue(i) {
  // Repères sur la trame de référence (noues partout) : stables quand les rangées se referment.
  const [ra, rb, mid] = layout.noue_ref[i], off = layout.noues_off[i];
  site.noue_off = (site.noue_off || []).filter((p) => distSeg(p, ra, rb)[0] >= 0.75);
  if (!off) site.noue_off.push(mid);
  solve(false); commit(); markMine(); toast(off ? "Noue rétablie" : "Noue supprimée");
}
// Traverses du concepteur : segment, deux poignées d'extrémité et, au milieu, la croix qui la supprime.
function drawTraverses() {
  const T = site.traverses || []; if (!T.length) return;
  ctx.save(); ctx.lineCap = "round";
  for (const t of T) {
    const a = toS(t[0]), b = toS(t[1]), cx = (a[0] + b[0]) / 2, cy = (a[1] + b[1]) / 2;
    ctx.strokeStyle = C.accent; ctx.lineWidth = 3; ctx.setLineDash([9, 6]); ctx.beginPath(); ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); ctx.stroke(); ctx.setLineDash([]);
    for (const q of [a, b]) { ctx.beginPath(); ctx.arc(q[0], q[1], 6, 0, Math.PI * 2); ctx.fillStyle = C["glass-solid"]; ctx.fill(); ctx.lineWidth = 2; ctx.stroke(); }
    ctx.save(); ctx.shadowColor = "rgba(0, 0, 0, .22)"; ctx.shadowBlur = 8; ctx.shadowOffsetY = 2;
    ctx.beginPath(); ctx.arc(cx, cy, 9, 0, Math.PI * 2); ctx.fillStyle = C["glass-solid"]; ctx.fill(); ctx.restore();
    ctx.lineWidth = 1; ctx.beginPath(); ctx.arc(cx, cy, 9, 0, Math.PI * 2); ctx.stroke();
    ctx.lineWidth = 2; ctx.beginPath(); ctx.moveTo(cx - 4.5, cy - 4.5); ctx.lineTo(cx + 4.5, cy + 4.5); ctx.moveTo(cx + 4.5, cy - 4.5); ctx.lineTo(cx - 4.5, cy + 4.5); ctx.stroke();
  }
  ctx.restore();
}
function drawNoueSkeleton() {
  noueBtns = []; const list = layout && !layout.error && layout.noues; if (!list) return;
  const RED = "#d93a2f", R = 9;
  ctx.save(); ctx.lineCap = "round";
  list.forEach((q, i) => {
    const off = layout.noues_off[i], a = noueAxis(q).map(toS),
      len = Math.hypot(a[1][0] - a[0][0], a[1][1] - a[0][1]); if (len < 6) return;
    const ux = (a[1][0] - a[0][0]) / len, uy = (a[1][1] - a[0][1]) / len, g = Math.min(len * 0.1, 5), col = off ? RED : C.accent;
    ctx.strokeStyle = col; ctx.lineWidth = 2; ctx.globalAlpha = 1;
    ctx.beginPath(); ctx.moveTo(a[0][0] + ux * g, a[0][1] + uy * g); ctx.lineTo(a[1][0] - ux * g, a[1][1] - uy * g); ctx.stroke();
    for (const e of [a[0], a[1]]) { const k = e === a[0] ? 1 : -1; ctx.beginPath(); ctx.moveTo(e[0] + ux * k * g - uy * 5, e[1] + uy * k * g + ux * 5); ctx.lineTo(e[0] + ux * k * g + uy * 5, e[1] + uy * k * g - ux * 5); ctx.stroke(); }
    if (len < 2 * R + 10) return;
    const cx = (a[0][0] + a[1][0]) / 2, cy = (a[0][1] + a[1][1]) / 2; noueBtns.push({ i, x: cx, y: cy, R });
    ctx.save(); ctx.shadowColor = "rgba(0, 0, 0, .22)"; ctx.shadowBlur = 8; ctx.shadowOffsetY = 2;
    ctx.beginPath(); ctx.arc(cx, cy, R, 0, Math.PI * 2); ctx.fillStyle = C["glass-solid"]; ctx.fill(); ctx.restore();
    ctx.strokeStyle = col; ctx.lineWidth = 1; ctx.beginPath(); ctx.arc(cx, cy, R, 0, Math.PI * 2); ctx.stroke();
    ctx.lineWidth = 2; ctx.beginPath();
    if (!off) { ctx.moveTo(cx - 4.5, cy - 4.5); ctx.lineTo(cx + 4.5, cy + 4.5); ctx.moveTo(cx + 4.5, cy - 4.5); ctx.lineTo(cx - 4.5, cy + 4.5); }
    else { ctx.arc(cx, cy, 5, -Math.PI * 0.6, Math.PI * 1.2); ctx.stroke(); ctx.beginPath(); ctx.moveTo(cx - 4.5, cy - 7); ctx.lineTo(cx - 3.2, cy - 2.6); ctx.lineTo(cx - 7.6, cy - 3.6); }
    ctx.stroke();
  });
  ctx.restore();
}
function drawNoues() {
  const list = layout && !layout.error && layout.noues; if (!list || !list.length) return;
  ctx.globalAlpha = 0.55; ctx.strokeStyle = C["tree-edge"]; ctx.lineCap = "butt"; ctx.lineJoin = "round";
  const step = Math.max(0.8, 5 / view.s), outlines = [], ticks = [], fonds = [];
  for (const [i, q] of list.entries()) {
    if (layout.noues_off[i]) continue;
    const L = Math.hypot(q[1][0] - q[0][0], q[1][1] - q[0][1]), nw = Math.hypot(q[3][0] - q[0][0], q[3][1] - q[0][1]); if (L < 0.6 || nw < 0.3) continue;
    const ux = (q[1][0] - q[0][0]) / L, uy = (q[1][1] - q[0][1]) / L, cx = (q[3][0] - q[0][0]) / nw, cy = (q[3][1] - q[0][1]) / nw;
    const r = Math.min(nw, L) / 2, W = (t, v) => [q[0][0] + ux * t + cx * v, q[0][1] + uy * t + cy * v];
    // Contour du haricot dans le repère de la noue (t le long, v en travers), sens trigonométrique.
    const pts = []; const arc = (t0, a0, a1) => { for (let k = 0; k <= 8; k++) { const a = a0 + (a1 - a0) * k / 8; pts.push(W(t0 + r * Math.cos(a), r + r * Math.sin(a))); } };
    arc(L - r, -Math.PI / 2, Math.PI / 2); arc(r, Math.PI / 2, 3 * Math.PI / 2);
    outlines.push(pts);
    let carry = 0, n = 0;
    for (let k = 0; k < pts.length; k++) {
      const a = pts[k], b = pts[(k + 1) % pts.length], seg = Math.hypot(b[0] - a[0], b[1] - a[1]); if (seg < 1e-6) continue;
      const dx = (b[0] - a[0]) / seg, dy = (b[1] - a[1]) / seg;
      for (let t = step - carry; t <= seg; t += step, n++) {
        const len = r * (n % 2 ? 0.3 : 0.55), o = [a[0] + dx * t, a[1] + dy * t];
        ticks.push([o, [o[0] - dy * len, o[1] + dx * len]]);
      }
      carry = (carry + seg) % step;
    }
    if (L > 3 * r) fonds.push([W(L * 0.12 + r * 0.5, r), W(L * 0.88 - r * 0.5, r)]);
  }
  ctx.globalAlpha = 1; ctx.fillStyle = "rgba(20,40,25,.07)"; ctx.beginPath(); for (const o of outlines) { const a = toS(o[0]); ctx.moveTo(a[0], a[1]); for (const p of o.slice(1)) { const b = toS(p); ctx.lineTo(b[0], b[1]); } ctx.closePath(); } ctx.fill(); ctx.globalAlpha = 0.55;
  const seg = (a, b) => { const p0 = toS(a), p1 = toS(b); ctx.moveTo(p0[0], p0[1]); ctx.lineTo(p1[0], p1[1]); };
  ctx.beginPath(); for (const [a, b] of ticks) seg(a, b); ctx.lineWidth = 0.5; ctx.stroke();
  ctx.beginPath(); for (const [a, b] of fonds) seg(a, b); ctx.lineWidth = 0.6; ctx.stroke();
  ctx.beginPath(); for (const o of outlines) { const a = toS(o[0]); ctx.moveTo(a[0], a[1]); for (const p of o.slice(1)) { const b = toS(p); ctx.lineTo(b[0], b[1]); } ctx.closePath(); } ctx.lineWidth = 0.9; ctx.stroke();
  ctx.globalAlpha = 1;
}
// Bassin de rétention : zone verte en creux. Haut de talus (trait fort), pied de talus (trait fin, fond
// plus sombre) et barbules, du haut vers le bas, alternativement longues et courtes pour figurer la pente.
// Ombre des arbres à maturité, telle que le contrôle d'ombrage la compte : houppiers réunis.
function drawCanopies() {
  const r = (params.canopy || 0) / 2 * view.s;
  if (!(r > 1) || !layout.trees.length) return;
  ctx.beginPath();
  for (const t of layout.trees) { const [x, y] = toS(t); ctx.moveTo(x + r, y); ctx.arc(x, y, r, 0, 2 * Math.PI); }
  ctx.fillStyle = C.tree; ctx.globalAlpha = 0.16; ctx.fill(); ctx.globalAlpha = 1;
}
// Ombrières photovoltaïques : toiture des rangées couvertes, faîtage et trame des panneaux.
function drawOmbrieres() {
  const L = layout.ombrieres;
  if (!L || !L.length) return;
  ctx.fillStyle = "rgba(38, 52, 78, .8)";
  ctx.beginPath(); for (const q of L) pathPoly(q); ctx.fill();
  if (view.s < 1.5) return;
  ctx.strokeStyle = "rgba(255, 255, 255, .3)"; ctx.lineWidth = Math.max(0.5, 0.05 * view.s); ctx.beginPath();
  const mix = (p, r, t) => toS([p[0] + (r[0] - p[0]) * t, p[1] + (r[1] - p[1]) * t]);
  for (const q of L) {
    const d01 = Math.hypot(q[1][0] - q[0][0], q[1][1] - q[0][1]), d12 = Math.hypot(q[2][0] - q[1][0], q[2][1] - q[1][1]);
    // a → b : grand côté, le long de la rangée.
    const [a, b, c, d] = d01 >= d12 ? q : [q[1], q[2], q[3], q[0]], n = Math.max(1, Math.round(Math.max(d01, d12) / 2.5));
    for (let i = 1; i < n; i++) { const p = mix(a, b, i / n), r = mix(d, c, i / n); ctx.moveTo(p[0], p[1]); ctx.lineTo(r[0], r[1]); }
    const m0 = mix(a, d, 0.5), m1 = mix(b, c, 0.5); ctx.moveTo(m0[0], m0[1]); ctx.lineTo(m1[0], m1[1]);
  }
  ctx.stroke();
}
function drawBasins() {
  const list = site.basins || [];
  list.forEach((P, bi) => {
    const top = polyOutline(P); if (top.length < 3) return;
    const info = layout && !layout.error && layout.basins && layout.basins[bi], bottom = info ? info.bottom : [], run = info ? info.run : 0;
    if (info && info.outer) { ctx.beginPath(); for (const r of info.outer) pathPoly(r); ctx.fillStyle = C.island; ctx.fill("evenodd"); }
    fillPoly(top, C.island);
    if (bottom.length) {
      ctx.beginPath(); for (const r of bottom) pathPoly(r); ctx.fillStyle = "rgba(20,40,25,.08)"; ctx.fill("evenodd");
    }
    ctx.globalAlpha = 0.55; ctx.strokeStyle = C["tree-edge"]; ctx.lineCap = "butt";
    if (run > 0) {
      // Barbules : normale intérieure (haut de talus en sens trigonométrique : à gauche), coupée au pied de talus.
      const ccw = polyArea(top) > 0, step = Math.max(1, 5 / view.s);
      const hitDist = (o, d) => { let best = Infinity; for (const r of bottom) for (let k = 0; k < r.length; k++) { const a = r[k], b = r[(k + 1) % r.length], ex = b[0] - a[0], ey = b[1] - a[1], den = d[0] * ey - d[1] * ex; if (Math.abs(den) < 1e-9) continue; const t = ((a[0] - o[0]) * ey - (a[1] - o[1]) * ex) / den, u = ((a[0] - o[0]) * d[1] - (a[1] - o[1]) * d[0]) / den; if (t > 1e-6 && u >= 0 && u <= 1) best = Math.min(best, t); } return best; };
      ctx.beginPath(); let carry = 0, n = 0;
      for (let k = 0; k < top.length; k++) {
        const a = top[k], b = top[(k + 1) % top.length], L = Math.hypot(b[0] - a[0], b[1] - a[1]); if (L < 1e-6) continue;
        const ux = (b[0] - a[0]) / L, uy = (b[1] - a[1]) / L, nx = ccw ? -uy : uy, ny = ccw ? ux : -ux;
        for (let s = step - carry; s <= L; s += step, n++) {
          const o = [a[0] + ux * s, a[1] + uy * s], len = Math.min((n % 2 ? 0.5 : 1) * run, bottom.length ? hitDist(o, [nx, ny]) : Infinity);
          if (!(len > 0.05)) continue;
          const p0 = toS(o), p1 = toS([o[0] + nx * len, o[1] + ny * len]); ctx.moveTo(p0[0], p0[1]); ctx.lineTo(p1[0], p1[1]);
        }
        carry = (carry + L) % step;
      }
      ctx.lineWidth = 0.5; ctx.stroke();
    }
    if (bottom.length) { ctx.beginPath(); for (const r of bottom) pathPoly(r); ctx.lineWidth = 0.6; ctx.stroke(); }
    ctx.beginPath(); pathPoly(top); ctx.lineWidth = 1; ctx.stroke();
    ctx.globalAlpha = 1;
    if (view.s > 2.2) {
      const c = toS([top.reduce((s, p) => s + p[0], 0) / top.length, top.reduce((s, p) => s + p[1], 0) / top.length]);
      ctx.fillStyle = C["building-label"]; ctx.textAlign = "center"; ctx.textBaseline = "middle";
      ctx.font = `600 ${Math.min(12, 1.6 * view.s)}px ${getComputedStyle(document.body).fontFamily}`;
      ctx.fillText("Bassin · H " + fmt1(params.basin_h) + " m", c[0], c[1]);
    }
  });
}
function fillPoly(pts, color) { ctx.beginPath(); pathPoly(pts); ctx.fillStyle = color; ctx.fill(); }

let needsDraw = false;
function redraw() { if (!needsDraw) { needsDraw = true; requestAnimationFrame(draw); } }

function draw() {
  pills = [];
  needsDraw = false;
  ctx.setTransform(DPR, 0, 0, DPR, 0, 0);
  const empty = mode === "edit" && !site.parcel.length && !baseShown();
  if ($("#empty").hidden === empty) $("#empty").hidden = !empty;
  $("#b-fit").disabled = !workBounds();
  if (mode === "map") { drawMap(); return; }
  ctx.fillStyle = C.ground; ctx.fillRect(0, 0, W, H);
  drawGrid();
  drawBasemap();
  if (!site.parcel.length) { drawCadSel(); drawScale(view.s); drawDraft(); return; }

  // Parcelle (pleine terre), translucide sur la photo aérienne
  ctx.beginPath(); pathPoly(outline()); ctx.fillStyle = C.parcel; ctx.globalAlpha = baseShown() && VIS.photo && basemap.img ? 0.55 : 1; ctx.fill(); ctx.globalAlpha = 1;
  // Calque Bâtiments et espaces verts : espaces verts imposés (leurs arbres relèvent de l'habillage)
  if (VIS.pbati) site.landscapes.forEach((p) => fillPoly(polyOutline(p), C.island));
  if (VIS.pbati) drawBasins();

  if (layout && !layout.error) {
    // Calque Structure : dalle du parc en ouvrage
    if (layout.ouvrage && VIS.structure) { ctx.beginPath(); pathPoly(layout.ouvrage.dalle); ctx.fillStyle = C.slab; ctx.fill(); }
    // Calque Voirie
    if (VIS.pvoirie) {
      ctx.fillStyle = C.asphalt;
      ctx.beginPath(); for (const r of layout.paved) pathPoly(r); ctx.fill("evenodd");
      ctx.beginPath(); for (const q of layout.pads) pathPoly(q); for (const q of layout.cross) pathPoly(q); for (const q of layout.entries) pathPoly(q); ctx.fill();
      if (layout.ouvrage) drawRampes(layout.ouvrage.rampes);
    }
    // Calque Places
    if (VIS.places) {
      // Parc en ouvrage : places propres au niveau affiché (rez ou dernier niveau), en plus de l'étage courant.
      const places = layout.ouvrage ? nivStalls() : layout.stalls;
      ctx.fillStyle = C.asphalt;
      ctx.beginPath(); for (const st of places) pathPoly(st.q); ctx.fill();
      const kinds = [null, C.pmr, C.irve, C["pmr-irve"]];
      for (let k = 1; k <= 3; k++) { ctx.beginPath(); for (const st of places) if (st.k === k) pathPoly(st.q); ctx.fillStyle = kinds[k]; ctx.fill(); }
      // Borne installée : repère clair au fond de la place (sinon la place IRVE est seulement pré-équipée).
      ctx.fillStyle = C.stripe; ctx.beginPath();
      for (const st of places) if (st.e) { const q = st.q.map(toS), x = (q[0][0] + q[1][0]) / 2, y = (q[0][1] + q[1][1]) / 2, cx = (q[0][0] + q[1][0] + q[2][0] + q[3][0]) / 4, cy = (q[0][1] + q[1][1] + q[2][1] + q[3][1]) / 4, r = 0.3 * view.s; const px = x + (cx - x) * 0.45, py = y + (cy - y) * 0.45; ctx.moveTo(px + r, py); ctx.arc(px, py, r, 0, 6.2832); }
      ctx.fill();
      ctx.strokeStyle = C.stripe; ctx.lineWidth = Math.max(0.6, 0.12 * view.s); ctx.lineCap = "butt";
      ctx.beginPath();
      for (const st of places) { const q = st.q.map(toS); ctx.moveTo(q[3][0], q[3][1]); ctx.lineTo(q[0][0], q[0][1]); ctx.lineTo(q[1][0], q[1][1]); ctx.lineTo(q[2][0], q[2][1]); }
      ctx.stroke();
      if (layout.ouvrage && layout.ouvrage.controle && nivActif() === "rdc") drawControle(layout.ouvrage.controle);
    }
    if (layout.ouvrage && VIS.noyaux) drawNoyaux(layout.ouvrage.noyaux);
    if (layout.ouvrage && VIS.structure) drawStructure(layout.ouvrage);
    if (layout.ouvrage && VIS.girations) drawGirations(nivGirations());
    // Calque Sens de circulation : flèches peintes sur la chaussée
    if (VIS.sens && view.s > 1.0 && layout.circulation && layout.circulation.arrows.length) {
      planTransform(ctx, DPR, view.s, view.ox, view.oy); ctx.fillStyle = C.stripe; ctx.fill(arrowsPath(layout)); ctx.setTransform(DPR, 0, 0, DPR, 0, 0);
    }
    // Calque Îlots plantés : espaces plantés (arrondis côté chaussée), puis îlots des rangées (rectangulaires)
    if (VIS.ilots) {
      ctx.fillStyle = C.island;
      ctx.beginPath(); for (const poly of layout.planted || []) for (const r of poly) pathPoly(r); ctx.fill("evenodd");
      ctx.beginPath(); for (const q of layout.islands) pathPoly(q); ctx.fill();
      drawNoues();
    }
    if (VIS.arbres && !layout.ouvrage) drawCanopies();
    // Couche Habillage : véhicules sous les arbres
    drawDressing();
    drawOmbrieres();
    if (SKEL) { drawSkeleton(); drawTraverses(); }
    if (NSKEL) drawNoueSkeleton(); else noueBtns = [];
  }

  // Calque Servitudes
  if (VIS.servitudes) site.exclusions.forEach((e) => {
    ctx.save(); ctx.beginPath(); pathPoly(polyOutline(e)); ctx.clip();
    ctx.fillStyle = C.excl; ctx.globalAlpha = 0.3; ctx.fillRect(0, 0, W, H); ctx.globalAlpha = 1;
    ctx.fillStyle = hatchPattern(); ctx.fillRect(0, 0, W, H); ctx.restore();
    ctx.beginPath(); pathPoly(polyOutline(e)); ctx.strokeStyle = C.excl; ctx.lineWidth = 1.2; ctx.stroke();
  });
  // Calque Bâtiments et espaces verts : bâtiments
  if (VIS.pbati) site.buildings.forEach((b) => {
    const p = rectPoly(b);
    fillPoly(p, C.building); ctx.strokeStyle = C["building-edge"]; ctx.lineWidth = 1.2; ctx.stroke();
    if (view.s > 2.2) {
      ctx.fillStyle = C["building-label"]; ctx.textAlign = "center"; ctx.textBaseline = "middle";
      ctx.font = `600 ${Math.min(14, 2.2 * view.s)}px ${getComputedStyle(document.body).fontFamily}`;
      const cent = toS(labelPoint(b));
      ctx.fillText("Bâtiment", cent[0], cent[1] - 9);
      ctx.font = `400 ${Math.min(12, 1.9 * view.s)}px ${getComputedStyle(document.body).fontFamily}`;
      ctx.fillText(fmtInt(Math.abs(polyArea(p))) + " m²", cent[0], cent[1] + 8);
    }
  });
  // Entrées des bâtiments : seuil de la largeur réglée, cheminement à travers le trottoir.
  if (VIS.pbati) site.buildings.forEach((b, bi) => {
    const d = doorGeom(b), hw = Math.min(params.door_w, d.L) / 2, m = params.building_margin, at = (s, o) => toS([d.p[0] + d.u[0] * s + d.n[0] * o, d.p[1] + d.u[1] * s + d.n[1] * o]);
    const wk = layout && layout.walks && layout.walks[bi], q = wk ? wk.map(toS) : [at(-hw, 0), at(hw, 0), at(hw, m), at(-hw, m)];
    ctx.beginPath(); q.forEach((v, k) => (k ? ctx.lineTo(v[0], v[1]) : ctx.moveTo(v[0], v[1]))); ctx.closePath(); ctx.fillStyle = C.stripe; ctx.globalAlpha = 0.5; ctx.fill(); ctx.globalAlpha = 1;
    ctx.beginPath(); ctx.moveTo(q[0][0], q[0][1]); ctx.lineTo(q[1][0], q[1][1]); ctx.strokeStyle = C.accent; ctx.lineWidth = 4; ctx.lineCap = "butt"; ctx.stroke();
    const c = toS(d.p), on = sel && sel.type === "building" && sel.i === site.buildings.indexOf(b);
    ctx.beginPath(); ctx.arc(c[0], c[1], on ? 5 : 3.5, 0, Math.PI * 2); ctx.fillStyle = on ? C.accent : C.ground; ctx.fill(); ctx.strokeStyle = C.accent; ctx.lineWidth = 1.6; ctx.stroke();
  });
  if (layout && layout.entrances) for (const e of layout.entrances) { const p = toS(e); ctx.beginPath(); ctx.arc(p[0], p[1], 4, 0, Math.PI * 2); ctx.fillStyle = C.setback; ctx.fill(); }

  // Limite de parcelle et recul
  ctx.beginPath(); pathPoly(outline()); ctx.strokeStyle = C["parcel-edge"]; ctx.lineWidth = 1.5; ctx.stroke();
  if (VIS.recul && layout && layout.setback_line) { ctx.setLineDash([6, 4]); ctx.strokeStyle = C.setback; ctx.lineWidth = 1.2; ctx.beginPath(); for (const r of layout.setback_line) pathPoly(r); ctx.stroke(); ctx.setLineDash([]); }

  drawPlaces();
  drawGir();
  drawCoreDrag();
  drawCadSel();
  drawHandles();
  drawEdgeBtns();
  drawSelDel();
  drawScale(view.s);
  drawDraft();
  drawGirDraft();
}

// Rampes du silo : dalle en enrobé, axe fléché dans le sens de la montée (ou de la descente), sens et
// pente au milieu de l'axe.
function drawRampes(rs) {
  if (!rs.length) return;
  const font = getComputedStyle(document.body).fontFamily;
  // Un plateau incliné n'est pas une emprise de rampe : seulement sa flèche de pente.
  ctx.fillStyle = C.asphalt; ctx.beginPath(); for (const r of rs) if (r.forme !== "plateau") { pathPoly(r.contour); pathPoly(r.trou || []); } ctx.fill("evenodd");
  ctx.strokeStyle = C.stripe; ctx.lineWidth = 1; ctx.stroke();
  for (const r of rs) {
    const a = r.axe.map(toS), n = a.length;
    ctx.beginPath(); ctx.moveTo(a[0][0], a[0][1]); for (let i = 1; i < n; i++) ctx.lineTo(a[i][0], a[i][1]); ctx.lineWidth = 1.5; ctx.stroke();
    const B = a[n - 1], P = a[n - 2], L = Math.hypot(B[0] - P[0], B[1] - P[1]) || 1, ux = (B[0] - P[0]) / L, uy = (B[1] - P[1]) / L;
    ctx.beginPath(); ctx.moveTo(B[0] - ux * 7 - uy * 4, B[1] - uy * 7 + ux * 4); ctx.lineTo(B[0], B[1]); ctx.lineTo(B[0] - ux * 7 + uy * 4, B[1] - uy * 7 - ux * 4); ctx.stroke();
    if (view.s > 2.5) {
      ctx.save();
      ctx.font = `600 11px ${font}`; ctx.textAlign = "center"; ctx.textBaseline = "bottom"; ctx.fillStyle = C.stripe;
      const nom = { u: "Demi-rampe", virage: "Demi-tour", plateau: "Pente" }[r.forme] || (r.montee ? "Montée" : "Descente");
      const txt = r.forme === "virage" ? nom : `${nom} ${(r.pente * 100).toFixed(r.forme === "droite" ? 0 : 1).replace(".", ",")} %`;
      if (r.forme === "helice") arcText(txt, a, toS(r.de), font);
      else {
        const k = Math.floor((n - 1) / 2), M = [(a[k][0] + a[k + 1][0]) / 2, (a[k][1] + a[k + 1][1]) / 2];
        let t = Math.atan2(a[k + 1][1] - a[k][1], a[k + 1][0] - a[k][0]); if (t > Math.PI / 2 || t < -Math.PI / 2) t += Math.PI;
        ctx.translate(M[0], M[1]); ctx.rotate(t); ctx.fillText(txt, 0, -4);
      }
      ctx.restore();
    }
  }
}

// Libellé d'une tour hélicoïdale : texte courbe le long de l'axe de sa voie (cercle passant par
// trois points de l'axe a, repère écran), centré sur le point opposé à la porte D, dans l'axe du
// parking, juste au-delà de l'axe, à la taille de la tour jusqu'à 11 px. Lettres tête vers
// l'extérieur, ou vers l'intérieur quand elles seraient à l'envers ; texte vertical lu de bas en
// haut.
function arcText(txt, a, D, font) {
  const [A, B, E] = [a[0], a[Math.floor(a.length / 3)], a[Math.floor((2 * a.length) / 3)]];
  const d = 2 * (A[0] * (B[1] - E[1]) + B[0] * (E[1] - A[1]) + E[0] * (A[1] - B[1]));
  if (Math.abs(d) < 1e-9) return;
  const q = (P) => P[0] * P[0] + P[1] * P[1];
  const C = [(q(A) * (B[1] - E[1]) + q(B) * (E[1] - A[1]) + q(E) * (A[1] - B[1])) / d, (q(A) * (E[0] - B[0]) + q(B) * (A[0] - E[0]) + q(E) * (B[0] - A[0])) / d];
  const phi = Math.atan2(C[1] - D[1], C[0] - D[0]), t = phi + Math.PI / 2;
  const fs = Math.min(11, 2 * view.s), s = Math.cos(t) > 0.01 || (Math.cos(t) > -0.01 && Math.sin(t) < 0) ? 1 : -1;
  const rb = Math.hypot(A[0] - C[0], A[1] - C[1]) + 3 + fs / 2;
  ctx.font = `600 ${fs}px ${font}`; ctx.textBaseline = "middle";
  const ws = [...txt].map((ch) => ctx.measureText(ch).width);
  let ang = phi - (s * ws.reduce((x, y) => x + y, 0)) / (2 * rb);
  [...txt].forEach((ch, i) => {
    const m = ang + (s * ws[i]) / (2 * rb);
    ctx.save(); ctx.translate(C[0] + rb * Math.cos(m), C[1] + rb * Math.sin(m)); ctx.rotate(m + (s * Math.PI) / 2); ctx.fillText(ch, 0, 0); ctx.restore();
    ang += (s * ws[i]) / rb;
  });
}

// Épures de giration des rampes (NF P 91-100, § 3.3) : bande balayée, trajectoire en tirets
// terminée d'une pointe ; en rouge quand un obstacle s'y trouve.
function drawGirations(gs) {
  for (const g of gs) {
    const col = g.conflits.length ? C.bad : C.accent;
    ctx.beginPath(); pathPoly(g.contour); ctx.fillStyle = col; ctx.globalAlpha = 0.14; ctx.fill();
    ctx.globalAlpha = 0.85; ctx.strokeStyle = col; ctx.lineWidth = 1; ctx.stroke();
    const a = g.axe.map(toS), n = a.length;
    ctx.beginPath(); ctx.moveTo(a[0][0], a[0][1]); for (let i = 1; i < n; i++) ctx.lineTo(a[i][0], a[i][1]);
    ctx.setLineDash([6, 4]); ctx.lineWidth = 1.3; ctx.stroke(); ctx.setLineDash([]);
    const B = a[n - 1], P = a[n - 2], L = Math.hypot(B[0] - P[0], B[1] - P[1]) || 1, ux = (B[0] - P[0]) / L, uy = (B[1] - P[1]) / L;
    ctx.beginPath(); ctx.moveTo(B[0] - ux * 7 - uy * 4, B[1] - uy * 7 + ux * 4); ctx.lineTo(B[0], B[1]); ctx.lineTo(B[0] - ux * 7 + uy * 4, B[1] - uy * 7 - ux * 4); ctx.stroke();
    ctx.globalAlpha = 1;
  }
}

// Escaliers et ascenseurs (PS 13) dans l'emprise d'une place : cage d'escalier à deux volées
// (marches de 0,28 m entre deux paliers, mur d'échiffre au milieu), gaine d'ascenseur barrée d'une
// croix ; la porte, sur l'allée, en trait épais.
function drawNoyaux(ns) {
  ctx.save();
  const moving = coreMoving();
  for (const n of ns) {
    if (moving && moving.n === n) { ctx.beginPath(); pathPoly(n.q); ctx.setLineDash([4, 3]); ctx.strokeStyle = C.building; ctx.lineWidth = 1; ctx.stroke(); ctx.setLineDash([]); continue; }
    drawCore(n, n.q, sel && sel.type === "core" && layout.ouvrage.noyaux[sel.i] === n);
  }
  ctx.restore();
}
function drawCore(n, q, on) {
  const lerp = (a, b, t) => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
  const fond = [q[0], q[1]], av = [q[3], q[2]];
  const larg = Math.hypot(q[1][0] - q[0][0], q[1][1] - q[0][1]), prof = Math.hypot(q[3][0] - q[0][0], q[3][1] - q[0][1]);
  ctx.beginPath(); pathPoly(q); ctx.fillStyle = C.building; ctx.globalAlpha = 0.18; ctx.fill(); ctx.globalAlpha = 1;
  ctx.strokeStyle = on ? C.accent : C.building; ctx.lineWidth = on ? 2.5 : 1.5; ctx.stroke();
  ctx.beginPath(); ctx.lineWidth = 0.8; ctx.strokeStyle = C.building;
  const seg = (a, b) => { const A = toS(a), B = toS(b); ctx.moveTo(A[0], A[1]); ctx.lineTo(B[0], B[1]); };
  if (n.ascenseur) { seg(q[0], q[2]); seg(q[1], q[3]); }
  else if (n.liaison) {
    // Liaison : deux demi-volées côte à côte (montée et descente d'une demi-hauteur) au droit du
    // joint, paliers de part et d'autre.
    const t0 = 0.5 - 1.25 / prof, t1 = 0.5 + 1.25 / prof, pas = 0.28 / prof;
    seg(lerp(lerp(q[0], q[1], 0.5), lerp(q[3], q[2], 0.5), t0), lerp(lerp(q[0], q[1], 0.5), lerp(q[3], q[2], 0.5), t1));
    for (let t = t0; t <= t1 + 1e-9; t += view.s * 0.28 > 2.5 ? pas : t1 - t0) seg(lerp(fond[0], av[0], t), lerp(fond[1], av[1], t));
  } else {
    // Paliers de 1,00 m à chaque bout, volées entre les deux, échiffre au milieu de la largeur.
    const t0 = Math.min(0.45, 1 / prof), t1 = 1 - t0, pas = 0.28 / prof;
    seg(lerp(lerp(q[0], q[1], 0.5), lerp(q[3], q[2], 0.5), t0), lerp(lerp(q[0], q[1], 0.5), lerp(q[3], q[2], 0.5), t1));
    if (view.s * 0.28 > 2.5) for (let t = t0; t <= t1 + 1e-9; t += pas) seg(lerp(fond[0], av[0], t), lerp(fond[1], av[1], t));
    else { seg(lerp(fond[0], av[0], t0), lerp(fond[1], av[1], t0)); seg(lerp(fond[0], av[0], t1), lerp(fond[1], av[1], t1)); }
  }
  ctx.stroke();
  const d = Math.min(0.45, 0.9 / 2 / larg);
  ctx.beginPath(); seg(lerp(av[0], av[1], 0.5 - d), lerp(av[0], av[1], 0.5 + d));
  if (n.liaison) seg(lerp(fond[0], fond[1], 0.5 - d), lerp(fond[0], fond[1], 0.5 + d));
  ctx.lineWidth = 3; ctx.strokeStyle = C.accent; ctx.stroke();
}

// Escaliers et ascenseurs déplacés à la main. La cage se soulève et suit le doigt ; la place visée
// s'ouvre (marquage effacé, contour pointillé) ; à la dépose, la cage se cale dans la place et le
// moteur refait le bilan. Les positions vont dans site.noyaux (points du site, recalés par le moteur
// sur la place la plus proche) ; sans saisie, placement automatique.
const LIFT = 1.06, OPEN_MS = 160, SETTLE_MS = 180;
let coreAnim = null;
const qCentre = (q) => [(q[0][0] + q[2][0]) / 2, (q[0][1] + q[2][1]) / 2];
const natureOf = (n) => (n.ascenseur ? "ascenseur" : n.liaison ? "liaison" : "escalier");
const CORE_NAME = { escalier: "Escalier", ascenseur: "Ascenseur", liaison: "Escalier de liaison" };
const coresOn = () => !!(layout && layout.ouvrage && layout.ouvrage.evacuation);
function coreMoving() { return drag && drag.kind === "core" && drag.moved ? drag.core : coreAnim; }
function ensureCores() { if (!site.noyaux) site.noyaux = layout.ouvrage.noyaux.map((n) => ({ nature: natureOf(n), p: qCentre(n.q).map((v) => +v.toFixed(3)) })); }
function coreEntry(n) {
  const c = qCentre(n.q), nat = natureOf(n); let best = -1, bd = Infinity;
  site.noyaux.forEach((e, k) => { const d = dist2(e.p, c); if (e.nature === nat && d < bd) { bd = d; best = k; } });
  return best;
}
function coreNear(p, nat) {
  const N = layout && layout.ouvrage ? layout.ouvrage.noyaux : []; let best = -1, bd = Infinity;
  N.forEach((n, k) => { const d = dist2(qCentre(n.q), p); if ((!nat || natureOf(n) === nat) && d < bd) { bd = d; best = k; } });
  return best;
}
// Emplacements possibles : places libres (deux places dos à dos de part et d'autre du joint pour
// l'escalier de liaison), et la place d'origine.
function coreTargets(n) {
  const others = layout.ouvrage.noyaux.filter((m) => m !== n), busy = (c) => others.some((m) => pointInPoly(c, m.q)), S = layout.stalls, T = [];
  if (n.liaison) {
    for (let a = 0; a < S.length; a++) for (let b = a + 1; b < S.length; b++) {
      const ca = qCentre(S[a].q), cb = qCentre(S[b].q), dx = cb[0] - ca[0], dy = cb[1] - ca[1], L = Math.hypot(dx, dy);
      if (L < 5.05 || L > 7.5) continue;
      const ax = [S[a].q[3][0] - S[a].q[0][0], S[a].q[3][1] - S[a].q[0][1]];
      if (Math.abs(ax[0] * dy - ax[1] * dx) / Math.hypot(ax[0], ax[1]) > 0.15) continue;
      const c = [(ca[0] + cb[0]) / 2, (ca[1] + cb[1]) / 2];
      if (!busy(c)) T.push({ c, q: [S[a].q, S[b].q] });
    }
  } else for (const st of S) { const c = qCentre(st.q); if (!busy(c)) T.push({ c, q: [st.q] }); }
  T.push({ c: qCentre(n.q), q: [n.q], home: true });
  return T;
}
const easeOut = (t) => 1 - (1 - t) ** 3;
function drawCoreDrag() {
  const m = coreMoving(); if (!m) return;
  const now = performance.now(), c0 = qCentre(m.n.q);
  let cur = m.cur || c0, s = LIFT, lift = 1, again = false;
  if (m === coreAnim) { const k = Math.min(1, (now - m.t0) / SETTLE_MS), e = easeOut(k); cur = [m.from[0] + (m.to[0] - m.from[0]) * e, m.from[1] + (m.to[1] - m.from[1]) * e]; s = LIFT + (1 - LIFT) * e; lift = 1 - e; again = k < 1; }
  // Place visée : le marquage s'efface, contour pointillé.
  const t = m.target;
  if (t && !t.home) {
    const k = Math.min(1, (now - (m.tt || 0)) / OPEN_MS); again = again || k < 1;
    for (const q of t.q) { ctx.beginPath(); pathPoly(q); ctx.globalAlpha = 0.85 * easeOut(k); ctx.fillStyle = C.asphalt; ctx.fill(); ctx.globalAlpha = 1; }
    ctx.beginPath(); for (const q of t.q) pathPoly(q); ctx.setLineDash([5, 4]); ctx.strokeStyle = C.accent; ctx.lineWidth = 1.5; ctx.stroke(); ctx.setLineDash([]);
  }
  // Cage soulevée : agrandie autour de son centre, ombre portée.
  const q = m.n.q.map((p) => [cur[0] + (p[0] - c0[0]) * s, cur[1] + (p[1] - c0[1]) * s]);
  ctx.save();
  ctx.shadowColor = `rgba(0, 0, 0, ${0.3 * lift})`; ctx.shadowBlur = 16 * lift; ctx.shadowOffsetY = 6 * lift;
  ctx.beginPath(); pathPoly(q); ctx.fillStyle = C.slab; ctx.fill();
  ctx.restore();
  drawCore(m.n, q, true);
  if (again) redraw();
}
function dropCore(c) {
  const t = c.target && !c.target.home ? c.target : null, c0 = qCentre(c.n.q);
  coreAnim = { n: c.n, from: c.cur || c0, to: t ? t.c : c0, t0: performance.now(), target: c.target, tt: c.tt };
  redraw();
  setTimeout(() => {
    if (t) {
      ensureCores(); const k = coreEntry(c.n);
      if (k >= 0) site.noyaux[k].p = t.c.map((v) => +v.toFixed(3));
      coreAnim = null; solve(false);
      const i = coreNear(t.c, natureOf(c.n)); setSel(i >= 0 ? { type: "core", i } : null);
      commit(); markMine();
    } else { coreAnim = null; redraw(); }
  }, SETTLE_MS);
}

// Parc en ouvrage : contour de dalle, poutres et solives (sous la dalle : tirets), files de la trame
// (trait mixte, repère au bout), poteaux.
function drawStructure(o) {
  const font = getComputedStyle(document.body).fontFamily;
  ctx.beginPath(); pathPoly(o.dalle); ctx.strokeStyle = C.building; ctx.lineWidth = 1.5; ctx.stroke();
  if (o.poutres.length) {
    ctx.save(); ctx.beginPath(); for (const q of o.poutres) pathPoly(q);
    ctx.globalAlpha = 0.12; ctx.fillStyle = C.building; ctx.fill(); ctx.globalAlpha = 1;
    ctx.setLineDash([5, 3]); ctx.strokeStyle = C.building; ctx.lineWidth = 1; ctx.stroke(); ctx.restore();
  }
  if (view.s > 0.8) {
    ctx.save(); ctx.strokeStyle = C.accent; ctx.globalAlpha = 0.55; ctx.lineWidth = 1; ctx.setLineDash([10, 3, 2, 3]);
    ctx.beginPath(); for (const a of o.axes) { const A = toS(a.a), B = toS(a.b); ctx.moveTo(A[0], A[1]); ctx.lineTo(B[0], B[1]); } ctx.stroke(); ctx.restore();
    const r = 9;
    ctx.font = `600 10px ${font}`; ctx.textAlign = "center"; ctx.textBaseline = "middle";
    for (const a of o.axes) {
      const A = toS(a.a), B = toS(a.b), L = Math.hypot(A[0] - B[0], A[1] - B[1]) || 1, c = [A[0] + (A[0] - B[0]) / L * r, A[1] + (A[1] - B[1]) / L * r];
      ctx.beginPath(); ctx.arc(c[0], c[1], r, 0, Math.PI * 2); ctx.fillStyle = C.ground; ctx.fill(); ctx.strokeStyle = C.accent; ctx.lineWidth = 1.2; ctx.stroke();
      ctx.fillStyle = C.accent; ctx.fillText(a.nom, c[0], c[1] + 0.5);
    }
  }
  ctx.fillStyle = C.building; ctx.beginPath(); for (const q of o.poteaux) pathPoly(q); ctx.fill();
}

// Point d'étiquette : centre de l'emprise, ou de la plus longue aile pour un L.
function labelPoint(o) {
  if (o.shape !== "L") return [o.cx, o.cy];
  const t = Math.max(6, Math.min(o.w, o.h) * 0.4);
  const local = o.w >= o.h ? [0, -o.h / 2 + t / 2] : [-o.w / 2 + t / 2, 0];
  const r = rot(local, o.rot);
  return [o.cx + r[0], o.cy + r[1]];
}


// Flèche : trait d'axe et chevron pointant dans la direction (ux, uy), en pixels écran.
function arrow(x, y, ux, uy, k, color, width) {
  const px = -uy, py = ux, tx = x + ux * k, ty = y + uy * k;
  ctx.strokeStyle = color; ctx.lineWidth = width; ctx.lineCap = "round"; ctx.lineJoin = "round"; ctx.beginPath();
  ctx.moveTo(x - ux * k, y - uy * k); ctx.lineTo(tx, ty);
  ctx.moveTo(tx - ux * k * 0.8 + px * k * 0.65, ty - uy * k * 0.8 + py * k * 0.65); ctx.lineTo(tx, ty);
  ctx.lineTo(tx - ux * k * 0.8 - px * k * 0.65, ty - uy * k * 0.8 - py * k * 0.65);
  ctx.stroke();
}

function drawGrid() {
  const steps = [5, 10, 25, 50, 100, 250];
  const st = steps.find((s) => s * view.s >= 28) || 500, big = st * 5;
  const [wx0, wy1] = toW(0, 0), [wx1, wy0] = toW(W, H);
  for (const [step, color] of [[st, C.grid], [big, C["grid-strong"]]]) {
    ctx.strokeStyle = color; ctx.lineWidth = 1; ctx.beginPath();
    for (let x = Math.floor(wx0 / step) * step; x <= wx1; x += step) { const sx = Math.round(x * view.s + view.ox) + 0.5; ctx.moveTo(sx, 0); ctx.lineTo(sx, H); }
    for (let y = Math.floor(wy0 / step) * step; y <= wy1; y += step) { const sy = Math.round(-y * view.s + view.oy) + 0.5; ctx.moveTo(0, sy); ctx.lineTo(W, sy); }
    ctx.stroke();
  }
}

function drawScale(pxPerM) {
  const target = 110 / pxPerM, nice = [1, 2, 5, 10, 20, 25, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000, 50000, 100000, 200000];
  const m = nice.reduce((b, v) => (Math.abs(v - target) < Math.abs(b - target) ? v : b), nice[0]);
  const len = m * pxPerM, narrow = W < 760;
  // Plan sur grand écran : en haut à droite, à côté du bouton de cadrage ; sinon en bas à droite.
  const top = !narrow && mode !== "map";
  const x = W - (narrow ? 16 : top ? 74 : 24) - len, y = top ? 44 : H - (narrow ? 96 : 30);
  const text = m >= 1000 ? m / 1000 + " km" : m + " m";
  ctx.beginPath(); ctx.moveTo(x, y - 5); ctx.lineTo(x, y); ctx.lineTo(x + len, y); ctx.lineTo(x + len, y - 5); ctx.moveTo(x + len / 2, y); ctx.lineTo(x + len / 2, y - 3);
  ctx.font = `600 11px ${getComputedStyle(document.body).fontFamily}`; ctx.textAlign = "right"; ctx.textBaseline = "bottom"; ctx.lineJoin = "round";
  ctx.strokeStyle = C.ground; ctx.lineWidth = 4; ctx.stroke(); ctx.strokeText(text, x + len, y - 7); // liseré lisible sur la photo
  ctx.strokeStyle = C.label; ctx.fillStyle = C.label; ctx.lineWidth = 1.5; ctx.stroke(); ctx.fillText(text, x + len, y - 7);
}

// ---- Poignées
// Limite i de la parcelle : segment droit, ou arc de cercle si site.arcs[i] ≠ 0 (flèche / corde, signée,
// positive vers l'extérieur). at(t) : point à la fraction t de l'angle ; nAt(t) : normale intérieure ;
// N : nombre de côtés quand l'arc est discrétisé (moteur, tracé), un côté tous les 4° environ.
const ARC_MAX = 0.5;
const arcK = (i) => (site.arcs && site.arcs[i]) || 0;
function edgeInfo(i) { return edgeG(site.parcel, i, arcK(i)); }
// `sg` : côté imposé (ligne ouverte, trajectoire de giration : flèche positive à droite du sens de tracé).
function edgeG(P, i, k0, sg) {
  const a = P[i], b = P[(i + 1) % P.length];
  const sgn = sg || Math.sign(polyArea(P)) || 1, dx = b[0] - a[0], dy = b[1] - a[1], L = Math.hypot(dx, dy) || 1;
  const u = [dx / L, dy / L], n = [-u[1] * sgn, u[0] * sgn], mid = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
  const k = Math.max(-ARC_MAX, Math.min(ARC_MAX, k0));
  const e = { a, b, u, n, L, mid, k, apex: mid, N: 1, at: (t) => [a[0] + dx * t, a[1] + dy * t], nAt: () => n };
  if (!k) return e;
  const s = k * L, o = [-n[0], -n[1]], tot = 4 * Math.atan(2 * Math.abs(k)), R = (L * L / 4 + s * s) / (2 * Math.abs(s)), dc = s - Math.sign(s) * R;
  const c = [mid[0] + o[0] * dc, mid[1] + o[1] * dc], a0 = Math.atan2(a[1] - c[1], a[0] - c[0]);
  const am = Math.atan2(mid[1] + o[1] * s - c[1], mid[0] + o[0] * s - c[0]), dir = Math.sign(Math.sin(am - a0)) || 1;
  e.N = Math.max(2, Math.min(48, Math.ceil(tot * 45 / Math.PI)));
  e.at = (t) => { if (t <= 0) return a; if (t >= 1) return b; const f = a0 + dir * tot * t; return [c[0] + R * Math.cos(f), c[1] + R * Math.sin(f)]; };
  e.nAt = (t) => { const p = e.at(t), q = s > 0 ? [c[0] - p[0], c[1] - p[1]] : [p[0] - c[0], p[1] - c[1]]; return [q[0] / R, q[1] / R]; };
  e.apex = e.at(0.5);
  return e;
}
// Contour discrétisé de la parcelle (arcs en côtés) : tracé, cadrage, moteur.
function outline() { const out = []; for (let i = 0; i < site.parcel.length; i++) { const e = edgeInfo(i); for (let j = 0; j < e.N; j++) out.push(j ? e.at(j / e.N) : e.a); } return out; }
// Servitudes et espaces verts : le sommet [x, y, k] porte la flèche k de l'arête qui en part.
const vk = (p) => p[2] || 0;
function polyOutline(P) { const out = []; for (let i = 0; i < P.length; i++) { const e = edgeG(P, i, vk(P[i])); for (let j = 0; j < e.N; j++) out.push(j ? e.at(j / e.N) : [e.a[0], e.a[1]]); } return out; }
// Référence d'une arête : { g: "parcel" | "exclusion" | "landscape" | "basin" | "gir", o: objet, i: arête }.
// "gir" : segment i de la trajectoire de giration, ligne ouverte (n − 1 segments).
function ringOf(r) { return r.g === "parcel" ? site.parcel : r.g === "gir" ? site.gir && site.gir.pts : (site[POLY[r.g]] || [])[r.o]; }
const kOf = (r) => (r.g === "parcel" ? arcK(r.i) : vk(ringOf(r)[r.i]));
function setK(r, k) { if (r.g === "parcel") { ensureArcs(); site.arcs[r.i] = k; } else { const P = ringOf(r), p = P[r.i]; P[r.i] = k ? [p[0], p[1], k] : [p[0], p[1]]; } }
function edgeRef(r) { const P = ringOf(r), open = r.g === "gir"; return P && r.i < P.length - (open ? 1 : 0) ? edgeG(P, r.i, kOf(r), open ? 1 : 0) : null; }
const sameRef = (a, b) => (!a && !b) || (a && b && a.g === b.g && a.o === b.o && a.i === b.i);
// `selectedOnly` : poignées d'arc ; hors parcelle, seuls les contours de l'objet sélectionné en portent.
function eachRing(fn, selectedOnly) {
  const on = (g, o) => !selectedOnly || (sel && sel.type === g && sel.i === o);
  fn({ g: "parcel" }, site.parcel);
  if (VIS.pbati) site.landscapes.forEach((P, o) => on("landscape", o) && fn({ g: "landscape", o }, P));
  if (VIS.pbati) (site.basins || []).forEach((P, o) => on("basin", o) && fn({ g: "basin", o }, P));
  if (VIS.servitudes) site.exclusions.forEach((P, o) => on("exclusion", o) && fn({ g: "exclusion", o }, P));
  if (girEditable() && on("gir")) fn({ g: "gir" }, site.gir.pts);
}
function nearRing(P, kf, w, open) {
  let best = null;
  for (let i = 0; i < P.length - (open ? 1 : 0); i++) {
    const e = edgeG(P, i, kf(i), open ? 1 : 0); let prev = e.a;
    for (let j = 1; j <= e.N; j++) { const q = e.at(j / e.N), [d, t] = distSeg(w, prev, q); if (!best || d < best.d) best = { d, i, t: (j - 1 + t) / e.N }; prev = q; }
  }
  return best;
}
function nearAnyEdge(w) {
  let best = null;
  eachRing((r, P) => { const open = r.g === "gir"; if (P.length < (open ? 2 : 3)) return; const nb = nearRing(P, r.g === "parcel" ? arcK : (i) => vk(P[i]), w, open); if (nb && (!best || nb.d < best.d)) best = { d: nb.d, t: nb.t, ref: { ...r, i: nb.i } }; });
  return best;
}
// Coupe un arc de flèche k au paramètre t : deux arcs du même cercle ; les fusionne.
const arcSplit = (k, t) => { const th = 4 * Math.atan(2 * k); return [Math.tan(th * t / 4) / 2, Math.tan(th * (1 - t) / 4) / 2]; };
const arcJoin = (k1, k2) => Math.max(-ARC_MAX, Math.min(ARC_MAX, Math.tan((4 * Math.atan(2 * k1) + 4 * Math.atan(2 * k2)) / 4) / 2));
const withK = (x, y, k) => (k ? [x, y, +k.toFixed(5)] : [x, y]);
// Même contour pour le moteur : reculs répétés, accès recalés sur le côté qui les porte.
function flatSite() {
  const parcel = [], setbacks = [], off = [];
  for (let i = 0; i < site.parcel.length; i++) { const e = edgeInfo(i); off[i] = parcel.length; for (let j = 0; j < e.N; j++) { parcel.push(j ? e.at(j / e.N) : e.a); setbacks.push(site.setbacks[i] || 0); } }
  const entries = site.entries.map((en) => { if (en.edge >= site.parcel.length) return en; const N = edgeInfo(en.edge).N, u = en.t * N, j = Math.min(N - 1, Math.floor(u)); return { edge: off[en.edge] + j, t: u - j }; });
  return { parcel, setbacks, entries };
}
// Point de la limite le plus proche : { d, i, t }.
function nearEdge(w) { return nearRing(site.parcel, arcK, w); }
// Poignée d'un arc : au-delà du sommet de l'arc (côté extérieur), pour ne pas masquer un accès posé au milieu.
const KNOB = 20;
function arcKnob(r) { const e = edgeRef(r), d = KNOB / view.s * Math.sign(e.k); return [e.apex[0] - e.n[0] * d, e.apex[1] - e.n[1] * d]; }
function setbackHandle(i) { const e = edgeInfo(i), s = Math.max(site.setbacks[i] || 0, 0); return [e.apex[0] + e.n[0] * s, e.apex[1] + e.n[1] * s]; }
function entryPoint(en) { return edgeInfo(en.edge).at(en.t); }
function objCorners(o) { const c = Math.cos(o.rot * Math.PI / 180), s = Math.sin(o.rot * Math.PI / 180), w = o.w / 2, h = o.h / 2; return [[-w, -h], [w, -h], [w, h], [-w, h]].map(([x, y]) => [o.cx + x * c - y * s, o.cy + x * s + y * c]); }
function rotHandle(o) { const c = Math.cos(o.rot * Math.PI / 180), s = Math.sin(o.rot * Math.PI / 180), d = o.h / 2 + 18 / view.s * 1.6; return [o.cx - d * s, o.cy + d * c]; }
// Angle aimanté au degré et aux directions des limites de la parcelle (à 2,5° près).
function snapRot(a) { a = snap(((a % 360) + 360) % 360, 1); for (let i = 0; i < site.parcel.length; i++) { const e = edgeInfo(i); const ea = Math.atan2(e.u[1], e.u[0]) * 180 / Math.PI; for (const k of [0, 90, 180, 270]) { const t = ((ea + k) % 360 + 360) % 360; if (Math.abs(t - a) < 2.5) a = +t.toFixed(2); } } return a; }
// Dalle du silo telle que calculée, au format des bâtiments : w le long des allées, h en travers, rot = direction des allées.
// site.slab (emprise dessinée) est absent tant que l'emprise est automatique.
function slabObj() {
  const ov = layout && layout.ouvrage; if (!ov) return null;
  const d = ov.dalle;
  return { cx: (d[0][0] + d[1][0] + d[2][0] + d[3][0]) / 4, cy: (d[0][1] + d[1][1] + d[2][1] + d[3][1]) / 4, w: ov.longueur, h: ov.largeur, rot: site.slab ? site.slab.rot : ov.angle };
}
const slabOn = () => !!(layout && layout.ouvrage && VIS.structure && mode === "edit");
// Rotation : pastille dans le prolongement du coin opposé aux repères de la trame (toujours dégagé).
function slabRotHandle() { const o = slabObj(), d = layout.ouvrage.dalle[2], v = [d[0] - o.cx, d[1] - o.cy], k = 34 / view.s / (Math.hypot(v[0], v[1]) || 1); return [d[0] + v[0] * k, d[1] + v[1] * k]; }
// Milieux des côtés : 0 et 2 en travers (−h, +h), 1 et 3 en long (+w, −w).
function slabMids(o) { const cs = objCorners(o); return cs.map((c, k) => [(c[0] + cs[(k + 1) % 4][0]) / 2, (c[1] + cs[(k + 1) % 4][1]) / 2]); }
// Après une modification, l'emprise enregistrée prend les dimensions retenues par le moteur.
function syncSlab() { const o = slabObj(); if (site.slab && o) site.slab = { cx: +o.cx.toFixed(3), cy: +o.cy.toFixed(3), w: o.w, h: o.h, rot: site.slab.rot }; }

// Boutons au milieu d'une limite survolée : ajouter un sommet, arrondir le segment. Sur la limite de
// parcelle, ils passent côté extérieur, hors des poignées de recul et des places.
function edgeBtns(r) {
  const e = r && !(r.g === "gir" && !girEditable()) && edgeRef(r); if (!e || e.L * view.s < 100) return [];
  const a = toS(e.apex), d = r.g === "parcel" ? EBTN.out : 0, m = [a[0] - e.n[0] * d, a[1] + e.n[1] * d];
  // Boutons posés sur le segment, dans son inclinaison (écran) : « + » côté gauche (ou haut), arc côté droit.
  let ux = e.u[0], uy = -e.u[1]; if (ux < -1e-6 || (Math.abs(ux) <= 1e-6 && uy > 0)) { ux = -ux; uy = -uy; }
  const ang = Math.atan2(uy, ux);
  return [{ part: "add", x: m[0] - ux * EBTN.gap, y: m[1] - uy * EBTN.gap, ang, mx: m[0], my: m[1] }, { part: "move", x: m[0], y: m[1], ang, mx: m[0], my: m[1] }, { part: "curve", x: m[0] + ux * EBTN.gap, y: m[1] + uy * EBTN.gap, ang, mx: m[0], my: m[1] }];
}
// Icône de suppression près de l'élément sélectionné (poignée, accès ou objet entier).
const ICON_R = 9; // rayon commun des icônes (celui des accès)
let selDelBtn = null;
function selAnchor() {
  if (!sel) return null;
  if (sel.type === "vertex") return site.parcel[sel.i] || null;
  if (sel.type === "entry") { const en = site.entries[sel.i]; return en && en.edge < site.parcel.length ? entryPoint(en) : null; }
  if (sel.type === "building") { const b = site.buildings[sel.i]; return b ? [b.cx, b.cy] : null; }
  if (sel.type === "gir") { const P = girEditable() && site.gir.pts; return !P ? null : sel.v != null ? P[sel.v] || null : girEdge(sel.e != null ? sel.e : 0).apex; }
  if (sel.type === "core") { const n = coresOn() && layout.ouvrage.noyaux[sel.i]; return n && !coreMoving() ? qCentre(n.q) : null; }
  if (POLY[sel.type]) {
    const e = site[POLY[sel.type]][sel.i]; if (!e) return null;
    if (sel.v != null && e[sel.v]) return e[sel.v];
    const o = polyOutline(e); return [o.reduce((a, q) => a + q[0], 0) / o.length, o.reduce((a, q) => a + q[1], 0) / o.length];
  }
  return null;
}
function drawSelDel() {
  selDelBtn = null; if (mode !== "edit") return;
  const a = selAnchor(); if (!a) return;
  const q = toS(a), x = Math.max(ICON_R + 4, Math.min(W - ICON_R - 4, q[0] + 24)), y = Math.max(ICON_R + 4, Math.min(H - ICON_R - 4, q[1] - 24));
  selDelBtn = { x, y };
  ctx.save(); ctx.translate(x, y); ctx.shadowColor = "rgba(0, 0, 0, .22)"; ctx.shadowBlur = 6; ctx.shadowOffsetY = 1.5;
  ctx.beginPath(); ctx.arc(0, 0, ICON_R, 0, Math.PI * 2); ctx.fillStyle = C["glass-solid"]; ctx.fill();
  ctx.shadowColor = "transparent"; ctx.strokeStyle = "#d93a2f"; ctx.lineWidth = 1.2; ctx.stroke();
  ctx.lineWidth = 1.6; ctx.lineCap = "round"; ctx.beginPath(); ctx.moveTo(-3.6, -3.6); ctx.lineTo(3.6, 3.6); ctx.moveTo(3.6, -3.6); ctx.lineTo(-3.6, 3.6); ctx.stroke();
  ctx.restore();
}
function deleteHandle() {
  if (sel && POLY[sel.type] && sel.v != null && site[POLY[sel.type]][sel.i].length > 3) { removePolyVertex(sel.type, sel.i, sel.v); return; }
  deleteSel();
}
function drawEdgeBtns() {
  if (mode !== "edit") return;
  for (const b of edgeBtns(hoverEdge)) {
    ctx.save(); ctx.translate(b.x, b.y);
    ctx.shadowColor = "rgba(0, 0, 0, .22)"; ctx.shadowBlur = 6; ctx.shadowOffsetY = 1.5;
    const on = b.part === "curve" && kOf(hoverEdge) !== 0;
    ctx.beginPath(); ctx.arc(0, 0, EBTN.r, 0, Math.PI * 2); ctx.fillStyle = on ? C.accent : C["glass-solid"]; ctx.fill();
    ctx.shadowColor = "transparent"; ctx.strokeStyle = C.sep; ctx.lineWidth = 1; ctx.stroke();
    ctx.strokeStyle = on ? C.ground : C.accent; ctx.lineWidth = 1.5; ctx.lineCap = "round"; ctx.beginPath();
    if (b.part === "add") { ctx.moveTo(-4, 0); ctx.lineTo(4, 0); ctx.moveTo(0, -4); ctx.lineTo(0, 4); }
    else if (b.part === "move") { ctx.arc(0, 0, 3.2, 0, Math.PI * 2); } // déplace le segment
    else if (on) { ctx.rotate(b.ang); ctx.moveTo(-5, 0); ctx.lineTo(5, 0); } // segment courbe : l'icône redresse
    else { ctx.rotate(b.ang); ctx.moveTo(-5, 3); ctx.quadraticCurveTo(0, -5.5, 5, 3); }
    ctx.stroke(); ctx.restore();
  }
}
function edgeBtnAt(sp) {
  for (const b of edgeBtns(hoverEdge)) if (Math.hypot(sp[0] - b.x, sp[1] - b.y) <= EBTN.r + 5) return b.part;
  return null;
}
function edgeUnder(sp) {
  if (mode !== "edit" || !site.parcel.length) return null;
  if (hoverEdge) { const b = edgeBtns(hoverEdge)[0]; if (b && Math.hypot(sp[0] - b.mx, sp[1] - b.my) <= EBTN.gap + EBTN.r + 8) return hoverEdge; }
  const nb = nearAnyEdge(toW(sp[0], sp[1]));
  return nb && nb.d * view.s < 12 && edgeRef(nb.ref).L * view.s >= 100 ? nb.ref : null;
}
function setHoverEdge(r) { if (!sameRef(r, hoverEdge)) { hoverEdge = r; redraw(); } }

function drawHandles() {
  if (mode === "draw") return;
  const font = getComputedStyle(document.body).fontFamily;
  // Reculs
  if (VIS.recul) for (let i = 0; i < site.parcel.length; i++) {
    const e = edgeInfo(i); if (e.L * view.s < 40) continue;
    const h = toS(setbackHandle(i)), m = toS(e.apex);
    ctx.strokeStyle = C.setback; ctx.lineWidth = 1; ctx.beginPath(); ctx.moveTo(m[0], m[1]); ctx.lineTo(h[0], h[1]); ctx.stroke();
    ctx.beginPath(); ctx.arc(h[0], h[1], 5.5, 0, Math.PI * 2); ctx.fillStyle = C.setback; ctx.fill(); ctx.strokeStyle = C.ground; ctx.lineWidth = 1.5; ctx.stroke();
    ctx.font = `600 11px ${font}`; ctx.fillStyle = C.setback; ctx.textAlign = "center"; ctx.textBaseline = "middle";
    const lx = h[0] + e.u[0] * 22, ly = h[1] - e.u[1] * 22;
    ctx.fillText(fmt1(site.setbacks[i] || 0) + " m", lx, ly);
  }
  // Poignée des arcs : glisser pour régler la flèche
  eachRing((r, P) => { for (let i = 0; i < P.length; i++) {
    const ref = { ...r, i }; if (!kOf(ref) || edgeRef(ref).L * view.s < 40) continue;
    const q = toS(edgeRef(ref).apex), p = toS(arcKnob(ref)), on = drag && drag.hit && drag.hit.kind === "arc" && sameRef(drag.hit.ref, ref);
    ctx.strokeStyle = C.accent; ctx.lineWidth = 1; ctx.beginPath(); ctx.moveTo(q[0], q[1]); ctx.lineTo(p[0], p[1]); ctx.stroke();
    ctx.beginPath(); ctx.arc(p[0], p[1], 5.5, 0, Math.PI * 2); ctx.fillStyle = on ? C.accent : C.ground; ctx.fill(); ctx.strokeStyle = C.accent; ctx.lineWidth = 1.8; ctx.stroke();
  } }, true);
  // Accès
  site.entries.forEach((en, i) => {
    if (en.edge >= site.parcel.length) return;
    const p = toS(entryPoint(en)), on = sel && sel.type === "entry" && sel.i === i;
    ctx.beginPath(); ctx.arc(p[0], p[1], 9, 0, Math.PI * 2); ctx.fillStyle = on ? C.accent : C.ground; ctx.fill(); ctx.strokeStyle = on ? C.ground : C.label; ctx.lineWidth = 1.5; ctx.stroke();
    const nn = edgeInfo(en.edge).nAt(en.t);
    arrow(p[0], p[1], nn[0], -nn[1], 4, on ? C.ground : C.label, 1.6);
  });
  // Sommets de parcelle
  const P = site.parcel, n = P.length;
  P.forEach((v, i) => {
    const p = toS(v), on = sel && sel.type === "vertex" && sel.i === i;
    const near = Math.min(Math.hypot(...[0, 1].map((k) => (P[(i + 1) % n][k] - v[k]) * view.s)), Math.hypot(...[0, 1].map((k) => (P[(i + n - 1) % n][k] - v[k]) * view.s)));
    squareHandle(p, on, near < 24 && !on ? 5 : null);
  });
  // Objet sélectionné
  if (!sel) return;
  if (sel.type === "building") {
    const o = site.buildings[sel.i];
    const cs = objCorners(o).map(toS);
    ctx.strokeStyle = C.accent; ctx.lineWidth = 1.5; ctx.setLineDash([4, 3]); ctx.beginPath(); cs.forEach((c, k) => (k ? ctx.lineTo(c[0], c[1]) : ctx.moveTo(c[0], c[1]))); ctx.closePath(); ctx.stroke(); ctx.setLineDash([]);
    const r = toS(rotHandle(o)), top = toS([(objCorners(o)[2][0] + objCorners(o)[3][0]) / 2, (objCorners(o)[2][1] + objCorners(o)[3][1]) / 2]);
    ctx.beginPath(); ctx.moveTo(top[0], top[1]); ctx.lineTo(r[0], r[1]); ctx.stroke();
    ctx.beginPath(); ctx.arc(r[0], r[1], 7, 0, Math.PI * 2); ctx.fillStyle = C.ground; ctx.fill(); ctx.strokeStyle = C.accent; ctx.lineWidth = 2; ctx.stroke();
    cs.forEach((c) => squareHandle(c, true));
    // Cotes
    const corners = objCorners(o);
    dimLabel(corners[0], corners[1], fmt1(o.w) + " m", font);
    dimLabel(corners[1], corners[2], fmt1(o.h) + " m", font);
  }
  if (POLY[sel.type]) {
    const e = site[POLY[sel.type]][sel.i];
    ctx.beginPath(); pathPoly(polyOutline(e)); ctx.strokeStyle = C.accent; ctx.lineWidth = 2; ctx.stroke();
    e.forEach((v, k) => squareHandle(toS(v), sel.v === k));
  }
  if (sel.type === "gir" && girEditable()) {
    const P = site.gir.pts; ctx.strokeStyle = C.accent; ctx.lineCap = "round"; ctx.lineJoin = "round";
    ctx.lineWidth = 2; strokeLine(girDense(P, 0, P.length - 1));
    if (sel.e != null && sel.e < P.length - 1) { ctx.lineWidth = 4; strokeLine(girDense(P, sel.e, sel.e + 1)); }
    P.forEach((v, k) => squareHandle(toS(v), sel.v === k, sel.v === k ? null : 6));
  }
  if (sel.type === "slab" && slabOn()) {
    const o = slabObj(), cs = objCorners(o), S = cs.map(toS), M = slabMids(o).map(toS), r = toS(slabRotHandle()), q = toS(layout.ouvrage.dalle[2]);
    const act = drag && drag.moved && drag.hit ? drag.hit : null;
    ctx.strokeStyle = C.accent; ctx.lineWidth = 1.5; ctx.setLineDash([4, 3]); ctx.beginPath(); S.forEach((c, k) => (k ? ctx.lineTo(c[0], c[1]) : ctx.moveTo(c[0], c[1]))); ctx.closePath(); ctx.stroke(); ctx.setLineDash([]);
    // Rotation : pastille reliée à son coin.
    ctx.beginPath(); ctx.moveTo(q[0], q[1]); ctx.lineTo(r[0], r[1]); ctx.stroke();
    ctx.beginPath(); ctx.arc(r[0], r[1], 8, 0, Math.PI * 2); ctx.fillStyle = act && act.kind === "srotate" ? C.accent : C.ground; ctx.fill(); ctx.strokeStyle = C.accent; ctx.lineWidth = 2; ctx.stroke();
    // Étirement : une barrette au milieu de chaque côté, dans son alignement.
    M.forEach((m, k) => {
      const a = S[k], b = S[(k + 1) % 4];
      ctx.save(); ctx.translate(m[0], m[1]); ctx.rotate(Math.atan2(b[1] - a[1], b[0] - a[0]));
      ctx.beginPath(); ctx.roundRect(-12, -4, 24, 8, 4); ctx.fillStyle = act && act.kind === "sresize" && act.k === k ? C.accent : C.ground; ctx.fill(); ctx.strokeStyle = C.accent; ctx.lineWidth = 1.8; ctx.stroke();
      ctx.restore();
    });
    // Cotes au quart des côtés, pour laisser les barrettes libres.
    const mw = slabMids(o);
    dimLabel(mw[3], cs[0], fmt1(o.h) + " m", font);
    dimLabel(cs[0], mw[0], fmt1(o.w) + " m", font);
  }
}
function squareHandle(p, on, size) {
  const s = size || (on ? 9 : 7);
  ctx.fillStyle = on ? C.accent : C.ground; ctx.strokeStyle = C.accent; ctx.lineWidth = 1.6;
  ctx.beginPath(); ctx.rect(Math.round(p[0] - s / 2) + 0.5, Math.round(p[1] - s / 2) + 0.5, s, s); ctx.fill(); ctx.stroke();
}
function dimLabel(a, b, text, font) {
  const A = toS(a), B = toS(b), m = [(A[0] + B[0]) / 2, (A[1] + B[1]) / 2];
  let ang = Math.atan2(B[1] - A[1], B[0] - A[0]); if (ang > Math.PI / 2) ang -= Math.PI; if (ang < -Math.PI / 2) ang += Math.PI;
  ctx.save(); ctx.translate(m[0], m[1]); ctx.rotate(ang);
  ctx.font = `600 11px ${font}`; const w = ctx.measureText(text).width + 10;
  ctx.fillStyle = C.accent; ctx.beginPath(); ctx.roundRect(-w / 2, -19, w, 16, 8); ctx.fill();
  ctx.fillStyle = "#fff"; ctx.textAlign = "center"; ctx.textBaseline = "middle"; ctx.fillText(text, 0, -11);
  ctx.restore();
}

// Tracé d'une nouvelle parcelle
let mode = "edit", draft = [], hoverW = null;
function drawDraft() {
  if (mode !== "draw" || !draft.length) return;
  ctx.strokeStyle = C.accent; ctx.lineWidth = 2; ctx.beginPath();
  draft.forEach((v, i) => { const p = toS(v); i ? ctx.lineTo(p[0], p[1]) : ctx.moveTo(p[0], p[1]); });
  if (hoverW) { const p = toS(hoverW); ctx.lineTo(p[0], p[1]); }
  ctx.stroke();
  draft.forEach((v, i) => squareHandle(toS(v), i === 0));
  if (hoverW && draft.length) { const a = draft[draft.length - 1]; const p = toS(hoverW); ctx.font = `600 11px ${getComputedStyle(document.body).fontFamily}`; ctx.fillStyle = C.accent; ctx.textAlign = "left"; ctx.fillText(fmt1(Math.hypot(hoverW[0] - a[0], hoverW[1] - a[1])) + " m", p[0] + 12, p[1] - 10); }
}

