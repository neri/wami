pub mod intcode;
pub mod intr;

use self::intcode::{ExceptionPosition, WasmImc, WasmIntMnemonic};
use crate::{
    leb128::*, opcode::*, WasmBlockType, WasmDecodeErrorKind, WasmMemArg, WasmModule,
    WasmTypeIndex, WasmValType,
};
use alloc::{boxed::Box, vec::Vec};
use bitflags::*;
use core::{cell::RefCell, ops::*};
use smallvec::SmallVec;

/// WebAssembly code block
pub struct WasmCodeBlock {
    func_index: usize,
    file_position: usize,
    local_types: SmallVec<[WasmValType; 16]>,
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
    pub fn local_types(&self) -> &[WasmValType] {
        self.local_types.as_slice()
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
        reader: &mut Leb128Reader,
        param_types: &[WasmValType],
        result_types: &[WasmValType],
        module: &WasmModule,
    ) -> Result<Self, WasmDecodeErrorKind> {
        let local_types = {
            let n_local_var_types: usize = reader.read()?;
            let mut local_var_types = Vec::with_capacity(n_local_var_types);
            for _ in 0..n_local_var_types {
                let repeat = reader.read_unsigned()?;
                let val = WasmValType::from_u64(reader.read()?)?;
                for _ in 0..repeat {
                    local_var_types.push(val);
                }
            }

            let mut vec = SmallVec::with_capacity(param_types.len() + local_var_types.len());
            vec.extend_from_slice(&param_types);
            vec.extend_from_slice(local_var_types.as_slice());
            vec
        };

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
            let position = ExceptionPosition::new(reader.position());
            let opcode = WasmOpcode::read_from(reader)?;

            // match opcode.proposal_type() {
            //     WasmProposalType::Mvp => {}
            //     WasmProposalType::MvpI64 => {}
            //     WasmProposalType::SignExtend => {}
            //     WasmProposalType::BulkMemoryOperations => {}
            //     #[cfg(not(feature = "float"))]
            //     WasmProposalType::MvpF32 | WasmProposalType::MvpF64 => {}
            //     _ => return Err(WasmDecodeErrorKind::UnsupportedOpCode(opcode)),
            // }

            macro_rules! MEM_LOAD {
                ($val_type:ident, $mnemonic:ident, $opcode:ident, $module:ident, $reader:ident, $position:ident, $int_codes:ident, $value_stack:ident,) => {
                    #[cfg(test)]
                    assert_eq!($opcode, WasmSingleOpcode::$mnemonic);

                    if !$module.has_memory() {
                        return Err(WasmDecodeErrorKind::OutOfMemory);
                    }
                    let arg: WasmMemArg = $reader.read()?;
                    let a = $value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    if a != WasmValType::I32 {
                        return Err(WasmDecodeErrorKind::TypeMismatch);
                    }
                    $int_codes.push(WasmImc::new(
                        WasmIntMnemonic::$mnemonic(arg.offset, $position).normalized(),
                        $value_stack.len().into(),
                    ));
                    $value_stack.push(WasmValType::$val_type);
                };
            }
            macro_rules! MEM_STORE {
                ($val_type:ident, $mnemonic:ident, $opcode:ident, $module:ident, $reader:ident, $position:ident, $int_codes:ident, $value_stack:ident,) => {
                    #[cfg(test)]
                    assert_eq!($opcode, WasmSingleOpcode::$mnemonic);

                    if !$module.has_memory() {
                        return Err(WasmDecodeErrorKind::OutOfMemory);
                    }
                    let arg: WasmMemArg = $reader.read()?;
                    let d = $value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    let i = $value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    if i != WasmValType::I32 && d != WasmValType::$val_type {
                        return Err(WasmDecodeErrorKind::TypeMismatch);
                    }
                    $int_codes.push(WasmImc::new(
                        WasmIntMnemonic::$mnemonic(arg.offset, $position).normalized(),
                        $value_stack.len().into(),
                    ));
                };
            }
            macro_rules! UNARY {
                ($val_type:ident, $mnemonic:ident, $opcode:ident, $position:ident, $int_codes:ident, $value_stack:ident,) => {
                    #[cfg(test)]
                    assert_eq!($opcode, WasmSingleOpcode::$mnemonic);

                    let a = *$value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    if a != WasmValType::$val_type {
                        return Err(WasmDecodeErrorKind::TypeMismatch);
                    }
                    $int_codes.push(WasmImc::new(
                        WasmIntMnemonic::$mnemonic,
                        StackLevel($value_stack.len() - 1),
                    ));
                };
            }
            macro_rules! UNARY2 {
                ($in_type:ident, $out_type:ident, $namespace:ident, $mnemonic:ident, $opcode:ident, $position:ident, $int_codes:ident, $value_stack:ident,) => {
                    #[cfg(test)]
                    assert_eq!($opcode, $namespace::$mnemonic);

                    let a = $value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    if a != WasmValType::$in_type {
                        return Err(WasmDecodeErrorKind::TypeMismatch);
                    }
                    $int_codes.push(WasmImc::new(
                        WasmIntMnemonic::$mnemonic,
                        $value_stack.len().into(),
                    ));
                    $value_stack.push(WasmValType::$out_type);
                };
                ($in_type:ident, $out_type:ident, $mnemonic:ident, $opcode:ident, $position:ident, $int_codes:ident, $value_stack:ident,) => {
                    UNARY2!(
                        $in_type,
                        $out_type,
                        WasmSingleOpcode,
                        $mnemonic,
                        $opcode,
                        $position,
                        $int_codes,
                        $value_stack,
                    )
                };
            }
            macro_rules! BIN_CMP {
                ($val_type:ident, $mnemonic:ident, $opcode:ident, $position:ident, $int_codes:ident, $value_stack:ident,) => {
                    #[cfg(test)]
                    assert_eq!($opcode, WasmSingleOpcode::$mnemonic);

                    let a = $value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    let b = $value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    if a != b || a != WasmValType::$val_type {
                        return Err(WasmDecodeErrorKind::TypeMismatch);
                    }
                    $int_codes.push(WasmImc::new(
                        WasmIntMnemonic::$mnemonic,
                        $value_stack.len().into(),
                    ));
                    $value_stack.push(WasmValType::I32);
                };
            }
            macro_rules! BIN_OP {
                ($val_type:ident, $mnemonic:ident, $opcode:ident, $position:ident, $int_codes:ident, $value_stack:ident,) => {
                    #[cfg(test)]
                    assert_eq!($opcode, WasmSingleOpcode::$mnemonic);

                    let a = $value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    let b = *$value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    if a != b || a != WasmValType::$val_type {
                        return Err(WasmDecodeErrorKind::TypeMismatch);
                    }
                    $int_codes.push(WasmImc::new(
                        WasmIntMnemonic::$mnemonic,
                        StackLevel($value_stack.len() - 1),
                    ));
                };
            }
            macro_rules! BIN_DIV {
                ($val_type:ident, $mnemonic:ident, $opcode:ident, $position:ident, $int_codes:ident, $value_stack:ident,) => {
                    #[cfg(test)]
                    assert_eq!($opcode, WasmSingleOpcode::$mnemonic);

                    let a = $value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    let b = *$value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    if a != b || a != WasmValType::$val_type {
                        return Err(WasmDecodeErrorKind::TypeMismatch);
                    }
                    $int_codes.push(WasmImc::new(
                        WasmIntMnemonic::$mnemonic($position),
                        StackLevel($value_stack.len() - 1),
                    ));
                };
            }

            match opcode {
                WasmOpcode::Single(opcode) => match opcode {
                    WasmSingleOpcode::Unreachable => {
                        int_codes.push(WasmImc::new(
                            WasmIntMnemonic::Unreachable(position),
                            value_stack.len().into(),
                        ));
                    }

                    WasmSingleOpcode::Nop => (),

                    WasmSingleOpcode::Block => {
                        let target = blocks.len();
                        let block_type = WasmBlockType::from_i64(reader.read()?)?;
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
                                WasmIntMnemonic::Block(target),
                                value_stack.len().into(),
                            ));
                        } else {
                            // TODO:
                            int_codes.push(WasmImc::new(
                                WasmIntMnemonic::Undefined(opcode.into(), position),
                                value_stack.len().into(),
                            ));
                        }
                    }
                    WasmSingleOpcode::Loop => {
                        let target = blocks.len();
                        let block_type = WasmBlockType::from_i64(reader.read()?)?;
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
                                WasmIntMnemonic::Block(target),
                                value_stack.len().into(),
                            ));
                        } else {
                            // TODO:
                            int_codes.push(WasmImc::new(
                                WasmIntMnemonic::Undefined(opcode.into(), position),
                                value_stack.len().into(),
                            ));
                        }
                    }
                    WasmSingleOpcode::If => {
                        let cc = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if cc != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        let block_type = WasmBlockType::from_i64(reader.read()?)?;
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
                            WasmIntMnemonic::Undefined(opcode.into(), position),
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
                            WasmIntMnemonic::Undefined(opcode.into(), position),
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
                                WasmIntMnemonic::End(block_ref),
                                value_stack.len().into(),
                            ));
                        } else {
                            if let Some(result_type) = result_types.first() {
                                let result_type2 =
                                    value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                                if *result_type != result_type2 {
                                    return Err(WasmDecodeErrorKind::TypeMismatch);
                                }
                                match result_type {
                                    WasmValType::I32 | WasmValType::I64 => {
                                        int_codes.push(WasmImc::new(
                                            WasmIntMnemonic::ReturnI,
                                            StackLevel(value_stack.len()),
                                        ))
                                    }
                                    WasmValType::F32 | WasmValType::F64 => {
                                        int_codes.push(WasmImc::new(
                                            WasmIntMnemonic::ReturnF,
                                            StackLevel(value_stack.len()),
                                        ))
                                    }
                                }
                            } else {
                                int_codes.push(WasmImc::new(
                                    WasmIntMnemonic::ReturnN,
                                    StackLevel(value_stack.len() - 1),
                                ));
                            }
                            break;
                        }
                    }

                    WasmSingleOpcode::Br => {
                        let br = reader.read_unsigned()? as usize;
                        let target = block_stack
                            .get(block_stack.len() - br - 1)
                            .ok_or(WasmDecodeErrorKind::OutOfBranch)?;
                        int_codes.push(WasmImc::new(
                            WasmIntMnemonic::Br(*target),
                            value_stack.len().into(),
                        ));
                    }
                    WasmSingleOpcode::BrIf => {
                        let br = reader.read_unsigned()? as usize;
                        let target = block_stack
                            .get(block_stack.len() - br - 1)
                            .ok_or(WasmDecodeErrorKind::OutOfBranch)?;
                        let cc = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if cc != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            WasmIntMnemonic::BrIf(*target),
                            value_stack.len().into(),
                        ));
                    }
                    WasmSingleOpcode::BrTable => {
                        let table_len = 1 + reader.read_unsigned()? as usize;
                        let mut table = Vec::with_capacity(table_len);
                        for _ in 0..table_len {
                            let br = reader.read_unsigned()? as usize;
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
                            WasmIntMnemonic::BrTable(table.into_boxed_slice()),
                            value_stack.len().into(),
                        ));
                    }

                    WasmSingleOpcode::Return => {
                        if let Some(result_type) = result_types.first() {
                            let result_type2 =
                                value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                            if *result_type != result_type2 {
                                return Err(WasmDecodeErrorKind::TypeMismatch);
                            }
                            match result_type {
                                WasmValType::I32 | WasmValType::I64 => {
                                    int_codes.push(WasmImc::new(
                                        WasmIntMnemonic::ReturnI,
                                        StackLevel(value_stack.len()),
                                    ))
                                }
                                WasmValType::F32 | WasmValType::F64 => {
                                    int_codes.push(WasmImc::new(
                                        WasmIntMnemonic::ReturnF,
                                        StackLevel(value_stack.len()),
                                    ))
                                }
                            }
                        } else {
                            int_codes.push(WasmImc::new(
                                WasmIntMnemonic::ReturnN,
                                StackLevel(value_stack.len() - 1),
                            ));
                        }
                    }

                    WasmSingleOpcode::Call => {
                        flags.remove(WasmBlockFlag::LEAF_FUNCTION);
                        let func_index = reader.read_unsigned()? as usize;
                        let function = module
                            .functions()
                            .get(func_index)
                            .ok_or(WasmDecodeErrorKind::InvalidData)?;
                        int_codes.push(WasmImc::new(
                            WasmIntMnemonic::Call(func_index, position),
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
                        let type_index = WasmTypeIndex(reader.read()?);
                        let _reserved = reader.read_unsigned()?;
                        let func_type = module
                            .type_by_ref(type_index)
                            .ok_or(WasmDecodeErrorKind::InvalidData)?;
                        let index = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if index != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            WasmIntMnemonic::CallIndirect(type_index, position),
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
                            WasmIntMnemonic::SelectI,
                            value_stack.len().into(),
                        ));
                        value_stack.push(a);
                    }

                    WasmSingleOpcode::LocalGet => {
                        let local_ref = reader.read_unsigned()? as usize;
                        let val = *local_types
                            .get(local_ref)
                            .ok_or(WasmDecodeErrorKind::InvalidLocal)?;
                        int_codes.push(WasmImc::new(
                            WasmIntMnemonic::LocalGetI(LocalVarIndex::new(local_ref)),
                            value_stack.len().into(),
                        ));
                        value_stack.push(val);
                    }
                    WasmSingleOpcode::LocalSet => {
                        let local_ref = reader.read_unsigned()? as usize;
                        let val = *local_types
                            .get(local_ref)
                            .ok_or(WasmDecodeErrorKind::InvalidLocal)?;
                        let stack = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if stack != val {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            WasmIntMnemonic::LocalSetI(LocalVarIndex::new(local_ref)),
                            value_stack.len().into(),
                        ));
                    }
                    WasmSingleOpcode::LocalTee => {
                        let local_ref = reader.read_unsigned()? as usize;
                        let val = *local_types
                            .get(local_ref)
                            .ok_or(WasmDecodeErrorKind::InvalidLocal)?;
                        let stack = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if stack != val {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        int_codes.push(WasmImc::new(
                            WasmIntMnemonic::LocalTeeI(LocalVarIndex::new(local_ref)),
                            StackLevel(value_stack.len() - 1),
                        ));
                    }

                    WasmSingleOpcode::GlobalGet => {
                        let global_ref = reader.read_unsigned()? as usize;
                        let val_type = module
                            .globals()
                            .get(global_ref)
                            .map(|v| v.val_type())
                            .ok_or(WasmDecodeErrorKind::InvalidGlobal)?;
                        int_codes.push(WasmImc::new(
                            WasmIntMnemonic::GlobalGetI(GlobalVarIndex(global_ref)),
                            value_stack.len().into(),
                        ));
                        value_stack.push(val_type);
                    }
                    WasmSingleOpcode::GlobalSet => {
                        let global_ref = reader.read_unsigned()? as usize;
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
                            WasmIntMnemonic::GlobalSetI(GlobalVarIndex(global_ref)),
                            value_stack.len().into(),
                        ));
                    }

                    WasmSingleOpcode::I32Load => {
                        #[rustfmt::skip]
                        MEM_LOAD!(I32, I32Load, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::I32Load8S => {
                        #[rustfmt::skip]
                        MEM_LOAD!(I32, I32Load8S, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::I32Load8U => {
                        #[rustfmt::skip]
                        MEM_LOAD!(I32, I32Load8U, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::I32Load16S => {
                        #[rustfmt::skip]
                        MEM_LOAD!(I32, I32Load16S, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::I32Load16U => {
                        #[rustfmt::skip]
                        MEM_LOAD!(I32, I32Load16U, opcode, module, reader, position, int_codes, value_stack, );
                    }

                    WasmSingleOpcode::I64Load => {
                        #[rustfmt::skip]
                        MEM_LOAD!(I64, I64Load, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::I64Load8S => {
                        #[rustfmt::skip]
                        MEM_LOAD!(I64, I64Load8S, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::I64Load8U => {
                        #[rustfmt::skip]
                        MEM_LOAD!(I64, I64Load8U, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::I64Load16S => {
                        #[rustfmt::skip]
                        MEM_LOAD!(I64, I64Load16S, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::I64Load16U => {
                        #[rustfmt::skip]
                        MEM_LOAD!(I64, I64Load16U, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::I64Load32S => {
                        #[rustfmt::skip]
                        MEM_LOAD!(I64, I64Load32S, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::I64Load32U => {
                        #[rustfmt::skip]
                        MEM_LOAD!(I64, I64Load32U, opcode, module, reader, position, int_codes, value_stack, );
                    }

                    WasmSingleOpcode::I32Store => {
                        #[rustfmt::skip]
                        MEM_STORE!(I32, I32Store, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::I32Store8 => {
                        #[rustfmt::skip]
                        MEM_STORE!(I32, I32Store8, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::I32Store16 => {
                        #[rustfmt::skip]
                        MEM_STORE!(I32, I32Store16, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::I64Store => {
                        #[rustfmt::skip]
                        MEM_STORE!(I64, I64Store, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::I64Store8 => {
                        #[rustfmt::skip]
                        MEM_STORE!(I64, I64Store8, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::I64Store16 => {
                        #[rustfmt::skip]
                        MEM_STORE!(I64, I64Store16, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::I64Store32 => {
                        #[rustfmt::skip]
                        MEM_STORE!(I64, I64Store32, opcode, module, reader, position, int_codes, value_stack, );
                    }

                    WasmSingleOpcode::F32Load => {
                        #[rustfmt::skip]
                        MEM_LOAD!(F32, F32Load, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::F64Load => {
                        #[rustfmt::skip]
                        MEM_LOAD!(F64, F64Load, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::F32Store => {
                        #[rustfmt::skip]
                        MEM_STORE!(F32, F32Store, opcode, module, reader, position, int_codes, value_stack, );
                    }
                    WasmSingleOpcode::F64Store => {
                        #[rustfmt::skip]
                        MEM_STORE!(F64, F64Store, opcode, module, reader, position, int_codes, value_stack, );
                    }

                    WasmSingleOpcode::MemorySize => {
                        let index = reader.read_unsigned()? as usize;
                        if index >= module.memories().len() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        int_codes.push(WasmImc::new(
                            WasmIntMnemonic::MemorySize,
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }

                    WasmSingleOpcode::MemoryGrow => {
                        let index = reader.read_unsigned()? as usize;
                        if index >= module.memories().len() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        int_codes.push(WasmImc::new(
                            WasmIntMnemonic::MemoryGrow,
                            StackLevel(value_stack.len() - 1),
                        ));
                        let a = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if a != WasmValType::I32 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                    }

                    WasmSingleOpcode::I32Const => {
                        let val: i32 = reader.read()?;
                        int_codes.push(WasmImc::new(
                            WasmIntMnemonic::I32Const(val),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I32);
                    }
                    WasmSingleOpcode::I64Const => {
                        let val: i64 = reader.read()?;
                        int_codes.push(WasmImc::new(
                            WasmIntMnemonic::I64Const(val),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::I64);
                    }
                    WasmSingleOpcode::F32Const => {
                        let val = reader.read_f32()?;
                        int_codes.push(WasmImc::new(
                            WasmIntMnemonic::F32Const(val),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::F32);
                    }
                    WasmSingleOpcode::F64Const => {
                        let val = reader.read_f64()?;
                        int_codes.push(WasmImc::new(
                            WasmIntMnemonic::F64Const(val),
                            value_stack.len().into(),
                        ));
                        value_stack.push(WasmValType::F64);
                    }

                    // unary operator [i32] -> [i32]
                    WasmSingleOpcode::I32Eqz => {
                        #[rustfmt::skip]
                        UNARY!(I32, I32Eqz, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32Clz => {
                        #[rustfmt::skip]
                        UNARY!(I32, I32Clz, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32Ctz => {
                        #[rustfmt::skip]
                        UNARY!(I32, I32Ctz, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32Popcnt => {
                        #[rustfmt::skip]
                        UNARY!(I32, I32Popcnt, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32Extend8S => {
                        #[rustfmt::skip]
                        UNARY!(I32, I32Extend8S, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32Extend16S => {
                        #[rustfmt::skip]
                        UNARY!(I32, I32Extend16S, opcode, position, int_codes, value_stack,);
                    }

                    // binary operator [i32, i32] -> [i32]
                    WasmSingleOpcode::I32Eq => {
                        #[rustfmt::skip]
                        BIN_CMP!(I32, I32Eq, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32Ne => {
                        #[rustfmt::skip]
                        BIN_CMP!(I32, I32Ne, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32LtS => {
                        #[rustfmt::skip]
                        BIN_CMP!(I32, I32LtS, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32LtU => {
                        #[rustfmt::skip]
                        BIN_CMP!(I32, I32LtU, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32GtS => {
                        #[rustfmt::skip]
                        BIN_CMP!(I32, I32GtS, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32GtU => {
                        #[rustfmt::skip]
                        BIN_CMP!(I32, I32GtU, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32LeS => {
                        #[rustfmt::skip]
                        BIN_CMP!(I32, I32LeS, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32LeU => {
                        #[rustfmt::skip]
                        BIN_CMP!(I32, I32LeU, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32GeS => {
                        #[rustfmt::skip]
                        BIN_CMP!(I32, I32GeS, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32GeU => {
                        #[rustfmt::skip]
                        BIN_CMP!(I32, I32GeU, opcode, position, int_codes, value_stack,);
                    }

                    WasmSingleOpcode::I32Add => {
                        #[rustfmt::skip]
                        BIN_OP!(I32, I32Add, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32Sub => {
                        #[rustfmt::skip]
                        BIN_OP!(I32, I32Sub, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32Mul => {
                        #[rustfmt::skip]
                        BIN_OP!(I32, I32Mul, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32DivS => {
                        #[rustfmt::skip]
                        BIN_DIV!(I32, I32DivS, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32DivU => {
                        #[rustfmt::skip]
                        BIN_DIV!(I32, I32DivU, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32RemS => {
                        #[rustfmt::skip]
                        BIN_DIV!(I32, I32RemS, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32RemU => {
                        #[rustfmt::skip]
                        BIN_DIV!(I32, I32RemU, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32And => {
                        #[rustfmt::skip]
                        BIN_OP!(I32, I32And, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32Or => {
                        #[rustfmt::skip]
                        BIN_OP!(I32, I32Or, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32Xor => {
                        #[rustfmt::skip]
                        BIN_OP!(I32, I32Xor, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32Shl => {
                        #[rustfmt::skip]
                        BIN_OP!(I32, I32Shl, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32ShrS => {
                        #[rustfmt::skip]
                        BIN_OP!(I32, I32ShrS, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32ShrU => {
                        #[rustfmt::skip]
                        BIN_OP!(I32, I32ShrU, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32Rotl => {
                        #[rustfmt::skip]
                        BIN_OP!(I32, I32Rotl, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32Rotr => {
                        #[rustfmt::skip]
                        BIN_OP!(I32, I32Rotr, opcode, position, int_codes, value_stack,);
                    }

                    // binary operator [i64, i64] -> [i32]
                    WasmSingleOpcode::I64Eq => {
                        #[rustfmt::skip]
                        BIN_CMP!(I64, I64Eq, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64Ne => {
                        #[rustfmt::skip]
                        BIN_CMP!(I64, I64Ne, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64LtS => {
                        #[rustfmt::skip]
                        BIN_CMP!(I64, I64LtS, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64LtU => {
                        #[rustfmt::skip]
                        BIN_CMP!(I64, I64LtU, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64GtS => {
                        #[rustfmt::skip]
                        BIN_CMP!(I64, I64GtS, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64GtU => {
                        #[rustfmt::skip]
                        BIN_CMP!(I64, I64GtU, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64LeS => {
                        #[rustfmt::skip]
                        BIN_CMP!(I64, I64LeS, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64LeU => {
                        #[rustfmt::skip]
                        BIN_CMP!(I64, I64LeU, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64GeS => {
                        #[rustfmt::skip]
                        BIN_CMP!(I64, I64GeS, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64GeU => {
                        #[rustfmt::skip]
                        BIN_CMP!(I64, I64GeU, opcode, position, int_codes, value_stack,);
                    }

                    // unary operator [i64] -> [i64]
                    WasmSingleOpcode::I64Clz => {
                        #[rustfmt::skip]
                        UNARY!(I64, I64Clz, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64Ctz => {
                        #[rustfmt::skip]
                        UNARY!(I64, I64Ctz, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64Popcnt => {
                        #[rustfmt::skip]
                        UNARY!(I64, I64Popcnt, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64Extend8S => {
                        #[rustfmt::skip]
                        UNARY!(I64, I64Extend8S, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64Extend16S => {
                        #[rustfmt::skip]
                        UNARY!(I64, I64Extend16S, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64Extend32S => {
                        #[rustfmt::skip]
                        UNARY!(I64, I64Extend32S, opcode, position, int_codes, value_stack,);
                    }

                    // binary operator [i64, i64] -> [i64]
                    WasmSingleOpcode::I64Add => {
                        #[rustfmt::skip]
                        BIN_OP!(I64, I64Add, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64Sub => {
                        #[rustfmt::skip]
                        BIN_OP!(I64, I64Sub, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64Mul => {
                        #[rustfmt::skip]
                        BIN_OP!(I64, I64Mul, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64DivS => {
                        #[rustfmt::skip]
                        BIN_DIV!(I64, I64DivS, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64DivU => {
                        #[rustfmt::skip]
                        BIN_DIV!(I64, I64DivU, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64RemS => {
                        #[rustfmt::skip]
                        BIN_DIV!(I64, I64RemS, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64RemU => {
                        #[rustfmt::skip]
                        BIN_DIV!(I64, I64RemU, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64And => {
                        #[rustfmt::skip]
                        BIN_OP!(I64, I64And, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64Or => {
                        #[rustfmt::skip]
                        BIN_OP!(I64, I64Or, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64Xor => {
                        #[rustfmt::skip]
                        BIN_OP!(I64, I64Xor, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64Shl => {
                        #[rustfmt::skip]
                        BIN_OP!(I64, I64Shl, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64ShrS => {
                        #[rustfmt::skip]
                        BIN_OP!(I64, I64ShrS, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64ShrU => {
                        #[rustfmt::skip]
                        BIN_OP!(I64, I64ShrU, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64Rotl => {
                        #[rustfmt::skip]
                        BIN_OP!(I64, I64Rotl, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64Rotr => {
                        #[rustfmt::skip]
                        BIN_OP!(I64, I64Rotr, opcode, position, int_codes, value_stack,);
                    }

                    // [i64] -> [i32]
                    WasmSingleOpcode::I64Eqz => {
                        #[rustfmt::skip]
                        UNARY2!(I64, I32, I64Eqz, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32WrapI64 => {
                        #[rustfmt::skip]
                        UNARY2!(I64, I32, I32WrapI64, opcode, position, int_codes, value_stack,);
                    }

                    // [i32] -> [i64]
                    WasmSingleOpcode::I64ExtendI32S => {
                        #[rustfmt::skip]
                        UNARY2!(I32, I64, I64ExtendI32S, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64ExtendI32U => {
                        #[rustfmt::skip]
                        UNARY2!(I32, I64, I64ExtendI32U, opcode, position, int_codes, value_stack,);
                    }

                    // [f32, f32] -> [i32]
                    WasmSingleOpcode::F32Eq => {
                        #[rustfmt::skip]
                        BIN_CMP!(F32, F32Eq, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Ne => {
                        #[rustfmt::skip]
                        BIN_CMP!(F32, F32Ne, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Lt => {
                        #[rustfmt::skip]
                        BIN_CMP!(F32, F32Lt, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Gt => {
                        #[rustfmt::skip]
                        BIN_CMP!(F32, F32Gt, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Le => {
                        #[rustfmt::skip]
                        BIN_CMP!(F32, F32Le, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Ge => {
                        #[rustfmt::skip]
                        BIN_CMP!(F32, F32Ge, opcode, position, int_codes, value_stack,);
                    }

                    // [f32] -> [f32]
                    WasmSingleOpcode::F32Abs => {
                        #[rustfmt::skip]
                        UNARY!(F32, F32Abs, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Neg => {
                        #[rustfmt::skip]
                        UNARY!(F32, F32Neg, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Ceil => {
                        #[rustfmt::skip]
                        UNARY!(F32, F32Ceil, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Floor => {
                        #[rustfmt::skip]
                        UNARY!(F32, F32Floor, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Trunc => {
                        #[rustfmt::skip]
                        UNARY!(F32, F32Trunc, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Nearest => {
                        #[rustfmt::skip]
                        UNARY!(F32, F32Nearest, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Sqrt => {
                        #[rustfmt::skip]
                        UNARY!(F32, F32Sqrt, opcode, position, int_codes, value_stack,);
                    }

                    // [f32, f32] -> [f32]
                    WasmSingleOpcode::F32Add => {
                        #[rustfmt::skip]
                        BIN_OP!(F32, F32Add, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Sub => {
                        #[rustfmt::skip]
                        BIN_OP!(F32, F32Sub, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Mul => {
                        #[rustfmt::skip]
                        BIN_OP!(F32, F32Mul, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Div => {
                        #[rustfmt::skip]
                        BIN_OP!(F32, F32Div, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Min => {
                        #[rustfmt::skip]
                        BIN_OP!(F32, F32Min, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Max => {
                        #[rustfmt::skip]
                        BIN_OP!(F32, F32Max, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32Copysign => {
                        #[rustfmt::skip]
                        BIN_OP!(F32, F32Copysign, opcode, position, int_codes, value_stack,);
                    }

                    // [f64, f64] -> [i32]
                    WasmSingleOpcode::F64Eq => {
                        #[rustfmt::skip]
                        BIN_CMP!(F64, F64Eq, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Ne => {
                        #[rustfmt::skip]
                        BIN_CMP!(F64, F64Ne, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Lt => {
                        #[rustfmt::skip]
                        BIN_CMP!(F64, F64Lt, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Gt => {
                        #[rustfmt::skip]
                        BIN_CMP!(F64, F64Gt, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Le => {
                        #[rustfmt::skip]
                        BIN_CMP!(F64, F64Le, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Ge => {
                        #[rustfmt::skip]
                        BIN_CMP!(F64, F64Ge, opcode, position, int_codes, value_stack,);
                    }

                    // [f64] -> [f64]
                    WasmSingleOpcode::F64Abs => {
                        #[rustfmt::skip]
                        UNARY!(F64, F64Abs, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Neg => {
                        #[rustfmt::skip]
                        UNARY!(F64, F64Neg, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Ceil => {
                        #[rustfmt::skip]
                        UNARY!(F64, F64Ceil, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Floor => {
                        #[rustfmt::skip]
                        UNARY!(F64, F64Floor, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Trunc => {
                        #[rustfmt::skip]
                        UNARY!(F64, F64Trunc, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Nearest => {
                        #[rustfmt::skip]
                        UNARY!(F64, F64Nearest, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Sqrt => {
                        #[rustfmt::skip]
                        UNARY!(F64, F64Sqrt, opcode, position, int_codes, value_stack,);
                    }

                    // [f64, f64] -> [f64]
                    WasmSingleOpcode::F64Add => {
                        #[rustfmt::skip]
                        BIN_OP!(F64, F64Add, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Sub => {
                        #[rustfmt::skip]
                        BIN_OP!(F64, F64Sub, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Mul => {
                        #[rustfmt::skip]
                        BIN_OP!(F64, F64Mul, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Div => {
                        #[rustfmt::skip]
                        BIN_OP!(F64, F64Div, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Min => {
                        #[rustfmt::skip]
                        BIN_OP!(F64, F64Min, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Max => {
                        #[rustfmt::skip]
                        BIN_OP!(F64, F64Max, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64Copysign => {
                        #[rustfmt::skip]
                        BIN_OP!(F64, F64Copysign, opcode, position, int_codes, value_stack,);
                    }

                    // [f32] -> [i32]
                    WasmSingleOpcode::I32TruncF32S => {
                        #[rustfmt::skip]
                        UNARY2!(F32, I32, I32TruncF32S, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32TruncF32U => {
                        #[rustfmt::skip]
                        UNARY2!(F32, I32, I32TruncF32U, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32TruncF64S => {
                        #[rustfmt::skip]
                        UNARY2!(F64, I32, I32TruncF64S, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I32TruncF64U => {
                        #[rustfmt::skip]
                        UNARY2!(F64, I32, I32TruncF64U, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64TruncF32S => {
                        #[rustfmt::skip]
                        UNARY2!(F32, I64, I64TruncF32S, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64TruncF32U => {
                        #[rustfmt::skip]
                        UNARY2!(F32, I64, I64TruncF32U, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64TruncF64S => {
                        #[rustfmt::skip]
                        UNARY2!(F64, I64, I64TruncF64S, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64TruncF64U => {
                        #[rustfmt::skip]
                        UNARY2!(F64, I64, I64TruncF64U, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32ConvertI32S => {
                        #[rustfmt::skip]
                        UNARY2!(I32, F32, F32ConvertI32S, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32ConvertI32U => {
                        #[rustfmt::skip]
                        UNARY2!(I32, F32, F32ConvertI32U, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32ConvertI64S => {
                        #[rustfmt::skip]
                        UNARY2!(I64, F32, F32ConvertI64S, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32ConvertI64U => {
                        #[rustfmt::skip]
                        UNARY2!(I64, F32, F32ConvertI64U, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32DemoteF64 => {
                        #[rustfmt::skip]
                        UNARY2!(F64, F32, F32DemoteF64, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64ConvertI32S => {
                        #[rustfmt::skip]
                        UNARY2!(I32, F64, F64ConvertI32S, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64ConvertI32U => {
                        #[rustfmt::skip]
                        UNARY2!(I32, F64, F64ConvertI32U, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64ConvertI64S => {
                        #[rustfmt::skip]
                        UNARY2!(I64, F64, F64ConvertI64S, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64ConvertI64U => {
                        #[rustfmt::skip]
                        UNARY2!(I64, F64, F64ConvertI64U, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64PromoteF32 => {
                        #[rustfmt::skip]
                        UNARY2!(F32, F64, F64PromoteF32, opcode, position, int_codes, value_stack,);
                    }

                    WasmSingleOpcode::I32ReinterpretF32 => {
                        #[rustfmt::skip]
                        UNARY2!(F32, I32, I32ReinterpretF32, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::I64ReinterpretF64 => {
                        #[rustfmt::skip]
                        UNARY2!(F64, I64, I64ReinterpretF64, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F32ReinterpretI32 => {
                        #[rustfmt::skip]
                        UNARY2!(I32, F32, F32ReinterpretI32, opcode, position, int_codes, value_stack,);
                    }
                    WasmSingleOpcode::F64ReinterpretI64 => {
                        #[rustfmt::skip]
                        UNARY2!(I64, F64, F64ReinterpretI64, opcode, position, int_codes, value_stack,);
                    }

                    WasmSingleOpcode::PrefixFC | WasmSingleOpcode::PrefixFD => unreachable!(),

                    _ => return Err(WasmDecodeErrorKind::UnsupportedOpCode(opcode.into())),
                },

                WasmOpcode::PrefixFC(opcode) => match opcode {
                    WasmOpcodeFC::I32TruncSatF32S => {
                        #[rustfmt::skip]
                        UNARY2!(F32, I32, WasmOpcodeFC, I32TruncSatF32S, opcode, position, int_codes, value_stack,);
                    }
                    WasmOpcodeFC::I32TruncSatF32U => {
                        #[rustfmt::skip]
                        UNARY2!(F32, I32, WasmOpcodeFC, I32TruncSatF32U, opcode, position, int_codes, value_stack,);
                    }
                    WasmOpcodeFC::I32TruncSatF64S => {
                        #[rustfmt::skip]
                        UNARY2!(F64, I32, WasmOpcodeFC, I32TruncSatF64S, opcode, position, int_codes, value_stack,);
                    }
                    WasmOpcodeFC::I32TruncSatF64U => {
                        #[rustfmt::skip]
                        UNARY2!(F64, I32, WasmOpcodeFC, I32TruncSatF64U, opcode, position, int_codes, value_stack,);
                    }
                    WasmOpcodeFC::I64TruncSatF32S => {
                        #[rustfmt::skip]
                        UNARY2!(F32, I64, WasmOpcodeFC, I64TruncSatF32S, opcode, position, int_codes, value_stack,);
                    }
                    WasmOpcodeFC::I64TruncSatF32U => {
                        #[rustfmt::skip]
                        UNARY2!(F32, I64, WasmOpcodeFC, I64TruncSatF32U, opcode, position, int_codes, value_stack,);
                    }
                    WasmOpcodeFC::I64TruncSatF64S => {
                        #[rustfmt::skip]
                        UNARY2!(F64, I64, WasmOpcodeFC, I64TruncSatF64S, opcode, position, int_codes, value_stack,);
                    }
                    WasmOpcodeFC::I64TruncSatF64U => {
                        #[rustfmt::skip]
                        UNARY2!(F64, I64, WasmOpcodeFC, I64TruncSatF64U, opcode, position, int_codes, value_stack,);
                    }

                    WasmOpcodeFC::MemoryCopy => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let memory_dst = reader.read_unsigned()? as usize;
                        if memory_dst >= module.memories().len() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let memory_src = reader.read_unsigned()? as usize;
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
                            WasmIntMnemonic::MemoryCopy(position),
                            value_stack.len().into(),
                        ));
                    }

                    WasmOpcodeFC::MemoryFill => {
                        if !module.has_memory() {
                            return Err(WasmDecodeErrorKind::OutOfMemory);
                        }
                        let index = reader.read_unsigned()? as usize;
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
                            WasmIntMnemonic::MemoryFill(position),
                            value_stack.len().into(),
                        ));
                    }

                    WasmOpcodeFC::MemoryInit
                    | WasmOpcodeFC::DataDrop
                    | WasmOpcodeFC::TableInit
                    | WasmOpcodeFC::ElemDrop
                    | WasmOpcodeFC::TableCopy => {
                        return Err(WasmDecodeErrorKind::UnsupportedOpCode(opcode.into()))
                    }
                },

                WasmOpcode::PrefixFD(_) => {
                    return Err(WasmDecodeErrorKind::UnsupportedOpCode(opcode.into()))
                }
            }
        }

        int_codes.push(WasmImc::new(
            WasmIntMnemonic::Unreachable(ExceptionPosition::new(reader.position())),
            value_stack.len().into(),
        ));

        if result_types.len() == 0 {
            if value_stack.len() > 0 {
                return Err(WasmDecodeErrorKind::InvalidStackLevel);
            }
        }

        macro_rules! fused2 {
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
                    (I32Const(val), LocalSetI(local_index)) => {
                        fused2!(int_codes, i, FusedI32SetConst(*local_index, *val));
                    }
                    (I32Const(val), I32Add) => {
                        fused2!(int_codes, i, FusedI32AddI(*val));
                    }
                    (I32Const(val), I32Sub) => {
                        fused2!(int_codes, i, FusedI32SubI(*val));
                    }
                    (I32Const(val), I32And) => {
                        fused2!(int_codes, i, FusedI32AndI(*val as u32));
                    }
                    (I32Const(val), I32Or) => {
                        fused2!(int_codes, i, FusedI32OrI(*val as u32));
                    }
                    (I32Const(val), I32Xor) => {
                        fused2!(int_codes, i, FusedI32XorI(*val as u32));
                    }
                    (I32Const(val), I32Shl) => {
                        fused2!(int_codes, i, FusedI32ShlI(*val as u32));
                    }
                    (I32Const(val), I32ShrS) => {
                        fused2!(int_codes, i, FusedI32ShrSI(*val as u32));
                    }
                    (I32Const(val), I32ShrU) => {
                        fused2!(int_codes, i, FusedI32ShrUI(*val as u32));
                    }

                    (I64Const(val), LocalSetI(local_index)) => {
                        fused2!(int_codes, i, FusedI64SetConst(*local_index, *val));
                    }
                    (I64Const(val), I64Add) => {
                        fused2!(int_codes, i, FusedI64AddI(*val));
                    }
                    (I64Const(val), I64Sub) => {
                        fused2!(int_codes, i, FusedI64SubI(*val));
                    }

                    (I32Eqz, BrIf(target)) => {
                        fused2!(int_codes, i, FusedI32BrZ(*target));
                    }
                    (I32Eq, BrIf(target)) => {
                        fused2!(int_codes, i, FusedI32BrEq(*target));
                    }
                    (I32Ne, BrIf(target)) => {
                        fused2!(int_codes, i, FusedI32BrNe(*target));
                    }
                    (I32LtS, BrIf(target)) => {
                        fused2!(int_codes, i, FusedI32BrLtS(*target));
                    }
                    (I32LtU, BrIf(target)) => {
                        fused2!(int_codes, i, FusedI32BrLtU(*target));
                    }
                    (I32GtS, BrIf(target)) => {
                        fused2!(int_codes, i, FusedI32BrGtS(*target));
                    }
                    (I32GtU, BrIf(target)) => {
                        fused2!(int_codes, i, FusedI32BrGtU(*target));
                    }
                    (I32LeS, BrIf(target)) => {
                        fused2!(int_codes, i, FusedI32BrLeS(*target));
                    }
                    (I32LeU, BrIf(target)) => {
                        fused2!(int_codes, i, FusedI32BrLeU(*target));
                    }
                    (I32GeS, BrIf(target)) => {
                        fused2!(int_codes, i, FusedI32BrGeS(*target));
                    }
                    (I32GeU, BrIf(target)) => {
                        fused2!(int_codes, i, FusedI32BrGeU(*target));
                    }

                    (I64Eqz, BrIf(target)) => {
                        fused2!(int_codes, i, FusedI64BrZ(*target));
                    }
                    (I64Eq, BrIf(target)) => {
                        fused2!(int_codes, i, FusedI64BrEq(*target));
                    }
                    (I64Ne, BrIf(target)) => {
                        fused2!(int_codes, i, FusedI64BrNe(*target));
                    }
                    _ => (),
                }
            }
        }

        // compaction and block adjustment
        let mut compacted = Vec::new();
        for code in int_codes {
            match *code.mnemonic() {
                WasmIntMnemonic::Nop => (),
                WasmIntMnemonic::Block(target) => {
                    let ref mut block = blocks[target].borrow_mut();
                    block.start_position = compacted.len();
                }
                WasmIntMnemonic::End(target) => {
                    let ref mut block = blocks[target].borrow_mut();
                    block.end_position = compacted.len();
                }
                _ => {
                    compacted.push(code);
                }
            }
        }
        compacted.shrink_to_fit();
        let mut int_codes = compacted;

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
            local_types,
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
    pub const fn new(val: usize) -> Self {
        Self(val)
    }

    #[inline]
    pub const fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobalVarIndex(usize);

impl GlobalVarIndex {
    #[inline]
    pub const fn new(val: usize) -> Self {
        Self(val)
    }

    #[inline]
    pub const fn as_usize(&self) -> usize {
        self.0 as usize
    }
}
