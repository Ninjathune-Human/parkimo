// Plateau : ce que la vue 3D lit du squelette, en données simples (nombres, points, drapeaux). C'est le seul fichier
// qui connaît les noms internes du squelette (S, calc, SYSTEMES, escalier()...) : si le squelette les change, c'est
// ici qu'on le voit, et l'essai `essai_vue3d.py` le signale. La scène (scene3d.js) ne lit que ce plateau.
/* global S, calc, SYSTEMES, ESCALIER, PENTE, escalier, escLong, hauteurRampe, etage */

const pt = (p) => [+p[0], +p[1]];
const quad = (q) => q.map(pt);

export function extraire() {
  if (!calc) return null;
  const sys = SYSTEMES[S.systeme] || SYSTEMES.poteaux, E = etage(sys), esc = escalier(sys), nb = S.branches.length;
  return {
    E, dalle: sys.dalle, retombee: sys.retombee, section: sys.section, voiles: !!sys.voile,
    parc: S.parc === "ventile" ? "ventile" : "enterre",
    niveaux: Math.max(1, Math.min(12, Math.round(S.niveaux || 4))),
    pente: PENTE,
    // Chaussées des branches (dans l'ordre des branches), puis carrefours. Une branche peut être une rampe.
    chaussees: calc.ch.map((q, k) => ({ q: quad(q), rampe: k < nb && !!S.branches[k].rampe })),
    rangees: calc.bandes.map(quad),
    places: calc.pl.map((o) => ({ q: quad(o.q), rouge: calc.loin.includes(o.q) || calc.genees.includes(o.q) })),
    pietons: [...calc.trottoirs.map((t) => quad(t.q)), ...calc.raccords.map(quad), ...(calc.passagesP || []).map((x) => quad(x.corps))],
    zebres: calc.zebres.map(quad),
    // Rampes posées dans une rangée : la voie `q` monte de q[0]-q[3] vers q[1]-q[2], dans le sens de la branche.
    rampes: calc.rampes.map((x, k) => {
      const H = hauteurRampe((S.rampes[k] || {}).type);
      return { q: quad(x.q), corps: quad(x.corps), hauteur: H, hors: !!x.hors, raide: !x.hors && H / x.L > PENTE + 1e-4, cle: `rampe:${k}` };
    }),
    // Rampes branchées : toute la chaussée de la branche monte, du premier nœud vers le second.
    rampesVoie: calc.rampesVoie.map((x) => ({ q: quad(x.q), hauteur: hauteurRampe(S.branches[x.i].rampe), sens: x.sens, raide: x.pente > PENTE + 1e-4, cle: `branche:${x.i}` })),
    helices: calc.helices.map((x, k) => ({ p: pt(x.p), int: x.int, ext: x.ext, axe: x.axe, sens: x.sens, raccord: !!x.raccord, raide: x.pente > PENTE + 1e-4, cle: `helice:${k}` })),
    // Escaliers : un repère local (origine au coin côté allée, `ua` le long de la rangée, `uv` vers le fond).
    escaliers: calc.escaliers.map((x, k) => {
      if (x.hors) return { hors: true, corps: quad(x.corps), cle: `escalier:${k}` };
      const u = x.r.u, O = x.r.P(x.a, x.s * x.h);
      return { hors: false, corps: quad(x.corps), O: pt(O), ua: pt(u), uv: [-u[1] * x.s, u[0] * x.s], L: x.L, prof: x.prof, Ls: escLong(), principal: !!x.principal, cle: `escalier:${k}` };
    }),
    marches: { n: esc.n, h: esc.h, n1: esc.n1, n2: esc.n2, course: esc.course, ...ESCALIER },
    poteaux: (S.pn || []).map((p, k) => (p ? { p: pt(p), conflit: calc.conflits.includes(p), cle: `poteau:${k}` } : null)).filter(Boolean),
    poutres: (S.pb || []).filter((b) => S.pn[b.a] && S.pn[b.b]).map((b) => ({ a: pt(S.pn[b.a]), b: pt(S.pn[b.b]), plein: !!b.plein, voile: !!b.voile, noyau: !!b.noyau })),
  };
}
