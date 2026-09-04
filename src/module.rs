use crate::instr::parse_instr_stream;
use crate::model::*;
use crate::valtype::val_type_to_string;
use std::ops::Range;
use wasmparser::{
    CompositeInnerType, ElementItems, ElementKind, ExternalKind, FromReader, KnownCustom, Name,
    Operator, Parser, Payload, SectionLimited, TypeRef,
};

fn to_section(name: &str, range: Range<usize>) -> RustSection {
    RustSection {
        section: name.to_string(),
        start_address: range.start as u32,
        end_address: range.end as u32,
    }
}

/// Reads every item out of a `SectionLimited` section together with the
/// byte range each item occupies. The end of the last item is the end of the
/// section itself.
fn with_ranges<'a, T: FromReader<'a>>(
    section: SectionLimited<'a, T>,
) -> Result<Vec<(T, u32, u32)>, String> {
    let section_end = section.range().end as u32;
    let mut values: Vec<T> = Vec::new();
    let mut starts: Vec<u32> = Vec::new();
    for item in section.into_iter_with_offsets() {
        let (offset, val) = item.map_err(|e| e.to_string())?;
        values.push(val);
        starts.push(offset as u32);
    }
    let mut out = Vec::with_capacity(values.len());
    let n = values.len();
    for (i, val) in values.into_iter().enumerate() {
        let start = starts[i];
        let end = if i + 1 < n { starts[i + 1] } else { section_end };
        out.push((val, start, end));
    }
    Ok(out)
}

pub fn parse(bytes: &[u8]) -> Result<RustModule, String> {
    let mut m = RustModule::default();

    let mut function_type_indices: Vec<u32> = Vec::new();
    let mut num_func_imports: u32 = 0;
    let mut num_table_imports: u32 = 0;
    let mut code_index: u32 = 0;

    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(|e| e.to_string())?;
        match payload {
            Payload::Version { .. } => {}

            Payload::TypeSection(reader) => {
                m.sections.push(to_section("type", reader.range()));
                let mut id = 0u32;
                for group in reader {
                    let group = group.map_err(|e| e.to_string())?;
                    for sub_type in group.into_types() {
                        match sub_type.composite_type.inner {
                            CompositeInnerType::Func(ft) => {
                                let params = ft
                                    .params()
                                    .iter()
                                    .map(|t| val_type_to_string(*t))
                                    .collect::<Result<Vec<_>, _>>()?;
                                let results = ft
                                    .results()
                                    .iter()
                                    .map(|t| val_type_to_string(*t))
                                    .collect::<Result<Vec<_>, _>>()?;
                                m.types.push(RustFuncType { id, params, results });
                                id += 1;
                            }
                            other => {
                                return Err(format!(
                                    "unsupported type section entry: {other:?}"
                                ));
                            }
                        }
                    }
                }
            }

            Payload::ImportSection(reader) => {
                m.sections.push(to_section("import", reader.range()));
                for (imp, start, end) in with_ranges(reader)? {
                    match imp.ty {
                        TypeRef::Func(type_index) => {
                            num_func_imports += 1;
                            m.func_imports.push(RustFuncImport {
                                module: imp.module.to_string(),
                                name: imp.name.to_string(),
                                type_index,
                                start_address: start,
                                end_address: end,
                            });
                        }
                        TypeRef::Table(_) => {
                            m.table_imports.push(RustTableImport {
                                module: imp.module.to_string(),
                                name: imp.name.to_string(),
                                id: num_table_imports,
                                start_address: start,
                                end_address: end,
                            });
                            num_table_imports += 1;
                        }
                        TypeRef::Memory(_) | TypeRef::Global(_) | TypeRef::Tag(_) => {
                            // Not modeled by the TypeScript ParsedModule.
                        }
                    }
                }
            }

            Payload::FunctionSection(reader) => {
                m.sections.push(to_section("func", reader.range()));
                for type_index in reader {
                    function_type_indices.push(type_index.map_err(|e| e.to_string())?);
                }
            }

            Payload::TableSection(reader) => {
                m.sections.push(to_section("table", reader.range()));
            }
            Payload::MemorySection(reader) => {
                m.sections.push(to_section("memory", reader.range()));
            }
            Payload::TagSection(reader) => {
                m.sections.push(to_section("custom", reader.range()));
            }

            Payload::GlobalSection(reader) => {
                m.sections.push(to_section("global", reader.range()));
                for (g, start, end) in with_ranges(reader)? {
                    let value_type = val_type_to_string(g.ty.content_type)?;
                    let init = parse_instr_stream(g.init_expr.get_operators_reader())?;
                    m.globals.push(RustGlobal {
                        value_type,
                        mutable: g.ty.mutable,
                        name: None,
                        init,
                        start_address: start,
                        end_address: end,
                    });
                }
            }

            Payload::ExportSection(reader) => {
                m.sections.push(to_section("export", reader.range()));
                for exp in reader {
                    let exp = exp.map_err(|e| e.to_string())?;
                    match exp.kind {
                        ExternalKind::Func => {
                            m.exported_funcs.push(RustFuncExport {
                                name: exp.name.to_string(),
                                func_index: exp.index,
                            });
                        }
                        ExternalKind::Table => {
                            m.table_exports.push(RustTableExport {
                                name: exp.name.to_string(),
                                id: exp.index,
                            });
                        }
                        _ => {}
                    }
                }
            }

            Payload::StartSection { range, .. } => {
                m.sections.push(to_section("start", range));
            }

            Payload::ElementSection(reader) => {
                m.sections.push(to_section("element", reader.range()));
                for elem in reader {
                    let elem = elem.map_err(|e| e.to_string())?;
                    let table_id = match &elem.kind {
                        ElementKind::Active { table_index, .. } => table_index.unwrap_or(0),
                        ElementKind::Passive | ElementKind::Declared => continue,
                    };
                    let funcs = match elem.items {
                        ElementItems::Functions(section) => section
                            .into_iter()
                            .collect::<Result<Vec<u32>, _>>()
                            .map_err(|e| e.to_string())?,
                        ElementItems::Expressions(_, section) => {
                            let mut funcs = Vec::new();
                            for expr in section {
                                let expr = expr.map_err(|e| e.to_string())?;
                                let mut ops = expr.get_operators_reader().into_iter();
                                if let Some(Ok(Operator::RefFunc { function_index })) = ops.next()
                                {
                                    funcs.push(function_index);
                                }
                            }
                            funcs
                        }
                    };
                    m.elements.push(RustElement {
                        table_id,
                        funcs,
                        start_address: elem.range.start as u32,
                        end_address: elem.range.end as u32,
                    });
                }
            }

            Payload::DataCountSection { .. } => {}

            Payload::DataSection(reader) => {
                m.sections.push(to_section("data", reader.range()));
            }

            Payload::CodeSectionStart { range, .. } => {
                m.sections.push(to_section("code", range));
            }

            Payload::CodeSectionEntry(body) => {
                let type_index = *function_type_indices
                    .get(code_index as usize)
                    .ok_or_else(|| "code section has more entries than the function section".to_string())?;
                let func_type = m
                    .types
                    .get(type_index as usize)
                    .ok_or_else(|| format!("function refers to unknown type index {type_index}"))?
                    .clone();
                let func_id = num_func_imports + code_index;

                let mut locals: Vec<RustLocal> = Vec::with_capacity(func_type.params.len());
                for (i, ty) in func_type.params.iter().enumerate() {
                    locals.push(RustLocal {
                        index: i as u32,
                        ty: ty.clone(),
                    });
                }
                let mut next_local_index = func_type.params.len() as u32;
                for group in body.get_locals_reader().map_err(|e| e.to_string())? {
                    let (count, val_type) = group.map_err(|e| e.to_string())?;
                    let ty = val_type_to_string(val_type)?;
                    for _ in 0..count {
                        locals.push(RustLocal {
                            index: next_local_index,
                            ty: ty.clone(),
                        });
                        next_local_index += 1;
                    }
                }

                let body_instrs =
                    parse_instr_stream(body.get_operators_reader().map_err(|e| e.to_string())?)?;

                m.funcs.push(RustFunc {
                    id: func_id,
                    name: format!("func_{func_id}"),
                    type_index,
                    locals,
                    body: body_instrs,
                });
                code_index += 1;
            }

            Payload::CustomSection(c) => {
                m.sections.push(to_section("custom", c.range()));
                if let KnownCustom::Name(name_reader) = c.as_known() {
                    for name in name_reader {
                        let name = match name {
                            Ok(n) => n,
                            Err(_) => continue, // tolerate a malformed/foreign "name" section
                        };
                        match name {
                            Name::Function(map) => {
                                for naming in map {
                                    let naming = naming.map_err(|e| e.to_string())?;
                                    m.func_names.push(RustNameEntry {
                                        index: naming.index,
                                        value: naming.name.to_string(),
                                    });
                                }
                            }
                            Name::Local(imap) => {
                                for indirect in imap {
                                    let indirect = indirect.map_err(|e| e.to_string())?;
                                    for naming in indirect.names {
                                        let naming = naming.map_err(|e| e.to_string())?;
                                        m.local_names.push(RustLocalNameEntry {
                                            function_index: indirect.index,
                                            local_index: naming.index,
                                            value: naming.name.to_string(),
                                        });
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            _ => {}
        }
    }

    // Resolve function names from the name section now that both the
    // function bodies and the name section (which may appear anywhere in
    // the binary) have been fully read.
    if !m.func_names.is_empty() {
        for f in &mut m.funcs {
            if let Some(n) = m.func_names.iter().find(|n| n.index == f.id) {
                f.name = n.value.clone();
            }
        }
    }

    Ok(m)
}
