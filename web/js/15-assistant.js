// ============================================================================
// Assistant : dialogue avec un modèle Claude qui agit sur le projet par les commandes du moteur.
// La clé d'API est celle de l'utilisateur ; elle reste dans ce navigateur.
// ============================================================================
const IA_URL = "https://api.anthropic.com/v1/messages", IA_VERSION = "2026-10-21", IA_MODELE = "claude-sonnet-5-5";
// Échanges d'outils au plus pour une demande : au-delà, l'assistant s'arrête et rend la main.
const IA_TOURS = 8;
const IA = { cle: "", modele: IA_MODELE, garder: false, messages: [], occupe: false };
try { const s = JSON.parse(localStorage.getItem("parkimo.ia") || "null"); if (s) { IA.cle = s.cle || ""; IA.modele = s.modele || IA_MODELE; IA.garder = true; } } catch (e) { /* stockage indisponible */ }
function iaSave() {
  try { if (IA.garder) localStorage.setItem("parkimo.ia", JSON.stringify({ cle: IA.cle, modele: IA.modele })); else localStorage.removeItem("parkimo.ia"); } catch (e) { /* stockage indisponible */ }
}

// État du projet tel que l'assistant le lit : le résumé du moteur, sur le plan affiché.
function iaResume() {
  const plan = layout && !layout.error ? { metrics: layout.metrics, constats: layout.constats || [], warnings: layout.warnings || [], angle: layout.angle, ouvrage: layout.ouvrage || null } : {};
  return JSON.stringify(Engine.call("resume_json", { projet: { site, params }, plan }));
}
function iaSysteme() {
  return [
    "Tu es l'assistant de Parkimo, un outil de conception de parcs de stationnement selon les règles françaises (NF P 91-100, accessibilité, recharge électrique).",
    "Tu agis sur le projet ouvert au moyen des outils. Unités : mètres et degrés. Repère du plan : x vers la droite, y vers le haut.",
    "Méthode : lis le projet si tu n'en connais pas l'état ; applique les modifications par appliquer_commandes, plusieurs commandes à la fois quand elles vont ensemble ; lis dans le résultat le nombre de places et les constats, puis dis en une ou deux phrases ce qui a changé (places avant et après, écarts de conformité).",
    "Les objets se désignent par leur rang, donné par lire_projet ; un rang change quand un objet de rang inférieur est retiré.",
    "Si la demande est ambiguë, ou si elle retire plusieurs objets, demande confirmation avant d'agir. Si les commandes ne permettent pas ce qui est demandé, dis-le sans détour.",
    "Réponds en français, brièvement, sans titre ni liste quand une phrase suffit. Ne décris pas les commandes : décris le résultat.",
    "Catalogue des commandes (JSON) :",
    JSON.stringify(Engine.call("catalogue_json", {})),
  ].join("\n");
}
function iaOutils() {
  const ops = Engine.call("catalogue_json", {}).map((c) => c.op);
  return [
    { name: "lire_projet", description: "Rend l'état du projet ouvert : places, constats de conformité, parcelle (sommets, côtés avec rang, longueur, recul, flèche), accès, bâtiments, zones, traverses, réglages. À appeler avant d'agir si l'état n'est pas connu.", input_schema: { type: "object", properties: {} } },
    {
      name: "appliquer_commandes",
      description: "Applique une liste de commandes au projet, en transaction : tout passe, ou rien ne change et le motif du refus est rendu. Rend le nouvel état du projet.",
      input_schema: { type: "object", properties: { commandes: { type: "array", description: "Commandes du catalogue, dans l'ordre d'application", items: { type: "object", properties: { op: { type: "string", enum: ops } }, required: ["op"] } } }, required: ["commandes"] },
    },
    { name: "annuler", description: "Annule la dernière modification du projet, comme le bouton Annuler. Rend l'état du projet.", input_schema: { type: "object", properties: {} } },
  ];
}
// Exécute un appel d'outil : résultat pour le modèle, et une ligne dans la conversation pour l'utilisateur.
function iaOutil(u) {
  try {
    if (u.name === "lire_projet") return { content: iaResume() };
    if (u.name === "appliquer_commandes") {
      const cs = u.input && u.input.commandes;
      if (!Array.isArray(cs) || !cs.length) return { content: "« commandes » doit être une liste non vide.", is_error: true };
      const places = () => (layout && layout.stalls ? layout.stalls.length : 0), avant = places(), r = appliquer(cs);
      if (!r.ok) { iaLigne("act bad", `Refusé : ${r.message}`); return { content: `Commande de rang ${r.index} refusée : ${r.message}. Rien n'a changé.`, is_error: true }; }
      iaLigne("act", `${cs.length} commande${cs.length > 1 ? "s appliquées" : " appliquée"} · ${avant} puis ${places()} places`);
      return { content: iaResume() };
    }
    if (u.name === "annuler") {
      if (hist.past.length < 2) return { content: "Rien à annuler.", is_error: true };
      undo(); iaLigne("act", "Dernière modification annulée");
      return { content: iaResume() };
    }
    return { content: `Outil inconnu : ${u.name}`, is_error: true };
  } catch (e) { return { content: String(e), is_error: true }; }
}
async function iaAppel() {
  const rep = await fetch(IA_URL, {
    method: "POST",
    headers: { "content-type": "application/json", "x-api-key": IA.cle, "anthropic-version": IA_VERSION, "anthropic-dangerous-direct-browser-access": "true" },
    body: JSON.stringify({ model: IA.modele, max_tokens: 2048, system: iaSysteme(), tools: iaOutils(), messages: IA.messages }),
  });
  const data = await rep.json().catch(() => null);
  if (!rep.ok) throw new Error(rep.status === 401 ? "Clé d'API refusée." : `L'API a répondu ${rep.status}${data && data.error && data.error.message ? " : " + data.error.message : "."}`);
  return data;
}
// Une demande suit toujours une réponse : après un échec, la nouvelle demande rejoint le dernier message de l'utilisateur.
function iaDemande(texte) {
  const dernier = IA.messages[IA.messages.length - 1], bloc = { type: "text", text: texte };
  if (dernier && dernier.role === "user") dernier.content = (Array.isArray(dernier.content) ? dernier.content : [{ type: "text", text: dernier.content }]).concat(bloc);
  else IA.messages.push({ role: "user", content: [bloc] });
}
async function iaEnvoyer(texte) {
  texte = texte.trim();
  if (!texte || IA.occupe) return;
  if (!IA.cle) { $("#ia-cfg").open = true; $("#ia-key").focus(); toast("Collez d'abord une clé d'API"); return; }
  IA.occupe = true; $("#ia-go").disabled = true; $("#ia-vide").hidden = true; $("#ia-cfg").open = false;
  iaLigne("moi", texte); iaDemande(texte);
  const attente = iaLigne("attente", "L'assistant réfléchit…");
  try {
    for (let tour = 0; ; tour++) {
      const rep = await iaAppel();
      IA.messages.push({ role: "assistant", content: rep.content });
      for (const b of rep.content) if (b.type === "text" && b.text.trim()) iaLigne("lui", b.text.trim(), attente);
      const appels = rep.content.filter((b) => b.type === "tool_use");
      if (rep.stop_reason !== "tool_use" || !appels.length) break;
      IA.messages.push({ role: "user", content: appels.map((u) => ({ type: "tool_result", tool_use_id: u.id, ...iaOutil(u) })) });
      $("#ia-log").append(attente);
      if (tour + 1 >= IA_TOURS) { iaLigne("err", `Arrêt après ${IA_TOURS} échanges d'outils : reformulez ou poursuivez.`, attente); break; }
    }
  } catch (e) {
    iaLigne("err", e instanceof TypeError ? "L'API est injoignable depuis cette page." : e.message, attente);
  }
  attente.remove(); IA.occupe = false; $("#ia-go").disabled = false;
}
// Ligne de la conversation : « moi », « lui », « act » (action sur le plan), « err », « attente ».
function iaLigne(cls, texte, avant) {
  const el = document.createElement("div"), log = $("#ia-log");
  el.className = "ia-m " + cls; el.textContent = texte;
  if (avant && avant.parentNode === log) log.insertBefore(el, avant); else log.append(el);
  el.scrollIntoView({ block: "nearest" });
  return el;
}
function iaOuvrir(ouvert) {
  $("#ia").hidden = !ouvert; $("#b-ia").setAttribute("aria-expanded", String(ouvert));
  if (!ouvert) return;
  closeMenus(); $("#ia-key").value = IA.cle; $("#ia-model").value = IA.modele; $("#ia-keep").checked = IA.garder;
  if (!IA.cle) $("#ia-cfg").open = true;
  else if (matchMedia("(pointer: fine)").matches) $("#ia-q").focus();
}
$("#b-ia").addEventListener("click", () => iaOuvrir($("#ia").hidden));
$("#ia-close").addEventListener("click", () => iaOuvrir(false));
$("#ia-new").addEventListener("click", () => { if (IA.occupe) return; IA.messages = []; $("#ia-log").replaceChildren(); $("#ia-vide").hidden = false; });
$("#ia-key").addEventListener("input", (ev) => { IA.cle = ev.target.value.trim(); iaSave(); });
$("#ia-model").addEventListener("input", (ev) => { IA.modele = ev.target.value.trim() || IA_MODELE; iaSave(); });
$("#ia-keep").addEventListener("change", (ev) => { IA.garder = ev.target.checked; iaSave(); });
$("#ia-vide").addEventListener("click", (ev) => { const b = ev.target.closest("[data-q]"); if (b) iaEnvoyer(b.dataset.q); });
$("#ia-form").addEventListener("submit", (ev) => { ev.preventDefault(); if (IA.occupe) return; const q = $("#ia-q"), t = q.value; q.value = ""; q.style.height = ""; iaEnvoyer(t); });
$("#ia-q").addEventListener("keydown", (ev) => { if (ev.key === "Enter" && !ev.shiftKey && !ev.isComposing) { ev.preventDefault(); $("#ia-form").requestSubmit(); } });
$("#ia-q").addEventListener("input", (ev) => { const q = ev.target; q.style.height = "auto"; q.style.height = Math.min(120, q.scrollHeight) + "px"; });
