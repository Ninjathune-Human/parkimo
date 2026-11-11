// ============================================================================
// Fond de carte IGN : recherche, emprise, photo découpée, calques cadastraux.
// Services de la Géoplateforme (data.geopf.fr) : WMTS, WFS Parcellaire Express, géocodage.
// ============================================================================
const GEO = { wmts: "https://data.geopf.fr/wmts", wfs: "https://data.geopf.fr/wfs/ows", geocode: "https://data.geopf.fr/geocodage/search" };
const R_EARTH = 6378137, HALF = Math.PI * R_EARTH;
const toMerc = (lon, lat) => [lon * Math.PI / 180 * R_EARTH, R_EARTH * Math.log(Math.tan(Math.PI / 4 + lat * Math.PI / 360))];
const fromMerc = (x, y) => [x / R_EARTH * 180 / Math.PI, (2 * Math.atan(Math.exp(y / R_EARTH)) - Math.PI / 2) * 180 / Math.PI];
const mpp = (z) => (2 * HALF) / (256 * 2 ** z);
const TMS = ["PM_0_19", "PM"];
let tmsIdx = 0;
const ORTHO = { layer: "ORTHOIMAGERY.ORTHOPHOTOS", style: "normal", fmt: "image/jpeg", max: 19 };
const CADR = { layer: "CADASTRALPARCELS.PARCELLAIRE_EXPRESS", style: "PCI vecteur", fmt: "image/png", max: 19 };
const CAD_TYPES = { parcelles: "parcelle", bati: "batiment", feuilles: "feuille", communes: "commune" };
// Calques : ce que l'on affiche ou masque. Chacun devient un calque AutoCAD à l'export.
const LAYER_DEFAULTS = {
  places: true, pvoirie: true, girations: true, noyaux: true, structure: true, sens: true, ilots: true, pbati: true, servitudes: true, recul: true, vehicules: true, arbres: true,
  ovoirie: true, bati: true, vegetation: true,
  photo: true, parcelles: true, numeros: true, feuilles: false, communes: true,
  girroues: true, girenv: true, girmarge: true, girconf: true, girplaces: true,
};
function tileUrl(L, z, x, y) {
  return `${GEO.wmts}?SERVICE=WMTS&REQUEST=GetTile&VERSION=1.0.0&LAYER=${L.layer}&STYLE=${encodeURIComponent(L.style)}&TILEMATRIXSET=${TMS[tmsIdx]}&TILEMATRIX=${z}&TILEROW=${y}&TILECOL=${x}&FORMAT=${encodeURIComponent(L.fmt)}`;
}

let basemap = null;   // fond importé : repère local, photo découpée, calques
let mineSite = null;  // « Mon site » mis de côté pendant la consultation d'un exemple

// ---- Carte plein écran (mode « map »)
const France = toMerc(2.6, 46.6);
const mapv = { x: France[0], y: France[1], z: 6 };
let mapRect = null, mapCad = true, mapStatus = { ok: 0, err: 0 };
const tiles = new Map();
function mToS(x, y) { const r = mpp(mapv.z); return [W / 2 + (x - mapv.x) / r, H / 2 - (y - mapv.y) / r]; }
function sToM(sx, sy) { const r = mpp(mapv.z); return [mapv.x + (sx - W / 2) * r, mapv.y - (sy - H / 2) * r]; }
const cosAt = (y) => Math.cos(fromMerc(0, y)[1] * Math.PI / 180);

// Image avec CORS si le serveur l'accepte, sinon sans (affichage seul).
function loadImage(url, onload, onerror, cors = true) {
  const im = new Image();
  if (cors) im.crossOrigin = "anonymous";
  im.onload = () => onload(im, cors);
  im.onerror = () => (cors ? loadImage(url, onload, onerror, false) : onerror());
  im.src = url;
}
function getTile(L, z, x, y) {
  const key = `${L.layer}/${TMS[tmsIdx]}/${z}/${x}/${y}`;
  let t = tiles.get(key);
  if (!t) {
    t = { img: null };
    tiles.set(key, t);
    loadImage(tileUrl(L, z, x, y), (im) => { t.img = im; mapStatus.ok++; redraw(); }, () => { t.err = true; mapStatus.err++; tileFailure(); });
    if (tiles.size > 700) tiles.delete(tiles.keys().next().value);
  }
  return t;
}
function tileFailure() {
  if (mapStatus.ok > 0 || mapStatus.err < 6) return;
  if (tmsIdx < TMS.length - 1) { tmsIdx++; tiles.clear(); mapStatus = { ok: 0, err: 0 }; redraw(); }
  else $("#mapmsg").hidden = false;
}
function drawTiles(L, alpha) {
  const tz = Math.max(0, Math.min(L.max, Math.round(mapv.z))), n = 2 ** tz, r = mpp(mapv.z), ts = 256 * 2 ** (mapv.z - tz);
  const [x0, y1] = sToM(0, 0), [x1, y0] = sToM(W, H);
  const tx0 = Math.max(0, Math.floor((x0 + HALF) / (2 * HALF) * n)), tx1 = Math.min(n - 1, Math.floor((x1 + HALF) / (2 * HALF) * n));
  const ty0 = Math.max(0, Math.floor((HALF - y1) / (2 * HALF) * n)), ty1 = Math.min(n - 1, Math.floor((HALF - y0) / (2 * HALF) * n));
  if ((tx1 - tx0 + 1) * (ty1 - ty0 + 1) > 400) return;
  ctx.globalAlpha = alpha;
  for (let ty = ty0; ty <= ty1; ty++) for (let tx = tx0; tx <= tx1; tx++) {
    const sx = W / 2 + ((tx / n) * 2 * HALF - HALF - mapv.x) / r, sy = H / 2 - (HALF - (ty / n) * 2 * HALF - mapv.y) / r;
    const t = getTile(L, tz, tx, ty);
    if (t.img) { ctx.drawImage(t.img, sx, sy, ts + 0.6, ts + 0.6); continue; }
    for (let up = 1; up <= 5 && tz - up >= 0; up++) { // tuile parente en attendant
      const p = tiles.get(`${L.layer}/${TMS[tmsIdx]}/${tz - up}/${tx >> up}/${ty >> up}`);
      if (p && p.img) { const f = 2 ** up, sub = 256 / f; ctx.drawImage(p.img, (tx % f) * sub, (ty % f) * sub, sub, sub, sx, sy, ts + 0.6, ts + 0.6); break; }
    }
  }
  ctx.globalAlpha = 1;
}
function rectCorners(r) { return [[r.x0, r.y0], [r.x1, r.y0], [r.x1, r.y1], [r.x0, r.y1]]; }
function drawMap() {
  ctx.fillStyle = C.ground; ctx.fillRect(0, 0, W, H);
  drawTiles(ORTHO, 1);
  if (mapCad && mapv.z >= 15) drawTiles(CADR, 0.95);
  if (mapRect) {
    const a = mToS(mapRect.x0, mapRect.y1), b = mToS(mapRect.x1, mapRect.y0);
    ctx.fillStyle = "rgba(8, 12, 10, .5)"; ctx.beginPath(); ctx.rect(0, 0, W, H); ctx.rect(a[0], a[1], b[0] - a[0], b[1] - a[1]); ctx.fill("evenodd");
    ctx.strokeStyle = C.accent; ctx.lineWidth = 2; ctx.strokeRect(a[0], a[1], b[0] - a[0], b[1] - a[1]);
    rectCorners(mapRect).forEach((c) => squareHandle(mToS(c[0], c[1]), true, 12));
  }
  const lat = fromMerc(mapv.x, mapv.y)[1] * Math.PI / 180;
  drawScale(1 / (mpp(mapv.z) * Math.cos(lat)));
}
function defaultRect(wm, hm, cx = mapv.x, cy = mapv.y) {
  const k = cosAt(cy), w = wm / k / 2, h = hm / k / 2;
  mapRect = { x0: cx - w, y0: cy - h, x1: cx + w, y1: cy + h };
}
function photoZoom(r) { let z = ORTHO.max; while (z > 12 && Math.max(r.x1 - r.x0, r.y1 - r.y0) / mpp(z) > 4096) z--; return z; }
function updateMapUI() {
  $("#m-ok").disabled = !mapRect;
  $("#m-cad").setAttribute("aria-pressed", String(mapCad)); $("#m-cad").classList.toggle("on", mapCad);
  const info = $("#mapinfo");
  if (!mapRect) { info.textContent = mapv.z < 14 ? "Cherchez un lieu, puis posez l'emprise" : "Touchez Emprise pour délimiter la zone à importer"; return; }
  const k = cosAt((mapRect.y0 + mapRect.y1) / 2), w = (mapRect.x1 - mapRect.x0) * k, h = (mapRect.y1 - mapRect.y0) * k;
  const area = w * h >= 10000 ? fmt1(w * h / 10000) + " ha" : fmtInt(w * h) + " m²";
  info.textContent = `${fmtInt(w)} × ${fmtInt(h)} m · ${area} · photo ${nf2.format(mpp(photoZoom(mapRect)) * k)} m/pixel`;
}
const nf2 = new Intl.NumberFormat("fr-FR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });

function enterMap() {
  setMode("edit"); closeMenus(); setSel(null);
  mode = "map"; document.body.classList.add("mapmode"); $("#mapui").hidden = false; $("#mapmsg").hidden = true;
  mapStatus = { ok: 0, err: 0 };
  if (basemap) {
    mapRect = { ...basemap.mrect };
    const span = Math.max(mapRect.x1 - mapRect.x0, mapRect.y1 - mapRect.y0);
    mapv.x = (mapRect.x0 + mapRect.x1) / 2; mapv.y = (mapRect.y0 + mapRect.y1) / 2;
    mapv.z = Math.min(19, Math.log2((2 * HALF) / 256 * Math.min(W, H) * 0.6 / span));
  }
  updateMapUI(); redraw();
  if (!basemap && matchMedia("(pointer: fine)").matches) setTimeout(() => $("#q").focus(), 60);
}
function exitMap() {
  mode = "edit"; document.body.classList.remove("mapmode"); $("#mapui").hidden = true; closeResults();
  canvas.classList.remove("dragging"); redraw();
}

// Interaction carte : glisser pour se déplacer, coins et intérieur de l'emprise pour l'ajuster.
function mapHit(sp) {
  if (!mapRect) return null;
  const cs = rectCorners(mapRect);
  for (let k = 0; k < 4; k++) { const p = mToS(cs[k][0], cs[k][1]); if (Math.hypot(p[0] - sp[0], p[1] - sp[1]) < 20) return { kind: "mcorner", k }; }
  const a = mToS(mapRect.x0, mapRect.y1), b = mToS(mapRect.x1, mapRect.y0);
  if (sp[0] > a[0] && sp[0] < b[0] && sp[1] > a[1] && sp[1] < b[1]) return { kind: "mmove" };
  return null;
}
function mapDown(sp) {
  closeResults();
  const h = mapHit(sp);
  drag = { kind: h ? h.kind : "mpan", k: h && h.k, sp, sp0: sp, start: mapRect && { ...mapRect }, moved: false };
  canvas.classList.add("dragging");
}
function mapMove(sp) {
  if (!drag) return;
  const dx = sp[0] - drag.sp[0], dy = sp[1] - drag.sp[1];
  if (!drag.moved && Math.hypot(dx, dy) < 3) return;
  drag.moved = true;
  if (drag.kind === "mpan") { const r = mpp(mapv.z); mapv.x -= dx * r; mapv.y += dy * r; drag.sp = sp; }
  else if (drag.kind === "mmove") {
    const a = sToM(...drag.sp0), b = sToM(...sp), s = drag.start;
    mapRect = { x0: s.x0 + b[0] - a[0], y0: s.y0 + b[1] - a[1], x1: s.x1 + b[0] - a[0], y1: s.y1 + b[1] - a[1] };
  } else if (drag.kind === "mcorner") {
    const opp = rectCorners(drag.start)[(drag.k + 2) % 4], p = sToM(...sp), min = 20 / cosAt(p[1]);
    const x0 = Math.min(opp[0], p[0]), x1 = Math.max(opp[0], p[0]), y0 = Math.min(opp[1], p[1]), y1 = Math.max(opp[1], p[1]);
    if (x1 - x0 > min && y1 - y0 > min) mapRect = { x0, y0, x1, y1 };
  }
  updateMapUI(); redraw();
}
function mapZoomAt(sp, z) {
  const before = sToM(sp[0], sp[1]);
  mapv.z = Math.max(3, Math.min(20, z));
  const after = sToM(sp[0], sp[1]);
  mapv.x += before[0] - after[0]; mapv.y += before[1] - after[1];
  updateMapUI(); redraw();
}

// ---- Recherche de lieu (tolérante : mots dans le désordre, accents et mots vides ignorés, fautes tolérées)
const STOP = new Set(["le", "la", "les", "l", "de", "du", "des", "d", "un", "une", "et", "en", "a", "au", "aux", "sur", "sous", "pour", "par"]);
const normTxt = (s) => String(s).normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase().replace(/[^a-z0-9]+/g, " ").trim();
const qWords = (s) => normTxt(s).split(" ").filter((w) => w && !STOP.has(w));
function lev(a, b) {
  let prev = Array.from({ length: b.length + 1 }, (_, j) => j);
  for (let i = 1; i <= a.length; i++) { const cur = [i]; for (let j = 1; j <= b.length; j++) cur[j] = Math.min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (a[i - 1] === b[j - 1] ? 0 : 1)); prev = cur; }
  return prev[b.length];
}
function wordHit(q, ws) {
  const tol = q.length >= 7 ? 2 : q.length >= 4 ? 1 : 0;
  return ws.some((w) => w.startsWith(q) || (tol > 0 && (lev(q, w) <= tol || lev(q, w.slice(0, q.length)) <= tol)));
}
const ADDR_KIND = { housenumber: ["Adresse", 19], street: ["Voie", 18], locality: ["Lieu-dit", 17], municipality: ["Commune", 15] };
function itemOf(f) {
  const p = f.properties || {}, [lon, lat] = f.geometry.coordinates;
  if (p._type === "parcel") return { label: `Parcelle ${p.section} ${+p.number}`, sub: [p.city, p.id].filter(Boolean).join(" · "), lon, lat, zoom: 19, kind: "parcel", api: 1 };
  if (p._type === "poi") {
    const cat = [].concat(p.category || []).join(", "), name = [].concat(p.toponym || p.name || "")[0], commune = /commune/.test(cat);
    return { label: name, sub: [cat, p.city, p.postcode && [].concat(p.postcode)[0]].filter(Boolean).join(" · "), lon, lat, zoom: commune ? 15 : 17, kind: commune ? "commune" : "poi", api: p.score || 0 };
  }
  const [kindLabel, zoom] = ADDR_KIND[p.type] || ["Adresse", 18];
  return { label: p.label, sub: [kindLabel, p.context].filter(Boolean).join(" · "), lon, lat, zoom, kind: p.type || "address", api: p.score || 0 };
}
async function geocode(params, signal) {
  const r = await fetch(GEO.geocode + "?" + new URLSearchParams(params), { signal });
  if (!r.ok) throw new Error("HTTP " + r.status);
  return ((await r.json()).features || []).filter((f) => f.geometry).map(itemOf);
}
let searchCtl = null, searchTimer = null, results = [], activeIdx = -1, pickOnResults = false;
async function runSearch(raw) {
  const q = raw.trim();
  if (searchCtl) searchCtl.abort();
  if (q.length < 2) { renderResults([], ""); return; }
  searchCtl = new AbortController();
  const signal = searchCtl.signal;
  try {
    // Coordonnées « lat, lon »
    const mc = q.match(/^(-?\d{1,3}(?:[.,]\d+)?)\s*[,; ]\s*(-?\d{1,3}(?:[.,]\d+)?)$/);
    if (mc) {
      let a = +mc[1].replace(",", "."), b = +mc[2].replace(",", ".");
      if (Math.abs(a) > 90) [a, b] = [b, a];
      return renderResults([{ label: `${a.toFixed(5)}, ${b.toFixed(5)}`, sub: "Coordonnées GPS", lat: a, lon: b, zoom: 18, kind: "coords", all: true, score: 2 }], q);
    }
    // Référence cadastrale : 06088000KY0116 ou « 06088 KY 116 »
    const compact = q.replace(/\s+/g, "").toUpperCase();
    let idu = /^\d{5}\d{3}[A-Z0-9]{2}\d{4}$/.test(compact) ? compact : null;
    const m = q.toUpperCase().match(/^(\d{5})\s+(?:(\d{3})\s+)?([A-Z0-9]{1,2})\s*(\d{1,4})$/);
    if (!idu && m) idu = m[1] + (m[2] || "000") + m[3].padStart(2, "0") + m[4].padStart(4, "0");
    if (idu) return renderResults((await geocode({ q: idu, index: "parcel", limit: 5 }, signal)).map((r) => ({ ...r, all: true, score: 2 })), q);

    const words = qWords(q), clean = words.join(" ");
    const variants = [q]; if (clean && clean !== normTxt(q)) variants.push(clean);
    const fetchAll = (vs) => Promise.all(vs.map((v) => geocode({ q: v, index: "address,poi", limit: 8, autocomplete: 1 }, signal).catch((e) => { if (e.name === "AbortError") throw e; return null; })));
    let lists = await fetchAll(variants);
    if (lists.every((l) => l === null)) return renderResults([], q, "Recherche indisponible : le service IGN ne répond pas depuis cette page.");
    let items = lists.flat().filter(Boolean);
    if (!items.length && words.length > 1) items = (await fetchAll(words.slice(0, 5).map((_, i) => words.filter((__, j) => j !== i).join(" ")))).flat().filter(Boolean);
    const seen = new Set();
    items = items.filter((r) => { const k = r.label + "|" + r.lon.toFixed(4) + r.lat.toFixed(4); if (seen.has(k)) return false; seen.add(k); return true; });
    for (const r of items) {
      const ws = qWords(r.label + " " + r.sub), hits = words.filter((w) => wordHit(w, ws)).length;
      r.all = hits === words.length; r.score = hits / Math.max(1, words.length) + 0.25 * r.api;
    }
    items.sort((a, b) => b.score - a.score);
    renderResults(items, q);
  } catch (e) {
    if (e.name !== "AbortError") renderResults([], q, "Recherche indisponible : le service IGN ne répond pas depuis cette page.");
  }
}
function renderResults(items, q, err) {
  const exact = items.filter((r) => r.all), approx = items.filter((r) => !r.all);
  results = [...exact, ...approx].slice(0, 8); activeIdx = results.length ? 0 : -1;
  const box = $("#results"); box.textContent = "";
  $("#q").setAttribute("aria-expanded", String(!!q));
  if (!q) { box.hidden = true; return; }
  box.hidden = false;
  if (err || !results.length) { const d = document.createElement("div"); d.className = "res-empty"; d.textContent = err || "Aucun lieu trouvé"; box.append(d); return; }
  results.forEach((r, i) => {
    if (!r.all && (i === 0 || results[i - 1].all)) { const h = document.createElement("div"); h.className = "res-head"; h.textContent = "Résultats approchants"; box.append(h); }
    const b = document.createElement("button"); b.className = "res"; b.type = "button"; b.setAttribute("role", "option"); b.id = "res-" + i;
    const t = document.createElement("b"); t.textContent = r.label; const s = document.createElement("small"); s.textContent = r.sub;
    b.append(t, s); b.addEventListener("click", () => pick(r)); box.append(b);
  });
  markActive();
  if (pickOnResults) { pickOnResults = false; if (results[0]) pick(results[0]); }
}
function markActive() { results.forEach((_, i) => { const b = $("#res-" + i); if (b) b.setAttribute("aria-selected", String(i === activeIdx)); }); }
function closeResults() { $("#results").hidden = true; $("#q").setAttribute("aria-expanded", "false"); }
function pick(r) {
  closeResults(); $("#q").value = r.label; $("#q-clear").hidden = false; $("#q").blur();
  [mapv.x, mapv.y] = toMerc(r.lon, r.lat); mapv.z = r.zoom;
  if (r.kind !== "commune" && r.kind !== "poi") {
    defaultRect(260, 200);
    // L'emprise et ses poignées doivent tenir à l'écran, téléphone compris.
    const fitZ = Math.log2(Math.min((W * 0.8) / (mapRect.x1 - mapRect.x0), (H * 0.6) / (mapRect.y1 - mapRect.y0)) * (2 * HALF) / 256);
    mapv.z = Math.min(r.zoom, fitZ);
  }
  updateMapUI(); redraw();
}
$("#q").addEventListener("input", (ev) => {
  $("#q-clear").hidden = !ev.target.value; clearTimeout(searchTimer);
  searchTimer = setTimeout(() => runSearch(ev.target.value), 220);
});
$("#q").addEventListener("keydown", (ev) => {
  if (ev.key === "ArrowDown" || ev.key === "ArrowUp") { ev.preventDefault(); if (!results.length) return; activeIdx = (activeIdx + (ev.key === "ArrowDown" ? 1 : -1) + results.length) % results.length; markActive(); }
  else if (ev.key === "Enter") { ev.preventDefault(); if (results[activeIdx] || results[0]) pick(results[activeIdx] || results[0]); else { pickOnResults = true; clearTimeout(searchTimer); runSearch(ev.target.value); } }
  else if (ev.key === "Escape") { ev.preventDefault(); ev.stopPropagation(); if (ev.target.value) { ev.target.value = ""; $("#q-clear").hidden = true; renderResults([], ""); } else ev.target.blur(); }
});
$("#q-clear").addEventListener("click", () => { $("#q").value = ""; $("#q-clear").hidden = true; renderResults([], ""); $("#q").focus(); });

// ---- Import de l'emprise : photo découpée au rectangle + vecteurs cadastraux en repère local
async function pool(items, n, fn) {
  let i = 0;
  const run = async () => { while (i < items.length) await fn(items[i++]); };
  await Promise.all(Array.from({ length: Math.min(n, items.length) }, run));
}
function progress(show, pct, text) {
  $("#progress").hidden = !show; if (!show) return;
  $("#pg-bar").style.width = Math.round(pct * 100) + "%"; $("#pg-text").textContent = text;
}
async function wfs(type, bbox, onCount) {
  const out = [];
  for (let start = 0, page = 0; page < 25; page++) {
    const r = await fetch(GEO.wfs + "?" + new URLSearchParams({ SERVICE: "WFS", VERSION: "2.0.0", REQUEST: "GetFeature", TYPENAMES: "CADASTRALPARCELS.PARCELLAIRE_EXPRESS:" + type, OUTPUTFORMAT: "application/json", BBOX: bbox, COUNT: "1000", STARTINDEX: String(start) }));
    if (!r.ok) throw new Error(type + " : HTTP " + r.status);
    const j = await r.json(), f = j.features || [];
    out.push(...f); onCount(out.length);
    start += f.length;
    if (f.length < 1000 || (j.numberMatched && start >= j.numberMatched)) break;
  }
  return out;
}
function ringLabelPoint(ring) {
  let a = 0, cx = 0, cy = 0;
  for (let i = 0; i < ring.length; i++) { const p = ring[i], q = ring[(i + 1) % ring.length], f = p[0] * q[1] - q[0] * p[1]; a += f; cx += (p[0] + q[0]) * f; cy += (p[1] + q[1]) * f; }
  if (Math.abs(a) > 1e-9) { const c = [cx / (3 * a), cy / (3 * a)]; if (pointInPoly(c, ring)) return c; }
  // Polygone concave : meilleur point d'une grille, le plus éloigné des bords.
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
  for (const [x, y] of ring) { x0 = Math.min(x0, x); x1 = Math.max(x1, x); y0 = Math.min(y0, y); y1 = Math.max(y1, y); }
  let best = ring[0], bd = -1;
  for (let i = 1; i < 10; i++) for (let j = 1; j < 10; j++) {
    const p = [x0 + (x1 - x0) * i / 10, y0 + (y1 - y0) * j / 10]; if (!pointInPoly(p, ring)) continue;
    let d = Infinity; for (let k = 0; k < ring.length; k++) d = Math.min(d, distSeg(p, ring[k], ring[(k + 1) % ring.length])[0]);
    if (d > bd) { bd = d; best = p; }
  }
  return best;
}
function featureOf(f, loc, keep) {
  const g = f.geometry; if (!g) return null;
  const raw = g.type === "Polygon" ? [g.coordinates] : g.type === "MultiPolygon" ? g.coordinates : [];
  const polys = raw.map((poly) => poly.map((ring) => { const r = ring.map(loc); if (r.length > 1 && r[0][0] === r[r.length - 1][0] && r[0][1] === r[r.length - 1][1]) r.pop(); return r; }).filter((r) => r.length >= 3)).filter((p) => p.length);
  if (!polys.length) return null;
  const bb = [Infinity, Infinity, -Infinity, -Infinity];
  for (const p of polys) for (const [x, y] of p[0]) { bb[0] = Math.min(bb[0], x); bb[1] = Math.min(bb[1], y); bb[2] = Math.max(bb[2], x); bb[3] = Math.max(bb[3], y); }
  const props = {}; for (const k of keep) if (f.properties && f.properties[k] !== undefined) props[k] = f.properties[k];
  const main = polys.reduce((m, p) => (Math.abs(polyArea(p[0])) > Math.abs(polyArea(m[0])) ? p : m), polys[0]);
  return { polys, bb, p: props, lp: keep.length ? ringLabelPoint(main[0]) : null };
}
async function importArea() {
  if (!mapRect) return;
  const r = { ...mapRect };
  const [lonW, latS] = fromMerc(r.x0, r.y0), [lonE, latN] = fromMerc(r.x1, r.y1);
  const O = [(r.x0 + r.x1) / 2, (r.y0 + r.y1) / 2], k = cosAt(O[1]);
  if (Math.max(r.x1 - r.x0, r.y1 - r.y0) * k > 3000) { toast("Emprise trop grande : 3 km de côté au maximum"); return; }
  $("#m-ok").disabled = true;
  const count = { tiles: 0, total: 0, parcelles: 0, bati: 0, feuilles: 0, communes: 0 };
  const status = () => progress(true, count.total ? count.tiles / count.total : 0, `Photo aérienne ${count.tiles} / ${count.total} · parcelles ${count.parcelles} · bâti ${count.bati}`);

  // Photo : tuiles au niveau le plus fin qui tient dans 4 096 pixels, recadrées au rectangle.
  const z = photoZoom(r), n = 2 ** z, px = (x) => (x + HALF) / (2 * HALF) * 256 * n, py = (y) => (HALF - y) / (2 * HALF) * 256 * n;
  const P0 = px(r.x0), P1 = px(r.x1), Q0 = py(r.y1), Q1 = py(r.y0);
  const cv = document.createElement("canvas"); cv.width = Math.max(1, Math.round(P1 - P0)); cv.height = Math.max(1, Math.round(Q1 - Q0));
  const g = cv.getContext("2d");
  const list = [];
  for (let ty = Math.floor(Q0 / 256); ty <= Math.floor((Q1 - 1e-6) / 256); ty++) for (let tx = Math.floor(P0 / 256); tx <= Math.floor((P1 - 1e-6) / 256); tx++) list.push([tx, ty]);
  count.total = list.length; status();
  let tainted = false, failed = 0;
  const photo = pool(list, 8, ([tx, ty]) => new Promise((res) => {
    loadImage(tileUrl(ORTHO, z, tx, ty), (im, cors) => { g.drawImage(im, tx * 256 - P0, ty * 256 - Q0); if (!cors) tainted = true; count.tiles++; status(); res(); }, () => { failed++; count.tiles++; status(); res(); });
  }));

  // Vecteurs : Parcellaire Express (PCI), même emprise.
  const bbox = `${latS},${lonW},${latN},${lonE},urn:ogc:def:crs:EPSG::4326`;
  const loc = ([lon, lat]) => { const [x, y] = toMerc(lon, lat); return [+((x - O[0]) * k).toFixed(2), +((y - O[1]) * k).toFixed(2)]; };
  const KEEP = { parcelles: ["idu", "section", "numero", "contenance", "nom_com", "code_insee"], bati: ["type"], feuilles: ["section", "feuille", "nom_com"], communes: [] };
  const vectors = Promise.all(Object.entries(CAD_TYPES).map(([key, type]) =>
    wfs(type, bbox, (c) => { count[key] = c; status(); })
      .then((fs) => [key, fs.map((f) => featureOf(f, loc, KEEP[key])).filter(Boolean)])
      .catch(() => [key, null])));
  const [, layersList] = await Promise.all([photo, vectors]);
  progress(false);
  $("#m-ok").disabled = false;
  const layers = Object.fromEntries(layersList.map(([key, v]) => [key, v || []]));
  const vecFailed = layersList.filter(([, v]) => v === null).length;
  const failedKeys = layersList.filter(([, v]) => v === null).map(([key]) => key);
  if (failed === list.length && vecFailed === layersList.length) { toast("Import impossible : le service IGN ne répond pas depuis cette page"); $("#mapmsg").hidden = false; return; }

  const prev = basemap;
  if (prev && exampleName === "mine" && site.parcel.length) reframeSite(prev, { O, k });
  else if (exampleName !== "mine" || !prev) { if (exampleName === "mine" && site.parcel.length) mineSite = null; site = { parcel: [], setbacks: [], exclusions: [], buildings: [], landscapes: [], basins: [], entries: [] }; exampleName = "mine"; }
  basemap = {
    O, k, z, mrect: r, tainted, img: failed === list.length ? null : cv,
    rect: { x0: (r.x0 - O[0]) * k, y0: (r.y0 - O[1]) * k, x1: (r.x1 - O[0]) * k, y1: (r.y1 - O[1]) * k },
    layers, failed: failedKeys, date: new Date().toLocaleDateString("fr-FR"),
  };
  progress(true, 1, "Calcul de l'occupation du sol…");
  await new Promise((res) => requestAnimationFrame(() => setTimeout(res, 0)));
  computeOccupation();
  progress(false);
  saveBasemap();
  exitMap(); syncExample(); updateLayersUI(); lastAngle = null; setSel(null); solve(false); commit(); fit();
  const msgs = [];
  if (failed) msgs.push(`${failed} tuile${failed > 1 ? "s" : ""} photo manquante${failed > 1 ? "s" : ""}`);
  if (vecFailed) msgs.push("certaines couches cadastrales n'ont pas pu être chargées");
  toast(msgs.length ? msgs.join(" · ") : site.parcel.length ? "Fond mis à jour" : "Touchez une parcelle cadastrale pour l'étudier");
}
// Recale le site d'un ancien repère local vers le nouveau.
function reframeSite(a, b) {
  const f = ([x, y]) => { const X = x / a.k + a.O[0], Y = y / a.k + a.O[1]; return [+((X - b.O[0]) * b.k).toFixed(3), +((Y - b.O[1]) * b.k).toFixed(3)]; };
  site.parcel = site.parcel.map(f);
  site.exclusions = site.exclusions.map((e) => e.map((p) => [...f(p), ...p.slice(2)]));
  site.landscapes = site.landscapes.map((e) => e.map((p) => [...f(p), ...p.slice(2)]));
  site.basins = (site.basins || []).map((e) => e.map((p) => [...f(p), ...p.slice(2)]));
  for (const o of site.buildings) { [o.cx, o.cy] = f([o.cx, o.cy]); o.w *= b.k / a.k; o.h *= b.k / a.k; }
}

// ---- Persistance du fond (IndexedDB : la photo dépasse la taille admise par localStorage)
const IDB = {
  db: null,
  open() { return this.db ? Promise.resolve(this.db) : new Promise((res, rej) => { const q = indexedDB.open("trame", 1); q.onupgradeneeded = () => q.result.createObjectStore("kv"); q.onsuccess = () => res((this.db = q.result)); q.onerror = () => rej(q.error); }); },
  async run(mode, fn) { const db = await this.open(); return new Promise((res, rej) => { const tx = db.transaction("kv", mode), q = fn(tx.objectStore("kv")); tx.oncomplete = () => res(q && q.result); tx.onerror = () => rej(tx.error); }); },
  put(k, v) { return this.run("readwrite", (s) => s.put(v, k)); },
  get(k) { return this.run("readonly", (s) => s.get(k)); },
  del(k) { return this.run("readwrite", (s) => s.delete(k)); },
};
async function saveBasemap() {
  try {
    if (!basemap) { await IDB.del("base"); return; }
    let blob = null;
    if (basemap.img && !basemap.tainted && basemap.img.toBlob) blob = await new Promise((res) => basemap.img.toBlob(res, "image/jpeg", 0.88));
    const { img, ...meta } = basemap;
    await IDB.put("base", { meta, blob });
    if (basemap.img && !blob) toast("Photo affichée mais non conservée : le serveur n'autorise pas sa copie");
  } catch (e) { toast("Fond non enregistré : stockage du navigateur indisponible"); }
}
async function loadBasemap() {
  try {
    const v = await IDB.get("base"); if (!v || !v.meta) return;
    let img = null;
    if (v.blob) img = window.createImageBitmap ? await createImageBitmap(v.blob) : await new Promise((res) => { const im = new Image(); im.onload = () => res(im); im.onerror = () => res(null); im.src = URL.createObjectURL(v.blob); });
    basemap = { ...v.meta, img };
  } catch (e) { basemap = null; }
}
function layerVis() { try { return { ...LAYER_DEFAULTS, ...JSON.parse(localStorage.getItem("trame.layers") || "{}") }; } catch (e) { return { ...LAYER_DEFAULTS }; } }
let VIS = layerVis(), PHOTO_OPACITY = VIS.opacity || 1;
function saveVis() { try { localStorage.setItem("trame.layers", JSON.stringify({ ...VIS, opacity: PHOTO_OPACITY })); } catch (e) { /* stockage indisponible */ } }
const baseShown = () => !!basemap && exampleName === "mine";

// ---- Occupation du sol : voirie = emprise − parcelles − bâti ; le reste de l'emprise est de la végétation.
function computeOccupation() {
  if (!basemap || !Engine.ex) return;
  const L = basemap.layers, r = basemap.rect;
  if ((basemap.failed || []).includes("parcelles") || (basemap.failed || []).includes("bati")) { basemap.occ = null; return; }
  const out = Engine.call("occupation_json", { rect: [r.x0, r.y0, r.x1, r.y1], parcels: L.parcelles.flatMap((f) => f.polys), bati: L.bati.flatMap((f) => f.polys) });
  if (out.error) { basemap.occ = null; return; }
  const feat = (poly) => {
    const bb = [Infinity, Infinity, -Infinity, -Infinity];
    for (const [x, y] of poly[0]) { bb[0] = Math.min(bb[0], x); bb[1] = Math.min(bb[1], y); bb[2] = Math.max(bb[2], x); bb[3] = Math.max(bb[3], y); }
    return { polys: [poly], bb, p: {} };
  };
  basemap.occ = { voirie: out.voirie.map(feat), vegetation: out.vegetation.map(feat), areas: { voirie: out.voirie_area, vegetation: out.vegetation_area, bati: out.bati_area } };
}
let vegPat = null;
function vegPattern() {
  if (vegPat) return vegPat;
  const c = document.createElement("canvas"); c.width = c.height = 9 * DPR;
  const g = c.getContext("2d"); g.strokeStyle = C.veg; g.lineWidth = 1.1 * DPR;
  g.beginPath(); g.moveTo(0, 0); g.lineTo(c.width, c.height); g.moveTo(-c.width / 2, c.height / 2); g.lineTo(c.width / 2, c.height * 1.5); g.moveTo(c.width / 2, -c.height / 2); g.lineTo(c.width * 1.5, c.height / 2); g.stroke();
  vegPat = ctx.createPattern(c, "repeat"); return vegPat;
}

// ---- Rendu du fond dans le plan
function drawBasemap() {
  if (!baseShown()) return;
  const b = basemap, r = b.rect, L = b.layers;
  const tl = toS([r.x0, r.y1]), br = toS([r.x1, r.y0]);
  if (VIS.photo && b.img) { ctx.globalAlpha = PHOTO_OPACITY; ctx.drawImage(b.img, tl[0], tl[1], br[0] - tl[0], br[1] - tl[1]); ctx.globalAlpha = 1; }
  ctx.save(); ctx.beginPath(); ctx.rect(tl[0], tl[1], br[0] - tl[0], br[1] - tl[1]); ctx.clip();
  const [vx0, vy1] = toW(0, 0), [vx1, vy0] = toW(W, H);
  const inView = (bb) => !(bb[2] < vx0 || bb[0] > vx1 || bb[3] < vy0 || bb[1] > vy1);
  const path = (list) => { ctx.beginPath(); for (const f of list) if (inView(f.bb)) for (const poly of f.polys) for (const ring of poly) pathPoly(ring); };
  // Couche Occupation du sol
  if (b.occ && VIS.ovoirie && b.occ.voirie.length) { path(b.occ.voirie); ctx.fillStyle = C["occ-voirie"]; ctx.fill("evenodd"); }
  if (b.occ && VIS.vegetation && b.occ.vegetation.length) {
    path(b.occ.vegetation); ctx.fillStyle = C["veg-tint"]; ctx.fill("evenodd"); ctx.fillStyle = vegPattern(); ctx.fill("evenodd");
  }
  if (VIS.bati && L.bati.length) {
    path(L.bati); ctx.fillStyle = C["cad-bati"]; ctx.fill("evenodd"); ctx.strokeStyle = C["cad-bati-edge"]; ctx.lineWidth = 1; ctx.stroke();
  }
  if (VIS.parcelles && L.parcelles.length) {
    path(L.parcelles); ctx.strokeStyle = C["cad-halo"]; ctx.lineWidth = 3; ctx.stroke(); ctx.strokeStyle = C.cad; ctx.lineWidth = 1.2; ctx.stroke();
  }
  if (VIS.feuilles && L.feuilles.length) {
    path(L.feuilles); ctx.setLineDash([10, 5]); ctx.strokeStyle = C["cad-halo"]; ctx.lineWidth = 4; ctx.stroke(); ctx.strokeStyle = C["cad-sheet"]; ctx.lineWidth = 2; ctx.stroke(); ctx.setLineDash([]);
  }
  if (VIS.communes && L.communes.length) {
    path(L.communes); ctx.setLineDash([14, 4, 3, 4]); ctx.strokeStyle = C["cad-halo"]; ctx.lineWidth = 5; ctx.stroke(); ctx.strokeStyle = C["cad-commune"]; ctx.lineWidth = 2.5; ctx.stroke(); ctx.setLineDash([]);
  }
  const font = getComputedStyle(document.body).fontFamily;
  const label = (text, p, size, color) => { const s = toS(p); ctx.font = `600 ${size}px ${font}`; ctx.lineWidth = 3; ctx.strokeStyle = C["cad-halo"]; ctx.strokeText(text, s[0], s[1]); ctx.fillStyle = color; ctx.fillText(text, s[0], s[1]); };
  ctx.textAlign = "center"; ctx.textBaseline = "middle"; ctx.lineJoin = "round";
  if (VIS.numeros && VIS.parcelles) for (const f of L.parcelles) {
    if (!inView(f.bb) || (f.bb[2] - f.bb[0]) * view.s < 26 || (f.bb[3] - f.bb[1]) * view.s < 14 || !f.p.numero) continue;
    label(String(+f.p.numero), f.lp, 11, C["cad-label"]);
  }
  if (VIS.feuilles) for (const f of L.feuilles) if (inView(f.bb) && f.p.section && (f.bb[2] - f.bb[0]) * view.s > 60) label(f.p.section, f.lp, 15, C["cad-sheet"]);
  ctx.restore();
}
function drawCadSel() {
  if (!sel || sel.type !== "cadastre" || !baseShown()) return;
  const f = basemap.layers.parcelles[sel.i]; if (!f) return;
  ctx.beginPath(); for (const poly of f.polys) for (const ring of poly) pathPoly(ring);
  ctx.fillStyle = C.accent; ctx.globalAlpha = 0.18; ctx.fill("evenodd"); ctx.globalAlpha = 1;
  ctx.strokeStyle = C.accent; ctx.lineWidth = 2.5; ctx.stroke();
}
function cadParcelAt(w) {
  if (!baseShown() || !VIS.parcelles) return -1;
  const L = basemap.layers.parcelles;
  for (let i = 0; i < L.length; i++) {
    const f = L[i]; if (w[0] < f.bb[0] || w[0] > f.bb[2] || w[1] < f.bb[1] || w[1] > f.bb[3]) continue;
    let inside = false; for (const poly of f.polys) for (const ring of poly) if (pointInPoly(w, ring)) inside = !inside;
    if (inside) return i;
  }
  return -1;
}

// ---- Parcelle cadastrale → parcelle d'étude
function simplifyRing(ring, tol) {
  let r = ring.slice(), changed = true;
  while (changed && r.length > 3) {
    changed = false;
    for (let i = 0; i < r.length && r.length > 3; i++) {
      const a = r[(i + r.length - 1) % r.length], b = r[i], c = r[(i + 1) % r.length];
      if (distSeg(b, a, c)[0] < tol) { r.splice(i, 1); changed = true; i--; }
    }
  }
  return r.map(([x, y]) => [+x.toFixed(2), +y.toFixed(2)]);
}
function cadName(f) { return `${f.p.section || ""} ${f.p.numero ? +f.p.numero : ""}`.trim(); }
function studyParcel(i, merge) {
  const f = basemap.layers.parcelles[i]; if (!f) return;
  let ring, split = false;
  if (merge && site.parcel.length >= 3) {
    const out = Engine.union([outline(), ...f.polys.map((p) => p[0])]);
    if (!out.rings || !out.rings.length) { toast("Réunion impossible"); return; }
    ring = out.rings[0]; split = out.rings.length > 1;
  } else ring = f.polys.reduce((m, p) => (Math.abs(polyArea(p[0])) > Math.abs(polyArea(m)) ? p[0] : m), f.polys[0][0]);
  ring = simplifyRing(ring, 0.12);
  let li = 0, lb = 0;
  ring.forEach((a, j) => { const b = ring[(j + 1) % ring.length], L = Math.hypot(b[0] - a[0], b[1] - a[1]); if (L > lb) { lb = L; li = j; } });
  const keep = merge ? site : { exclusions: [], buildings: [], landscapes: [], basins: [] };
  site = { ...keep, arcs: [], parcel: ring, setbacks: ring.map(() => 3), entries: [{ edge: li, t: ENTRY_T }], cadastre: [...(merge ? site.cadastre || [] : []), f.p.idu].filter(Boolean) };
  exampleName = "mine"; syncExample(); lastAngle = null; setSel(null); solve(false); commit();
  if (split) toast("Parcelles non contiguës : seule la plus grande emprise est conservée");
  else toast(merge ? `Parcelle ${cadName(f)} ajoutée · ${fmtInt(Math.abs(polyArea(ring)))} m²` : `Parcelle ${cadName(f)} à l'étude · reculs de 3 m, accès sur la plus longue limite`);
}

// ---- Panneau des couches : chaque interrupteur est un calque
function updateLayersUI() {
  $("#lay-base").hidden = !baseShown();
  $("#lay-gir").hidden = !gir && !places.res;
  document.querySelectorAll("[data-lay]").forEach((c) => { c.checked = !!VIS[c.dataset.lay]; });
  if (!basemap) return;
  $("#lay-op").value = Math.round(PHOTO_OPACITY * 100); $("#lay-op").disabled = !VIS.photo;
  $("#lay-date").textContent = "importé le " + basemap.date + (basemap.img ? "" : " · photo non conservée");
}
// Un objet dont le calque est masqué n'est plus sélectionnable.
const SEL_LAYER = { cadastre: "parcelles", building: "pbati", landscape: "pbati", basin: "pbati", exclusion: "servitudes" };
document.querySelectorAll("[data-lay]").forEach((c) => c.addEventListener("change", () => {
  VIS[c.dataset.lay] = c.checked;
  if (sel && SEL_LAYER[sel.type] === c.dataset.lay && !c.checked) setSel(null);
  saveVis(); updateLayersUI(); redraw();
}));
$("#lay-op").addEventListener("input", (ev) => { PHOTO_OPACITY = +ev.target.value / 100; redraw(); });
$("#lay-op").addEventListener("change", saveVis);
$("#lay-reframe").addEventListener("click", enterMap);
$("#lay-remove").addEventListener("click", () => { basemap = null; saveBasemap(); setSel(null); updateLayersUI(); redraw(); toast("Fond de carte retiré"); });
$("#layers-head").addEventListener("click", () => setPanel("layers", $("#layers-head").getAttribute("aria-expanded") !== "true"));
$("#sel-study").addEventListener("click", () => sel && sel.type === "cadastre" && studyParcel(sel.i, false));
$("#sel-merge").addEventListener("click", () => sel && sel.type === "cadastre" && studyParcel(sel.i, true));

// ---- Barre d'outils carte
$("#n-map").addEventListener("click", () => { closeMenus(); enterMap(); });
$("#m-back").addEventListener("click", exitMap);
$("#m-rect").addEventListener("click", () => {
  const k = cosAt(mapv.y), r = mpp(mapv.z) * k;
  defaultRect(Math.max(80, Math.min(1500, W * 0.55 * r)), Math.max(60, Math.min(1500, H * 0.45 * r)));
  updateMapUI(); redraw();
});
$("#m-cad").addEventListener("click", () => { mapCad = !mapCad; updateMapUI(); redraw(); });
$("#m-ok").addEventListener("click", importArea);

