use crate::*;
use ast::global::GlobalType;
use ir::{export::ExportDesc, index::*, types::IdType, Module, WasmSectionId};
use leb128::{Leb128Writer, WriteError, WriteLeb128};
use wasm::expr::ConstExpr;

#[derive(Default)]
pub struct Globals(pub(super) Vec<Global>);

#[derive(Debug)]
pub struct Global {
    global_type: GlobalType,
    expr: ConstExpr,
}

impl Globals {
    pub(super) fn convert(
        module: &mut Module,
        globals: Vec<ast::global::Global>,
    ) -> Result<(), AssembleError> {
        let n_imports = module.imports.num_import_globals();
        for ast_global in globals {
            if matches!(ast_global.vis, Some(ast::types::ExtVis::Import(_))) {
                continue;
            }

            let ast::global::Global {
                id,
                vis,
                global_type,
                expr,
            } = ast_global;

            let globalidx = GlobalIndex((n_imports + module.globals.0.len()) as u32);

            if let Some(ref id) = id {
                module.register_ast_name(id, IdType::Global(globalidx))?;
            }

            if let Some(vis) = vis {
                match vis {
                    ast::types::ExtVis::Import(_) => {}
                    ast::types::ExtVis::Export(export) => {
                        module
                            .exports
                            .export(export.name(), ExportDesc::Global(globalidx))?;
                    }
                }
            }

            module.globals.0.push(Global { global_type, expr });
        }

        Ok(())
    }

    pub fn write_to_wasm(&self, writer: &mut Leb128Writer) -> Result<WasmSectionId, WriteError> {
        if self.0.len() > 0 {
            writer.write(self.0.len())?;
            for item in self.0.iter() {
                item.global_type.write_to_wasm(writer)?;
                item.expr.write_to_wasm(writer)?;
            }
        }
        Ok(WasmSectionId::Global)
    }
}

impl core::fmt::Debug for Globals {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}
