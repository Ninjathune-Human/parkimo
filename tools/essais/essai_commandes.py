"""Essai des commandes dans Chrome : catalogue, application, refus, annulation, silo.
Usage : essai_commandes.py page.html dossier_des_captures"""
import sys, json, pathlib
from playwright.sync_api import sync_playwright
page_path = pathlib.Path(sys.argv[1]).resolve()
out = pathlib.Path(sys.argv[2]); out.mkdir(exist_ok=True)
errs = []
ETAT = "({places: layout.stalls.length, slant: params.slant, stall_w: params.stall_w, champ: document.querySelector('#p-stall_w').value, batiments: site.buildings.length, bassins: (site.basins || []).length, silo: !!layout.ouvrage, annulable: !document.querySelector('#b-undo').disabled})"
with sync_playwright() as p:
    b = p.chromium.launch(executable_path="/usr/bin/google-chrome", args=["--no-sandbox"])
    for name, vp in [("pc", {"width": 1440, "height": 900}), ("iphone", {"width": 390, "height": 844})]:
        ctx = b.new_context(viewport=vp, device_scale_factor=2 if name == "iphone" else 1, has_touch=name == "iphone")
        pg = ctx.new_page()
        pg.on("console", lambda m: errs.append(f"[{name}] {m.type}: {m.text}") if m.type == "error" else None)
        pg.on("pageerror", lambda e: errs.append(f"[{name}] pageerror: {e}"))
        pg.goto(page_path.as_uri())
        pg.wait_for_function("typeof layout !== 'undefined' && layout && layout.stalls && layout.stalls.length > 0", timeout=30000)
        pg.wait_for_timeout(1000)
        r = {"vue": name, "catalogue": pg.evaluate("parkimo.catalogue().map((c) => c.op)")}
        r["depart"] = pg.evaluate(ETAT)
        pg.screenshot(path=str(out / f"{name}-commandes-avant.png"))
        r["reponse"] = pg.evaluate("""parkimo.appliquer([
          { op: "regler", cle: "slant", valeur: 60 }, { op: "regler", cle: "stall_w", valeur: 2.3 },
          { op: "retirer_batiment", batiment: 0 },
          { op: "ajouter_zone", nature: "bassin", points: [[100, 12], [150, 12], [150, 40], [100, 40]] }])""")
        pg.wait_for_timeout(800)
        r["apres"] = pg.evaluate(ETAT)
        pg.screenshot(path=str(out / f"{name}-commandes-apres.png"))
        r["refus"] = pg.evaluate("""parkimo.appliquer([{ op: "regler", cle: "slant", valeur: 90 }, { op: "regler", cle: "aisle", valeur: 99 }])""")
        r["apres_refus"] = pg.evaluate(ETAT)
        pg.click("#b-undo"); pg.wait_for_timeout(800)
        r["apres_annulation"] = pg.evaluate(ETAT)
        r["silo"] = [pg.evaluate("""parkimo.appliquer([{ op: "regler", cle: "silo", valeur: true }, { op: "regler", cle: "silo_niveaux", valeur: 3 }])"""), pg.evaluate(ETAT)]
        r["plan"] = pg.evaluate("({places: parkimo.plan().stalls.length, constats: parkimo.plan().constats.length, projet: Object.keys(parkimo.projet())})")
        print(json.dumps(r, ensure_ascii=False))
        ctx.close()
    b.close()
print("erreurs console:", len(errs)); [print(" ", e[:300]) for e in errs[:15]]
