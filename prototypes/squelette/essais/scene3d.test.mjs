// Essais de la scène 3D sur les plateaux de référence (lus dans le squelette par tools/essais/essai_vue3d.py).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import clip from "polygon-clipping";
import { scene, EDICULE, PALIER, ALLEGE, ACROTERE } from "../scene3d.js";

const dir = new URL("./plateaux/", import.meta.url);
const plateaux = Object.fromEntries(readdirSync(dir).filter((f) => f.endsWith(".json")).map((f) => [f.slice(0, -5), JSON.parse(readFileSync(new URL(f, dir), "utf8"))]));
const aire = (r) => Math.abs(r.reduce((m, p, i) => { const n = r[(i + 1) % r.length]; return m + p[0] * n[1] - n[0] * p[1]; }, 0)) / 2;
const aireV = (v) => aire(v.anneaux[0]) - v.anneaux.slice(1).reduce((m, r) => m + aire(r), 0);
const proche = (a, b, e = 1e-6) => Math.abs(a - b) <= e;
const scenes = Object.fromEntries(Object.entries(plateaux).map(([k, P]) => { const t = performance.now(); const sc = scene(P, clip); console.log(`${k} : ${(performance.now() - t).toFixed(1)} ms, ${sc.volumes.length} volumes, ${sc.rubans.length} rubans, ${sc.sols.length} sols`); return [k, { P, sc }]; }));

test("contrat : le plateau porte les champs que la scène lit", () => {
  const champs = ["E", "dalle", "retombee", "section", "voiles", "parc", "niveaux", "pente", "chaussees", "rangees", "places", "pietons", "zebres", "rampes", "rampesVoie", "helices", "escaliers", "marches", "poteaux", "poutres"];
  for (const [k, P] of Object.entries(plateaux)) for (const c of champs) assert.ok(c in P, `${k} : ${c} manque`);
  assert.ok(Object.keys(plateaux).length >= 4);
});

for (const [k, { P, sc }] of Object.entries(scenes)) {
  const ventile = P.parc === "ventile", n = P.niveaux, T = n, H = ventile ? n - 1 : n, F = sc.niveaux;

  test(`${k} : un plancher par niveau, plus la toiture du silo ou la couverture du parc enterré`, () => {
    assert.equal(F.length, T + 1);
    for (let f = 1; f < F.length; f++) assert.ok(proche(F[f].z - F[f - 1].z, P.E), `hauteur d'étage ${f}`);
    assert.ok(proche(ventile ? F[0].z : F[T].z, 0), "cote 0 : rez du silo ou couverture du parc enterré");
    const dalles = new Set(sc.volumes.filter((v) => ["dalle", "dallage", "radier", "couverture"].includes(v.matiere)).map((v) => v.niveau));
    assert.deepEqual([...dalles].sort((a, b) => a - b), F.map((_, f) => f));
  });

  test(`${k} : places de la scène = places du plateau × niveaux`, () => {
    assert.equal(sc.places, P.places.length * n);
    assert.equal(sc.traits.length, P.places.length * n * 3);
  });

  test(`${k} : chaque dalle est ouverte de ses trémies`, () => {
    const plein = sc.volumes.filter((v) => v.niveau === 0 && ["dallage", "radier"].includes(v.matiere)).reduce((m, v) => m + aireV(v), 0);
    const ouverte = sc.volumes.filter((v) => v.niveau === 1 && ["dalle", "couverture"].includes(v.matiere)).reduce((m, v) => m + aireV(v), 0);
    const M = P.marches, trem = P.rampes.filter((r) => !r.hors).reduce((m, r) => m + aire(r.q), 0) + P.rampesVoie.reduce((m, r) => m + aire(r.q), 0)
      + P.helices.reduce((m, h) => m + 64 / 2 * h.ext * h.ext * Math.sin((2 * Math.PI) / 64), 0)
      + P.escaliers.filter((x) => !x.hors && x.plan).reduce((m, x) => { const rs = [...x.plan.zones.filter((z) => z.nom === "palier").map((z) => z.r), ...x.plan.volees.map((v) => v.r)], b = [Math.min(...rs.map((r) => r[0])), Math.min(...rs.map((r) => r[1])), Math.max(...rs.map((r) => r[2])), Math.max(...rs.map((r) => r[3]))];
        return m + (b[2] - b[0]) * (b[3] - b[1]) + x.plan.zones.filter((z) => z.nom === "ascenseur" || z.nom === "gaine").reduce((s, z) => s + (z.r[2] - z.r[0]) * (z.r[3] - z.r[1]), 0); }, 0);
    assert.ok(trem > 0);
    assert.ok(Math.abs(plein - ouverte - trem) < 0.01 * plein, `${plein.toFixed(1)} - ${ouverte.toFixed(1)} ≠ ${trem.toFixed(1)}`);
  });

  test(`${k} : le dernier niveau de stationnement a ses poteaux et ses poutres, sous la toiture ou la couverture`, () => {
    const der = F.findLastIndex((x) => x.places > 0);
    assert.equal(der, T - 1);
    assert.equal(sc.volumes.filter((v) => v.matiere.startsWith("poteau") && v.niveau === der).length, P.poteaux.length);
    assert.ok(sc.volumes.some((v) => v.matiere === "couverture" && v.niveau === T));
  });

  test(`${k} : poteaux sous les poutres, d'un plancher à l'autre`, () => {
    const pot = sc.volumes.filter((v) => v.matiere.startsWith("poteau"));
    assert.equal(pot.length, P.poteaux.length * T);
    for (const v of pot) assert.ok(proche(v.z1 - v.z0, P.E - P.dalle - P.retombee, 1e-3), `${v.z1 - v.z0}`);
    assert.equal(sc.volumes.filter((v) => v.matiere === "poutre").length, P.retombee > 0 ? P.poutres.filter((b) => !b.noyau && !b.voile).length * T : 0);
  });

  test(`${k} : rampes d'un plancher au suivant, hélices jusqu'au dernier`, () => {
    const nb = P.rampes.filter((r) => !r.hors).length + P.rampesVoie.length + P.helices.length;
    assert.equal(sc.rampes.length, nb * H);
    for (const r of sc.rampes) {
      const z0 = r.axe[0][2], z1 = r.axe[r.axe.length - 1][2], f = r.niveau;
      assert.ok(proche(z0, F[f].z), "pied de rampe au plancher");
      if (r.genre === "helice") assert.ok(proche(z1, F[f + 1].z, 1e-6), "un tour par étage");
    }
    for (const h of sc.rampes.filter((r) => r.genre === "helice" && r.niveau === H - 1)) assert.ok(proche(h.axe[h.axe.length - 1][2], F[H].z, 1e-6));
  });

  test(`${k} : escaliers, deux volées par étage desservi ; cage jusqu'à la toiture du silo, édicule en enterré`, () => {
    const M = P.marches, cages = P.escaliers.filter((x) => !x.hors);
    assert.equal(sc.volumes.filter((v) => v.matiere === "marche").length, cages.length * H * (M.n1 - 1 + M.n2 - 1));
    for (const x of cages) {
      const toit = sc.volumes.filter((v) => v.cle === x.cle).reduce((m, v) => Math.max(m, v.z1), -Infinity);
      assert.ok(proche(toit, ventile ? F[T].z : F[T].z + EDICULE + PALIER, 1e-3), `${toit}`);
    }
    for (const x of cages) assert.ok(x.plan, `${x.cle} : plan du noyau`);
    // Murs percés : à chaque étage, la partie basse des murs (portes, baie) s'arrête à 2,10 m.
    for (const x of cages) for (let f = 0; f < T; f++) assert.ok(sc.volumes.some((v) => v.cle === x.cle && v.matiere === "noyau" && v.niveau === f && proche(v.z0, F[f].z) && proche(v.z1, F[f].z + 2.1, 1e-3)));
    // Chaque volée monte d'un palier à l'autre : la dernière marche de la seconde est à une marche du plancher du dessus.
    const hautes = sc.volumes.filter((v) => v.matiere === "marche" && v.niveau === 0).map((v) => v.z1);
    if (cages.length) assert.ok(proche(Math.max(...hautes), F[1].z - M.h, 1e-3));
  });

  test(`${k} : façades et terrain`, () => {
    const al = sc.volumes.filter((v) => v.matiere === "allege" && !v.cle);
    if (ventile) {
      assert.ok(al.length > 0 && al.every((v) => v.niveau >= 1 && v.niveau <= H && proche(v.z1 - v.z0, ALLEGE)), "allèges aux niveaux de stationnement, hors rez");
      assert.ok(sc.volumes.some((v) => v.matiere === "acrotere" && v.niveau === T && proche(v.z1 - v.z0, ACROTERE)), "acrotère de la toiture");
      assert.ok(sc.volumes.some((v) => v.matiere === "terrain" && v.z1 < 0));
    } else {
      assert.equal(al.length, 0);
      const t = sc.volumes.filter((v) => v.matiere === "terrain");
      assert.ok(t.length && t.every((v) => proche(v.z1, 0) && v.anneaux.length >= 2), "terrain percé du parc, arasé au sol");
    }
  });

  test(`${k} : cotes basses sous les cotes hautes, rien hors de l'enveloppe`, () => {
    const m = 0.6, [x0, y0] = sc.min, [x1, y1] = sc.max;
    for (const v of sc.volumes) {
      assert.ok(v.z0 < v.z1);
      if (v.matiere === "terrain" || (v.matiere === "voile" && !ventile)) continue;
      for (const r of v.anneaux) for (const p of r) assert.ok(p[0] >= x0 - m && p[0] <= x1 + m && p[1] >= y0 - m && p[1] <= y1 + m, `${v.matiere} hors de l'enveloppe`);
    }
    for (const r of sc.rubans) assert.ok(r.z0 < r.z1);
    assert.ok(!sc.constats.some((c) => c.startsWith("Polygones")), sc.constats.join(" ; "));
  });
}

test("scène vide sans plan", () => {
  const sc = scene(null, clip);
  assert.equal(sc.volumes.length, 0);
});
