use crate::*;
use ir::{Module, WasmSectionId};
use leb128::{Leb128Writer, WriteError, WriteLeb128};
use wasm::expr::ConstExpr;

#[derive(Default)]
pub struct DataSegments(pub(super) Vec<Data>);

pub struct Data {
    offset: ConstExpr,
    bytes: Vec<u8>,
}

impl DataSegments {
    pub(super) fn convert(
        module: &mut Module,
        data_segments: Vec<ast::data::Data>,
    ) -> Result<(), AssembleError> {
        for data in data_segments {
            let ast::data::Data {
                mem_use: _,
                offset,
                bytes,
            } = data;

            module.data_segs.0.push(Data { offset, bytes });
        }
        Ok(())
    }

    pub fn write_to_wasm(&self, writer: &mut Leb128Writer) -> Result<WasmSectionId, WriteError> {
        if self.0.len() > 0 {
            writer.write(self.0.len())?;
            for item in self.0.iter() {
                writer.write(0)?;
                item.offset.write_to_wasm(writer)?;
                writer.write_blob(&item.bytes)?;
            }
        }
        Ok(WasmSectionId::Data)
    }
}

impl core::fmt::Debug for DataSegments {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}

impl core::fmt::Debug for Data {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Data")
            .field("offset", &self.offset)
            .field("bytes", &DumpHex(&self.bytes))
            .finish()
    }
}
