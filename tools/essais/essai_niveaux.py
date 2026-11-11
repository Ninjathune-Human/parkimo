"""Essai du sélecteur de niveaux dans Chrome : silo à 4, 2 puis 1 niveau, retour en surface.
Usage : essai_niveaux.py page.html dossier_des_captures"""
import sys, json, pathlib
from playwright.sync_api import sync_playwright
page_path = pathlib.Path(sys.argv[1]).resolve()
out = pathlib.Path(sys.argv[2]); out.mkdir(exist_ok=True)
errs = []
ETAT = "({cache: document.querySelector('#niv').hidden, niveaux: [...document.querySelectorAll('#niv button')].map(b => [b.textContent, b.getAttribute('aria-checked'), b.getBoundingClientRect().height >= 44 && b.getBoundingClientRect().width >= 44]), actif: nivActif(), enPlus: layout.ouvrage ? nivPlaces().length : null, epures: layout.ouvrage ? nivGirations().length : null})"
with sync_playwright() as p:
    b = p.chromium.launch(executable_path="/usr/bin/google-chrome", args=["--no-sandbox"])
    for name, vp in [("pc", {"width": 1440, "height": 900}), ("iphone", {"width": 390, "height": 844})]:
        ctx = b.new_context(viewport=vp, device_scale_factor=2, has_touch=name == "iphone")
        pg = ctx.new_page()
        pg.on("console", lambda m: errs.append(f"[{name}] {m.type}: {m.text}") if m.type == "error" else None)
        pg.on("pageerror", lambda e: errs.append(f"[{name}] pageerror: {e}"))
        pg.goto(page_path.as_uri())
        pg.wait_for_function("typeof layout !== 'undefined' && layout && layout.stalls && layout.stalls.length > 0", timeout=30000)
        pg.wait_for_timeout(1000)
        r = {"vue": name, "surface": pg.evaluate(ETAT)}
        pg.evaluate("(params.silo = true, params.silo_niveaux = 4, syncParamsUI(), solve(false), fitView && 0)"); pg.wait_for_timeout(900)
        o = pg.evaluate("({n: layout.stalls.length, rdc: layout.ouvrage.places_rdc, niveau: layout.ouvrage.places_niveau, dernier: layout.ouvrage.places_dernier, total: layout.metrics.stalls, epures: layout.ouvrage.girations.map(g => g.bas)})")
        r["silo4"] = [o, pg.evaluate(ETAT)]
        pg.screenshot(path=str(out / f"{name}-niveaux-rdc.png"))
        for nid in ["courant", "dernier"]:
            pg.click(f"#niv button[data-n={nid}]"); pg.wait_for_timeout(300)
            r[nid] = pg.evaluate(ETAT)
            pg.screenshot(path=str(out / f"{name}-niveaux-{nid}.png"))
        # Le niveau choisi tient après un nouveau calcul.
        pg.evaluate("(solve(false), 0)"); pg.wait_for_timeout(600)
        r["apres_calcul"] = pg.evaluate("nivActif()")
        pg.evaluate("(params.silo_niveaux = 2, solve(false), 0)"); pg.wait_for_timeout(700)
        r["silo2"] = pg.evaluate(ETAT)
        pg.evaluate("(params.silo_niveaux = 1, solve(false), 0)"); pg.wait_for_timeout(700)
        r["silo1"] = pg.evaluate(ETAT)
        pg.evaluate("(params.silo = false, syncParamsUI(), solve(false), 0)"); pg.wait_for_timeout(700)
        r["retour_surface"] = pg.evaluate("document.querySelector('#niv').hidden")
        print(json.dumps(r, ensure_ascii=False))
    b.close()
print("erreurs:", errs)
