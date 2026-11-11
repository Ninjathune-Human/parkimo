"""Essai du rez payant dans Chrome : interrupteur, nombre de voies, niveaux, bilan, annulation.
Usage : essai_controle.py page.html dossier_des_captures"""
import sys, json, pathlib
from playwright.sync_api import sync_playwright
page_path = pathlib.Path(sys.argv[1]).resolve()
out = pathlib.Path(sys.argv[2]); out.mkdir(exist_ok=True)
errs = []
ETAT = "(() => { const o = layout.ouvrage, c = o.controle; return { voies: c ? c.voies.map(v => v.sens) : null, ilots: c ? c.ilots.length : 0, local: c && c.local ? c.local.surface : null, prises: c ? c.places_prises : 0, rdc: o.places_rdc, niveau: o.places_niveau, total: layout.metrics.stalls, affichees: nivStalls().length, actif: nivActif(), bilan: document.querySelector('#r-controle').hidden ? null : document.querySelector('#s-controle').textContent, champ: !document.querySelector('#voies-field').hidden }; })()"
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
        pg.evaluate("(params.silo = true, params.silo_niveaux = 3, syncParamsUI(), solve(false), commit(), 0)"); pg.wait_for_timeout(900)
        r = {"vue": name, "sans": pg.evaluate(ETAT)}
        # Interrupteur de Paramètres, puis deux voies par le champ.
        pg.click("#b-params"); pg.wait_for_timeout(300)
        pg.locator("#p-silo_controle").scroll_into_view_if_needed(); pg.click("#p-silo_controle"); pg.wait_for_timeout(900)
        r["avec"] = pg.evaluate(ETAT)
        pg.fill("#p-silo_voies", "2"); pg.wait_for_timeout(900)
        r["deux_voies"] = pg.evaluate(ETAT)
        pg.fill("#p-silo_voies", "3"); pg.wait_for_timeout(900)
        pg.keyboard.press("Escape"); pg.wait_for_timeout(200)
        if pg.evaluate("!document.querySelector('#params').hidden"):
            pg.click("#b-params"); pg.wait_for_timeout(300)
        # Cadrage sur le dispositif, au rez puis à l'étage courant.
        pg.evaluate("(() => { const c = layout.ouvrage.controle, q = c.voies[1].q, x = (q[0][0] + q[2][0]) / 2, y = (q[0][1] + q[2][1]) / 2; view.s = W < 760 ? 9 : 16; view.ox = W / 2 - x * view.s; view.oy = H / 2 + y * view.s; redraw(); })()"); pg.wait_for_timeout(400)
        pg.screenshot(path=str(out / f"{name}-controle-rdc.png"))
        pg.click("#niv button[data-n=courant]"); pg.wait_for_timeout(300)
        r["etage"] = pg.evaluate(ETAT)
        pg.screenshot(path=str(out / f"{name}-controle-etage.png"))
        pg.click("#niv button[data-n=rdc]"); pg.wait_for_timeout(200)
        # Par une commande, puis annulation ; un seul niveau.
        r["commande"] = pg.evaluate("(() => { const x = parkimo.appliquer([{op:'regler', cle:'silo_controle', valeur:false}]); return [x.ok, !!layout.ouvrage.controle]; })()")
        pg.evaluate("undo()"); pg.wait_for_timeout(600)
        r["annule"] = pg.evaluate("!!layout.ouvrage.controle")
        pg.evaluate("(params.silo_niveaux = 1, solve(false), 0)"); pg.wait_for_timeout(700)
        r["un_niveau"] = pg.evaluate(ETAT)
        print(json.dumps(r, ensure_ascii=False))
    b.close()
print("erreurs:", errs)
