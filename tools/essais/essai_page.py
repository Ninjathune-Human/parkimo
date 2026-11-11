"""Essai de la page dans Chrome : plan de surface, test de giration de toutes les places, silo.
Usage : essai_page.py page.html dossier_des_captures"""
import sys, json, pathlib
from playwright.sync_api import sync_playwright
page_path = pathlib.Path(sys.argv[1]).resolve()
out = pathlib.Path(sys.argv[2]); out.mkdir(exist_ok=True)
errs = []
with sync_playwright() as p:
    b = p.chromium.launch(executable_path="/usr/bin/google-chrome", args=["--no-sandbox"])
    for name, vp in [("pc", {"width": 1440, "height": 900}), ("iphone", {"width": 390, "height": 844})]:
        ctx = b.new_context(viewport=vp, device_scale_factor=2 if name == "iphone" else 1, has_touch=name == "iphone")
        pg = ctx.new_page()
        pg.on("console", lambda m: errs.append(f"[{name}] {m.type}: {m.text}") if m.type == "error" else None)
        pg.on("pageerror", lambda e: errs.append(f"[{name}] pageerror: {e}"))
        pg.goto(page_path.as_uri())
        pg.wait_for_function("typeof layout !== 'undefined' && layout && layout.stalls && layout.stalls.length > 0", timeout=30000)
        pg.wait_for_timeout(1500)
        r = {"vue": name}
        r["surface"] = pg.evaluate("({places: layout.stalls.length, total: document.querySelector('#s-total').textContent, pmr: layout.metrics.pmr, warn: layout.warnings})")
        pg.screenshot(path=str(out / f"{name}-surface.png"))
        if name == "pc":
            # Giration : test de toutes les places.
            pg.click("#b-gir"); pg.wait_for_timeout(400)
            pg.click("#g-test")
            pg.wait_for_function("document.querySelector('#g-tally') && /\\d/.test(document.querySelector('#g-tally').textContent) && (document.querySelector('#g-prog').hidden || getComputedStyle(document.querySelector('#g-prog')).display === 'none')", timeout=240000)
            r["giration"] = pg.evaluate("document.querySelector('#g-tally').textContent.trim().replace(/\\s+/g,' ')")
            pg.screenshot(path=str(out / "pc-giration.png"))
            pg.keyboard.press("Escape")
        # Ouvrage : silo à trois niveaux.
        r["silo"] = pg.evaluate("(params.silo = true, params.silo_niveaux = 3, solve(false), layout.ouvrage ? {places: layout.stalls.length, niveaux: layout.ouvrage.niveaux, rampes: layout.ouvrage.rampes.length, noyaux: layout.ouvrage.noyaux.length, poteaux: layout.ouvrage.poteaux.length, warn: layout.warnings.length} : null)")
        pg.wait_for_timeout(800)
        pg.screenshot(path=str(out / f"{name}-silo.png"))
        print(json.dumps(r, ensure_ascii=False))
        ctx.close()
    b.close()
print("erreurs console:", len(errs)); [print(" ", e[:300]) for e in errs[:15]]
