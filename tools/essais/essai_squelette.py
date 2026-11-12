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
        E = lambda x, y: pg.evaluate(f"versE([{x}, {y}])")
        # Surfaces : chaussées, bandes et rangées se joignent sans se recouvrir ni laisser de vide. La somme de leurs aires
        # vaut l'emprise mesurée, sur l'exemple puis avec le carrefour à trois voies déplacé en biais.
        PAV = "(() => { const aire = (q) => Math.abs(q.reduce((s, p, i) => { const n = q[(i + 1) % q.length]; return s + p[0] * n[1] - n[0] * p[1]; }, 0)) / 2; return +([...calc.ch, ...calc.bandes].reduce((s, q) => s + aire(q), 0) / calc.aire).toFixed(3); })()"
        r["pavage"] = pg.evaluate(PAV)
        pg.evaluate("(() => { garder(); S.noeuds[1] = [56, -6]; rafraichir(); })()"); pg.wait_for_timeout(200)
        r["pavage_biais"] = [pg.evaluate("S.noeuds[1]"), pg.evaluate(PAV), pg.evaluate("[document.querySelector('#s-pietons').textContent, calc.conflits.length, calc.pl.length]")]
        q = E(*pg.evaluate("S.noeuds[1]")); pg.mouse.move(*q); pg.mouse.wheel(0, -1400); pg.wait_for_timeout(300)
        pg.screenshot(path=str(out / f"{name}-squelette-noeud-biais.png"))
        pg.click("#b-annuler"); pg.wait_for_timeout(200); pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        # Un nœud glissé emmène ce qui ne tient au reste que par lui : ici l'antenne suit le nœud du carrefour, la boucle reste.
        def glisser(de, vers):
            n, m = E(*de), E(*vers)
            pg.mouse.move(*n); pg.mouse.down(); pg.mouse.move((n[0] + m[0]) / 2, (n[1] + m[1]) / 2); pg.mouse.move(*m); pg.mouse.up(); pg.wait_for_timeout(300)
        glisser((60, 0), (65, 0.4))
        r["antenne_suit"] = pg.evaluate("S.noeuds.map(p => p.map(v => +v.toFixed(2)))")
        pg.click("#b-annuler"); pg.wait_for_timeout(200)
        glisser((0, 0), (-5, 0.4))
        r["boucle_reste"] = pg.evaluate("S.noeuds.map(p => p.map(v => +v.toFixed(2)))")
        pg.click("#b-annuler"); pg.wait_for_timeout(200)
        r["suite_annulee"] = pg.evaluate("S.noeuds.map(p => p.map(v => +v.toFixed(2)))")
        # Bandes piétonnes : un réseau continu au départ ; en ôter sur les deux pignons le coupe en tronçons.
        PI = "[document.querySelector('#s-pietons').textContent, calc.trottoirs.length, calc.zebres.length > 0, calc.pl.length]"
        r["pietons"] = pg.evaluate(PI)
        for x in (0, 60):
            q = E(x, 7.5); pg.mouse.click(*q); pg.wait_for_timeout(250)
            pg.click('#fiche .seg[data-cote="pieton"] button[data-t="aucun"]'); pg.wait_for_timeout(200)
            r["pietons_sans_%d" % x] = pg.evaluate(PI)
        q = E(60, 7.5); pg.mouse.click(*q); pg.wait_for_timeout(250)
        pg.click('#fiche .seg[data-cote="pieton"] button[data-t="g"]'); pg.wait_for_timeout(200)
        r["pietons_interieur"] = pg.evaluate(PI)
        for _ in range(3): pg.click("#b-annuler"); pg.wait_for_timeout(200)
        r["pietons_retablis"] = pg.evaluate(PI)
        pg.mouse.click(5, 5); pg.wait_for_timeout(200)
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
        # Contreventement : l'exemple a des remplissages en maçonnerie dans deux sens ; en ôter un sens se lit au bilan.
        CV = "[document.querySelector('#s-contrev').textContent, S.pb.filter(b => b.plein).length]"
        r["contreventement"] = pg.evaluate(CV)
        q = E(-8.3, 15); pg.mouse.move(*q); pg.mouse.wheel(0, -2400); pg.wait_for_timeout(300)
        for y in (8.75, 15):
            q = E(-8.3, y); pg.mouse.click(*q); pg.wait_for_timeout(250)
            r["poutre_choisie"] = pg.evaluate("[document.querySelector('#f-long').textContent, !document.querySelector('#f-mur').hidden, document.querySelector('#b-plein').getAttribute('aria-pressed')]")
            pg.click("#b-plein"); pg.wait_for_timeout(200)
        r["sans_remplissage"] = pg.evaluate(CV)
        pg.screenshot(path=str(out / f"{name}-squelette-contreventement.png"))
        for _ in range(2): pg.click("#b-annuler"); pg.wait_for_timeout(200)
        r["remplissage_retabli"] = pg.evaluate(CV)
        pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        # La trame suit les voies : rien de choisi, la fiche de la structure le dit.
        r["fiche_structure"] = pg.evaluate("[document.querySelector('#fiche').hidden, document.querySelector('#f-trame').hidden, document.querySelector('#b-trame').getAttribute('aria-pressed'), document.querySelector('#f-long').textContent, S.auto]")
        # On sépare deux parties : le nœud du bas à gauche descend, l'îlot s'ouvre. La structure s'adapte à chaque branche, sans rien toucher d'autre.
        pg.click('#calques button[data-c="0"]'); n, m = E(0, 0), E(-5, -20)
        pg.mouse.move(*n); pg.mouse.down(); pg.mouse.move((n[0] + m[0]) / 2, (n[1] + m[1]) / 2); pg.mouse.move(*m); pg.mouse.up(); pg.wait_for_timeout(300)
        pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        r["separe"] = [pg.evaluate("S.noeuds[0]"), pg.evaluate(ST), pg.evaluate(CV)]
        # Au nœud déplacé, l'angle n'est plus droit : les deux bandes piétonnes se coupent l'une sur l'autre, leurs bords extérieurs se rejoignent en un point.
        r["separe_pietons"] = pg.evaluate("(() => { const A = calc.trottoirs.find(t => t.i === 0).q[3], B = calc.trottoirs.find(t => t.i === 3).q[2]; return [document.querySelector('#s-pietons').textContent, +Math.hypot(A[0] - B[0], A[1] - B[1]).toFixed(3)]; })()")
        q = E(*pg.evaluate("S.noeuds[0]")); pg.mouse.move(*q); pg.mouse.wheel(0, -1600); pg.wait_for_timeout(300)
        pg.screenshot(path=str(out / f"{name}-squelette-carrefour-pietons.png"))
        pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        pg.screenshot(path=str(out / f"{name}-squelette-separe.png"))
        pg.click("#b-annuler"); pg.wait_for_timeout(200); pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        pg.click('#calques button[data-c="1"]'); pg.wait_for_timeout(200)
        r["separe_annule"] = [pg.evaluate(ST)["poteaux"], pg.evaluate(CV)]
        # Soixante déformations au hasard : la trame automatique ne met jamais de poteau dans une voie ni dans une place.
        r["alea"] = pg.evaluate("""(() => { let seed = 7; const rnd = () => (seed = (seed * 16807) % 2147483647) / 2147483647; const base = JSON.stringify(S), out = { essais: 0, enVoie: 0, genees: 0, reseauCoupe: 0 };
          for (let n = 0; n < 60; n++) { S = JSON.parse(base); pointer(); for (const N of S.noeuds) { N[0] += Math.round((rnd() * 30 - 15) / 2.5) * 2.5; N[1] += Math.round((rnd() * 30 - 15) / 2.5) * 2.5; } calculer(); out.essais++;
            if (calc.conflits.length) out.enVoie++; if (calc.genees.length) out.genees++; if (!/continu/.test(document.querySelector('#s-pietons').textContent)) out.reseauCoupe++; }
          S = JSON.parse(base); pointer(); rafraichir(); return out; })()""")
        n, m = E(32.5, 7.5), E(32.5, 0)
        pg.mouse.move(*n); pg.mouse.down(); pg.mouse.move((n[0] + m[0]) / 2, (n[1] + m[1]) / 2); pg.mouse.move(*m); pg.mouse.up(); pg.wait_for_timeout(300)
        r["poteau_en_voie"] = pg.evaluate(ST)
        r["a_la_main"] = pg.evaluate("[S.auto, document.querySelector('#f-long').textContent]")
        pg.screenshot(path=str(out / f"{name}-squelette-structure.png"))
        pg.click("#b-annuler"); pg.wait_for_timeout(200)
        # Une poutre de plus, tracée depuis un poteau ; puis la choisir.
        pg.click("#b-tracer"); a, c = E(47.5, 23.3), E(47.5, 30)
        pg.mouse.move(*a); pg.mouse.down(); pg.mouse.move((a[0] + c[0]) / 2, (a[1] + c[1]) / 2); pg.mouse.move(*c); pg.mouse.up(); pg.wait_for_timeout(300)
        pg.click("#b-tracer")
        r["poutre"] = pg.evaluate(ST)
        # Poser un poteau d'un toucher : il s'aimante à un coin de place ; puis porter ses poutres le long de la voie.
        pg.click("#b-tracer"); q = E(17.8, 12.4); pg.mouse.click(*q); pg.wait_for_timeout(250); pg.click("#b-tracer")
        r["pose"] = pg.evaluate("({n: S.pn.filter(Boolean).length, p: S.pn[choixN], fiche: document.querySelector('#f-long').textContent, actions: !document.querySelector('#f-portee').hidden, genees: calc.genees.length})")
        pg.click('#f-portee button[data-p="travers"]'); pg.wait_for_timeout(200)
        r["portee_travers"] = pg.evaluate("[S.pb.filter(b => b.a === choixN || b.b === choixN).map(b => +repere(b, S.pn).L.toFixed(2)), document.querySelector('#f-long').textContent]")
        # Glisser ce poteau au milieu d'une place : il s'aimante à la ligne la plus proche, la place reste.
        a, c = E(*pg.evaluate("S.pn[choixN]")), E(18.9, 10.4)
        pg.mouse.move(*a); pg.mouse.down(); pg.mouse.move((a[0] + c[0]) / 2, (a[1] + c[1]) / 2); pg.mouse.move(*c); pg.mouse.up(); pg.wait_for_timeout(300)
        r["glisse"] = pg.evaluate("({p: S.pn[choixN], genees: calc.genees.length, places: calc.pl.length})")
        pg.screenshot(path=str(out / f"{name}-squelette-trame.png"))
        pg.click("#b-suppr"); pg.wait_for_timeout(200)
        r["supprime"] = pg.evaluate("[S.pn.filter(Boolean).length, S.pb.length, document.querySelector('#fiche').hidden]")
        # Un poteau posé près de l'angle du bâtiment s'y aimante.
        pg.click("#b-tracer"); q = E(-8.0, -7.8); pg.mouse.click(*q); pg.wait_for_timeout(250); pg.click("#b-tracer")
        r["poteau_d_angle"] = pg.evaluate("S.pn[choixN]")
        pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        pg.mouse.click(5, 5); pg.wait_for_timeout(200)
        pg.screenshot(path=str(out / f"{name}-squelette-angles.png"))
        pg.click('#calques button[data-c="0"]'); pg.wait_for_timeout(200)
        r["retour_voies"] = pg.evaluate("[calque, NO === S.noeuds, document.querySelector('#fiche').hidden]")
        print(json.dumps(r, ensure_ascii=False))
    b.close()
print("erreurs:", errs)
