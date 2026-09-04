use crate::model::RustInstr;
use crate::valtype::block_type_to_string;
use wasmparser::{Operator, OperatorsReader};

/// Parses a flat operator stream (a function body, a global init expression,
/// or an element-segment offset expression) into the nested instruction tree
/// shape expected by the TypeScript hydration layer. Nesting for
/// `block`/`loop`/`if` is reconstructed from the flat, offset-tracked
/// operator stream using an explicit frame stack (mirroring how
/// `WasmInstruction.subInstructions` nests things on the TypeScript side).
pub fn parse_instr_stream(reader: OperatorsReader) -> Result<Vec<RustInstr>, String> {
    let tokens = collect_tokens(reader)?;
    build_tree(tokens)
}

fn collect_tokens(reader: OperatorsReader<'_>) -> Result<Vec<(Operator<'_>, u32, u32)>, String> {
    let mut tokens = Vec::new();
    let mut pending: Option<(Operator, u32)> = None;
    for item in reader.into_iter_with_offsets() {
        let (op, offset) = item.map_err(|e| e.to_string())?;
        let start = offset as u32;
        if let Some((prev_op, prev_start)) = pending.take() {
            tokens.push((prev_op, prev_start, start));
        }
        pending = Some((op, start));
    }
    if let Some((prev_op, prev_start)) = pending {
        // The last operator of any valid function body / const expr is
        // always a 1-byte `end` with no trailing operator to look up.
        tokens.push((prev_op, prev_start, prev_start + 1));
    }
    Ok(tokens)
}

struct IfFrame {
    start: u32,
    result_type: Option<String>,
    consequence: Vec<RustInstr>,
    alternative: Vec<RustInstr>,
    in_alt: bool,
}

enum Frame {
    Top(Vec<RustInstr>),
    Block { start: u32, body: Vec<RustInstr> },
    Loop { start: u32, result_type: Option<String>, body: Vec<RustInstr> },
    If(IfFrame),
}

impl Frame {
    fn active_list(&mut self) -> &mut Vec<RustInstr> {
        match self {
            Frame::Top(v) => v,
            Frame::Block { body, .. } => body,
            Frame::Loop { body, .. } => body,
            Frame::If(f) => {
                if f.in_alt {
                    &mut f.alternative
                } else {
                    &mut f.consequence
                }
            }
        }
    }
}

fn build_tree(tokens: Vec<(Operator, u32, u32)>) -> Result<Vec<RustInstr>, String> {
    let mut stack: Vec<Frame> = vec![Frame::Top(Vec::new())];

    for (op, start, end) in tokens {
        match op {
            Operator::Block { .. } => {
                stack.push(Frame::Block {
                    start,
                    body: Vec::new(),
                });
            }
            Operator::Loop { blockty } => {
                let result_type = block_type_to_string(blockty)?;
                stack.push(Frame::Loop {
                    start,
                    result_type,
                    body: Vec::new(),
                });
            }
            Operator::If { blockty } => {
                let result_type = block_type_to_string(blockty)?;
                stack.push(Frame::If(IfFrame {
                    start,
                    result_type,
                    consequence: Vec::new(),
                    alternative: Vec::new(),
                    in_alt: false,
                }));
            }
            Operator::Else => match stack.last_mut() {
                Some(Frame::If(f)) => f.in_alt = true,
                _ => return Err("'else' encountered outside of an 'if' block".to_string()),
            },
            Operator::End => {
                let end_leaf = leaf_instr(&Operator::End, start, end)?;
                if stack.len() == 1 {
                    stack.last_mut().unwrap().active_list().push(end_leaf);
                } else {
                    let finished = stack.pop().unwrap();
                    let instr = match finished {
                        Frame::Block {
                            start: bstart,
                            mut body,
                        } => {
                            body.push(end_leaf);
                            RustInstr {
                                kind: "block",
                                opcode: 0x02,
                                start: bstart,
                                end,
                                body: Some(body),
                                ..Default::default()
                            }
                        }
                        Frame::Loop {
                            start: lstart,
                            result_type,
                            mut body,
                        } => {
                            body.push(end_leaf);
                            RustInstr {
                                kind: "loop",
                                opcode: 0x03,
                                start: lstart,
                                end,
                                result_type,
                                body: Some(body),
                                ..Default::default()
                            }
                        }
                        Frame::If(mut f) => {
                            if f.in_alt {
                                f.alternative.push(end_leaf);
                            } else {
                                f.consequence.push(end_leaf);
                            }
                            RustInstr {
                                kind: "if",
                                opcode: 0x04,
                                start: f.start,
                                end,
                                result_type: f.result_type,
                                consequence: Some(f.consequence),
                                alternative: Some(f.alternative),
                                ..Default::default()
                            }
                        }
                        Frame::Top(_) => unreachable!("balanced by construction"),
                    };
                    stack.last_mut().unwrap().active_list().push(instr);
                }
            }
            other => {
                let leaf = leaf_instr(&other, start, end)?;
                stack.last_mut().unwrap().active_list().push(leaf);
            }
        }
    }

    match stack.pop() {
        Some(Frame::Top(v)) if stack.is_empty() => Ok(v),
        _ => Err("unbalanced block/loop/if structure in instruction stream".to_string()),
    }
}

fn base(kind: &'static str, opcode: u32, start: u32, end: u32) -> RustInstr {
    RustInstr {
        kind,
        opcode,
        start,
        end,
        ..Default::default()
    }
}

fn indexed(kind: &'static str, opcode: u32, index: u32, start: u32, end: u32) -> RustInstr {
    RustInstr {
        index: Some(index),
        ..base(kind, opcode, start, end)
    }
}

fn mem_arg_offset(memarg: wasmparser::MemArg) -> u32 {
    memarg.offset as u32
}

/// Builds a `RustInstr` for any non-structural (non block/loop/if/else)
/// operator. The `opcode`/`sub_opcode` values assigned here MUST match the
/// `WasmCode` constants in `src/webassembly/wasm/wasm_opcode.ts`.
fn leaf_instr(op: &Operator, start: u32, end: u32) -> Result<RustInstr, String> {
    use Operator::*;
    let i = match op {
        Unreachable => base("plain", 0x00, start, end),
        Nop => base("plain", 0x01, start, end),
        End => base("plain", 0x0b, start, end),
        Return => base("plain", 0x0f, start, end),
        Drop => base("plain", 0x1a, start, end),
        Select => base("plain", 0x1b, start, end),

        Br { relative_depth } => RustInstr {
            target: Some(*relative_depth),
            ..base("branch", 0x0c, start, end)
        },
        BrIf { relative_depth } => RustInstr {
            target: Some(*relative_depth),
            ..base("branch", 0x0d, start, end)
        },
        BrTable { targets } => {
            let mut all: Vec<u32> = targets
                .targets()
                .collect::<Result<Vec<u32>, _>>()
                .map_err(|e| e.to_string())?;
            all.push(targets.default());
            RustInstr {
                targets: Some(all),
                ..base("branchTable", 0x0e, start, end)
            }
        }

        Call { function_index } => RustInstr {
            func_index: Some(*function_index),
            func_name: Some(format!("func_{function_index}")),
            ..base("call", 0x10, start, end)
        },
        CallIndirect { type_index, .. } => RustInstr {
            type_index: Some(*type_index),
            ..base("callIndirect", 0x11, start, end)
        },

        LocalGet { local_index } => indexed("indexed", 0x20, *local_index, start, end),
        LocalSet { local_index } => indexed("indexed", 0x21, *local_index, start, end),
        LocalTee { local_index } => indexed("indexed", 0x22, *local_index, start, end),
        GlobalGet { global_index } => indexed("indexed", 0x23, *global_index, start, end),
        GlobalSet { global_index } => indexed("indexed", 0x24, *global_index, start, end),

        TableGet { table } => indexed("indexed", 0x25, *table, start, end),
        TableSet { table } => indexed("indexed", 0x26, *table, start, end),

        I32Load { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x28, start, end)
        },
        I64Load { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x29, start, end)
        },
        F32Load { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x2a, start, end)
        },
        F64Load { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x2b, start, end)
        },
        I32Load8S { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x2c, start, end)
        },
        I32Load8U { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x2d, start, end)
        },
        I32Load16S { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x2e, start, end)
        },
        I32Load16U { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x2f, start, end)
        },
        I64Load8S { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x30, start, end)
        },
        I64Load8U { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x31, start, end)
        },
        I64Load16S { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x32, start, end)
        },
        I64Load16U { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x33, start, end)
        },
        I64Load32S { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x34, start, end)
        },
        I64Load32U { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x35, start, end)
        },
        I32Store { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x36, start, end)
        },
        I64Store { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x37, start, end)
        },
        F32Store { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x38, start, end)
        },
        F64Store { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x39, start, end)
        },
        I32Store8 { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x3a, start, end)
        },
        I32Store16 { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x3b, start, end)
        },
        I64Store8 { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x3c, start, end)
        },
        I64Store16 { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x3d, start, end)
        },
        I64Store32 { memarg } => RustInstr {
            offset: Some(mem_arg_offset(*memarg)),
            ..base("memOp", 0x3e, start, end)
        },

        MemorySize { .. } => base("plain", 0x3f, start, end),
        MemoryGrow { .. } => base("plain", 0x40, start, end),

        I32Const { value } => RustInstr {
            i32_value: Some(*value),
            ..base("constI32", 0x41, start, end)
        },
        I64Const { value } => {
            let bits = *value as u64;
            RustInstr {
                i64_low: Some((bits & 0xFFFF_FFFF) as u32 as i32),
                i64_high: Some((bits >> 32) as u32 as i32),
                ..base("constI64", 0x42, start, end)
            }
        }
        F32Const { value } => RustInstr {
            f32_value: Some(f32::from_bits(value.bits())),
            ..base("constF32", 0x43, start, end)
        },
        F64Const { value } => RustInstr {
            f64_value: Some(f64::from_bits(value.bits())),
            ..base("constF64", 0x44, start, end)
        },

        I32Eqz => base("plain", 0x45, start, end),
        I32Eq => base("plain", 0x46, start, end),
        I32Ne => base("plain", 0x47, start, end),
        I32LtS => base("plain", 0x48, start, end),
        I32LtU => base("plain", 0x49, start, end),
        I32GtS => base("plain", 0x4a, start, end),
        I32GtU => base("plain", 0x4b, start, end),
        I32LeS => base("plain", 0x4c, start, end),
        I32LeU => base("plain", 0x4d, start, end),
        I32GeS => base("plain", 0x4e, start, end),
        I32GeU => base("plain", 0x4f, start, end),

        I64Eqz => base("plain", 0x50, start, end),
        I64Eq => base("plain", 0x51, start, end),
        I64Ne => base("plain", 0x52, start, end),
        I64LtS => base("plain", 0x53, start, end),
        I64LtU => base("plain", 0x54, start, end),
        I64GtS => base("plain", 0x55, start, end),
        I64GtU => base("plain", 0x56, start, end),
        I64LeS => base("plain", 0x57, start, end),
        I64LeU => base("plain", 0x58, start, end),
        I64GeS => base("plain", 0x59, start, end),
        I64GeU => base("plain", 0x5a, start, end),

        F32Eq => base("plain", 0x5b, start, end),
        F32Ne => base("plain", 0x5c, start, end),
        F32Lt => base("plain", 0x5d, start, end),
        F32Gt => base("plain", 0x5e, start, end),
        F32Le => base("plain", 0x5f, start, end),
        F32Ge => base("plain", 0x60, start, end),

        F64Eq => base("plain", 0x61, start, end),
        F64Ne => base("plain", 0x62, start, end),
        F64Lt => base("plain", 0x63, start, end),
        F64Gt => base("plain", 0x64, start, end),
        F64Le => base("plain", 0x65, start, end),
        F64Ge => base("plain", 0x66, start, end),

        I32Clz => base("plain", 0x67, start, end),
        I32Ctz => base("plain", 0x68, start, end),
        I32Popcnt => base("plain", 0x69, start, end),
        I32Add => base("plain", 0x6a, start, end),
        I32Sub => base("plain", 0x6b, start, end),
        I32Mul => base("plain", 0x6c, start, end),
        I32DivS => base("plain", 0x6d, start, end),
        I32DivU => base("plain", 0x6e, start, end),
        I32RemS => base("plain", 0x6f, start, end),
        I32RemU => base("plain", 0x70, start, end),
        I32And => base("plain", 0x71, start, end),
        I32Or => base("plain", 0x72, start, end),
        I32Xor => base("plain", 0x73, start, end),
        I32Shl => base("plain", 0x74, start, end),
        I32ShrS => base("plain", 0x75, start, end),
        I32ShrU => base("plain", 0x76, start, end),
        I32Rotl => base("plain", 0x77, start, end),
        I32Rotr => base("plain", 0x78, start, end),

        I64Clz => base("plain", 0x79, start, end),
        I64Ctz => base("plain", 0x7a, start, end),
        I64Popcnt => base("plain", 0x7b, start, end),
        I64Add => base("plain", 0x7c, start, end),
        I64Sub => base("plain", 0x7d, start, end),
        I64Mul => base("plain", 0x7e, start, end),
        I64DivS => base("plain", 0x7f, start, end),
        I64DivU => base("plain", 0x80, start, end),
        I64RemS => base("plain", 0x81, start, end),
        I64RemU => base("plain", 0x82, start, end),
        I64And => base("plain", 0x83, start, end),
        I64Or => base("plain", 0x84, start, end),
        I64Xor => base("plain", 0x85, start, end),
        I64Shl => base("plain", 0x86, start, end),
        I64ShrS => base("plain", 0x87, start, end),
        I64ShrU => base("plain", 0x88, start, end),
        I64Rotl => base("plain", 0x89, start, end),
        I64Rotr => base("plain", 0x8a, start, end),

        F32Abs => base("plain", 0x8b, start, end),
        F32Neg => base("plain", 0x8c, start, end),
        F32Ceil => base("plain", 0x8d, start, end),
        F32Floor => base("plain", 0x8e, start, end),
        F32Trunc => base("plain", 0x8f, start, end),
        F32Nearest => base("plain", 0x90, start, end),
        F32Sqrt => base("plain", 0x91, start, end),
        F32Add => base("plain", 0x92, start, end),
        F32Sub => base("plain", 0x93, start, end),
        F32Mul => base("plain", 0x94, start, end),
        F32Div => base("plain", 0x95, start, end),
        F32Min => base("plain", 0x96, start, end),
        F32Max => base("plain", 0x97, start, end),
        F32Copysign => base("plain", 0x98, start, end),

        F64Abs => base("plain", 0x99, start, end),
        F64Neg => base("plain", 0x9a, start, end),
        F64Ceil => base("plain", 0x9b, start, end),
        F64Floor => base("plain", 0x9c, start, end),
        F64Trunc => base("plain", 0x9d, start, end),
        F64Nearest => base("plain", 0x9e, start, end),
        F64Sqrt => base("plain", 0x9f, start, end),
        F64Add => base("plain", 0xa0, start, end),
        F64Sub => base("plain", 0xa1, start, end),
        F64Mul => base("plain", 0xa2, start, end),
        F64Div => base("plain", 0xa3, start, end),
        F64Min => base("plain", 0xa4, start, end),
        F64Max => base("plain", 0xa5, start, end),
        F64Copysign => base("plain", 0xa6, start, end),

        I32WrapI64 => base("plain", 0xa7, start, end),
        I32TruncF32S => base("plain", 0xa8, start, end),
        I32TruncF32U => base("plain", 0xa9, start, end),
        I32TruncF64S => base("plain", 0xaa, start, end),
        I32TruncF64U => base("plain", 0xab, start, end),
        I64ExtendI32S => base("plain", 0xac, start, end),
        I64ExtendI32U => base("plain", 0xad, start, end),
        I64TruncF32S => base("plain", 0xae, start, end),
        I64TruncF32U => base("plain", 0xaf, start, end),
        I64TruncF64S => base("plain", 0xb0, start, end),
        I64TruncF64U => base("plain", 0xb1, start, end),
        F32ConvertI32S => base("plain", 0xb2, start, end),
        F32ConvertI32U => base("plain", 0xb3, start, end),
        F32ConvertI64S => base("plain", 0xb4, start, end),
        F32ConvertI64U => base("plain", 0xb5, start, end),
        F32DemoteF64 => base("plain", 0xb6, start, end),
        F64ConvertI32S => base("plain", 0xb7, start, end),
        F64ConvertI32U => base("plain", 0xb8, start, end),
        F64ConvertI64S => base("plain", 0xb9, start, end),
        F64ConvertI64U => base("plain", 0xba, start, end),
        F64PromoteF32 => base("plain", 0xbb, start, end),
        I32ReinterpretF32 => base("plain", 0xbc, start, end),
        I64ReinterpretF64 => base("plain", 0xbd, start, end),
        F32ReinterpretI32 => base("plain", 0xbe, start, end),
        F64ReinterpretI64 => base("plain", 0xbf, start, end),

        I32Extend8S => base("plain", 0xc0, start, end),
        I32Extend16S => base("plain", 0xc1, start, end),
        I64Extend8S => base("plain", 0xc2, start, end),
        I64Extend16S => base("plain", 0xc3, start, end),
        I64Extend32S => base("plain", 0xc4, start, end),

        RefNull { .. } => base("plain", 0xd0, start, end),
        RefIsNull => base("plain", 0xd1, start, end),
        RefFunc { .. } => base("plain", 0xd2, start, end),

        MemoryCopy { .. } => RustInstr {
            sub_opcode: Some(0x0a),
            ..base("plain", 0xfc, start, end)
        },
        MemoryFill { .. } => RustInstr {
            sub_opcode: Some(0x0b),
            ..base("plain", 0xfc, start, end)
        },
        TableInit { .. } => RustInstr {
            sub_opcode: Some(0x0c),
            ..base("plain", 0xfc, start, end)
        },
        TableCopy { .. } => RustInstr {
            sub_opcode: Some(0x0e),
            ..base("plain", 0xfc, start, end)
        },
        TableGrow { table } => RustInstr {
            sub_opcode: Some(0x0f),
            index: Some(*table),
            ..base("indexed", 0xfc, start, end)
        },
        TableSize { table } => RustInstr {
            sub_opcode: Some(0x10),
            index: Some(*table),
            ..base("indexed", 0xfc, start, end)
        },
        TableFill { table } => RustInstr {
            sub_opcode: Some(0x11),
            index: Some(*table),
            ..base("indexed", 0xfc, start, end)
        },

        Block { .. } | Loop { .. } | If { .. } | Else => {
            return Err(format!("{op:?} must be handled by the structural frame stack"));
        }

        unsupported => {
            return Err(format!("unsupported wasm instruction: {unsupported:?}"));
        }
    };
    Ok(i)
}
