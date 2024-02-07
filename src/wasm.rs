//! WebAssembly Interpreter
use crate::cg::WasmCodeBlock;
use crate::leb128::{self, *};
use crate::memory::WasmMemory;
use crate::opcode::{WasmMnemonic, WasmOpcode};
use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::*;
use alloc::vec::Vec;
use core::error::Error;
use core::fmt;
use core::mem::{size_of, transmute, ManuallyDrop};
use core::num::NonZeroU32;
use core::ops::*;
use core::ptr::slice_from_raw_parts_mut;
use core::slice;
use core::str;
use core::sync::atomic::{AtomicU64, Ordering};
use smallvec::SmallVec;

pub type WasmDynFunc =
    fn(&WasmInstance, &[WasmUnionValue]) -> Result<WasmValue, WasmRuntimeErrorKind>;

pub enum ImportResult<T> {
    Ok(T),
    NoModule,
    NoMethod,
}

pub struct WebAssembly;

impl WebAssembly {
    /// Minimal valid module size, Magic(4) + Version(4) + Empty sections(0) = 8
    pub const MINIMAL_MOD_SIZE: usize = 8;
    /// Magic number of WebAssembly Binary Format
    pub const MAGIC: [u8; 4] = *b"\0asm";
    /// Current version number is 1
    pub const VER_CURRENT: [u8; 4] = *b"\x01\0\0\0";

    /// The length of the vector always is a multiple of the WebAssembly page size,
    /// which is defined to be the constant 65536 – abbreviated 64Ki.
    pub const PAGE_SIZE: usize = 65536;

    /// Identify the file format
    #[inline]
    pub fn identify(bytes: &[u8]) -> bool {
        bytes.len() >= Self::MINIMAL_MOD_SIZE
            && &bytes[0..4] == Self::MAGIC
            && &bytes[4..8] == Self::VER_CURRENT
    }

    /// Instantiate wasm module
    pub fn instantiate<F>(bytes: &[u8], imports_resolver: F) -> Result<WasmInstance, Box<dyn Error>>
    where
        F: FnMut(&str, &str, &WasmType) -> ImportResult<WasmDynFunc> + Copy,
    {
        Self::compile(bytes)?.instantiate(imports_resolver)
    }

    /// Compile wasm module
    #[inline]
    pub fn compile(bytes: &[u8]) -> Result<WasmModule, Box<dyn Error>> {
        WasmModule::compile(bytes)
    }

    #[inline]
    #[must_use]
    pub fn validate(bytes: &[u8]) -> bool {
        Self::compile(bytes).is_ok()
    }
}

/// WebAssembly module
#[derive(Default)]
pub struct WasmModule {
    types: Vec<WasmType>,
    imports: Vec<WasmImport>,
    functions: Vec<WasmFunction>,
    tables: Vec<WasmTable>,
    memories: Vec<WasmMemory>,
    globals: Vec<WasmGlobal>,
    exports: Vec<WasmExport>,
    start: Option<usize>,
    data_count: Option<usize>,
    custom_sections: BTreeMap<String, Box<[u8]>>,
    names: Option<WasmName>,
}

impl fmt::Debug for WasmModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WasmModule").finish()
    }
}

impl WasmModule {
    #[cfg(test)]
    fn empty() -> Self {
        Self {
            types: Vec::new(),
            imports: Vec::new(),
            functions: Vec::new(),
            tables: Vec::new(),
            memories: Vec::from_iter([WasmMemory::zero()]),
            globals: Vec::new(),
            exports: Vec::new(),
            start: None,
            data_count: None,
            custom_sections: BTreeMap::new(),
            names: None,
        }
    }

    #[inline]
    fn compile(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        if !WebAssembly::identify(bytes) {
            return Err(CompileErrorKind::BadExecutable.into());
        }
        let mut module = Self::default();
        let mut reader = Leb128Reader::from_slice(&bytes[8..]);
        let reader = &mut reader;
        let mut last_section_id = WasmSectionId::Type;

        while let Some(mut section) = WasmSection::from_reader(reader)? {
            match section.section_id {
                WasmSectionId::Custom => {
                    match section.reader.read() {
                        Ok(section_name) => {
                            let mut blob = Vec::new();
                            section
                                .reader
                                .read_to_end(&mut blob)
                                .map_err(|err| CompileErrorKind::from(err))?;

                            match section_name {
                                WasmName::SECTION_NAME => {
                                    let mut reader = Leb128Reader::from_slice(blob.as_slice());
                                    module.names = WasmName::from_reader(&mut reader).ok();
                                }
                                _ => (),
                            }

                            module
                                .custom_sections
                                .insert(section_name.to_owned(), blob.into_boxed_slice());
                        }
                        Err(_) => {
                            // ignored
                        }
                    };
                }
                _ => (),
            }
        }
        reader.reset();

        while let Some(section) = WasmSection::from_reader(reader)? {
            if section.section_id.depends_on_order() {
                if last_section_id > section.section_id {
                    return Err(CompileErrorKind::InvalidSectionOrder(section.section_id).into());
                } else {
                    last_section_id = section.section_id;
                }
            }
            match section.section_id {
                WasmSectionId::Custom => (),
                WasmSectionId::Type => module.parse_sec_type(section)?,
                WasmSectionId::Import => module.parse_sec_import(section)?,
                WasmSectionId::Function => module.parse_sec_func(section)?,
                WasmSectionId::Table => module.parse_sec_table(section)?,
                WasmSectionId::Memory => module.parse_sec_memory(section)?,
                WasmSectionId::Global => module.parse_sec_global(section)?,
                WasmSectionId::Export => module.parse_sec_export(section)?,
                WasmSectionId::Start => module.parse_sec_start(section)?,
                WasmSectionId::Element => module.parse_sec_elem(section)?,
                WasmSectionId::Code => module.parse_sec_code(section)?,
                WasmSectionId::Data => module.parse_sec_data(section)?,
                WasmSectionId::DataCount => module.parse_sec_data_count(section)?,
            };
        }

        module.types.shrink_to_fit();
        module.imports.shrink_to_fit();
        module.functions.shrink_to_fit();
        module.tables.shrink_to_fit();
        module.memories.shrink_to_fit();
        module.globals.shrink_to_fit();
        module.exports.shrink_to_fit();

        Ok(module)
    }

    pub fn instantiate<F>(mut self, mut imports_resolver: F) -> Result<WasmInstance, Box<dyn Error>>
    where
        F: FnMut(&str, &str, &WasmType) -> ImportResult<WasmDynFunc> + Copy,
    {
        let mut func_idx = 0;
        for import in &self.imports {
            match import.desc {
                WasmImportDescriptor::Function(type_index) => {
                    match imports_resolver(
                        &import.mod_name,
                        &import.name,
                        self.type_by_index(type_index),
                    ) {
                        ImportResult::Ok(dyn_func) => {
                            self.functions[func_idx].resolve(dyn_func)?;
                        }
                        ImportResult::NoModule => {
                            return Err(LinkError::NoModule(import.mod_name.clone()).into())
                        }
                        ImportResult::NoMethod => {
                            return Err(LinkError::NoMethod(import.name.clone()).into())
                        }
                    }
                    func_idx += 1;
                }
                WasmImportDescriptor::Memory(_) => {
                    // TODO: import memory
                }
            }
        }
        Ok(WasmInstance::new(self))
    }

    /// Parse "type" section
    fn parse_sec_type(&mut self, mut section: WasmSection) -> Result<(), CompileErrorKind> {
        let n_items: usize = section.reader.read()?;
        for _ in 0..n_items {
            let ft = WasmType::from_reader(&mut section.reader)?;
            self.types.push(ft);
        }
        Ok(())
    }

    /// Parse "import" section
    fn parse_sec_import(&mut self, mut section: WasmSection) -> Result<(), CompileErrorKind> {
        let n_items: usize = section.reader.read()?;
        for _ in 0..n_items {
            let import = WasmImport::from_reader(&mut section.reader)?;
            match import.desc {
                WasmImportDescriptor::Function(type_index) => {
                    let index = self.functions.len();
                    let func_type = self
                        .types
                        .get(type_index.as_usize())
                        .ok_or(CompileErrorKind::InvalidType(type_index))?;
                    self.functions.push(WasmFunction::from_import(
                        index,
                        type_index,
                        func_type.clone(),
                    ));
                }
                WasmImportDescriptor::Memory(memtype) => {
                    // TODO: import memory
                    self.memories.push(WasmMemory::new(memtype)?);
                }
            }
            self.imports.push(import);
        }
        Ok(())
    }

    /// Parse "func" section
    fn parse_sec_func(&mut self, mut section: WasmSection) -> Result<(), CompileErrorKind> {
        let n_items: usize = section.reader.read()?;
        let base_index = self.imports.len();
        for index in 0..n_items {
            let type_index = WasmTypeIndex(section.reader.read()?);
            let func_type = self
                .types
                .get(type_index.as_usize())
                .ok_or(CompileErrorKind::InvalidType(type_index))?;
            self.functions.push(WasmFunction::internal(
                base_index + index,
                type_index,
                func_type.clone(),
            ));
        }
        Ok(())
    }

    /// Parse "export" section
    fn parse_sec_export(&mut self, mut section: WasmSection) -> Result<(), CompileErrorKind> {
        let n_items: usize = section.reader.read()?;
        for _ in 0..n_items {
            let export = WasmExport::new(&self, &mut section.reader)?;
            self.exports.push(export);
        }
        Ok(())
    }

    /// Parse "memory" section
    fn parse_sec_memory(&mut self, mut section: WasmSection) -> Result<(), CompileErrorKind> {
        let n_items: usize = section.reader.read()?;
        for _ in 0..n_items {
            let limit = WasmLimit::from_reader(&mut section.reader, true)?;
            self.memories.push(WasmMemory::new(limit)?);
        }
        Ok(())
    }

    /// Parse "table" section
    fn parse_sec_table(&mut self, mut section: WasmSection) -> Result<(), CompileErrorKind> {
        let n_items: usize = section.reader.read()?;
        for _ in 0..n_items {
            let table = WasmTable::from_reader(&mut section.reader)?;
            self.tables.push(table);
        }
        Ok(())
    }

    /// Parse "elem" section
    fn parse_sec_elem(&mut self, mut section: WasmSection) -> Result<(), CompileError> {
        let n_items: usize = section.reader.read()?;
        for _ in 0..n_items {
            let tabidx: usize = section.reader.read()?;
            let offset = self.eval_offset(&mut section)?;
            let n_elements: usize = section.reader.read()?;
            let table = self
                .tables
                .get_mut(tabidx)
                .ok_or(CompileErrorKind::InvalidData)?;
            for i in offset..offset + n_elements {
                let elem: usize = section.reader.read()?;
                table.table.get_mut(i).map(|v| *v = elem);
            }
        }
        Ok(())
    }

    /// Parse "code" section
    fn parse_sec_code(&mut self, mut section: WasmSection) -> Result<(), CompileError> {
        let base = self
            .functions
            .iter()
            .enumerate()
            .find(|(_, v)| !v.is_external && matches!(v.content(), WasmFunctionContent::Unresolved))
            .map(|(i, _)| i)
            .ok_or(CompileErrorKind::OutOfFunction)?;

        let n_items: usize = section.reader.read()?;
        for i in 0..n_items {
            let index = base + i;

            let func_def = self
                .functions
                .get(index)
                .ok_or(CompileErrorKind::OutOfFunction)?;
            let length: usize = section.reader.read()?;
            let file_position = section.file_position() + section.reader.position();
            let mut reader = section.reader.sub_slice(length).unwrap();
            let code_block = WasmCodeBlock::generate(
                index,
                file_position,
                &mut reader,
                func_def.param_types(),
                func_def.result_types(),
                self,
            )?;

            self.functions
                .get_mut(index)
                .ok_or(CompileErrorKind::OutOfFunction)
                .and_then(|v| v.set_code_block(code_block))?;
        }
        Ok(())
    }

    /// Parse "data" section
    fn parse_sec_data(&mut self, mut section: WasmSection) -> Result<(), CompileError> {
        let n_items: usize = section.reader.read()?;
        for _ in 0..n_items {
            let memidx: usize = section.reader.read()?;
            let offset = self.eval_offset(&mut section)?;
            let src = section.reader.read_blob()?;
            let memory = self
                .memories
                .get_mut(memidx)
                .ok_or(CompileErrorKind::InvalidData)?;
            memory.write_slice(offset, src).unwrap();
        }
        Ok(())
    }

    /// Parse "start" section
    fn parse_sec_start(&mut self, mut section: WasmSection) -> Result<(), CompileErrorKind> {
        let index: usize = section.reader.read()?;
        self.start = Some(index);
        Ok(())
    }

    /// Parse "global" section
    fn parse_sec_global(&mut self, mut section: WasmSection) -> Result<(), CompileError> {
        let n_items: usize = section.reader.read()?;
        for _ in 0..n_items {
            let val_type = section
                .reader
                .read_byte()
                .map_err(|v| v.into())
                .and_then(|v| WasmValType::from_u8(v))?;
            let is_mutable = section.reader.read_byte()? == 1;
            let value = self.eval_const_expr(&mut section)?;

            if !value.is_valid_type(val_type) {
                return Err(CompileErrorKind::InvalidGlobal.into());
            }

            WasmGlobal::new(value, is_mutable).map(|v| self.globals.push(v))?;
        }
        Ok(())
    }

    /// Parse "datacount" section
    fn parse_sec_data_count(&mut self, mut section: WasmSection) -> Result<(), CompileErrorKind> {
        let count: usize = section.reader.read()?;
        self.data_count = Some(count);
        Ok(())
    }

    fn eval_offset(&self, section: &mut WasmSection) -> Result<usize, CompileError> {
        self.eval_const_expr(section)
            .and_then(|v| {
                v.get_i32()
                    .map_err(|_| CompileErrorKind::InvalidData.into())
            })
            .map(|v| v as usize)
    }

    fn eval_const_expr(&self, section: &mut WasmSection) -> Result<WasmValue, CompileError> {
        let base_position = section.file_position();
        let mut ex_position = ExceptionPosition::UNKNOWN;
        let reader = &mut section.reader;
        self._eval_const_expr(reader, &mut ex_position)
            .map_err(|kind| {
                CompileError::new(
                    kind,
                    ExceptionPosition::new(base_position + ex_position.position()),
                    CompileErrorSource::ConstantExpression(ex_position),
                )
            })
    }

    fn _eval_const_expr(
        &self,
        reader: &mut Leb128Reader,
        ex_position: &mut ExceptionPosition,
    ) -> Result<WasmValue, CompileErrorKind> {
        let mut vs = Vec::new();
        loop {
            *ex_position = ExceptionPosition::new(reader.position());
            let bc = WasmOpcode::fetch(reader)?;
            match bc {
                WasmOpcode::I32Const(v) => vs.push(WasmValue::from(v)),
                WasmOpcode::I64Const(v) => vs.push(WasmValue::from(v)),
                WasmOpcode::F32Const(v) => vs.push(WasmValue::from(v)),
                WasmOpcode::F64Const(v) => vs.push(WasmValue::from(v)),
                WasmOpcode::End => match vs.last() {
                    Some(v) => return Ok(*v),
                    None => {
                        return Err(CompileErrorKind::InvalidData);
                    }
                },
                _ => return Err(CompileErrorKind::UnsupportedBytecode(bc.mnemonic())),
            }
        }
    }

    #[inline]
    pub(crate) fn type_by_index(&self, index: WasmTypeIndex) -> &WasmType {
        unsafe { self.types.get_unchecked(index.as_usize()) }
    }

    #[inline]
    pub fn imports<'a>(&'a self) -> impl Iterator<Item = ModuleImport<'a>> {
        self.imports.iter().map(|v| ModuleImport {
            module: v.mod_name.as_str(),
            name: v.name.as_str(),
            kind: ImportExportKind::from_import_desc(&v.desc),
        })
    }

    #[inline]
    pub fn exports<'a>(&'a self) -> impl Iterator<Item = ModuleExport<'a>> {
        self.exports.iter().map(|v| ModuleExport {
            name: v.name.as_str(),
            kind: ImportExportKind::from_export_desc(&v.desc),
        })
    }

    #[inline]
    pub fn custom_sections<'a>(&'a self, section_name: &str) -> Option<&Box<[u8]>> {
        self.custom_sections.get(section_name)
    }

    #[inline]
    pub fn has_memory(&self) -> bool {
        if let Some(memory) = self.memories.first() {
            memory.size() > 0
        } else {
            false
        }
    }

    #[inline]
    pub fn memories(&self) -> &[WasmMemory] {
        &self.memories
    }

    #[inline]
    pub(crate) fn functions(&self) -> &[WasmFunction] {
        self.functions.as_slice()
    }

    #[inline]
    pub(crate) fn elem_get(&self, index: usize) -> Option<&WasmFunction> {
        self.tables
            .get(0)
            .and_then(|v| v.table.get(index))
            .and_then(|v| self.functions.get(*v))
    }

    #[inline]
    pub(crate) fn func_position(&self, index: usize) -> Option<usize> {
        self.functions.get(index).and_then(|v| match v.content() {
            WasmFunctionContent::CodeBlock(v) => Some(v.file_position()),
            _ => None,
        })
    }

    #[inline]
    pub(crate) fn globals(&self) -> &[WasmGlobal] {
        self.globals.as_slice()
    }

    #[inline]
    pub(crate) fn global_get(&self, index: GlobalVarIndex) -> &WasmGlobal {
        #[cfg(test)]
        let _ = self.globals[index.as_usize()];

        unsafe { self.globals.get_unchecked(index.as_usize()) }
    }

    #[inline]
    pub(crate) fn global(&self, name: &str) -> Result<&WasmGlobal, WasmRuntimeErrorKind> {
        for export in &self.exports {
            if let WasmExportDesc::Global(index) = export.desc {
                if export.name == name {
                    return Ok(self.global_get(index));
                }
            }
        }
        Err(WasmRuntimeErrorKind::NoMethod)
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) fn data_count(&self) -> Option<usize> {
        self.data_count
    }

    #[inline]
    pub(crate) fn names(&self) -> Option<&WasmName> {
        self.names.as_ref()
    }
}

pub struct ModuleExport<'a> {
    pub name: &'a str,
    pub kind: ImportExportKind,
}

pub struct ModuleImport<'a> {
    pub module: &'a str,
    pub name: &'a str,
    pub kind: ImportExportKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportExportKind {
    Function,
    Table,
    Memory,
    Global,
}

impl ImportExportKind {
    #[inline]
    fn from_import_desc(desc: &WasmImportDescriptor) -> Self {
        match *desc {
            WasmImportDescriptor::Function(_) => Self::Function,
            WasmImportDescriptor::Memory(_) => Self::Memory,
        }
    }

    #[inline]
    fn from_export_desc(desc: &WasmExportDesc) -> Self {
        match *desc {
            WasmExportDesc::Function(_) => Self::Function,
            WasmExportDesc::Table(_) => Self::Table,
            WasmExportDesc::Memory(_) => Self::Memory,
            WasmExportDesc::Global(_) => Self::Global,
        }
    }
}

#[derive(Debug)]
pub struct WasmInstance {
    module: WasmModule,
}

impl WasmInstance {
    #[inline]
    fn new(module: WasmModule) -> Self {
        Self { module }
    }

    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            module: WasmModule::empty(),
        }
    }

    #[inline]
    pub fn module(&self) -> &WasmModule {
        &self.module
    }

    #[inline]
    pub fn exports(&self) -> impl Iterator<Item = ModuleExport<'_>> {
        self.module.exports()
    }

    #[inline]
    pub fn function(&self, name: &str) -> Result<WasmRunnable, WasmRuntimeErrorKind> {
        for export in &self.module.exports {
            if let WasmExportDesc::Function(index) = export.desc {
                if export.name == name {
                    return self
                        .module
                        .functions
                        .get(index)
                        .map(|v| WasmRunnable::new(v, self))
                        .ok_or(WasmRuntimeErrorKind::NoMethod);
                }
            }
        }
        Err(WasmRuntimeErrorKind::NoMethod)
    }

    #[inline]
    pub fn memory(&self, index: usize) -> Option<&WasmMemory> {
        self.module.memories().get(index)
    }

    #[inline]
    pub fn global(&self, name: &str) -> Result<&WasmGlobal, WasmRuntimeErrorKind> {
        self.module.global(name)
    }
}

/// WebAssembly memory argument
#[derive(Debug, Copy, Clone)]
pub struct WasmMemArg {
    pub align: u32,
    pub offset: u32,
}

impl WasmMemArg {
    #[inline]
    pub const fn new(offset: u32, align: u32) -> Self {
        Self { offset, align }
    }
}

impl<'a, 'b> ReadLeb128<'a, WasmMemArg> for Leb128Reader<'b> {
    fn read(&'a mut self) -> Result<WasmMemArg, ReadError> {
        let a = self.read()?;
        let o = self.read()?;
        Ok(WasmMemArg::new(o, a))
    }
}

/// WebAssembly section
pub struct WasmSection<'a> {
    section_id: WasmSectionId,
    file_position: usize,
    reader: Leb128Reader<'a>,
}

impl<'a> WasmSection<'a> {
    pub fn from_reader<'b>(
        reader: &'b mut Leb128Reader<'a>,
    ) -> Result<Option<WasmSection<'a>>, CompileErrorKind> {
        if reader.is_eof() {
            return Ok(None);
        }
        let section_type = reader.read_byte()?;
        let Some(section_id) = WasmSectionId::from_u8(section_type) else {
            return Err(CompileErrorKind::UnexpectedToken);
        };

        let magic_numer = 8;
        let length: usize = reader.read()?;
        let file_position = reader.position() + magic_numer;
        let _reader = reader
            .sub_slice(length)
            .ok_or(CompileErrorKind::InternalInconsistency)?;

        Ok(Some(Self {
            section_id,
            file_position,
            reader: _reader,
        }))
    }
}

impl WasmSection<'_> {
    #[inline]
    pub const fn section_id(&self) -> WasmSectionId {
        self.section_id
    }

    #[inline]
    pub const fn file_position(&self) -> usize {
        self.file_position
    }

    #[inline]
    pub const fn content_size(&self) -> usize {
        self.reader.len()
    }

    #[inline]
    pub fn custom_section_name(&self) -> Option<String> {
        if self.section_id != WasmSectionId::Custom {
            return None;
        }
        let mut blob = self.reader.cloned();
        blob.reset();
        blob.get_string().ok()
    }
}

/// WebAssembly section types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WasmSectionId {
    Type,
    Import,
    Function,
    Table,
    Memory,
    Global,
    Export,
    Start,
    Element,
    DataCount,
    Code,
    Data,
    Custom,
}

impl WasmSectionId {
    #[inline]
    pub const fn depends_on_order(&self) -> bool {
        match self {
            WasmSectionId::Custom => false,
            _ => true,
        }
    }

    #[inline]
    pub const fn from_u8(val: u8) -> Option<Self> {
        Some(match val {
            0 => Self::Custom,
            1 => Self::Type,
            2 => Self::Import,
            3 => Self::Function,
            4 => Self::Table,
            5 => Self::Memory,
            6 => Self::Global,
            7 => Self::Export,
            8 => Self::Start,
            9 => Self::Element,
            10 => Self::Code,
            11 => Self::Data,
            12 => Self::DataCount,
            _ => return None,
        })
    }
}

/// WebAssembly primitive types
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum WasmValType {
    I32,
    I64,
    F32,
    F64,
}

impl WasmValType {
    #[inline]
    pub const fn from_u8(v: u8) -> Result<Self, CompileErrorKind> {
        match v {
            0x7F => Ok(Self::I32),
            0x7E => Ok(Self::I64),
            0x7D => Ok(Self::F32),
            0x7C => Ok(Self::F64),
            // 0x7B => Ok(Self::V128),
            // 0x78 => Ok(Self::I8),
            // 0x77 => Ok(Self::I16),
            // 0x70 => Ok(Self::FuncRef),
            // 0x6F => Ok(Self::ExternRef),
            _ => Err(CompileErrorKind::UnexpectedToken),
        }
    }

    #[inline]
    pub const fn from_i64(v: i64) -> Result<Self, CompileErrorKind> {
        match v {
            -1 => Ok(Self::I32),
            -2 => Ok(Self::I64),
            -3 => Ok(Self::F32),
            -4 => Ok(Self::F64),
            // -5 => Ok(Self::V128),
            // -8 => Ok(Self::I8),
            // -9 => Ok(Self::I16),
            // -16 => Ok(Self::FuncRef),
            // -17 => Ok(Self::ExternRef),
            _ => Err(CompileErrorKind::UnexpectedToken),
        }
    }

    #[inline]
    pub fn mnemonic(&self) -> char {
        match *self {
            Self::I32 => 'i',
            Self::I64 => 'l',
            Self::F32 => 'f',
            Self::F64 => 'd',
            // Self::V128 => 'v',
            // Self::I8 => 'c',
            // Self::I16 => 'w',
            // Self::FuncRef | Self::ExternRef => '_',
        }
    }
}

impl From<i32> for WasmValType {
    #[inline]
    fn from(_: i32) -> Self {
        Self::I32
    }
}

impl From<i64> for WasmValType {
    #[inline]
    fn from(_: i64) -> Self {
        Self::I64
    }
}

impl From<f32> for WasmValType {
    #[inline]
    fn from(_: f32) -> Self {
        Self::F32
    }
}

impl From<f64> for WasmValType {
    #[inline]
    fn from(_: f64) -> Self {
        Self::F64
    }
}

impl fmt::Display for WasmValType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::I32 => "i32",
                Self::I64 => "i64",
                Self::F32 => "f32",
                Self::F64 => "f64",
                // Self::V128 => "v128",
                // Self::I8 => "i8",
                // Self::I16 => "i16",
                // Self::FuncRef => "func",
                // Self::ExternRef => "extern",
            }
        )
    }
}

/// WebAssembly block types
#[repr(isize)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WasmBlockType {
    Empty = -64,
    I32 = -1,
    I64 = -2,
    F32 = -3,
    F64 = -4,
    // I8 = -8,
    // I16 = -9,
}

impl WasmBlockType {
    pub const fn from_i64(v: i64) -> Result<Self, CompileErrorKind> {
        match v {
            -64 => Ok(Self::Empty),
            -1 => Ok(Self::I32),
            -2 => Ok(Self::I64),
            -3 => Ok(Self::F32),
            -4 => Ok(Self::F64),
            _ => Err(CompileErrorKind::InvalidData),
        }
    }

    pub const fn into_type(self) -> Option<WasmValType> {
        match self {
            WasmBlockType::Empty => None,
            WasmBlockType::I32 => Some(WasmValType::I32),
            WasmBlockType::I64 => Some(WasmValType::I64),
            WasmBlockType::F32 => Some(WasmValType::F32),
            WasmBlockType::F64 => Some(WasmValType::F64),
        }
    }
}

impl<'a> ReadLeb128<'a, WasmBlockType> for Leb128Reader<'_> {
    #[inline]
    fn read(&'a mut self) -> Result<WasmBlockType, ReadError> {
        let value: i64 = self.read()?;
        WasmBlockType::from_i64(value).map_err(|_| ReadError::InvalidData)
    }
}

/// WebAssembly memory limit
#[derive(Debug, Copy, Clone)]
pub struct WasmLimit {
    min: u32,
    max: Option<NonZeroU32>,
    is_shared: bool,
}

impl WasmLimit {
    #[inline]
    pub const fn zero() -> Self {
        Self {
            min: 0,
            max: None,
            is_shared: false,
        }
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.max().unwrap_or(self.min()) == 0
    }

    #[inline]
    fn from_reader(reader: &mut Leb128Reader, is_memory: bool) -> Result<Self, CompileErrorKind> {
        let limit_type = reader.read().map_err(|v| v.into()).and_then(|v| {
            WasmLimitType::new(v, is_memory).ok_or(CompileErrorKind::UnexpectedToken)
        })?;

        let min = reader.read()?;
        let max = if limit_type.has_max() {
            NonZeroU32::new(reader.read()?)
        } else {
            None
        };

        let is_shared = limit_type.is_shared();

        Ok(Self {
            min,
            max,
            is_shared,
        })
    }

    #[inline]
    pub const fn min(&self) -> u32 {
        self.min
    }

    #[inline]
    pub fn max(&self) -> Option<u32> {
        self.max.map(|v| v.get())
    }

    #[inline]
    pub const fn is_shared(&self) -> bool {
        self.is_shared
    }
}

#[derive(Clone, Copy)]
pub struct WasmLimitType(u8);

impl WasmLimitType {
    const ALL_MEMORY: u64 = (
        Self::HAS_MAX
        //| Self::IS_SHARED
        //| Self::IS_MEMORY64
    ) as u64;

    const ALL_OTHER: u64 = (Self::HAS_MAX) as u64;

    const HAS_MAX: u8 = 0b0000_0001;

    const IS_SHARED: u8 = 0b0000_0010;

    const IS_MEMORY64: u8 = 0b0000_0100;

    #[inline]
    pub const fn new(value: u64, is_memory: bool) -> Option<Self> {
        if is_memory {
            if value <= Self::ALL_MEMORY {
                Some(Self(value as u8))
            } else {
                None
            }
        } else {
            if value <= Self::ALL_OTHER {
                Some(Self(value as u8))
            } else {
                None
            }
        }
    }

    /// Limit Type has `max` field
    #[inline]
    pub const fn has_max(&self) -> bool {
        (self.0 & Self::HAS_MAX) != 0
    }

    /// Limit Type is `shared`
    /// TODO: **SUPPORTED IN THE FUTURE**
    #[inline]
    pub const fn is_shared(&self) -> bool {
        (self.0 & Self::IS_SHARED) != 0
    }

    /// Limit Type is `memory64`
    /// TODO: **SUPPORTED IN THE FUTURE**
    #[inline]
    pub const fn is_memory64(&self) -> bool {
        (self.0 & Self::IS_MEMORY64) != 0
    }
}

/// WebAssembly table object
pub struct WasmTable {
    limit: WasmLimit,
    table: Vec<usize>,
}

impl WasmTable {
    #[inline]
    fn from_reader(reader: &mut Leb128Reader) -> Result<Self, CompileErrorKind> {
        match reader.read_byte() {
            Ok(0x70) => (),
            Err(err) => return Err(err.into()),
            _ => return Err(CompileErrorKind::UnexpectedToken),
        };
        WasmLimit::from_reader(reader, false).map(|limit| {
            let size = limit.min() as usize;
            let mut table = Vec::with_capacity(size);
            table.resize(size, 0);
            Self { limit, table }
        })
    }

    #[inline]
    pub const fn limit(&self) -> WasmLimit {
        self.limit
    }

    #[inline]
    pub fn table(&mut self) -> &mut [usize] {
        self.table.as_mut_slice()
    }
}

/// A type that represents the type of WebAssembly function.
///
/// There are two types of functions in WebAssembly: those that are imported from external modules and those that have bytecode in the same module.
///
/// It appears as the third section (`0x03`) in the WebAssembly binary.
pub struct WasmFunction {
    is_external: bool,
    index: usize,
    type_index: WasmTypeIndex,
    func_type: WasmType,
    content: WasmFunctionContent,
}

pub(crate) enum WasmFunctionContent {
    Unresolved,
    CodeBlock(WasmCodeBlock),
    Dynamic(WasmDynFunc),
}

impl WasmFunction {
    #[inline]
    fn from_import(index: usize, type_index: WasmTypeIndex, func_type: WasmType) -> Self {
        Self {
            is_external: true,
            index,
            type_index,
            func_type,
            content: WasmFunctionContent::Unresolved,
        }
    }

    #[inline]
    fn internal(index: usize, type_index: WasmTypeIndex, func_type: WasmType) -> Self {
        Self {
            is_external: false,
            index,
            type_index,
            func_type,
            content: WasmFunctionContent::Unresolved,
        }
    }

    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub const fn type_index(&self) -> WasmTypeIndex {
        self.type_index
    }

    #[inline]
    pub fn param_types(&self) -> &[WasmValType] {
        self.func_type.param_types.as_slice()
    }

    #[inline]
    pub fn result_types(&self) -> &[WasmValType] {
        self.func_type.result_types.as_slice()
    }

    #[inline]
    pub(crate) fn content(&self) -> &WasmFunctionContent {
        &self.content
    }

    pub(crate) fn set_code_block(
        &mut self,
        code_block: WasmCodeBlock,
    ) -> Result<(), CompileErrorKind> {
        if self.is_external {
            Err(CompileErrorKind::OutOfFunction)
        } else {
            match self.content {
                WasmFunctionContent::Unresolved => {
                    self.content = WasmFunctionContent::CodeBlock(code_block);
                    Ok(())
                }
                WasmFunctionContent::CodeBlock(_) | WasmFunctionContent::Dynamic(_) => {
                    Err(CompileErrorKind::InternalInconsistency)
                }
            }
        }
    }

    pub(crate) fn resolve(&mut self, dyn_func: WasmDynFunc) -> Result<(), LinkError> {
        if self.is_external {
            match self.content {
                WasmFunctionContent::Unresolved => {
                    self.content = WasmFunctionContent::Dynamic(dyn_func);
                    Ok(())
                }
                WasmFunctionContent::CodeBlock(_) | WasmFunctionContent::Dynamic(_) => {
                    Err(LinkError::InternalInconsistency)
                }
            }
        } else {
            Err(LinkError::InternalInconsistency)
        }
    }
}

/// A type that holds the signature of a function that combines a list of argument types with a list of return types.
///
/// It appears as the first section (`0x01`) in the WebAssembly binary.
#[derive(Debug, Clone)]
pub struct WasmType {
    param_types: SmallVec<[WasmValType; 8]>,
    result_types: SmallVec<[WasmValType; 8]>,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WasmTypeIndex(u32);

impl WasmTypeIndex {
    #[inline]
    pub fn new(module: &WasmModule, val: u32) -> Option<Self> {
        ((val as usize) < module.types.len()).then(|| Self(val))
    }

    #[inline]
    pub const fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

impl WasmType {
    fn from_reader(reader: &mut Leb128Reader) -> Result<Self, CompileErrorKind> {
        match reader.read_byte() {
            Ok(0x60) => (),
            Err(err) => return Err(err.into()),
            _ => return Err(CompileErrorKind::UnexpectedToken),
        };
        let n_params: usize = reader.read()?;
        let mut param_types = SmallVec::with_capacity(n_params);
        for _ in 0..n_params {
            reader
                .read_byte()
                .map_err(|v| v.into())
                .and_then(|v| WasmValType::from_u8(v))
                .map(|v| param_types.push(v))?;
        }
        let n_result: usize = reader.read()?;
        let mut result_types = SmallVec::with_capacity(n_result);
        for _ in 0..n_result {
            reader
                .read_byte()
                .map_err(|v| v.into())
                .and_then(|v| WasmValType::from_u8(v))
                .map(|v| result_types.push(v))?;
        }
        Ok(Self {
            param_types,
            result_types,
        })
    }

    #[inline]
    pub fn param_types(&self) -> &[WasmValType] {
        &self.param_types
    }

    #[inline]
    pub fn result_types(&self) -> &[WasmValType] {
        &self.result_types
    }

    /// Get the function signature
    ///
    /// For example:
    ///     `fn(int, long, float, double) -> void` -> `"vilfd"`
    #[inline]
    pub fn signature(&self) -> String {
        let result_types = if self.result_types.is_empty() {
            "v".to_owned()
        } else {
            self.result_types.iter().map(|v| v.mnemonic()).collect()
        };
        let param_types = if self.param_types.is_empty() {
            "v".to_owned()
        } else {
            self.param_types.iter().map(|v| v.mnemonic()).collect()
        };

        format!("{result_types}{param_types}")
    }
}

impl fmt::Display for WasmType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.param_types.len() > 0 {
            write!(f, " (param")?;
            for param in self.param_types.iter() {
                write!(f, " {}", param)?;
            }
            write!(f, ")")?;
        }
        if self.result_types.len() > 0 {
            write!(f, " (result")?;
            for result in self.result_types.iter() {
                write!(f, " {}", result)?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

/// WebAssembly import object
///
/// It appears as the second section (`0x02`) in the WebAssembly binary.
#[derive(Debug, Clone)]
pub struct WasmImport {
    mod_name: String,
    name: String,
    desc: WasmImportDescriptor,
}

impl WasmImport {
    #[inline]
    fn from_reader(reader: &mut Leb128Reader) -> Result<Self, CompileErrorKind> {
        let mod_name = reader.get_string()?;
        let name = reader.get_string()?;
        let desc = WasmImportDescriptor::from_reader(reader)?;

        Ok(Self {
            mod_name,
            name,
            desc,
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub enum WasmImportDescriptor {
    Function(WasmTypeIndex),
    // Table(_),
    Memory(WasmLimit),
    // Global(_),
}

impl WasmImportDescriptor {
    #[inline]
    fn from_reader(mut reader: &mut Leb128Reader) -> Result<Self, CompileErrorKind> {
        let import_type = reader.read_byte()?;
        match import_type {
            0 => reader
                .read()
                .map(|v| Self::Function(WasmTypeIndex(v)))
                .map_err(|v| v.into()),
            // 1 => reader.read().map(|v| Self::Table(v)),
            2 => WasmLimit::from_reader(&mut reader, true).map(|v| Self::Memory(v)),
            // 3 => reader.read().map(|v| Self::Global(v)),
            _ => Err(CompileErrorKind::UnexpectedToken),
        }
    }
}

/// WebAssembly export object
pub struct WasmExport {
    name: String,
    desc: WasmExportDesc,
}

impl WasmExport {
    #[inline]
    fn new(module: &WasmModule, reader: &mut Leb128Reader) -> Result<Self, CompileErrorKind> {
        let name = reader.get_string()?;
        let desc = WasmExportDesc::new(module, reader)?;
        Ok(Self { name, desc })
    }
}

#[derive(Debug)]
pub enum WasmExportDesc {
    Function(usize),
    Table(usize),
    Memory(usize),
    Global(GlobalVarIndex),
}

impl WasmExportDesc {
    #[inline]
    fn new(module: &WasmModule, reader: &mut Leb128Reader) -> Result<Self, CompileErrorKind> {
        reader
            .read_byte()
            .map_err(|v| v.into())
            .and_then(|v| match v {
                0 => reader
                    .read()
                    .map(|v| Self::Function(v))
                    .map_err(|v| v.into()),
                1 => reader.read().map(|v| Self::Table(v)).map_err(|v| v.into()),
                2 => reader.read().map(|v| Self::Memory(v)).map_err(|v| v.into()),
                3 => {
                    let index: u32 = reader.read()?;
                    ((index as usize) < module.globals.len())
                        .then(|| Self::Global(unsafe { GlobalVarIndex::new(index) }))
                        .ok_or(CompileErrorKind::InvalidGlobal)
                }
                _ => Err(CompileErrorKind::UnexpectedToken),
            })
    }
}

#[derive(Clone)]
pub struct CompileError {
    kind: CompileErrorKind,
    file_position: ExceptionPosition,
    source: CompileErrorSource,
}

#[derive(Debug, Clone)]
pub enum CompileErrorSource {
    Unknown,
    ConstantExpression(ExceptionPosition),
    Function(usize, Option<String>, ExceptionPosition, Option<WasmOpcode>),
}

impl CompileError {
    #[inline]
    pub fn new(
        kind: CompileErrorKind,
        file_position: ExceptionPosition,
        source: CompileErrorSource,
    ) -> Self {
        Self {
            kind,
            file_position,
            source,
        }
    }

    #[inline]
    pub fn kind(&self) -> &CompileErrorKind {
        &self.kind
    }

    #[inline]
    pub fn file_position(&self) -> &ExceptionPosition {
        &self.file_position
    }

    #[inline]
    pub fn source(&self) -> &CompileErrorSource {
        &self.source
    }

    pub fn downcast_clone(err: &Box<dyn Error>) -> Option<Self> {
        if let Some(err) = err.downcast_ref::<CompileError>() {
            Some(err.clone())
        } else if let Some(err) = err.downcast_ref::<CompileErrorKind>() {
            Some(CompileError::new(
                err.clone(),
                ExceptionPosition::UNKNOWN,
                CompileErrorSource::Unknown,
            ))
        } else {
            None
        }
    }
}

impl fmt::Debug for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self as &dyn fmt::Display).fmt(f)
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.source() {
            CompileErrorSource::Unknown => {
                if self.file_position().is_valid() {
                    write!(
                        f,
                        "CompileError: {:?} at {:x}",
                        self.kind,
                        self.file_position.position(),
                    )
                } else {
                    write!(f, "CompileError: {:?}", self.kind,)
                }
            }
            CompileErrorSource::ConstantExpression(position) => {
                write!(
                    f,
                    "CompileError: {:?} at expression 0x{:x}(0x{:x})",
                    self.kind,
                    self.file_position.position(),
                    position.position(),
                )
            }
            CompileErrorSource::Function(func_idx, name, position, bytecode) => {
                write!(
                    f,
                    "CompileError: {:?} at function[{}]{} 0x{:x}(0x{:x}) {}",
                    self.kind,
                    func_idx,
                    name.as_ref()
                        .map(|v| format!(" <{}>", v))
                        .unwrap_or("".to_string()),
                    self.file_position.position(),
                    position.position(),
                    bytecode
                        .as_ref()
                        .map(|v| format!("{:?}", v))
                        .unwrap_or("???".to_string()),
                )
            }
        }
    }
}

impl Error for CompileError {}

impl From<CompileErrorKind> for CompileError {
    #[inline]
    fn from(value: CompileErrorKind) -> Self {
        CompileError {
            kind: value,
            file_position: ExceptionPosition::UNKNOWN,
            source: CompileErrorSource::Unknown,
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExceptionPosition(u32);

impl ExceptionPosition {
    pub const UNKNOWN: Self = Self::new(0);

    #[inline]
    pub const fn new(position: usize) -> Self {
        Self(position as u32)
    }

    #[inline]
    pub const fn position(&self) -> usize {
        self.0 as usize
    }

    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CompileErrorKind {
    /// Not an executable file.
    BadExecutable,
    /// Unexpected end of stream.
    UnexpectedEof,
    /// Unexpected token detected during decoding.
    UnexpectedToken,
    /// Detected a bytecode that cannot be decoded.
    InvalidBytecode(u8),
    /// Detected a bytecode that cannot be decoded.
    InvalidBytecode2(u8, u32),
    /// Unsupported bytecode
    UnsupportedBytecode(WasmMnemonic),
    /// Unsupported global data type
    UnsupportedGlobalType(WasmValType),
    /// Unprocessable section order found.
    InvalidSectionOrder(WasmSectionId),
    /// Invalid parameter was specified.
    InvalidData,
    /// Invalid stack level.
    InvalidStackLevel,
    /// Specified a non-existent type.
    InvalidType(WasmTypeIndex),
    /// Invalid global variable specified.
    InvalidGlobal,
    /// Invalid local variable specified.
    InvalidLocal,
    /// Value stack is out of range
    OutOfStack,
    /// Branching targets are out of nest range
    OutOfBranch,
    /// Out of memory
    OutOfMemory,
    /// Code Section
    OutOfFunction,
    /// The type of the value stack does not match.
    TypeMismatch,
    /// Termination of invalid blocks
    BlockMismatch,
    /// The `else` block and the `if` block do not match.
    ElseWithoutIf,
    ///
    ElseNotExists,
    /// Internal error
    InternalInconsistency,
    /// For debugging purposes
    ForDebug(usize),
}

impl CompileErrorKind {
    pub fn downcast_ref(err: &Box<dyn Error>) -> Option<&Self> {
        if let Some(err) = err.downcast_ref::<CompileErrorKind>() {
            Some(err)
        } else if let Some(err) = err.downcast_ref::<CompileError>() {
            Some(err.kind())
        } else {
            None
        }
    }
}

impl fmt::Display for CompileErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for CompileErrorKind {}

impl From<leb128::ReadError> for CompileErrorKind {
    #[inline]
    fn from(value: leb128::ReadError) -> Self {
        match value {
            ReadError::InvalidData => CompileErrorKind::InvalidData,
            ReadError::UnexpectedEof => CompileErrorKind::UnexpectedEof,
            ReadError::OutOfBounds => CompileErrorKind::UnexpectedToken,
        }
    }
}

impl From<leb128::ReadError> for CompileError {
    #[inline]
    fn from(value: leb128::ReadError) -> Self {
        CompileError::from(CompileErrorKind::from(value))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LinkError {
    /// Imported function does not exist.
    NoMethod(String),
    /// Imported module does not exist.
    NoModule(String),

    InternalInconsistency,
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for LinkError {}

#[derive(Debug, PartialEq)]
pub enum WasmRuntimeErrorKind {
    /// (not an error) Exit the application
    Exit,
    /// (recoverable) Memory couldn't be borrowed
    MemoryBorrowError,
    /// (recoverable) Would block
    WouldBlock,
    /// (unrecoverable) Argument type mismatch (e.g., call instruction).
    InvalidParameter,
    /// (unrecoverable) Intermediate code that could not be converted
    NotSupported,
    /// (unrecoverable) The Unreachable instruction was executed.
    Unreachable,
    /// (unrecoverable) Memory Boundary Errors
    OutOfBounds,
    /// (unrecoverable) The specified function cannot be found.
    NoMethod,
    /// (unrecoverable) Device by zero
    DivideByZero,
    /// (unrecoverable) The type of call instructions do not match.
    TypeMismatch,
    /// (unrecoverable) Internal error
    InternalInconsistency,
    /// (unrecoverable) Out of Memory
    OutOfMemory,
}

/// A type that holds a WebAssembly primitive value with a type information tag.
#[derive(Debug, Copy, Clone)]
pub enum WasmValue {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl WasmValue {
    #[inline]
    pub const fn default_for(val_type: WasmValType) -> Self {
        match val_type {
            WasmValType::I32 => Self::I32(0),
            WasmValType::I64 => Self::I64(0),
            WasmValType::F32 => Self::F32(0.0),
            WasmValType::F64 => Self::F64(0.0),
        }
    }

    #[inline]
    pub const fn val_type(&self) -> WasmValType {
        match self {
            WasmValue::I32(_) => WasmValType::I32,
            WasmValue::I64(_) => WasmValType::I64,
            WasmValue::F32(_) => WasmValType::F32,
            WasmValue::F64(_) => WasmValType::F64,
        }
    }

    #[inline]
    pub const fn is_valid_type(&self, val_type: WasmValType) -> bool {
        match (*self, val_type) {
            (Self::I32(_), WasmValType::I32) => true,
            (Self::I64(_), WasmValType::I64) => true,
            (Self::F32(_), WasmValType::F32) => true,
            (Self::F64(_), WasmValType::F64) => true,
            _ => false,
        }
    }

    #[inline]
    pub const fn get_i32(self) -> Result<i32, WasmRuntimeErrorKind> {
        match self {
            Self::I32(a) => Ok(a),
            _ => return Err(WasmRuntimeErrorKind::TypeMismatch),
        }
    }

    #[inline]
    pub const fn get_u32(self) -> Result<u32, WasmRuntimeErrorKind> {
        match self {
            Self::I32(a) => Ok(a as u32),
            _ => return Err(WasmRuntimeErrorKind::TypeMismatch),
        }
    }

    #[inline]
    pub const fn get_i64(self) -> Result<i64, WasmRuntimeErrorKind> {
        match self {
            Self::I64(a) => Ok(a),
            _ => return Err(WasmRuntimeErrorKind::TypeMismatch),
        }
    }

    #[inline]
    pub const fn get_u64(self) -> Result<u64, WasmRuntimeErrorKind> {
        match self {
            Self::I64(a) => Ok(a as u64),
            _ => return Err(WasmRuntimeErrorKind::TypeMismatch),
        }
    }

    #[inline]
    pub const fn get_f32(self) -> Result<f32, WasmRuntimeErrorKind> {
        match self {
            Self::F32(a) => Ok(a),
            _ => return Err(WasmRuntimeErrorKind::TypeMismatch),
        }
    }

    #[inline]
    pub const fn get_f64(self) -> Result<f64, WasmRuntimeErrorKind> {
        match self {
            Self::F64(a) => Ok(a),
            _ => return Err(WasmRuntimeErrorKind::TypeMismatch),
        }
    }

    #[inline]
    pub fn map_i32<F>(self, f: F) -> Result<WasmValue, WasmRuntimeErrorKind>
    where
        F: FnOnce(i32) -> i32,
    {
        match self {
            Self::I32(a) => Ok(f(a).into()),
            _ => return Err(WasmRuntimeErrorKind::TypeMismatch),
        }
    }

    #[inline]
    pub fn map_i64<F>(self, f: F) -> Result<WasmValue, WasmRuntimeErrorKind>
    where
        F: FnOnce(i64) -> i64,
    {
        match self {
            Self::I64(a) => Ok(f(a).into()),
            _ => return Err(WasmRuntimeErrorKind::TypeMismatch),
        }
    }
}

impl From<i32> for WasmValue {
    #[inline]
    fn from(v: i32) -> Self {
        Self::I32(v)
    }
}

impl From<u32> for WasmValue {
    #[inline]
    fn from(v: u32) -> Self {
        Self::I32(v as i32)
    }
}

impl From<i64> for WasmValue {
    #[inline]
    fn from(v: i64) -> Self {
        Self::I64(v)
    }
}

impl From<u64> for WasmValue {
    #[inline]
    fn from(v: u64) -> Self {
        Self::I64(v as i64)
    }
}

impl From<f32> for WasmValue {
    #[inline]
    fn from(v: f32) -> Self {
        Self::F32(v)
    }
}

impl From<f64> for WasmValue {
    #[inline]
    fn from(v: f64) -> Self {
        Self::F64(v)
    }
}

impl From<bool> for WasmValue {
    #[inline]
    fn from(v: bool) -> Self {
        Self::I32(v as i32)
    }
}

impl fmt::Display for WasmValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::I32(v) => write!(f, "{}", v),
            Self::I64(v) => write!(f, "{}", v),
            Self::F32(_) => write!(f, "(#!F32)"),
            Self::F64(_) => write!(f, "(#!F64)"),
        }
    }
}

/// A shared data type for storing in the value stack in the WebAssembly interpreter.
///
/// The internal representation is `union`, so information about the type needs to be provided externally.
#[derive(Copy, Clone)]
pub union WasmUnionValue {
    usize: usize,
    i32: i32,
    u32: u32,
    i64: i64,
    u64: u64,
    f32: f32,
    f64: f64,
}

impl WasmUnionValue {
    #[inline(always)]
    const fn _is_32bit_env() -> bool {
        size_of::<usize>() == size_of::<u32>()
    }

    #[inline]
    pub const fn zero() -> Self {
        Self { u64: 0 }
    }

    #[inline]
    pub const fn from_bool(v: bool) -> Self {
        if v {
            Self::from_usize(1)
        } else {
            Self::from_usize(0)
        }
    }

    #[inline]
    pub const fn from_usize(v: usize) -> Self {
        Self { usize: v }
    }

    #[inline]
    pub const fn from_i32(v: i32) -> Self {
        Self { i32: v }
    }

    #[inline]
    pub const fn from_u32(v: u32) -> Self {
        Self { u32: v }
    }

    #[inline]
    pub const fn from_i64(v: i64) -> Self {
        Self { i64: v }
    }

    #[inline]
    pub const fn from_u64(v: u64) -> Self {
        Self { u64: v }
    }

    #[inline]
    pub const fn from_f32(v: f32) -> Self {
        Self { f32: v }
    }

    #[inline]
    pub const fn from_f64(v: f64) -> Self {
        Self { f64: v }
    }

    #[inline]
    pub unsafe fn get_bool(&self) -> bool {
        unsafe { self.i32 != 0 }
    }

    #[inline]
    pub fn write_bool(&mut self, val: bool) {
        self.usize = val as usize;
    }

    #[inline]
    pub unsafe fn get_i32(&self) -> i32 {
        unsafe { self.i32 }
    }

    #[inline]
    pub unsafe fn get_u32(&self) -> u32 {
        unsafe { self.u32 }
    }

    #[inline]
    pub fn write_i32(&mut self, val: i32) {
        self.copy_from_i32(&Self::from(val));
    }

    #[inline]
    pub unsafe fn get_i64(&self) -> i64 {
        unsafe { self.i64 }
    }

    #[inline]
    pub unsafe fn get_u64(&self) -> u64 {
        unsafe { self.u64 }
    }

    #[inline]
    pub fn write_i64(&mut self, val: i64) {
        *self = Self::from(val);
    }

    #[inline]
    pub fn write_f32(&mut self, val: f32) {
        self.copy_from_i32(&Self::from(val));
    }

    #[inline]
    pub fn write_f64(&mut self, val: f64) {
        *self = Self::from(val);
    }

    #[inline]
    pub unsafe fn get_f32(&self) -> f32 {
        unsafe { self.f32 }
    }

    #[inline]
    pub unsafe fn get_f64(&self) -> f64 {
        unsafe { self.f64 }
    }

    #[inline]
    pub unsafe fn get_i8(&self) -> i8 {
        unsafe { self.u32 as i8 }
    }

    #[inline]
    pub unsafe fn get_u8(&self) -> u8 {
        unsafe { self.u32 as u8 }
    }

    #[inline]
    pub unsafe fn get_i16(&self) -> i16 {
        unsafe { self.u32 as i16 }
    }

    #[inline]
    pub unsafe fn get_u16(&self) -> u16 {
        unsafe { self.u32 as u16 }
    }

    /// Retrieves the value held by the instance as a value of type `i32` and re-stores the value processed by the closure.
    #[inline]
    pub unsafe fn map_i32<F>(&mut self, f: F)
    where
        F: FnOnce(i32) -> i32,
    {
        let val = unsafe { self.i32 };
        self.copy_from_i32(&Self::from(f(val)));
    }

    /// Retrieves the value held by the instance as a value of type `u32` and re-stores the value processed by the closure.
    #[inline]
    pub unsafe fn map_u32<F>(&mut self, f: F)
    where
        F: FnOnce(u32) -> u32,
    {
        let val = unsafe { self.u32 };
        self.copy_from_i32(&Self::from(f(val)));
    }

    /// Retrieves the value held by the instance as a value of type `i64` and re-stores the value processed by the closure.
    #[inline]
    pub unsafe fn map_i64<F>(&mut self, f: F)
    where
        F: FnOnce(i64) -> i64,
    {
        let val = unsafe { self.i64 };
        *self = Self::from(f(val));
    }

    /// Retrieves the value held by the instance as a value of type `u64` and re-stores the value processed by the closure.
    #[inline]
    pub unsafe fn map_u64<F>(&mut self, f: F)
    where
        F: FnOnce(u64) -> u64,
    {
        let val = unsafe { self.u64 };
        *self = Self::from(f(val));
    }

    #[inline]
    pub unsafe fn map_f32<F>(&mut self, f: F)
    where
        F: FnOnce(f32) -> f32,
    {
        let val = unsafe { self.f32 };
        self.copy_from_i32(&Self::from(f(val)));
    }

    #[inline]
    pub unsafe fn map_f64<F>(&mut self, f: F)
    where
        F: FnOnce(f64) -> f64,
    {
        let val = unsafe { self.f64 };
        *self = Self::from(f(val));
    }

    /// Converts the value held by the instance to the [WasmValue] type as a value of the specified type.
    #[inline]
    pub unsafe fn get_by_type(&self, val_type: WasmValType) -> WasmValue {
        match val_type {
            WasmValType::I32 => WasmValue::I32(unsafe { self.get_i32() }),
            WasmValType::I64 => WasmValue::I64(unsafe { self.get_i64() }),
            WasmValType::F32 => WasmValue::F32(unsafe { self.get_f32() }),
            WasmValType::F64 => WasmValue::F64(unsafe { self.get_f64() }),
        }
    }

    #[inline]
    pub fn copy_from_i32(&mut self, other: &Self) {
        if Self::_is_32bit_env() {
            self.u32 = unsafe { other.u32 };
        } else {
            *self = *other;
        }
    }
}

impl Default for WasmUnionValue {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

impl From<bool> for WasmUnionValue {
    #[inline]
    fn from(v: bool) -> Self {
        Self::from_bool(v)
    }
}

impl From<u32> for WasmUnionValue {
    #[inline]
    fn from(v: u32) -> Self {
        Self::from_u32(v)
    }
}

impl From<i32> for WasmUnionValue {
    #[inline]
    fn from(v: i32) -> Self {
        Self::from_i32(v)
    }
}

impl From<u64> for WasmUnionValue {
    #[inline]
    fn from(v: u64) -> Self {
        Self::from_u64(v)
    }
}

impl From<i64> for WasmUnionValue {
    #[inline]
    fn from(v: i64) -> Self {
        Self::from_i64(v)
    }
}

impl From<f32> for WasmUnionValue {
    #[inline]
    fn from(v: f32) -> Self {
        Self::from_f32(v)
    }
}

impl From<f64> for WasmUnionValue {
    #[inline]
    fn from(v: f64) -> Self {
        Self::from_f64(v)
    }
}

impl From<WasmValue> for WasmUnionValue {
    #[inline]
    fn from(v: WasmValue) -> Self {
        match v {
            WasmValue::I32(v) => Self::from_i32(v),
            WasmValue::I64(v) => Self::from_i64(v),
            WasmValue::F32(v) => Self::from_f32(v),
            WasmValue::F64(v) => Self::from_f64(v),
        }
    }
}

pub unsafe trait UnsafeInto<T> {
    unsafe fn unsafe_into(self) -> T;
}

unsafe impl UnsafeInto<u32> for WasmUnionValue {
    #[inline]
    unsafe fn unsafe_into(self) -> u32 {
        unsafe { self.get_u32() }
    }
}

unsafe impl UnsafeInto<i32> for WasmUnionValue {
    #[inline]
    unsafe fn unsafe_into(self) -> i32 {
        unsafe { self.get_i32() }
    }
}

unsafe impl UnsafeInto<u64> for WasmUnionValue {
    #[inline]
    unsafe fn unsafe_into(self) -> u64 {
        unsafe { self.get_u64() }
    }
}

unsafe impl UnsafeInto<i64> for WasmUnionValue {
    #[inline]
    unsafe fn unsafe_into(self) -> i64 {
        unsafe { self.get_i64() }
    }
}

unsafe impl UnsafeInto<f32> for WasmUnionValue {
    #[inline]
    unsafe fn unsafe_into(self) -> f32 {
        unsafe { self.get_f32() }
    }
}

unsafe impl UnsafeInto<f64> for WasmUnionValue {
    #[inline]
    unsafe fn unsafe_into(self) -> f64 {
        unsafe { self.get_f64() }
    }
}

/// WebAssembly global variable
pub struct WasmGlobal {
    data: AtomicU64,
    val_type: WasmValType,
    is_mutable: bool,
}

impl WasmGlobal {
    #[inline]
    pub fn new(val: WasmValue, is_mutable: bool) -> Result<Self, CompileErrorKind> {
        let val_type = val.val_type();
        let val = WasmUnionValue::from(val);
        Ok(Self {
            data: AtomicU64::new(unsafe { val.get_u64() }),
            val_type,
            is_mutable,
        })
    }

    #[inline]
    pub fn raw_value(&self) -> WasmUnionValue {
        unsafe { transmute(self.data.load(Ordering::Relaxed)) }
    }

    #[inline]
    pub fn value(&self) -> WasmValue {
        unsafe { self.raw_value().get_by_type(self.val_type) }
    }

    #[inline]
    pub fn set_raw_value(&self, val: WasmUnionValue) {
        self.data.store(unsafe { transmute(val) }, Ordering::SeqCst);
    }

    #[inline]
    pub const fn val_type(&self) -> WasmValType {
        self.val_type
    }

    #[inline]
    pub const fn is_mutable(&self) -> bool {
        self.is_mutable
    }
}

/// WebAssembly name section
pub struct WasmName {
    module: Option<String>,
    functions: Vec<(usize, String)>,
    globals: Vec<(usize, String)>,
}

impl WasmName {
    pub const SECTION_NAME: &'static str = "name";

    fn from_reader(reader: &mut Leb128Reader) -> Result<Self, CompileErrorKind> {
        let mut module = None;
        let mut functions = Vec::new();
        let mut globals = Vec::new();

        while !reader.is_eof() {
            let name_id = reader.read_byte()?;
            let blob = reader.read_blob()?;
            let Some(name_id) = WasmNameSubsectionType::from_u8(name_id) else {
                continue;
            };
            let mut reader = Leb128Reader::from_slice(blob);
            match name_id {
                WasmNameSubsectionType::Module => module = reader.get_string().ok(),
                WasmNameSubsectionType::Function => {
                    let length = reader.read()?;
                    for _ in 0..length {
                        let idx: usize = reader.read()?;
                        let s = reader.get_string()?;
                        functions.push((idx, s));
                    }
                }
                WasmNameSubsectionType::Global => {
                    let length: usize = reader.read()?;
                    for _ in 0..length {
                        let idx: usize = reader.read()?;
                        let s = reader.get_string()?;
                        globals.push((idx, s));
                    }
                }
                _ => (),
            }
        }

        Ok(Self {
            module,
            functions,
            globals,
        })
    }

    #[inline]
    pub fn module(&self) -> Option<&str> {
        self.module.as_ref().map(|v| v.as_str())
    }

    #[inline]
    pub fn functions(&self) -> &[(usize, String)] {
        self.functions.as_slice()
    }

    pub fn func_by_index(&self, index: usize) -> Option<&str> {
        let functions = self.functions();
        match functions.binary_search_by_key(&index, |(k, _v)| *k) {
            Ok(v) => functions.get(v).map(|(_k, v)| v.as_str()),
            Err(_) => None,
        }
    }

    #[inline]
    pub fn globals(&self) -> &[(usize, String)] {
        self.globals.as_slice()
    }

    pub fn global_by_index(&self, idx: usize) -> Option<&str> {
        let globals = self.globals();
        match globals.binary_search_by_key(&idx, |(k, _v)| *k) {
            Ok(v) => globals.get(v).map(|(_k, v)| v.as_str()),
            Err(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum WasmNameSubsectionType {
    Module = 0,
    Function = 1,
    Local = 2,
    Labels = 3,
    Type = 4,
    Table = 5,
    Memory = 6,
    Global = 7,
    ElemSegment = 8,
    DataSegment = 9,
}

impl WasmNameSubsectionType {
    #[inline]
    pub fn from_u8(val: u8) -> Option<Self> {
        Some(match val {
            0 => Self::Module,
            1 => Self::Function,
            2 => Self::Local,
            3 => Self::Labels,
            4 => Self::Type,
            5 => Self::Table,
            6 => Self::Memory,
            7 => Self::Global,
            8 => Self::ElemSegment,
            9 => Self::DataSegment,
            _ => return None,
        })
    }
}

/// Instance type to invoke the function
#[derive(Copy, Clone)]
pub struct WasmRunnable<'a> {
    function: &'a WasmFunction,
    instance: &'a WasmInstance,
}

impl<'a> WasmRunnable<'a> {
    #[inline]
    const fn new(function: &'a WasmFunction, instance: &'a WasmInstance) -> Self {
        Self { function, instance }
    }
}

impl WasmRunnable<'_> {
    #[inline]
    pub const fn function(&self) -> &WasmFunction {
        &self.function
    }

    #[inline]
    pub const fn instance(&self) -> &WasmInstance {
        &self.instance
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalVarIndex(u32);

impl LocalVarIndex {
    #[inline]
    pub const unsafe fn new(val: u32) -> Self {
        Self(val)
    }

    #[inline]
    pub const fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalVarIndex(u32);

impl GlobalVarIndex {
    #[inline]
    pub const unsafe fn new(val: u32) -> Self {
        Self(val)
    }

    #[inline]
    pub const fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

#[repr(transparent)]
pub struct BrTableVec {
    inner: *mut u32,
}

impl BrTableVec {
    pub fn new(slice: &[u32]) -> Self {
        let mut vec = Vec::new();
        vec.push(slice.len() as u32);
        vec.extend_from_slice(slice);
        let mut slice = ManuallyDrop::new(vec.into_boxed_slice());
        let p = slice.as_mut_ptr();
        Self { inner: p }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        unsafe { self.inner.read() as usize }
    }
}

impl Drop for BrTableVec {
    fn drop(&mut self) {
        let len = self.len() + 1;
        let vec = unsafe { Box::from_raw(slice_from_raw_parts_mut(self.inner, len)) };
        drop(vec);
    }
}

impl Deref for BrTableVec {
    type Target = [u32];

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.inner.add(1), self.len()) }
    }
}

impl DerefMut for BrTableVec {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { slice::from_raw_parts_mut(self.inner.add(1), self.len()) }
    }
}

impl Clone for BrTableVec {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.deref())
    }
}

impl<'a> ReadLeb128<'a, BrTableVec> for Leb128Reader<'_> {
    #[inline]
    fn read(&'a mut self) -> Result<BrTableVec, ReadError> {
        let table_len: usize = self.read()?;
        let table_len = table_len + 1;
        let mut table = Vec::with_capacity(table_len);
        for _ in 0..table_len {
            let br: u32 = self.read()?;
            table.push(br);
        }
        Ok(BrTableVec::new(&table))
    }
}

impl fmt::Debug for BrTableVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BrTable").finish()
    }
}
