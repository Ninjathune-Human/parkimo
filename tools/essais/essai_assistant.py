"""Essai de l'assistant dans Chrome, l'API d'Anthropic étant remplacée par des réponses écrites
d'avance : en-têtes et corps de la demande, boucle d'outils (lecture, commandes, refus), annulation,
clé refusée, clé absente. Usage : essai_assistant.py page.html dossier_des_captures"""
import sys, json, pathlib
from playwright.sync_api import sync_playwright
page_path = pathlib.Path(sys.argv[1]).resolve()
out = pathlib.Path(sys.argv[2]); out.mkdir(exist_ok=True)
CORS = {"access-control-allow-origin": "*", "access-control-allow-headers": "*", "access-control-allow-methods": "POST, OPTIONS"}
errs, vus = [], []

def reponses():
    """Réponses successives de l'API pour une demande : lire, agir, se tromper, conclure."""
    yield {"stop_reason": "tool_use", "content": [{"type": "text", "text": "Je regarde le projet."}, {"type": "tool_use", "id": "toolu_1", "name": "lire_projet", "input": {}}]}
    yield {"stop_reason": "tool_use", "content": [{"type": "tool_use", "id": "toolu_2", "name": "appliquer_commandes", "input": {"commandes": [{"op": "regler", "cle": "slant", "valeur": 60}, {"op": "regler", "cle": "stall_w", "valeur": 2.3}]}}]}
    yield {"stop_reason": "tool_use", "content": [{"type": "tool_use", "id": "toolu_3", "name": "appliquer_commandes", "input": {"commandes": [{"op": "regler", "cle": "aisle", "valeur": 99}]}}]}
    yield {"stop_reason": "end_turn", "content": [{"type": "text", "text": "Les places sont à 60° : 247 places avant, moins après. Aucun écart."}]}

with sync_playwright() as p:
    b = p.chromium.launch(executable_path="/usr/bin/google-chrome", args=["--no-sandbox"])
    for name, vp in [("pc", {"width": 1440, "height": 900}), ("iphone", {"width": 390, "height": 844})]:
        ctx = b.new_context(viewport=vp, device_scale_factor=2 if name == "iphone" else 1, has_touch=name == "iphone")
        pg = ctx.new_page()
        pg.on("console", lambda m: errs.append(f"[{name}] {m.type}: {m.text}") if m.type == "error" and "401" not in m.text else None)
        pg.on("pageerror", lambda e: errs.append(f"[{name}] pageerror: {e}"))
        suite, etat = reponses(), {"refuser": False}
        def api(route):
            rq = route.request
            if rq.method == "OPTIONS":
                return route.fulfill(status=204, headers=CORS)
            vus.append({"entetes": {k: v for k, v in rq.headers.items() if k.startswith("anthropic") or k in ("x-api-key", "content-type")}, "corps": json.loads(rq.post_data)})
            if etat["refuser"]:
                return route.fulfill(status=401, headers=CORS, content_type="application/json", body=json.dumps({"type": "error", "error": {"type": "authentication_error", "message": "invalid x-api-key"}}))
            route.fulfill(status=200, headers=CORS, content_type="application/json", body=json.dumps({"id": "msg_1", "type": "message", "role": "assistant", "model": "essai", **next(suite)}))
        pg.route("https://api.anthropic.com/**", api)
        pg.goto(page_path.as_uri())
        pg.wait_for_function("typeof layout !== 'undefined' && layout && layout.stalls && layout.stalls.length > 0", timeout=30000)
        pg.wait_for_timeout(800)
        LOG = "[...document.querySelectorAll('#ia-log .ia-m')].map((e) => [e.className.replace('ia-m ', ''), e.textContent])"
        r = {"vue": name}
        pg.click("#b-ia"); pg.wait_for_timeout(300)
        r["ouverture"] = pg.evaluate("({ouvert: !document.querySelector('#ia').hidden, reglages: document.querySelector('#ia-cfg').open, suggestions: !document.querySelector('#ia-vide').hidden})")
        pg.screenshot(path=str(out / f"{name}-assistant-vide.png"))
        # Sans clé : la demande n'est pas envoyée.
        pg.fill("#ia-q", "Passe les places à 60°"); pg.press("#ia-q", "Enter"); pg.wait_for_timeout(300)
        r["sans_cle"] = [len(vus), pg.evaluate(LOG), pg.evaluate("document.querySelector('#toast').textContent")]
        pg.fill("#ia-key", "sk-ant-essai"); pg.wait_for_timeout(100)
        n0 = len(vus)
        pg.fill("#ia-q", "Passe les places à 60°"); pg.press("#ia-q", "Enter")
        pg.wait_for_function("!IA.occupe", timeout=20000); pg.wait_for_timeout(600)
        mes = vus[n0:]
        premier, dernier = mes[0], mes[-1]["corps"]["messages"]
        r["demande"] = {
            "appels": len(mes), "entetes": premier["entetes"], "modele": premier["corps"]["model"], "max_tokens": premier["corps"]["max_tokens"],
            "outils": [t["name"] for t in premier["corps"]["tools"]], "catalogue_dans_le_systeme": "regler_recul" in premier["corps"]["system"],
            "premier_message": premier["corps"]["messages"][0],
            "resultat_lecture": json.loads(mes[1]["corps"]["messages"][-1]["content"][0]["content"])["places"]["total"],
            "resultat_commandes": json.loads(mes[2]["corps"]["messages"][-1]["content"][0]["content"])["places"]["total"],
            "resultat_refus": mes[3]["corps"]["messages"][-1]["content"][0],
            "roles": [m["role"] for m in dernier],
        }
        r["conversation"] = pg.evaluate(LOG)
        r["demande_visible"] = pg.evaluate("(() => { const f = document.querySelector('#ia-form').getBoundingClientRect(), p = document.querySelector('#ia').getBoundingClientRect(), t = document.querySelector('#toolbar').getBoundingClientRect(); return f.height > 40 && f.bottom <= p.bottom + 1 && p.bottom <= t.top + 1 && p.top >= 0; })()")
        r["plan"] = pg.evaluate("({places: layout.stalls.length, slant: params.slant, stall_w: params.stall_w, annulable: !document.querySelector('#b-undo').disabled})")
        pg.screenshot(path=str(out / f"{name}-assistant.png"))
        pg.click("#b-undo"); pg.wait_for_timeout(600)
        r["apres_annulation"] = pg.evaluate("({places: layout.stalls.length, slant: params.slant})")
        # Clé refusée par l'API.
        etat["refuser"] = True
        pg.fill("#ia-q", "Et maintenant ?"); pg.press("#ia-q", "Enter")
        pg.wait_for_function("!IA.occupe", timeout=20000)
        r["cle_refusee"] = pg.evaluate(LOG)[-1]
        r["cle_non_memorisee"] = pg.evaluate("localStorage.getItem('parkimo.ia')")
        # La touche F, tapée dans la demande, ne recadre pas le plan.
        vue = pg.evaluate("JSON.stringify(view)"); pg.focus("#ia-q"); pg.keyboard.type("fff"); pg.wait_for_timeout(200)
        r["clavier_isole"] = vue == pg.evaluate("JSON.stringify(view)")
        pg.click("#ia-close"); r["ferme"] = pg.evaluate("document.querySelector('#ia').hidden")
        print(json.dumps(r, ensure_ascii=False, indent=1))
        ctx.close()
    b.close()
print("erreurs console:", len(errs)); [print(" ", e[:300]) for e in errs[:15]]
