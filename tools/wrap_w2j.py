"""Transforme la sortie module de wasm2js en script classique exposant window.TrameJS."""
import sys

src = open(sys.argv[1], encoding="utf-8").read()
body = "\n".join(l for l in src.split("\n") if not l.startswith("export "))
open(sys.argv[2], "w", encoding="utf-8").write("(function(){\n" + body + "\nwindow.TrameJS = retasmFunc;\n})();\n")
