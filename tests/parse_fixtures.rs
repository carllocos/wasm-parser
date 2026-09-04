use std::fs;
use std::path::Path;

/// Sanity-checks the parser against a handful of the wasmito repo's existing
/// `.wasm` test fixtures (compiled from Rust, C, Zig, AssemblyScript, and
/// hand-written WAT). These paths are relative to this crate
/// (`wasmito/wasm-parser`), i.e. `../test/data/...` in the parent repo.
fn fixture(rel: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(rel);
    fs::read(&path).unwrap_or_else(|e| panic!("failed to read fixture {path:?}: {e}"))
}

#[test]
fn parses_rust_blink_lambda() {
    let bytes = fixture("test/data/rust/blink_lambda/blink_lambda.wasm");
    let module = wasm_parser::parse_wasm_module_native(&bytes).expect("should parse");
    assert_eq!(module.funcs.len(), 8);
    assert_eq!(module.func_imports.len(), 3);
    assert_eq!(module.globals.len(), 3);
}

#[test]
fn parses_wat_fac() {
    let bytes = fixture("test/data/wat/fac/fac.wasm");
    let module = wasm_parser::parse_wasm_module_native(&bytes).expect("should parse");
    assert_eq!(module.funcs.len(), 2);
}

#[test]
fn parses_wat_dimmer_with_tables_and_elements() {
    let bytes = fixture("test/data/wat/dimmer/dimmer.wasm");
    let module = wasm_parser::parse_wasm_module_native(&bytes).expect("should parse");
    assert_eq!(module.funcs.len(), 5);
    assert_eq!(module.func_imports.len(), 8);
    assert_eq!(module.globals.len(), 5);
}

#[test]
fn parses_assemblyscript_module_with_bulk_memory_ops() {
    let bytes = fixture("test/data/assemblyscript/blink_intermittent_if/blink_intermittent_if.wasm");
    let module = wasm_parser::parse_wasm_module_native(&bytes).expect("should parse");
    assert_eq!(module.funcs.len(), 33);
}

#[test]
fn every_instruction_has_a_well_formed_byte_range() {
    let bytes = fixture("test/data/rust/fac/fac.wasm");
    let module = wasm_parser::parse_wasm_module_native(&bytes).expect("should parse");
    fn check(instrs: &[wasm_parser::RustInstr]) {
        for i in instrs {
            assert!(i.start < i.end, "instr {:?} has start >= end", i.kind);
            if let Some(body) = &i.body {
                check(body);
            }
            if let Some(c) = &i.consequence {
                check(c);
            }
            if let Some(a) = &i.alternative {
                check(a);
            }
        }
    }
    for f in &module.funcs {
        check(&f.body);
    }
    for g in &module.globals {
        check(&g.init);
    }
}
