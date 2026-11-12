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
        r["angles"] = pg.evaluate("calc.angles.map(a => a.x)")
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
        # Squelette de la structure : poteaux et poutres de l'exemple, puis un poteau glissé dans une voie.
        ST = "({poteaux: calc.pot.length, conflits: calc.conflits.length, genees: calc.genees.length, places: calc.pl.length, texte: document.querySelector('#s-structure').textContent, fiche: document.querySelector('#fiche').hidden ? null : document.querySelector('#f-long').textContent, lignes: [...document.querySelectorAll('#fiche .ligne')].filter(l => !l.hidden).length})"
        pg.click('#calques button[data-c="1"]'); pg.wait_for_timeout(200)
        r["structure"] = pg.evaluate(ST)
        n, m = E(30, 7.5), E(30, 1)
        pg.mouse.move(*n); pg.mouse.down(); pg.mouse.move((n[0] + m[0]) / 2, (n[1] + m[1]) / 2); pg.mouse.move(*m); pg.mouse.up(); pg.wait_for_timeout(300)
        r["poteau_en_voie"] = pg.evaluate(ST)
        pg.screenshot(path=str(out / f"{name}-squelette-structure.png"))
        pg.click("#b-annuler"); pg.wait_for_timeout(200)
        # Une poutre de plus, tracée depuis un poteau ; puis la choisir.
        pg.click("#b-tracer"); a, c = E(52.5, 22.5), E(52.5, 30)
        pg.mouse.move(*a); pg.mouse.down(); pg.mouse.move((a[0] + c[0]) / 2, (a[1] + c[1]) / 2); pg.mouse.move(*c); pg.mouse.up(); pg.wait_for_timeout(300)
        pg.click("#b-tracer")
        r["poutre"] = pg.evaluate(ST)
        # Poser un poteau d'un toucher : il s'aimante à un coin de place ; puis porter ses poutres le long de la voie.
        pg.click("#b-tracer"); q = E(15.3, 12.2); pg.mouse.click(*q); pg.wait_for_timeout(250); pg.click("#b-tracer")
        r["pose"] = pg.evaluate("({n: S.pn.filter(Boolean).length, p: S.pn[choixN], fiche: document.querySelector('#f-long').textContent, actions: !document.querySelector('#f-portee').hidden, genees: calc.genees.length})")
        pg.click('#f-portee button[data-p="travers"]'); pg.wait_for_timeout(200)
        r["portee_travers"] = pg.evaluate("[S.pb.filter(b => b.a === choixN || b.b === choixN).map(b => +repere(b, S.pn).L.toFixed(2)), document.querySelector('#f-long').textContent]")
        # Glisser ce poteau au milieu d'une place : il s'aimante à la ligne la plus proche, la place reste.
        a, c = E(*pg.evaluate("S.pn[choixN]")), E(16.4, 10.4)
        pg.mouse.move(*a); pg.mouse.down(); pg.mouse.move((a[0] + c[0]) / 2, (a[1] + c[1]) / 2); pg.mouse.move(*c); pg.mouse.up(); pg.wait_for_timeout(300)
        r["glisse"] = pg.evaluate("({p: S.pn[choixN], genees: calc.genees.length, places: calc.pl.length})")
        pg.screenshot(path=str(out / f"{name}-squelette-trame.png"))
        pg.click("#b-suppr"); pg.wait_for_timeout(200)
        r["supprime"] = pg.evaluate("[S.pn.filter(Boolean).length, S.pb.length, document.querySelector('#fiche').hidden]")
        # Un poteau posé près de l'angle du bâtiment s'y aimante.
        pg.click("#b-tracer"); q = E(-7.2, -7.0); pg.mouse.click(*q); pg.wait_for_timeout(250); pg.click("#b-tracer")
        r["poteau_d_angle"] = pg.evaluate("S.pn[choixN]")
        pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        pg.screenshot(path=str(out / f"{name}-squelette-angles.png"))
        pg.click('#calques button[data-c="0"]'); pg.wait_for_timeout(200)
        r["retour_voies"] = pg.evaluate("[calque, NO === S.noeuds, document.querySelector('#fiche').hidden]")
        print(json.dumps(r, ensure_ascii=False))
    b.close()
print("erreurs:", errs)
