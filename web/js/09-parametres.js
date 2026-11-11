// ============================================================================
// Paramètres.
// ============================================================================
const PKEYS = ["stall_w", "stall_d", "noue_w", "basin_h", "basin_margin", "aisle", "aisle_one", "entry_w", "building_margin", "door_w", "planter_every", "cap", "canopy", "pmr_ratio", "irve_ratio", "irve_equip_ratio", "silo_niveaux", "silo_etage", "silo_pente", "silo_pu", "silo_pv", "silo_retrait", "silo_travee", "silo_facade", "silo_voies"];
// Densité de la trame du silo : libellés du bilan et du tableur.
const DENSITE = { longue: "Longue", moyenne: "Moyenne", courte: "Courte" };
const IMPLANTATION = { rangee: "En rangée", pignon: "En pignon", demi: "Demi-niveaux", helice: "Hélice", helice_ext: "Hélices hors dalle", helice_double: "Hélice à double sens", helice_centre: "Double sens au centre", helice_anneau: "Double sens en anneau", helice_facade: "Double sens en façade", inclines: "Plateaux inclinés" };
// Systèmes constructifs du silo : libellé, poteau par défaut [le long des allées, en travers], largeur de place,
// places au plus entre poteaux (acier-béton : solives sur les poteaux, 5,00 m au plus).
const SYSTEMES = {
  coule: { nom: "Béton coulé en place", pu: 0.3, pv: 0.6, w: 2.5, travee: 3 },
  prefa: { nom: "Préfabriqué béton", pu: 0.3, pv: 0.6, w: 2.4, travee: 3 },
  mixte: { nom: "Mixte acier-béton", pu: 0.18, pv: 0.171, w: 2.5, travee: 2, densite: "longue" },
};
// Estimation au ratio (parking de surface, métropole niçoise) : barème par tranche de places, en € HT/place.
const BAREME_0 = [
  { a: 20, b: 50, t: [2300, 3800], e: [2700, 4400] },
  { a: 50, b: 150, t: [2000, 3200], e: [2350, 3700] },
  { a: 150, b: 300, t: [1800, 2800], e: [2100, 3250] },
  { a: 300, b: 600, t: [1600, 2500], e: [1850, 2900] },
  { a: 600, b: 1000, t: [1500, 2300], e: [1750, 2650] },
];
// Barème modifiable par le concepteur : réglage de présentation gardé dans le navigateur, hors projet.
let BAREME = JSON.parse(JSON.stringify(BAREME_0));
try { const sv = JSON.parse(localStorage.getItem("trame.bareme") || "null"); if (Array.isArray(sv) && sv.length === BAREME.length) sv.forEach((r, i) => { for (const k of ["t", "e"]) if (Array.isArray(r[k]) && r[k].length === 2 && r[k].every((v) => isFinite(v) && v >= 0)) BAREME[i][k] = r[k]; }); } catch (e) { /* stockage indisponible */ }
function saveBareme() { try { localStorage.setItem("trame.bareme", JSON.stringify(BAREME)); } catch (e) { /* stockage indisponible */ } }
function fmtEuro(v) { return v >= 1e6 ? (v / 1e6).toLocaleString("fr-FR", { maximumFractionDigits: 2 }) + " M€" : Math.round(v / 1e3).toLocaleString("fr-FR") + " k€"; }
function buildBareme() {
  const inp = (i, k, j) => `<input type="text" inputmode="numeric" autocomplete="off" data-i="${i}" data-k="${k}" data-j="${j}" value="${BAREME[i][k][j]}" aria-label="${k === "t" ? "Travaux seuls" : "Études et MOE"}, ${j ? "maximum" : "minimum"}, ${BAREME[i].a} à ${BAREME[i].b} places">`;
  $("#c-bar").innerHTML = '<tr><th>Places</th><th>Travaux seuls (€/place)</th><th>Avec études + MOE</th></tr>' + BAREME.map((r, i) =>
    `<tr data-i="${i}"><td>${r.a} à ${r.b}</td><td>${inp(i, "t", 0)} – ${inp(i, "t", 1)}</td><td>${inp(i, "e", 0)} – ${inp(i, "e", 1)}</td></tr>`).join("");
}
$("#c-bar").addEventListener("input", (ev) => {
  const t = ev.target, v = parseFloat(String(t.value).replace(/\s/g, "").replace(",", ".")); if (!isFinite(v) || v < 0) return;
  BAREME[+t.dataset.i][t.dataset.k][+t.dataset.j] = v; saveBareme(); updateCost();
});
$("#c-reset").addEventListener("click", () => { BAREME = JSON.parse(JSON.stringify(BAREME_0)); saveBareme(); buildBareme(); updateCost(); toast("Barème d'origine rétabli"); });
buildBareme();
function updateCost() {
  const n = layout && layout.metrics ? layout.metrics.stalls : 0, ord = (x) => [Math.min(x[0], x[1]), Math.max(x[0], x[1])], rng = (r) => `${fmtInt(r[0])} – ${fmtInt(r[1])}`;
  if (!n) { for (const id of ["#c-mid", "#c-tr", "#c-tt", "#c-er", "#c-et"]) $(id).textContent = "–"; $("#c-midlabel").textContent = ""; $("#c-note").textContent = "Aucun plan calculé."; return; }
  // Tranche : la plus basse qui contient n (à 50 places exactement, la tranche 20-50) ; hors barème, tranche extrême.
  let i = BAREME.findIndex((r) => n <= r.b); if (i < 0) i = BAREME.length - 1;
  const r = BAREME[i], T = ord(r.t), E = ord(r.e), tot = (x) => [n * x[0], n * x[1]];
  $("#c-bar").querySelectorAll("tr[data-i]").forEach((tr) => tr.classList.toggle("cur", +tr.dataset.i === i));
  const tt = tot(T), et = tot(E);
  $("#c-mid").textContent = fmtEuro((tt[0] + tt[1]) / 2);
  $("#c-midlabel").textContent = `€ HT, travaux seuls, valeur centrale pour ${fmtInt(n)} places`;
  $("#c-tr").textContent = rng(T); $("#c-tt").textContent = `${fmtEuro(tt[0])} – ${fmtEuro(tt[1])}`;
  $("#c-er").textContent = rng(E); $("#c-et").textContent = `${fmtEuro(et[0])} – ${fmtEuro(et[1])}`;
  $("#c-note").textContent = n < BAREME[0].a ? "Moins de 20 places : hors barème, la première tranche est appliquée par défaut." : n > BAREME[4].b ? "Plus de 1 000 places : hors barème, la dernière tranche est appliquée par défaut." : `Tranche ${r.a} à ${r.b} places.`;
}
function updateSlantNote() {
  const a = layout && layout.row_aisle ? fmt1(layout.row_aisle).replace(/,0$/, "") : "";
  $("#sl-note").textContent = params.slant < 90
    ? `Épis à ${params.slant}° : allée de ${a || "–"} m entre rangées (Neufert p. 272), à sens unique en alternance. `
    : `Places perpendiculaires : allée réglée dans Paramètres (${fmt1(params.aisle).replace(/,0$/, "")} m ; Neufert p. 272 : ${layout && layout.neufert_aisle ? fmt1(layout.neufert_aisle).replace(/,0$/, "") : "–"} m pour des places de ${fmt1(params.stall_w).replace(/,0$/, "")} m).`;
}
function syncParamsUI() {
  for (const k of PKEYS) $("#p-" + k).value = String(params[k]).replace(".", ",");
  for (const b of $("#t-seg").children) b.setAttribute("aria-pressed", String((b.dataset.t === "silo") === !!params.silo));
  for (const b of $("#prog-seg").children) b.setAttribute("aria-pressed", String(b.dataset.g === params.programme));
  $("#silo-fields").hidden = !params.silo; $("#p-silo_rives").checked = !!params.silo_rives; $("#p-ombrieres").checked = !!params.ombrieres;
  $("#p-silo_controle").checked = !!params.silo_controle; $("#voies-field").hidden = !params.silo_controle;
  for (const b of $("#sy-seg").children) b.setAttribute("aria-pressed", String(b.dataset.s === params.silo_systeme));
  for (const b of $("#d-seg").children) { b.setAttribute("aria-pressed", String(b.dataset.d === params.silo_densite)); b.disabled = !!SYSTEMES[params.silo_systeme].densite && b.dataset.d !== SYSTEMES[params.silo_systeme].densite; }
  $("#ra-sel").value = params.silo_rampes; $("#asc-sel").value = params.silo_asc || "deux"; $("#p-silo_ventile").checked = params.silo_ventile !== false;
  $("#ra-field").hidden = $("#pslv-field").hidden = Math.round(params.silo_niveaux) < 2;
  for (const b of $("#sl-seg").children) b.setAttribute("aria-pressed", String(+b.dataset.sl === params.slant));
  updateSlantNote();
  const auto = params.angle === null;
  $("#o-auto").setAttribute("aria-pressed", String(auto)); $("#o-fixed").setAttribute("aria-pressed", String(!auto));
  $("#o-range").hidden = auto; if (!auto) $("#o-range").value = Math.round(params.angle);
  if (!$("#modmenu").hidden) placeMenu("#b-mod", "#modmenu");
}
let paramTimer = null;
for (const k of PKEYS) $("#p-" + k).addEventListener("input", (ev) => {
  const v = parseFloat(ev.target.value.replace(",", ".")); if (!isFinite(v)) return;
  params[k] = v; if (k === "silo_niveaux") $("#ra-field").hidden = $("#pslv-field").hidden = Math.round(v) < 2; clearTimeout(paramTimer); paramTimer = setTimeout(() => { lastAngle = null; solve(false); commit(); }, 120);
});
$("#prog-seg").addEventListener("click", (ev) => {
  const b = ev.target.closest("button[data-g]"); if (!b || b.dataset.g === params.programme) return;
  params.programme = b.dataset.g; syncParamsUI(); solve(false); commit();
});
$("#t-seg").addEventListener("click", (ev) => {
  const b = ev.target.closest("button[data-t]"); if (!b || (b.dataset.t === "silo") === !!params.silo) return;
  params.silo = b.dataset.t === "silo"; syncParamsUI(); lastAngle = null; solve(false); commit();
  if (!$("#params").hidden) placeMenu("#b-params", "#params");
});
$("#p-ombrieres").addEventListener("change", (ev) => { params.ombrieres = ev.target.checked; solve(false); commit(); });
$("#p-silo_rives").addEventListener("change", (ev) => { params.silo_rives = ev.target.checked; lastAngle = null; solve(false); commit(); });
$("#p-silo_controle").addEventListener("change", (ev) => { params.silo_controle = ev.target.checked; syncParamsUI(); solve(false); commit(); });
$("#p-silo_ventile").addEventListener("change", (ev) => { params.silo_ventile = ev.target.checked; lastAngle = null; solve(false); commit(); });
// Choisir un système applique ses valeurs par défaut (poteau, largeur de place, portée imposée).
$("#sy-seg").addEventListener("click", (ev) => {
  const b = ev.target.closest("button[data-s]"); if (!b || b.dataset.s === params.silo_systeme) return;
  const S = SYSTEMES[b.dataset.s];
  params.silo_systeme = b.dataset.s; params.silo_pu = S.pu; params.silo_pv = S.pv; params.stall_w = S.w; params.silo_travee = S.travee;
  if (S.densite) params.silo_densite = S.densite;
  syncParamsUI(); lastAngle = null; solve(false); commit();
});
$("#ra-sel").addEventListener("change", (ev) => {
  params.silo_rampes = ev.target.value; lastAngle = null; solve(false); commit();
});
$("#asc-sel").addEventListener("change", (ev) => {
  params.silo_asc = ev.target.value;
  if (site.noyaux) { site.noyaux = null; toast("Escaliers et ascenseurs replacés automatiquement"); }
  solve(false); commit();
});
$("#d-seg").addEventListener("click", (ev) => {
  const b = ev.target.closest("button[data-d]"); if (!b || b.dataset.d === params.silo_densite) return;
  params.silo_densite = b.dataset.d; syncParamsUI(); lastAngle = null; solve(false); commit();
});
$("#sl-seg").addEventListener("click", (ev) => {
  const b = ev.target.closest("button[data-sl]"); if (!b || +b.dataset.sl === params.slant) return;
  params.slant = +b.dataset.sl; syncParamsUI(); lastAngle = null; solve(false); commit();
});
$("#o-auto").addEventListener("click", () => { params.angle = null; syncParamsUI(); solve(false); commit(); });
$("#o-fixed").addEventListener("click", () => { params.angle = layout ? Math.round(layout.angle) : 0; syncParamsUI(); solve(false); commit(); });
$("#o-range").addEventListener("input", (ev) => { params.angle = +ev.target.value; requestSolve(false); });
$("#o-range").addEventListener("change", () => commit());

// Squelette des voies : axes de la chaussée. Au milieu de chaque voie, une pastille à deux boutons :
// sens de parcours (à inverser sur une voie à sens unique) et nombre de voies (2 ⇄ 1). Le choix est
// mémorisé dans le site par un repère (milieu de la voie, direction), recalé à chaque calcul.
$("#p-nskel").checked = NSKEL;
$("#p-nskel").addEventListener("change", (ev) => {
  NSKEL = ev.target.checked; try { localStorage.setItem("trame.nskel", NSKEL ? "1" : "0"); } catch (e) { /* stockage indisponible */ }
  if (NSKEL) toast("Au milieu de chaque noue : une pastille pour la supprimer ou la rétablir"); redraw();
});
$("#p-skel").checked = SKEL;
$("#p-skel").addEventListener("change", (ev) => {
  SKEL = ev.target.checked; try { localStorage.setItem("trame.skel", SKEL ? "1" : "0"); } catch (e) { /* stockage indisponible */ }
  if (SKEL) toast("Au milieu de chaque voie : son sens et son nombre de voies"); redraw();
});
function distPoly(p, pts) { let d = Infinity; for (let k = 1; k < pts.length; k++) d = Math.min(d, distSeg(p, pts[k - 1], pts[k])[0]); return d; }
// Point d'un axe à la fraction f de sa longueur (le milieu par défaut) et sa direction, dans l'ordre des
// points (sens de parcours d'une voie à sens unique).
function axisMid(pts, f = 0.5) {
  let L = 0; for (let k = 1; k < pts.length; k++) L += Math.hypot(pts[k][0] - pts[k - 1][0], pts[k][1] - pts[k - 1][1]);
  let acc = 0; for (let k = 1; k < pts.length; k++) { const a = pts[k - 1], b = pts[k], l = Math.hypot(b[0] - a[0], b[1] - a[1]); if (acc + l >= L * f && l > 0) { const t = (L * f - acc) / l; return { p: [+(a[0] + (b[0] - a[0]) * t).toFixed(2), +(a[1] + (b[1] - a[1]) * t).toFixed(2)], d: [+((b[0] - a[0]) / l).toFixed(3), +((b[1] - a[1]) / l).toFixed(3)], L }; } acc += l; }
  return { p: pts[0], d: [1, 0], L };
}
// Pose un repère sur la voie i ; sans effet attendu (circulation coupée, voie non desservie), le site reste tel quel.
function setLane(i, rule, done) {
  const a = layout.circulation.axes[i], prev = site.lanes || [];
  site.lanes = prev.filter((r, k) => k !== a.rule && distPoly(r.p, a.pts) > Math.max(1.5, a.w / 2) + 1).concat([rule]);
  const k = site.lanes.length - 1;
  solve(false);
  const c = layout && layout.circulation, x = c && c.axes.find((ax) => ax.rule === k);
  const ok = x && x.sens === rule.sens && (!rule.fix || (() => { const m = axisMid(x.pts); return m.d[0] * rule.d[0] + m.d[1] * rule.d[1] > 0; })());
  if (!ok) { const cut = c && c.refused > 0; site.lanes = prev; solve(false); toast(cut ? (rule.fix ? "Sens impossible : la circulation serait coupée" : "Sens unique impossible : la circulation serait coupée") : "Voie non reliée à un accès"); return; }
  commit(); markMine(); toast(done);
}
function toggleLanes(i) {
  const a = layout.circulation.axes[i], m = axisMid(a.pts), sens = a.sens === 2 ? 1 : 2;
  setLane(i, { p: m.p, sens, d: m.d, fix: false }, sens === 1 ? "Voie à sens unique" : "Voie à double sens");
}
function reverseLane(i) {
  const a = layout.circulation.axes[i];
  if (a.sens === 2) { toast("Voie à double sens : passez-la à une voie pour choisir son sens"); return; }
  const m = axisMid(a.pts);
  setLane(i, { p: m.p, sens: 1, d: [-m.d[0], -m.d[1]], fix: true }, "Sens de parcours inversé");
}
function drawPills(axes) {
  pills = [];
  const font = getComputedStyle(document.body).fontFamily, half = PILL.w / 2, r = PILL.h / 2;
  // Les plus longues d'abord, au milieu de la voie ; si la place est prise (voies parallèles proches),
  // un peu avant ou après. Une pastille n'en recouvre jamais une autre ; voie trop courte à l'écran : rien.
  const order = axes.map((a, i) => ({ a, i, m: axisMid(a.pts) })).filter((o) => o.m.L * view.s >= PILL.w + 12).sort((x, y) => y.m.L - x.m.L);
  for (const o of order) {
    let c = null;
    for (const f of [0.5, 0.32, 0.68]) {
      const q = toS(f === 0.5 ? o.m.p : axisMid(o.a.pts, f).p);
      if (q[0] < -half || q[1] < -r || q[0] > W + half || q[1] > H + r) continue;
      if (!pills.some((b) => Math.abs(b.x - q[0]) < PILL.w + 6 && Math.abs(b.y - q[1]) < PILL.h + 6)) { c = q; break; }
    }
    if (!c) continue;
    pills.push({ i: o.i, x: c[0], y: c[1] });
    const one = o.a.sens === 1, ang = Math.atan2(-o.m.d[1], o.m.d[0]);
    ctx.save(); ctx.translate(c[0], c[1]);
    ctx.shadowColor = "rgba(0, 0, 0, .22)"; ctx.shadowBlur = 8; ctx.shadowOffsetY = 2;
    ctx.beginPath(); ctx.arc(-half + r, 0, r, Math.PI / 2, Math.PI * 1.5); ctx.arc(half - r, 0, r, -Math.PI / 2, Math.PI / 2); ctx.closePath(); ctx.fillStyle = C["glass-solid"]; ctx.fill();
    ctx.shadowColor = "transparent"; ctx.strokeStyle = C.sep; ctx.lineWidth = 1; ctx.stroke();
    ctx.beginPath(); ctx.moveTo(0, -r + 7); ctx.lineTo(0, r - 7); ctx.stroke();
    // Sens de parcours : flèche dans le sens de la voie, ou flèches opposées (circulation à droite).
    ctx.save(); ctx.translate(-half / 2, 0); ctx.rotate(ang);
    ctx.strokeStyle = one ? C.accent : C["label-2"]; ctx.lineWidth = 2; ctx.lineCap = "round"; ctx.lineJoin = "round";
    ctx.beginPath();
    if (one) { ctx.moveTo(-7, 0); ctx.lineTo(7, 0); ctx.moveTo(3, -4); ctx.lineTo(7, 0); ctx.lineTo(3, 4); }
    else { ctx.moveTo(-7, 3.5); ctx.lineTo(7, 3.5); ctx.moveTo(4, 1); ctx.lineTo(7, 3.5); ctx.lineTo(4, 6); ctx.moveTo(7, -3.5); ctx.lineTo(-7, -3.5); ctx.moveTo(-4, -6); ctx.lineTo(-7, -3.5); ctx.lineTo(-4, -1); }
    ctx.stroke(); ctx.restore();
    // Nombre de voies
    ctx.fillStyle = C.label; ctx.font = `600 15px ${font}`; ctx.textAlign = "center"; ctx.textBaseline = "middle";
    ctx.fillText(one ? "1" : "2", half / 2, 1);
    ctx.restore();
  }
}
function drawSkeleton() {
  const axes = layout.circulation ? layout.circulation.axes : [];
  if (!axes.length) return;
  const line = (pts) => { const a = toS(pts[0]); ctx.moveTo(a[0], a[1]); for (let k = 1; k < pts.length; k++) { const b = toS(pts[k]); ctx.lineTo(b[0], b[1]); } };
  ctx.save(); ctx.lineCap = "round"; ctx.lineJoin = "round";
  // Double sens : deux traits (un trait large évidé) ; sens unique : un trait et des chevrons.
  ctx.beginPath(); for (const a of axes) if (a.sens === 2) line(a.pts);
  ctx.strokeStyle = C.accent; ctx.lineWidth = 6; ctx.stroke(); ctx.strokeStyle = C.asphalt; ctx.lineWidth = 2.4; ctx.stroke();
  ctx.beginPath(); for (const a of axes) if (a.sens === 1) line(a.pts);
  ctx.strokeStyle = C.accent; ctx.lineWidth = 2; ctx.stroke();
  ctx.beginPath();
  for (const a of axes) if (a.sens === 1) {
    const P = a.pts.map(toS); let next = 24, acc = 0;
    for (let k = 1; k < P.length; k++) {
      const dx = P[k][0] - P[k - 1][0], dy = P[k][1] - P[k - 1][1], l = Math.hypot(dx, dy); if (!l) continue;
      const ux = dx / l, uy = dy / l;
      while (next <= acc + l) { const t = next - acc, x = P[k - 1][0] + ux * t, y = P[k - 1][1] + uy * t; ctx.moveTo(x - ux * 4 - uy * 4, y - uy * 4 + ux * 4); ctx.lineTo(x, y); ctx.lineTo(x - ux * 4 + uy * 4, y - uy * 4 - ux * 4); next += 48; }
      acc += l;
    }
  }
  ctx.stroke();
  // Nœuds : carrefours, accès, fonds d'impasse
  ctx.fillStyle = C.accent; ctx.strokeStyle = C.stripe; ctx.lineWidth = 1.5;
  ctx.beginPath(); for (const a of axes) for (const q of [a.pts[0], a.pts[a.pts.length - 1]]) { const p = toS(q); ctx.moveTo(p[0] + 3.5, p[1]); ctx.arc(p[0], p[1], 3.5, 0, Math.PI * 2); }
  ctx.fill(); ctx.stroke();
  ctx.restore();
  drawPills(axes);
}

