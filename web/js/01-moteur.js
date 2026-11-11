"use strict";
// ============================================================================
// Moteur Rust (WebAssembly) : JSON en entrée, JSON en sortie.
// ============================================================================
const Engine = {
  ex: null,
  kind: "WebAssembly",
  async load() {
    try {
      const b64 = document.getElementById("engine-wasm").textContent.trim();
      const bin = atob(b64);
      const bytes = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
      const { instance } = await WebAssembly.instantiate(bytes, {});
      this.bytes = bytes;
      this.ex = instance.exports;
    } catch (e) {
      if (!window.TrameJS) await loadLazyEngine();
      if (!window.TrameJS) throw e;
      this.ex = window.TrameJS; this.kind = "JavaScript";
    }
  },
  call(fn, obj) {
    const ex = this.ex;
    const b = new TextEncoder().encode(JSON.stringify(obj));
    const p = ex.alloc(b.length);
    new Uint8Array(ex.memory.buffer, p, b.length).set(b);
    const o = ex[fn](p, b.length);
    const out = new TextDecoder().decode(new Uint8Array(ex.memory.buffer, o, ex.out_len()));
    ex.dealloc(p, b.length);
    return JSON.parse(out);
  },
  solve(obj) { return this.call("solve_json", obj); },
  union(polys) { return this.call("union_json", { polys }); },
};

// Repli JavaScript du moteur, injecté seulement si WebAssembly est refusé (mode Isolement d'iOS, ancien navigateur).
function loadLazyEngine() {
  const el = document.getElementById("engine-js-lazy");
  if (!el) return Promise.resolve();
  return new Promise((res) => {
    const sc = document.createElement("script");
    sc.src = URL.createObjectURL(new Blob([el.textContent], { type: "text/javascript" }));
    sc.onload = res; sc.onerror = res;
    document.head.appendChild(sc);
  });
}
// Une promesse qui ne répond pas ne doit jamais bloquer le démarrage.
const withTimeout = (p, ms) => Promise.race([p, new Promise((res) => setTimeout(res, ms))]);

