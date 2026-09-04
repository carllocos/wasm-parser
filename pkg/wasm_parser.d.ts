/* tslint:disable */
/* eslint-disable */

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
