// ============================================================================
// Niveaux du parc en ouvrage : le plan montre le rez, l'étage courant ou le dernier niveau, les
// trois dispositions distinctes d'un silo (les étages intermédiaires sont identiques).
// Réglage d'affichage, hors projet.
// ============================================================================
let NIV = "rdc";
// Niveaux distincts, du plus haut au plus bas ; vide pour un parc de surface ou d'un seul niveau.
function nivListe() {
  const o = layout && !layout.error && layout.ouvrage, n = o ? o.niveaux : 0;
  if (n < 2) return [];
  const L = [{ id: "dernier", nom: "R+" + (n - 1), places: o.places_dernier }];
  if (n > 2) L.push({ id: "courant", nom: n === 3 ? "R+1" : `R+1 à R+${n - 2}`, places: o.places_niveau });
  L.push({ id: "rdc", nom: "RDC", places: o.places_rdc });
  return L;
}
// Un parc d'un seul niveau montre son étage courant, ou son rez s'il est payant.
function nivActif() {
  const L = nivListe();
  if (L.length) return (L.find((x) => x.id === NIV) || L[L.length - 1]).id;
  return layout && !layout.error && layout.ouvrage && layout.ouvrage.controle ? "rdc" : "courant";
}
// Places propres au niveau affiché, en plus de celles de l'étage courant.
function nivPlaces() {
  const o = layout.ouvrage, a = o ? nivActif() : "courant";
  return a === "rdc" ? o.places_rdc_seul || [] : a === "dernier" ? o.places_dernier_seul || [] : [];
}
// Places du niveau affiché : celles de l'étage courant, sans celles que le rez payant lui prend,
// puis les places propres au niveau.
let nivCache = { layout: null, a: "", v: null };
function nivStalls() {
  const o = layout.ouvrage, a = nivActif();
  if (nivCache.layout === layout && nivCache.a === a) return nivCache.v;
  const hors = a === "rdc" && o.places_rdc_retirees && o.places_rdc_retirees.length ? new Set(o.places_rdc_retirees) : null;
  const v = a === "courant" ? layout.stalls : (hors ? layout.stalls.filter((_, i) => !hors.has(i)) : layout.stalls).concat(nivPlaces().map((q) => ({ q, k: 0, e: false })));
  nivCache = { layout, a, v };
  return v;
}
// Épures de giration du niveau affiché : pas de bas de rampe au dernier niveau, pas de haut au rez.
function nivGirations() { const a = nivActif(); return layout.ouvrage.girations.filter((g) => a === "courant" || g.bas === (a === "rdc")); }
// Rez payant : voies fléchées, îlots, barrières, bornes et local d'accueil.
function drawControle(c) {
  const mid = (a, b) => toS([(a[0] + b[0]) / 2, (a[1] + b[1]) / 2]);
  ctx.fillStyle = C.asphalt; ctx.beginPath(); for (const v of c.voies) pathPoly(v.q); ctx.fill();
  // Sens de chaque voie : du dehors vers le dedans pour une entrée, deux pointes pour une voie réversible.
  ctx.strokeStyle = ctx.fillStyle = C.stripe; ctx.lineWidth = Math.max(1, 0.14 * view.s); ctx.lineCap = "round";
  const pointe = (p, d, h) => { const n = [-d[1], d[0]]; ctx.beginPath(); ctx.moveTo(p[0] + d[0] * h, p[1] + d[1] * h); ctx.lineTo(p[0] + n[0] * h * 0.55, p[1] + n[1] * h * 0.55); ctx.lineTo(p[0] - n[0] * h * 0.55, p[1] - n[1] * h * 0.55); ctx.closePath(); ctx.fill(); };
  for (const v of c.voies) {
    const f = mid(v.q[0], v.q[1]), g = mid(v.q[2], v.q[3]), L = Math.hypot(g[0] - f[0], g[1] - f[1]), d = [(g[0] - f[0]) / L, (g[1] - f[1]) / L], h = 0.55 * view.s;
    const a = [f[0] + d[0] * L * 0.3, f[1] + d[1] * L * 0.3], b = [f[0] + d[0] * L * 0.7, f[1] + d[1] * L * 0.7];
    ctx.beginPath(); ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); ctx.stroke();
    if (v.sens !== "sortie") pointe(b, d, h);
    if (v.sens !== "entree") pointe(a, [-d[0], -d[1]], h);
  }
  ctx.fillStyle = C.stripe; ctx.globalAlpha = 0.85; ctx.beginPath(); for (const q of c.ilots) pathPoly(q); ctx.fill(); ctx.globalAlpha = 1;
  ctx.strokeStyle = C.bad; ctx.lineWidth = Math.max(1.5, 0.16 * view.s); ctx.lineCap = "butt"; ctx.beginPath();
  for (const s of c.barrieres) { const a = toS(s[0]), b = toS(s[1]); ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); }
  ctx.stroke();
  ctx.fillStyle = C.accent; const r = Math.max(1.5, 0.2 * view.s);
  for (const p of c.bornes) { const s = toS(p); ctx.fillRect(s[0] - r, s[1] - r, 2 * r, 2 * r); }
  if (c.local) {
    ctx.fillStyle = C["building-edge"]; ctx.beginPath(); pathPoly(c.local.q); ctx.fill();
    ctx.fillStyle = C.building; ctx.beginPath(); pathPoly(c.local.interieur); ctx.fill();
    if (view.s > 3) {
      const q = c.local.q, m = toS([(q[0][0] + q[2][0]) / 2, (q[0][1] + q[2][1]) / 2]);
      const FONT = getComputedStyle(document.body).fontFamily;
      ctx.fillStyle = C["building-label"]; ctx.textAlign = "center"; ctx.textBaseline = "middle";
      ctx.font = `600 ${Math.min(13, 0.9 * view.s)}px ${FONT}`; ctx.fillText("Accueil", m[0], m[1] - 0.6 * view.s);
      ctx.font = `400 ${Math.min(11, 0.75 * view.s)}px ${FONT}`; ctx.fillText(fmt1(c.local.surface) + " m²", m[0], m[1] + 0.6 * view.s);
    }
  }
}
function updateNiv() {
  const L = nivListe(), a = nivActif(), el = $("#niv");
  el.hidden = !L.length;
  el.replaceChildren(...L.map((x) => {
    const b = document.createElement("button"), nom = document.createElement("span"), n = document.createElement("small");
    b.type = "button"; b.dataset.n = x.id; b.setAttribute("role", "radio"); b.setAttribute("aria-checked", String(x.id === a));
    b.setAttribute("aria-label", `${x.nom}, ${x.places} places`); nom.textContent = x.nom; n.textContent = x.places;
    b.append(nom, n); return b;
  }));
}
$("#niv").addEventListener("click", (ev) => {
  const b = ev.target.closest("button[data-n]"); if (!b) return;
  NIV = b.dataset.n; updateNiv(); redraw();
});
