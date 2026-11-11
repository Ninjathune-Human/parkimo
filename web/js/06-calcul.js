// ============================================================================
// Calcul et bilan.
// ============================================================================
function solve(quick) {
  if (!Engine.ex || site.parcel.length < 3) { layout = null; gir = null; updateStats(quick); redraw(); return; }
  const t0 = performance.now();
  try { layout = Engine.solve(engineInput(quick)); } catch (e) { layout = { error: String(e) }; }
  lastMs = performance.now() - t0;
  if (!quick && layout && !layout.error) lastAngle = layout.angle;
  // Repères du squelette recalés par le moteur sur le milieu de leur voie (une voie resserrée se décale).
  const ln = !quick && layout && layout.circulation && layout.circulation.lanes;
  if (ln && site.lanes && ln.length === site.lanes.length) site.lanes = ln;
  if (!quick) { placesReset(); girRun(); refine(); }
  if (sel && sel.type === "slab") { if (!(layout && layout.ouvrage)) sel = null; updateSelbar(); }
  updateStats(quick); updateNiv(); redraw();
}
// Places au sortir des virages : le contrôle (recherche de giration, quelques secondes) tourne en
// arrière-plan, sur un Web Worker à part de ceux du test des places ; le plan affiché tout de suite
// est celui du calcul sans lui, remplacé s'il retire des places. Un seul contrôle à la fois : le plan
// modifié entre-temps est contrôlé à la fin du précédent.
let refineRev = 0, refining = false;
function refine() {
  const rev = ++refineRev;
  if (refining || !layout || layout.error || !layout.stalls.length) return;
  refining = true;
  workerCall("solve_json", engineInput(false, true), POOL).then((out) => {
    if (rev !== refineRev || !out || out.error || !layout || layout.error || out.stalls.length === layout.stalls.length) return;
    layout = out;
    placesReset(); girRun(); updateStats(false); updateNiv(); redraw();
  }).catch(() => {}).finally(() => { refining = false; if (rev !== refineRev) refine(); });
}
let solveQueued = false, solveQuick = false;
function requestSolve(quick) { solveQuick = quick; if (!solveQueued) { solveQueued = true; requestAnimationFrame(() => { solveQueued = false; solve(solveQuick); }); } }

const nf1 = new Intl.NumberFormat("fr-FR", { minimumFractionDigits: 1, maximumFractionDigits: 1 });
const nf0 = new Intl.NumberFormat("fr-FR", { maximumFractionDigits: 0 });
const fmt1 = (x) => nf1.format(x), fmtInt = (x) => nf0.format(x);
function chip(ok) { return `<span class="chip ${ok ? "ok" : "bad"}">${ok ? "conforme" : "insuffisant"}</span>`; }
// Conformité : constats du dernier calcul complet. Un calcul rapide (pendant un glisser) n'en rend
// pas : les précédents restent affichés jusqu'au relâchement.
let constats = [];
const cstNum = (x, u) => (u === "m" ? x.toFixed(2).replace(".", ",") : fmtInt(x));
function updateConf() {
  const head = $("#conf-head"), body = $("#conf-body");
  head.hidden = !constats.length; body.hidden = !constats.length || head.getAttribute("aria-expanded") !== "true";
  const n = (st) => constats.filter((c) => c.statut === st).length, e = n("ecart"), i = n("info");
  $("#s-conf").innerHTML = (e ? `<span class="chip bad">${e} écart${e > 1 ? "s" : ""}</span>` : constats.length ? `<span class="chip ok">conforme</span>` : "") + (i ? `<span class="chip grey">${i} à justifier</span>` : "");
  const rank = { ecart: 0, info: 1, conforme: 2 }, el = (tag, cls, text) => { const x = document.createElement(tag); x.className = cls; if (text) x.textContent = text; return x; };
  body.replaceChildren(...[...constats].sort((a, b) => rank[a.statut] - rank[b.statut]).map((c) => {
    const k = c.statut === "ecart" ? "bad" : c.statut === "conforme" ? "ok" : "", u = c.unite && c.unite !== "places" ? " " + c.unite : "";
    const v = c.mesure == null ? "" : c.statut === "info" || c.exige == null ? cstNum(c.mesure, c.unite) + u : `${cstNum(c.mesure, c.unite)} / ${cstNum(c.exige, c.unite)}${u}`;
    const row = el("div", "cst " + k);
    row.append(el("i", "dot " + k), el("span", "cst-l", c.libelle), el("span", "cst-v", v), el("span", "cst-s", c.source + (c.detail ? " · " + c.detail : "")));
    return row;
  }));
}
$("#conf-head").addEventListener("click", () => { const h = $("#conf-head"); h.setAttribute("aria-expanded", String(h.getAttribute("aria-expanded") !== "true")); updateConf(); });

function updateStats(quick) {
  const m = layout && layout.metrics;
  if (!quick || !m) constats = (m && layout.constats) || [];
  updateConf();
  if (!m) {
    $("#s-total").textContent = "0";
    for (const id of ["#s-pmr", "#s-irve", "#s-irve-eq", "#s-split", "#s-trees", "#s-eff", "#s-parcel", "#s-green", "#s-angle", "#o-val"]) $(id).textContent = "–";
    $("#warn").hidden = !baseShown() || site.parcel.length >= 3; $("#warn").textContent = "Touchez une parcelle cadastrale puis « Étudier », ou tracez une parcelle.";
    $("#engine").textContent = "";
    return;
  }
  $("#s-total").textContent = fmtInt(m.stalls); updateCost();
  const ov = layout.ouvrage;
  $("#r-dalle").hidden = !ov; $("#lay-structure").hidden = !ov; $("#k-eff").textContent = ov ? "Dalle par place" : "Enrobé par place";
  // Tours hors de la dalle : longueur de la dalle seule (l'emprise les comprend).
  if (ov) $("#s-dalle").textContent = `${fmt1(ov.largeur)} × ${fmt1(Math.hypot(ov.dalle[1][0] - ov.dalle[0][0], ov.dalle[1][1] - ov.dalle[0][1]))} m · ${ov.modules}${ov.demi_module ? ",5" : ""} module${ov.modules + (ov.demi_module ? 0.5 : 0) >= 2 ? "s" : ""}`;
  $("#r-trame").hidden = !ov; if (ov) $("#s-trame").textContent = `${DENSITE[params.silo_densite]} · portée ${fmt1(ov.portee)} m`;
  $("#r-niveaux").hidden = !ov || ov.niveaux < 2; if (ov && ov.niveaux > 1) $("#s-niveaux").textContent = ov.demi_niveaux ? `${ov.niveaux} en ${2 * ov.niveaux} demi-niveaux · ${fmtInt(ov.places_niveau)} places par niveau` : ov.plateaux_inclines ? `${ov.niveaux} · ${fmtInt(ov.places_niveau)} places par niveau, plateaux inclinés` : `${ov.niveaux} · ${fmtInt(ov.places_rdc)} places au rez, ${ov.niveaux > 2 ? fmtInt(ov.places_niveau) + " par niveau courant, " : ""}${fmtInt(ov.places_dernier)} au dernier`;
  $("#r-rampes").hidden = !ov || ov.niveaux < 2; $("#lay-girations").hidden = !ov || !ov.girations.length;
  if (ov && ov.niveaux > 1) { const k = ov.girations.filter((g) => g.conflits.length).length; $("#s-rampes").innerHTML = ov.rampes.length ? `${IMPLANTATION[ov.implantation]}<span class="chip ${k ? "bad" : "ok"}">${k ? (k > 1 ? k + " girations gênées" : "giration gênée") : "girations conformes"}</span>` : `aucune<span class="chip bad">impossible</span>`; }
  const ev = ov && ov.evacuation;
  $("#r-noyaux").hidden = !ev; $("#lay-noyaux").hidden = !ov || !ov.noyaux.length;
  $("#asc-field").hidden = !(ev && ov.demi_niveaux);
  for (const k of ["escalier", "ascenseur"]) $(`[data-add="${k}"]`).hidden = !ev;
  $('[data-add="liaison"]').hidden = !(ev && ov.demi_niveaux);
  if (ev) {
    $("#s-noyaux").innerHTML = `${ev.escaliers}${ev.ascenseurs ? ` · ${ev.ascenseurs} ascenseur${ev.ascenseurs > 1 ? "s" : ""}` : ""}${ev.liaisons ? " · liaison" : ""}${ev.manuel ? " · à la main" : ""}<span class="chip ${ev.hors ? "bad" : "ok"}">${ev.hors ? `${ev.hors} place${ev.hors > 1 ? "s" : ""} trop loin` : `${fmt1(ev.distance_max)} m au plus`}</span>`;
    $("#r-noyaux").title = `PS 13 : ${ev.limite_deux} m entre deux escaliers opposés, ${ev.limite_un} m sinon${ev.largement_ventile ? " (parc largement ventilé)" : ""}, dans l'axe des circulations`;
  }
  $("#r-plancher").hidden = !ov; if (ov) { $("#s-plancher").textContent = `${ov.plancher.court} · étage ≥ ${ov.plancher.hauteur_min.toFixed(2).replace(".", ",")} m`; $("#r-plancher").title = ov.plancher.libelle + " (" + ov.plancher.source + ")"; }
  $("#s-pmr").innerHTML = `${m.pmr} / ${m.pmr_required}${chip(m.pmr >= m.pmr_required)}`;
  $("#s-irve").innerHTML = `${m.irve} / ${m.irve_required}${chip(m.irve >= m.irve_required)}`;
  $("#s-irve-eq").innerHTML = `${m.irve_equipped} / ${m.irve_equipped_required}${chip(m.irve_equipped >= m.irve_equipped_required)}`;
  $("#s-split").textContent = `${m.perimeter_stalls} / ${m.interior_stalls}`;
  $("#s-trees").textContent = fmtInt(m.trees);
  $("#s-eff").textContent = m.stalls ? fmt1(m.m2_paved_per_stall) + " m²" : "–";
  $("#s-parcel").textContent = m.parcel_area >= 10000 ? fmt1(m.parcel_area / 10000).replace(/,0$/, "") + " ha" : fmtInt(m.parcel_area) + " m²";
  $("#s-green").textContent = m.parcel_area && !ov ? fmtInt(100 * m.green_area / m.parcel_area) + " %" : "–";
  $("#s-angle").textContent = fmt1(layout.angle) + "°" + (params.angle === null ? "" : " imposé");
  const w = layout.warnings || [];
  $("#warn").hidden = !w.length; $("#warn").textContent = w.join(" · ");
  $("#engine").textContent = `Moteur Rust · ${Engine.kind} · ${fmt1(lastMs)} ms · ${fmtInt(layout.evals)} variantes ${quick ? "(direct)" : "testées"}`;
  $("#o-val").textContent = fmt1(layout.angle) + "°"; updateSlantNote();
}

