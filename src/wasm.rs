use crate::{intcode::*, opcode::*, *};
use alloc::{borrow::ToOwned, boxed::Box, format, string::*, vec::Vec};
use bitflags::*;
use core::{
    cell::{RefCell, UnsafeCell},
    fmt,
    mem::{size_of, transmute},
    ops::*,
    slice, str,
    sync::atomic::{AtomicU32, Ordering},
};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

pub type WasmDynFunc =
    fn(&WasmModule, &[WasmUnsafeValue]) -> Result<WasmValue, WasmRuntimeErrorKind>;

pub enum ImportResult<T> {
    Ok(T),
    NoModule,
    NoMethod,
    Later,
}

/// WebAssembly loader
pub struct WasmLoader {
    module: WasmModule,
}

impl WasmLoader {
    /// Minimal valid module size, Magic(4) + Version(4) + Empty sections(0) = 8
    const MINIMAL_MOD_SIZE: usize = 8;
    /// Magic number of WebAssembly Binary Format
    const MAGIC: u32 = 0x6D736100;
    /// Current Version
    const VER_CURRENT: u32 = 0x0000_0001;

    #[inline]
    pub const fn new() -> Self {
        Self {
            module: WasmModule::new(),
        }
    }

    /// Identify the file format
    #[inline]
    pub fn identity(blob: &[u8]) -> bool {
        blob.len() >= Self::MINIMAL_MOD_SIZE
            && unsafe { WasmEndian::read_u32(blob, 0) } == Self::MAGIC
            && unsafe { WasmEndian::read_u32(blob, 4) } == Self::VER_CURRENT
    }

    /// Instantiate wasm modules from slice
    pub fn instantiate<F>(blob: &[u8], resolver: F) -> Result<WasmModule, WasmDecodeErrorKind>
    where
        F: FnMut(&str, &str, &WasmType) -> ImportResult<WasmDynFunc> + Copy,
    {
        if Self::identity(blob) {
            let mut loader = Self::new();
            loader.load(blob, resolver).map(|_| loader.module)
        } else {
            return Err(WasmDecodeErrorKind::BadExecutable);
        }
    }

    /// Load wasm from slice
    pub fn load<F>(&mut self, blob: &[u8], import_resolver: F) -> Result<(), WasmDecodeErrorKind>
    where
        F: FnMut(&str, &str, &WasmType) -> ImportResult<WasmDynFunc> + Copy,
    {
        let mut blob = Leb128Stream::from_slice(&blob[8..]);
        while let Some(mut section) = blob.next_section()? {
            match section.section_type {
                WasmSectionType::Custom => {
                    match section.stream.get_string() {
                        Ok(WasmName::SECTION_NAME) => {
                            self.module.names = WasmName::from_stream(&mut section.stream).ok()
                        }
                        _ => (),
                    }
                    Ok(())
                }
                WasmSectionType::Type => self.parse_sec_type(section),
                WasmSectionType::Import => self.parse_sec_import(section, import_resolver),
                WasmSectionType::Table => self.parse_sec_table(section),
                WasmSectionType::Memory => self.parse_sec_memory(section),
                WasmSectionType::Element => self.parse_sec_elem(section),
                WasmSectionType::Function => self.parse_sec_func(section),
                WasmSectionType::Export => self.parse_sec_export(section),
                WasmSectionType::Code => self.parse_sec_code(section),
                WasmSectionType::Data => self.parse_sec_data(section),
                WasmSectionType::Start => self.parse_sec_start(section),
                WasmSectionType::Global => self.parse_sec_global(section),
                WasmSectionType::DataCount => self.parse_sec_data_count(section),
            }?;
        }

        self.module.types.shrink_to_fit();
        self.module.imports.shrink_to_fit();
        self.module.functions.shrink_to_fit();
        self.module.tables.shrink_to_fit();
        self.module.memories.shrink_to_fit();
        self.module.exports.shrink_to_fit();

        Ok(())
    }

    /// Returns a module
    #[inline]
    pub const fn module(&self) -> &WasmModule {
        &self.module
    }

    /// Consumes self and returns a module.
    #[inline]
    pub fn into_module(self) -> WasmModule {
        self.module
    }

    /// Parse "type" section
    fn parse_sec_type(&mut self, mut section: WasmSection) -> Result<(), WasmDecodeErrorKind> {
        let n_items = section.stream.read_unsigned()? as usize;
        for _ in 0..n_items {
            let ft = WasmType::from_stream(&mut section.stream)?;
            self.module.types.push(ft);
        }
        Ok(())
    }

    /// Parse "import" section
    fn parse_sec_import<F>(
        &mut self,
        mut section: WasmSection,
        mut resolver: F,
    ) -> Result<(), WasmDecodeErrorKind>
    where
        F: FnMut(&str, &str, &WasmType) -> ImportResult<WasmDynFunc> + Copy,
    {
        let n_items = section.stream.read_unsigned()? as usize;
        for _ in 0..n_items {
            let mut import = WasmImport::from_stream(&mut section.stream)?;
            match import.index {
                WasmImportIndex::Type(index) => {
                    import.func_ref = self.module.n_ext_func;
                    let func_type = self
                        .module
                        .types
                        .get(index)
                        .ok_or(WasmDecodeErrorKind::InvalidType)?;
                    let dlink = match resolver(import.mod_name(), import.name(), func_type) {
                        ImportResult::Ok(v) => v,
                        ImportResult::NoMethod => {
                            return Err(WasmDecodeErrorKind::NoMethod(import.name().to_owned()))
                        }
                        ImportResult::NoModule => {
                            return Err(WasmDecodeErrorKind::NoModule(import.mod_name().to_owned()))
                        }
                        ImportResult::Later => todo!(),
                    };
                    self.module.functions.push(WasmFunction::from_import(
                        self.module.n_ext_func,
                        index,
                        func_type.clone(),
                        dlink,
                    ));
                    self.module.n_ext_func += 1;
                }
                WasmImportIndex::Memory(memtype) => {
                    // TODO: import memory
                    self.module.memories.push(WasmMemory::new(memtype));
                }
            }
            self.module.imports.push(import);
        }
        Ok(())
    }

    /// Parse "func" section
    fn parse_sec_func(&mut self, mut section: WasmSection) -> Result<(), WasmDecodeErrorKind> {
        let n_items = section.stream.read_unsigned()? as usize;
        let base_index = self.module.imports.len();
        for index in 0..n_items {
            let type_index = section.stream.read_unsigned()? as usize;
            let func_type = self
                .module
                .types
                .get(type_index)
                .ok_or(WasmDecodeErrorKind::InvalidType)?;
            self.module.functions.push(WasmFunction::internal(
                base_index + index,
                type_index,
                func_type.clone(),
            ));
        }
        Ok(())
    }

    /// Parse "export" section
    fn parse_sec_export(&mut self, mut section: WasmSection) -> Result<(), WasmDecodeErrorKind> {
        let n_items = section.stream.read_unsigned()? as usize;
        for i in 0..n_items {
            let export = WasmExport::from_stream(&mut section.stream)?;
            if let WasmExportIndex::Function(index) = export.index {
                self.module
                    .functions
                    .get_mut(index)
                    .map(|v| v.origin = WasmFunctionOrigin::Export(i));
            }
            self.module.exports.push(export);
        }
        Ok(())
    }

    /// Parse "memory" section
    fn parse_sec_memory(&mut self, mut section: WasmSection) -> Result<(), WasmDecodeErrorKind> {
        let n_items = section.stream.read_unsigned()?;
        for _ in 0..n_items {
            let limit = WasmLimit::from_stream(&mut section.stream)?;
            self.module.memories.push(WasmMemory::new(limit));
        }
        Ok(())
    }

    /// Parse "table" section
    fn parse_sec_table(&mut self, mut section: WasmSection) -> Result<(), WasmDecodeErrorKind> {
        let n_items = section.stream.read_unsigned()?;
        for _ in 0..n_items {
            let table = WasmTable::from_stream(&mut section.stream)?;
            self.module.tables.push(table);
        }
        Ok(())
    }

    /// Parse "elem" section
    fn parse_sec_elem(&mut self, mut section: WasmSection) -> Result<(), WasmDecodeErrorKind> {
        let n_items = section.stream.read_unsigned()?;
        for _ in 0..n_items {
            let tabidx = section.stream.read_unsigned()? as usize;
            let offset = self.eval_offset(&mut section.stream)? as usize;
            let n_elements = section.stream.read_unsigned()? as usize;
            let table = self
                .module
                .tables
                .get_mut(tabidx)
                .ok_or(WasmDecodeErrorKind::InvalidParameter)?;
            for i in offset..offset + n_elements {
                let elem = section.stream.read_unsigned()? as usize;
                table.table.get_mut(i).map(|v| *v = elem);
            }
        }
        Ok(())
    }

    /// Parse "code" section
    fn parse_sec_code(&mut self, mut section: WasmSection) -> Result<(), WasmDecodeErrorKind> {
        let n_items = section.stream.read_unsigned()? as usize;
        for i in 0..n_items {
            let index = i + self.module.n_ext_func;
            let module = &mut self.module;
            let func_def = module
                .functions
                .get(index)
                .ok_or(WasmDecodeErrorKind::InvalidParameter)?;
            let length = section.stream.read_unsigned()? as usize;
            let file_position = section.file_position() + section.stream.position();
            let blob = section.stream.get_bytes(length)?;
            let mut stream = Leb128Stream::from_slice(blob);
            let body = WasmCodeBlock::generate(
                index,
                file_position,
                &mut stream,
                func_def.param_types(),
                func_def.result_types(),
                module,
            )?;

            self.module.functions[index].code_block = Some(body);
        }
        Ok(())
    }

    /// Parse "data" section
    fn parse_sec_data(&mut self, mut section: WasmSection) -> Result<(), WasmDecodeErrorKind> {
        let n_items = section.stream.read_unsigned()?;
        for _ in 0..n_items {
            let memidx = section.stream.read_unsigned()? as usize;
            let offset = self.eval_offset(&mut section.stream)?;
            let src = section.stream.read_bytes()?;
            let memory = self
                .module
                .memories
                .get_mut(memidx)
                .ok_or(WasmDecodeErrorKind::InvalidParameter)?;
            memory.write_slice(offset, src).unwrap();
        }
        Ok(())
    }

    /// Parse "start" section
    fn parse_sec_start(&mut self, mut section: WasmSection) -> Result<(), WasmDecodeErrorKind> {
        let index = section.stream.read_unsigned()? as usize;
        self.module.start = Some(index);
        Ok(())
    }

    /// Parse "global" section
    fn parse_sec_global(&mut self, mut section: WasmSection) -> Result<(), WasmDecodeErrorKind> {
        let n_items = section.stream.read_unsigned()? as usize;
        for _ in 0..n_items {
            let val_type = section
                .stream
                .read_byte()
                .and_then(|v| WasmValType::from_u64(v as u64))?;
            let is_mutable = section.stream.read_byte()? == 1;
            let value = self.eval_expr(&mut section.stream)?;

            if !value.is_valid_type(val_type) {
                return Err(WasmDecodeErrorKind::InvalidGlobal);
            }

            WasmGlobal::new(value, is_mutable).map(|v| self.module.globals.push(v))?;
        }
        Ok(())
    }

    /// Parse "datacount" section
    fn parse_sec_data_count(
        &mut self,
        mut section: WasmSection,
    ) -> Result<(), WasmDecodeErrorKind> {
        let count = section.stream.read_unsigned()? as usize;
        self.module.data_count = Some(count);
        Ok(())
    }

    fn eval_offset(&self, mut stream: &mut Leb128Stream) -> Result<usize, WasmDecodeErrorKind> {
        self.eval_expr(&mut stream)
            .and_then(|v| {
                v.get_i32()
                    .map_err(|_| WasmDecodeErrorKind::InvalidParameter)
            })
            .map(|v| v as usize)
    }

    fn eval_expr(&self, stream: &mut Leb128Stream) -> Result<WasmValue, WasmDecodeErrorKind> {
        stream
            .read_byte()
            .and_then(|opc| match WasmSingleOpcode::new(opc) {
                Some(WasmSingleOpcode::I32Const) => stream.read_signed().and_then(|r| {
                    stream
                        .read_byte()
                        .and_then(|v| match WasmSingleOpcode::new(v) {
                            Some(WasmSingleOpcode::End) => Ok(WasmValue::I32(r as i32)),
                            _ => Err(WasmDecodeErrorKind::UnexpectedToken),
                        })
                }),
                Some(WasmSingleOpcode::I64Const) => stream.read_signed().and_then(|r| {
                    stream
                        .read_byte()
                        .and_then(|v| match WasmSingleOpcode::new(v) {
                            Some(WasmSingleOpcode::End) => Ok(WasmValue::I64(r)),
                            _ => Err(WasmDecodeErrorKind::UnexpectedToken),
                        })
                }),
                _ => Err(WasmDecodeErrorKind::UnexpectedToken),
            })
    }
}

/// WebAssembly module
pub struct WasmModule {
    types: Vec<WasmType>,
    imports: Vec<WasmImport>,
    exports: Vec<WasmExport>,
    memories: Vec<WasmMemory>,
    tables: Vec<WasmTable>,
    functions: Vec<WasmFunction>,
    start: Option<usize>,
    globals: Vec<WasmGlobal>,
    data_count: Option<usize>,
    names: Option<WasmName>,
    n_ext_func: usize,
}

impl WasmModule {
    #[inline]
    pub const fn new() -> Self {
        Self {
            types: Vec::new(),
            memories: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            tables: Vec::new(),
            functions: Vec::new(),
            start: None,
            globals: Vec::new(),
            data_count: None,
            names: None,
            n_ext_func: 0,
        }
    }

    #[inline]
    pub fn types(&self) -> &[WasmType] {
        self.types.as_slice()
    }

    #[inline]
    pub fn type_by_ref(&self, index: usize) -> Option<&WasmType> {
        self.types.get(index)
    }

    #[inline]
    pub fn imports(&self) -> &[WasmImport] {
        self.imports.as_slice()
    }

    #[inline]
    pub fn exports(&self) -> &[WasmExport] {
        self.exports.as_slice()
    }

    #[inline]
    pub fn memories(&self) -> &[WasmMemory] {
        self.memories.as_slice()
    }

    #[inline]
    pub fn memories_mut(&mut self) -> &mut [WasmMemory] {
        self.memories.as_mut_slice()
    }

    #[inline]
    pub fn has_memory(&self) -> bool {
        self.memories.len() > 0
    }

    #[inline]
    pub fn memory(&self, index: usize) -> Option<&WasmMemory> {
        self.memories.get(index)
    }

    #[inline]
    pub unsafe fn memory_unchecked(&self, index: usize) -> &WasmMemory {
        unsafe { self.memories.get_unchecked(index) }
    }

    #[inline]
    pub fn tables(&mut self) -> &mut [WasmTable] {
        self.tables.as_mut_slice()
    }

    #[inline]
    pub fn elem_get(&self, index: usize) -> Option<&WasmFunction> {
        self.tables
            .get(0)
            .and_then(|v| v.table.get(index))
            .and_then(|v| self.functions.get(*v))
    }

    #[inline]
    pub fn functions(&self) -> &[WasmFunction] {
        self.functions.as_slice()
    }

    #[inline]
    pub fn func_by_index(&self, index: usize) -> Result<WasmRunnable, WasmRuntimeErrorKind> {
        self.functions
            .get(index)
            .map(|v| WasmRunnable::from_function(v, self))
            .ok_or(WasmRuntimeErrorKind::NoMethod)
    }

    #[inline]
    pub(crate) fn codeblock(&self, index: usize) -> Option<&WasmCodeBlock> {
        self.functions.get(index).and_then(|v| v.code_block())
    }

    #[inline]
    pub fn entry_point(&self) -> Result<WasmRunnable, WasmRuntimeErrorKind> {
        self.start
            .ok_or(WasmRuntimeErrorKind::NoMethod)
            .and_then(|v| self.func_by_index(v))
    }

    /// Get a reference to the exported function with the specified name
    #[inline]
    pub fn func(&self, name: &str) -> Result<WasmRunnable, WasmRuntimeErrorKind> {
        for export in &self.exports {
            if let WasmExportIndex::Function(v) = export.index {
                if export.name == name {
                    return self.func_by_index(v);
                }
            }
        }
        Err(WasmRuntimeErrorKind::NoMethod)
    }

    #[inline]
    pub fn globals(&self) -> &[WasmGlobal] {
        self.globals.as_slice()
    }

    #[inline]
    pub fn global_get(&self, index: usize) -> Option<&WasmGlobal> {
        self.globals.get(index)
    }

    #[inline]
    pub unsafe fn global_get_unchecked(&self, index: usize) -> &WasmGlobal {
        unsafe { self.globals.get_unchecked(index) }
    }

    #[inline]
    pub fn data_count(&self) -> Option<usize> {
        self.data_count
    }

    #[inline]
    pub fn names(&self) -> Option<&WasmName> {
        self.names.as_ref()
    }
}

struct WasmEndian;

impl WasmEndian {
    #[inline]
    unsafe fn read_u16(slice: &[u8], offset: usize) -> u16 {
        unsafe {
            let p = slice.as_ptr().add(offset) as *const u16;
            p.read_unaligned().to_le()
        }
    }

    #[inline]
    unsafe fn read_u32(slice: &[u8], offset: usize) -> u32 {
        unsafe {
            let p = slice.as_ptr().add(offset) as *const u32;
            p.read_unaligned().to_le()
        }
    }

    #[inline]
    unsafe fn read_u64(slice: &[u8], offset: usize) -> u64 {
        unsafe {
            let p = slice.as_ptr().add(offset) as *const u64;
            p.read_unaligned().to_le()
        }
    }

    #[inline]
    unsafe fn write_u16(slice: &mut [u8], offset: usize, val: u16) {
        unsafe {
            let p = slice.as_mut_ptr().add(offset) as *mut u16;
            p.write_unaligned(val.to_le());
        }
    }

    #[inline]
    unsafe fn write_u32(slice: &mut [u8], offset: usize, val: u32) {
        unsafe {
            let p = slice.as_mut_ptr().add(offset) as *mut u32;
            p.write_unaligned(val.to_le());
        }
    }

    #[inline]
    unsafe fn write_u64(slice: &mut [u8], offset: usize, val: u64) {
        unsafe {
            let p = slice.as_mut_ptr().add(offset) as *mut u64;
            p.write_unaligned(val.to_le());
        }
    }
}

/// Stream encoded with LEB128
pub struct Leb128Stream<'a> {
    blob: &'a [u8],
    position: usize,
}

impl<'a> Leb128Stream<'a> {
    /// Instantiates from a slice
    #[inline]
    pub const fn from_slice(slice: &'a [u8]) -> Self {
        Self {
            blob: slice,
            position: 0,
        }
    }

    #[inline]
    pub fn cloned(&self) -> Self {
        Self {
            blob: self.blob,
            position: self.position,
        }
    }
}

#[allow(dead_code)]
impl Leb128Stream<'_> {
    /// Returns to the origin of the stream
    #[inline]
    pub fn reset(&mut self) {
        self.position = 0;
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.blob.len()
    }

    /// Gets current position of stream
    #[inline]
    pub const fn position(&self) -> usize {
        self.position
    }

    #[inline]
    pub fn set_position(&mut self, val: usize) {
        self.position = val;
    }

    /// Returns whether the end of the stream has been reached
    #[inline]
    pub const fn is_eof(&self) -> bool {
        self.position >= self.blob.len()
    }

    /// Reads one byte from a stream
    #[inline]
    pub fn read_byte(&mut self) -> Result<u8, WasmDecodeErrorKind> {
        self.blob
            .get(self.position)
            .map(|v| {
                self.position += 1;
                *v
            })
            .ok_or(WasmDecodeErrorKind::UnexpectedEof)
    }

    /// Returns a slice of the specified number of bytes from the stream
    pub fn get_bytes(&mut self, size: usize) -> Result<&[u8], WasmDecodeErrorKind> {
        self.blob
            .get(self.position..self.position + size)
            .map(|v| {
                self.position += size;
                v
            })
            .ok_or(WasmDecodeErrorKind::UnexpectedEof)
    }

    /// Reads multiple bytes from the stream
    #[inline]
    pub fn read_bytes(&mut self) -> Result<&[u8], WasmDecodeErrorKind> {
        self.read_unsigned()
            .and_then(move |size| self.get_bytes(size as usize))
    }

    /// Reads an unsigned integer from a stream
    pub fn read_unsigned(&mut self) -> Result<u64, WasmDecodeErrorKind> {
        let mut value: u64 = 0;
        let mut scale = 0;
        let mut cursor = self.position;
        loop {
            let d = match self.blob.get(cursor) {
                Some(v) => *v,
                None => return Err(WasmDecodeErrorKind::UnexpectedEof),
            };
            cursor += 1;

            value |= (d as u64 & 0x7F) << scale;
            scale += 7;
            if (d & 0x80) == 0 {
                break;
            }
        }
        self.position = cursor;
        Ok(value)
    }

    /// Reads a signed integer from a stream
    pub fn read_signed(&mut self) -> Result<i64, WasmDecodeErrorKind> {
        let mut value: u64 = 0;
        let mut scale = 0;
        let mut cursor = self.position;
        let signed = loop {
            let d = match self.blob.get(cursor) {
                Some(v) => *v,
                None => return Err(WasmDecodeErrorKind::UnexpectedEof),
            };
            cursor += 1;

            value |= (d as u64 & 0x7F) << scale;
            let signed = (d & 0x40) != 0;
            if (d & 0x80) == 0 {
                break signed;
            }
            scale += 7;
        };
        self.position = cursor;
        if signed {
            Ok((value | 0xFFFF_FFFF_FFFF_FFC0 << scale) as i64)
        } else {
            Ok(value as i64)
        }
    }

    #[inline]
    pub fn read_f32(&mut self) -> Result<f32, WasmDecodeErrorKind> {
        self.get_bytes(4)
            .map(|v| unsafe { transmute(WasmEndian::read_u32(v, 0)) })
    }

    #[inline]
    pub fn read_f64(&mut self) -> Result<f64, WasmDecodeErrorKind> {
        self.get_bytes(8)
            .map(|v| unsafe { transmute(WasmEndian::read_u64(v, 0)) })
    }

    /// Reads the UTF-8 encoded string from the stream
    #[inline]
    pub fn get_string(&mut self) -> Result<&str, WasmDecodeErrorKind> {
        self.read_bytes()
            .and_then(|v| str::from_utf8(v).map_err(|_| WasmDecodeErrorKind::UnexpectedToken))
    }

    #[inline]
    pub fn read_opcode(&mut self) -> Result<WasmOpcode, WasmDecodeErrorKind> {
        self.read_byte().and_then(|lead| {
            WasmOpcode::decode(lead, WasmDecodeErrorKind::InvalidBytecode(lead), || {
                self.read_unsigned().map(|v| v as u32)
            })
        })
    }

    #[inline]
    pub fn read_memarg(&mut self) -> Result<WasmMemArg, WasmDecodeErrorKind> {
        let a = self.read_unsigned()? as u32;
        let o = self.read_unsigned()? as u32;
        Ok(WasmMemArg::new(o, a))
    }

    fn next_section_triple(
        &mut self,
    ) -> Result<Option<(WasmSectionType, usize, usize)>, WasmDecodeErrorKind> {
        if self.is_eof() {
            return Ok(None);
        }
        let section_type = self.read_byte()?;
        let Some(section_type) = FromPrimitive::from_u8(section_type) else {
                    return Err(WasmDecodeErrorKind::UnexpectedToken)
                };

        let magic = 8;
        let length = self.read_unsigned()? as usize;
        let file_position = self.position + magic;
        self.position += length;

        Ok(Some((section_type, file_position, length)))
    }

    fn next_section(&mut self) -> Result<Option<WasmSection>, WasmDecodeErrorKind> {
        let magic = 8;
        self.next_section_triple().map(|v| {
            v.map(|(section_type, file_position, length)| {
                let stream = Leb128Stream::from_slice(
                    &self.blob[file_position - magic..file_position + length - magic],
                );
                WasmSection {
                    section_type,
                    file_position,
                    stream,
                }
            })
        })
    }

    pub fn write_unsigned(vec: &mut Vec<u8>, value: u64) {
        let mut value = value;
        loop {
            let byte = value & 0x7F;
            value >>= 7;
            if value == 0 {
                vec.push(byte as u8);
                break;
            } else {
                vec.push(0x80 | byte as u8);
            }
        }
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

    #[inline]
    pub const fn offset_by(&self, base: u32) -> usize {
        (self.offset as u64 + base as u64) as usize
    }
}

/// WebAssembly section stream
pub struct WasmSection<'a> {
    section_type: WasmSectionType,
    file_position: usize,
    stream: Leb128Stream<'a>,
}

impl WasmSection<'_> {
    #[inline]
    pub const fn section_type(&self) -> WasmSectionType {
        self.section_type
    }

    #[inline]
    pub const fn file_position(&self) -> usize {
        self.file_position
    }

    #[inline]
    pub const fn content_size(&self) -> usize {
        self.stream.len()
    }

    #[inline]
    pub fn custom_section_name(&self) -> Option<String> {
        if self.section_type != WasmSectionType::Custom {
            return None;
        }
        let mut blob = self.stream.cloned();
        blob.reset();
        blob.get_string().map(|v| v.to_string()).ok()
    }

    pub fn write_to_vec(&self, vec: &mut Vec<u8>) {
        vec.push(self.section_type() as u8);
        Leb128Stream::write_unsigned(vec, self.content_size() as u64);
        vec.extend_from_slice(self.stream.blob);
    }
}

/// WebAssembly section types
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, FromPrimitive)]
pub enum WasmSectionType {
    Custom = 0,
    Type,
    Import,
    Function,
    Table,
    Memory,
    Global,
    Export,
    Start,
    Element,
    Code,
    Data,
    DataCount,
}

/// WebAssembly primitive types
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum WasmValType {
    I32 = 0x7F,
    I64 = 0x7E,
    F32 = 0x7D,
    F64 = 0x7C,
}

impl WasmValType {
    #[inline]
    const fn from_u64(v: u64) -> Result<Self, WasmDecodeErrorKind> {
        match v {
            0x7F => Ok(WasmValType::I32),
            0x7E => Ok(WasmValType::I64),
            0x7D => Ok(WasmValType::F32),
            0x7C => Ok(WasmValType::F64),
            _ => Err(WasmDecodeErrorKind::UnexpectedToken),
        }
    }

    #[inline]
    pub fn mnemonic(&self) -> char {
        match *self {
            WasmValType::I32 => 'i',
            WasmValType::I64 => 'l',
            WasmValType::F32 => 'f',
            WasmValType::F64 => 'd',
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
                WasmValType::I32 => "i32",
                WasmValType::I64 => "i64",
                WasmValType::F32 => "f32",
                WasmValType::F64 => "f64",
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
}

impl WasmBlockType {
    pub const fn from_i64(v: i64) -> Result<Self, WasmDecodeErrorKind> {
        match v {
            -64 => Ok(Self::Empty),
            -1 => Ok(Self::I32),
            -2 => Ok(Self::I64),
            -3 => Ok(Self::F32),
            -4 => Ok(Self::F64),
            _ => Err(WasmDecodeErrorKind::InvalidParameter),
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

/// WebAssembly memory limit
#[derive(Debug, Copy, Clone)]
pub struct WasmLimit {
    min: u32,
    max: u32,
}

impl WasmLimit {
    #[inline]
    fn from_stream(stream: &mut Leb128Stream) -> Result<Self, WasmDecodeErrorKind> {
        match stream.read_unsigned() {
            Ok(0) => stream.read_unsigned().map(|min| Self {
                min: min as u32,
                max: min as u32,
            }),
            Ok(1) => {
                let min = stream.read_unsigned()? as u32;
                let max = stream.read_unsigned()? as u32;
                Ok(Self { min, max })
            }
            Err(err) => Err(err),
            _ => Err(WasmDecodeErrorKind::UnexpectedToken),
        }
    }

    #[inline]
    pub const fn min(&self) -> u32 {
        self.min
    }

    #[inline]
    pub const fn max(&self) -> u32 {
        self.max
    }
}

/// WebAssembly memory object
pub struct WasmMemory {
    limit: WasmLimit,
    data: UnsafeCell<Vec<u8>>,
}

impl WasmMemory {
    /// The length of the vector always is a multiple of the WebAssembly page size,
    /// which is defined to be the constant 65536 – abbreviated 64Ki.
    pub const PAGE_SIZE: usize = 65536;

    #[inline]
    pub fn new(limit: WasmLimit) -> Self {
        let size = limit.min as usize * Self::PAGE_SIZE;
        let mut data = Vec::with_capacity(size);
        data.resize(size, 0);
        Self {
            limit,
            data: UnsafeCell::new(data),
        }
    }

    #[inline]
    pub const fn limit(&self) -> WasmLimit {
        self.limit
    }

    #[inline]
    fn as_slice(&self) -> &[u8] {
        unsafe { &*self.data.get() }
    }

    #[inline]
    fn as_mut_slice(&self) -> &mut [u8] {
        unsafe { &mut *self.data.get() }
    }

    /// memory.size
    #[inline]
    pub fn size(&self) -> i32 {
        let memory = self.as_slice();
        (memory.len() / Self::PAGE_SIZE) as i32
    }

    /// memory.grow
    pub fn grow(&self, delta: i32) -> i32 {
        let memory = unsafe { &mut *self.data.get() };
        let old_size = memory.len();
        if delta > 0 {
            let additional = delta as usize * Self::PAGE_SIZE;
            if memory.try_reserve(additional).is_err() {
                return -1;
            }
            memory.resize(old_size + additional, 0);
            (old_size / Self::PAGE_SIZE) as i32
        } else if delta == 0 {
            (old_size / Self::PAGE_SIZE) as i32
        } else {
            -1
        }
    }

    pub fn slice<'a>(
        &'a self,
        offset: usize,
        size: usize,
    ) -> Result<&'a [u8], WasmRuntimeErrorKind> {
        let memory = self.as_slice();
        let limit = memory.len();
        if offset < limit && size < limit && offset + size < limit {
            Ok(unsafe { slice::from_raw_parts(memory.as_ptr().add(offset), size) })
        } else {
            Err(WasmRuntimeErrorKind::OutOfBounds)
        }
    }

    pub unsafe fn slice_mut<'a>(
        &'a self,
        offset: usize,
        size: usize,
    ) -> Result<&'a mut [u8], WasmRuntimeErrorKind> {
        let memory = self.as_mut_slice();
        let limit = memory.len();
        if offset < limit && size < limit && offset + size < limit {
            Ok(unsafe { slice::from_raw_parts_mut(memory.as_mut_ptr().add(offset), size) })
        } else {
            Err(WasmRuntimeErrorKind::OutOfBounds)
        }
    }

    pub unsafe fn transmute<T>(&self, offset: usize) -> Result<&T, WasmRuntimeErrorKind> {
        let memory = self.as_slice();
        let limit = memory.len();
        let size = size_of::<T>();
        if offset < limit && size < limit && offset + size < limit {
            Ok(unsafe { transmute(memory.as_ptr().add(offset)) })
        } else {
            Err(WasmRuntimeErrorKind::OutOfBounds)
        }
    }

    pub fn read_u32_array(
        &self,
        offset: usize,
        len: usize,
    ) -> Result<&[u32], WasmRuntimeErrorKind> {
        let memory = self.as_slice();
        let limit = memory.len();
        let size = len * 4;
        if offset < limit && size < limit && offset + size < limit {
            unsafe {
                Ok(slice::from_raw_parts(
                    memory.as_ptr().add(offset) as *const u32,
                    len,
                ))
            }
        } else {
            Err(WasmRuntimeErrorKind::OutOfBounds)
        }
    }

    /// Write slice to memory
    pub fn write_slice(&self, offset: usize, src: &[u8]) -> Result<(), WasmRuntimeErrorKind> {
        let memory = self.as_mut_slice();
        let count = src.len();
        let limit = memory.len();
        if offset < limit && count < limit && offset + count < limit {
            unsafe {
                memory
                    .as_mut_ptr()
                    .add(offset)
                    .copy_from_nonoverlapping(src.as_ptr(), count);
            }
            Ok(())
        } else {
            Err(WasmRuntimeErrorKind::OutOfBounds)
        }
    }

    pub fn write_bytes(
        &self,
        offset: usize,
        val: u8,
        count: usize,
    ) -> Result<(), WasmRuntimeErrorKind> {
        let memory = self.as_mut_slice();
        let limit = memory.len();
        if offset < limit && count < limit && offset + count < limit {
            unsafe {
                memory.as_mut_ptr().add(offset).write_bytes(val, count);
            }
            Ok(())
        } else {
            Err(WasmRuntimeErrorKind::OutOfBounds)
        }
    }

    pub fn copy(&self, dest: usize, src: usize, count: usize) -> Result<(), WasmRuntimeErrorKind> {
        let memory = self.as_mut_slice();
        let limit = memory.len();
        if dest < limit
            && src < limit
            && count < limit
            && dest + count < limit
            && src + count < limit
        {
            unsafe {
                memory
                    .as_mut_ptr()
                    .add(dest)
                    .copy_from(memory.as_ptr().add(src), count);
            }
            Ok(())
        } else {
            Err(WasmRuntimeErrorKind::OutOfBounds)
        }
    }

    #[inline]
    fn effective_address(
        offset: u32,
        index: u32,
        limit: usize,
    ) -> Result<usize, WasmRuntimeErrorKind> {
        let ea = (offset as u64).wrapping_add(index as u64);
        if ea < limit as u64 {
            Ok(ea as usize)
        } else {
            Err(WasmRuntimeErrorKind::OutOfBounds)
        }
    }

    #[inline]
    pub fn read_u8(&self, offset: u32, index: u32) -> Result<u8, WasmRuntimeErrorKind> {
        let slice = self.as_slice();
        Self::effective_address(offset, index, slice.len())
            .map(|ea| unsafe { slice.as_ptr().add(ea).read() })
    }

    #[inline]
    pub fn write_u8(&self, offset: u32, index: u32, val: u8) -> Result<(), WasmRuntimeErrorKind> {
        let slice = self.as_mut_slice();
        Self::effective_address(offset, index, slice.len()).map(|ea| unsafe {
            slice.as_mut_ptr().add(ea).write(val);
        })
    }

    #[inline]
    pub fn read_u16(&self, offset: u32, index: u32) -> Result<u16, WasmRuntimeErrorKind> {
        let slice = self.as_slice();
        Self::effective_address(offset, index, slice.len() - 1)
            .map(|ea| unsafe { WasmEndian::read_u16(slice, ea) })
    }

    #[inline]
    pub fn write_u16(&self, offset: u32, index: u32, val: u16) -> Result<(), WasmRuntimeErrorKind> {
        let slice = self.as_mut_slice();
        Self::effective_address(offset, index, slice.len() - 1).map(|ea| unsafe {
            WasmEndian::write_u16(slice, ea, val);
        })
    }

    #[inline]
    pub fn read_u32(&self, offset: u32, index: u32) -> Result<u32, WasmRuntimeErrorKind> {
        let slice = self.as_slice();
        Self::effective_address(offset, index, slice.len() - 3)
            .map(|ea| unsafe { WasmEndian::read_u32(slice, ea) })
    }

    #[inline]
    pub fn write_u32(&self, offset: u32, index: u32, val: u32) -> Result<(), WasmRuntimeErrorKind> {
        let slice = self.as_mut_slice();
        Self::effective_address(offset, index, slice.len() - 3).map(|ea| unsafe {
            WasmEndian::write_u32(slice, ea, val);
        })
    }

    #[inline]
    pub fn read_u64(&self, offset: u32, index: u32) -> Result<u64, WasmRuntimeErrorKind> {
        let slice = self.as_slice();
        Self::effective_address(offset, index, slice.len() - 7)
            .map(|ea| unsafe { WasmEndian::read_u64(slice, ea) })
    }

    #[inline]
    pub fn write_u64(&self, offset: u32, index: u32, val: u64) -> Result<(), WasmRuntimeErrorKind> {
        let slice = self.as_mut_slice();
        Self::effective_address(offset, index, slice.len() - 7).map(|ea| unsafe {
            WasmEndian::write_u64(slice, ea, val);
        })
    }
}

/// WebAssembly table object
pub struct WasmTable {
    limit: WasmLimit,
    table: Vec<usize>,
}

impl WasmTable {
    #[inline]
    fn from_stream(stream: &mut Leb128Stream) -> Result<Self, WasmDecodeErrorKind> {
        match stream.read_unsigned() {
            Ok(0x70) => (),
            Err(err) => return Err(err),
            _ => return Err(WasmDecodeErrorKind::UnexpectedToken),
        };
        WasmLimit::from_stream(stream).map(|limit| {
            let size = limit.min as usize;
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
    index: usize,
    type_index: usize,
    func_type: WasmType,
    origin: WasmFunctionOrigin,
    code_block: Option<WasmCodeBlock>,
    dlink: Option<WasmDynFunc>,
}

impl WasmFunction {
    #[inline]
    fn from_import(
        index: usize,
        type_index: usize,
        func_type: WasmType,
        dlink: WasmDynFunc,
    ) -> Self {
        Self {
            index,
            type_index,
            func_type,
            origin: WasmFunctionOrigin::Import(index),
            code_block: None,
            dlink: Some(dlink),
        }
    }

    #[inline]
    fn internal(index: usize, type_index: usize, func_type: WasmType) -> Self {
        Self {
            index,
            type_index,
            func_type,
            origin: WasmFunctionOrigin::Internal,
            code_block: None,
            dlink: None,
        }
    }

    #[inline]
    pub const fn index(&self) -> usize {
        self.index
    }

    #[inline]
    pub const fn type_index(&self) -> usize {
        self.type_index
    }

    #[inline]
    pub const fn param_types(&self) -> &[WasmValType] {
        &self.func_type.param_types
    }

    #[inline]
    pub const fn result_types(&self) -> &[WasmValType] {
        &self.func_type.result_types
    }

    #[inline]
    pub const fn origin(&self) -> WasmFunctionOrigin {
        self.origin
    }

    #[inline]
    pub const fn code_block(&self) -> Option<&WasmCodeBlock> {
        self.code_block.as_ref()
    }

    #[inline]
    pub fn dlink(&self) -> Option<WasmDynFunc> {
        self.dlink
    }
}

#[derive(Debug, Copy, Clone)]
pub enum WasmFunctionOrigin {
    Internal,
    Export(usize),
    Import(usize),
}

/// A type that holds the signature of a function that combines a list of argument types with a list of return types.
///
/// It appears as the first section (`0x01`) in the WebAssembly binary.
#[derive(Debug, Clone)]
pub struct WasmType {
    param_types: Box<[WasmValType]>,
    result_types: Box<[WasmValType]>,
}

impl WasmType {
    fn from_stream(stream: &mut Leb128Stream) -> Result<Self, WasmDecodeErrorKind> {
        match stream.read_unsigned() {
            Ok(0x60) => (),
            Err(err) => return Err(err),
            _ => return Err(WasmDecodeErrorKind::UnexpectedToken),
        };
        let n_params = stream.read_unsigned()? as usize;
        let mut params = Vec::with_capacity(n_params);
        for _ in 0..n_params {
            stream
                .read_unsigned()
                .and_then(|v| WasmValType::from_u64(v))
                .map(|v| params.push(v))?;
        }
        let n_result = stream.read_unsigned()? as usize;
        let mut result = Vec::with_capacity(n_result);
        for _ in 0..n_result {
            stream
                .read_unsigned()
                .and_then(|v| WasmValType::from_u64(v))
                .map(|v| result.push(v))?;
        }
        Ok(Self {
            param_types: params.into_boxed_slice(),
            result_types: result.into_boxed_slice(),
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
            for param in self.param_types.into_iter() {
                write!(f, " {}", param)?;
            }
            write!(f, ")")?;
        }
        if self.result_types.len() > 0 {
            write!(f, " (result")?;
            for result in self.result_types.into_iter() {
                write!(f, " {}", result)?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WasmTypeIndex(pub usize);

/// WebAssembly import object
///
/// It appears as the second section (`0x02`) in the WebAssembly binary.
#[derive(Debug, Clone)]
pub struct WasmImport {
    mod_name: String,
    name: String,
    index: WasmImportIndex,
    func_ref: usize,
}

impl WasmImport {
    #[inline]
    fn from_stream(stream: &mut Leb128Stream) -> Result<Self, WasmDecodeErrorKind> {
        let mod_name = stream.get_string()?.to_string();
        let name = stream.get_string()?.to_string();
        let index = WasmImportIndex::from_stream(stream)?;

        Ok(Self {
            mod_name,
            name,
            index,
            func_ref: 0,
        })
    }

    #[inline]
    pub fn mod_name(&self) -> &str {
        self.mod_name.as_ref()
    }

    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    #[inline]
    pub const fn index(&self) -> WasmImportIndex {
        self.index
    }
}

#[derive(Debug, Copy, Clone)]
pub enum WasmImportIndex {
    Type(usize),
    // Table(usize),
    Memory(WasmLimit),
    // Global(usize),
}

impl WasmImportIndex {
    #[inline]
    fn from_stream(mut stream: &mut Leb128Stream) -> Result<Self, WasmDecodeErrorKind> {
        stream.read_unsigned().and_then(|v| match v {
            0 => stream.read_unsigned().map(|v| Self::Type(v as usize)),
            // 1 => stream.read_unsigned().map(|v| Self::Table(v as usize)),
            2 => WasmLimit::from_stream(&mut stream).map(|v| Self::Memory(v)),
            // 3 => stream.read_unsigned().map(|v| Self::Global(v as usize)),
            _ => Err(WasmDecodeErrorKind::UnexpectedToken),
        })
    }
}

/// WebAssembly export object
pub struct WasmExport {
    name: String,
    index: WasmExportIndex,
}

impl WasmExport {
    #[inline]
    fn from_stream(stream: &mut Leb128Stream) -> Result<Self, WasmDecodeErrorKind> {
        let name = stream.get_string()?.to_string();
        let index = WasmExportIndex::from_stream(stream)?;
        Ok(Self { name, index })
    }

    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    #[inline]
    pub const fn index(&self) -> WasmExportIndex {
        self.index
    }
}

#[derive(Debug, Copy, Clone)]
pub enum WasmExportIndex {
    Function(usize),
    Table(usize),
    Memory(usize),
    Global(usize),
}

impl WasmExportIndex {
    #[inline]
    fn from_stream(stream: &mut Leb128Stream) -> Result<Self, WasmDecodeErrorKind> {
        stream.read_unsigned().and_then(|v| match v {
            0 => stream.read_unsigned().map(|v| Self::Function(v as usize)),
            1 => stream.read_unsigned().map(|v| Self::Table(v as usize)),
            2 => stream.read_unsigned().map(|v| Self::Memory(v as usize)),
            3 => stream.read_unsigned().map(|v| Self::Global(v as usize)),
            _ => Err(WasmDecodeErrorKind::UnexpectedToken),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum WasmDecodeErrorKind {
    /// Not an executable file.
    BadExecutable,
    /// We've reached the end of an unexpected stream.
    UnexpectedEof,
    /// Unexpected token detected during decoding.
    UnexpectedToken,
    /// Detected a bytecode that cannot be decoded.
    InvalidBytecode(u8),
    /// Unsupported opcode
    UnsupportedOpCode(WasmOpcode),
    /// Unsupported global data type
    UnsupportedGlobalType,
    /// Invalid parameter was specified.
    InvalidParameter,
    /// Invalid stack level.
    InvalidStackLevel,
    /// Specified a non-existent type.
    InvalidType,
    /// Invalid global variable specified.
    InvalidGlobal,
    /// Invalid local variable specified.
    InvalidLocal,
    /// Value stack is out of range
    OutOfStack,
    /// Branching targets are out of nest range
    OutOfBranch,
    /// Accessing non-existent memory
    OutOfMemory,
    /// The type of the value stack does not match.
    TypeMismatch,
    /// Termination of invalid blocks
    BlockMismatch,
    /// The `else` block and the `if` block do not match.
    ElseWithoutIf,
    /// Imported function does not exist.
    NoMethod(String),
    /// Imported module does not exist.
    NoModule(String),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WasmRuntimeErrorKind {
    /// Exit the application (not an error)
    Exit,
    // InternalInconsistency,
    InvalidParameter,
    NotSupprted,
    Unreachable,
    OutOfBounds,
    OutOfMemory,
    NoMethod,
    DivideByZero,
    TypeMismatch,
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
pub union WasmUnsafeValue {
    usize: usize,
    i32: i32,
    u32: u32,
    i64: i64,
    u64: u64,
    f32: f32,
    f64: f64,
}

impl WasmUnsafeValue {
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
    pub unsafe fn write_bool(&mut self, val: bool) {
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
    pub unsafe fn write_i32(&mut self, val: i32) {
        unsafe {
            self.copy_from_i32(&Self::from(val));
        }
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
    pub unsafe fn write_i64(&mut self, val: i64) {
        *self = Self::from(val);
    }

    #[inline]
    pub unsafe fn write_f32(&mut self, val: f32) {
        self.f32 = val;
    }

    #[inline]
    pub unsafe fn write_f64(&mut self, val: f64) {
        self.f64 = val;
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
        unsafe {
            self.copy_from_i32(&Self::from(f(val)));
        }
    }

    /// Retrieves the value held by the instance as a value of type `u32` and re-stores the value processed by the closure.
    #[inline]
    pub unsafe fn map_u32<F>(&mut self, f: F)
    where
        F: FnOnce(u32) -> u32,
    {
        let val = unsafe { self.u32 };
        unsafe { self.copy_from_i32(&Self::from(f(val))) };
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
        unsafe { self.copy_from_i32(&Self::from(f(val))) };
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
    pub unsafe fn copy_from_i32(&mut self, other: &Self) {
        if Self::_is_32bit_env() {
            self.u32 = unsafe { other.u32 };
        } else {
            *self = *other;
        }
    }
}

impl Default for WasmUnsafeValue {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

impl From<bool> for WasmUnsafeValue {
    #[inline]
    fn from(v: bool) -> Self {
        Self::from_bool(v)
    }
}

impl From<u32> for WasmUnsafeValue {
    #[inline]
    fn from(v: u32) -> Self {
        Self::from_u32(v)
    }
}

impl From<i32> for WasmUnsafeValue {
    #[inline]
    fn from(v: i32) -> Self {
        Self::from_i32(v)
    }
}

impl From<u64> for WasmUnsafeValue {
    #[inline]
    fn from(v: u64) -> Self {
        Self::from_u64(v)
    }
}

impl From<i64> for WasmUnsafeValue {
    #[inline]
    fn from(v: i64) -> Self {
        Self::from_i64(v)
    }
}

impl From<f32> for WasmUnsafeValue {
    #[inline]
    fn from(v: f32) -> Self {
        Self::from_f32(v)
    }
}

impl From<f64> for WasmUnsafeValue {
    #[inline]
    fn from(v: f64) -> Self {
        Self::from_f64(v)
    }
}

impl From<WasmValue> for WasmUnsafeValue {
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

unsafe impl UnsafeInto<i32> for WasmUnsafeValue {
    #[inline]
    unsafe fn unsafe_into(self) -> i32 {
        unsafe { self.get_i32() }
    }
}

unsafe impl UnsafeInto<i64> for WasmUnsafeValue {
    #[inline]
    unsafe fn unsafe_into(self) -> i64 {
        unsafe { self.get_i64() }
    }
}

unsafe impl UnsafeInto<f32> for WasmUnsafeValue {
    #[inline]
    unsafe fn unsafe_into(self) -> f32 {
        unsafe { self.get_f32() }
    }
}

unsafe impl UnsafeInto<f64> for WasmUnsafeValue {
    #[inline]
    unsafe fn unsafe_into(self) -> f64 {
        unsafe { self.get_f64() }
    }
}

/// WebAssembly global variable
pub struct WasmGlobal {
    data: AtomicU32,
    val_type: WasmValType,
    is_mutable: bool,
}

impl WasmGlobal {
    #[inline]
    pub fn new(val: WasmValue, is_mutable: bool) -> Result<Self, WasmDecodeErrorKind> {
        let val_type = val.val_type();
        let val = val
            .get_u32()
            .map_err(|_| WasmDecodeErrorKind::UnsupportedGlobalType)?;
        Ok(Self {
            data: AtomicU32::new(val),
            val_type,
            is_mutable,
        })
    }

    #[inline]
    pub fn value(&self) -> WasmValue {
        self.data.load(Ordering::Relaxed).into()
    }

    #[inline]
    pub fn set_value(&self, val: WasmUnsafeValue) {
        self.data.store(unsafe { val.get_u32() }, Ordering::SeqCst);
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
    //locals: Vec<>,
    globals: Vec<(usize, String)>,
}

impl WasmName {
    pub const SECTION_NAME: &'static str = "name";

    fn from_stream(stream: &mut Leb128Stream) -> Result<Self, WasmDecodeErrorKind> {
        let mut module = None;
        let mut functions = Vec::new();
        let mut globals = Vec::new();

        while !stream.is_eof() {
            let name_id = stream.read_byte()?;
            let blob = stream.read_bytes()?;
            let Some(name_id) = FromPrimitive::from_u8(name_id) else {
                        continue
                    };
            let mut stream = Leb128Stream::from_slice(blob);
            match name_id {
                WasmNameSubsectionType::Module => {
                    module = stream.get_string().map(|s| s.to_string()).ok()
                }
                WasmNameSubsectionType::Function => {
                    let length = stream.read_unsigned()? as usize;
                    for _ in 0..length {
                        let idx = stream.read_unsigned()? as usize;
                        let s = stream.get_string().map(|s| s.to_string())?;
                        functions.push((idx, s));
                    }
                }
                WasmNameSubsectionType::Global => {
                    let length = stream.read_unsigned()? as usize;
                    for _ in 0..length {
                        let idx = stream.read_unsigned()? as usize;
                        let s = stream.get_string().map(|s| s.to_string())?;
                        globals.push((idx, s));
                    }
                }
                _ => {
                    // TODO:
                }
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

    pub fn func_by_index(&self, idx: usize) -> Option<&str> {
        let functions = self.functions();
        match functions.binary_search_by_key(&idx, |(k, _v)| *k) {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, FromPrimitive)]
enum WasmNameSubsectionType {
    Module = 0,
    Function,
    Local,
    Labels,
    Type,
    Table,
    Memory,
    Global,
    ElemSegment,
    DataSegment,
}

/// WebAssembly code block
pub struct WasmCodeBlock {
    func_index: usize,
    file_position: usize,
    local_types: Box<[WasmValType]>,
    max_stack: usize,
    flags: WasmBlockFlag,
    int_codes: Box<[WasmImc]>,
}

bitflags! {
    pub struct WasmBlockFlag: usize {
        const LEAF_FUNCTION     = 0b0000_0000_0000_0001;
    }
}

impl WasmCodeBlock {
    #[inline]
    pub const fn func_index(&self) -> usize {
        self.func_index
    }

    #[inline]
    pub const fn file_position(&self) -> usize {
        self.file_position
    }

    #[inline]
    pub const fn local_types(&self) -> &[WasmValType] {
        &self.local_types
    }

    /// Returns the maximum size of the value stack.
    #[inline]
    pub const fn max_value_stack(&self) -> usize {
        self.max_stack
    }

    /// Returns whether or not this function block does not call any other functions.
    #[inline]
    pub const fn is_leaf(&self) -> bool {
        self.flags.contains(WasmBlockFlag::LEAF_FUNCTION)
    }

    /// Returns an intermediate code block.
    #[inline]
    pub const fn intermediate_codes(&self) -> &[WasmImc] {
        &self.int_codes
    }

    /// Analyzes the WebAssembly bytecode stream to generate intermediate code blocks.
    pub fn generate(
        func_index: usize,
        file_position: usize,
        stream: &mut Leb128Stream,
        param_types: &[WasmValType],
        result_types: &[WasmValType],
        module: &WasmModule,
    ) -> Result<Self, WasmDecodeErrorKind> {
        let n_local_types = stream.read_unsigned()? as usize;
        let mut local_types = Vec::with_capacity(n_local_types);
        for _ in 0..n_local_types {
            let repeat = stream.read_unsigned()?;
            let val = stream
                .read_unsigned()
                .and_then(|v| WasmValType::from_u64(v))?;
            for _ in 0..repeat {
                local_types.push(val);
            }
        }
        let mut local_var_types = Vec::with_capacity(param_types.len() + local_types.len());
        for param_type in param_types {
            local_var_types.push(*param_type);
        }
        local_var_types.extend_from_slice(local_types.as_slice());

        let mut blocks = Vec::new();
        let mut block_stack = Vec::new();
        let mut value_stack = Vec::new();
        let mut max_stack = 0;
        let mut max_block_level = 0;
        let mut flags = WasmBlockFlag::LEAF_FUNCTION;

        let mut int_codes: Vec<WasmImc> = Vec::new();

        loop {
            max_stack = usize::max(max_stack, value_stack.len());
            max_block_level = usize::max(max_block_level, block_stack.len());
            let position = stream.position();
            let opcode = stream.read_opcode()?;

            // match opcode.proposal_type() {
            //     WasmProposalType::Mvp => {}
            //     WasmProposalType::MvpI64 => {}
            //     WasmProposalType::SignExtend => {}
            //     WasmProposalType::BulkMemoryOperations => {}
            //     #[cfg(not(feature = "float"))]
            //     WasmProposalType::MvpF32 | WasmProposalType::MvpF64 => {}
            //     _ => return Err(WasmDecodeErrorKind::UnsupportedOpCode(opcode)),
            // }

            match opcode {
                WasmOpcode::Single(v) => match v {
                    WasmSingleOpcode::Unreachable => {
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::Unreachable,
                            value_stack.len().into(),
                        ));
                    }

                    WasmSingleOpcode::Nop => (),

                    WasmSingleOpcode::Block => {
                        let target = blocks.len();
                        let block_type = stream
                            .read_signed()
                            .and_then(|v| WasmBlockType::from_i64(v))?;
                        let block = RefCell::new(WasmBlockContext {
                            inst_type: BlockInstType::Block,
                            block_type,
                            stack_level: value_stack.len(),
                            start_position: 0,
                            end_position: 0,
                            else_position: 0,
                        });
                        block_stack.push(target);
                        blocks.push(block);
                        if block_type == WasmBlockType::Empty {
                            int_codes.push(WasmImc::new(
                                position,
                                opcode,
                                WasmIntMnemonic::Block(target),
                                value_stack.len().into(),
                            ));
                        } else {
                            // TODO:
                            int_codes.push(WasmImc::new(
                                position,
                                opcode,
                                WasmIntMnemonic::Undefined,
                                value_stack.len().into(),
                            ));
                        }
                    }
                    WasmSingleOpcode::Loop => {
                        let target = blocks.len();
                        let block_type = stream
                            .read_signed()
                            .and_then(|v| WasmBlockType::from_i64(v))?;
                        let block = RefCell::new(WasmBlockContext {
                            inst_type: BlockInstType::Loop,
                            block_type,
                            stack_level: value_stack.len(),
                            start_position: 0,
                            end_position: 0,
                            else_position: 0,
                        });
                        block_stack.push(target);
                        blocks.push(block);
                        if block_type == WasmBlockType::Empty {
                            int_codes.push(WasmImc::new(
                                position,
                                opcode,
                                WasmIntMnemonic::Block(target),
                                value_stack.len().into(),
                            ));
                        } else {
                            // TODO:
                            int_codes.push(WasmImc::new(
                                position,
                                opcode,
                                WasmIntMnemonic::Undefined,
                                value_stack.len().into(),
                            ));
                        }
                    }
                    WasmSingleOpcode::If => {
                        let cc = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if cc != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        let block_type = stream
                            .read_signed()
                            .and_then(|v| WasmBlockType::from_i64(v))?;
                        let block = RefCell::new(WasmBlockContext {
                            inst_type: BlockInstType::If,
                            block_type,
                            stack_level: value_stack.len(),
                            start_position: 0,
                            end_position: 0,
                            else_position: 0,
                        });
                        block_stack.push(blocks.len());
                        blocks.push(block);
                        // TODO: if else block
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::Undefined,
                            value_stack.len().into(),
                        ));
                    }
                    WasmSingleOpcode::Else => {
                        let block_ref = block_stack
                            .last()
                            .ok_or(WasmDecodeErrorKind::ElseWithoutIf)?;
                        let block = blocks.get(*block_ref).unwrap().borrow();
                        if block.inst_type != BlockInstType::If {
                            return Err(WasmDecodeErrorKind::ElseWithoutIf);
                        }
                        let n_drops = value_stack.len() - block.stack_level;
                        for _ in 0..n_drops {
                            value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        }
                        // TODO: if else block
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::Undefined,
                            value_stack.len().into(),
                        ));
                    }
                    WasmSingleOpcode::End => {
                        if block_stack.len() > 0 {
                            let block_ref = block_stack
                                .pop()
                                .ok_or(WasmDecodeErrorKind::BlockMismatch)?;
                            let block = blocks.get(block_ref).unwrap().borrow();
                            let n_drops = value_stack.len() - block.stack_level;
                            for _ in 0..n_drops {
                                value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                            }
                            block.block_type.into_type().map(|v| {
                                value_stack.push(v);
                            });
                            int_codes.push(WasmImc::new(
                                position,
                                opcode,
                                WasmIntMnemonic::End(block_ref),
                                value_stack.len().into(),
                            ));
                            // TODO: type check
                        } else {
                            int_codes.push(WasmImc::new(
                                position,
                                opcode,
                                WasmIntMnemonic::Return,
                                StackLevel(value_stack.len() - 1),
                            ));
                            break;
                        }
                    }

                    WasmSingleOpcode::Br => {
                        let br = stream.read_unsigned()? as usize;
                        let target = block_stack
                            .get(block_stack.len() - br - 1)
                            .ok_or(WasmDecodeErrorKind::OutOfBranch)?;
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::Br(*target),
                            value_stack.len().into(),
                        ));
                    }
                    WasmSingleOpcode::BrIf => {
                        let br = stream.read_unsigned()? as usize;
                        let target = block_stack
                            .get(block_stack.len() - br - 1)
                            .ok_or(WasmDecodeErrorKind::OutOfBranch)?;
                        let cc = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if cc != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::BrIf(*target),
                            value_stack.len().into(),
                        ));
                    }
                    WasmSingleOpcode::BrTable => {
                        let table_len = 1 + stream.read_unsigned()? as usize;
                        let mut table = Vec::with_capacity(table_len);
                        for _ in 0..table_len {
                            let br = stream.read_unsigned()? as usize;
                            let target = block_stack
                                .get(block_stack.len() - br - 1)
                                .ok_or(WasmDecodeErrorKind::OutOfBranch)?;
                            table.push(*target);
                        }
                        let cc = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if cc != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::BrTable(table.into_boxed_slice()),
                            value_stack.len().into(),
                        ));
                    }

                    WasmSingleOpcode::Return => {
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::Return,
                            StackLevel(value_stack.len() - 1),
                        ));
                        // TODO: type check
                    }

                    WasmSingleOpcode::Call => {
                        flags.remove(WasmBlockFlag::LEAF_FUNCTION);
                        let func_index = stream.read_unsigned()? as usize;
                        let function = module
                            .functions
                            .get(func_index)
                            .ok_or(WasmDecodeErrorKind::InvalidParameter)?;
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::Call(func_index),
                            value_stack.len().into(),
                        ));
                        // TODO: type check
                        for _param in function.param_types() {
                            value_stack.pop();
                        }
                        for result in function.result_types() {
                            value_stack.push(result.clone());
                        }
                    }
                    WasmSingleOpcode::CallIndirect => {
                        flags.remove(WasmBlockFlag::LEAF_FUNCTION);
                        let type_index = stream.read_unsigned()? as usize;
                        let _reserved = stream.read_unsigned()? as usize;
                        let func_type = module
                            .type_by_ref(type_index)
                            .ok_or(WasmDecodeErrorKind::InvalidParameter)?;
                        let index = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if index != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::CallIndirect(type_index),
                            value_stack.len().into(),
                        ));
                        // TODO: type check
                        for _param in func_type.param_types() {
                            value_stack.pop();
                        }
                        for result in func_type.result_types() {
                            value_stack.push(result.clone());
                        }
                    }

                    // WasmOpcode::ReturnCall
                    // WasmOpcode::ReturnCallIndirect
                    WasmSingleOpcode::Drop => {
                        value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    }

                    WasmSingleOpcode::Select => {
                        let cc = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || cc != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::Select,
                            value_stack.len().into(),
                        ));
                        value_stack.push(a);
                    }

                    WasmSingleOpcode::LocalGet => {
                        let local_ref = stream.read_unsigned()? as usize;
                        let val = *local_var_types
                            .get(local_ref)
                            .ok_or(WasmDecodeErrorKind::InvalidLocal)?;
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            match val {
                                WasmValType::I32 | WasmValType::F32 => {
                                    WasmIntMnemonic::LocalGet32(LocalVarIndex(local_ref))
                                }
                                WasmValType::I64 | WasmValType::F64 => {
                                    WasmIntMnemonic::LocalGet(LocalVarIndex(local_ref))
                                }
                            },
                            value_stack.len().into(),
                        ));
                        value_stack.push(val);
                    }
                    WasmSingleOpcode::LocalSet => {
                        let local_ref = stream.read_unsigned()? as usize;
                        let val = *local_var_types
                            .get(local_ref)
                            .ok_or(WasmDecodeErrorKind::InvalidLocal)?;
                        let stack = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if stack != val {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            match val {
                                WasmValType::I32 | WasmValType::F32 => {
                                    WasmIntMnemonic::LocalSet32(LocalVarIndex(local_ref))
                                }
                                WasmValType::I64 | WasmValType::F64 => {
                                    WasmIntMnemonic::LocalSet(LocalVarIndex(local_ref))
                                }
                            },
                            value_stack.len().into(),
                        ));
                    }
                    WasmSingleOpcode::LocalTee => {
                        let local_ref = stream.read_unsigned()? as usize;
                        let val = *local_var_types
                            .get(local_ref)
                            .ok_or(WasmDecodeErrorKind::InvalidLocal)?;
                        let stack = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if stack != val {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            match val {
                                WasmValType::I32 | WasmValType::F32 => {
                                    WasmIntMnemonic::LocalTee32(LocalVarIndex(local_ref))
                                }
                                WasmValType::I64 | WasmValType::F64 => {
                                    WasmIntMnemonic::LocalTee(LocalVarIndex(local_ref))
                                }
                            },
                            StackLevel(value_stack.len() - 1),
                        ));
                    }

                    WasmSingleOpcode::GlobalGet => {
                        let global_ref = stream.read_unsigned()? as usize;
                        let val_type = module
                            .globals()
                            .get(global_ref)
                            .map(|v| v.val_type())
                            .ok_or(WasmDecodeErrorKind::InvalidGlobal)?;
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::GlobalGet(global_ref),
                            value_stack.len().into(),
                        ));
                        value_stack.push(val_type);
                    }
                    WasmSingleOpcode::GlobalSet => {
                        let global_ref = stream.read_unsigned()? as usize;
                        let global = module
                            .globals()
                            .get(global_ref)
                            .ok_or(WasmDecodeErrorKind::InvalidGlobal)?;
                        let val_type = global.val_type();
                        let is_mutable = global.is_mutable();
                        if !is_mutable {
                            return Err(WasmDecodeErrorKind::InvalidGlobal);
                        }
                        let stack = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if stack != val_type {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::GlobalSet(global_ref),
                            value_stack.len().into(),
                        ));
                    }

                    WasmSingleOpcode::I32Load => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Load(arg.offset),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I32Load8S => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Load8S(arg.offset),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I32Load8U => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Load8U(arg.offset),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I32Load16S => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Load16S(arg.offset),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I32Load16U => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Load16U(arg.offset),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }

                    WasmSingleOpcode::I64Load => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Load(arg.offset),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I64);
                    }
                    WasmSingleOpcode::I64Load8S => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Load8S(arg.offset),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I64);
                    }
                    WasmSingleOpcode::I64Load8U => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Load8U(arg.offset),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I64);
                    }
                    WasmSingleOpcode::I64Load16S => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Load16S(arg.offset),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I64);
                    }
                    WasmSingleOpcode::I64Load16U => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Load16U(arg.offset),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I64);
                    }
                    WasmSingleOpcode::I64Load32S => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Load32S(arg.offset),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I64);
                    }
                    WasmSingleOpcode::I64Load32U => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Load32U(arg.offset),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I64);
                    }

                    WasmSingleOpcode::I32Store => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let d = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let i = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if i != d && i != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Store(arg.offset),
                            value_stack.len().into(),
                        ));
                    }
                    WasmSingleOpcode::I32Store8 => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let d = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let i = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if i != d && i != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Store8(arg.offset),
                            value_stack.len().into(),
                        ));
                    }
                    WasmSingleOpcode::I32Store16 => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let d = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let i = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if i != d && i != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Store16(arg.offset),
                            value_stack.len().into(),
                        ));
                    }

                    WasmSingleOpcode::I64Store => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let d = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let i = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if i != WasmValType::I32 && d != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Store(arg.offset),
                            value_stack.len().into(),
                        ));
                    }
                    WasmSingleOpcode::I64Store8 => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let d = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let i = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if i != WasmValType::I32 && d != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Store8(arg.offset),
                            value_stack.len().into(),
                        ));
                    }
                    WasmSingleOpcode::I64Store16 => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let d = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let i = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if i != WasmValType::I32 && d != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Store16(arg.offset),
                            value_stack.len().into(),
                        ));
                    }
                    WasmSingleOpcode::I64Store32 => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let d = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let i = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if i != WasmValType::I32 && d != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Store32(arg.offset),
                            value_stack.len().into(),
                        ));
                    }

                    #[cfg(feature = "float")]
                    WasmSingleOpcode::F32Load => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::F32Load(arg.offset),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::F32);
                    }
                    #[cfg(feature = "float")]
                    WasmSingleOpcode::F32Store => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let d = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let i = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if i != WasmValType::I32 && d != WasmValType::F32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::F32Store(arg.offset),
                            value_stack.len().into(),
                        ));
                    }

                    #[cfg(feature = "float64")]
                    WasmSingleOpcode::F64Load => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::F64Load(arg.offset),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::F64);
                    }
                    #[cfg(feature = "float64")]
                    WasmSingleOpcode::F64Store => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let arg = stream.read_memarg()?;
                        let d = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let i = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if i != WasmValType::I32 && d != WasmValType::F64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::F64Store(arg.offset),
                            value_stack.len().into(),
                        ));
                    }

                    WasmSingleOpcode::MemorySize => {
                        let index = stream.read_unsigned()? as usize;
                        if index >= module.memories.len() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::MemorySize,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }

                    WasmSingleOpcode::MemoryGrow => {
                        let index = stream.read_unsigned()? as usize;
                        if index >= module.memories.len() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::MemoryGrow,
                            StackLevel(value_stack.len() - 1),
                        ));
                        let a = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }

                    WasmSingleOpcode::I32Const => {
                        let val = stream.read_signed()?;
                        let val: i32 = val
                            .try_into()
                            .map_err(|_| WasmDecodeErrorKind::InvalidParameter)?;
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Const(val),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I64Const => {
                        let val = stream.read_signed()?;
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Const(val),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I64);
                    }
                    #[cfg(feature = "float")]
                    WasmSingleOpcode::F32Const => {
                        let val = stream.read_f32()?;
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::F32Const(val),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::F32);
                    }
                    #[cfg(feature = "float64")]
                    WasmSingleOpcode::F64Const => {
                        let val = stream.read_f64()?;
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::F64Const(val),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::F64);
                    }

                    // unary operator [i32] -> [i32]
                    WasmSingleOpcode::I32Eqz => {
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Eqz,
                            StackLevel(value_stack.len() - 1),
                        ));
                        let a = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }
                    WasmSingleOpcode::I32Clz => {
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Clz,
                            StackLevel(value_stack.len() - 1),
                        ));
                        let a = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }
                    WasmSingleOpcode::I32Ctz => {
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Ctz,
                            StackLevel(value_stack.len() - 1),
                        ));
                        let a = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }
                    WasmSingleOpcode::I32Popcnt => {
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Popcnt,
                            StackLevel(value_stack.len() - 1),
                        ));
                        let a = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }
                    WasmSingleOpcode::I32Extend8S => {
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Extend8S,
                            StackLevel(value_stack.len() - 1),
                        ));
                        let a = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }
                    WasmSingleOpcode::I32Extend16S => {
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Extend16S,
                            StackLevel(value_stack.len() - 1),
                        ));
                        let a = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }

                    // binary operator [i32, i32] -> [i32]
                    WasmSingleOpcode::I32Eq => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Eq,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I32Ne => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Ne,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I32LtS => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32LtS,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I32LtU => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32LtU,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I32GtS => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32GtS,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I32GtU => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32GtU,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I32LeS => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32LeS,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I32LeU => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32LeU,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I32GeS => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32GeS,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I32GeU => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32GeU,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I32Add => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Add,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I32Sub => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Sub,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I32Mul => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Mul,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I32DivS => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32DivS,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I32DivU => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32DivU,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I32RemS => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32RemS,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I32RemU => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32RemU,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I32And => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32And,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I32Or => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Or,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I32Xor => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Xor,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I32Shl => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Shl,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I32ShrS => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32ShrS,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I32ShrU => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32ShrU,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I32Rotl => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Rotl,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I32Rotr => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32Rotr,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }

                    // binary operator [i64, i64] -> [i32]
                    WasmSingleOpcode::I64Eq => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Eq,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I64Ne => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Ne,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I64LtS => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64LtS,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I64LtU => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64LtU,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I64GtS => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64GtS,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I64GtU => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64GtU,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I64LeS => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64LeS,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I64LeU => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64LeU,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I64GeS => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64GeS,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I64GeU => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64GeU,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }

                    // unary operator [i64] -> [i64]
                    WasmSingleOpcode::I64Clz => {
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Clz,
                            StackLevel(value_stack.len() - 1),
                        ));
                        let a = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }
                    WasmSingleOpcode::I64Ctz => {
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Ctz,
                            StackLevel(value_stack.len() - 1),
                        ));
                        let a = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }
                    WasmSingleOpcode::I64Popcnt => {
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Popcnt,
                            StackLevel(value_stack.len() - 1),
                        ));
                        let a = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }
                    WasmSingleOpcode::I64Extend8S => {
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Extend8S,
                            StackLevel(value_stack.len() - 1),
                        ));
                        let a = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }
                    WasmSingleOpcode::I64Extend16S => {
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Extend16S,
                            StackLevel(value_stack.len() - 1),
                        ));
                        let a = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }
                    WasmSingleOpcode::I64Extend32S => {
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Extend32S,
                            StackLevel(value_stack.len() - 1),
                        ));
                        let a = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }

                    // binary operator [i64, i64] -> [i64]
                    WasmSingleOpcode::I64Add => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Add,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I64Sub => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Sub,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I64Mul => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Mul,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I64DivS => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64DivS,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I64DivU => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64DivU,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I64RemS => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64RemS,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I64RemU => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64RemU,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I64And => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64And,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I64Or => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Or,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I64Xor => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Xor,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I64Shl => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Shl,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I64ShrS => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64ShrS,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I64ShrU => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64ShrU,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I64Rotl => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Rotl,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                    WasmSingleOpcode::I64Rotr => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Rotr,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }

                    // [i64] -> [i32]
                    WasmSingleOpcode::I64Eqz => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64Eqz,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I32WrapI64 => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32WrapI64,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }

                    // [i32] -> [i64]
                    WasmSingleOpcode::I64ExtendI32S => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64ExtendI32S,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I64);
                    }
                    WasmSingleOpcode::I64ExtendI32U => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64ExtendI32U,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I64);
                    }

                    // [f32] -> [i32]
                    #[cfg(feature = "float")]
                    WasmSingleOpcode::I32TruncF32S | WasmSingleOpcode::I32TruncF32U => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::F32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        value_stack.push(WasmValType::I32);
                    }

                    // [f32, f32] -> [i32]
                    #[cfg(feature = "float")]
                    WasmSingleOpcode::F32Eq
                    | WasmSingleOpcode::F32Ne
                    | WasmSingleOpcode::F32Lt
                    | WasmSingleOpcode::F32Gt
                    | WasmSingleOpcode::F32Le
                    | WasmSingleOpcode::F32Ge => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::F32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        value_stack.push(WasmValType::I32);
                    }

                    // [f32] -> [f32]
                    #[cfg(feature = "float")]
                    WasmSingleOpcode::F32Abs
                    | WasmSingleOpcode::F32Neg
                    | WasmSingleOpcode::F32Ceil
                    | WasmSingleOpcode::F32Floor
                    | WasmSingleOpcode::F32Trunc
                    | WasmSingleOpcode::F32Nearest
                    | WasmSingleOpcode::F32Sqrt => {
                        let a = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }

                    // [f32, f32] -> [f32]
                    #[cfg(feature = "float")]
                    WasmSingleOpcode::F32Add
                    | WasmSingleOpcode::F32Sub
                    | WasmSingleOpcode::F32Mul
                    | WasmSingleOpcode::F32Div
                    | WasmSingleOpcode::F32Min
                    | WasmSingleOpcode::F32Max
                    | WasmSingleOpcode::F32Copysign => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::F32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }

                    // [f64] -> [i32]
                    #[cfg(feature = "float64")]
                    WasmSingleOpcode::I32TruncF64S | WasmSingleOpcode::I32TruncF64U => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::F64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        value_stack.push(WasmValType::I32);
                    }

                    // [f64] -> [i64]
                    #[cfg(feature = "float")]
                    WasmSingleOpcode::I64TruncF32S
                    | WasmSingleOpcode::I64TruncF32U
                    | WasmSingleOpcode::I64TruncF64S
                    | WasmSingleOpcode::I64TruncF64U => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::F64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        value_stack.push(WasmValType::I64);
                    }

                    // [f64, f64] -> [i32]
                    #[cfg(feature = "float64")]
                    WasmSingleOpcode::F64Eq
                    | WasmSingleOpcode::F64Ne
                    | WasmSingleOpcode::F64Lt
                    | WasmSingleOpcode::F64Gt
                    | WasmSingleOpcode::F64Le
                    | WasmSingleOpcode::F64Ge => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::F64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        value_stack.push(WasmValType::I32);
                    }

                    // [f64] -> [f64]
                    #[cfg(feature = "float64")]
                    WasmSingleOpcode::F64Abs
                    | WasmSingleOpcode::F64Neg
                    | WasmSingleOpcode::F64Ceil
                    | WasmSingleOpcode::F64Floor
                    | WasmSingleOpcode::F64Trunc
                    | WasmSingleOpcode::F64Nearest
                    | WasmSingleOpcode::F64Sqrt => {
                        let a = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::F64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }

                    // [f64, f64] -> [f64]
                    #[cfg(feature = "float64")]
                    WasmSingleOpcode::F64Add
                    | WasmSingleOpcode::F64Sub
                    | WasmSingleOpcode::F64Mul
                    | WasmSingleOpcode::F64Div
                    | WasmSingleOpcode::F64Min
                    | WasmSingleOpcode::F64Max
                    | WasmSingleOpcode::F64Copysign => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != b || a != WasmValType::F64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }

                    // [i32] -> [f32]
                    #[cfg(feature = "float")]
                    WasmSingleOpcode::F32ConvertI32S | WasmSingleOpcode::F32ConvertI32U => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        value_stack.push(WasmValType::F32);
                    }

                    // [i64] -> [f32]
                    #[cfg(feature = "float")]
                    WasmSingleOpcode::F32ConvertI64S | WasmSingleOpcode::F32ConvertI64U => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        value_stack.push(WasmValType::F32);
                    }

                    // [f64] -> [f32]
                    #[cfg(feature = "float64")]
                    WasmSingleOpcode::F32DemoteF64 => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::F64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        value_stack.push(WasmValType::F32);
                    }

                    // [i32] -> [f64]
                    #[cfg(feature = "float64")]
                    WasmSingleOpcode::F64ConvertI32S | WasmSingleOpcode::F64ConvertI32U => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        value_stack.push(WasmValType::F64);
                    }

                    // [i64] -> [f64]
                    #[cfg(feature = "float64")]
                    WasmSingleOpcode::F64ConvertI64S | WasmSingleOpcode::F64ConvertI64U => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        value_stack.push(WasmValType::F64);
                    }

                    // [f32] -> [f64]
                    #[cfg(feature = "float64")]
                    WasmSingleOpcode::F64PromoteF32 => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::F32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        value_stack.push(WasmValType::F64);
                    }

                    WasmSingleOpcode::I32ReinterpretF32 => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::F32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I32ReinterpretF32,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I64ReinterpretF64 => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::F64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::I64ReinterpretF64,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I64);
                    }
                    WasmSingleOpcode::F32ReinterpretI32 => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::F32ReinterpretI32,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::F32);
                    }
                    WasmSingleOpcode::F64ReinterpretI64 => {
                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I64 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::F64ReinterpretI64,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::F64);
                    }

                    WasmSingleOpcode::PrefixFC | WasmSingleOpcode::PrefixFD => unreachable!(),

                    _ => return Err(WasmDecodeErrorKind::UnsupportedOpCode(opcode.into())),
                },

                WasmOpcode::PrefixFC(v) => match v {
                    WasmOpcodeFC::MemoryCopy => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let memory_dst = stream.read_unsigned()? as usize;
                        if memory_dst >= module.memories.len() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let memory_src = stream.read_unsigned()? as usize;
                        if memory_src >= module.memories.len() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }

                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let c = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 || b != WasmValType::I32 || c != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }

                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::MemoryCopy,
                            value_stack.len().into(),
                        ));
                    }

                    WasmOpcodeFC::MemoryFill => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let index = stream.read_unsigned()? as usize;
                        if index >= module.memories.len() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }

                        let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        let c = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 || b != WasmValType::I32 || c != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }

                        int_codes.push(WasmImc::new(
                            position,
                            opcode,
                            WasmIntMnemonic::MemoryFill,
                            value_stack.len().into(),
                        ));
                    }

                    #[allow(unreachable_patterns)]
                    _ => return Err(WasmDecodeErrorKind::UnsupportedOpCode(opcode.into())),
                },

                WasmOpcode::PrefixFD(_) => {
                    return Err(WasmDecodeErrorKind::UnsupportedOpCode(opcode.into()))
                }
            }
        }

        if result_types.len() > 0 {
            if result_types.len() != value_stack.len() {
                return Err(WasmDecodeErrorKind::TypeMismatch);
            }

            for result_type in result_types {
                let val = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                if *result_type != val {
                    return Err(WasmDecodeErrorKind::TypeMismatch);
                }
            }
        } else {
            if value_stack.len() > 0 {
                return Err(WasmDecodeErrorKind::InvalidStackLevel);
            }
        }

        macro_rules! fused_inst {
            ( $array:ident, $index:expr, $opr:expr ) => {
                let next = $index + 1;
                $array[next].mnemonic = $opr;
                $array[$index].mnemonic = Nop;
            };
        }

        // fused instructions
        if int_codes.len() > 2 {
            let limit = int_codes.len() - 1;
            for i in 0..limit {
                use WasmIntMnemonic::*;
                let this_op = int_codes[i].mnemonic();
                let next_op = int_codes[i + 1].mnemonic();
                match (this_op, next_op) {
                    (I32Const(val), LocalSet(local_index)) => {
                        fused_inst!(int_codes, i, FusedI32SetConst(*local_index, *val));
                    }
                    (I32Const(val), I32Add) => {
                        fused_inst!(int_codes, i, FusedI32AddI(*val));
                    }
                    (I32Const(val), I32Sub) => {
                        fused_inst!(int_codes, i, FusedI32SubI(*val));
                    }
                    (I32Const(val), I32And) => {
                        fused_inst!(int_codes, i, FusedI32AndI(*val));
                    }
                    (I32Const(val), I32Or) => {
                        fused_inst!(int_codes, i, FusedI32OrI(*val));
                    }
                    (I32Const(val), I32Xor) => {
                        fused_inst!(int_codes, i, FusedI32XorI(*val));
                    }
                    (I32Const(val), I32Shl) => {
                        fused_inst!(int_codes, i, FusedI32ShlI(*val));
                    }
                    (I32Const(val), I32ShrS) => {
                        fused_inst!(int_codes, i, FusedI32ShrSI(*val));
                    }
                    (I32Const(val), I32ShrU) => {
                        fused_inst!(int_codes, i, FusedI32ShrUI(*val));
                    }

                    (I64Const(val), LocalSet(local_index)) => {
                        fused_inst!(int_codes, i, FusedI64SetConst(*local_index, *val));
                    }
                    (I64Const(val), I64Add) => {
                        fused_inst!(int_codes, i, FusedI64AddI(*val));
                    }
                    (I64Const(val), I64Sub) => {
                        fused_inst!(int_codes, i, FusedI64SubI(*val));
                    }

                    (I32Eqz, BrIf(target)) => {
                        fused_inst!(int_codes, i, FusedI32BrZ(*target));
                    }
                    (I32Eq, BrIf(target)) => {
                        fused_inst!(int_codes, i, FusedI32BrEq(*target));
                    }
                    (I32Ne, BrIf(target)) => {
                        fused_inst!(int_codes, i, FusedI32BrNe(*target));
                    }
                    (I32LtS, BrIf(target)) => {
                        fused_inst!(int_codes, i, FusedI32BrLtS(*target));
                    }
                    (I32LtU, BrIf(target)) => {
                        fused_inst!(int_codes, i, FusedI32BrLtU(*target));
                    }
                    (I32GtS, BrIf(target)) => {
                        fused_inst!(int_codes, i, FusedI32BrGtS(*target));
                    }
                    (I32GtU, BrIf(target)) => {
                        fused_inst!(int_codes, i, FusedI32BrGtU(*target));
                    }
                    (I32LeS, BrIf(target)) => {
                        fused_inst!(int_codes, i, FusedI32BrLeS(*target));
                    }
                    (I32LeU, BrIf(target)) => {
                        fused_inst!(int_codes, i, FusedI32BrLeU(*target));
                    }
                    (I32GeS, BrIf(target)) => {
                        fused_inst!(int_codes, i, FusedI32BrGeS(*target));
                    }
                    (I32GeU, BrIf(target)) => {
                        fused_inst!(int_codes, i, FusedI32BrGeU(*target));
                    }

                    (I64Eqz, BrIf(target)) => {
                        fused_inst!(int_codes, i, FusedI64BrZ(*target));
                    }
                    (I64Eq, BrIf(target)) => {
                        fused_inst!(int_codes, i, FusedI64BrEq(*target));
                    }
                    (I64Ne, BrIf(target)) => {
                        fused_inst!(int_codes, i, FusedI64BrNe(*target));
                    }
                    _ => (),
                }
            }
        }

        // compaction and block adjustment
        let mut actual_len = 0;
        for index in 0..int_codes.len() {
            let code = &int_codes[index];
            match *code.mnemonic() {
                WasmIntMnemonic::Nop => (),
                WasmIntMnemonic::Block(target) => {
                    let ref mut block = blocks[target].borrow_mut();
                    block.start_position = actual_len;
                }
                WasmIntMnemonic::End(target) => {
                    let ref mut block = blocks[target].borrow_mut();
                    block.end_position = actual_len;
                }
                _ => {
                    unsafe {
                        int_codes
                            .as_mut_ptr()
                            .add(actual_len)
                            .write(int_codes.as_ptr().add(index).read());
                    }
                    actual_len += 1;
                }
            }
        }
        unsafe {
            int_codes.set_len(actual_len);
        }
        int_codes.shrink_to_fit();

        // fixes branching targets
        for code in int_codes.iter_mut() {
            code.adjust_branch_target(|_opcode, target| {
                blocks
                    .get(target)
                    .ok_or(WasmDecodeErrorKind::OutOfBranch)
                    .map(|block| block.borrow().preferred_target())
            })?;
        }

        Ok(Self {
            func_index,
            file_position,
            local_types: local_var_types.into_boxed_slice(),
            max_stack,
            flags,
            int_codes: int_codes.into_boxed_slice(),
        })
    }
}

/// A type of block instruction (e.g., `block`, `loop`, `if`).
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BlockInstType {
    Block,
    Loop,
    If,
}

#[derive(Debug, Copy, Clone)]
struct WasmBlockContext {
    pub inst_type: BlockInstType,
    pub block_type: WasmBlockType,
    pub stack_level: usize,
    pub start_position: usize,
    pub end_position: usize,
    #[allow(dead_code)]
    pub else_position: usize,
}

impl WasmBlockContext {
    #[inline]
    pub fn preferred_target(&self) -> usize {
        if self.inst_type == BlockInstType::Loop {
            self.start_position
        } else {
            self.end_position
        }
    }
}

/// Instance type to invoke the function
#[derive(Copy, Clone)]
pub struct WasmRunnable<'a> {
    function: &'a WasmFunction,
    module: &'a WasmModule,
}

impl<'a> WasmRunnable<'a> {
    #[inline]
    const fn from_function(function: &'a WasmFunction, module: &'a WasmModule) -> Self {
        Self { function, module }
    }
}

impl WasmRunnable<'_> {
    #[inline]
    pub const fn function(&self) -> &WasmFunction {
        &self.function
    }

    #[inline]
    pub const fn module(&self) -> &WasmModule {
        &self.module
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackLevel(usize);

impl StackLevel {
    #[inline]
    pub const fn as_usize(&self) -> usize {
        self.0 as usize
    }

    #[inline]
    pub const fn zero() -> Self {
        Self(0)
    }
}

// TODO: Will be removed in the future
impl Add<usize> for StackLevel {
    type Output = StackLevel;

    #[inline]
    fn add(self, rhs: usize) -> Self::Output {
        StackLevel(self.0.wrapping_add(rhs))
    }
}

impl Add<StackOffset> for StackLevel {
    type Output = StackLevel;

    #[inline]
    fn add(self, rhs: StackOffset) -> Self::Output {
        StackLevel(self.0.wrapping_add(rhs.0))
    }
}

impl Sub<StackOffset> for StackLevel {
    type Output = StackLevel;

    #[inline]
    fn sub(self, rhs: StackOffset) -> Self::Output {
        StackLevel(self.0.wrapping_sub(rhs.0))
    }
}

impl From<usize> for StackLevel {
    #[inline]
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackOffset(usize);

impl StackOffset {
    #[inline]
    pub const fn new(value: usize) -> Self {
        Self(value)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalVarIndex(usize);

impl LocalVarIndex {
    #[inline]
    pub const fn as_usize(&self) -> usize {
        self.0 as usize
    }
}
