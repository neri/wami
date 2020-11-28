// WebAssembly Loader

use super::opcode::*;
use super::wasmrt::*;
use crate::*;
use alloc::string::*;
use alloc::sync::Arc;
use alloc::vec::Vec;
use byteorder::*;
use core::cell::{RefCell, UnsafeCell};
use core::fmt;
use core::ops::*;
use core::slice;
use core::str;

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

    pub(super) fn new() -> Self {
        Self {
            module: WasmModule::new(),
        }
    }

    /// Identify the file format
    pub fn identity(blob: &[u8]) -> bool {
        blob.len() >= Self::MINIMAL_MOD_SIZE
            && LE::read_u32(&blob[0..4]) == Self::MAGIC
            && LE::read_u32(&blob[4..8]) == Self::VER_CURRENT
    }

    /// Instantiate wasm modules from slice
    pub fn instantiate(blob: &[u8]) -> Result<WasmModule, WasmDecodeError> {
        if Self::identity(blob) {
            let mut loader = Self::new();
            loader.load(blob).map(|_| loader.module)
        } else {
            return Err(WasmDecodeError::BadExecutable);
        }
    }

    pub(super) fn load(&mut self, blob: &[u8]) -> Result<(), WasmDecodeError> {
        let mut blob = Leb128Stream::from_slice(&blob[8..]);
        while let Some(mut section) = blob.next_section()? {
            match section.section_type {
                WasmSectionType::Custom => Ok(()),
                WasmSectionType::Type => self.parse_sec_type(&mut section),
                WasmSectionType::Import => self.parse_sec_import(&mut section),
                WasmSectionType::Table => self.parse_sec_table(&mut section),
                WasmSectionType::Memory => self.parse_sec_memory(&mut section),
                WasmSectionType::Element => self.parse_sec_elem(&mut section),
                WasmSectionType::Function => self.parse_sec_func(&mut section),
                WasmSectionType::Export => self.parse_sec_export(&mut section),
                WasmSectionType::Code => self.parse_sec_code(&mut section),
                WasmSectionType::Data => self.parse_sec_data(&mut section),
                WasmSectionType::Start => self.parse_sec_start(&mut section),
                WasmSectionType::Global => self.parse_sec_global(&mut section),
                // _ => Err(WasmDecodeError::UnexpectedToken),
            }?;
        }
        Ok(())
    }

    pub fn print_stat(&mut self) {
        self.module.print_stat();
    }

    pub fn module(&mut self) -> &WasmModule {
        &self.module
    }

    /// Parse "type" section
    fn parse_sec_type(&mut self, section: &mut WasmSection) -> Result<(), WasmDecodeError> {
        let n_items = section.stream.read_uint()? as usize;
        for _ in 0..n_items {
            let ft = WasmType::from_stream(&mut section.stream)?;
            self.module.types.push(ft);
        }
        Ok(())
    }

    /// Parse "import" section
    fn parse_sec_import(&mut self, section: &mut WasmSection) -> Result<(), WasmDecodeError> {
        let n_items = section.stream.read_uint()? as usize;
        for _ in 0..n_items {
            let import = WasmImport::from_stream(&mut section.stream)?;
            if let WasmImportIndex::Type(index) = import.index {
                self.module
                    .functions
                    .push(WasmFunction::from_import(index, self.module.n_ext_func));
                self.module.n_ext_func += 1;
            }
            self.module.imports.push(import);
        }
        Ok(())
    }

    /// Parse "func" section
    fn parse_sec_func(&mut self, section: &mut WasmSection) -> Result<(), WasmDecodeError> {
        let n_items = section.stream.read_uint()?;
        for _ in 0..n_items {
            let index = section.stream.read_uint()? as usize;
            self.module.functions.push(WasmFunction::internal(index));
        }
        Ok(())
    }

    /// Parse "export" section
    fn parse_sec_export(&mut self, section: &mut WasmSection) -> Result<(), WasmDecodeError> {
        let n_items = section.stream.read_uint()? as usize;
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
    fn parse_sec_memory(&mut self, section: &mut WasmSection) -> Result<(), WasmDecodeError> {
        let n_items = section.stream.read_uint()?;
        for _ in 0..n_items {
            let limit = WasmLimit::from_stream(&mut section.stream)?;
            self.module.memories.push(WasmMemory::new(limit));
        }
        Ok(())
    }

    /// Parse "table" section
    fn parse_sec_table(&mut self, section: &mut WasmSection) -> Result<(), WasmDecodeError> {
        let n_items = section.stream.read_uint()?;
        for _ in 0..n_items {
            let table = WasmTable::from_stream(&mut section.stream)?;
            self.module.tables.push(table);
        }
        Ok(())
    }

    /// Parse "elem" section
    fn parse_sec_elem(&mut self, section: &mut WasmSection) -> Result<(), WasmDecodeError> {
        let n_items = section.stream.read_uint()?;
        for _ in 0..n_items {
            let tabidx = section.stream.read_uint()? as usize;
            let offset = self.eval_offset(&mut section.stream)? as usize;
            let n_elements = section.stream.read_uint()? as usize;
            let table = self
                .module
                .tables
                .get_mut(tabidx)
                .ok_or(WasmDecodeError::InvalidParameter)?;
            for i in offset..offset + n_elements {
                let elem = section.stream.read_uint()? as usize;
                table.table.get_mut(i).map(|v| *v = elem);
            }
        }
        Ok(())
    }

    /// Parse "code" section
    fn parse_sec_code(&mut self, section: &mut WasmSection) -> Result<(), WasmDecodeError> {
        let n_items = section.stream.read_uint()? as usize;
        for i in 0..n_items {
            let index = i + self.module.n_ext_func;
            let module = &mut self.module;
            let func_def = module
                .functions
                .get(index)
                .ok_or(WasmDecodeError::InvalidParameter)?;
            let func_type = module
                .type_by_ref(func_def.type_ref)
                .ok_or(WasmDecodeError::InvalidParameter)?;
            let body = WasmFunctionBody::from_stream(&mut section.stream, func_type, module)?;
            self.module.functions[index].body = Some(body);
        }
        Ok(())
    }

    /// Parse "data" section
    fn parse_sec_data(&mut self, section: &mut WasmSection) -> Result<(), WasmDecodeError> {
        let n_items = section.stream.read_uint()?;
        for _ in 0..n_items {
            let memidx = section.stream.read_uint()? as usize;
            let offset = self.eval_offset(&mut section.stream)?;
            let src = section.stream.read_bytes()?;
            let memory = self
                .module
                .memories
                .get_mut(memidx)
                .ok_or(WasmDecodeError::InvalidParameter)?;
            memory.write_bytes(offset, src).unwrap();
        }
        Ok(())
    }

    /// Parse "start" section
    fn parse_sec_start(&mut self, section: &mut WasmSection) -> Result<(), WasmDecodeError> {
        let index = section.stream.read_uint()? as usize;
        self.module.start = Some(index);
        Ok(())
    }

    /// Parse "global" section
    fn parse_sec_global(&mut self, section: &mut WasmSection) -> Result<(), WasmDecodeError> {
        let n_items = section.stream.read_uint()? as usize;
        for _ in 0..n_items {
            let val_type = section
                .stream
                .read_byte()
                .and_then(|v| WasmValType::from_u64(v as u64))?;
            let is_mutable = section.stream.read_byte()? == 1;
            let value = self
                .eval_offset(&mut section.stream)
                .map(|v| WasmValue::I32(v as i32))?;

            let global = WasmGlobal {
                val_type,
                is_mutable,
                value,
            };
            self.module.globals.push(global);
        }
        Ok(())
    }

    fn eval_offset(&mut self, stream: &mut Leb128Stream) -> Result<usize, WasmDecodeError> {
        stream
            .read_byte()
            .and_then(|opc| match WasmOpcode::from_u8(opc) {
                WasmOpcode::I32Const => stream.read_sint().and_then(|r| {
                    match stream.read_byte().map(|v| WasmOpcode::from_u8(v)) {
                        Ok(WasmOpcode::End) => Ok((r as u32) as usize),
                        _ => Err(WasmDecodeError::UnexpectedToken),
                    }
                }),
                _ => Err(WasmDecodeError::UnexpectedToken),
            })
    }
}

pub struct WasmModule {
    types: Vec<WasmType>,
    imports: Vec<WasmImport>,
    exports: Vec<WasmExport>,
    memories: Vec<WasmMemory>,
    tables: Vec<WasmTable>,
    functions: Vec<WasmFunction>,
    start: Option<usize>,
    globals: Vec<WasmGlobal>,
    n_ext_func: usize,
}

impl WasmModule {
    const fn new() -> Self {
        Self {
            types: Vec::new(),
            memories: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            tables: Vec::new(),
            functions: Vec::new(),
            start: None,
            globals: Vec::new(),
            n_ext_func: 0,
        }
    }

    pub fn types(&self) -> &[WasmType] {
        self.types.as_slice()
    }

    pub fn type_by_ref(&self, index: usize) -> Option<&WasmType> {
        self.types.get(index)
    }

    pub fn imports(&self) -> &[WasmImport] {
        self.imports.as_slice()
    }

    pub fn exports(&self) -> &[WasmExport] {
        self.exports.as_slice()
    }

    pub fn memories(&mut self) -> &mut [WasmMemory] {
        self.memories.as_mut_slice()
    }

    pub fn tables(&mut self) -> &mut [WasmTable] {
        self.tables.as_mut_slice()
    }

    pub fn func_by_ref(&self, index: usize) -> Result<&WasmFunction, WasmRuntimeError> {
        self.functions.get(index).ok_or(WasmRuntimeError::NoMethod)
    }

    pub fn entry_point(&self) -> Result<&WasmFunction, WasmRuntimeError> {
        self.start
            .ok_or(WasmRuntimeError::NoMethod)
            .and_then(|v| self.func_by_ref(v))
    }

    /// Get a reference to the exported function with the specified name
    pub fn function(&self, name: &str) -> Result<&WasmFunction, WasmRuntimeError> {
        for export in &self.exports {
            if let WasmExportIndex::Function(v) = export.index {
                if export.name == name {
                    return self.func_by_ref(v);
                }
            }
        }
        Err(WasmRuntimeError::NoMethod)
    }

    pub fn print_stat(&mut self) {
        for (func_idx, function) in self.functions.iter().enumerate() {
            let type_ref = self.types.get(function.type_ref).unwrap();

            match function.origin {
                WasmFunctionOrigin::Internal => {
                    println!("func {}{}", func_idx, type_ref);
                    let _ = self.disassemble(func_idx);
                }
                WasmFunctionOrigin::Export(v) => {
                    let export = self.exports.get(v).unwrap();
                    println!("func {} (export {}){}", func_idx, export.name, type_ref);
                    let _ = self.disassemble(func_idx);
                }
                WasmFunctionOrigin::Import(v) => {
                    let import = self.imports.get(v).unwrap();
                    println!(
                        "func {} (import {}.{}){} ",
                        func_idx, import.mod_name, import.name, type_ref,
                    );
                }
            }
        }
    }

    pub fn disassemble(&self, func_idx: usize) -> Result<(), WasmDecodeError> {
        let func = self.functions.get(func_idx).unwrap();
        let type_ref = self.types.get(func.type_ref).unwrap();
        let body = match func.body.as_ref() {
            Some(v) => v,
            None => {
                println!("  (#ERROR)");
                return Err(WasmDecodeError::UnexpectedEof);
            }
        };
        let locals = body.local_types.as_slice();
        if locals.len() > 0 {
            let mut local_index = type_ref.params.len();
            for local in locals {
                println!(" (local ${}, {})", local_index, local);
                local_index += 1;
            }
        }
        let code_block = body.code_block.borrow();
        let mut stream = Leb128Stream::from_slice(&code_block);
        let mut block_level = 1;
        while let Ok(opcode) = stream.read_byte() {
            let op = WasmOpcode::from_u8(opcode);

            match op.mnemonic_type() {
                WasmMnemonicType::Else => {
                    block_level -= 1;
                    Self::nest(block_level);
                    println!("else");
                    block_level += 1;
                }
                WasmMnemonicType::End => {
                    if block_level > 1 {
                        block_level -= 1;
                        Self::nest(block_level);
                        println!("end");
                    } else {
                        break;
                    }
                }
                _ => {
                    Self::nest(block_level);
                }
            }

            match op.mnemonic_type() {
                WasmMnemonicType::Else | WasmMnemonicType::End => (),

                WasmMnemonicType::Implied => println!("{}", op.to_str()),

                WasmMnemonicType::Block => {
                    let type_ref = stream.read_byte().and_then(|v| {
                        WasmBlockType::from_i64(v as i64)
                            .map_err(|_| WasmDecodeError::UnexpectedToken)
                    })?;
                    match type_ref {
                        WasmBlockType::Empty => println!("{}", op.to_str(),),
                        _ => println!("{} {:?}", op.to_str(), type_ref),
                    }
                    block_level += 1;
                }
                WasmMnemonicType::Br
                | WasmMnemonicType::Call
                | WasmMnemonicType::CallIndirect
                | WasmMnemonicType::Local
                | WasmMnemonicType::Global
                | WasmMnemonicType::MemSize => {
                    let opr = stream.read_uint()?;
                    println!("{} {}", op.to_str(), opr);
                }
                WasmMnemonicType::BrTable => {
                    let n_vec = stream.read_uint()?;
                    print!("{} ", op.to_str());
                    for _ in 0..n_vec {
                        let target = stream.read_uint()?;
                        print!(" {}", target);
                    }
                    let target = stream.read_uint()?;
                    println!(" {}", target);
                }
                WasmMnemonicType::Memory => {
                    let a = stream.read_uint()?;
                    let o = stream.read_uint()?;
                    println!("{} offset={} align={}", op.to_str(), o, a);
                }
                WasmMnemonicType::I32 => {
                    let opr = stream.read_sint()? as i32;
                    println!("{} {} ;; 0x{:x}", op.to_str(), opr, opr);
                }
                WasmMnemonicType::I64 => {
                    let opr = stream.read_sint()?;
                    println!("{} {} ;; 0x{:x}", op.to_str(), opr, opr);
                }

                WasmMnemonicType::F32 => todo!(),
                WasmMnemonicType::F64 => todo!(),
            }
        }
        Ok(())
    }

    fn nest(level: usize) {
        let level = usize::min(level, 20);
        for _ in 0..level {
            print!("  ");
        }
    }
}

pub struct Leb128Stream<'a> {
    blob: &'a [u8],
    position: usize,
    fetch_position: usize,
}

impl<'a> Leb128Stream<'a> {
    /// Instantiates from a slice
    pub const fn from_slice(slice: &'a [u8]) -> Self {
        Self {
            blob: slice,
            position: 0,
            fetch_position: 0,
        }
    }
}

#[allow(dead_code)]
impl Leb128Stream<'_> {
    /// Returns to the origin of the stream
    #[inline]
    pub fn reset(&mut self) {
        self.position = 0;
        self.fetch_position = 0;
    }

    /// Gets current position of stream
    #[inline]
    pub const fn position(&self) -> usize {
        self.position
    }

    #[inline]
    pub const fn fetch_position(&self) -> usize {
        self.fetch_position
    }

    /// Returns whether the end of the stream has been reached
    #[inline]
    pub const fn is_eof(&self) -> bool {
        self.position >= self.blob.len()
    }

    /// Reads one byte from a stream
    pub fn read_byte(&mut self) -> Result<u8, WasmDecodeError> {
        if self.is_eof() {
            return Err(WasmDecodeError::UnexpectedEof);
        }
        let d = self.blob[self.position];
        self.position += 1;
        Ok(d)
    }

    /// Returns a slice of the specified number of bytes from the stream
    pub fn get_bytes(&mut self, size: usize) -> Result<&[u8], WasmDecodeError> {
        let limit = self.blob.len();
        if self.position <= limit && size <= limit && self.position + size <= limit {
            let offset = self.position;
            self.position += size;
            Ok(&self.blob[offset..offset + size])
        } else {
            Err(WasmDecodeError::UnexpectedEof)
        }
    }

    /// Reads multiple bytes from the stream
    #[inline]
    pub fn read_bytes(&mut self) -> Result<&[u8], WasmDecodeError> {
        self.read_uint()
            .and_then(move |size| self.get_bytes(size as usize))
    }

    /// Reads an unsigned integer from a stream
    pub fn read_uint(&mut self) -> Result<u64, WasmDecodeError> {
        let mut value: u64 = 0;
        let mut scale = 0;
        let mut cursor = self.position;
        loop {
            if self.is_eof() {
                return Err(WasmDecodeError::UnexpectedEof);
            }
            let d = self.blob[cursor];
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
    pub fn read_sint(&mut self) -> Result<i64, WasmDecodeError> {
        let mut value: u64 = 0;
        let mut scale = 0;
        let mut cursor = self.position;
        let signed = loop {
            if self.is_eof() {
                return Err(WasmDecodeError::UnexpectedEof);
            }
            let d = self.blob[cursor];
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

    /// Reads the UTF-8 encoded string from the stream
    #[inline]
    pub fn get_string(&mut self) -> Result<&str, WasmDecodeError> {
        self.read_bytes()
            .and_then(|v| str::from_utf8(v).map_err(|_| WasmDecodeError::UnexpectedToken))
    }

    #[inline]
    pub fn read_opcode(&mut self) -> Result<WasmOpcode, WasmDecodeError> {
        self.fetch_position = self.position();
        self.read_byte().map(|v| WasmOpcode::from_u8(v))
    }

    #[inline]
    pub fn read_memarg(&mut self) -> Result<(u32, u32), WasmDecodeError> {
        let a = self.read_uint()? as u32;
        let o = self.read_uint()? as u32;
        Ok((o, a))
    }

    fn next_section(&mut self) -> Result<Option<WasmSection>, WasmDecodeError> {
        let section_type = match self.read_byte().ok() {
            Some(v) => v,
            None => return Ok(None),
        };

        let blob = self.read_bytes()?;
        let stream = Leb128Stream::from_slice(blob);
        Ok(Some(WasmSection {
            section_type: section_type.into(),
            stream,
        }))
    }
}

struct WasmSection<'a> {
    section_type: WasmSectionType,
    stream: Leb128Stream<'a>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialOrd, PartialEq)]
enum WasmSectionType {
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
}

impl From<u8> for WasmSectionType {
    fn from(v: u8) -> Self {
        match v {
            1 => WasmSectionType::Type,
            2 => WasmSectionType::Import,
            3 => WasmSectionType::Function,
            4 => WasmSectionType::Table,
            5 => WasmSectionType::Memory,
            6 => WasmSectionType::Global,
            7 => WasmSectionType::Export,
            8 => WasmSectionType::Start,
            9 => WasmSectionType::Element,
            10 => WasmSectionType::Code,
            11 => WasmSectionType::Data,
            _ => WasmSectionType::Custom,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum WasmValType {
    I32 = 0x7F,
    I64 = 0x7E,
    F32 = 0x7D,
    F64 = 0x7C,
}

impl WasmValType {
    const fn from_u64(v: u64) -> Result<Self, WasmDecodeError> {
        match v {
            0x7F => Ok(WasmValType::I32),
            0x7E => Ok(WasmValType::I64),
            0x7D => Ok(WasmValType::F32),
            0x7C => Ok(WasmValType::F64),
            _ => Err(WasmDecodeError::UnexpectedToken),
        }
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WasmBlockType {
    Empty = 0x40,
    I32 = 0x7F,
    I64 = 0x7E,
    F32 = 0x7D,
    F64 = 0x7C,
}

impl WasmBlockType {
    pub const fn from_i64(v: i64) -> Result<Self, WasmDecodeError> {
        match v {
            0x40 => Ok(Self::Empty),
            0x7F => Ok(Self::I32),
            0x7E => Ok(Self::I64),
            0x7D => Ok(Self::F32),
            0x7C => Ok(Self::F64),
            _ => Err(WasmDecodeError::InvalidParameter),
        }
    }

    pub const fn into_type<'a>(self) -> &'a [WasmValType] {
        match self {
            WasmBlockType::Empty => &[],
            WasmBlockType::I32 => &[WasmValType::I32],
            WasmBlockType::I64 => &[WasmValType::I64],
            WasmBlockType::F32 => &[WasmValType::F32],
            WasmBlockType::F64 => &[WasmValType::F64],
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct WasmLimit {
    min: u32,
    max: u32,
}

impl WasmLimit {
    fn from_stream(stream: &mut Leb128Stream) -> Result<Self, WasmDecodeError> {
        match stream.read_uint() {
            Ok(0) => stream.read_uint().map(|min| Self {
                min: min as u32,
                max: min as u32,
            }),
            Ok(1) => {
                let min = stream.read_uint()? as u32;
                let max = stream.read_uint()? as u32;
                Ok(Self { min, max })
            }
            Err(err) => Err(err),
            _ => Err(WasmDecodeError::UnexpectedToken),
        }
    }
}

#[allow(dead_code)]
pub struct WasmMemory {
    limit: WasmLimit,
    memory: Arc<UnsafeCell<Vec<u8>>>,
}

impl WasmMemory {
    const PAGE_SIZE: usize = 0x10000;

    fn new(limit: WasmLimit) -> Self {
        let size = limit.min as usize * Self::PAGE_SIZE;
        let mut memory = Vec::with_capacity(size);
        memory.resize(size, 0);
        Self {
            limit,
            memory: Arc::new(UnsafeCell::new(memory)),
        }
    }

    pub fn limit(&self) -> WasmLimit {
        self.limit
    }

    pub fn memory_arc(&mut self) -> Arc<UnsafeCell<Vec<u8>>> {
        self.memory.clone()
    }

    pub fn memory(&self) -> &[u8] {
        unsafe { self.memory.get().as_ref().unwrap() }
    }

    pub fn memory_mut(&mut self) -> &mut [u8] {
        unsafe { self.memory.get().as_mut().unwrap() }
    }

    /// Read the specified range of memory
    pub fn read_bytes(&self, offset: usize, size: usize) -> Result<&[u8], WasmMemoryError> {
        let memory = self.memory();
        let limit = memory.len();
        if offset < limit && size < limit && offset + size < limit {
            unsafe { Ok(slice::from_raw_parts(&memory[offset] as *const _, size)) }
        } else {
            Err(WasmMemoryError::OutOfBounds)
        }
    }

    /// Write slice to memory
    pub fn write_bytes(&mut self, offset: usize, src: &[u8]) -> Result<(), WasmMemoryError> {
        let memory = self.memory_mut();
        let size = src.len();
        let limit = memory.len();
        if offset < limit && size < limit && offset + size < limit {
            let dest = &mut memory[offset] as *mut u8;
            let src = &src[0] as *const u8;
            unsafe {
                dest.copy_from_nonoverlapping(src, size);
            }
            Ok(())
        } else {
            Err(WasmMemoryError::OutOfBounds)
        }
    }

    // pub fn grow(&mut self, delta: usize)
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum WasmMemoryError {
    NullPointerException,
    OutOfBounds,
    OutOfMemory,
}

pub struct WasmTable {
    limit: WasmLimit,
    table: Vec<usize>,
}

impl WasmTable {
    fn from_stream(stream: &mut Leb128Stream) -> Result<Self, WasmDecodeError> {
        match stream.read_uint() {
            Ok(0x70) => (),
            Err(err) => return Err(err),
            _ => return Err(WasmDecodeError::UnexpectedToken),
        };
        WasmLimit::from_stream(stream).map(|limit| {
            let size = limit.min as usize;
            let mut table = Vec::with_capacity(size);
            table.resize(size, 0);
            Self { limit, table }
        })
    }

    pub fn limit(&self) -> WasmLimit {
        self.limit
    }

    pub fn table(&mut self) -> &mut [usize] {
        self.table.as_mut_slice()
    }
}

pub struct WasmFunction {
    type_ref: usize,
    origin: WasmFunctionOrigin,
    body: Option<WasmFunctionBody>,
}

impl WasmFunction {
    fn from_import(type_ref: usize, index: usize) -> Self {
        Self {
            type_ref,
            origin: WasmFunctionOrigin::Import(index),
            body: None,
        }
    }

    fn internal(type_ref: usize) -> Self {
        Self {
            type_ref,
            origin: WasmFunctionOrigin::Internal,
            body: None,
        }
    }

    pub fn type_ref(&self) -> usize {
        self.type_ref
    }

    pub fn origin(&self) -> WasmFunctionOrigin {
        self.origin
    }

    pub fn body(&self) -> Option<&WasmFunctionBody> {
        self.body.as_ref()
    }

    pub fn invoke(&self, params: &[WasmValue]) -> Result<WasmValue, WasmRuntimeError> {
        let body = self.body.as_ref().ok_or(WasmRuntimeError::NoMethod)?;

        let mut locals = Vec::new();
        for param in params {
            locals.push(*param);
        }
        for local in &body.local_types {
            locals.push(WasmValue::default_for(*local));
        }

        let result_types = body.result_types.as_slice();

        let code_ref = body.code_block.borrow();
        let mut code_block = WasmCodeBlock::from_slice(&code_ref);
        code_block.invoke(locals.as_mut_slice(), result_types)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum WasmFunctionOrigin {
    Internal,
    Export(usize),
    Import(usize),
}

#[derive(Debug, Clone)]
pub struct WasmType {
    params: Vec<WasmValType>,
    result: Vec<WasmValType>,
}

impl WasmType {
    fn from_stream(stream: &mut Leb128Stream) -> Result<Self, WasmDecodeError> {
        match stream.read_uint() {
            Ok(0x60) => (),
            Err(err) => return Err(err),
            _ => return Err(WasmDecodeError::UnexpectedToken),
        };
        let n_params = stream.read_uint()? as usize;
        let mut params = Vec::with_capacity(n_params);
        for _ in 0..n_params {
            stream
                .read_uint()
                .and_then(|v| WasmValType::from_u64(v))
                .map(|v| params.push(v))?;
        }
        let n_result = stream.read_uint()? as usize;
        let mut result = Vec::with_capacity(n_result);
        for _ in 0..n_result {
            stream
                .read_uint()
                .and_then(|v| WasmValType::from_u64(v))
                .map(|v| result.push(v))?;
        }
        Ok(Self { params, result })
    }

    pub fn param_types(&self) -> &[WasmValType] {
        self.params.as_slice()
    }

    pub fn result_types(&self) -> &[WasmValType] {
        self.result.as_slice()
    }
}

impl fmt::Display for WasmType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.params.len() > 0 {
            write!(f, " (param")?;
            for param in &self.params {
                write!(f, " {}", param)?;
            }
            write!(f, ")")?;
        }
        if self.result.len() > 0 {
            write!(f, " (result")?;
            for result in &self.result {
                write!(f, " {}", result)?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

pub struct WasmImport {
    mod_name: String,
    name: String,
    index: WasmImportIndex,
}

impl WasmImport {
    fn from_stream(stream: &mut Leb128Stream) -> Result<Self, WasmDecodeError> {
        let mod_name = stream.get_string()?.to_string();
        let name = stream.get_string()?.to_string();
        let index = WasmImportIndex::from_stream(stream)?;

        Ok(Self {
            mod_name,
            name,
            index,
        })
    }

    pub fn mod_name(&self) -> &str {
        self.mod_name.as_ref()
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub const fn index(&self) -> WasmImportIndex {
        self.index
    }
}

#[derive(Debug, Copy, Clone)]
pub enum WasmImportIndex {
    Type(usize),
    Table(usize),
    Memory(usize),
    Global(usize),
}

impl WasmImportIndex {
    fn from_stream(stream: &mut Leb128Stream) -> Result<Self, WasmDecodeError> {
        stream.read_uint().and_then(|v| match v {
            0 => stream.read_uint().map(|v| Self::Type(v as usize)),
            1 => stream.read_uint().map(|v| Self::Table(v as usize)),
            2 => stream.read_uint().map(|v| Self::Memory(v as usize)),
            3 => stream.read_uint().map(|v| Self::Global(v as usize)),
            _ => Err(WasmDecodeError::UnexpectedToken),
        })
    }
}

pub struct WasmExport {
    name: String,
    index: WasmExportIndex,
}

impl WasmExport {
    fn from_stream(stream: &mut Leb128Stream) -> Result<Self, WasmDecodeError> {
        let name = stream.get_string()?.to_string();
        let index = WasmExportIndex::from_stream(stream)?;
        Ok(Self { name, index })
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

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
    fn from_stream(stream: &mut Leb128Stream) -> Result<Self, WasmDecodeError> {
        stream.read_uint().and_then(|v| match v {
            0 => stream.read_uint().map(|v| Self::Function(v as usize)),
            1 => stream.read_uint().map(|v| Self::Table(v as usize)),
            2 => stream.read_uint().map(|v| Self::Memory(v as usize)),
            3 => stream.read_uint().map(|v| Self::Global(v as usize)),
            _ => Err(WasmDecodeError::UnexpectedToken),
        })
    }
}

pub struct WasmFunctionBody {
    param_types: Vec<WasmValType>,
    local_types: Vec<WasmValType>,
    result_types: Vec<WasmValType>,
    code_block: Arc<RefCell<Vec<u8>>>,
    block_info: WasmBlockInfo,
}

impl WasmFunctionBody {
    fn from_stream(
        stream: &mut Leb128Stream,
        func_type: &WasmType,
        module: &WasmModule,
    ) -> Result<Self, WasmDecodeError> {
        let blob = stream.read_bytes()?;
        let mut stream = Leb128Stream::from_slice(blob);
        let n_locals = stream.read_uint()? as usize;
        let mut locals = Vec::new();
        for _ in 0..n_locals {
            let repeat = stream.read_uint()?;
            let val = stream.read_uint().and_then(|v| WasmValType::from_u64(v))?;
            for _ in 0..repeat {
                locals.push(val);
            }
        }
        let code_block = Arc::new(RefCell::new(blob[stream.position..].to_vec()));
        let param_types = func_type.params.clone();
        let result_types = func_type.result.clone();

        let block_info = {
            let mut local_types = Vec::new();
            for param_type in &param_types {
                local_types.push(param_type.clone());
            }
            for local in &locals {
                local_types.push(local.clone());
            }
            let code_ref = code_block.borrow();
            let mut code_block = Leb128Stream::from_slice(&code_ref);
            WasmBlockInfo::analyze(&mut code_block, &local_types, &result_types, module)
        }?;

        Ok(Self {
            param_types,
            local_types: locals,
            result_types,
            code_block,
            block_info,
        })
    }

    pub fn param_types(&self) -> &[WasmValType] {
        self.param_types.as_slice()
    }

    pub fn local_types(&self) -> &[WasmValType] {
        self.local_types.as_slice()
    }

    pub fn result_types(&self) -> &[WasmValType] {
        self.result_types.as_slice()
    }

    pub fn block_info(&self) -> &WasmBlockInfo {
        &self.block_info
    }

    pub fn code_block(&self) -> Arc<RefCell<Vec<u8>>> {
        self.code_block.clone()
    }
}

#[allow(dead_code)]
pub struct WasmGlobal {
    val_type: WasmValType,
    is_mutable: bool,
    value: WasmValue,
}

#[derive(Debug, Copy, Clone)]
pub enum WasmDecodeError {
    BadExecutable,
    UnexpectedEof,
    UnexpectedToken,
    InvalidParameter,
    InvalidStackLevel,
    InvalidGlobal,
    InvalidLocal,
    OutOfStack,
    TypeMismatch,
    BlockMismatch,
    ElseWithoutIf,
    UnreachableTrap,
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum WasmRuntimeError {
    OutOfBounds,
    OutOfMemory,
    UnexpectedEof,
    UnexpectedToken,
    InvalidParameter,
    InvalidBytecode,
    NoMethod,
    DivideByZero,
    TypeMismatch,
}

#[derive(Debug, Copy, Clone)]
pub enum WasmValue {
    Empty,
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

impl WasmValue {
    pub fn default_for(val_type: WasmValType) -> Self {
        match val_type {
            WasmValType::I32 => Self::I32(0),
            WasmValType::I64 => Self::I64(0),
            WasmValType::F32 => Self::F32(0.0),
            WasmValType::F64 => Self::F64(0.0),
        }
    }

    #[inline]
    pub fn get_i32(self) -> Result<i32, WasmRuntimeError> {
        match self {
            Self::I32(a) => Ok(a),
            _ => return Err(WasmRuntimeError::TypeMismatch),
        }
    }

    #[inline]
    pub fn get_u32(self) -> Result<u32, WasmRuntimeError> {
        match self {
            Self::I32(a) => Ok(a as u32),
            _ => return Err(WasmRuntimeError::TypeMismatch),
        }
    }

    #[inline]
    pub fn get_i64(self) -> Result<i64, WasmRuntimeError> {
        match self {
            Self::I64(a) => Ok(a),
            _ => return Err(WasmRuntimeError::TypeMismatch),
        }
    }

    #[inline]
    pub fn get_u64(self) -> Result<u64, WasmRuntimeError> {
        match self {
            Self::I64(a) => Ok(a as u64),
            _ => return Err(WasmRuntimeError::TypeMismatch),
        }
    }

    #[inline]
    pub fn map_i32<F>(self, f: F) -> Result<WasmValue, WasmRuntimeError>
    where
        F: FnOnce(i32) -> i32,
    {
        match self {
            Self::I32(a) => Ok(f(a).into()),
            _ => return Err(WasmRuntimeError::TypeMismatch),
        }
    }

    #[inline]
    pub fn map_i64<F>(self, f: F) -> Result<WasmValue, WasmRuntimeError>
    where
        F: FnOnce(i64) -> i64,
    {
        match self {
            Self::I64(a) => Ok(f(a).into()),
            _ => return Err(WasmRuntimeError::TypeMismatch),
        }
    }
}

impl From<i32> for WasmValue {
    fn from(v: i32) -> Self {
        Self::I32(v)
    }
}

impl From<u32> for WasmValue {
    fn from(v: u32) -> Self {
        Self::I32(v as i32)
    }
}

impl From<i64> for WasmValue {
    fn from(v: i64) -> Self {
        Self::I64(v)
    }
}

impl From<u64> for WasmValue {
    fn from(v: u64) -> Self {
        Self::I64(v as i64)
    }
}

impl From<f32> for WasmValue {
    fn from(v: f32) -> Self {
        Self::F32(v)
    }
}

impl From<f64> for WasmValue {
    fn from(v: f64) -> Self {
        Self::F64(v)
    }
}

impl From<bool> for WasmValue {
    fn from(v: bool) -> Self {
        Self::I32(if v { 1 } else { 0 })
    }
}

impl fmt::Display for WasmValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Empty => write!(f, "()"),
            Self::I32(v) => write!(f, "{}", v),
            Self::I64(v) => write!(f, "{}", v),
            Self::F32(_) => write!(f, "(#!F32)"),
            Self::F64(_) => write!(f, "(#!F64)"),
        }
    }
}

pub struct WasmCodeBlock<'a> {
    code: Leb128Stream<'a>,
}

impl<'a> WasmCodeBlock<'a> {
    pub fn from_slice(slice: &'a [u8]) -> Self {
        Self {
            code: Leb128Stream::from_slice(slice),
        }
    }

    pub fn reset(&mut self) {
        self.code.reset();
    }

    pub const fn position(&self) -> usize {
        self.code.position()
    }

    pub const fn fetch_position(&self) -> usize {
        self.code.fetch_position
    }

    pub fn read_opcode(&mut self) -> Result<WasmOpcode, WasmRuntimeError> {
        self.code.read_opcode().map_err(|err| Self::map_err(err))
    }

    pub fn read_sint(&mut self) -> Result<i64, WasmRuntimeError> {
        self.code.read_sint().map_err(|err| Self::map_err(err))
    }

    pub fn read_uint(&mut self) -> Result<u64, WasmRuntimeError> {
        self.code.read_uint().map_err(|err| Self::map_err(err))
    }

    pub fn read_byte(&mut self) -> Result<u8, WasmRuntimeError> {
        self.code.read_byte().map_err(|err| Self::map_err(err))
    }

    pub fn read_memarg(&mut self) -> Result<(u32, u32), WasmRuntimeError> {
        self.code.read_memarg().map_err(|err| Self::map_err(err))
    }

    fn map_err(err: WasmDecodeError) -> WasmRuntimeError {
        match err {
            WasmDecodeError::UnexpectedEof => WasmRuntimeError::UnexpectedEof,
            _ => WasmRuntimeError::UnexpectedToken,
        }
    }

    #[inline]
    pub fn invoke(
        &mut self,
        locals: &mut [WasmValue],
        result_types: &[WasmValType],
    ) -> Result<WasmValue, WasmRuntimeError> {
        let mut ctx = WasmRuntimeContext::new();
        ctx.run(self, locals, result_types)
    }
}

#[derive(Debug)]
pub struct WasmBlockInfo {
    max_stack: usize,
    blocks: Vec<WasmBlockContext>,
}

impl WasmBlockInfo {
    /// Analyze block info
    pub fn analyze<'a>(
        code_block: &mut Leb128Stream,
        local_types: &[WasmValType],
        result_types: &[WasmValType],
        module: &WasmModule,
    ) -> Result<Self, WasmDecodeError> {
        let mut blocks = Vec::new();
        let mut block_stack = Vec::new();
        let mut value_stack = Vec::new();
        let mut max_stack = 0;

        loop {
            max_stack = usize::max(max_stack, value_stack.len());
            let position = code_block.position();
            let opcode = code_block.read_opcode()?;
            let old_values = value_stack.clone();

            match opcode {
                WasmOpcode::Unreachable => (),

                WasmOpcode::Nop => (),

                WasmOpcode::Block => {
                    let block_type = code_block
                        .read_byte()
                        .and_then(|v| WasmBlockType::from_i64(v as i64))?;
                    let block = RefCell::new(WasmBlockContext {
                        inst_type: BlockInstType::Block,
                        block_type,
                        stack_level: value_stack.len(),
                        start_position: position,
                        end_position: 0,
                        else_position: 0,
                    });
                    block_stack.push(blocks.len());
                    blocks.push(block);
                }
                WasmOpcode::Loop => {
                    let block_type = code_block
                        .read_byte()
                        .and_then(|v| WasmBlockType::from_i64(v as i64))?;
                    let block = RefCell::new(WasmBlockContext {
                        inst_type: BlockInstType::Loop,
                        block_type,
                        stack_level: value_stack.len(),
                        start_position: position,
                        end_position: 0,
                        else_position: 0,
                    });
                    block_stack.push(blocks.len());
                    blocks.push(block);
                }
                WasmOpcode::If => {
                    let cc = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if cc != WasmValType::I32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    let block_type = code_block
                        .read_byte()
                        .and_then(|v| WasmBlockType::from_i64(v as i64))?;
                    let block = RefCell::new(WasmBlockContext {
                        inst_type: BlockInstType::If,
                        block_type,
                        stack_level: value_stack.len(),
                        start_position: position,
                        end_position: 0,
                        else_position: 0,
                    });
                    block_stack.push(blocks.len());
                    blocks.push(block);
                }
                WasmOpcode::Else => {
                    let block_ref = block_stack.last().ok_or(WasmDecodeError::ElseWithoutIf)?;
                    let mut block = blocks.get(*block_ref).unwrap().borrow_mut();
                    if block.inst_type != BlockInstType::If {
                        return Err(WasmDecodeError::ElseWithoutIf);
                    }
                    block.else_position = position;
                    let n_drops = value_stack.len() - block.stack_level;
                    for _ in 0..n_drops {
                        value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    }
                }
                WasmOpcode::End => {
                    if block_stack.len() > 0 {
                        let block_ref = block_stack.pop().ok_or(WasmDecodeError::BlockMismatch)?;
                        let mut block = blocks.get(block_ref).unwrap().borrow_mut();
                        block.end_position = code_block.position();
                        let n_drops = value_stack.len() - block.stack_level;
                        for _ in 0..n_drops {
                            value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                        }
                        block.block_type.into_type().first().map(|v| {
                            value_stack.push(v.clone());
                        });
                    // TODO: type check
                    } else {
                        break;
                    }
                }

                WasmOpcode::Br => {
                    let _br = code_block.read_uint()?;
                }
                WasmOpcode::BrIf => {
                    let _br = code_block.read_uint()?;
                    let cc = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if cc != WasmValType::I32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }
                WasmOpcode::BrTable => {
                    let table_len = code_block.read_uint()?;
                    for _ in 0..table_len {
                        let _br = code_block.read_uint()?;
                    }
                    let _br = code_block.read_uint()?;
                }

                WasmOpcode::Return => {
                    // TODO: type check
                }

                WasmOpcode::Call | WasmOpcode::ReturnCall => {
                    let func_index = code_block.read_uint()? as usize;
                    let function = module
                        .func_by_ref(func_index)
                        .map_err(|_| WasmDecodeError::InvalidParameter)?;
                    let func_type = module
                        .type_by_ref(function.type_ref)
                        .ok_or(WasmDecodeError::InvalidParameter)?;
                    for _param in func_type.param_types() {
                        value_stack.pop();
                    }
                    for result in func_type.result_types() {
                        value_stack.push(result.clone());
                    }
                }
                WasmOpcode::CallIndirect | WasmOpcode::ReturnCallIndirect => {
                    let _br = code_block.read_uint()?;
                    let cc = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if cc != WasmValType::I32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }

                WasmOpcode::Drop => {
                    value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                }
                WasmOpcode::Select => {
                    let cc = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    let b = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != b || cc != WasmValType::I32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(a);
                }

                WasmOpcode::LocalGet => {
                    let local_ref = code_block.read_uint()? as usize;
                    let val = *local_types
                        .get(local_ref)
                        .ok_or(WasmDecodeError::InvalidLocal)?;
                    value_stack.push(val);
                }
                WasmOpcode::LocalSet => {
                    let local_ref = code_block.read_uint()? as usize;
                    let val = *local_types
                        .get(local_ref)
                        .ok_or(WasmDecodeError::InvalidLocal)?;
                    let stack = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if stack != val {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }
                WasmOpcode::LocalTee => {
                    let local_ref = code_block.read_uint()? as usize;
                    let val = *local_types
                        .get(local_ref)
                        .ok_or(WasmDecodeError::InvalidLocal)?;
                    let stack = *value_stack.last().ok_or(WasmDecodeError::OutOfStack)?;
                    if stack != val {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }

                WasmOpcode::GlobalGet => {
                    let global_ref = code_block.read_uint()? as usize;
                    let global = module
                        .globals
                        .get(global_ref)
                        .ok_or(WasmDecodeError::InvalidGlobal)?;
                    value_stack.push(global.val_type);
                }
                WasmOpcode::GlobalSet => {
                    let global_ref = code_block.read_uint()? as usize;
                    let global = module
                        .globals
                        .get(global_ref)
                        .ok_or(WasmDecodeError::InvalidGlobal)?;
                    let stack = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if stack != global.val_type {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }

                WasmOpcode::I32Load
                | WasmOpcode::I32Load8S
                | WasmOpcode::I32Load8U
                | WasmOpcode::I32Load16S
                | WasmOpcode::I32Load16U => {
                    let _ = code_block.read_memarg()?;
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::I32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::I32);
                }

                WasmOpcode::I64Load
                | WasmOpcode::I64Load8S
                | WasmOpcode::I64Load8U
                | WasmOpcode::I64Load16S
                | WasmOpcode::I64Load16U
                | WasmOpcode::I64Load32S
                | WasmOpcode::I64Load32U => {
                    let _ = code_block.read_memarg()?;
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::I32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::I64);
                }

                WasmOpcode::I32Store | WasmOpcode::I32Store8 | WasmOpcode::I32Store16 => {
                    let _ = code_block.read_memarg()?;
                    let d = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    let i = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if i != d && i != WasmValType::I32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }
                WasmOpcode::I64Store
                | WasmOpcode::I64Store8
                | WasmOpcode::I64Store16
                | WasmOpcode::I64Store32 => {
                    let _ = code_block.read_memarg()?;
                    let d = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    let i = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if i != WasmValType::I32 && d != WasmValType::I64 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }

                WasmOpcode::F32Load => {
                    let _ = code_block.read_memarg()?;
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::I32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::F32);
                }
                WasmOpcode::F64Load => {
                    let _ = code_block.read_memarg()?;
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::I32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::F64);
                }
                WasmOpcode::F32Store => {
                    let _ = code_block.read_memarg()?;
                    let d = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    let i = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if i != WasmValType::I32 && d != WasmValType::F32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }
                WasmOpcode::F64Store => {
                    let _ = code_block.read_memarg()?;
                    let d = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    let i = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if i != WasmValType::I32 && d != WasmValType::F64 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }

                WasmOpcode::MemorySize => {
                    let _ = code_block.read_uint()?;
                    value_stack.push(WasmValType::I32);
                }

                WasmOpcode::MemoryGrow => {
                    let _ = code_block.read_uint()?;
                    let a = *value_stack.last().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::I32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }

                WasmOpcode::I32Const => {
                    let _ = code_block.read_sint()?;
                    value_stack.push(WasmValType::I32);
                }
                WasmOpcode::I64Const => {
                    let _ = code_block.read_sint()?;
                    value_stack.push(WasmValType::I64);
                }

                // [i32] -> [i32]
                WasmOpcode::I32Eqz
                | WasmOpcode::I32Clz
                | WasmOpcode::I32Ctz
                | WasmOpcode::I32Popcnt
                | WasmOpcode::I32Extend8S
                | WasmOpcode::I32Extend16S => {
                    let a = *value_stack.last().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::I32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }

                // [i32, i32] -> [i32]
                WasmOpcode::I32Eq
                | WasmOpcode::I32Ne
                | WasmOpcode::I32LtS
                | WasmOpcode::I32LtU
                | WasmOpcode::I32GtS
                | WasmOpcode::I32GtU
                | WasmOpcode::I32LeS
                | WasmOpcode::I32LeU
                | WasmOpcode::I32GeS
                | WasmOpcode::I32GeU
                | WasmOpcode::I32Add
                | WasmOpcode::I32Sub
                | WasmOpcode::I32Mul
                | WasmOpcode::I32DivS
                | WasmOpcode::I32DivU
                | WasmOpcode::I32RemS
                | WasmOpcode::I32RemU
                | WasmOpcode::I32And
                | WasmOpcode::I32Or
                | WasmOpcode::I32Xor
                | WasmOpcode::I32Shl
                | WasmOpcode::I32ShrS
                | WasmOpcode::I32ShrU
                | WasmOpcode::I32Rotl
                | WasmOpcode::I32Rotr => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    let b = *value_stack.last().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != b || a != WasmValType::I32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }

                // [i64, i64] -> [i32]
                WasmOpcode::I64Eq
                | WasmOpcode::I64Ne
                | WasmOpcode::I64LtS
                | WasmOpcode::I64LtU
                | WasmOpcode::I64GtS
                | WasmOpcode::I64GtU
                | WasmOpcode::I64LeS
                | WasmOpcode::I64LeU
                | WasmOpcode::I64GeS
                | WasmOpcode::I64GeU => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    let b = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != b || a != WasmValType::I64 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::I32);
                }

                // [i64] -> [i64]
                WasmOpcode::I64Clz
                | WasmOpcode::I64Ctz
                | WasmOpcode::I64Popcnt
                | WasmOpcode::I64Extend8S
                | WasmOpcode::I64Extend16S
                | WasmOpcode::I64Extend32S => {
                    let a = *value_stack.last().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::I64 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }

                // [i64, i64] -> [i64]
                WasmOpcode::I64Add
                | WasmOpcode::I64Sub
                | WasmOpcode::I64Mul
                | WasmOpcode::I64DivS
                | WasmOpcode::I64DivU
                | WasmOpcode::I64RemS
                | WasmOpcode::I64RemU
                | WasmOpcode::I64And
                | WasmOpcode::I64Or
                | WasmOpcode::I64Xor
                | WasmOpcode::I64Shl
                | WasmOpcode::I64ShrS
                | WasmOpcode::I64ShrU
                | WasmOpcode::I64Rotl
                | WasmOpcode::I64Rotr => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    let b = *value_stack.last().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != b || a != WasmValType::I64 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }

                // [i64] -> [i32]
                WasmOpcode::I64Eqz | WasmOpcode::I32WrapI64 => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::I64 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::I32);
                }

                // [i32] -> [i64]
                WasmOpcode::I64ExtendI32S | WasmOpcode::I64ExtendI32U => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::I32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::I64);
                }

                // WasmOpcode::F32Const => {}
                // WasmOpcode::F64Const => {}

                // float

                // [f32] -> [i32]
                WasmOpcode::I32TruncF32S
                | WasmOpcode::I32TruncF32U
                | WasmOpcode::I32ReinterpretF32 => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::F32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::I32);
                }

                // [f32, f32] -> [i32]
                WasmOpcode::F32Eq
                | WasmOpcode::F32Ne
                | WasmOpcode::F32Lt
                | WasmOpcode::F32Gt
                | WasmOpcode::F32Le
                | WasmOpcode::F32Ge => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    let b = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != b || a != WasmValType::F32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::I32);
                }

                // [f32] -> [f32]
                WasmOpcode::F32Abs
                | WasmOpcode::F32Neg
                | WasmOpcode::F32Ceil
                | WasmOpcode::F32Floor
                | WasmOpcode::F32Trunc
                | WasmOpcode::F32Nearest
                | WasmOpcode::F32Sqrt => {
                    let a = *value_stack.last().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::I32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }

                // [f32, f32] -> [f32]
                WasmOpcode::F32Add
                | WasmOpcode::F32Sub
                | WasmOpcode::F32Mul
                | WasmOpcode::F32Div
                | WasmOpcode::F32Min
                | WasmOpcode::F32Max
                | WasmOpcode::F32Copysign => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    let b = *value_stack.last().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != b || a != WasmValType::F32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }

                // [f64] -> [i32]
                WasmOpcode::I32TruncF64S | WasmOpcode::I32TruncF64U => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::F64 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::I32);
                }

                // [f64] -> [i64]
                WasmOpcode::I64TruncF32S
                | WasmOpcode::I64TruncF32U
                | WasmOpcode::I64TruncF64S
                | WasmOpcode::I64TruncF64U
                | WasmOpcode::I64ReinterpretF64 => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::F64 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::I32);
                }

                // [f64, f64] -> [i32]
                WasmOpcode::F64Eq
                | WasmOpcode::F64Ne
                | WasmOpcode::F64Lt
                | WasmOpcode::F64Gt
                | WasmOpcode::F64Le
                | WasmOpcode::F64Ge => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    let b = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != b || a != WasmValType::F64 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::I32);
                }

                // [f64] -> [f64]
                WasmOpcode::F64Abs
                | WasmOpcode::F64Neg
                | WasmOpcode::F64Ceil
                | WasmOpcode::F64Floor
                | WasmOpcode::F64Trunc
                | WasmOpcode::F64Nearest
                | WasmOpcode::F64Sqrt => {
                    let a = *value_stack.last().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::F64 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }

                // [f64, f64] -> [f64]
                WasmOpcode::F64Add
                | WasmOpcode::F64Sub
                | WasmOpcode::F64Mul
                | WasmOpcode::F64Div
                | WasmOpcode::F64Min
                | WasmOpcode::F64Max
                | WasmOpcode::F64Copysign => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    let b = *value_stack.last().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != b || a != WasmValType::F64 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                }

                // [i32] -> [f32]
                WasmOpcode::F32ConvertI32S
                | WasmOpcode::F32ConvertI32U
                | WasmOpcode::F32ReinterpretI32 => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::I32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::F32);
                }

                // [i64] -> [f64]
                WasmOpcode::F32ConvertI64S | WasmOpcode::F32ConvertI64U => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::I64 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::F32);
                }

                // [f64] -> [f32]
                WasmOpcode::F32DemoteF64 => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::F64 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::F32);
                }

                // [i32] -> [f64]
                WasmOpcode::F64ConvertI32S | WasmOpcode::F64ConvertI32U => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::I32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::F64);
                }

                // [i64] -> [f64]
                WasmOpcode::F64ConvertI64S
                | WasmOpcode::F64ConvertI64U
                | WasmOpcode::F64ReinterpretI64 => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::I64 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::F64);
                }

                // [f32] -> [f64]
                WasmOpcode::F64PromoteF32 => {
                    let a = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                    if a != WasmValType::F32 {
                        return Err(WasmDecodeError::TypeMismatch);
                    }
                    value_stack.push(WasmValType::F64);
                }

                _ => return Err(WasmDecodeError::UnreachableTrap),
            }

            println!(
                "{}[{}]> {:04x} {:02x} {} {:?} -> {:?}",
                block_stack.len(),
                value_stack.len(),
                position,
                opcode as u8,
                opcode.to_str(),
                old_values,
                value_stack
            );
        }

        if result_types.len() > 0 {
            if result_types.len() != value_stack.len() {
                return Err(WasmDecodeError::TypeMismatch);
            }

            for result_type in result_types {
                let val = value_stack.pop().ok_or(WasmDecodeError::OutOfStack)?;
                if *result_type != val {
                    return Err(WasmDecodeError::TypeMismatch);
                }
            }
        } else {
            if value_stack.len() > 0 {
                return Err(WasmDecodeError::InvalidStackLevel);
            }
        }

        let mut blocks2 = Vec::new();
        for block in blocks {
            blocks2.push(block.borrow().clone());
        }
        let blocks = blocks2;

        Ok(Self { max_stack, blocks })
    }

    pub const fn max_stack(&self) -> usize {
        self.max_stack
    }

    pub fn blocks(&self) -> &[WasmBlockContext] {
        self.blocks.as_slice()
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BlockInstType {
    Block,
    Loop,
    If,
}

#[derive(Debug, Copy, Clone)]
pub struct WasmBlockContext {
    pub inst_type: BlockInstType,
    pub block_type: WasmBlockType,
    pub stack_level: usize,
    pub start_position: usize,
    pub end_position: usize,
    pub else_position: usize,
}

#[cfg(test)]
mod tests {

    #[test]
    fn instantiate() {
        let minimal = [0, 97, 115, 109, 1, 0, 0, 0];
        super::WasmLoader::instantiate(&minimal).unwrap();
    }

    #[test]
    #[should_panic(expected = "BadExecutable")]
    fn instantiate_2() {
        let too_small = [0, 97, 115, 109, 1, 0, 0];
        super::WasmLoader::instantiate(&too_small).unwrap();
    }
    #[test]
    #[should_panic(expected = "UnexpectedEof")]
    fn instantiate_3() {
        let minimal_bad = [0, 97, 115, 109, 1, 0, 0, 0, 1];
        super::WasmLoader::instantiate(&minimal_bad).unwrap();
    }

    #[test]
    fn leb128() {
        let data = [0x7F, 0xFF, 0x00];
        let mut stream = super::Leb128Stream::from_slice(&data);

        stream.reset();
        let test = stream.read_uint().unwrap();
        assert_eq!(test, 127);
        let test = stream.read_uint().unwrap();
        assert_eq!(test, 127);

        stream.reset();
        let test = stream.read_sint().unwrap();
        assert_eq!(test, -1);
        let test = stream.read_sint().unwrap();
        assert_eq!(test, 127);
    }
}
