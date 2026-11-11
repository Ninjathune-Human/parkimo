"""Assemble une page d'essai locale : interface courante (web/) et moteur WebAssembly tiré d'une page
compilée par l'intégration continue (branche compile/<branche> : git show origin/compile/<branche>:index.html).
Usage : page_locale.py depot page_compilee.html sortie.html"""
import pathlib, re, sys
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent.parent))
from page import source
root = pathlib.Path(sys.argv[1])
built = pathlib.Path(sys.argv[2]).read_text(encoding="utf-8")
wasm = re.search(r'<script id="engine-wasm" type="application/octet-stream">(.*?)</script>', built, re.S).group(1)
page = source(root)
symbols = (root / "src" / "symbols.json").read_text(encoding="utf-8")
body = page.replace("__WASM_BASE64__", wasm).replace("__SYMBOLS__", symbols).replace("__ENGINE_JS__", "")
head, rest = body.split("<style>", 1)
pathlib.Path(sys.argv[3]).write_text(
    '<!doctype html>\n<html lang="fr">\n<head>\n<meta charset="utf-8">\n'
    '<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">\n'
    + head + "<style>" + rest.split("</style>", 1)[0] + "</style>\n</head>\n<body>\n" + rest.split("</style>", 1)[1] + "\n</body>\n</html>\n",
    encoding="utf-8")
