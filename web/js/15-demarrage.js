// ============================================================================
// Démarrage.
// ============================================================================
const mq = window.matchMedia("(prefers-color-scheme: dark)");
mq.addEventListener("change", () => { readColors(); redraw(); });
new MutationObserver(() => { readColors(); redraw(); }).observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
function relayout(cw, ch) { resize(cw, ch); if (mode === "map") updateMapUI(); if (!$("#addmenu").hidden) placeMenu("#b-add", "#addmenu"); if (!$("#modmenu").hidden) placeMenu("#b-mod", "#modmenu"); if (!$("#costmenu").hidden) placeMenu("#b-cost", "#costmenu"); if (!$("#girmenu").hidden) placeMenu("#b-gir", "#girmenu"); if (!$("#params").hidden) placeMenu("#b-params", "#params"); if (!$("#dress").hidden) placeMenu("#b-dress", "#dress"); if (!$("#export").hidden) placeMenu("#b-export", "#export"); if (!$("#newmenu").hidden) placeMenu("#b-new", "#newmenu"); }
if (window.ResizeObserver) new ResizeObserver((es) => { const r = es[es.length - 1].contentRect; relayout(r.width, r.height); }).observe(canvas);
else window.addEventListener("resize", () => relayout());

(async function boot() {
  let restored = false;
  readColors(); resize();
  try {
    const saved = localStorage.getItem("trame.v1");
    if (saved) { const o = JSON.parse(saved); if (o.site && o.site.parcel) { site = normSite(o.site); params = { ...DEFAULT_PARAMS, ...o.params }; exampleName = o.exampleName || "mine"; mineSite = normSite(o.mineSite || null); if (o.meta && isFinite(o.meta.created)) meta = { created: o.meta.created, modified: isFinite(o.meta.modified) ? o.meta.modified : o.meta.created }; restored = true; } }
  } catch (e) { /* stockage indisponible : on démarre sur l'exemple */ }
  await withTimeout(loadBasemap(), 2500);
  // Première visite : un exemple ; un projet vidé volontairement reste vide.
  if (!restored && !site.parcel.length && !baseShown()) { site = EXAMPLES.ruisseau(); exampleName = "ruisseau"; }
  updateLayersUI();
  if (W < 520) { $("#stats-head").setAttribute("aria-expanded", "false"); $("#stats-body").hidden = true; $("#stats").dataset.open = "false"; }
  if (W < 760) { $("#layers-head").setAttribute("aria-expanded", "false"); $("#layers-body").hidden = true; }
  syncParamsUI(); syncExample();
  if (workBounds()) fit(); else { view.ox = W / 2; view.oy = H / 2; }
  try { await Engine.load(); $("#boot").remove(); }
  catch (e) { $("#boot").textContent = "Le moteur n'a pas pu démarrer dans ce navigateur : " + e.message; return; }
  if (basemap && basemap.occ === undefined) { computeOccupation(); saveBasemap(); }
  solve(false); commit();
})();
