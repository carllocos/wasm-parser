//! JSON data model shared with the TypeScript hydration layer
//! (`src/webassembly/parsers/wasm_module_parser_rust.ts` in the wasmito repo).
//!
//! Every address (`start`/`end`/`*_address`) is an absolute byte offset into
//! the original `.wasm` binary. Instruction opcodes are encoded as the raw
//! WebAssembly binary opcode byte (`opcode`) plus, for the 0xFC-prefixed
//! multi-byte instructions, the secondary byte (`sub_opcode`). The TS side
//! resolves these numbers back to a `WasmOpcode` via the existing
//! `wasmOpcodeFromNr` helper, so the numbering here must match the
//! `WasmCode` constants in `src/webassembly/wasm/wasm_opcode.ts` exactly.

use serde::Serialize;

#[derive(Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RustInstr {
    pub kind: &'static str,
    pub opcode: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_opcode: Option<u32>,
    pub start: u32,
    pub end: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub func_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub func_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i32_value: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i64_low: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i64_high: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f32_value: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f64_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Vec<RustInstr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consequence: Option<Vec<RustInstr>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternative: Option<Vec<RustInstr>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RustSection {
    pub section: String,
    pub start_address: u32,
    pub end_address: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RustFuncType {
    pub id: u32,
    pub params: Vec<String>,
    pub results: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RustFuncImport {
    pub module: String,
    pub name: String,
    pub type_index: u32,
    pub start_address: u32,
    pub end_address: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RustTableImport {
    pub module: String,
    pub name: String,
    pub id: u32,
    pub start_address: u32,
    pub end_address: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RustFuncExport {
    pub name: String,
    pub func_index: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RustTableExport {
    pub name: String,
    pub id: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RustElement {
    pub table_id: u32,
    pub funcs: Vec<u32>,
    pub start_address: u32,
    pub end_address: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RustLocal {
    pub index: u32,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RustFunc {
    pub id: u32,
    pub name: String,
    pub type_index: u32,
    pub locals: Vec<RustLocal>,
    pub body: Vec<RustInstr>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RustGlobal {
    pub value_type: String,
    pub mutable: bool,
    pub name: Option<String>,
    pub init: Vec<RustInstr>,
    pub start_address: u32,
    pub end_address: u32,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RustNameEntry {
    pub index: u32,
    pub value: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RustLocalNameEntry {
    pub function_index: u32,
    pub local_index: u32,
    pub value: String,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RustModule {
    pub sections: Vec<RustSection>,
    pub types: Vec<RustFuncType>,
    pub func_imports: Vec<RustFuncImport>,
    pub table_imports: Vec<RustTableImport>,
    pub funcs: Vec<RustFunc>,
    pub globals: Vec<RustGlobal>,
    pub exported_funcs: Vec<RustFuncExport>,
    pub table_exports: Vec<RustTableExport>,
    pub elements: Vec<RustElement>,
    pub func_names: Vec<RustNameEntry>,
    pub local_names: Vec<RustLocalNameEntry>,
}
