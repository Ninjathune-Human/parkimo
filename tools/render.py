"""Rendu de contrôle d'un plan produit par `examples/run` : python3 render.py plan.out.json image.png"""
import json
import sys

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Circle, Polygon
from matplotlib.path import Path
from matplotlib.patches import PathPatch

d = json.load(open(sys.argv[1]))
L = d["layout"]
fig, ax = plt.subplots(figsize=(14, 10), dpi=110)
ax.set_aspect("equal")
ax.axis("off")


def poly(pts, **kw):
    ax.add_patch(Polygon(pts, closed=True, **kw))


def rings(rs, **kw):
    verts, codes = [], []
    for r in rs:
        verts += r + [r[0]]
        codes += [Path.MOVETO] + [Path.LINETO] * (len(r) - 1) + [Path.CLOSEPOLY]
    if verts:
        ax.add_patch(PathPatch(Path(verts, codes), **kw))


poly(d["site_parcel"], fc="#b9d8a6", ec="#2b3a2b", lw=1.2)
for e in d["exclusions"]:
    poly(e, fc="#9b5a8f", ec="none", alpha=0.75)
for l in d["landscapes"]:
    poly(l, fc="#5c9a4a", ec="none")
for b in d["buildings"]:
    poly(b, fc="#4a524e", ec="#111", lw=1)
for r in L["setback_line"]:
    poly(r, fc="none", ec="#e08a2c", lw=0.8, ls="--")
rings(L["paved"], fc="#8d9196", ec="none")
for q in L["pads"] + L["cross"] + L["entries"]:
    poly(q, fc="#8d9196", ec="none")
for s in L["stalls"]:
    q = s["q"]
    col = {0: "#8d9196", 1: "#2f6fd6", 2: "#3aa35b", 3: "#1d9bd1"}[s["k"]]
    poly(q, fc=col, ec="none")
    ax.plot([q[3][0], q[0][0], q[1][0], q[2][0]], [q[3][1], q[0][1], q[1][1], q[2][1]], color="white", lw=0.5)
for q in L["islands"]:
    poly(q, fc="#6fae5c", ec="none")
for t in L["trees"]:
    ax.add_patch(Circle(t, 1.6, fc="#2e7d32", ec="#1b5e20", lw=0.4))
for e in L.get("entrances", []):
    ax.add_patch(Circle(e, 1.2, fc="#e08a2c"))
m = L["metrics"]
ax.set_title(
    f"{m['stalls']} places  |  {m['m2_paved_per_stall']} m² enrobé/place  |  PMR {m['pmr']}  IRVE {m['irve']}  arbres {m['trees']}  |  angle {L['angle']}°  {L['warnings']}",
    fontsize=10,
)
ax.autoscale_view()
plt.savefig(sys.argv[2], bbox_inches="tight")
