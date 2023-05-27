pub mod intcode;
pub mod intr;

use self::intcode::*;
use crate::{opcode::*, *};
use alloc::{boxed::Box, vec::Vec};
use bitflags::*;
use core::{cell::RefCell, ops::*};

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
                        let block = RefCell::new(BlockContext {
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
                        let block = RefCell::new(BlockContext {
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
                        let block = RefCell::new(BlockContext {
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
                            .functions()
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
                        if index >= module.memories().len() {
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
                        if index >= module.memories().len() {
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
                        if memory_dst >= module.memories().len() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let memory_src = stream.read_unsigned()? as usize;
                        if memory_src >= module.memories().len() {
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
                        if index >= module.memories().len() {
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
enum BlockInstType {
    Block,
    Loop,
    If,
}

#[derive(Debug, Copy, Clone)]
struct BlockContext {
    pub inst_type: BlockInstType,
    pub block_type: WasmBlockType,
    pub stack_level: usize,
    pub start_position: usize,
    pub end_position: usize,
    #[allow(dead_code)]
    pub else_position: usize,
}

impl BlockContext {
    #[inline]
    pub fn preferred_target(&self) -> usize {
        if self.inst_type == BlockInstType::Loop {
            self.start_position
        } else {
            self.end_position
        }
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
