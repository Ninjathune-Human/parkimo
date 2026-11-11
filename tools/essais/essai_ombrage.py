"""Essai de l'ombrage dans Chrome : constats par défaut, ombrières par Paramètres, houppier, commande.
Usage : essai_ombrage.py page.html dossier_des_captures"""
import sys, json, pathlib
from playwright.sync_api import sync_playwright
page_path = pathlib.Path(sys.argv[1]).resolve()
out = pathlib.Path(sys.argv[2]); out.mkdir(exist_ok=True)
errs = []
URBA = "constats.filter(c => c.code.startsWith('urba.')).map(c => [c.code, c.statut, c.mesure, c.exige, c.detail])"
with sync_playwright() as p:
    b = p.chromium.launch(executable_path="/usr/bin/google-chrome", args=["--no-sandbox"])
    for name, vp in [("pc", {"width": 1440, "height": 900}), ("iphone", {"width": 390, "height": 844})]:
        ctx = b.new_context(viewport=vp, device_scale_factor=2, has_touch=name == "iphone")
        pg = ctx.new_page()
        pg.on("console", lambda m: errs.append(f"[{name}] {m.type}: {m.text}") if m.type == "error" else None)
        pg.on("pageerror", lambda e: errs.append(f"[{name}] pageerror: {e}"))
        pg.goto(page_path.as_uri())
        pg.wait_for_function("typeof layout !== 'undefined' && layout && layout.stalls && layout.stalls.length > 0", timeout=30000)
        pg.wait_for_timeout(1200)
        r = {"vue": name}
        r["defaut"] = [pg.evaluate("layout.ombrage"), pg.evaluate(URBA)]
        if name == "iphone":
            pg.click("#stats-head"); pg.wait_for_timeout(300)
        pg.click("#conf-head"); pg.wait_for_timeout(300)
        pg.screenshot(path=str(out / f"{name}-ombrage-defaut.png"))
        if name == "iphone":
            pg.click("#stats-head"); pg.wait_for_timeout(300)
        # Ombrières par l'interrupteur de Paramètres, houppier porté à 12 m par son champ.
        pg.click("#b-params"); pg.wait_for_timeout(300)
        pg.locator("#p-ombrieres").scroll_into_view_if_needed(); pg.click("#p-ombrieres"); pg.wait_for_timeout(700)
        r["ombrieres"] = [pg.evaluate("[params.ombrieres, layout.ombrieres.length, layout.pads.length, layout.trees.length]"), pg.evaluate(URBA)]
        pg.screenshot(path=str(out / f"{name}-ombrage-parametres.png"))
        pg.fill("#p-canopy", "12"); pg.wait_for_timeout(900)
        pg.keyboard.press("Escape"); pg.wait_for_timeout(300)
        if name == "iphone" and pg.evaluate("!document.querySelector('#params').hidden"):
            pg.click("#b-params"); pg.wait_for_timeout(300)
        r["mixte"] = [pg.evaluate("layout.ombrage"), pg.evaluate(URBA)]
        pg.screenshot(path=str(out / f"{name}-ombrage-mixte.png"))
        # Par une commande, comme le ferait l'assistant ; puis annulation.
        r["commande"] = pg.evaluate("(() => { const x = parkimo.appliquer([{op:'regler', cle:'ombrieres', valeur:false}, {op:'regler', cle:'canopy', valeur:8}]); return [x.ok, params.ombrieres, params.canopy, layout.ombrieres.length]; })()")
        pg.evaluate("undo()"); pg.wait_for_timeout(500)
        r["annule"] = pg.evaluate("[params.ombrieres, params.canopy, layout.ombrieres.length]")
        # Silo : ni ombrage ni ombrières.
        pg.evaluate("(params.silo = true, syncParamsUI(), solve(false), 0)"); pg.wait_for_timeout(800)
        r["silo"] = [pg.evaluate("layout.ombrieres.length"), pg.evaluate(URBA)]
        print(json.dumps(r, ensure_ascii=False, indent=1))
    b.close()
print("erreurs:", errs)
