use wasmparser::{BlockType, ValType};

/// Maps a wasmparser `ValType` to the string keys understood by
/// `WASM.typing` in `src/webassembly/wasm.ts` (i32, i64, f32, f64, anyfunc,
/// externref).
pub fn val_type_to_string(vt: ValType) -> Result<String, String> {
    match vt {
        ValType::I32 => Ok("i32".to_string()),
        ValType::I64 => Ok("i64".to_string()),
        ValType::F32 => Ok("f32".to_string()),
        ValType::F64 => Ok("f64".to_string()),
        ValType::Ref(r) => {
            if r.is_func_ref() {
                Ok("anyfunc".to_string())
            } else if r.is_extern_ref() {
                Ok("externref".to_string())
            } else {
                Err(format!("unsupported reference type {r:?}"))
            }
        }
        ValType::V128 => Err("SIMD (v128) types are not supported".to_string()),
    }
}

/// Maps a wasmparser `BlockType` to an optional WASM.Type string. Only the
/// single-result-value form is supported, matching the pre-existing
/// TypeScript parser (multi-value block types were never handled there
/// either).
pub fn block_type_to_string(bt: BlockType) -> Result<Option<String>, String> {
    match bt {
        BlockType::Empty => Ok(None),
        BlockType::Type(vt) => Ok(Some(val_type_to_string(vt)?)),
        BlockType::FuncType(_) => {
            Err("multi-value block types are not supported".to_string())
        }
    }
}
