// ============================================================================
// Ajouts, suppression, menus.
// ============================================================================
function viewCenter() { const narrow = W < 760; return toW(narrow ? W / 2 : (W + 190 - 300) / 2, H / 2); }
function addObject(kind) {
  const c = viewCenter().map((v) => snap(v, 0.5));
  if (CORE_NAME[kind]) {
    // Ajouté au milieu de la vue s'il tombe sur la dalle, sinon au milieu de la dalle ; le moteur le cale sur la place la plus proche.
    if (!coresOn()) return;
    const D = layout.ouvrage.dalle, p = pointInPoly(c, D) ? c : [D.reduce((a, q) => a + q[0], 0) / D.length, D.reduce((a, q) => a + q[1], 0) / D.length];
    ensureCores(); site.noyaux.push({ nature: kind, p: p.map((v) => +v.toFixed(3)) });
    solve(false); const i = coreNear(p, kind); setSel(i >= 0 ? { type: "core", i } : null); commit(); markMine();
    toast(`${CORE_NAME[kind]} ajouté : glissez-le à sa place`);
    return;
  }
  if (kind === "building" || kind === "buildingL") { site.buildings.push({ cx: c[0], cy: c[1], w: kind === "buildingL" ? 36 : 30, h: kind === "buildingL" ? 30 : 18, rot: 0, shape: kind === "buildingL" ? "L" : "rect" }); setSel({ type: "building", i: site.buildings.length - 1 }); }
  if (kind === "landscape") {
    site.landscapes.push([[-10, -6], [2, -7], [10, -4], [9, 6], [-1, 7], [-10, 5]].map(([x, y]) => [c[0] + x, c[1] + y]));
    setSel({ type: "landscape", i: site.landscapes.length - 1 });
  }
  if (kind === "basin") {
    (site.basins = site.basins || []).push([[-13, -3], [-6, -8], [6, -8], [13, -3], [13, 3], [6, 8], [-6, 8], [-13, 3]].map(([x, y]) => [+(c[0] + x).toFixed(2), +(c[1] + y).toFixed(2), 0.12]));
    setSel({ type: "basin", i: site.basins.length - 1 });
  }
  if (kind === "exclusion") {
    const r = 24; site.exclusions.push([[c[0] - r * 1.6, c[1] - 4], [c[0] - r * 0.4, c[1] - 7], [c[0] + r * 0.6, c[1] - 3], [c[0] + r * 1.6, c[1] - 6], [c[0] + r * 1.6, c[1] + 6], [c[0] + r * 0.4, c[1] + 8], [c[0] - r * 0.7, c[1] + 4], [c[0] - r * 1.6, c[1] + 6]].map((p) => p.map((v) => +v.toFixed(2))));
    setSel({ type: "exclusion", i: site.exclusions.length - 1 });
  }
  if (kind === "traverse") {
    const a = (layout && !layout.error ? layout.angle : 0) * Math.PI / 180, ux = -Math.sin(a) * 15, uy = Math.cos(a) * 15;
    (site.traverses = site.traverses || []).push([[+(c[0] - ux).toFixed(2), +(c[1] - uy).toFixed(2)], [+(c[0] + ux).toFixed(2), +(c[1] + uy).toFixed(2)]]);
    if (!SKEL) { SKEL = true; $("#p-skel").checked = true; try { localStorage.setItem("trame.skel", "1"); } catch (e) { /* stockage indisponible */ } }
    toast("Traverse ajoutée : déplacez ses extrémités, la croix la supprime");
  }
  if (kind === "entry") {
    const best = nearEdge(c);
    // Au tiers de la limite la plus proche, aux deux tiers si un accès y est déjà, sinon au droit du
    // centre de la vue.
    const free = (t) => !site.entries.some((en) => en.edge === best.i && Math.abs(en.t - t) < 0.1);
    if (best) { site.entries.push({ edge: best.i, t: [ENTRY_T, 1 - ENTRY_T].find(free) ?? Math.max(0.1, Math.min(0.9, best.t)) }); setSel({ type: "entry", i: site.entries.length - 1 }); }
  }
  solve(false); commit(); markMine();
}
function deleteSel() {
  if (!sel) return;
  if (sel.type === "gir") { if (!girEditable()) return; if (sel.v != null) removeGirPoint(sel.v); else girClear(); return; }
  if (sel.type === "building") site.buildings.splice(sel.i, 1);
  else if (sel.type === "landscape") site.landscapes.splice(sel.i, 1);
  else if (sel.type === "basin") site.basins.splice(sel.i, 1);
  else if (sel.type === "exclusion") site.exclusions.splice(sel.i, 1);
  else if (sel.type === "entry") site.entries.splice(sel.i, 1);
  else if (sel.type === "core") { const n = layout.ouvrage.noyaux[sel.i]; ensureCores(); site.noyaux.splice(coreEntry(n), 1); toast(`${CORE_NAME[natureOf(n)]} retiré`); }
  else if (sel.type === "vertex") { if (site.parcel.length > 3) { removeParcelVertex(sel.i); return; } toast("Une parcelle garde au moins trois sommets"); return; }
  setSel(null); solve(false); commit(); markMine();
}
$("#sel-rot").addEventListener("click", () => {
  // Dalle : même emprise, allées perpendiculaires.
  if (sel.type === "slab") { const o = slabObj(); site.slab = { cx: o.cx, cy: o.cy, w: o.h, h: o.w, rot: (o.rot + 90) % 360 }; solve(false); syncSlab(); updateSelbar(); commit(); markMine(); return; }
  const o = site.buildings[sel.i]; o.rot = (o.rot + 90) % 360; updateSelbar(); solve(false); commit();
});
$("#sel-auto").addEventListener("click", () => {
  if (sel && sel.type === "core") { site.noyaux = null; setSel(null); solve(false); commit(); markMine(); toast("Escaliers et ascenseurs replacés automatiquement"); return; }
  site.slab = null; solve(false); updateSelbar(); commit(); markMine();
});
$("#sel-shape").addEventListener("click", () => { const o = site.buildings[sel.i]; o.shape = o.shape === "L" ? "rect" : "L"; updateSelbar(); solve(false); commit(); });

function closeMenus(except) {
  for (const [btn, menu] of [["#b-add", "#addmenu"], ["#b-mod", "#modmenu"], ["#b-cost", "#costmenu"], ["#b-gir", "#girmenu"], ["#b-params", "#params"], ["#b-dress", "#dress"], ["#b-export", "#export"], ["#b-new", "#newmenu"]]) if (menu !== except) { $(menu).hidden = true; $(btn).setAttribute("aria-expanded", "false"); }
}
function toggleMenu(btn, menu) {
  const open = $(menu).hidden; closeMenus(menu);
  $(menu).hidden = !open; $(btn).setAttribute("aria-expanded", String(open));
  if (open) placeMenu(btn, menu);
}
function placeMenu(btn, menu) {
  if (menu === "#params") { const p = $(menu); p.classList.remove("two"); if (p.scrollHeight > p.clientHeight + 1 && window.innerWidth >= 760) p.classList.add("two"); }
  const b = $(btn).getBoundingClientRect(), m = $(menu), mw = m.offsetWidth;
  const left = Math.max(16, Math.min(window.innerWidth - mw - 16, b.left + b.width / 2 - mw / 2));
  m.style.left = left + "px";
}
$("#b-add").addEventListener("click", () => toggleMenu("#b-add", "#addmenu"));
$("#b-mod").addEventListener("click", () => toggleMenu("#b-mod", "#modmenu"));
$("#b-cost").addEventListener("click", () => { updateCost(); toggleMenu("#b-cost", "#costmenu"); });
$("#b-gir").addEventListener("click", () => { updateGirUI(); updatePlacesUI(); toggleMenu("#b-gir", "#girmenu"); });
$("#b-params").addEventListener("click", () => toggleMenu("#b-params", "#params"));
$("#b-dress").addEventListener("click", () => { syncDressUI(); toggleMenu("#b-dress", "#dress"); });
$("#b-export").addEventListener("click", () => { updateExportUI(); toggleMenu("#b-export", "#export"); });
document.querySelectorAll("[data-add]").forEach((b) => b.addEventListener("click", () => { closeMenus(); if (mode === "draw") setMode("edit"); addObject(b.dataset.add); }));
$("#b-draw").addEventListener("click", () => { closeMenus(); setMode(mode === "draw" ? "edit" : "draw"); if (mode === "draw") toast("Touchez le plan pour placer les sommets"); });
$("#b-undo").addEventListener("click", undo);
$("#b-redo").addEventListener("click", redo);
$("#b-fit").addEventListener("click", () => { closeMenus(); fit(true); });
// Panneaux Bilan et Couches dépliables ; sur téléphone, ils se partagent le haut de l'écran : un seul ouvert.
function setPanel(name, open) {
  const head = $("#" + name + "-head"), body = $("#" + name + "-body");
  head.setAttribute("aria-expanded", String(open)); body.hidden = !open;
  if (name === "stats") $("#stats").dataset.open = String(open);
  if (open && W < 520) setPanel(name === "stats" ? "layers" : "stats", false);
}
$("#stats-head").addEventListener("click", () => setPanel("stats", $("#stats-head").getAttribute("aria-expanded") !== "true"));

window.addEventListener("keydown", (ev) => {
  if (ev.target.tagName === "INPUT" || ev.target.tagName === "SELECT" || ev.target.tagName === "TEXTAREA") return;
  if (mode === "map") { if (ev.key === "Escape") exitMap(); return; }
  const mod = ev.metaKey || ev.ctrlKey;
  if (mod && ev.key.toLowerCase() === "z") { ev.preventDefault(); ev.shiftKey ? redo() : undo(); return; }
  if (mod && ev.key.toLowerCase() === "y") { ev.preventDefault(); redo(); return; }
  if (mode === "gir") {
    if (ev.key === "Escape") setMode("edit");
    if (ev.key === "Enter") finishGir();
    if (ev.key === "Backspace" || ev.key === "Delete") { ev.preventDefault(); girPop(); }
    return;
  }
  if (ev.key === "Escape") { if (mode === "draw") setMode("edit"); else { hoverEdge = null; setSel(null); closeMenus(); } }
  if (ev.key === "Enter" && mode === "draw" && draft.length >= 3) finishDraft();
  if ((ev.key === "Delete" || ev.key === "Backspace") && sel) { ev.preventDefault(); deleteSel(); }
  if (ev.key === "f" && !mod) fit(true);
});

