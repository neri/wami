use crate::*;
use core::num::NonZeroU32;
use ir::{Module, WasmSectionId, index::*, types::IdType};
use leb128::{Leb128Writer, WriteError, WriteLeb128};

#[derive(Default)]
pub struct Memories(pub(super) Vec<Memory>);

#[derive(Debug)]
pub struct Memory {
    pub min: u32,
    pub max: Option<NonZeroU32>,
}

impl Memories {
    pub(super) fn convert(
        module: &mut Module,
        memories: Vec<ast::memory::Memory>,
    ) -> Result<(), AssembleError> {
        for ast_memory in memories {
            if let Some(id) = ast_memory.id() {
                let memidx = MemoryIndex(module.memories.0.len() as u32);
                module.register_ast_name(id, IdType::Memory(memidx))?;
            }

            let memory = Memory {
                min: ast_memory.min(),
                max: ast_memory.max(),
            };
            module.memories.0.push(memory);
        }

        Ok(())
    }

    pub fn write_to_wasm(&self, writer: &mut Leb128Writer) -> Result<WasmSectionId, WriteError> {
        if self.0.len() > 0 {
            writer.write(self.0.len())?;
            for item in self.0.iter() {
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
        Ok(WasmSectionId::Memory)
    }
}

impl core::fmt::Debug for Memories {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}
