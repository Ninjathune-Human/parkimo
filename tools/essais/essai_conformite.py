"""Essai de la ligne Conformité dans Chrome : défauts, écart, calcul rapide, programme, silo.
Usage : essai_conformite.py page.html dossier_des_captures"""
import sys, json, pathlib
from playwright.sync_api import sync_playwright
page_path = pathlib.Path(sys.argv[1]).resolve()
out = pathlib.Path(sys.argv[2]); out.mkdir(exist_ok=True)
errs = []
LIST = "[...document.querySelectorAll('#conf-body .cst')].map(r => [r.className.replace('cst','').trim(), r.querySelector('.cst-l').textContent, r.querySelector('.cst-v').textContent, r.querySelector('.cst-s').textContent])"
HEAD = "({cache: document.querySelector('#conf-head').hidden, resume: document.querySelector('#s-conf').textContent, ouvert: !document.querySelector('#conf-body').hidden})"
with sync_playwright() as p:
    b = p.chromium.launch(executable_path="/usr/bin/google-chrome", args=["--no-sandbox"])
    for name, vp in [("pc", {"width": 1440, "height": 900}), ("iphone", {"width": 390, "height": 844})]:
        ctx = b.new_context(viewport=vp, device_scale_factor=2 if name == "iphone" else 1, has_touch=name == "iphone")
        pg = ctx.new_page()
        pg.on("console", lambda m: errs.append(f"[{name}] {m.type}: {m.text}") if m.type == "error" else None)
        pg.on("pageerror", lambda e: errs.append(f"[{name}] pageerror: {e}"))
        pg.goto(page_path.as_uri())
        pg.wait_for_function("typeof layout !== 'undefined' && layout && layout.stalls && layout.stalls.length > 0", timeout=30000)
        pg.wait_for_timeout(1200)
        r = {"vue": name, "titre": pg.title()}
        if name == "iphone":
            pg.click("#stats-head"); pg.wait_for_timeout(300)
        r["defaut"] = pg.evaluate(HEAD)
        pg.click("#conf-head"); pg.wait_for_timeout(300)
        r["defaut_liste"] = pg.evaluate(LIST)
        pg.screenshot(path=str(out / f"{name}-conf-defaut.png"))
        # Allée de 5,50 m : un écart, en tête de liste.
        pg.evaluate("(params.aisle = 5.5, syncParamsUI(), solve(false), 0)"); pg.wait_for_timeout(600)
        r["allee_5_50"] = [pg.evaluate(HEAD), pg.evaluate(LIST)[0]]
        pg.screenshot(path=str(out / f"{name}-conf-ecart.png"))
        # Glisser (calcul rapide) : les constats restent.
        pg.evaluate("(solve(true), 0)"); pg.wait_for_timeout(300)
        r["apres_calcul_rapide"] = pg.evaluate(HEAD)["resume"]
        if name == "pc":
            # Programme privatif par la commande de Paramètres.
            pg.click("#b-params"); pg.wait_for_timeout(300)
            pg.click("#prog-seg [data-g=privatif]"); pg.wait_for_timeout(600)
            r["privatif"] = [pg.evaluate("params.programme"), pg.evaluate(HEAD)["resume"], [x[1] for x in pg.evaluate(LIST)]]
            pg.screenshot(path=str(out / "pc-params-programme.png"))
            pg.click("#prog-seg [data-g=public]"); pg.keyboard.press("Escape"); pg.wait_for_timeout(300)
        # Silo à trois niveaux.
        pg.evaluate("(params.aisle = 6, params.silo = true, params.silo_niveaux = 3, syncParamsUI(), solve(false), 0)"); pg.wait_for_timeout(800)
        r["silo"] = [pg.evaluate(HEAD)["resume"], [x[:3] for x in pg.evaluate(LIST) if x[0] != "ok"]]
        pg.screenshot(path=str(out / f"{name}-conf-silo.png"))
        print(json.dumps(r, ensure_ascii=False, indent=1))
        ctx.close()
    b.close()
print("erreurs console:", len(errs)); [print(" ", e[:300]) for e in errs[:15]]
