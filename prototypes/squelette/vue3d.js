// Vue 3D du squelette (lot 2 du plan claude/plan-vue-3d.md) : la scène décrite par scene3d.js, rendue par three.js.
// La vue ne décide rien : elle affiche ce que la scène décrit, et se reconstruit après chaque calcul du squelette.
// Chargée par le squelette avec le drapeau ?3d dans l'adresse. Rendu à la demande : aucune image sans changement.
/* global S, garder, enregistrer, rafraichir */
import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { LineSegments2 } from "three/addons/lines/LineSegments2.js";
import { LineSegmentsGeometry } from "three/addons/lines/LineSegmentsGeometry.js";
import { LineMaterial } from "three/addons/lines/LineMaterial.js";
import { EffectComposer } from "three/addons/postprocessing/EffectComposer.js";
import { RenderPass } from "three/addons/postprocessing/RenderPass.js";
import { GTAOPass } from "three/addons/postprocessing/GTAOPass.js";
import { OutputPass } from "three/addons/postprocessing/OutputPass.js";
import clip from "polygon-clipping";
import { extraire } from "./plateau.js";
import { scene as decrire } from "./scene3d.js";

// ---- Rendus. Trait : faces blanches, arêtes noires, ombres légères (vidéo 2). Maquette : carton blanc, arêtes fines,
// ombres douces et occlusion ambiante au repos (vidéo 1). La couleur porte le sens, comme dans le plan.
const BASE = { terrain: "#E6E8E4", dallage: "#F1F1EF", radier: "#EDEDEB", dalle: "#F7F7F5", couverture: "#F7F7F5", enrobe: "#E1E3E1", place: "#ECEDEB",
  constat: "#F2B3AE", pieton: "#C6DAEE", zebre: "#FFFFFF", poteau: "#FAFAF8", poteau_constat: "#FF6B61", poutre: "#F0F0EE", voile: "#EBE9E5",
  maconnerie: "#E4DCD1", noyau: "#FBFBF9", marche: "#F4F4F2", palier: "#F4F4F2", ascenseur: "#D6DCE2", rampe: "#E8DCC3", rampe_constat: "#F2B3AE",
  rive: "#F8F8F6", allege: "#F8F8F6" };
const SOMBRE = { terrain: "#1E2326", dallage: "#343B40", radier: "#30363A", dalle: "#3B4247", couverture: "#3B4247", enrobe: "#2E3438", place: "#363D42",
  constat: "#7A3A36", pieton: "#2F4C69", zebre: "#C9D0CD", poteau: "#454D53", poteau_constat: "#FF453A", poutre: "#40474D", voile: "#3E454A",
  maconnerie: "#4A443D", noyau: "#4A5258", marche: "#42494F", palier: "#42494F", ascenseur: "#38424C", rampe: "#5E5240", rampe_constat: "#7A3A36",
  rive: "#4A5258", allege: "#4A5258" };
const TRAIT = { ...Object.fromEntries(Object.keys(BASE).map((k) => [k, "#FFFFFF"])), terrain: "#F3F3F1", enrobe: "#F2F2F2", constat: "#F7CBC7",
  pieton: "#E3ECF6", rampe: "#F6EFE1", rampe_constat: "#F7CBC7", poteau_constat: "#FF8A80", ascenseur: "#EEF0F2", maconnerie: "#F5F1EB" };
const sombre = matchMedia("(prefers-color-scheme: dark)");
const RENDUS = {
  trait: () => ({ couleurs: sombre.matches ? SOMBRE : TRAIT, arete: sombre.matches ? "#E9EDEB" : "#1B201E", opacite: 1, epaisseur: 1.25, marque: sombre.matches ? "#8E979D" : "#9AA0A5", ciel: sombre.matches ? 1.2 : 2.3, soleil: 0.9, ombre: 0.32, ao: false }),
  maquette: () => ({ couleurs: sombre.matches ? SOMBRE : BASE, arete: sombre.matches ? "#E9EDEB" : "#2B3136", opacite: sombre.matches ? 0.28 : 0.32, epaisseur: 1, marque: sombre.matches ? "#7D868C" : "#A2A9AE", ciel: sombre.matches ? 1.0 : 1.5, soleil: 1.7, ombre: 0.55, ao: true }),
};
const OMBRE_PORTEE = new Set(["sols", "terrain"]); // ne portent pas d'ombre
const ECORCHE = new Set(["couverture"]); // parc enterré : la couverture est levée, comme le couvercle d'une maquette
const css = (v) => getComputedStyle(document.documentElement).getPropertyValue(v).trim();
const reglages = (() => { try { return { rendu: "maquette", ...JSON.parse(localStorage.getItem("parkimo.vue3d") || "{}") }; } catch (e) { return { rendu: "maquette" }; } })();
const garderReglages = () => { try { localStorage.setItem("parkimo.vue3d", JSON.stringify(reglages)); } catch (e) { /* stockage indisponible */ } };

// ---- Interface : le canevas sous les panneaux du squelette, le bouton 3D de la barre, le panneau de la vue.
document.head.insertAdjacentHTML("beforeend", `<style>
#vue3d { position: fixed; inset: 0; width: 100%; height: 100%; touch-action: none; display: none; opacity: 0; transition: opacity .22s ease; }
body.en3d #vue3d { display: block; } body.en3d #vue3d.visible { opacity: 1; }
body.en3d #aide { display: none; }
#p3d { position: fixed; right: max(16px, env(safe-area-inset-right)); top: calc(16px + env(safe-area-inset-top)); border-radius: 20px; padding: 12px 16px; min-width: 260px; display: none; }
body.en3d #p3d { display: block; }
#p3d-det > summary { display: grid; grid-template-columns: 1fr auto 14px; column-gap: 12px; align-items: center; cursor: pointer; list-style: none; min-height: 24px; }
#p3d-det > summary::-webkit-details-marker { display: none; }
#p3d-det > summary h1 { margin: 0; font-size: 12px; font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--label-2); }
#p3d-det > summary span { font-size: 13px; color: var(--label-2); font-variant-numeric: tabular-nums; }
#p3d-det > summary::after { content: ""; width: 7px; height: 7px; border-right: 1.5px solid var(--label-2); border-bottom: 1.5px solid var(--label-2); transform: rotate(-135deg) translate(-2px, -2px); transition: transform .15s; justify-self: end; }
#p3d-det:not([open]) > summary::after { transform: rotate(45deg) translate(-2px, -2px); }
#p3d .rang { display: grid; grid-template-columns: 62px 1fr; align-items: center; gap: 8px; margin-top: 8px; font-size: 13px; color: var(--label-2); }
#p3d .pas { display: grid; grid-template-columns: 44px 1fr 44px; align-items: center; background: rgba(127,127,127,.16); border-radius: 14px; padding: 2px; }
#p3d .pas button { min-height: 40px; min-width: 0; padding: 0; border-radius: 12px; font-size: 20px; font-weight: 400; }
#p3d .pas output { text-align: center; color: var(--label); font-weight: 600; font-variant-numeric: tabular-nums; }
#p3d-constats { margin: 8px 0 0; font-size: 12px; color: var(--rouge); max-width: 240px; }
#p3d-constats:empty { display: none; }
@media (max-width: 760px) { #p3d { right: max(8px, env(safe-area-inset-right)); top: calc(80px + env(safe-area-inset-top)); padding: 8px 14px; border-radius: 18px; min-width: 0; } } /* sous le bilan replié */
</style>`);
const toile = document.createElement("canvas");
toile.id = "vue3d"; toile.setAttribute("aria-label", "Vue 3D du parc");
document.querySelector("#plan").after(toile);
document.querySelector("#bilan").insertAdjacentHTML("afterend", `<section id="p3d" class="verre" aria-label="Vue 3D">
  <details id="p3d-det"${innerWidth < 760 ? "" : " open"}><summary><h1>Vue 3D</h1><span id="p3d-resume">–</span></summary>
  <div class="rang"><span>Niveaux</span><div class="pas"><button id="p3d-moins" aria-label="Un niveau de moins">−</button><output id="p3d-n">4</output><button id="p3d-plus" aria-label="Un niveau de plus">+</button></div></div>
  <div class="rang"><span>Rendu</span><div class="seg" id="p3d-rendu"><button data-r="trait">Trait</button><button data-r="maquette">Maquette</button></div></div>
  <p id="p3d-constats"></p></details></section>`);
const $ = (s) => document.querySelector(s), bouton = $("#b-3d");
bouton.hidden = false;

// ---- three.js
const renderer = new THREE.WebGLRenderer({ canvas: toile, antialias: true });
renderer.setPixelRatio(Math.min(devicePixelRatio || 1, 2));
renderer.shadowMap.enabled = true; renderer.shadowMap.type = THREE.PCFShadowMap;
const monde = new THREE.Scene(), modele = new THREE.Group();
monde.add(modele);
const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 1, 4000);
const controles = new OrbitControls(camera, toile);
controles.enableDamping = true; controles.dampingFactor = 0.12; controles.zoomToCursor = true;
controles.maxPolarAngle = Math.PI / 2 - 0.02; controles.minZoom = 0.15; controles.maxZoom = 40;
const ciel = new THREE.HemisphereLight(0xffffff, 0xd8dad6, 1.5), soleil = new THREE.DirectionalLight(0xffffff, 1.7);
soleil.castShadow = true; soleil.shadow.bias = -0.0004; soleil.shadow.normalBias = 0.03; soleil.shadow.radius = 3;
soleil.shadow.mapSize.set(innerWidth < 760 ? 1024 : 2048, innerWidth < 760 ? 1024 : 2048);
monde.add(ciel, soleil, soleil.target);
let composer = null, ao = null, aretesAO = [];
let W = 0, H = 0, echelle = 6, sc = null, actif = false, sale = true, attente = false, repos = 0, premiere = true, place = false;
const materiaux = new Map(), lignes = { aretes: new LineMaterial({ linewidth: 1, transparent: true }), marques: new LineMaterial({ linewidth: 1 }) };

// Plan (x, y) et cote z vers three.js : y vers le haut, le nord vers -z. Vu du dessus, le plan garde son orientation.
const V3 = (x, y, z) => [x, z, -y];

// Une face (triangle) tournée vers `n` : l'ordre des sommets est corrigé au besoin.
function tri(pos, a, b, c, n) {
  const u0 = b[0] - a[0], u1 = b[1] - a[1], u2 = b[2] - a[2], v0 = c[0] - a[0], v1 = c[1] - a[1], v2 = c[2] - a[2];
  if ((u1 * v2 - u2 * v1) * n[0] + (u2 * v0 - u0 * v2) * n[1] + (u0 * v1 - u1 * v0) * n[2] < 0) [b, c] = [c, b];
  pos.push(a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2]);
}
const HAUT = [0, 1, 0], BAS = [0, -1, 0], oppose = (n) => [-n[0], -n[1], -n[2]];
const seg = (ar, a, b) => ar.push(a[0], a[1], a[2], b[0], b[1], b[2]);

// Prisme droit : dessus, dessous, côtés ; arêtes du contour en haut et en bas, verticales aux angles francs.
function prisme(v, pos, ar) {
  const R = v.anneaux, pts = R.flat(), tris = THREE.ShapeUtils.triangulateShape(R[0].map((p) => new THREE.Vector2(p[0], p[1])), R.slice(1).map((r) => r.map((p) => new THREE.Vector2(p[0], p[1]))));
  for (const [a, b, c] of tris) {
    tri(pos, V3(...pts[a], v.z1), V3(...pts[b], v.z1), V3(...pts[c], v.z1), HAUT);
    tri(pos, V3(...pts[a], v.z0), V3(...pts[b], v.z0), V3(...pts[c], v.z0), BAS);
  }
  for (const r of R) r.forEach((p, k) => {
    const q = r[(k + 1) % r.length], o = r[(k + r.length - 1) % r.length], d = [q[0] - p[0], q[1] - p[1]], l = Math.hypot(d[0], d[1]) || 1;
    const n = [d[1] / l, 0, d[0] / l]; // à droite du sens de parcours : vers l'extérieur
    const a0 = V3(...p, v.z0), b0 = V3(...q, v.z0), b1 = V3(...q, v.z1), a1 = V3(...p, v.z1);
    tri(pos, a0, b0, b1, n); tri(pos, a0, b1, a1, n);
    seg(ar, a0, b0); seg(ar, a1, b1);
    const e = [p[0] - o[0], p[1] - o[1]], le = Math.hypot(e[0], e[1]) || 1;
    if ((e[0] * d[0] + e[1] * d[1]) / (le * l) < Math.cos(0.35)) seg(ar, a0, a1);
  });
}

// Ruban : section rectangulaire balayée le long d'un axe en trois dimensions (rampes, hélices, rives).
function ruban(r, pos, ar) {
  const A = r.axe, k = A.length, g = r.decalage + r.largeur / 2, d = r.decalage - r.largeur / 2;
  const P = A.map((p, i) => { const a = A[Math.max(0, i - 1)], b = A[Math.min(k - 1, i + 1)], dx = b[0] - a[0], dy = b[1] - a[1], l = Math.hypot(dx, dy) || 1, n = [-dy / l, dx / l];
    return { g: [p[0] + n[0] * g, p[1] + n[1] * g], d: [p[0] + n[0] * d, p[1] + n[1] * d], z: p[2], n: [n[0], 0, -n[1]], t: [dx / l, 0, -dy / l] }; });
  const c = (p) => ({ gh: V3(...p.g, p.z + r.z1), dh: V3(...p.d, p.z + r.z1), db: V3(...p.d, p.z + r.z0), gb: V3(...p.g, p.z + r.z0) });
  const C = P.map(c);
  for (let i = 0; i + 1 < k; i++) {
    const a = C[i], b = C[i + 1], n = P[i].n, nd = oppose(n);
    tri(pos, a.gh, a.dh, b.dh, HAUT); tri(pos, a.gh, b.dh, b.gh, HAUT);
    tri(pos, a.gb, a.db, b.db, BAS); tri(pos, a.gb, b.db, b.gb, BAS);
    tri(pos, a.gh, a.gb, b.gb, n); tri(pos, a.gh, b.gb, b.gh, n);
    tri(pos, a.dh, a.db, b.db, nd); tri(pos, a.dh, b.db, b.dh, nd);
    for (const e of ["gh", "dh", "db", "gb"]) seg(ar, a[e], b[e]);
  }
  for (const [i, s] of [[0, -1], [k - 1, 1]]) { const a = C[i], n = P[i].t.map((x) => x * s);
    tri(pos, a.gh, a.dh, a.db, n); tri(pos, a.gh, a.db, a.gb, n); seg(ar, a.gh, a.dh); seg(ar, a.dh, a.db); seg(ar, a.db, a.gb); seg(ar, a.gb, a.gh); }
}

// Sol peint : la face du dessus seule.
function sol(s, pos) {
  const R = s.anneaux, pts = R.flat(), tris = THREE.ShapeUtils.triangulateShape(R[0].map((p) => new THREE.Vector2(p[0], p[1])), R.slice(1).map((r) => r.map((p) => new THREE.Vector2(p[0], p[1]))));
  for (const [a, b, c] of tris) tri(pos, V3(...pts[a], s.z), V3(...pts[b], s.z), V3(...pts[c], s.z), HAUT);
}

function materiau(m) {
  if (!materiaux.has(m)) {
    // Les volumes reculent d'un cran : leurs arêtes restent visibles. Les sols peints, quelques millimètres au-dessus de
    // la dalle, n'en ont pas besoin ; les avancer ferait passer les marquages dessous.
    const peint = ["enrobe", "place", "constat", "pieton", "zebre"].includes(m);
    materiaux.set(m, new THREE.MeshLambertMaterial({ color: 0xffffff, polygonOffset: !peint, polygonOffsetFactor: 1, polygonOffsetUnits: 1 }));
  }
  return materiaux.get(m);
}

// ---- Construction : un groupe par niveau (l'éclaté viendra les écarter), un maillage par matière, deux jeux de traits.
function vider(o) { for (const c of [...o.children]) { vider(c); if (c.geometry) c.geometry.dispose(); o.remove(c); } }
function construire() {
  const t0 = performance.now(), P = extraire();
  sc = decrire(P, clip);
  const t1 = performance.now();
  vider(modele); aretesAO = [];
  const niveaux = new Map(), du = (f) => { if (!niveaux.has(f)) niveaux.set(f, { mat: new Map(), ar: [], mq: [] }); return niveaux.get(f); };
  const de = (N, m) => { if (!N.mat.has(m)) N.mat.set(m, []); return N.mat.get(m); };
  for (const v of sc.volumes) { if (ECORCHE.has(v.matiere)) continue; const N = du(v.niveau); prisme(v, de(N, v.matiere), N.ar); }
  for (const r of sc.rubans) { const N = du(r.niveau); ruban(r, de(N, r.matiere), N.ar); }
  for (const s of sc.sols) { const N = du(s.niveau); sol(s, de(N, `sol:${s.matiere}`)); }
  for (const t of sc.traits) { const N = du(t.niveau); seg(N.mq, V3(...t.a, t.z), V3(...t.b, t.z)); }
  for (const [f, N] of niveaux) {
    const g = new THREE.Group(); g.name = `niveau ${f}`; g.userData.niveau = f;
    for (const [m, pos] of N.mat) {
      const geo = new THREE.BufferGeometry(); geo.setAttribute("position", new THREE.Float32BufferAttribute(pos, 3)); geo.computeVertexNormals();
      const nom = m.startsWith("sol:") ? m.slice(4) : m, me = new THREE.Mesh(geo, materiau(nom)); me.name = nom;
      me.receiveShadow = true; me.castShadow = !m.startsWith("sol:") && !OMBRE_PORTEE.has(nom);
      g.add(me);
    }
    for (const [cle, l, mat] of [["aretes", N.ar, lignes.aretes], ["marques", N.mq, lignes.marques]]) {
      if (!l.length) continue;
      const geo = new LineSegmentsGeometry(); geo.setPositions(l);
      const li = new LineSegments2(geo, mat); li.name = cle; g.add(li); aretesAO.push(li);
    }
    modele.add(g);
  }
  sale = false;
  appliquerRendu();
  ombres();
  $("#p3d-n").textContent = P ? P.niveaux : "–";
  $("#p3d-resume").textContent = sc.places ? `${sc.niveaux.filter((x) => x.places).length} niv. · ${sc.places.toLocaleString("fr-FR")} pl.` : "aucun plan";
  $("#p3d-constats").textContent = sc.constats.join(" · ");
  window.vue3d.mesure = { scene: +(t1 - t0).toFixed(1), maillage: +(performance.now() - t1).toFixed(1), volumes: sc.volumes.length, rubans: sc.rubans.length, sols: sc.sols.length };
  if (premiere) { premiere = false; cadrer(); }
}

function appliquerRendu() {
  const R = (RENDUS[reglages.rendu] || RENDUS.maquette)();
  for (const [m, mat] of materiaux) mat.color.set(R.couleurs[m] || "#FF00FF");
  lignes.aretes.color.set(R.arete); lignes.aretes.opacity = R.opacite; lignes.aretes.linewidth = R.epaisseur;
  lignes.marques.color.set(R.marque); lignes.marques.linewidth = 1;
  ciel.intensity = R.ciel; ciel.groundColor.set(sombre.matches ? 0x202528 : 0xd8dad6); soleil.intensity = R.soleil; soleil.shadow.intensity = R.ombre;
  monde.background = new THREE.Color(css("--fond") || "#EEF1EF");
  for (const b of document.querySelectorAll("#p3d-rendu button")) b.setAttribute("aria-pressed", String(b.dataset.r === reglages.rendu));
}

// Le soleil vient de l'ouest-sud-ouest, assez haut : les ombres tombent vers la droite de l'axonométrie.
function ombres() {
  if (!sc) return;
  const c = new THREE.Vector3(...V3((sc.min[0] + sc.max[0]) / 2, (sc.min[1] + sc.max[1]) / 2, (sc.haut + sc.bas) / 2)), r = Math.hypot(sc.max[0] - sc.min[0], sc.max[1] - sc.min[1], sc.haut - sc.bas) / 2 + 14;
  soleil.target.position.copy(c); soleil.position.copy(c).add(new THREE.Vector3(-1, 1.6, 0.55).normalize().multiplyScalar(r * 2));
  const o = soleil.shadow.camera; o.left = o.bottom = -r; o.right = o.top = r; o.near = 0.5; o.far = r * 4; o.updateProjectionMatrix();
  soleil.shadow.needsUpdate = true;
}

// ---- Caméra : axonométrie orthographique, à 30° environ au-dessus de l'horizon. Cadrer : tout le parc dans la vue
// libre, sous le même angle. À la première vue, vu du sud-ouest, tourné de 30° au plus si le parc remplit mieux l'écran
// (un téléphone tenu droit) : la vue reste de biais, jamais dans l'axe du bâtiment.
const SUD_OUEST = Math.atan2(1, -1); // azimut de la caméra (plan three.js x, z) : vers l'ouest et le sud
function echelleVue(c, dir) {
  camera.position.copy(c).add(dir.clone().multiplyScalar(1500)); camera.up.set(0, 1, 0); camera.lookAt(c); camera.updateMatrixWorld();
  const droite = new THREE.Vector3().setFromMatrixColumn(camera.matrixWorld, 0), haut = new THREE.Vector3().setFromMatrixColumn(camera.matrixWorld, 1);
  let ex = 0, ey = 0;
  for (const x of [sc.min[0], sc.max[0]]) for (const y of [sc.min[1], sc.max[1]]) for (const z of [sc.bas, sc.haut]) { const p = new THREE.Vector3(...V3(x, y, z)).sub(c); ex = Math.max(ex, Math.abs(p.dot(droite))); ey = Math.max(ey, Math.abs(p.dot(haut))); }
  const etroit = W < 760, hautLibre = H - (etroit ? 230 : 150), largeLibre = W - (etroit ? 24 : 120);
  return Math.max(0.5, Math.min(largeLibre / (2 * ex), hautLibre / (2 * ey)));
}
const azimut = (a) => new THREE.Vector3(Math.cos(a), 0.6, Math.sin(a)).normalize();
function cadrer() {
  if (!sc || !sc.volumes.length) return;
  const c = new THREE.Vector3(...V3((sc.min[0] + sc.max[0]) / 2, (sc.min[1] + sc.max[1]) / 2, (sc.haut + sc.bas) / 2));
  let dir = place ? camera.position.clone().sub(controles.target).normalize() : azimut(SUD_OUEST);
  if (!place) { let mieux = -1; for (let k = -2; k <= 2; k++) { const a = SUD_OUEST + (k * Math.PI) / 12, e = echelleVue(c, azimut(a)) * (1 - 0.04 * Math.abs(k)); if (e > mieux) { mieux = e; dir = azimut(a); } } }
  place = true; controles.target.copy(c); echelle = echelleVue(c, dir);
  camera.zoom = 1; cadrage(); controles.update(); demander();
}
function cadrage() { camera.left = -W / 2 / echelle; camera.right = W / 2 / echelle; camera.top = H / 2 / echelle; camera.bottom = -H / 2 / echelle; camera.updateProjectionMatrix(); }

function taille() {
  W = toile.clientWidth; H = toile.clientHeight; if (!W || !H) return;
  renderer.setSize(W, H, false); cadrage();
  if (composer) { composer.setSize(W, H); ao.setSize(W * renderer.getPixelRatio(), H * renderer.getPixelRatio()); }
  demander();
}

// ---- Rendu à la demande. Pendant un mouvement : image simple. Au repos : occlusion ambiante en rendu Maquette.
function demander() { clearTimeout(repos); if (!attente && actif) { attente = true; requestAnimationFrame(image); } }
function image() {
  attente = false; if (!actif) return;
  if (sale) construire();
  const bouge = controles.update();
  renderer.render(monde, camera);
  if (bouge) demander(); else repos = setTimeout(soigne, 140);
}
function soigne() {
  const R = (RENDUS[reglages.rendu] || RENDUS.maquette)();
  if (!actif || !R.ao) return;
  if (!composer) {
    composer = new EffectComposer(renderer); composer.addPass(new RenderPass(monde, camera));
    ao = new GTAOPass(monde, camera, W, H); ao.blendIntensity = 0.85;
    ao.updateGtaoMaterial({ radius: 1.2, distanceExponent: 1.5, thickness: 1.5, scale: 1.0, samples: 16 });
    const rendu = ao.render.bind(ao); // les traits n'entrent pas dans le calcul de l'occlusion : ils restent dans l'image
    ao.render = (...a) => { for (const l of aretesAO) l.visible = false; rendu(...a); for (const l of aretesAO) l.visible = true; };
    composer.addPass(ao); composer.addPass(new OutputPass()); composer.setSize(W, H);
  }
  composer.render();
}

// ---- Bascule 2D / 3D : un fondu ; la levée de la maquette depuis le plan viendra au lot 3.
function entrer() {
  if (actif) return; actif = true; document.body.classList.add("en3d"); bouton.setAttribute("aria-pressed", "true");
  taille(); demander(); requestAnimationFrame(() => toile.classList.add("visible"));
}
function sortir() {
  if (!actif) return; actif = false; clearTimeout(repos); bouton.setAttribute("aria-pressed", "false"); toile.classList.remove("visible");
  document.body.classList.remove("en3d");
}
bouton.addEventListener("click", () => (actif ? sortir() : entrer()));
// Les commandes du plan gardent leur place : Emprise et Tracer ramènent au plan et entrent dans leur mode ; Cadrer cadre la vue active.
for (const id of ["#b-emprise", "#b-tracer"]) $(id).addEventListener("click", () => sortir(), { capture: true });
$("#b-cadrer").addEventListener("click", (ev) => { if (!actif) return; ev.stopImmediatePropagation(); cadrer(); }, { capture: true });
controles.addEventListener("change", demander);
new ResizeObserver(taille).observe(toile);
sombre.addEventListener("change", () => { appliquerRendu(); demander(); });

// ---- Panneau : niveaux (annulables, enregistrés avec le plan) et rendu (réglage de la vue, gardé dans ce navigateur).
const niveaux = (d) => { const n = Math.max(1, Math.min(12, Math.round(S.niveaux || 4) + d)); if (n === (S.niveaux || 4)) return; garder(); S.niveaux = n; enregistrer(); rafraichir(); };
$("#p3d-moins").addEventListener("click", () => niveaux(-1));
$("#p3d-plus").addEventListener("click", () => niveaux(1));
for (const b of document.querySelectorAll("#p3d-rendu button")) b.addEventListener("click", () => { reglages.rendu = b.dataset.r; garderReglages(); appliquerRendu(); demander(); });

// Le squelette appelle maj() après chaque calcul : la scène sera reconstruite à la prochaine image, si la vue est ouverte.
window.vue3d = { maj() { sale = true; demander(); }, entrer, sortir, cadrer, camera, controles, mesure: null, scene: () => sc };
