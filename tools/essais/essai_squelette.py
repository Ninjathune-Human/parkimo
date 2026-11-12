"""Essai de la page du squelette dans Chrome : exemple, tracé d'une branche, réglage d'un côté, glisser d'un nœud, annulation.
Usage : essai_squelette.py page.html dossier_des_captures"""
import sys, json, pathlib
from playwright.sync_api import sync_playwright
page_path = pathlib.Path(sys.argv[1]).resolve()
out = pathlib.Path(sys.argv[2]); out.mkdir(exist_ok=True)
errs = []
ETAT = "({places: calc.pl.length, emprise: Math.round(calc.aire), branches: S.branches.length, noeuds: S.noeuds.length, choix, fiche: !document.querySelector('#fiche').hidden, texte: document.querySelector('#f-long').textContent})"
with sync_playwright() as p:
    b = p.chromium.launch(executable_path="/usr/bin/google-chrome", args=["--no-sandbox"])
    for name, vp in [("pc", {"width": 1440, "height": 900}), ("iphone", {"width": 390, "height": 844})]:
        ctx = b.new_context(viewport=vp, device_scale_factor=2, has_touch=name == "iphone")
        pg = ctx.new_page()
        pg.on("console", lambda m: errs.append(f"[{name}] {m.type}: {m.text}") if m.type == "error" else None)
        pg.on("pageerror", lambda e: errs.append(f"[{name}] pageerror: {e}"))
        pg.goto(page_path.as_uri()); pg.wait_for_timeout(600)
        r = {"vue": name, "depart": pg.evaluate(ETAT)}
        pg.screenshot(path=str(out / f"{name}-squelette-depart.png"))
        E = lambda x, y: pg.evaluate(f"versE([{x}, {y}])")
        # Tracer une branche depuis le milieu de la branche du bas, vers le bas : un nœud se crée sur la branche.
        pg.click("#b-tracer")
        a, c = E(30, 0), E(30, -30)
        pg.mouse.move(*a); pg.mouse.down(); pg.mouse.move((a[0] + c[0]) / 2, (a[1] + c[1]) / 2); pg.mouse.move(*c); pg.mouse.up(); pg.wait_for_timeout(300)
        r["trace"] = pg.evaluate(ETAT)
        pg.click("#b-tracer")
        # Régler le côté gauche de la branche choisie en épi à 60°, puis la passer en voie seule à droite.
        pg.click('.seg[data-cote="g"] button[data-t="60"]'); pg.click('.seg[data-cote="d"] button[data-t="aucun"]'); pg.wait_for_timeout(200)
        r["reglee"] = pg.evaluate(ETAT)
        # Glisser le bout de l'antenne vers le haut : elle pivote autour de son nœud.
        n, m = E(90, 0), E(85, 20)
        pg.mouse.move(*n); pg.mouse.down(); pg.mouse.move((n[0] + m[0]) / 2, (n[1] + m[1]) / 2); pg.mouse.move(*m); pg.mouse.up(); pg.wait_for_timeout(300)
        r["noeud"] = [pg.evaluate("S.noeuds[4].map(v => +v.toFixed(2))"), pg.evaluate(ETAT)["places"]]
        pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        pg.screenshot(path=str(out / f"{name}-squelette-apres.png"))
        for _ in range(4): pg.click("#b-annuler")
        r["annule"] = pg.evaluate(ETAT)
        print(json.dumps(r, ensure_ascii=False))
    b.close()
print("erreurs:", errs)
