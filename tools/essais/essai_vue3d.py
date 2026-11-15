"""Essai de la vue 3D du squelette dans Chrome.
1. Plateaux de référence : quatre plans du squelette (exemple, silo ventilé avec rampe posée et hélice, rampe
   branchée, enterré en acier), lus par plateau.js et écrits dans prototypes/squelette/essais/plateaux/ pour les
   essais de la scène sous Node (npm test dans prototypes/squelette).
2. Vue (drapeau ?3d) : bascule, mise à jour après une modification, captures PC, iPad et iPhone, console sans erreur,
   durée de reconstruction ; sans drapeau, la page du squelette est inchangée (pas de bouton 3D, aucun module chargé).
Usage : essai_vue3d.py dossier_des_captures [dossier_vendor] [--plateaux : seulement les plateaux de référence]
dossier_vendor (facultatif, machine sans accès au CDN) : three/ (paquet npm three@0.187.0 décompressé) et
polygon-clipping.esm.js (polygon-clipping 0.15.7 en un seul module ES) ; les demandes à cdn.jsdelivr.net y sont servies."""
import functools, http.server, json, pathlib, sys, threading
from playwright.sync_api import sync_playwright

root = pathlib.Path(__file__).resolve().parents[2]
dossier = root / "prototypes" / "squelette"
args = [a for a in sys.argv[1:] if not a.startswith("--")]; PLATEAUX_SEULS = "--plateaux" in sys.argv
out = pathlib.Path(args[0]); out.mkdir(parents=True, exist_ok=True)
vendor = pathlib.Path(args[1]) if len(args) > 1 else None
plateaux = dossier / "essais" / "plateaux"; plateaux.mkdir(parents=True, exist_ok=True)

class Servi(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *a): pass
srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Servi, directory=str(dossier)))
threading.Thread(target=srv.serve_forever, daemon=True).start()
URL = f"http://127.0.0.1:{srv.server_address[1]}/index.html"

def cdn(route):
    u = route.request.url
    if "polygon-clipping" in u:
        return route.fulfill(path=str(vendor / "polygon-clipping.esm.js"), content_type="text/javascript")
    rel = u.split("/npm/three@0.187.0/", 1)[1]
    return route.fulfill(path=str(vendor / "three" / rel), content_type="text/javascript")

# Plans de référence : chacun part de l'exemple du squelette (stockage vidé), puis le modifie.
SCENARIOS = {
    "exemple": "",
    "ventile": """S.parc = 'ventile'; S.systeme = 'poteaux'; S.niveaux = 5;
        const q = calc.bandes.find((q) => q.droit && q.i === 0); S.rampes = [{ p: [(q[0][0] + q[2][0]) / 2, (q[0][1] + q[2][1]) / 2], type: 'droite' }];
        S.helices = [{ p: [99, 0], sens: 1 }];""",
    "rampe-voie": "S.parc = 'ventile'; S.niveaux = 3; const b = S.branches[4]; b.rampe = 'droite'; b.sens = 1; b.g = b.d = 'aucun';",
    "acier-enterre": "S.parc = 'enterre'; S.systeme = 'acier'; S.niveaux = 3;",
}

def panneau(pg, ouvert):
    if pg.evaluate("document.querySelector('#p3d-det').open") != ouvert: pg.click("#p3d-det > summary"); pg.wait_for_timeout(150)

errs = []
with sync_playwright() as p:
    b = p.chromium.launch(executable_path="/usr/bin/google-chrome", args=["--no-sandbox", "--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"])
    ctx = b.new_context(viewport={"width": 1440, "height": 900})
    pg = ctx.new_page()
    pg.on("console", lambda m: errs.append(f"[plateaux] {m.type}: {m.text}") if m.type == "error" and "favicon" not in (m.location or {}).get("url", "") else None)
    pg.on("pageerror", lambda e: errs.append(f"[plateaux] pageerror: {e}"))
    for nom, js in SCENARIOS.items():
        pg.goto(URL); pg.evaluate("localStorage.clear()"); pg.goto(URL); pg.wait_for_timeout(300)
        if js: pg.evaluate("() => { " + js + " pointer(); rafraichir(); }"); pg.wait_for_timeout(200)
        pl = pg.evaluate("async () => (await import('./plateau.js')).extraire()")
        (plateaux / f"{nom}.json").write_text(json.dumps(pl, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
        print(nom, {k: len(v) if isinstance(v, list) else v for k, v in pl.items() if k in ("niveaux", "parc", "chaussees", "places", "rampes", "rampesVoie", "helices", "escaliers", "poteaux", "poutres")})
    ctx.close()
    if PLATEAUX_SEULS: b.close(); print("\n".join(errs) or "console sans erreur"); sys.exit(1 if errs else 0)

    # Sans drapeau : la page du squelette est celle d'avant (aucun module, pas de bouton 3D).
    ctx = b.new_context(viewport={"width": 1440, "height": 900}); pg = ctx.new_page(); vus = []
    pg.on("request", lambda r: vus.append(r.url))
    pg.goto(URL); pg.evaluate("localStorage.clear()"); pg.goto(URL); pg.wait_for_timeout(400)
    assert pg.evaluate("document.querySelector('#b-3d').hidden && !window.vue3d"), "bouton 3D visible sans drapeau"
    assert not any(("vue3d" in u or "jsdelivr" in u) for u in vus), [u for u in vus if "vue3d" in u or "jsdelivr" in u]
    ctx.close()

    # Avec le drapeau : bascule, rendus, niveaux, annuler, retour au plan par Emprise ; trois formats d'écran.
    FORMATS = [("pc", {"width": 1440, "height": 900}, False), ("ipad", {"width": 1180, "height": 820}, True), ("iphone", {"width": 390, "height": 844}, True)]
    mesures = {}
    for nom, vp, tactile in FORMATS:
        ctx = b.new_context(viewport=vp, device_scale_factor=2 if nom == "iphone" else 1, has_touch=tactile); pg = ctx.new_page()
        if vendor: pg.route("https://cdn.jsdelivr.net/**", cdn)
        pg.on("console", lambda m, nom=nom: errs.append(f"[{nom}] {m.type}: {m.text}") if m.type in ("error", "warning") and "favicon" not in (m.location or {}).get("url", "") and "GPU stall" not in m.text and "swiftshader" not in m.text.lower() else None)
        pg.on("pageerror", lambda e, nom=nom: errs.append(f"[{nom}] pageerror: {e}"))
        for scen in (["exemple", "ventile"] if nom != "ipad" else ["ventile"]):
            pg.goto(URL + "?3d"); pg.evaluate("localStorage.clear()"); pg.goto(URL + "?3d")
            pg.wait_for_function("window.vue3d && !document.querySelector('#b-3d').hidden", timeout=20000)
            if SCENARIOS[scen]: pg.evaluate("() => { " + SCENARIOS[scen] + " pointer(); rafraichir(); }")
            pg.click("#b-3d"); pg.wait_for_function("window.vue3d.mesure", timeout=20000); pg.wait_for_timeout(900)
            assert pg.evaluate("document.body.classList.contains('en3d') && getComputedStyle(document.querySelector('#vue3d')).opacity === '1'")
            m = pg.evaluate("window.vue3d.mesure"); mesures[f"{nom}-{scen}"] = m; print(nom, scen, m, flush=True)
            for rendu in ("maquette", "trait"):
                panneau(pg, True); pg.click(f'#p3d-rendu button[data-r="{rendu}"]')
                if nom == "iphone": panneau(pg, False) # sur iPhone, le panneau est replié par défaut
                pg.wait_for_timeout(700); pg.screenshot(path=str(out / f"{nom}-{scen}-{rendu}.png"))
            panneau(pg, True); pg.click('#p3d-rendu button[data-r="maquette"]')
        # Un niveau de plus : la scène suit ; Annuler revient ; Emprise ramène au plan.
        panneau(pg, True)
        n0 = pg.evaluate("window.vue3d.scene().niveaux.length")
        pg.click("#p3d-plus"); pg.wait_for_timeout(500)
        assert pg.evaluate("window.vue3d.scene().niveaux.length") == n0 + 1 and pg.evaluate("S.niveaux") == 6
        pg.click("#b-annuler"); pg.wait_for_timeout(500)
        assert pg.evaluate("window.vue3d.scene().niveaux.length") == n0 and pg.evaluate("S.niveaux") == 5
        # Gestes : un glisser fait tourner la vue (OrbitControls), sans toucher au plan.
        avant = pg.evaluate("JSON.stringify(S)"), pg.evaluate("window.vue3d.camera.position.toArray()")
        cx, cy = vp["width"] / 2, vp["height"] / 2
        pg.mouse.move(cx, cy); pg.mouse.down(); pg.mouse.move(cx + 80, cy + 10, steps=6); pg.mouse.up(); pg.wait_for_timeout(500)
        apres = pg.evaluate("JSON.stringify(S)"), pg.evaluate("window.vue3d.camera.position.toArray()")
        assert avant[0] == apres[0] and avant[1] != apres[1], "le glisser doit tourner la vue sans modifier le plan"
        pg.screenshot(path=str(out / f"{nom}-tourne.png"))
        pg.click("#b-emprise"); pg.wait_for_function("!document.body.classList.contains('en3d')", timeout=8000)
        assert pg.evaluate("document.querySelector('#b-emprise').getAttribute('aria-pressed')") == "true"
        barre = pg.evaluate("[...document.querySelectorAll('#barre > button')].map((b) => Math.round(b.getBoundingClientRect().top))")
        assert len(set(barre)) == 1, f"[{nom}] les boutons de la barre ne tiennent pas sur une ligne : {barre}"
        ctx.close()

    # Étape 3, la levée. (a) La première image 3D est le plan : mêmes points aux mêmes pixels. (b) Une bascule
    # interrompue repart en sens inverse. (c) Au retour, le plan est recalé sur le point regardé en 3D, à la même
    # échelle. (d) Mouvement réduit : pas d'animation. Puis une suite d'images de la levée, pour la vidéo.
    ctx = b.new_context(viewport={"width": 1280, "height": 800}); pg = ctx.new_page()
    if vendor: pg.route("https://cdn.jsdelivr.net/**", cdn)
    pg.on("console", lambda m: errs.append(f"[levee] {m.type}: {m.text}") if m.type == "error" and "favicon" not in (m.location or {}).get("url", "") else None)
    pg.on("pageerror", lambda e: errs.append(f"[levee] pageerror: {e}"))
    pg.goto(URL + "?3d"); pg.evaluate("localStorage.clear()"); pg.goto(URL + "?3d"); pg.wait_for_function("window.vue3d", timeout=20000)
    pg.evaluate("() => { " + SCENARIOS["ventile"] + " pointer(); rafraichir(); }"); pg.wait_for_timeout(300)
    pts = pg.evaluate("[...calc.ch[0], ...calc.ch[2], ...calc.pl.slice(0, 6).map((p) => p.q[0])]")
    plan = pg.evaluate("(pts) => pts.map(versE)", pts)
    pg.screenshot(path=str(out / "levee-0-plan.png"))
    pg.click("#b-3d"); pg.evaluate("window.vue3d.figer(0)")
    vu = pg.evaluate("(pts) => pts.map((p) => window.vue3d.projeter(p[0], p[1], 0))", pts)
    ecart = max(abs(a[0] - c[0]) + abs(a[1] - c[1]) for a, c in zip(plan, vu))
    print("levée : écart plan / première image 3D", round(ecart, 3), "px"); assert ecart < 1.0, ecart
    for k, p_ in enumerate([0, 0.15, 0.3, 0.45, 0.6, 0.75, 0.9, 1]):
        pg.evaluate(f"window.vue3d.figer({p_})"); pg.screenshot(path=str(out / f"levee-{k + 1}.png"))
    # (b) bascule interrompue
    pg.goto(URL + "?3d"); pg.wait_for_function("window.vue3d", timeout=20000)
    pg.click("#b-3d"); pg.wait_for_timeout(450); e1 = pg.evaluate("window.vue3d.etat()")
    pg.click("#b-3d"); pg.wait_for_timeout(50); e2 = pg.evaluate("window.vue3d.etat()")
    assert e1["sens"] == 1 and e2["sens"] == -1 and e2["anim"], (e1, e2)
    pg.wait_for_function("!document.body.classList.contains('en3d')", timeout=8000)
    # (c) retour recalé : en 3D, on s'approche d'un point ; revenu au plan, ce point est au centre, à la même échelle.
    pg.click("#b-3d"); pg.wait_for_function("window.vue3d.etat().actif && !window.vue3d.etat().anim", timeout=8000)
    cible = pg.evaluate("(() => { const v = window.vue3d, t = v.controles.target; t.x += 12; t.z -= 4; v.camera.zoom = 1.6; v.camera.updateProjectionMatrix(); v.controles.update(); return [t.x, -t.z]; })()")
    pg.wait_for_timeout(300); pg.click("#b-3d"); pg.wait_for_function("!document.body.classList.contains('en3d')", timeout=8000)
    centre = pg.evaluate("(c) => [versE(c), [cv.clientWidth / 2, cv.clientHeight / 2], vue.s]", cible)
    assert abs(centre[0][0] - centre[1][0]) < 1 and abs(centre[0][1] - centre[1][1]) < 1, centre
    # (d) mouvement réduit
    pg.emulate_media(reduced_motion="reduce"); pg.click("#b-3d"); pg.wait_for_timeout(250)
    assert pg.evaluate("window.vue3d.etat().progres") == 1
    ctx.close()
    b.close()

print(json.dumps(mesures, indent=1))
print("\n".join(errs) or "console sans erreur")
assert not errs
