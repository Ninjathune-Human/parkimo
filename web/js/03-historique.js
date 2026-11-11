// ============================================================================
// Historique et persistance (navigateur du visiteur uniquement).
// ============================================================================
const hist = { past: [], future: [] };
let meta = { created: Date.now(), modified: Date.now() };  // dates de création et de dernière modification du projet
function snapshot() { return JSON.stringify({ site, params, exampleName }); }
function commit() {
  const s = snapshot();
  if (hist.past[hist.past.length - 1] !== s) { meta.modified = Date.now(); hist.past.push(s); if (hist.past.length > 80) hist.past.shift(); hist.future = []; }
  try { localStorage.setItem("trame.v1", JSON.stringify({ site, params, exampleName, mineSite, meta })); } catch (e) { /* stockage indisponible */ }
  updateUndo();
}
function restore(s) { const o = JSON.parse(s); site = o.site; params = o.params; exampleName = o.exampleName; sel = null; updateSelbar(); syncParamsUI(); syncExample(); updateLayersUI(); solve(false); }
function undo() { if (mode === "gir") { girPop(); return; } if (hist.past.length < 2) return; hist.future.push(hist.past.pop()); restore(hist.past[hist.past.length - 1]); updateUndo(); }
function redo() { if (!hist.future.length) return; const s = hist.future.pop(); hist.past.push(s); restore(s); updateUndo(); }
// Commandes : la porte d'entrée commune à l'interface, à une IA et aux scripts. Le moteur applique
// la liste en transaction (tout passe, ou rien ne change) ; le résultat s'annule comme un geste.
function appliquer(commandes) {
  let r;
  try { r = Engine.call("commandes_json", { projet: { site, params }, commandes }); } catch (e) { r = { error: String(e) }; }
  if (!r || !r.ok) return { ok: false, index: r && r.erreur ? r.erreur.index : 0, message: (r && ((r.erreur && r.erreur.message) || r.error)) || "moteur indisponible" };
  site = normSite(r.projet.site); params = { ...DEFAULT_PARAMS, ...r.projet.params }; sel = null; lastAngle = null;
  updateSelbar(); syncParamsUI(); updateLayersUI(); solve(false); commit(); markMine();
  return { ok: true };
}
window.parkimo = {
  appliquer,
  catalogue: () => Engine.call("catalogue_json", {}),
  projet: () => JSON.parse(JSON.stringify({ site, params })),
  plan: () => layout,
};
function updateUndo() { $("#b-undo").disabled = mode === "gir" ? !gdraft.pts.length : hist.past.length < 2; $("#b-redo").disabled = !hist.future.length; }

