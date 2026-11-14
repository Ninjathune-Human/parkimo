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
        # Surface du plan (iPhone) : le bilan est replié au départ (une ligne : places et principes), il se déplie d'un toucher
        # sur son titre, la barre basse tient sur peu de hauteur, et l'aide s'efface au premier toucher du plan.
        H_ = "(() => { const q = (s) => document.querySelector(s).getBoundingClientRect(); return { bilan: Math.round(q('#bilan').height), ouvert: document.querySelector('#bilan-det').open, barre: Math.round(q('#barre').height), resume: document.querySelector('#s-resume').textContent, aide: getComputedStyle(document.querySelector('#aide')).display }; })()"
        r["surface"] = pg.evaluate(H_)
        if name == "iphone":
            assert not r["surface"]["ouvert"] and r["surface"]["bilan"] <= 64 and r["surface"]["barre"] <= 100 and r["surface"]["resume"] == "11 sur 11", r["surface"]
            assert r["surface"]["aide"] != "none", r["surface"]
            pg.screenshot(path=str(out / "iphone-replie.png"))
            pg.click("#bilan-det > summary"); pg.wait_for_timeout(200)
            ouvert = pg.evaluate(H_); assert ouvert["ouvert"] and ouvert["bilan"] > 200, ouvert
            pg.click("#bilan-det > summary"); pg.wait_for_timeout(200)
            assert not pg.evaluate("document.querySelector('#bilan-det').open")
            pg.mouse.click(195, 500); pg.wait_for_timeout(200)
            assert pg.evaluate(H_)["aide"] == "none", "l'aide reste affichée après un toucher du plan"
            pg.click('#calques button[data-c="1"]'); pg.wait_for_timeout(150)
            assert not pg.evaluate("document.querySelector('#aide').classList.contains('lu')"), "l'aide ne revient pas au changement de calque"
            pg.click('#calques button[data-c="0"]'); pg.wait_for_timeout(150)
            vu = pg.evaluate("(() => { const b = document.querySelector('#bilan').getBoundingClientRect().bottom, k = document.querySelector('#barre').getBoundingClientRect().top; cadrer(); const pts = calc.ch.flat().map(versE); return [Math.min(...pts.map((p) => p[1])) >= b, Math.max(...pts.map((p) => p[1])) <= k]; })()")
            assert vu == [True, True], vu
        else:
            assert r["surface"]["ouvert"], r["surface"]
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
        assert r["rampe_voie_annulee"] == [5, base, 11], r["rampe_voie_annulee"]
        # Circulations verticales (ERP, articles PS 13 et PS 24). Un escalier est un noyau posé dans une rangée : deux places,
        # quatre pour l'escalier principal qui loge deux ascenseurs. La distance se mesure dans l'axe des voies, de l'axe de la
        # place à la porte (article PS 13 § 1) : 40 m au plus entre deux escaliers, 25 m sinon.
        ES = "({n: calc.escaliers.length, principaux: calc.escaliers.filter((x) => x.principal).length, longs: calc.escaliers.map((x) => x.L), hors: calc.escaliers.filter((x) => x.hors).length, max: +calc.distMax.toFixed(1), loin: calc.loin.length, viol: calc.principes.filter((p) => !p.ok).map((p) => p.id), bilan: document.querySelector('#s-escaliers').textContent, places: calc.pl.length, murs: S.pb.filter((b) => b.noyau).length, contrev: document.querySelector('#s-contrev').textContent, fiche: document.querySelector('#fiche').hidden ? null : document.querySelector('#f-long').textContent})"
        e = pg.evaluate(ES); r["escaliers"] = [e]
        assert e["n"] >= 2 and e["principaux"] == 1 and sorted(set(e["longs"])) == [5, 10] and e["hors"] == 0 and 0 < e["max"] <= 40 and e["loin"] == 0 and not e["viol"] and e["murs"] >= 8 and e["places"] == base, e
        # La place la plus proche d'un escalier : le long de l'axe de la voie, puis la demi-largeur de la voie jusqu'à la porte.
        r["escalier_proche"] = pg.evaluate("(() => { const x = calc.escaliers.find((o) => TYPES[S.branches[o.i][o.c]].angle === 90), o = calc.pl.filter((p) => p.i === x.i && p.c === x.c).sort((p, q) => p.dist - q.dist)[0]; return [+o.dist.toFixed(2), o.limite]; })()")
        assert 2.99 < r["escalier_proche"][0] < 8, r["escalier_proche"]
        # Placement automatique : le plus petit nombre d'escaliers qui dessert toutes les places, puis le moins de passages.
        # Sur l'exemple : trois escaliers sans passage, deux avec un passage à travers l'îlot.
        AU = "({auto: S.escAuto, n: calc.escaliers.length, principaux: calc.escaliers.filter((x) => x.principal).length, loin: calc.loin.length, hors: calc.escHors, max: +calc.distMax.toFixed(1), viol: calc.principes.filter((p) => !p.ok).map((p) => p.id), p: S.escaliers.map((x) => x.p.map((v) => +v.toFixed(2))), ms: Math.round(placer.ms), abrege: placer.abrege, k: placer.k, passages: S.passages.length})"
        au = pg.evaluate(AU); r["auto"] = [au]
        assert au["auto"] is True and au["k"] == [3, 2] and au["n"] == 2 and au["passages"] == 1 and au["principaux"] == 1 and au["loin"] == 0 and au["hors"] == 0 and not au["viol"] and au["ms"] < 1500 and au["abrege"] is False, au
        # Aucun n'est de trop : sans l'un d'eux, des places ne sont plus desservies.
        r["auto_minimal"] = pg.evaluate("(() => { const g = JSON.stringify(S), out = []; for (let k = 0; k < JSON.parse(g).escaliers.length; k++) { S = JSON.parse(g); pointer(); S.escAuto = false; S.escaliers.splice(k, 1); if (!S.escaliers.some((x) => x.principal)) S.escaliers[0].principal = true; calculer(); out.push(calc.loin.length); } S = JSON.parse(g); pointer(); calculer(); return out; })()")
        assert all(x > 0 for x in r["auto_minimal"]), r["auto_minimal"]
        # Le plan change, les escaliers suivent : l'antenne allongée en reçoit ce qu'il faut, aucune place ne reste à découvert.
        r["auto_suit"] = pg.evaluate("(() => { const g = JSON.stringify(S); S.noeuds[4][0] = 140; calculer(); const x = [calc.escaliers.length, calc.loin.length, calc.escHors, calc.principes.filter((p) => !p.ok).map((p) => p.id).filter((k) => k === 'escaliers').length, Math.round(placer.ms)]; S = JSON.parse(g); pointer(); calculer(); return x; })()")
        assert r["auto_suit"][0] > au["n"] and r["auto_suit"][1:4] == [0, 0, 0], r["auto_suit"]
        assert pg.evaluate(AU)["p"] == au["p"]
        # Trois calques : Voies, Poteaux, Escaliers. Les escaliers ne se manient que dans le leur, et ne se posent
        # que là : la ligne « Piétons » d'une voie n'a plus de bouton d'escalier.
        k = pg.evaluate("calc.escaliers.findIndex((o) => !o.principal)")
        de = pg.evaluate(f"centre(calc.escaliers[{k}].corps)"); vers = pg.evaluate("(() => { const x = calc.escaliers.find((o) => o.principal), c = centre(x.corps); return [c[0] + x.r.u[0] * 12.5, c[1] + x.r.u[1] * 12.5]; })()")
        q = E(*de); pg.mouse.click(*q); pg.wait_for_timeout(250)
        r["calques"] = pg.evaluate("[[...document.querySelectorAll('#calques button')].map((b) => b.textContent), document.querySelectorAll('#fiche .seg[data-cote=\"pieton\"] button').length, choixE]")
        assert r["calques"] == [["Voies", "Poteaux", "Escaliers"], 4, -1], r["calques"]
        pg.mouse.click(5, 5); pg.wait_for_timeout(200)
        pg.click('#calques button[data-c="2"]'); pg.wait_for_timeout(300)
        # Le réseau des escaliers : un nœud à la porte de chacun, des liens d'escalier à escalier par l'axe des voies,
        # et de chaque escalier un lien jusqu'à la place la plus éloignée qu'il dessert.
        LI = """(() => { const l = calc.liens, R = (x) => +x.toFixed(1), long = (p) => p.slice(1).reduce((t, q, k) => t + Math.hypot(q[0] - p[k][0], q[1] - p[k][1]), 0), pres = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]) < 0.01;
          return { noeuds: l.noeuds.length, entre: l.entre.map((x) => [x.de, x.vers, R(x.long), R(long(x.pts)), pres(x.pts[0], l.noeuds[x.vers]) && pres(x.pts[x.pts.length - 1], l.noeuds[x.de])]),
            places: l.places.map((x) => [x.j, R(x.dist), R(long(x.pts)), calc.pl.some((o) => { const c = centre(o.q), d = axePlace(o.q), v = [x.pts[0][0] - c[0], x.pts[0][1] - c[1]]; return Math.abs(v[0] * d[1] - v[1] * d[0]) < 0.01 && Math.hypot(...v) < 8; }) && pres(x.pts[x.pts.length - 1], l.noeuds[x.j])]), max: R(calc.distMax),
            barre: (() => { const b = document.querySelector('#barre').getBoundingClientRect(), f = [...document.querySelectorAll('#barre button')]; return [b.left >= 0 && b.right <= innerWidth, f.every((x) => x.scrollWidth <= x.clientWidth), f.every((x) => x.getBoundingClientRect().height >= 40)]; })() }; })()"""
        li = pg.evaluate(LI); r["liens"] = li; n = e["n"]
        assert li["noeuds"] == n and len(li["places"]) == n and sorted(x[0] for x in li["places"]) == list(range(n)), li
        assert set(sum(([x[0], x[1]] for x in li["entre"]), [])) == set(range(n)) and all(x[4] and abs(x[2] - x[3]) < 0.05 for x in li["entre"]), li
        assert all(x[3] and x[1] <= 40 and abs(x[1] - x[2]) < 0.05 for x in li["places"]) and max(x[1] for x in li["places"]) == li["max"], li
        assert li["barre"] == [True, True, True], li["barre"]
        pg.screenshot(path=str(out / f"{name}-squelette-escaliers.png"))
        # Rien de choisi : la fiche du placement, « Placement automatique » enfoncé.
        FA = "[document.querySelector('#fiche').hidden, document.querySelector('#f-escauto').hidden, document.querySelector('#b-escauto').getAttribute('aria-pressed'), document.querySelector('#f-long').textContent, document.querySelector('#b-suppr').hidden]"
        r["fiche_auto"] = [pg.evaluate(FA)]
        assert r["fiche_auto"][0][:3] == [False, False, "true"] and r["fiche_auto"][0][4] is True and "suit le plan" in r["fiche_auto"][0][3], r["fiche_auto"]
        # Dans ce calque, les voies ne se saisissent pas : glisser sur un nœud ne déplace que la vue.
        glisser([0, 0], [5, 5]); pg.click("#b-cadrer"); pg.wait_for_timeout(300)
        r["voie_inerte"] = pg.evaluate("[S.noeuds[0], choix]")
        assert r["voie_inerte"] == [[0, 0], -1], r["voie_inerte"]
        # Un escalier glissé contre l'escalier principal : des places passent hors distance, le principe est violé.
        glisser(de, vers)
        e = pg.evaluate(ES); r["escaliers"].append(e)
        assert e["loin"] > 0 and e["viol"] == ["escaliers"] and e["hors"] == 0, e
        pg.screenshot(path=str(out / f"{name}-squelette-escalier-loin.png"))
        # Un escalier glissé à la main : le placement ne suit plus le plan. « Placement automatique » le rétablit.
        q = E(30, 8); pg.mouse.click(*q); pg.wait_for_timeout(250)
        r["fiche_auto"].append(pg.evaluate(FA))
        assert pg.evaluate("S.escAuto") is False and r["fiche_auto"][1][2] == "false" and "à la main" in r["fiche_auto"][1][3], r["fiche_auto"]
        pg.click("#b-escauto"); pg.wait_for_timeout(300)
        e = pg.evaluate(AU); r["auto"].append(e)
        assert e["auto"] is True and e["p"] == au["p"] and e["loin"] == 0, e
        # On le touche : sa fiche. « Escalier principal » lui donne les deux ascenseurs, et les retire à l'autre.
        q = E(*de); pg.mouse.click(*q); pg.wait_for_timeout(250)
        r["escalier_choisi"] = pg.evaluate("[document.querySelector('#f-long').textContent, document.querySelector('#b-principal').getAttribute('aria-pressed'), [...document.querySelectorAll('#fiche .ligne')].filter((l) => !l.hidden).length, (() => { const f = document.querySelector('#fiche').getBoundingClientRect(), b = document.querySelector('#barre').getBoundingClientRect(); return f.bottom <= b.top; })()]")
        assert r["escalier_choisi"][0].startswith("Escalier") and r["escalier_choisi"][1:] == ["false", 0, True], r["escalier_choisi"]
        pg.click("#b-principal"); pg.wait_for_timeout(250)
        e = pg.evaluate(ES); r["escaliers"].append(e)
        assert e["principaux"] == 1 and pg.evaluate(f"[calc.escaliers[{k}].principal, calc.escaliers[{k}].L, S.escAuto]") == [True, 10, False] and "principal" in e["fiche"], e
        pg.click("#b-annuler"); pg.wait_for_timeout(200)
        q = E(*de); pg.mouse.click(*q); pg.wait_for_timeout(250); pg.click("#b-suppr"); pg.wait_for_timeout(250)
        e = pg.evaluate(ES); r["escaliers"].append(e)
        assert e["n"] == r["escaliers"][0]["n"] - 1 and e["principaux"] == 1, e
        pg.click("#b-annuler"); pg.wait_for_timeout(200)
        # « Tracer », puis un toucher dans une rangée : un escalier de plus. Hors de toute rangée, rien ne se pose.
        pg.click("#b-tracer"); q = E(40, -5.5); pg.mouse.click(*q); pg.wait_for_timeout(250)
        e = pg.evaluate(ES); r["escaliers"].append(e)
        assert e["n"] == r["escaliers"][0]["n"] + 1 and e["principaux"] == 1 and e["hors"] == 0 and e["fiche"].startswith("Escalier"), e
        q = E(40, -40); pg.mouse.click(*q); pg.wait_for_timeout(250)
        assert pg.evaluate("calc.escaliers.length") == e["n"]
        pg.click("#b-tracer"); pg.click("#b-annuler"); pg.wait_for_timeout(200)
        # Sans aucun escalier, toutes les places sont non desservies : en rouge dans ce calque.
        r["rouges"] = pg.evaluate("(() => { const g = S.escaliers; S.escAuto = false; S.escaliers = []; calculer(); const x = [rouges().length === calc.pl.length, calc.liens.noeuds.length]; S.escaliers = g; S.escAuto = true; calculer(); return [...x, rouges().length]; })()")
        assert r["rouges"] == [True, 0, 0], r["rouges"]
        pg.click('#calques button[data-c="0"]'); pg.wait_for_timeout(200); pg.mouse.click(5, 5); pg.wait_for_timeout(200)
        r["escaliers_retablis"] = pg.evaluate(ES)
        # Sans aucun escalier, des places ne sont desservies par rien : le principe est violé, sans teinter tout le plan.
        r["escalier_aucun"] = pg.evaluate("(() => { const g = S.escaliers; S.escAuto = false; S.escaliers = []; calculer(); const x = [calc.principes.filter((p) => !p.ok).map((p) => p.id), document.querySelector('#s-escaliers').textContent, calc.loin.length]; S.escaliers = g; S.escAuto = true; calculer(); return x; })()")
        assert r["escalier_aucun"] == [["escaliers"], "aucun", 0], r["escalier_aucun"]
        assert r["escaliers_retablis"]["n"] == r["escaliers"][0]["n"] and r["escaliers_retablis"]["places"] == base, r["escaliers_retablis"]
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
        # Emprise : un rectangle tracé reçoit une première implantation, la meilleure parmi plusieurs, que les modules affinent ensuite.
        EM = """(() => { const o = {}; const dedans = (r) => { const P = [...calc.ch, ...calc.bandes, ...calc.pl.map((p) => p.q)].flat(); return Math.min(...P.map((p) => p[0])) >= r[0] - 0.05 && Math.max(...P.map((p) => p[0])) <= r[2] + 0.05 && Math.min(...P.map((p) => p[1])) >= r[1] - 0.05 && Math.max(...P.map((p) => p[1])) <= r[3] + 0.05; };
          const sauve = JSON.stringify(S);
          for (const [l, w] of [[60, 32], [32, 60], [90, 48], [40, 17], [25, 70], [70, 20]]) { const r = [10, -5, 10 + l, -5 + w], n = emprise(r); o[l + "x" + w] = n ? { places: calc.pl.length, dedans: dedans(r), viol: calc.principes.filter((p) => !p.ok).map((p) => p.id), escaliers: calc.escaliers.length, loin: calc.loin.length, par: +(l * w / calc.pl.length).toFixed(1), emprise: S.emprise } : null; }
          o.petit = emprise([0, 0, 10, 10]) ? "applique" : "refuse"; o.intact = JSON.stringify(S) !== sauve ? "modifie" : "intact";
          S = JSON.parse(sauve); pointer(); calculer(); return o; })()"""
        r["emprise"] = pg.evaluate(EM)
        for k, v in r["emprise"].items():
            if k in ("petit", "intact"): continue
            assert v and v["dedans"] and not v["viol"] and v["loin"] == 0 and v["escaliers"] >= 1 and v["par"] <= 36, (k, v)
        assert r["emprise"]["petit"] == "refuse", r["emprise"]
        # Le geste : le bouton « Emprise », un rectangle tiré sur le plan ; annuler rend le plan d'avant.
        if name == "iphone": pg.click("#bilan-det > summary") if pg.evaluate("document.querySelector('#bilan-det').open") else None
        av = pg.evaluate("[calc.pl.length, S.emprise || null, passe.length]")
        pg.click("#b-emprise"); assert pg.evaluate("document.querySelector('#b-emprise').getAttribute('aria-pressed')") == "true"
        a, c = ([50, 220], [340, 560]) if name == "iphone" else ([500, 350], [1000, 740])
        pg.mouse.move(*a); pg.mouse.down(); pg.mouse.move(*[(a[0] + c[0]) / 2, (a[1] + c[1]) / 2], steps=3); pg.mouse.move(*c, steps=3)
        r["emprise_trace"] = pg.evaluate("S.emprise || null"); pg.mouse.up(); pg.wait_for_function("!document.querySelector('#aide').textContent.startsWith('Calcul')", timeout=60000); pg.wait_for_timeout(300)
        ap = pg.evaluate("({ pl: calc.pl.length, emprise: S.emprise, esc: calc.escaliers.length, viol: calc.principes.filter((p) => !p.ok).length, bouton: document.querySelector('#b-emprise').getAttribute('aria-pressed'), annuler: !document.querySelector('#b-annuler').disabled })")
        r["emprise_geste"] = ap; assert ap["emprise"] and ap["pl"] > 20 and ap["esc"] >= 1 and ap["viol"] == 0 and ap["bouton"] == "false" and ap["annuler"], ap
        pg.screenshot(path=str(out / f"{name}-emprise.png"))
        pg.click("#b-annuler"); pg.wait_for_timeout(300)
        assert pg.evaluate("[calc.pl.length, S.emprise || null]") == [av[0], av[1]], "annuler ne rend pas le plan d'avant"
        # Mesure de l'article PS 13 § 1 : dans l'axe des circulations des véhicules, depuis l'axe de la place (son point de
        # rencontre avec l'axe de la voie) jusqu'à la porte de l'escalier. 40 m / 25 m en parc enterré, 50 m / 30 m en parc
        # largement ventilé. Pas de traversée entre les voitures garées.
        PS = """(() => { const sauve = JSON.stringify(S), o = {}, X = (q) => centre(q)[0];
          const plan = (br, no, esc, parc) => { S = { ...S, v: 5, noeuds: no, branches: br, pn: [], pb: [], rampes: [], escAuto: false, auto: true, escaliers: esc, passages: [], parc, emprise: undefined }; cleEsc = null; pointer(); calculer(); };
          // 1. Une voie seule, un escalier côté gauche : porte à x = 30, à 3 m de l'axe.
          plan([{ a: 0, b: 1, g: "90", d: "90", pieton: "auto" }], [[0, 0], [60, 0]], [{ p: [30, 5.5] }], "enterre");
          const porte = calc.escaliers[0].a + 2.5; o.porte = porte;
          o.ecart90 = Math.max(...calc.pl.filter((q) => q.dist < Infinity).map((q) => Math.abs(q.dist - (Math.abs(X(q.q) - porte) + 3))));
          o.loin25 = [calc.loin.length, calc.pl.filter((q) => Math.abs(X(q.q) - porte) + 3 > 25.01).length];
          plan([{ a: 0, b: 1, g: "90", d: "90", pieton: "auto" }], [[0, 0], [60, 0]], [{ p: [30, 5.5] }], "ventile");
          o.loin30 = [calc.loin.length, calc.pl.filter((q) => Math.abs(X(q.q) - porte) + 3 > 30.01).length];
          // 2. Places à 45° : le départ est la rencontre de l'axe de la place (son grand côté) avec l'axe de la voie.
          plan([{ a: 0, b: 1, g: "90", d: "45", pieton: "auto" }], [[0, 0], [60, 0]], [{ p: [30, 5.5] }], "enterre");
          const p2 = calc.escaliers[0].a + 2.5, cote = (q) => { let m = null; for (let k = 0; k < 4; k++) { const A = q[k], B = q[(k + 1) % 4], l = Math.hypot(B[0] - A[0], B[1] - A[1]); if (!m || l > m.l) m = { l, d: [(B[0] - A[0]) / l, (B[1] - A[1]) / l] }; } return m.d; };
          o.ecart45 = Math.max(...calc.pl.filter((q) => q.c === "d" && q.dist < Infinity).map((q) => { const c = centre(q.q), d = cote(q.q), s = c[0] - c[1] * d[0] / d[1]; return Math.abs(q.dist - (Math.abs(Math.min(60, Math.max(0, s)) - p2) + Math.abs(s - Math.min(60, Math.max(0, s))) + 3)); }));
          o.decale45 = calc.pl.filter((q) => q.c === "d").some((q) => Math.abs(q.dist - (Math.abs(X(q.q) - p2) + 3)) > 1);
          // 3. Deux voies parallèles reliées par une voie à un seul bout : la place d'en face passe par la voie de liaison.
          plan([{ a: 0, b: 1, g: "90", d: "90", pieton: "auto" }, { a: 2, b: 3, g: "90", d: "90", pieton: "auto" }, { a: 0, b: 2, g: "aucun", d: "aucun", pieton: "auto" }], [[0, 0], [90, 0], [0, 16], [90, 16]], [{ p: [45, 5.5] }], "enterre");
          const p3 = calc.escaliers[0].a + 2.5, face = calc.pl.filter((q) => q.i === 1);
          o.ecartFace = Math.max(...face.map((q) => Math.abs(q.dist - (p3 + 16 + Math.abs(X(q.q)) + 3))));
          S = JSON.parse(sauve); cleEsc = null; pointer(); calculer(); return o; })()"""
        r["ps13"] = pg.evaluate(PS)
        q_ = r["ps13"]
        assert q_["ecart90"] < 0.01 and q_["loin25"][0] == q_["loin25"][1] > 0 and q_["loin30"][0] == q_["loin30"][1] < q_["loin25"][0], q_
        assert q_["ecart45"] < 0.01 and q_["decale45"] and q_["ecartFace"] < 0.01, q_
        # Passages : une place retirée de chaque côté d'une double rangée relie deux voies parallèles ; la mesure le franchit
        # d'un axe de voie à l'autre. Placement automatique : le moins d'escaliers, puis le moins de passages.
        PA = """(() => { const sauve = JSON.stringify(S), o = {}, X = (q) => centre(q)[0];
          const plan = (br, no, esc, pas, auto) => { S = { ...S, v: 5, noeuds: no, branches: br, pn: [], pb: [], rampes: [], escAuto: auto, auto: true, escaliers: esc, passages: pas, parc: "enterre", emprise: undefined }; cleEsc = null; pointer(); calculer(); };
          const deux = [{ a: 0, b: 1, g: "90", d: "90", pieton: "auto" }, { a: 2, b: 3, g: "90", d: "90", pieton: "auto" }, { a: 0, b: 2, g: "aucun", d: "aucun", pieton: "auto" }], nd = [[0, 0], [90, 0], [0, 16], [90, 16]];
          plan(deux, nd, [{ p: [45, -5.5] }], [], false); const sans = calc.pl.length;
          plan(deux, nd, [{ p: [45, -5.5] }], [{ p: [60, 8] }], false);
          const x = calc.passagesP[0], porte = calc.escaliers[0].a + 2.5; o.passages = calc.passagesP.length; o.perdues = sans - calc.pl.length; o.w = x && +x.w.toFixed(2);
          o.ecart = Math.max(...calc.pl.filter((q) => q.i === 1).map((q) => Math.abs(q.dist - Math.min(porte + 16 + Math.abs(X(q.q)) + 3, Math.abs(porte - x.a) + x.w + Math.abs(X(q.q) - x.b) + 3))));
          o.par = calc.pl.filter((q) => q.i === 1 && Math.abs(X(q.q) - 60) < 3).map((q) => +q.dist.toFixed(1));
          // Boucle de deux voies de 100 m : le placement automatique ouvre des passages s'ils économisent un escalier.
          const boucle = [{ a: 0, b: 1, g: "90", d: "90", pieton: "auto" }, { a: 2, b: 3, g: "90", d: "90", pieton: "auto" }, { a: 0, b: 2, g: "aucun", d: "aucun", pieton: "auto" }, { a: 1, b: 3, g: "aucun", d: "aucun", pieton: "auto" }], nb = [[0, 0], [100, 0], [0, 16], [100, 16]];
          plan(boucle, nb, [], [], true);
          o.auto = { k: placer.k, esc: calc.escaliers.length, pas: S.passages.length, viol: calc.principes.filter((p) => !p.ok).map((p) => p.id), bilan: document.querySelector("#s-escaliers").textContent };
          const garde = JSON.stringify(S.passages); o.minimal = JSON.parse(garde).map((_, k) => { S.escAuto = false; S.passages = JSON.parse(garde).filter((_, m) => m !== k); calculer(); return calc.loin.length > 0; });
          S.passages = JSON.parse(garde); S.escAuto = true; cleEsc = null; calculer();
          // Le plan change : les passages suivent.
          S.noeuds[1] = [130, 0]; S.noeuds[3] = [130, 16]; calculer(); o.suit = { esc: calc.escaliers.length, pas: S.passages.length, viol: calc.principes.filter((p) => !p.ok).map((p) => p.id), change: JSON.stringify(S.passages) !== garde };
          S = JSON.parse(sauve); cleEsc = null; pointer(); calculer(); return o; })()"""
        r["passages"] = pg.evaluate(PA); q_ = r["passages"]
        assert q_["passages"] == 1 and q_["perdues"] == 2 and q_["w"] == 16 and q_["ecart"] < 0.01, q_
        assert q_["auto"]["k"][1] < q_["auto"]["k"][0] and q_["auto"]["esc"] == q_["auto"]["k"][1] and q_["auto"]["pas"] >= 1 and not q_["auto"]["viol"] and "passage" in q_["auto"]["bilan"], q_
        assert all(q_["minimal"]) and not q_["suit"]["viol"] and q_["suit"]["change"], q_
        # Le type de parc : dans le calque des escaliers, sans rien de choisi, la fiche porte le choix Enterré / Largement ventilé.
        if name == "iphone" and not pg.evaluate("document.querySelector('#bilan-det').open"): pass
        pg.click('#calques button[data-c="2"]'); pg.wait_for_timeout(150)
        if pg.evaluate("document.querySelector('#fiche').hidden"): pg.mouse.click(*pg.evaluate("versE([200, 200])")); pg.wait_for_timeout(150)
        PC = "({ parc: S.parc, n: calc.escaliers.length, max: calc.distMax, loin: calc.loin.length, ok: calc.principes.find((p) => p.id === 'escaliers').ok, nom: calc.principes.find((p) => p.id === 'escaliers').nom, boutons: [...document.querySelectorAll('#f-parc button')].map((b) => [b.textContent, b.getAttribute('aria-pressed')]), vu: !document.querySelector('#f-parc').hidden && !document.querySelector('#fiche').hidden })"
        en = pg.evaluate(PC)
        assert en["vu"] and en["parc"] == "enterre" and en["boutons"] == [["Enterré", "true"], ["Largement ventilé", "false"]] and en["ok"] and "40 m" in en["nom"], en
        pg.click('#f-parc button[data-parc="ventile"]'); pg.wait_for_timeout(300)
        ve = pg.evaluate(PC); r["parc"] = [en, ve]
        assert ve["parc"] == "ventile" and ve["boutons"][1][1] == "true" and ve["ok"] and ve["n"] <= en["n"] and ve["max"] <= 50 and "50 m" in ve["nom"], ve
        assert pg.evaluate("JSON.parse(localStorage.getItem('parkimo.squelette.5')).parc") == "ventile"
        pg.click("#b-annuler"); pg.wait_for_timeout(300)
        assert pg.evaluate("S.parc") == "enterre" and pg.evaluate("calc.escaliers.length") == en["n"], "annuler ne rend pas le parc enterré"
        pg.click('#calques button[data-c="0"]'); pg.wait_for_timeout(150)
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
        # Sur les plans déformés : jamais de poteau en voie, et aucune poutre au-delà de la portée du système, ni d'escalier trop loin.
        assert r["alea"]["enVoie"] == 0 and "portee" not in r["alea"] and "escaliers" not in r["alea"], r["alea"]
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
