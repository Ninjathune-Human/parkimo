"""Symboles d'habillage (véhicules vus de dessus, arbres) : source unique de la géométrie.
Écrit src/symbols.json, lu par l'application (rendu) et par tools/dxf_template.py (blocs AutoCAD).
Usage : python3 tools/symbols.py

Primitives : {"k": rôle, "p": [[x, y], ...]} polygone fermé, "o": true pour une polyligne ouverte,
ou {"k": rôle, "c": [x, y], "r": rayon} cercle. Véhicules en mètres, avant vers +x, centrés.
Arbres en rayon unitaire (le bloc est inséré à l'échelle du rayon de houppier)."""
import json
import math
import pathlib


def r3(v):
    return round(v + 0.0, 3)


def poly(k, pts, open_=False):
    d = {"k": k, "p": [[r3(x), r3(y)] for x, y in pts]}
    if open_:
        d["o"] = True
    return d


def circle(k, c, r):
    return {"k": k, "c": [r3(c[0]), r3(c[1])], "r": r3(r)}


# ---------------------------------------------------------------------------
# Véhicules
# ---------------------------------------------------------------------------

def superellipse(a, b, pf, pr, n=72):
    """Contour CCW de demi-axes a (longueur) et b (largeur), exposant pf à l'avant, pr à l'arrière."""
    out = []
    for i in range(n):
        t = 2 * math.pi * i / n
        c, s = math.cos(t), math.sin(t)
        p = pf if c >= 0 else pr
        out.append((a * math.copysign(abs(c) ** (2 / p), c), b * math.copysign(abs(s) ** (2 / p), s)))
    return out


def half_width(a, b, p, x):
    """Demi-largeur de la superellipse à l'abscisse x."""
    u = min(1.0, abs(x) / a)
    return b * (1 - u ** p) ** (1 / p)


def edge(x, hw, bow, n=8):
    """Bord transversal courbe de y = -hw à +hw, bombé de `bow` vers +x au centre."""
    return [(x + bow * (1 - (2 * i / n - 1) ** 2), hw * (2 * i / n - 1)) for i in range(n + 1)]


def vehicle(vid, nom, L, W, hood, ws, roof, rw, pf, pr, poids, rails=False, doors=False):
    a, b = L / 2, W / 2
    hw = lambda x: half_width(a, b, pf if x >= 0 else pr, x)
    xa = a - hood            # pied de pare-brise
    xb = xa - ws             # haut de pare-brise, avant du pavillon
    xc = xb - roof           # arrière du pavillon
    xd = xc - rw             # bas de lunette arrière
    inset_glass, inset_roof, side = 0.11, 0.24, 0.10
    prims = [poly("body", superellipse(a, b, pf, pr))]
    # Rétroviseurs au pied du pare-brise.
    for sg in (1, -1):
        y0 = hw(xa) - 0.03
        m = [(xa - 0.02, y0), (xa - 0.04, y0 + 0.15), (xa - 0.22, y0 + 0.17), (xa - 0.27, y0)]
        prims.append(poly("mirror", [(x, sg * y) for x, y in m] if sg > 0 else [(x, -y) for x, y in reversed(m)]))
    # Pare-brise : bord avant bombé vers l'avant, bord haut moins bombé.
    hwa, hwb = hw(xa) - inset_glass, hw(xb) - inset_roof
    front = edge(xa, hwa, 0.10)
    top = edge(xb, hwb, 0.05)
    prims.append(poly("glass", list(reversed(front)) + top))
    # Lunette arrière : bombée vers l'arrière.
    if rw > 0:
        hwc, hwd = hw(xc) - inset_roof, hw(xd) - inset_glass - 0.04
        rtop = edge(xc, hwc, -0.05)
        rbot = edge(xd, hwd, -0.09)
        prims.append(poly("glass", rtop + list(reversed(rbot))))
    # Vitres latérales : bandes le long du pavillon.
    for sg in (1, -1):
        y_in, y_out = hw(xb) - inset_roof, hw(xb) - inset_roof + side
        y_in2, y_out2 = hw(xc) - inset_roof, hw(xc) - inset_roof + side
        s = [(xb - 0.02, y_in), (xb - 0.12, y_out), (xc + 0.10, y_out2), (xc + 0.02, y_in2)]
        prims.append(poly("glass", [(x, sg * y) for x, y in (s if sg > 0 else list(reversed(s)))]))
    # Pavillon, entre pare-brise et lunette.
    roof_front = edge(xb, hwb, 0.05)
    roof_back = edge(xc, hw(xc) - inset_roof, -0.05)
    prims.append(poly("roof", roof_front + list(reversed(roof_back))))
    if rails:
        for sg in (1, -1):
            y = sg * (hwb - 0.10)
            prims.append(poly("line", [(xb - 0.12, y), (xc + 0.12, y)], True))
    if doors:
        prims.append(poly("line", [(xd, 0), (-a + 0.01, 0)], True))
    return {"id": vid, "nom": nom, "bloc": "VEH_" + vid.upper(), "L": L, "W": W, "poids": poids, "prims": prims}


VEHICLES = [
    vehicle("citadine", "Citadine", 3.70, 1.66, 0.78, 0.74, 1.30, 0.42, 4.2, 5.5, 22),
    vehicle("compacte", "Compacte", 4.30, 1.80, 0.98, 0.84, 1.42, 0.48, 4.4, 5.5, 28),
    vehicle("berline", "Berline", 4.80, 1.84, 1.18, 0.80, 1.24, 0.66, 4.6, 5.0, 14),
    vehicle("break", "Break", 4.70, 1.83, 1.10, 0.80, 1.98, 0.34, 4.6, 6.0, 10),
    vehicle("suv", "SUV", 4.55, 1.88, 1.12, 0.70, 1.80, 0.40, 6.0, 7.0, 18, rails=True),
    vehicle("utilitaire", "Utilitaire", 4.45, 1.86, 0.92, 0.68, 2.40, 0.22, 6.5, 9.0, 8, doors=True),
]

# ---------------------------------------------------------------------------
# Arbres (rayon unitaire)
# ---------------------------------------------------------------------------

def radial(f, n=144):
    return [(f(2 * math.pi * i / n) * math.cos(2 * math.pi * i / n), f(2 * math.pi * i / n) * math.sin(2 * math.pi * i / n)) for i in range(n)]


def polar(r, a):
    return (r * math.cos(a), r * math.sin(a))


def feuillu():
    prims = [poly("crown", radial(lambda t: 0.88 + 0.12 * abs(math.cos(5.5 * t))))]
    for i in range(5):
        a = 2 * math.pi * i / 5 + 0.3
        prims.append(poly("line", [polar(0.1, a), polar(0.42, a), polar(0.62, a - 0.32)], True))
        prims.append(poly("line", [polar(0.42, a), polar(0.64, a + 0.28)], True))
    prims.append(circle("trunk", (0, 0), 0.08))
    return prims


def pin_parasol():
    prims = [poly("crown", radial(lambda t: 0.84 + 0.16 * abs(math.cos(3.5 * t + 0.35 * math.sin(2 * t)))))]
    prims.append(poly("line", radial(lambda t: 0.5 + 0.08 * abs(math.cos(2.5 * t + 0.6)), 90)))
    for i in range(7):
        a = 2 * math.pi * i / 7
        prims.append(poly("line", [polar(0.08, a), polar(0.34, a + 0.12)], True))
    prims.append(circle("trunk", (0, 0), 0.07))
    return prims


def olivier():
    clumps = [(0.0, 0.0, 0.42), (0.44, 0.10, 0.44), (0.18, 0.46, 0.40), (-0.30, 0.38, 0.42),
              (-0.46, -0.10, 0.40), (-0.16, -0.46, 0.42), (0.32, -0.36, 0.40)]
    prims = [circle("crown", (x, y), r) for x, y, r in clumps]
    prims.append(circle("trunk", (0.04, -0.02), 0.09))
    return prims


def palmier():
    """Palmes pennées : contour dentelé (folioles) autour d'une nervure légèrement arquée."""
    prims = []
    for i in range(9):
        a = 2 * math.pi * i / 9 + 0.12 * math.sin(i * 1.7)
        bend = 0.22 * (1 if i % 2 else -1)
        n = 22
        left, right, mid = [], [], []
        for j in range(n + 1):
            t = j / n
            ang = a + bend * t * t
            c = polar(0.12 + 0.88 * t, ang)
            w = 0.17 * math.sin(math.pi * t) ** 0.7 * (1.0 if j % 2 else 0.45)
            nx, ny = -math.sin(ang), math.cos(ang)
            left.append((c[0] + nx * w, c[1] + ny * w))
            right.append((c[0] - nx * w, c[1] - ny * w))
            mid.append(c)
        prims.append(poly("crown", left + list(reversed(right))[1:-1]))
        prims.append(poly("line", mid[:-2], True))
    prims.append(circle("trunk", (0, 0), 0.13))
    return prims


def conifere():
    star = [polar(1.0 if i % 2 == 0 else 0.86, math.pi * i / 16) for i in range(32)]
    inner = [polar(0.56 if i % 2 == 0 else 0.42, math.pi * i / 9 + 0.17) for i in range(18)]
    return [poly("crown", star), poly("line", inner), circle("trunk", (0, 0), 0.07)]


TREES = [
    {"id": "feuillu", "nom": "Feuillu", "r": 2.5, "couleur": "#4D7E44", "prims": feuillu()},
    {"id": "pin", "nom": "Pin parasol", "r": 3.2, "couleur": "#3E6A3B", "prims": pin_parasol()},
    {"id": "olivier", "nom": "Olivier", "r": 2.0, "couleur": "#8C9C70", "prims": olivier()},
    {"id": "palmier", "nom": "Palmier", "r": 2.4, "couleur": "#5F8F45", "prims": palmier()},
    {"id": "conifere", "nom": "Conifère", "r": 1.6, "couleur": "#355E44", "prims": conifere()},
]
for t in TREES:
    t["bloc"] = "ARB_" + t["id"].upper()

# ---------------------------------------------------------------------------
# Marquage au sol : flèche directionnelle droite, pointe vers +x, centrée.
# ---------------------------------------------------------------------------

ARROW_L, ARROW_W = 3.0, 0.9
ARROW = [(-1.5, -0.15), (0.3, -0.15), (0.3, -0.45), (1.5, 0.0), (0.3, 0.45), (0.3, 0.15), (-1.5, 0.15)]
MARKINGS = [{"id": "fleche", "nom": "Flèche directionnelle", "bloc": "SIG_FLECHE", "L": ARROW_L, "W": ARROW_W,
             "prims": [poly("paint", ARROW)]}]

if __name__ == "__main__":
    out = pathlib.Path(__file__).resolve().parent.parent / "src" / "symbols.json"
    data = {"vehicules": VEHICLES, "arbres": TREES, "marquages": MARKINGS}
    out.write_text(json.dumps(data, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    print(out, out.stat().st_size, "octets,", len(VEHICLES), "véhicules,", len(TREES), "arbres,", len(MARKINGS), "marquage")
