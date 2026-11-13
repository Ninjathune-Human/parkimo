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
        def glisser(de, vers):
            n, m = E(*de), E(*vers)
            pg.mouse.move(*n); pg.mouse.down(); pg.mouse.move((n[0] + m[0]) / 2, (n[1] + m[1]) / 2); pg.mouse.move(*m); pg.mouse.up(); pg.wait_for_timeout(300)
        # Décision 8 : entre la norme et le Neufert, la cote la plus généreuse. Allée à double sens de 6,00 m (bande piétonne
        # comprise), places en épi de 2,50 m, 5,30 m de profondeur à 60°, places en long de 5,70 m ; le module à 90° fait 16 m.
        r["cotes"] = pg.evaluate("[2 * demi(S.branches[0]), TYPES.aucun.allee, +(TYPES['60'].pas * Math.sin(Math.PI / 3)).toFixed(2), TYPES['60'].prof, +(TYPES['45'].pas * Math.sin(Math.PI / 4)).toFixed(2), TYPES['45'].prof, TYPES['45'].allee, TYPES['0'].pas, S.noeuds[2][1], calc.trottoirs.length > 0 && +Math.abs(calc.trottoirs[0].q[0][1] - calc.trottoirs[0].q[2][1]).toFixed(2)]")
        assert r["cotes"] == [6, 6, 2.5, 5.3, 2.5, 4.8, 3.5, 5.7, 16, 0.8], r["cotes"]
        base = r["depart"]["places"]
        # Trame commune (charte, section 5). Les places se comptent entre nus : une travée de deux ou trois places, puis
        # l'appui. À la marge, la dernière travée se resserre à 2,40 m ou 2,30 m si cela loge une place de plus.
        TC = pg.evaluate("""(() => { const f = (L, k, o) => { const t = travees(L, SYSTEMES[k], !!o); return [t.pl.length, t.pl.filter((p) => p.w < 2.5).map((p) => p.w), t.ap.length]; };
          return { beton60: f(60, "beton"), poteaux60: f(60, "poteaux"), voiles60: f(60, "voiles"), marge240: f(12.4, "beton"), marge230: f(12.2, "beton"), coin: f(5, "poteaux", true), acier: f(12.5, "acier") }; })()""")
        assert TC == {"beton60": [24, [], 9], "poteaux60": [22, [], 9], "voiles60": [23, [], 9], "marge240": [5, [2.4, 2.4], 3], "marge230": [5, [2.3, 2.3], 3], "coin": [2, [2.3, 2.3], 1], "acier": [5, [], 4]}, TC
        # Sur l'exemple : le système par défaut (grande portée béton) garde les places de l'essai ; chaque autre système
        # se choisit dans la fiche de la structure, ne met aucun poteau dans une place ni dans une voie, et sa portée se contrôle.
        SY = "({systeme: S.systeme, places: calc.pl.length, etroites: calc.pl.filter((p) => p.q.w < 2.5).length, poteaux: calc.pot.length, voiles: S.pb.filter((b) => b.voile).length, enVoie: calc.conflits.length, enPlace: calc.dansPlace, portee: [+calc.porteeNette.toFixed(2), calc.porteeLim], viol: calc.principes.filter((p) => !p.ok).map((p) => p.id), boutons: [...document.querySelectorAll('#f-systeme button')].map((x) => x.getAttribute('aria-pressed'))})"
        pg.click('#calques button[data-c="1"]'); pg.wait_for_timeout(200)
        r["systemes"] = [pg.evaluate(SY)]
        assert r["systemes"][0]["systeme"] == "beton" and r["systemes"][0]["places"] == base and not r["systemes"][0]["viol"], r["systemes"]
        for k in ("poteaux", "voiles", "acier"):
            pg.click(f'#f-systeme button[data-s="{k}"]'); pg.wait_for_timeout(300)
            e = pg.evaluate(SY); r["systemes"].append(e)
            assert e["systeme"] == k and e["enVoie"] == 0 and e["enPlace"] == 0 and e["places"] > 90 and not e["viol"], e  # les remplissages de l'exemple restent en place : contreventé d'un système à l'autre
            pg.screenshot(path=str(out / f"{name}-squelette-systeme-{k}.png"))
        assert r["systemes"][1]["places"] < base and r["systemes"][2]["voiles"] > 10 and r["systemes"][3]["places"] >= r["systemes"][1]["places"], r["systemes"]
        q = E(5, 5); pg.mouse.move(*q); pg.mouse.wheel(0, -1500); pg.wait_for_timeout(300)
        pg.click('#f-systeme button[data-s="voiles"]'); pg.wait_for_timeout(300)
        pg.screenshot(path=str(out / f"{name}-squelette-systeme-voiles-detail.png"))
        pg.click('#f-systeme button[data-s="poteaux"]'); pg.wait_for_timeout(300)
        pg.screenshot(path=str(out / f"{name}-squelette-systeme-poteaux-detail.png"))
        for _ in range(5): pg.click("#b-annuler"); pg.wait_for_timeout(150)
        pg.click("#b-cadrer"); pg.wait_for_timeout(300); pg.click('#calques button[data-c="0"]'); pg.wait_for_timeout(200)
        r["systeme_retabli"] = pg.evaluate("[S.systeme, calc.pl.length, document.querySelector('#s-contrev').textContent]")
        assert r["systeme_retabli"][:2] == ["beton", base], r["systeme_retabli"]
        # Rampe branchée (charte, lot 4 ; décision 7). Une voie tracée d'équerre depuis la voie du bas devient une rampe (« Toute la voie ») : c'est
        # une voie du squelette, large de 3,25 m (6,50 m en double sens), sans place, sans bande piétonne, sans épure en S.
        # Le réseau piéton reste d'un seul tenant en traversant son entrée ; sa pente se contrôle (15 % au plus).
        RV = "(() => { const b = S.branches[S.branches.length - 1], i = S.branches.length - 1; return { rampe: b.rampe || null, sens: b.sens || null, largeur: +(2 * demi(b)).toFixed(2), places: calc.pl.length, surElle: calc.pl.filter((p) => p.i === i).length, bande: bandeDe(b), epures: calc.rampes.length, pietons: document.querySelector('#s-pietons').textContent, bilan: document.querySelector('#s-rampes').textContent, viol: calc.principes.filter((p) => !p.ok).map((p) => p.id), lignes: [...document.querySelectorAll('#fiche .ligne')].filter((l) => !l.hidden).map((l) => l.firstElementChild.textContent), fiche: document.querySelector('#f-long').textContent }; })()"
        pg.click("#b-tracer"); glisser((30, 0), (30, -30)); pg.click("#b-tracer")
        pg.click('#fiche .seg[data-cote="rampe"] button[data-t="voie"]'); pg.wait_for_timeout(300)
        e = pg.evaluate(RV); r["rampe_voie"] = [e]
        assert e["rampe"] == "droite" and e["sens"] == 1 and e["largeur"] == 3.25 and e["surElle"] == 0 and e["bande"] == "aucun" and e["epures"] == 0 and "continu" in e["pietons"] and not e["viol"] and e["lignes"] == ["Rampe", "Sens"] and "pente 10,4 %" in e["fiche"], e
        pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        pg.screenshot(path=str(out / f"{name}-squelette-rampe-voie.png"))
        pg.click('#fiche .seg[data-cote="sens"] button[data-t="2"]'); pg.wait_for_timeout(300)
        e = pg.evaluate(RV); r["rampe_voie"].append(e)
        assert e["sens"] == 2 and e["largeur"] == 6.5 and not e["viol"] and e["places"] < r["rampe_voie"][0]["places"], e
        pg.click('#fiche .seg[data-cote="rampe"] button[data-t="demi"]'); pg.wait_for_timeout(300)
        e = pg.evaluate(RV); r["rampe_voie"].append(e)
        assert e["rampe"] == "demi" and e["epures"] == 0 and "pente 5,2 %" in e["fiche"], e
        pg.click('#fiche .seg[data-cote="rampe"] button[data-t="droite"]'); pg.wait_for_timeout(300)
        # Trop courte pour un étage : la pente dépasse 15 %, le principe est violé.
        r["rampe_raide"] = pg.evaluate("(() => { const k = S.branches[S.branches.length - 1].b, y = S.noeuds[k][1]; S.noeuds[k][1] = -12.5; calculer(); const o = [calc.principes.filter((p) => !p.ok).map((p) => p.id), calc.principes.find((p) => p.id === 'pente').detail]; S.noeuds[k][1] = y; calculer(); return o; })()")
        assert r["rampe_raide"][0] == ["pente"], r["rampe_raide"]
        # Redevenue une voie : elle reprend sa largeur et sa bande.
        pg.click('#fiche .seg[data-cote="rampe"] button[data-t="voie"]'); pg.wait_for_timeout(300)
        e = pg.evaluate(RV); r["rampe_voie"].append(e)
        assert e["rampe"] is None and e["largeur"] == 6 and e["lignes"] == ["Gauche", "Droite", "Piétons", "Rampe"], e
        for _ in range(6): pg.click("#b-annuler"); pg.wait_for_timeout(150)
        pg.click("#b-cadrer"); pg.wait_for_timeout(300); pg.mouse.click(5, 5); pg.wait_for_timeout(200)
        r["rampe_voie_annulee"] = pg.evaluate("[S.branches.length, calc.pl.length, calc.principes.length]")
        assert r["rampe_voie_annulee"] == [5, base, 10], r["rampe_voie_annulee"]
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
        glisser((60, 0), (65, 0.4))
        r["antenne_suit"] = pg.evaluate("S.noeuds.map(p => p.map(v => +v.toFixed(2)))")
        pg.click("#b-annuler"); pg.wait_for_timeout(200)
        glisser((0, 0), (-5, 0.4))
        r["boucle_reste"] = pg.evaluate("S.noeuds.map(p => p.map(v => +v.toFixed(2)))")
        pg.click("#b-annuler"); pg.wait_for_timeout(200)
        r["suite_annulee"] = pg.evaluate("S.noeuds.map(p => p.map(v => +v.toFixed(2)))")
        # Aimantation des voies : des places à 45° côté îlot laissent un vide de 0,20 m entre les deux rangées dos à dos.
        # En glissant la voie, elle s'aimante là où les deux fonds de rangée se touchent (0,20 m), hors du pas de 2,50 m ;
        # l'antenne suit. Plus loin, sans alignement à portée, le pas de 2,50 m reprend.
        q = E(30, 0); pg.mouse.click(*q); pg.wait_for_timeout(250)
        pg.click('#fiche .seg[data-cote="g"] button[data-t="45"]'); pg.wait_for_timeout(200)
        pg.mouse.click(5, 5); pg.wait_for_timeout(200)
        q = E(30, 0); pg.mouse.move(*q); pg.mouse.wheel(0, -1200); pg.wait_for_timeout(300)
        glisser((30, 0), (30, 0.7))
        r["ilot_aimante"] = pg.evaluate("S.noeuds.map(p => p.map(v => +v.toFixed(2)))")
        glisser((30, 0.2), (30, -5.0))
        r["voie_au_pas"] = pg.evaluate("S.noeuds[0].map(v => +v.toFixed(2))")
        for _ in range(3): pg.click("#b-annuler"); pg.wait_for_timeout(200)
        pg.click("#b-cadrer"); pg.wait_for_timeout(300); pg.mouse.click(5, 5); pg.wait_for_timeout(200)
        # Logique d'ensemble : en « auto », les bandes longent les façades et font le tour. Une seconde boucle tracée
        # sous la première : ses trois voies portent leur bande côté façade, le réseau reste continu.
        r["bandes_auto"] = pg.evaluate("S.branches.map(bandeDe)")
        # Passages piétons : d'équerre sur la voie traversée. Celui de l'entrée de l'antenne tient entre deux abscisses.
        r["passage_droit"] = pg.evaluate("[...new Set(calc.zebres.flat().map(p => +p[0].toFixed(2)))].sort((a, b) => a - b)")
        pg.click("#b-tracer")
        for de, vers in (((0, 0), (0, -17.5)), ((0, -17.5), (60, -17.5)), ((60, -17.5), (60, 0))):
            glisser(de, vers); pg.mouse.click(5, 5); pg.wait_for_timeout(200)  # la fiche de la voie tracée se replie avant le tracé suivant
        pg.click("#b-tracer"); pg.mouse.click(5, 5); pg.wait_for_timeout(200); pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        r["boucle_basse"] = [pg.evaluate("S.branches.map(bandeDe)"), pg.evaluate("[S.branches.length, document.querySelector('#s-pietons').textContent, calc.zebres.length, calc.pl.length]")]
        pg.screenshot(path=str(out / f"{name}-squelette-deux-boucles.png"))
        for _ in range(3): pg.click("#b-annuler"); pg.wait_for_timeout(200)
        pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        # Bâtiment en anneau autour d'une cour : les bandes font le tour côté façade extérieure, sans aucune traversée,
        # quel que soit le sens dans lequel chaque voie a été tracée (celle du haut est tracée à l'envers).
        pg.evaluate("""(() => { garder(); const v = (a, b) => ({ a, b, g: "90", d: "90", pieton: "auto" });
          S.noeuds = [[0, 0], [60, 0], [60, 45], [0, 45]]; S.branches = [v(0, 1), v(1, 2), v(3, 2), v(3, 0)]; pointer(); rafraichir(); })()"""); pg.wait_for_timeout(200)
        pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        r["anneau"] = pg.evaluate("[S.branches.map(bandeDe), calc.zebres.length, document.querySelector('#s-pietons').textContent]")
        pg.screenshot(path=str(out / f"{name}-squelette-anneau.png"))
        pg.click("#b-annuler"); pg.wait_for_timeout(200); pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        # Rampes, par typologie. Une rampe droite ajoutée depuis la fiche d'une voie prend, dans la rangée, ses 20 m
        # plus une giration en S à chaque bout (8,46 m) : seize places en moins, aucun poteau dans la voie de rampe ni
        # dans les épures. On la glisse où l'on veut, elle s'aimante à la rangée la plus proche qui peut la recevoir.
        # Une seconde, dos à dos : couplées ; glissée ailleurs : séparées. On la choisit, on la supprime.
        RP = "[S.rampes.length, calc.rampes.map(x => [x.i, x.c, x.a]), calc.pl.length, calc.conflits.length, calc.genees.length, document.querySelector('#s-rampes').textContent]"
        q = E(30, 0); pg.mouse.click(*q); pg.wait_for_timeout(250)
        pg.click('#fiche .seg[data-cote="rampe"] button[data-t="droite"]'); pg.wait_for_timeout(250)
        r["rampe"] = [pg.evaluate(RP), pg.evaluate("document.querySelector('#f-long').textContent")]
        # Un poteau posé à la main dans une épure de giration est signalé comme dans une voie.
        r["rampe_epure"] = pg.evaluate("(() => { const x = calc.rampes[0], e = x.epures[0].axe, p = [(e[3][0] + e[4][0]) / 2, (e[3][1] + e[4][1]) / 2]; S.auto = false; S.pn.push(p); calculer(); const n = calc.conflits.length; S.pn.pop(); S.auto = true; calculer(); return [x.epures.length, +GIR.s.toFixed(2), n, calc.conflits.length]; })()")
        glisser((30, 5), (30, 10))
        r["rampe_glissee"] = pg.evaluate(RP)
        q = E(30, 0); pg.mouse.click(*q); pg.wait_for_timeout(250)
        pg.click('#fiche .seg[data-cote="rampe"] button[data-t="demi"]'); pg.wait_for_timeout(250)
        r["rampes_couplees"] = pg.evaluate(RP)
        pg.screenshot(path=str(out / f"{name}-squelette-rampes.png"))
        glisser((30, 5), (10, -5))
        r["rampes_separees"] = pg.evaluate(RP)
        q = E(30, 10); pg.mouse.click(*q); pg.wait_for_timeout(250)
        r["rampe_choisie"] = pg.evaluate("document.querySelector('#f-long').textContent")
        pg.click("#b-suppr"); pg.wait_for_timeout(250)
        r["rampe_supprimee"] = pg.evaluate(RP)
        for _ in range(5): pg.click("#b-annuler"); pg.wait_for_timeout(200)
        r["rampes_annulees"] = pg.evaluate("[(S.rampes || []).length, calc.pl.length]")
        pg.mouse.click(5, 5); pg.wait_for_timeout(200)
        # Registre des principes (charte, section 8) : tous respectés sur l'exemple, affichés au bilan.
        PR = "[document.querySelector('#s-principes').textContent, calc.principes.filter(p => !p.ok).map(p => p.id)]"
        r["principes"] = [pg.evaluate(PR), pg.evaluate("calc.principes.length")]
        # Arbitrage : un poteau forcé dans une place ne la retire pas, c'est lui qui est signalé ; une voie qui en croise
        # une autre sans nœud, et un îlot trop étroit pour ses deux rangées, sont des principes violés.
        r["arbitrage_poteau"] = pg.evaluate("(() => { const n = calc.pl.length; S.auto = false; S.pn.push([34.25, 5.5]); calculer(); const o = [n, calc.pl.length, calc.dansPlace, calc.principes.filter(p => !p.ok).map(p => p.id)]; S.pn.pop(); S.auto = true; calculer(); return o; })()")
        r["arbitrage_voies"] = pg.evaluate("(() => { const base = JSON.stringify(S); S.noeuds.push([30, -20], [30, 6]); S.branches.push({ a: 5, b: 6, g: 'aucun', d: 'aucun', pieton: 'aucun' }); pointer(); calculer(); const a = [calc.croisements, calc.principes.filter(p => !p.ok).map(p => p.id)]; S = JSON.parse(base); pointer(); S.noeuds[2][1] = S.noeuds[3][1] = 12.5; calculer(); const b = [calc.chevauchements, calc.pl.length, calc.principes.filter(p => !p.ok).map(p => p.id)]; S = JSON.parse(base); pointer(); rafraichir(); return [a, b]; })()")
        # Décision 2 : une rampe posée dans la rangée de façade ; la bande piétonne de cette voie bascule hors des girations.
        q = E(30, 0); pg.mouse.click(*q); pg.wait_for_timeout(250)
        pg.click('#fiche .seg[data-cote="rampe"] button[data-t="droite"]'); pg.wait_for_timeout(250)
        glisser((30, 5), (30, -5))
        r["bande_hors_giration"] = [pg.evaluate("[calc.rampes[0].i, calc.rampes[0].c, bandeDe(S.branches[0])]"), pg.evaluate(PR), pg.evaluate("calc.zebres.length")]
        for _ in range(2): pg.click("#b-annuler"); pg.wait_for_timeout(200)
        pg.mouse.click(5, 5); pg.wait_for_timeout(200)
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
        # Toucher hors du dessin : tout se désélectionne et la fiche se replie, dans les deux calques et en mode Tracer.
        F = "[document.querySelector('#fiche').hidden, choix, choixN]"
        q = E(30, 0); pg.mouse.click(*q); pg.wait_for_timeout(250); avant = pg.evaluate(F)
        q = E(30, 35); pg.mouse.click(*q); pg.wait_for_timeout(250)
        r["hors_voies"] = [avant, pg.evaluate(F)]
        q = E(30, 0); pg.mouse.click(*q); pg.wait_for_timeout(250); pg.click("#b-tracer")
        q = E(30, 35); pg.mouse.click(*q); pg.wait_for_timeout(250)
        r["hors_tracer"] = pg.evaluate(F); pg.click("#b-tracer")
        pg.click('#calques button[data-c="1"]'); pg.wait_for_timeout(200); avant = pg.evaluate(F)
        q = E(30, 35); pg.mouse.click(*q); pg.wait_for_timeout(250); dehors = pg.evaluate(F)
        q = E(29.25, 0); pg.mouse.move(*q); pg.mouse.wheel(0, -1800); pg.wait_for_timeout(300)
        q = E(29.25, 0); pg.mouse.click(*q); pg.wait_for_timeout(250)
        r["hors_poteaux"] = [avant, dehors, pg.evaluate(F)]
        pg.click("#b-cadrer"); pg.wait_for_timeout(300); pg.click('#calques button[data-c="0"]'); pg.wait_for_timeout(200)
        # Tracer une branche depuis le milieu de la branche du bas, vers le bas : un nœud se crée sur la branche.
        pg.click("#b-tracer")
        a, c = E(30, 0), E(30, -30)
        pg.mouse.move(*a); pg.mouse.down(); pg.mouse.move((a[0] + c[0]) / 2, (a[1] + c[1]) / 2); pg.mouse.move(*c); pg.mouse.up(); pg.wait_for_timeout(300)
        r["trace"] = pg.evaluate(ETAT)
        # La voie tracée reçoit d'office une bande piétonne, du côté qui rejoint celles des voies qu'elle touche : le réseau reste continu.
        r["trace_pietons"] = pg.evaluate("[S.branches[S.branches.length - 1].pieton || null, document.querySelector('#s-pietons').textContent, calc.trottoirs.length]")
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
        q = E(-5.5, -8); pg.mouse.move(*q); pg.mouse.wheel(0, -1200); pg.wait_for_timeout(300)  # de près : la travée fait 5 m entre deux poteaux
        q = E(-5.5, -8); pg.mouse.click(*q); pg.wait_for_timeout(250)
        r["poutre_choisie"] = pg.evaluate("[document.querySelector('#f-long').textContent, !document.querySelector('#f-mur').hidden, document.querySelector('#b-plein').getAttribute('aria-pressed')]")
        pg.click("#b-plein"); pg.wait_for_timeout(200)
        r["sans_remplissage"] = pg.evaluate(CV)
        pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        pg.screenshot(path=str(out / f"{name}-squelette-contreventement.png"))
        pg.click("#b-annuler"); pg.wait_for_timeout(200)
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
            if (calc.conflits.length) out.enVoie++; if (calc.genees.length) out.genees++; if (!/continu/.test(document.querySelector('#s-pietons').textContent)) out.reseauCoupe++;
            for (const p of calc.principes) if (!p.ok) out[p.id] = (out[p.id] || 0) + 1; }
          S = JSON.parse(base); pointer(); rafraichir(); return out; })()""")
        n, m = E(33, 8), E(33, 0)
        pg.mouse.move(*n); pg.mouse.down(); pg.mouse.move((n[0] + m[0]) / 2, (n[1] + m[1]) / 2); pg.mouse.move(*m); pg.mouse.up(); pg.wait_for_timeout(300)
        r["poteau_en_voie"] = pg.evaluate(ST)
        r["a_la_main"] = pg.evaluate("[S.auto, document.querySelector('#f-long').textContent]")
        r["principe_viole"] = pg.evaluate(PR)
        # Décision 1 : un poteau lâché dans une place ne la condamne pas, il gagne la ligne la plus proche.
        pg.click("#b-annuler"); pg.wait_for_timeout(200)
        q = E(33, 8); pg.mouse.move(*q); pg.mouse.wheel(0, -1500); pg.wait_for_timeout(300)
        glisser((33, 8), (34.25, 5.5))
        r["poteau_sur_ligne"] = pg.evaluate("[S.pn[choixN], calc.genees.length]")
        pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        q = E(33, 8); pg.mouse.click(*q); pg.wait_for_timeout(200)
        pg.screenshot(path=str(out / f"{name}-squelette-structure.png"))
        pg.click("#b-annuler"); pg.wait_for_timeout(200)
        # Une poutre de plus, tracée depuis un poteau ; puis la choisir.
        pg.click("#b-tracer"); a, c = E(48, 24), E(48, 31.5)
        pg.mouse.move(*a); pg.mouse.down(); pg.mouse.move((a[0] + c[0]) / 2, (a[1] + c[1]) / 2); pg.mouse.move(*c); pg.mouse.up(); pg.wait_for_timeout(300)
        pg.click("#b-tracer")
        r["poutre"] = pg.evaluate(ST)
        # Poser un poteau d'un toucher : il s'aimante à un coin de place ; puis porter ses poutres le long de la voie.
        pg.click("#b-tracer"); q = E(17.8, 12.9); pg.mouse.click(*q); pg.wait_for_timeout(250); pg.click("#b-tracer")
        r["pose"] = pg.evaluate("({n: S.pn.filter(Boolean).length, p: S.pn[choixN], fiche: document.querySelector('#f-long').textContent, actions: !document.querySelector('#f-portee').hidden, genees: calc.genees.length})")
        pg.click('#f-portee button[data-p="travers"]'); pg.wait_for_timeout(200)
        r["portee_travers"] = pg.evaluate("[S.pb.filter(b => b.a === choixN || b.b === choixN).map(b => +repere(b, S.pn).L.toFixed(2)), document.querySelector('#f-long').textContent]")
        # Glisser ce poteau au milieu d'une place : il s'aimante à la ligne la plus proche, la place reste.
        a, c = E(*pg.evaluate("S.pn[choixN]")), E(19.2, 10.6)
        pg.mouse.move(*a); pg.mouse.down(); pg.mouse.move((a[0] + c[0]) / 2, (a[1] + c[1]) / 2); pg.mouse.move(*c); pg.mouse.up(); pg.wait_for_timeout(300)
        r["glisse"] = pg.evaluate("({p: S.pn[choixN], genees: calc.genees.length, places: calc.pl.length})")
        pg.screenshot(path=str(out / f"{name}-squelette-trame.png"))
        pg.click("#b-suppr"); pg.wait_for_timeout(200)
        r["supprime"] = pg.evaluate("[S.pn.filter(Boolean).length, S.pb.length, document.querySelector('#fiche').hidden]")
        # Un poteau posé près de l'angle du bâtiment s'y aimante.
        pg.click("#b-tracer"); q = E(-7.7, -7.5); pg.mouse.click(*q); pg.wait_for_timeout(250); pg.click("#b-tracer")
        r["poteau_d_angle"] = pg.evaluate("S.pn[choixN]")
        pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        pg.mouse.click(5, 5); pg.wait_for_timeout(200)
        pg.screenshot(path=str(out / f"{name}-squelette-angles.png"))
        pg.click('#calques button[data-c="0"]'); pg.wait_for_timeout(200)
        r["retour_voies"] = pg.evaluate("[calque, NO === S.noeuds, document.querySelector('#fiche').hidden]")
        print(json.dumps(r, ensure_ascii=False))
    b.close()
print("erreurs:", errs)
