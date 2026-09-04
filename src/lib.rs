mod instr;
mod model;
mod module;
mod valtype;

pub use model::{RustFunc, RustGlobal, RustInstr, RustModule};

use wasm_bindgen::prelude::*;

/// Parses a WebAssembly binary and returns a JSON string describing its
/// module structure (sections, types, imports, functions with their nested
/// instruction trees, globals, exports, and elements).
///
/// This is the Rust equivalent of the `@webassemblyjs/wasm-parser`-backed
/// `parseWasmModule` in `src/webassembly/parsers/wasm_module_parser.ts`. The
/// TypeScript hydration layer in
/// `src/webassembly/parsers/wasm_module_parser_rust.ts` turns this JSON back
/// into the same `ParsedModule` shape (and, from there, the same
/// `WasmInstruction`/`WASMFunction`/`WasmModule` class instances) that the
/// JS-parser-backed code produces.
#[wasm_bindgen(js_name = parseWasmModule)]
pub fn parse_wasm_module(bytes: &[u8]) -> Result<String, JsError> {
    let parsed = module::parse(bytes).map_err(|e| JsError::new(&e))?;
    serde_json::to_string(&parsed).map_err(|e| JsError::new(&e.to_string()))
}

/// Native (non-wasm) entry point used by the crate's own Rust unit/integration
/// tests, where going through `wasm-bindgen`'s JS glue isn't necessary.
pub fn parse_wasm_module_native(bytes: &[u8]) -> Result<model::RustModule, String> {
    module::parse(bytes)
}
