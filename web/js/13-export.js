// ============================================================================
// Export DXF (moteur Rust) : RGF93 / Lambert-93 ou CC, calques AutoCAD.
// ============================================================================
// Calque de l'interface → calques AutoCAD correspondants.
const DXF_LAYERS = {
  places: ["PRJ_PLACES", "PRJ_PLACES_PMR", "PRJ_PLACES_IRVE", "PRJ_PLACES_IRVE_EQ"], pvoirie: ["PRJ_VOIRIE", "PRJ_RAMPES"], girations: ["PRJ_GIRATIONS"], noyaux: ["PRJ_NOYAUX"], structure: ["STR_DALLE", "STR_POUTRES", "STR_POTEAUX", "STR_AXES", "STR_REPERES"], sens: ["PRJ_SIGNALISATION"], ilots: ["PRJ_ILOTS", "PRJ_NOUES"],
  vehicules: ["HAB_VEHICULES"], arbres: ["HAB_ARBRES"],
  pbati: ["PRJ_BATIMENTS", "PRJ_ESPACES_VERTS", "PRJ_BASSIN_TALUS_HAUT", "PRJ_BASSIN_TALUS_BAS"], servitudes: ["PRJ_SERVITUDES"], recul: ["PRJ_RECUL"],
  ovoirie: ["OCC_VOIRIE"], bati: ["OCC_BATI"], vegetation: ["OCC_VEGETATION"],
  parcelles: ["CAD_PARCELLES"], numeros: ["CAD_NUMEROS"], feuilles: ["CAD_FEUILLES"], communes: ["CAD_COMMUNES"],
  girroues: ["GIR_TRAJECTOIRE", "GIR_ROUES"], girenv: ["GIR_CARROSSERIE", "GIR_VEHICULE"], girmarge: ["GIR_MARGE"], girconf: ["GIR_CONFLITS"],
};
let XCRS = "L93";
try { XCRS = localStorage.getItem("trame.crs") === "CC" ? "CC" : "L93"; } catch (e) { /* stockage indisponible */ }
function ccZone() { const lat = fromMerc(basemap.O[0], basemap.O[1])[1]; return Math.max(42, Math.min(50, Math.round(lat))); }
function updateExportUI() {
  const geo = baseShown();
  $("#x-geo").hidden = !geo;
  if (geo) {
    const z = ccZone();
    $("#x-cc").textContent = "CC" + z;
    $("#x-l93").setAttribute("aria-pressed", String(XCRS === "L93")); $("#x-cc").setAttribute("aria-pressed", String(XCRS === "CC"));
    $("#x-epsg").textContent = "EPSG:" + (XCRS === "CC" ? 3900 + z : 2154);
    $("#x-note").textContent = "RGF93 · AutoCAD 2010 et suivants · unités : mètres. Les calques masqués sont exportés éteints.";
  } else {
    $("#x-note").textContent = "Plan non géoréférencé : coordonnées locales en mètres. Importez un fond de carte pour exporter en Lambert-93. Les calques masqués sont exportés éteints.";
  }
  $("#x-go").disabled = site.parcel.length < 3 && !geo; $("#x-xls").disabled = !(layout && layout.metrics);
  $("#x-bim").hidden = !(layout && layout.ouvrage);
}
$("#x-l93").addEventListener("click", () => { XCRS = "L93"; try { localStorage.setItem("trame.crs", XCRS); } catch (e) { /* stockage indisponible */ } updateExportUI(); });
$("#x-cc").addEventListener("click", () => { XCRS = "CC"; try { localStorage.setItem("trame.crs", XCRS); } catch (e) { /* stockage indisponible */ } updateExportUI(); });
$("#x-go").addEventListener("click", exportDxf);
$("#x-ifc").addEventListener("click", exportIfc);

function slug(t) { return normTxt(t || "plan").replace(/ /g, "-") || "plan"; }
async function exportDxf() {
  if (!Engine.ex) return;
  const geo = baseShown(), crs = geo ? (XCRS === "CC" ? "CC" + ccZone() : "L93") : "LOCAL";
  const off = Object.entries(DXF_LAYERS).filter(([k]) => !VIS[k] || (k === "numeros" && !VIS.parcelles)).flatMap(([, v]) => v);
  const dr = dressing();
  const dress = {
    vehicules: dr.cars.map((c) => ({ b: c.v.bloc, p: [+c.x.toFixed(3), +c.y.toFixed(3)], a: +c.a.toFixed(2) })),
    arbres: dr.trees.map((t) => ({ b: t.sp.bloc, p: [t.x, t.y], a: +t.a.toFixed(1), s: +t.s.toFixed(3) })),
  };
  // Plan complet, places au sortir des virages contrôlées, comme le plan affiché une fois mis à jour.
  const input = { site: engineInput(false, true), georef: null, crs, off, cadastre: {}, date: "", dress };
  if (gir) input.giration = girEpure(site.gir);
  let commune = "";
  if (geo) {
    const L = basemap.layers, r = basemap.rect, occ = basemap.occ;
    const f2 = (f, label) => ({ polys: f.polys, label: label || null, lp: f.lp || null });
    input.georef = { o: basemap.O, k: basemap.k };
    input.date = basemap.date;
    input.cadastre = {
      emprise: [r.x0, r.y0, r.x1, r.y1],
      parcelles: L.parcelles.map((f) => f2(f, f.p.numero ? String(+f.p.numero) : null)),
      feuilles: L.feuilles.map((f) => f2(f, f.p.section)),
      communes: L.communes.map((f) => f2(f)),
      bati: L.bati.map((f) => f2(f)),
      voirie: occ ? occ.voirie.flatMap((f) => f.polys) : [],
      vegetation: occ ? occ.vegetation.flatMap((f) => f.polys) : [],
    };
    commune = (L.parcelles.find((f) => f.p.nom_com) || { p: {} }).p.nom_com || "";
  }
  const out = Engine.call("export_dxf_json", input);
  if (out.error) { toast("Export impossible : " + out.error); return; }
  const day = new Date().toISOString().slice(0, 10);
  const name = `parkimo-${slug(commune || (exampleName === "mine" ? "mon site" : exampleName))}-${crs.toLowerCase()}-${day}.dxf`;
  closeMenus();
  await saveFile(name, out.dxf, `${fmtInt(out.entities)} entités · ${out.crs}${out.epsg ? " (EPSG:" + out.epsg + ")" : ""}`, "application/dxf", "DXF");
}

// Maquette IFC4 du parc en ouvrage : graine des identifiants propre au projet (date de création).
async function exportIfc() {
  if (!Engine.ex || !(layout && layout.ouvrage)) return;
  const geo = baseShown(), crs = geo ? (XCRS === "CC" ? "CC" + ccZone() : "L93") : "LOCAL";
  const input = { site: engineInput(false), georef: geo ? { o: basemap.O, k: basemap.k } : null, crs, date: new Date().toISOString().slice(0, 19), projet: "parkimo-" + meta.created };
  const out = Engine.call("export_ifc_json", input);
  if (out.error || out.erreur) { toast("Export impossible : " + (out.error || out.erreur)); return; }
  const commune = geo ? ((basemap.layers.parcelles.find((f) => f.p.nom_com) || { p: {} }).p.nom_com || "") : "";
  const name = `parkimo-${slug(commune || (exampleName === "mine" ? "mon site" : exampleName))}-silo-${new Date().toISOString().slice(0, 10)}.ifc`;
  closeMenus();
  await saveFile(name, out.ifc, `${fmtInt(out.elements)} éléments · ${out.crs}${out.epsg ? " (EPSG:" + out.epsg + ")" : ""}`, "application/x-step", "IFC");
}

// Export tableur : classeur XML Spreadsheet 2003 (.xls), lu par Excel, LibreOffice et Numbers, sans bibliothèque.
const xe = (t) => String(t).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
const frDate = (t) => new Date(t).toLocaleString("fr-FR", { dateStyle: "long", timeStyle: "short" });
function projectInfo() {
  const info = { commune: "", refs: (site.cadastre || []).join(", "), lat: null, lon: null, lieu: "" };
  if (baseShown() && site.parcel.length >= 3) {
    const L = basemap.layers.parcelles, ids = site.cadastre || [];
    const f = L.find((q) => ids.includes(q.p.idu) && q.p.nom_com) || L.find((q) => q.p.nom_com);
    info.commune = f ? f.p.nom_com : "";
    const c = site.parcel.reduce((a, q) => [a[0] + q[0] / site.parcel.length, a[1] + q[1] / site.parcel.length], [0, 0]);
    [info.lon, info.lat] = fromMerc(basemap.O[0] + c[0] / basemap.k, basemap.O[1] + c[1] / basemap.k);
  }
  return info;
}
function buildXls() {
  const m = layout.metrics, n = m.stalls, pa = m.parcel_area, I = projectInfo();
  const bat = site.buildings.reduce((a, b) => a + Math.abs(polyArea(rectPoly(b))), 0);
  let bi = BAREME.findIndex((r) => n <= r.b); if (bi < 0) bi = BAREME.length - 1;
  const br = BAREME[bi], ord = (x) => [Math.min(x[0], x[1]), Math.max(x[0], x[1])], T = ord(br.t), E = ord(br.e);
  const rows = [];  // [style, label, value, unité / commentaire]
  const sec = (t) => rows.push(["S", t]);
  const row = (l, v, u, st) => rows.push([st || (typeof v === "number" ? "N0" : "T"), l, v, u || ""]);
  const pct = (a, b) => (b ? a / b : "");
  sec("Projet");
  row("Nom", exampleName === "mine" ? "Mon site" : exampleName);
  row("Date de création", frDate(meta.created)); row("Dernière modification", frDate(meta.modified)); row("Export", frDate(Date.now()));
  sec("Emplacement");
  row("Commune", I.commune || "Non géoréférencé");
  if (I.refs) row("Références cadastrales", I.refs);
  if (I.lat !== null) { row("Latitude", I.lat, "° (WGS84)", "N5"); row("Longitude", I.lon, "° (WGS84)", "N5"); row("Carte", `https://www.openstreetmap.org/?mlat=${I.lat.toFixed(6)}&mlon=${I.lon.toFixed(6)}#map=18/${I.lat.toFixed(6)}/${I.lon.toFixed(6)}`); }
  sec("Places de stationnement");
  row("Total des places", n, "places"); row("Places standard", m.standard, "places");
  row("Places PMR", m.pmr, `places · exigées : ${m.pmr_required}`); row("Conformité PMR", m.pmr >= m.pmr_required ? "conforme" : "non conforme");
  row("Places IRVE pré-équipées", m.irve, `places · exigées : ${m.irve_required}`); row("Conformité IRVE pré-équipement", m.irve >= m.irve_required ? "conforme" : "non conforme");
  row("Places IRVE équipées (bornes)", m.irve_equipped, `places · exigées : ${m.irve_equipped_required}`); row("Conformité IRVE bornes", m.irve_equipped >= m.irve_equipped_required ? "conforme" : "non conforme");
  row("Places en bordure", m.perimeter_stalls, "places"); row("Places au cœur", m.interior_stalls, "places");
  row("Part en bordure", pct(m.perimeter_stalls, n), "%", "P"); row("Part PMR", pct(m.pmr, n), "%", "P"); row("Part IRVE pré-équipées", pct(m.irve, n), "%", "P"); row("Part IRVE bornes", pct(m.irve_equipped, n), "%", "P");
  row("Arbres plantés", m.trees, "arbres"); row("Arbres pour 10 places", n ? 10 * m.trees / n : "", "arbres", "N1");
  row("Angle des rangées", layout.angle, params.angle === null ? "° (optimisé)" : "° (imposé)", "N1");
  sec("Surfaces et ratios");
  row("Surface de la parcelle", pa, "m²"); row("Surface constructible (après reculs)", m.buildable_area, "m²");
  row("Surface de bâtiments", bat, "m²"); row("Surface enrobée", m.paved_area, "m²"); row("Surface d'espaces verts", m.green_area, "m²");
  row("Part d'espaces verts", pct(m.green_area, pa), "% de la parcelle", "P"); row("Part enrobée", pct(m.paved_area, pa), "% de la parcelle", "P");
  row("Enrobé par place", n ? m.m2_paved_per_stall : "", "m² / place", "N1"); row("Parcelle par place", n ? m.m2_parcel_per_stall : "", "m² / place", "N1");
  row("Densité", n && pa ? n / (pa / 10000) : "", "places / ha", "N1"); row("Constructible par place", n ? m.buildable_area / n : "", "m² / place", "N1");
  sec("Hypothèses de calcul");
  for (const k of PKEYS) { const lab = $(`label[for="p-${k}"]`), u = $("#p-" + k).parentElement.textContent.trim(); if (lab) row(lab.textContent, params[k], u, "N2"); }
  if (layout.ouvrage) {
    const pl = layout.ouvrage.plancher;
    const ov = layout.ouvrage;
    if (ov.niveaux > 1) {
      row("Niveaux", ov.niveaux, ov.demi_niveaux ? `niveaux (${2 * ov.niveaux} demi-niveaux)` : "niveaux"); row("Places au rez", ov.places_rdc, "places"); if (ov.niveaux > 2) row("Places par niveau courant", ov.places_niveau, "places"); row("Places au dernier niveau", ov.places_dernier, "places");
      row("Implantation des rampes", ov.rampes.length ? IMPLANTATION[ov.implantation] : "aucune");
      const pc = (r) => `${(r.pente * 100).toFixed(1).replace(".", ",")} %`, m2 = (x) => x.toFixed(2).replace(".", ",");
      for (const r of ov.rampes) {
        if (r.forme === "u") row("Demi-rampe en U à double sens", `${pc(r)}, ${m2(r.longueur)} m dans l'axe de la voie intérieure, un demi-étage`);
        else if (r.forme === "helice") row(r.montee ? "Tour hélicoïdale de montée" : "Tour hélicoïdale de descente", `${pc(r)}, ${m2(r.longueur)} m par tour dans l'axe de la voie, un tour par niveau`);
        else if (r.forme === "virage") row("Demi-tour horizontal à double sens", `${m2(r.longueur)} m dans l'axe de la voie intérieure`);
        else if (r.forme === "plateau") row("Plateau incliné", `${pc(r)} sur ${m2(r.longueur)} m, un demi-étage`);
        else row(r.montee ? "Rampe de montée" : "Rampe de descente", `${Math.round(r.pente * 100)} %, ${m2(r.longueur)} m (raccords R 10 et R 15 compris)`);
      }
      for (const g of ov.girations) row(`Giration : ${g.libelle.toLowerCase()}`, g.conflits.length ? `gênée par ${g.conflits.join(", ")}` : "conforme (voie de 4,00 m, rayon extérieur 8,50 m, NF P 91-100 § 3.3)");
      const ev = ov.evacuation;
      if (ev) {
        row("Escaliers", ev.escaliers, "cages, dans l'emprise d'une place"); row("Ascenseurs", ev.ascenseurs, ov.demi_niveaux && ev.ascenseurs > 1 ? "gaines, une par demi-plateau" : "gaines, contre le premier escalier");
        if (ev.liaisons) row("Escaliers de liaison", ev.liaisons, "entre les deux demi-plateaux, places PMR du côté de l'ascenseur");
        row("Parc largement ventilé (PS 3)", ev.largement_ventile ? "oui" : "non");
        row("Distance admise à un escalier", `${ev.limite_deux} m entre deux escaliers opposés, ${ev.limite_un} m sinon (PS 13, dans l'axe des circulations)`);
        row("Plus grande distance à un escalier", ev.distance_max, "m", "N1"); row("Places au-delà", ev.hors, "places");
      }
    }
    row("Système constructif (silo)", SYSTEMES[params.silo_systeme].nom); row("Portée de la trame (silo)", DENSITE[params.silo_densite]); row("Portée en travers", layout.ouvrage.portee, "m", "N2");
    row("Plancher", pl.libelle); if (pl.ossature) row(pl.ossature.solive ? "Solives" : "Poutres", `${pl.ossature.profil.replace("_", " ")} : ${layout.ouvrage.poutres.length}`); row("Épaisseur du plancher", pl.epaisseur, "m", "N2"); row("Retombée", pl.retombee, "m", "N2"); row("Hauteur d'étage minimale", pl.hauteur_min, "m (NF P 91-100, § 3.2)", "N2"); row("Source du plancher", pl.source);
  }
  sec("Estimation financière (€ HT, métropole niçoise)");
  row("Tranche du barème", `${br.a} à ${br.b} places`);
  row("Travaux seuls : ratio mini", T[0], "€ / place"); row("Travaux seuls : ratio maxi", T[1], "€ / place");
  row("Travaux seuls : total mini", n * T[0], "€", "E"); row("Travaux seuls : total maxi", n * T[1], "€", "E"); row("Travaux seuls : valeur centrale", n * (T[0] + T[1]) / 2, "€", "E");
  row("Études et MOE comprises : ratio mini", E[0], "€ / place"); row("Études et MOE comprises : ratio maxi", E[1], "€ / place");
  row("Études et MOE comprises : total mini", n * E[0], "€", "E"); row("Études et MOE comprises : total maxi", n * E[1], "€", "E"); row("Études et MOE comprises : valeur centrale", n * (E[0] + E[1]) / 2, "€", "E");
  row("Travaux seuls (centrale) par m² de parcelle", pa ? n * (T[0] + T[1]) / 2 / pa : "", "€ / m²", "N1"); row("Travaux seuls (centrale) par m² enrobé", m.paved_area ? n * (T[0] + T[1]) / 2 / m.paved_area : "", "€ / m²", "N1");
  const cell = (st, v) => v === "" || v === undefined ? `<Cell ss:StyleID="${st}"/>` : typeof v === "number" ? `<Cell ss:StyleID="${st}"><Data ss:Type="Number">${isFinite(v) ? v : 0}</Data></Cell>` : `<Cell ss:StyleID="${st}"><Data ss:Type="String">${xe(v)}</Data></Cell>`;
  const body = rows.map(([st, l, v, u]) => st === "S" ? `<Row ss:Height="24"><Cell ss:StyleID="S"><Data ss:Type="String">${xe(l)}</Data></Cell><Cell ss:StyleID="S"/><Cell ss:StyleID="S"/></Row>` : `<Row>${cell("L", l)}${cell(st, v)}${cell("U", u)}</Row>`).join("");
  const bar = '<Row>' + ["Places", "Travaux mini (€/place)", "Travaux maxi (€/place)", "Études + MOE mini (€/place)", "Études + MOE maxi (€/place)", "Tranche appliquée"].map((h) => cell("S", h)).join("") + "</Row>" +
    BAREME.map((r, i) => { const t = ord(r.t), e = ord(r.e); return `<Row>${cell("L", `${r.a} à ${r.b}`)}${cell("N0", t[0])}${cell("N0", t[1])}${cell("N0", e[0])}${cell("N0", e[1])}${cell("L", i === bi ? "oui" : "")}</Row>`; }).join("");
  const st = (id, extra, nf) => `<Style ss:ID="${id}"><Font ss:FontName="Calibri" ss:Size="11"${extra.bold ? ' ss:Bold="1"' : ""}${extra.color ? ` ss:Color="${extra.color}"` : ""}/>${extra.fill ? `<Interior ss:Color="${extra.fill}" ss:Pattern="Solid"/>` : ""}<Alignment ss:Vertical="Center"${extra.right ? ' ss:Horizontal="Right"' : ""}/>${nf ? `<NumberFormat ss:Format="${nf}"/>` : ""}</Style>`;
  const styles = st("Default", {}) + st("L", {}) + st("U", { color: "#6B7280" }) + st("T", { right: true }) + st("S", { bold: true, fill: "#E8EDF2" }) +
    st("N0", { right: true }, "#,##0") + st("N1", { right: true }, "#,##0.0") + st("N2", { right: true }, "#,##0.00") + st("N5", { right: true }, "0.00000") +
    st("P", { right: true }, "0.0%") + st("E", { right: true, bold: true }, "#,##0\\ &quot;€&quot;");
  const cols = (w) => w.map((x) => `<Column ss:Width="${x}"/>`).join("");
  return `<?xml version="1.0" encoding="UTF-8"?><?mso-application progid="Excel.Sheet"?>
<Workbook xmlns="urn:schemas-microsoft-com:office:spreadsheet" xmlns:ss="urn:schemas-microsoft-com:office:spreadsheet"><Styles>${styles}</Styles>
<Worksheet ss:Name="Synthèse"><Table>${cols([290, 170, 220])}${body}</Table></Worksheet>
<Worksheet ss:Name="Barème"><Table>${cols([90, 140, 140, 170, 170, 120])}${bar}</Table></Worksheet></Workbook>`;
}
async function exportXls() {
  if (!layout || !layout.metrics) { toast("Aucun plan calculé à exporter"); return; }
  const I = projectInfo(), name = `parkimo-${slug(I.commune || (exampleName === "mine" ? "mon site" : exampleName))}-${new Date().toISOString().slice(0, 10)}.xls`;
  closeMenus();
  await saveFile(name, buildXls(), `${fmtInt(layout.metrics.stalls)} places`, "application/vnd.ms-excel", "Tableur");
}
$("#x-xls").addEventListener("click", exportXls);

// Enregistrement : lien de téléchargement dans la version autonome ; dans Claude, capacité « downloads »
// (le format .dxf n'y étant pas admis, le fichier est livré dans une archive .zip).
async function saveFile(name, text, info, mime, kind) {
  const claudeView = window.claude && typeof window.claude.use === "function";
  if (claudeView) {
    const dl = await window.claude.use("downloads");
    if (!dl) { toast("Téléchargement indisponible ici : utilisez la version autonome index.html"); return; }
    try { await dl.save({ filename: name.replace(/\.\w+$/, ".zip"), data: zipStore(name, text) }); toast(kind + " enregistré dans une archive ZIP · " + info); }
    catch (e) { if (!e || e.code !== "declined") toast("Téléchargement indisponible ici : utilisez la version autonome index.html"); }
    return;
  }
  const a = document.createElement("a");
  a.href = URL.createObjectURL(new Blob([text], { type: mime }));
  a.download = name; document.body.append(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(a.href), 5000);
  toast(kind + " téléchargé · " + info);
}
// Archive ZIP minimale (un fichier, sans compression).
function zipStore(name, text) {
  const enc = new TextEncoder(), data = enc.encode(text), fname = enc.encode(name);
  let crc = ~0;
  for (let i = 0; i < data.length; i++) { crc ^= data[i]; for (let k = 0; k < 8; k++) crc = (crc >>> 1) ^ (0xEDB88320 & -(crc & 1)); }
  crc = ~crc >>> 0;
  const d = new Date(), time = (d.getHours() << 11) | (d.getMinutes() << 5) | (d.getSeconds() >> 1), date = ((d.getFullYear() - 1980) << 9) | ((d.getMonth() + 1) << 5) | d.getDate();
  const head = (central) => {
    const b = new DataView(new ArrayBuffer(central ? 46 : 30)); let o = 0;
    const u32 = (v) => { b.setUint32(o, v, true); o += 4; }, u16 = (v) => { b.setUint16(o, v, true); o += 2; };
    u32(central ? 0x02014b50 : 0x04034b50); if (central) u16(20);
    u16(20); u16(0x0800); u16(0); u16(time); u16(date); u32(crc); u32(data.length); u32(data.length); u16(fname.length); u16(0);
    if (central) { u16(0); u16(0); u16(0); u32(0); u32(0); }
    return new Uint8Array(b.buffer);
  };
  const local = head(false), central = head(true);
  const end = new DataView(new ArrayBuffer(22));
  end.setUint32(0, 0x06054b50, true); end.setUint16(8, 1, true); end.setUint16(10, 1, true);
  end.setUint32(12, central.length + fname.length, true); end.setUint32(16, local.length + fname.length + data.length, true);
  return new Blob([local, fname, data, central, fname, new Uint8Array(end.buffer)], { type: "application/zip" });
}

