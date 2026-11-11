#!/usr/bin/env sh
# Compile le moteur en WebAssembly et assemble dist/index.html.
# Si wasm2js et terser (binaryen, npm) sont installés, produit aussi dist/trame.html avec un repli JavaScript.
set -e
cd "$(dirname "$0")"
cargo build --release --lib --target wasm32-unknown-unknown
WASM=target/wasm32-unknown-unknown/release/parkimo.wasm
FEATS="--enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals --enable-multivalue --enable-reference-types"
if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -O3 $FEATS "$WASM" -o target/parkimo.opt.wasm
  WASM=target/parkimo.opt.wasm
fi
FALLBACK=""
if command -v wasm2js >/dev/null 2>&1 && command -v terser >/dev/null 2>&1; then
  wasm-opt $FEATS --signext-lowering --llvm-memory-copy-fill-lowering --llvm-nontrapping-fptoint-lowering -O3 \
    target/wasm32-unknown-unknown/release/parkimo.wasm -o target/parkimo.mvp.wasm
  wasm2js -O3 target/parkimo.mvp.wasm -o target/parkimo.w2j.mjs
  python3 tools/wrap_w2j.py target/parkimo.w2j.mjs target/parkimo.w2j.js
  terser target/parkimo.w2j.js -c passes=2 -m -o target/parkimo.fallback.js
  FALLBACK=target/parkimo.fallback.js
fi
python3 tools/bundle.py "$WASM" $FALLBACK
