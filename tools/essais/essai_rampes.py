"""Essai du déplacement d'une rampe à la main dans Chrome : glisser, annuler, double appui.
Usage : essai_rampes.py page.html dossier_des_captures"""
import sys, json, pathlib
from playwright.sync_api import sync_playwright
page_path = pathlib.Path(sys.argv[1]).resolve()
out = pathlib.Path(sys.argv[2]); out.mkdir(exist_ok=True)
errs = []
ETAT = "(() => { const o = layout.ouvrage, r = o.rampes[0], q = r.q, c = [(q[0][0] + q[2][0]) / 2, (q[0][1] + q[2][1]) / 2], s = toS(c), a = toS(r.a), d = toS(r.de), cr = canvas.getBoundingClientRect(); return { centre: c.map(v => +v.toFixed(2)), ecran: [s[0] + cr.left, s[1] + cr.top], axe: [a[0] - d[0], a[1] - d[1]], ancres: site.rampes || null, places: layout.stalls.length, epures: o.girations.length, conflits: o.girations.filter(g => g.conflits.length).length, impl: o.implantation }; })()"
with sync_playwright() as p:
    b = p.chromium.launch(executable_path="/usr/bin/google-chrome", args=["--no-sandbox"])
    ctx = b.new_context(viewport={"width": 1440, "height": 900}, device_scale_factor=2)
    pg = ctx.new_page()
    pg.on("console", lambda m: errs.append(f"{m.type}: {m.text}") if m.type == "error" else None)
    pg.on("pageerror", lambda e: errs.append(f"pageerror: {e}"))
    pg.goto(page_path.as_uri())
    pg.wait_for_function("typeof layout !== 'undefined' && layout && layout.stalls && layout.stalls.length > 0", timeout=30000)
    pg.wait_for_timeout(1000)
    pg.evaluate("(params.silo = true, params.silo_niveaux = 3, params.silo_rampes = 'rangee', syncParamsUI(), solve(false), commit(), 0)"); pg.wait_for_timeout(900)
    e0 = pg.evaluate(ETAT); r = {"avant": e0}
    pg.screenshot(path=str(out / "pc-rampes-avant.png"))
    # Glisser la montée de 150 px dans le sens de son axe.
    n = (e0["axe"][0] ** 2 + e0["axe"][1] ** 2) ** 0.5; ux, uy = e0["axe"][0] / n, e0["axe"][1] / n
    x, y = e0["ecran"]
    pg.mouse.move(x, y); pg.mouse.down()
    for k in range(1, 7):
        pg.mouse.move(x + ux * 25 * k, y + uy * 25 * k); pg.wait_for_timeout(250)
    pg.mouse.up(); pg.wait_for_timeout(900)
    r["apres"] = pg.evaluate(ETAT)
    pg.screenshot(path=str(out / "pc-rampes-apres.png"))
    pg.evaluate("undo()"); pg.wait_for_timeout(700)
    r["annule"] = pg.evaluate(ETAT)
    pg.evaluate("redo()"); pg.wait_for_timeout(700)
    e2 = pg.evaluate(ETAT)
    # Double appui : retour au placement d'office.
    pg.mouse.dblclick(e2["ecran"][0], e2["ecran"][1]); pg.wait_for_timeout(900)
    r["double_appui"] = pg.evaluate(ETAT)
    print(json.dumps(r, ensure_ascii=False))
    b.close()
print("erreurs:", errs)
