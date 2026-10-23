"""Assemble l'application mono-fichier : python3 tools/bundle.py trame.wasm
Produit dist/index.html (autonome, GitHub Pages) et dist/trame.html (contenu seul, pour artifact)."""
import base64
import pathlib
import sys

root = pathlib.Path(__file__).resolve().parent.parent
wasm = pathlib.Path(sys.argv[1]).read_bytes()
page = (root / "web" / "app.html").read_text(encoding="utf-8")
assert "__WASM_BASE64__" in page
body = page.replace("__WASM_BASE64__", base64.b64encode(wasm).decode("ascii"))
# Repli JavaScript optionnel : python3 tools/bundle.py trame.wasm moteur.js
fallback = pathlib.Path(sys.argv[2]).read_text(encoding="utf-8") if len(sys.argv) > 2 else ""
dist = root / "dist"
dist.mkdir(exist_ok=True)
safe = fallback.replace("</script", "<\\/script")
# Page publiée : repli exécuté d'emblée (le chargement à la demande par blob: y est refusé).
(dist / "trame.html").write_text(body.replace("__ENGINE_JS__", f"<script>{safe}</script>" if safe else ""), encoding="utf-8")
# Version autonome : repli conservé en texte, injecté seulement si WebAssembly échoue.
body = body.replace("__ENGINE_JS__", f'<script type="text/plain" id="engine-js-lazy">{safe}</script>' if safe else "")
head, rest = body.split("<style>", 1)
standalone = (
    "<!doctype html>\n<html lang=\"fr\">\n<head>\n<meta charset=\"utf-8\">\n"
    "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1, viewport-fit=cover\">\n"
    + head + "<style>" + rest.split("</style>", 1)[0] + "</style>\n</head>\n<body>\n"
    + rest.split("</style>", 1)[1] + "\n</body>\n</html>\n"
)
(dist / "index.html").write_text(standalone, encoding="utf-8")
print(f"wasm {len(wasm)/1024:.0f} Ko -> index.html {len(standalone.encode())/1024:.0f} Ko")
