/* tslint:disable */
/* eslint-disable */

/**
 * Returns the number of 64KiB pages the module's linear memory was
 * initially declared with, i.e. the `initial` field of its `MemoryType`
 * (whether the memory is defined locally or imported). Returns `0` if the
 * module declares no memory at all.
 *
 * This is a convenience for callers that only need this one figure and
 * would otherwise have to parse the full `parseWasmModule` JSON output and
 * read `memories[0].initialPages` / `memoryImports[0].initialPages`
 * themselves.
 */
export function getInitialMemoryPages(bytes: Uint8Array): number;

/**
 * Parses a WebAssembly binary and returns a JSON string describing its
 * module structure (sections, types, imports, functions with their nested
 * instruction trees, globals, exports, and elements).
 *
 * This is the Rust equivalent of the `@webassemblyjs/wasm-parser`-backed
 * `parseWasmModule` in `src/webassembly/parsers/wasm_module_parser.ts`. The
 * TypeScript hydration layer in
 * `src/webassembly/parsers/wasm_module_parser_rust.ts` turns this JSON back
 * into the same `ParsedModule` shape (and, from there, the same
 * `WasmInstruction`/`WASMFunction`/`WasmModule` class instances) that the
 * JS-parser-backed code produces.
 */
export function parseWasmModule(bytes: Uint8Array): string;
