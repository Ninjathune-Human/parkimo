// ============================================================================
// Nouveau projet : efface la parcelle, les objets et le fond ; garde hypothèses et calques.
// ============================================================================
function newProject() {
  closeMenus(); setMode("edit");
  site = { parcel: [], setbacks: [], exclusions: [], buildings: [], landscapes: [], basins: [], entries: [] };
  exampleName = "mine"; mineSite = null; basemap = null; layout = null; lastAngle = null;
  meta = { created: Date.now(), modified: Date.now() };
  saveBasemap();
  hist.past = []; hist.future = []; commit();
  syncExample(); updateLayersUI(); setSel(null); solve(false);
  view.s = 4; view.ox = W / 2; view.oy = H / 2; redraw();
  $("#q").value = ""; $("#q-clear").hidden = true;
}
$("#b-new").addEventListener("click", () => { $("#new-list").hidden = false; $("#new-confirm").hidden = true; toggleMenu("#b-new", "#newmenu"); });
$("#n-blank").addEventListener("click", () => {
  // Rien à perdre (exemple non modifié, sans fond) : effacement direct ; sinon confirmation.
  if (exampleName !== "mine" && !mineSite && !basemap) { newProject(); return; }
  $("#new-list").hidden = true; $("#new-confirm").hidden = false; placeMenu("#b-new", "#newmenu");
});
$("#n-cancel").addEventListener("click", () => closeMenus());
$("#n-ok").addEventListener("click", newProject);
$("#e-map").addEventListener("click", enterMap);
$("#e-draw").addEventListener("click", () => { setMode("draw"); toast("Touchez le plan pour placer les sommets"); });

