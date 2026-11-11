"""Projets de référence pour tests/projet.rs : pour chaque projet de l'interface, l'entrée du moteur
que l'interface en tire (engineInput). Usage : fixtures.py page.html sortie.json"""
import sys, json, pathlib
from playwright.sync_api import sync_playwright
CAS = {
  "ruisseau": "site = normSite(EXAMPLES.ruisseau()); params = { ...DEFAULT_PARAMS };",
  "courbe": "site = normSite(EXAMPLES.courbe()); params = { ...DEFAULT_PARAMS };",
  "arcs_et_objets": """site = normSite(EXAMPLES.ruisseau()); params = { ...DEFAULT_PARAMS, slant: 60, pmr_ratio: 3, door_w: 4, building_margin: 1.5, planter_every: 6.4, canopy: 10, ombrieres: true };
    site.arcs[1] = 0.2; site.arcs[3] = -0.1;
    site.landscapes[0][1] = [30, 8, 0.12];
    site.basins.push([[60, 10], [80, 10, -0.2], [80, 30], [60, 30]]);
    site.buildings.push({ cx: 60, cy: 60, w: 20, h: 12, rot: 15, door: { e: 1, t: 0.3 } });
    site.traverses = [[[40, 20], [40, 60]]];
    site.entries.push({ edge: 1, t: 0.4 });""",
  "silo": """site = normSite(EXAMPLES.ruisseau()); params = { ...DEFAULT_PARAMS, silo: true, silo_niveaux: 3, silo_rampes: "helice", angle: 12, silo_densite: "courte", silo_ventile: false, silo_controle: true, silo_voies: 2 };
    site.slab = { cx: 80, cy: 25, w: 90, h: 32, rot: 12, fixe: [1, 0] }; site.rampes = [[70, 20], null];""",
  "sans_batiment": "site = normSite({ parcel: [[0, 0], [60, 0], [60, 40], [0, 40]], setbacks: [3], exclusions: [], buildings: [], landscapes: [], entries: [{ edge: 2, t: 0.25 }] }); params = { ...DEFAULT_PARAMS, programme: 'privatif' };",
}
out = []
with sync_playwright() as p:
    b = p.chromium.launch(executable_path="/usr/bin/google-chrome", args=["--no-sandbox"])
    pg = b.new_page()
    pg.goto(pathlib.Path(sys.argv[1]).resolve().as_uri())
    pg.wait_for_function("typeof layout !== 'undefined' && layout && layout.stalls && layout.stalls.length > 0", timeout=30000)
    print("exemples:", pg.evaluate("Object.keys(EXAMPLES)"))
    for nom, js in CAS.items():
        r = pg.evaluate("(() => { " + js + " solve(false); return JSON.parse(JSON.stringify({ projet: { site, params }, entree: engineInput(false), places: layout.stalls.length })); })()")
        r["nom"] = nom
        out.append(r); print(nom, r["places"], "places,", len(r["entree"]["parcel"]), "sommets")
    b.close()
pathlib.Path(sys.argv[2]).write_text(json.dumps(out, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
