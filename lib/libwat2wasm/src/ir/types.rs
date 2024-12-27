use crate::*;
use ir::{index::*, WasmSectionId};
use leb128::{Leb128Writer, WriteError, WriteLeb128};
use types::ValType;

#[derive(Default)]
pub struct Types(pub(super) Vec<Type>);

#[derive(Debug)]
pub struct Type {
    params: Vec<ValType>,
    results: Vec<ValType>,
    signature: String,
}

impl Types {
    pub fn define(&mut self, new_item: Type) -> Result<TypeIndex, AssembleError> {
        for (index, item) in self.0.iter().enumerate() {
            if *item == new_item {
                return Ok(TypeIndex(index as u32));
            }
        }
        let result = TypeIndex(self.0.len() as u32);
        self.0.push(new_item);
        Ok(result)
    }

    pub fn find(&self, expected: Type) -> Option<TypeIndex> {
        for (index, item) in self.0.iter().enumerate() {
            if *item == expected {
                return Some(TypeIndex(index as u32));
            }
        }
        None
    }

    pub fn write_to_wasm(&self, writer: &mut Leb128Writer) -> Result<WasmSectionId, WriteError> {
        if self.0.len() > 0 {
            writer.write(self.0.len())?;
            for item in self.0.iter() {
                writer.write_byte(0x60)?;
                writer.write(item.params().len())?;
                for param in item.params() {
                    writer.write(param.as_bytecode())?;
                }
                writer.write(item.results().len())?;
                for result in item.results() {
                    writer.write(result.as_bytecode())?;
                }
            }
        }
        Ok(WasmSectionId::Type)
    }
}

impl core::fmt::Debug for Types {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}

impl Type {
    pub fn from_iter(
        params: impl ExactSizeIterator<Item = ValType>,
        results: impl ExactSizeIterator<Item = ValType>,
    ) -> Self {
        // params + results + {void(1) + void(1) + separator(1)}(=3)
        let mut signature = String::with_capacity(params.len() + results.len() + 3);
        let mut params_ = Vec::with_capacity(params.len());
        if params.len() > 0 {
            for param in params {
                params_.push(param);
                signature.push_str(param.signature());
            }
        } else {
            signature.push('v');
        }
        signature.push(':');
        let mut results_ = Vec::with_capacity(results.len());
        if results.len() > 0 {
            for result in results {
                results_.push(result);
                signature.push_str(result.signature());
            }
        } else {
            signature.push('v');
        }
        signature.shrink_to_fit();

        Type {
            params: params_,
            results: results_,
            signature,
        }
    }

    #[inline]
    pub fn from_ast(params: &[ast::types::IdAndValtype], results: &[ValType]) -> Self {
        Self::from_iter(
            params.iter().map(|v| v.valtype()),
            results.iter().map(|v| *v),
        )
    }

    #[inline]
    pub fn params(&self) -> &[ValType] {
        &self.params
    }

    #[inline]
    pub fn results(&self) -> &[ValType] {
        &self.results
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        self.signature == other.signature
    }
}

#[derive(Debug)]
pub struct IdAndValType {
    id: Option<String>,
    valtype: ValType,
}

impl IdAndValType {
    #[inline]
    pub fn identifier(&self) -> Option<&str> {
        self.id.as_ref().map(|v| v.as_str())
    }

    #[inline]
    pub fn valtype(&self) -> &ValType {
        &self.valtype
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IdType {
    Type(TypeIndex),
    Func(FuncIndex),
    Table(TableIndex),
    Memory(MemoryIndex),
    Global(GlobalIndex),
}

impl core::fmt::Debug for IdType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Type(arg0) => write!(f, "{:#?}", arg0),
            Self::Func(arg0) => write!(f, "{:#?}", arg0),
            Self::Table(arg0) => write!(f, "{:#?}", arg0),
            Self::Memory(arg0) => write!(f, "{:#?}", arg0),
            Self::Global(arg0) => write!(f, "{:#?}", arg0),
        }
    }
}
