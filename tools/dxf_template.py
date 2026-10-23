"""Génère src/dxf_template.dxf : squelette AutoCAD 2010 (DXF UTF-8) (en-tête, tables, calques, bloc ARBRE),
produit par ezdxf. Le moteur Rust y insère les entités. Usage : python3 tools/dxf_template.py"""
import io
import pathlib

import ezdxf

# Nom, couleur ACI, type de ligne, épaisseur (1/100 mm), description
LAYERS = [
    ("EMPRISE", 8, "DASHED", 13, "Emprise importee"),
    ("INFO", 7, "Continuous", 18, "Systeme de coordonnees et sources"),
    ("CAD_PARCELLES", 30, "Continuous", 18, "Cadastre : parcelles"),
    ("CAD_NUMEROS", 30, "Continuous", 13, "Cadastre : numeros de parcelle"),
    ("CAD_FEUILLES", 211, "DASHED", 35, "Cadastre : feuilles et sections"),
    ("CAD_COMMUNES", 170, "DASHDOT", 50, "Cadastre : limites communales"),
    ("OCC_VOIRIE", 9, "Continuous", 13, "Occupation du sol : voirie (domaine non cadastre)"),
    ("OCC_BATI", 8, "Continuous", 25, "Occupation du sol : bati"),
    ("OCC_VEGETATION", 94, "Continuous", 13, "Occupation du sol : vegetation"),
    ("PRJ_PARCELLE", 7, "Continuous", 50, "Projet : parcelle d'etude"),
    ("PRJ_RECUL", 40, "DASHED", 18, "Projet : limites de recul"),
    ("PRJ_VOIRIE", 8, "Continuous", 35, "Projet : limite des surfaces enrobees"),
    ("PRJ_PLACES", 7, "Continuous", 18, "Projet : places standard"),
    ("PRJ_PLACES_PMR", 150, "Continuous", 18, "Projet : places PMR"),
    ("PRJ_PLACES_IRVE", 3, "Continuous", 18, "Projet : places pre-equipees IRVE"),
    ("PRJ_ILOTS", 92, "Continuous", 18, "Projet : ilots plantes"),
    ("PRJ_ARBRES", 94, "Continuous", 18, "Projet : arbres"),
    ("PRJ_BATIMENTS", 7, "Continuous", 50, "Projet : batiments"),
    ("PRJ_ESPACES_VERTS", 92, "Continuous", 18, "Projet : espaces verts"),
    ("PRJ_SERVITUDES", 6, "Continuous", 25, "Projet : servitudes"),
]

doc = ezdxf.new("R2010", setup=False, units=6)
doc.header["$MEASUREMENT"] = 1
doc.header["$LTSCALE"] = 1.0
doc.header["$PSLTSCALE"] = 1
doc.linetypes.add("DASHED", [1.5, 1.0, -0.5], description="Tirets __ __ __")
doc.linetypes.add("DASHDOT", [3.0, 2.0, -0.5, 0.0, -0.5], description="Mixte __ . __ .")
for name, color, lt, lw, desc in LAYERS:
    layer = doc.layers.add(name, color=color, linetype=lt, lineweight=lw)
    layer.description = desc
blk = doc.blocks.new("ARBRE", base_point=(0, 0))
blk.add_circle((0, 0), 1.0, dxfattribs={"layer": "0", "color": 0})
blk.add_line((-0.25, 0), (0.25, 0), dxfattribs={"layer": "0", "color": 0})
blk.add_line((0, -0.25), (0, 0.25), dxfattribs={"layer": "0", "color": 0})

buf = io.StringIO()
doc.write(buf)
out = pathlib.Path(__file__).resolve().parent.parent / "src" / "dxf_template.dxf"
out.write_text(buf.getvalue(), encoding="utf-8", newline="\n")
print(out, len(buf.getvalue()), "octets,", len(LAYERS), "calques")
