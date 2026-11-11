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
function nivActif() { const L = nivListe(); return L.length ? (L.find((x) => x.id === NIV) || L[L.length - 1]).id : "courant"; }
// Places propres au niveau affiché, en plus de celles de l'étage courant.
function nivPlaces() {
  const o = layout.ouvrage, a = o ? nivActif() : "courant";
  return a === "rdc" ? o.places_rdc_seul || [] : a === "dernier" ? o.places_dernier_seul || [] : [];
}
// Épures de giration du niveau affiché : pas de bas de rampe au dernier niveau, pas de haut au rez.
function nivGirations() { const a = nivActif(); return layout.ouvrage.girations.filter((g) => a === "courant" || g.bas === (a === "rdc")); }
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
