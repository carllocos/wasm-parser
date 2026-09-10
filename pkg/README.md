# wasm-parser

A Rust `.wasm` module parser built on
[`wasmparser`](https://github.com/bytecodealliance/wasm-tools/tree/main/crates/wasmparser)
(from bytecodealliance/wasm-tools), compiled to WebAssembly with
[`wasm-bindgen`](https://github.com/rustwasm/wasm-bindgen) so it can be used
from the wasmito TypeScript codebase as a drop-in replacement for the
`@webassemblyjs/wasm-parser`-backed parser.

It exposes two functions:

- `parseWasmModule(bytes: Uint8Array): string` decodes a WebAssembly binary
  and returns a JSON string describing its sections, types, imports,
  functions (with a fully nested instruction tree, byte-accurate start/end
  addresses, and resolved names from the `name` custom section), globals,
  linear memories (`memories`/`memoryImports`, with initial/maximum page
  counts, `shared`, `memory64`, plus a top-level `initialMemoryPages`
  shortcut — `0` if the module declares no memory), exports, and active
  element segments. See `src/model.rs` for the exact JSON shape.
- `getInitialMemoryPages(bytes: Uint8Array): number` is a convenience that
  returns just `initialMemoryPages` above without needing to parse the full
  JSON module.

## Building

```sh
npm run build:wasm-parser
```

This runs `wasm-pack build --target nodejs --out-dir pkg`, producing a
`pkg/` directory (a `.wasm` binary plus a synchronous CommonJS/`.d.ts` glue
module — the `nodejs` wasm-bindgen target loads the wasm module
synchronously via `fs`, so no `await`/async init is needed). The root
`package.json` depends on it via `"wasm-parser": "file:./wasm-parser/pkg"`,
so run this build once (and again after changing any Rust source) before
`npm install`/`npm run build`.

## Using it from TypeScript

This crate is a low-level parser only — it does not know about wasmito's
`WasmInstruction`/`WASMFunction`/`WasmModule` class hierarchy. The
TypeScript hydration layer that turns its JSON output into those classes
lives in `src/webassembly/parsers/wasm_module_parser_rust.ts`, and
`src/webassembly/wasm/wasm_module_rust.ts` is the Rust-backed counterpart of
`wasm_module.ts` that uses it. Both are plain copies of the original
`@webassemblyjs/wasm-parser`-based files, adapted to consume this crate's
output — the original files are untouched.

## Rust-side layout

- `src/model.rs` — the JSON data model (serde `Serialize` structs).
- `src/valtype.rs` — `wasmparser` value/block type → `WASM.Type` string
  mapping.
- `src/instr.rs` — turns a flat `wasmparser` operator stream (a function
  body, a global init expression, or an element-segment offset expression)
  into the nested `block`/`loop`/`if` instruction tree the TypeScript side
  expects, using an explicit frame stack (`wasmparser` only exposes a flat,
  offset-tracked token stream — nesting is reconstructed here).
- `src/module.rs` — walks every `wasmparser::Payload` of a module and
  assembles the top-level `RustModule`.
- `src/lib.rs` — the `wasm-bindgen` entry point.

Instruction opcodes are encoded as the raw WebAssembly binary opcode byte
(plus, for 0xFC-prefixed multi-byte instructions, the secondary byte). The
TypeScript side resolves these back to a `WasmOpcode` via the existing
`wasmOpcodeFromNr` helper in `wasm_opcode.ts`, so the numbering in
`src/instr.rs` must stay in sync with the `WasmCode` constants there.

## Testing

```sh
cargo test
```

Compares parsed output against the repo's existing `.wasm` fixtures under
`test/data/`.

### TypeScript bindings

`test-ts/` has example/regression tests for the generated `pkg/` bindings
(`getInitialMemoryPages`, `parseWasmModule`), useful as usage examples for
consumers. They run directly against the `.ts` sources via Node's built-in
type-stripping support (no bundler/ts-node needed), so a recent Node (22.18+,
or any version with unflagged `--experimental-strip-types`) is required:

```sh
npm install                                    # installs typescript/@types/node dev dependencies
wasm-pack build --target nodejs --out-dir pkg  # see "Building" above; rebuilds pkg/ from src/
npm run test:ts
npm run typecheck:ts  # optional: tsc --noEmit over test-ts/
```
