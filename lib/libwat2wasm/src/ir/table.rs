use crate::*;
use ast::table::RefType;
use core::num::NonZeroU32;
use ir::{index::*, types::IdType, Module, WasmSectionId};
use leb128::{Leb128Writer, WriteError, WriteLeb128};

#[derive(Default)]
pub struct Tables(pub(super) Vec<Table>);

#[derive(Debug)]
pub struct Table {
    pub min: u32,
    pub max: Option<NonZeroU32>,
    pub reftype: RefType,
}

impl Tables {
    pub(super) fn convert(
        module: &mut Module,
        tables: Vec<ast::table::Table>,
    ) -> Result<(), AssembleError> {
        for ast_table in tables {
            if let Some(id) = ast_table.id() {
                let tableidx = TableIndex(module.tables.0.len() as u32);
                module.register_ast_name(id, IdType::Table(tableidx))?;
            }

            let table = Table {
                min: ast_table.min(),
                max: ast_table.max(),
                reftype: ast_table.reftype(),
            };
            module.tables.0.push(table);
        }

        Ok(())
    }

    pub fn write_to_wasm(&self, writer: &mut Leb128Writer) -> Result<WasmSectionId, WriteError> {
        if self.0.len() > 0 {
            writer.write(self.0.len())?;
            for item in self.0.iter() {
                item.reftype.write_to_wasm(writer)?;
                if let Some(max) = item.max {
                    writer.write(1)?;
                    writer.write(item.min)?;
                    writer.write(max.get())?;
                } else {
                    writer.write(0)?;
                    writer.write(item.min)?;
                }
            }
        }
        Ok(WasmSectionId::Table)
    }
}

impl core::fmt::Debug for Tables {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}
