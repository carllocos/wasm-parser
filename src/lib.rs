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

/// Returns the number of 64KiB pages the module's linear memory was
/// initially declared with, i.e. the `initial` field of its `MemoryType`
/// (whether the memory is defined locally or imported). Returns `0` if the
/// module declares no memory at all.
///
/// This is a convenience for callers that only need this one figure and
/// would otherwise have to parse the full `parseWasmModule` JSON output and
/// read `memories[0].initialPages` / `memoryImports[0].initialPages`
/// themselves.
#[wasm_bindgen(js_name = getInitialMemoryPages)]
pub fn get_initial_memory_pages(bytes: &[u8]) -> Result<u32, JsError> {
    let parsed = module::parse(bytes).map_err(|e| JsError::new(&e))?;
    Ok(parsed.initial_memory_pages)
}
