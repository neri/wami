pub mod intcode;
pub mod intr;

use self::intcode::{ExceptionPosition, WasmImc, WasmIntMnemonic};
use crate::{bytecode::*, leb128::*, *};
use alloc::{boxed::Box, vec::Vec};
use bitflags::*;
use core::cell::RefCell;
use smallvec::SmallVec;

#[cfg(test)]
use core::assert_matches::assert_matches;

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
                let val = WasmValType::from_u8(reader.read_byte()?)?;
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

        macro_rules! MEM_LOAD {
            ($val_type:ident, $mnemonic:ident, $bytecode:ident, $arg:expr, $module:ident, $reader:ident, $position:ident, $int_codes:ident, $value_stack:ident,) => {
                #[cfg(test)]
                assert_matches!($bytecode, WasmBytecode::$mnemonic(_));

                if !$module.has_memory() {
                    return Err(WasmDecodeErrorKind::OutOfMemory);
                }
                let a = $value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                if a != WasmValType::I32 {
                    return Err(WasmDecodeErrorKind::TypeMismatch);
                }
                $int_codes.push(WasmImc::new(
                    WasmIntMnemonic::$mnemonic($arg.offset, $position).normalized(),
                    $value_stack.len().into(),
                ));
                $value_stack.push(WasmValType::$val_type);
            };
        }
        macro_rules! MEM_STORE {
            ($val_type:ident, $mnemonic:ident, $bytecode:ident, $arg:expr, $module:ident, $reader:ident, $position:ident, $int_codes:ident, $value_stack:ident,) => {
                #[cfg(test)]
                assert_matches!($bytecode, WasmBytecode::$mnemonic(_));

                if !$module.has_memory() {
                    return Err(WasmDecodeErrorKind::OutOfMemory);
                }
                let d = $value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                let i = $value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                if i != WasmValType::I32 && d != WasmValType::$val_type {
                    return Err(WasmDecodeErrorKind::TypeMismatch);
                }
                $int_codes.push(WasmImc::new(
                    WasmIntMnemonic::$mnemonic($arg.offset, $position).normalized(),
                    $value_stack.len().into(),
                ));
            };
        }
        macro_rules! UNARY {
            ($val_type:ident, $mnemonic:ident, $opcode:ident, $position:ident, $int_codes:ident, $value_stack:ident,) => {
                #[cfg(test)]
                assert_matches!($opcode, WasmBytecode::$mnemonic);

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
            ($in_type:ident, $out_type:ident, $mnemonic:ident, $opcode:ident, $position:ident, $int_codes:ident, $value_stack:ident,) => {
                #[cfg(test)]
                assert_matches!($opcode, WasmBytecode::$mnemonic);

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
        }
        macro_rules! BIN_CMP {
            ($val_type:ident, $mnemonic:ident, $opcode:ident, $position:ident, $int_codes:ident, $value_stack:ident,) => {
                #[cfg(test)]
                assert_matches!($opcode, WasmBytecode::$mnemonic);

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
                assert_matches!($opcode, WasmBytecode::$mnemonic);

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
                assert_matches!($opcode, WasmBytecode::$mnemonic);

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

        loop {
            max_stack = usize::max(max_stack, value_stack.len());
            max_block_level = usize::max(max_block_level, block_stack.len());
            let position = ExceptionPosition::new(reader.position());
            let bytecode = WasmBytecode::fetch(reader)?;

            match bytecode {
                WasmBytecode::Unreachable => {
                    int_codes.push(WasmImc::new(
                        WasmIntMnemonic::Unreachable(position),
                        value_stack.len().into(),
                    ));
                }

                WasmBytecode::Nop => (),

                WasmBytecode::Block(block_type) => {
                    let target = blocks.len();
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
                            WasmIntMnemonic::Undefined(bytecode.into(), position),
                            value_stack.len().into(),
                        ));
                    }
                }
                WasmBytecode::Loop(block_type) => {
                    let target = blocks.len();
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
                            WasmIntMnemonic::Undefined(bytecode.into(), position),
                            value_stack.len().into(),
                        ));
                    }
                }
                WasmBytecode::If(block_type) => {
                    let cc = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    if cc != WasmValType::I32 {
                        return Err(WasmDecodeErrorKind::TypeMismatch);
                    }
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
                        WasmIntMnemonic::Undefined(bytecode.into(), position),
                        value_stack.len().into(),
                    ));
                }
                WasmBytecode::Else => {
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
                        WasmIntMnemonic::Undefined(bytecode.into(), position),
                        value_stack.len().into(),
                    ));
                }
                WasmBytecode::End => {
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
                                StackLevel(value_stack.len()),
                            ));
                        }
                        break;
                    }
                }

                WasmBytecode::Br(br) => {
                    let target = block_stack
                        .get(block_stack.len() - (br as usize) - 1)
                        .ok_or(WasmDecodeErrorKind::OutOfBranch)?;
                    int_codes.push(WasmImc::new(
                        WasmIntMnemonic::Br(*target as u32),
                        value_stack.len().into(),
                    ));
                }
                WasmBytecode::BrIf(br) => {
                    let target = block_stack
                        .get(block_stack.len() - (br as usize) - 1)
                        .ok_or(WasmDecodeErrorKind::OutOfBranch)?;
                    let cc = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    if cc != WasmValType::I32 {
                        return Err(WasmDecodeErrorKind::TypeMismatch);
                    }
                    int_codes.push(WasmImc::new(
                        WasmIntMnemonic::BrIf(*target as u32),
                        value_stack.len().into(),
                    ));
                }
                WasmBytecode::BrTable(mut table) => {
                    for item in table.iter_mut() {
                        let target = block_stack
                            .get(block_stack.len() - (*item as usize) - 1)
                            .ok_or(WasmDecodeErrorKind::OutOfBranch)?;
                        *item = *target as u32;
                    }
                    let cc = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    if cc != WasmValType::I32 {
                        return Err(WasmDecodeErrorKind::TypeMismatch);
                    }
                    int_codes.push(WasmImc::new(
                        WasmIntMnemonic::BrTable(table),
                        value_stack.len().into(),
                    ));
                }

                WasmBytecode::Return => {
                    if let Some(result_type) = result_types.first() {
                        let result_type2 =
                            value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                        if *result_type != result_type2 {
                            return Err(WasmDecodeErrorKind::TypeMismatch);
                        }
                        match result_type {
                            WasmValType::I32 | WasmValType::I64 => int_codes.push(WasmImc::new(
                                WasmIntMnemonic::ReturnI,
                                StackLevel(value_stack.len()),
                            )),
                            WasmValType::F32 | WasmValType::F64 => int_codes.push(WasmImc::new(
                                WasmIntMnemonic::ReturnF,
                                StackLevel(value_stack.len()),
                            )),
                        }
                    } else {
                        int_codes.push(WasmImc::new(
                            WasmIntMnemonic::ReturnN,
                            StackLevel(value_stack.len() - 1),
                        ));
                    }
                }

                WasmBytecode::Call(func_index) => {
                    flags.remove(WasmBlockFlag::LEAF_FUNCTION);
                    let function = module
                        .functions()
                        .get(func_index as usize)
                        .ok_or(WasmDecodeErrorKind::InvalidData)?;
                    int_codes.push(WasmImc::new(
                        WasmIntMnemonic::Call(func_index as usize, position),
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
                WasmBytecode::CallIndirect(type_index, _reserved) => {
                    flags.remove(WasmBlockFlag::LEAF_FUNCTION);
                    let type_index = WasmTypeIndex::new(module, type_index)
                        .ok_or(WasmDecodeErrorKind::InvalidData)?;
                    let func_type = module.type_by_index(type_index);
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

                WasmBytecode::Drop => {
                    value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                }

                WasmBytecode::Select => {
                    let cc = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    let b = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    let a = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    if a != b || cc != WasmValType::I32 {
                        return Err(WasmDecodeErrorKind::TypeMismatch);
                    }
                    // match a {
                    //     WasmValType::I32 | WasmValType::I64 => {
                    int_codes.push(WasmImc::new(
                        WasmIntMnemonic::SelectI,
                        value_stack.len().into(),
                    ));
                    //     }
                    //     WasmValType::F32 | WasmValType::F64 => {
                    //         int_codes.push(WasmImc::new(
                    //             WasmIntMnemonic::SelectF,
                    //             value_stack.len().into(),
                    //         ));
                    //     }
                    // }
                    value_stack.push(a);
                }

                WasmBytecode::LocalGet(local_index) => {
                    let val = *local_types
                        .get(local_index as usize)
                        .ok_or(WasmDecodeErrorKind::InvalidLocal)?;
                    int_codes.push(WasmImc::new(
                        WasmIntMnemonic::LocalGetI(unsafe { LocalVarIndex::new(local_index) }),
                        value_stack.len().into(),
                    ));
                    value_stack.push(val);
                }
                WasmBytecode::LocalSet(local_index) => {
                    let val = *local_types
                        .get(local_index as usize)
                        .ok_or(WasmDecodeErrorKind::InvalidLocal)?;
                    let stack = value_stack.pop().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    if stack != val {
                        return Err(WasmDecodeErrorKind::TypeMismatch);
                    }
                    int_codes.push(WasmImc::new(
                        WasmIntMnemonic::LocalSetI(unsafe { LocalVarIndex::new(local_index) }),
                        value_stack.len().into(),
                    ));
                }
                WasmBytecode::LocalTee(local_index) => {
                    let val = *local_types
                        .get(local_index as usize)
                        .ok_or(WasmDecodeErrorKind::InvalidLocal)?;
                    let stack = *value_stack.last().ok_or(WasmDecodeErrorKind::OutOfStack)?;
                    if stack != val {
                        return Err(WasmDecodeErrorKind::TypeMismatch);
                    }
                    int_codes.push(WasmImc::new(
                        WasmIntMnemonic::LocalTeeI(unsafe { LocalVarIndex::new(local_index) }),
                        StackLevel(value_stack.len() - 1),
                    ));
                }

                WasmBytecode::GlobalGet(global_index) => {
                    let val_type = module
                        .globals()
                        .get(global_index as usize)
                        .map(|v| v.val_type())
                        .ok_or(WasmDecodeErrorKind::InvalidGlobal)?;
                    int_codes.push(WasmImc::new(
                        WasmIntMnemonic::GlobalGetI(unsafe { GlobalVarIndex::new(global_index) }),
                        value_stack.len().into(),
                    ));
                    value_stack.push(val_type);
                }
                WasmBytecode::GlobalSet(global_index) => {
                    let global = module
                        .globals()
                        .get(global_index as usize)
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
                        WasmIntMnemonic::GlobalSetI(unsafe { GlobalVarIndex::new(global_index) }),
                        value_stack.len().into(),
                    ));
                }

                WasmBytecode::I32Load(memarg) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(I32, I32Load, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::I32Load8S(memarg) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(I32, I32Load8S, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::I32Load8U(memarg) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(I32, I32Load8U, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::I32Load16S(memarg) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(I32, I32Load16S, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::I32Load16U(memarg) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(I32, I32Load16U, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }

                WasmBytecode::I64Load(memarg) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(I64, I64Load, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::I64Load8S(memarg) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(I64, I64Load8S, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::I64Load8U(memarg) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(I64, I64Load8U, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::I64Load16S(memarg) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(I64, I64Load16S, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::I64Load16U(memarg) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(I64, I64Load16U, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::I64Load32S(memarg) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(I64, I64Load32S, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::I64Load32U(memarg) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(I64, I64Load32U, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }

                WasmBytecode::I32Store(memarg) => {
                    #[rustfmt::skip]
                    MEM_STORE!(I32, I32Store, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::I32Store8(memarg) => {
                    #[rustfmt::skip]
                    MEM_STORE!(I32, I32Store8, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::I32Store16(memarg) => {
                    #[rustfmt::skip]
                    MEM_STORE!(I32, I32Store16, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::I64Store(memarg) => {
                    #[rustfmt::skip]
                    MEM_STORE!(I64, I64Store, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::I64Store8(memarg) => {
                    #[rustfmt::skip]
                    MEM_STORE!(I64, I64Store8, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::I64Store16(memarg) => {
                    #[rustfmt::skip]
                    MEM_STORE!(I64, I64Store16, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::I64Store32(memarg) => {
                    #[rustfmt::skip]
                    MEM_STORE!(I64, I64Store32, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }

                WasmBytecode::F32Load(memarg) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(F32, F32Load, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::F64Load(memarg) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(F64, F64Load, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::F32Store(memarg) => {
                    #[rustfmt::skip]
                    MEM_STORE!(F32, F32Store, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }
                WasmBytecode::F64Store(memarg) => {
                    #[rustfmt::skip]
                    MEM_STORE!(F64, F64Store, bytecode, memarg, module, reader, position, int_codes, value_stack, );
                }

                WasmBytecode::MemorySize(index) => {
                    if (index as usize) >= module.memories().len() {
                        return Err(WasmDecodeErrorKind::OutOfMemory);
                    }
                    int_codes.push(WasmImc::new(
                        WasmIntMnemonic::MemorySize,
                        value_stack.len().into(),
                    ));
                    value_stack.push(WasmValType::I32);
                }

                WasmBytecode::MemoryGrow(index) => {
                    if (index as usize) >= module.memories().len() {
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

                WasmBytecode::I32Const(val) => {
                    int_codes.push(WasmImc::new(
                        WasmIntMnemonic::I32Const(val),
                        value_stack.len().into(),
                    ));
                    value_stack.push(WasmValType::I32);
                }
                WasmBytecode::I64Const(val) => {
                    int_codes.push(WasmImc::new(
                        WasmIntMnemonic::I64Const(val),
                        value_stack.len().into(),
                    ));
                    value_stack.push(WasmValType::I64);
                }
                WasmBytecode::F32Const(val) => {
                    int_codes.push(WasmImc::new(
                        WasmIntMnemonic::F32Const(val),
                        value_stack.len().into(),
                    ));
                    value_stack.push(WasmValType::F32);
                }
                WasmBytecode::F64Const(val) => {
                    int_codes.push(WasmImc::new(
                        WasmIntMnemonic::F64Const(val),
                        value_stack.len().into(),
                    ));
                    value_stack.push(WasmValType::F64);
                }

                // unary operator [i32] -> [i32]
                WasmBytecode::I32Eqz => {
                    UNARY!(I32, I32Eqz, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32Clz => {
                    UNARY!(I32, I32Clz, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32Ctz => {
                    UNARY!(I32, I32Ctz, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32Popcnt => {
                    UNARY!(I32, I32Popcnt, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32Extend8S => {
                    UNARY!(I32, I32Extend8S, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32Extend16S => {
                    UNARY!(
                        I32,
                        I32Extend16S,
                        bytecode,
                        position,
                        int_codes,
                        value_stack,
                    );
                }

                // binary operator [i32, i32] -> [i32]
                WasmBytecode::I32Eq => {
                    BIN_CMP!(I32, I32Eq, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32Ne => {
                    BIN_CMP!(I32, I32Ne, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32LtS => {
                    BIN_CMP!(I32, I32LtS, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32LtU => {
                    BIN_CMP!(I32, I32LtU, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32GtS => {
                    BIN_CMP!(I32, I32GtS, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32GtU => {
                    BIN_CMP!(I32, I32GtU, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32LeS => {
                    BIN_CMP!(I32, I32LeS, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32LeU => {
                    BIN_CMP!(I32, I32LeU, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32GeS => {
                    BIN_CMP!(I32, I32GeS, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32GeU => {
                    BIN_CMP!(I32, I32GeU, bytecode, position, int_codes, value_stack,);
                }

                WasmBytecode::I32Add => {
                    BIN_OP!(I32, I32Add, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32Sub => {
                    BIN_OP!(I32, I32Sub, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32Mul => {
                    BIN_OP!(I32, I32Mul, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32DivS => {
                    BIN_DIV!(I32, I32DivS, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32DivU => {
                    BIN_DIV!(I32, I32DivU, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32RemS => {
                    BIN_DIV!(I32, I32RemS, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32RemU => {
                    BIN_DIV!(I32, I32RemU, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32And => {
                    BIN_OP!(I32, I32And, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32Or => {
                    BIN_OP!(I32, I32Or, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32Xor => {
                    BIN_OP!(I32, I32Xor, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32Shl => {
                    BIN_OP!(I32, I32Shl, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32ShrS => {
                    BIN_OP!(I32, I32ShrS, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32ShrU => {
                    BIN_OP!(I32, I32ShrU, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32Rotl => {
                    BIN_OP!(I32, I32Rotl, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32Rotr => {
                    BIN_OP!(I32, I32Rotr, bytecode, position, int_codes, value_stack,);
                }

                // binary operator [i64, i64] -> [i32]
                WasmBytecode::I64Eq => {
                    BIN_CMP!(I64, I64Eq, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64Ne => {
                    BIN_CMP!(I64, I64Ne, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64LtS => {
                    BIN_CMP!(I64, I64LtS, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64LtU => {
                    BIN_CMP!(I64, I64LtU, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64GtS => {
                    BIN_CMP!(I64, I64GtS, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64GtU => {
                    BIN_CMP!(I64, I64GtU, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64LeS => {
                    BIN_CMP!(I64, I64LeS, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64LeU => {
                    BIN_CMP!(I64, I64LeU, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64GeS => {
                    BIN_CMP!(I64, I64GeS, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64GeU => {
                    BIN_CMP!(I64, I64GeU, bytecode, position, int_codes, value_stack,);
                }

                // unary operator [i64] -> [i64]
                WasmBytecode::I64Clz => {
                    UNARY!(I64, I64Clz, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64Ctz => {
                    UNARY!(I64, I64Ctz, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64Popcnt => {
                    UNARY!(I64, I64Popcnt, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64Extend8S => {
                    UNARY!(I64, I64Extend8S, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64Extend16S => {
                    UNARY!(
                        I64,
                        I64Extend16S,
                        bytecode,
                        position,
                        int_codes,
                        value_stack,
                    );
                }
                WasmBytecode::I64Extend32S => {
                    UNARY!(
                        I64,
                        I64Extend32S,
                        bytecode,
                        position,
                        int_codes,
                        value_stack,
                    );
                }

                // binary operator [i64, i64] -> [i64]
                WasmBytecode::I64Add => {
                    BIN_OP!(I64, I64Add, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64Sub => {
                    BIN_OP!(I64, I64Sub, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64Mul => {
                    BIN_OP!(I64, I64Mul, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64DivS => {
                    BIN_DIV!(I64, I64DivS, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64DivU => {
                    BIN_DIV!(I64, I64DivU, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64RemS => {
                    BIN_DIV!(I64, I64RemS, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64RemU => {
                    BIN_DIV!(I64, I64RemU, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64And => {
                    BIN_OP!(I64, I64And, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64Or => {
                    BIN_OP!(I64, I64Or, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64Xor => {
                    BIN_OP!(I64, I64Xor, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64Shl => {
                    BIN_OP!(I64, I64Shl, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64ShrS => {
                    BIN_OP!(I64, I64ShrS, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64ShrU => {
                    BIN_OP!(I64, I64ShrU, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64Rotl => {
                    BIN_OP!(I64, I64Rotl, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64Rotr => {
                    BIN_OP!(I64, I64Rotr, bytecode, position, int_codes, value_stack,);
                }

                // [i64] -> [i32]
                WasmBytecode::I64Eqz => {
                    UNARY2!(I64, I32, I64Eqz, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32WrapI64 => {
                    #[rustfmt::skip]
                    UNARY2!(I64, I32, I32WrapI64, bytecode, position, int_codes, value_stack,);
                }

                // [i32] -> [i64]
                WasmBytecode::I64ExtendI32S => {
                    #[rustfmt::skip]
                    UNARY2!(I32, I64, I64ExtendI32S, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64ExtendI32U => {
                    #[rustfmt::skip]
                    UNARY2!(I32, I64, I64ExtendI32U, bytecode, position, int_codes, value_stack,);
                }

                // [f32, f32] -> [i32]
                WasmBytecode::F32Eq => {
                    BIN_CMP!(F32, F32Eq, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Ne => {
                    BIN_CMP!(F32, F32Ne, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Lt => {
                    BIN_CMP!(F32, F32Lt, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Gt => {
                    BIN_CMP!(F32, F32Gt, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Le => {
                    BIN_CMP!(F32, F32Le, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Ge => {
                    BIN_CMP!(F32, F32Ge, bytecode, position, int_codes, value_stack,);
                }

                // [f32] -> [f32]
                WasmBytecode::F32Abs => {
                    UNARY!(F32, F32Abs, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Neg => {
                    UNARY!(F32, F32Neg, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Ceil => {
                    UNARY!(F32, F32Ceil, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Floor => {
                    UNARY!(F32, F32Floor, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Trunc => {
                    UNARY!(F32, F32Trunc, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Nearest => {
                    UNARY!(F32, F32Nearest, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Sqrt => {
                    UNARY!(F32, F32Sqrt, bytecode, position, int_codes, value_stack,);
                }

                // [f32, f32] -> [f32]
                WasmBytecode::F32Add => {
                    BIN_OP!(F32, F32Add, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Sub => {
                    BIN_OP!(F32, F32Sub, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Mul => {
                    BIN_OP!(F32, F32Mul, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Div => {
                    BIN_OP!(F32, F32Div, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Min => {
                    BIN_OP!(F32, F32Min, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Max => {
                    BIN_OP!(F32, F32Max, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32Copysign => {
                    BIN_OP!(F32, F32Copysign, bytecode, position, int_codes, value_stack,);
                }

                // [f64, f64] -> [i32]
                WasmBytecode::F64Eq => {
                    BIN_CMP!(F64, F64Eq, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Ne => {
                    BIN_CMP!(F64, F64Ne, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Lt => {
                    BIN_CMP!(F64, F64Lt, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Gt => {
                    BIN_CMP!(F64, F64Gt, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Le => {
                    BIN_CMP!(F64, F64Le, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Ge => {
                    BIN_CMP!(F64, F64Ge, bytecode, position, int_codes, value_stack,);
                }

                // [f64] -> [f64]
                WasmBytecode::F64Abs => {
                    UNARY!(F64, F64Abs, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Neg => {
                    UNARY!(F64, F64Neg, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Ceil => {
                    UNARY!(F64, F64Ceil, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Floor => {
                    UNARY!(F64, F64Floor, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Trunc => {
                    UNARY!(F64, F64Trunc, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Nearest => {
                    UNARY!(F64, F64Nearest, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Sqrt => {
                    UNARY!(F64, F64Sqrt, bytecode, position, int_codes, value_stack,);
                }

                // [f64, f64] -> [f64]
                WasmBytecode::F64Add => {
                    BIN_OP!(F64, F64Add, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Sub => {
                    BIN_OP!(F64, F64Sub, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Mul => {
                    BIN_OP!(F64, F64Mul, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Div => {
                    BIN_OP!(F64, F64Div, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Min => {
                    BIN_OP!(F64, F64Min, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Max => {
                    BIN_OP!(F64, F64Max, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64Copysign => {
                    BIN_OP!(F64, F64Copysign, bytecode, position, int_codes, value_stack,);
                }

                // [f32] -> [i32]
                WasmBytecode::I32TruncF32S => {
                    #[rustfmt::skip]
                    UNARY2!(F32, I32, I32TruncF32S, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32TruncF32U => {
                    #[rustfmt::skip]
                    UNARY2!(F32, I32, I32TruncF32U, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32TruncF64S => {
                    #[rustfmt::skip]
                    UNARY2!(F64, I32, I32TruncF64S, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32TruncF64U => {
                    #[rustfmt::skip]
                    UNARY2!(F64, I32, I32TruncF64U, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64TruncF32S => {
                    #[rustfmt::skip]
                    UNARY2!(F32, I64, I64TruncF32S, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64TruncF32U => {
                    #[rustfmt::skip]
                    UNARY2!(F32, I64, I64TruncF32U, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64TruncF64S => {
                    #[rustfmt::skip]
                    UNARY2!(F64, I64, I64TruncF64S, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64TruncF64U => {
                    #[rustfmt::skip]
                    UNARY2!(F64, I64, I64TruncF64U, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32ConvertI32S => {
                    #[rustfmt::skip]
                    UNARY2!(I32, F32, F32ConvertI32S, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32ConvertI32U => {
                    #[rustfmt::skip]
                    UNARY2!(I32, F32, F32ConvertI32U, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32ConvertI64S => {
                    #[rustfmt::skip]
                    UNARY2!(I64, F32, F32ConvertI64S, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32ConvertI64U => {
                    #[rustfmt::skip]
                    UNARY2!(I64, F32, F32ConvertI64U, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32DemoteF64 => {
                    #[rustfmt::skip]
                    UNARY2!(F64, F32, F32DemoteF64, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64ConvertI32S => {
                    #[rustfmt::skip]
                    UNARY2!(I32, F64, F64ConvertI32S, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64ConvertI32U => {
                    #[rustfmt::skip]
                    UNARY2!(I32, F64, F64ConvertI32U, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64ConvertI64S => {
                    #[rustfmt::skip]
                    UNARY2!(I64, F64, F64ConvertI64S, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64ConvertI64U => {
                    #[rustfmt::skip]
                    UNARY2!(I64, F64, F64ConvertI64U, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64PromoteF32 => {
                    #[rustfmt::skip]
                    UNARY2!(F32, F64, F64PromoteF32, bytecode, position, int_codes, value_stack,);
                }

                WasmBytecode::I32ReinterpretF32 => {
                    #[rustfmt::skip]
                    UNARY2!(F32, I32, I32ReinterpretF32, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64ReinterpretF64 => {
                    #[rustfmt::skip]
                    UNARY2!(F64, I64, I64ReinterpretF64, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F32ReinterpretI32 => {
                    #[rustfmt::skip]
                    UNARY2!(I32, F32, F32ReinterpretI32, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::F64ReinterpretI64 => {
                    #[rustfmt::skip]
                    UNARY2!(I64, F64, F64ReinterpretI64, bytecode, position, int_codes, value_stack,);
                }

                WasmBytecode::I32TruncSatF32S => {
                    #[rustfmt::skip]
                    UNARY2!(F32, I32, I32TruncSatF32S, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32TruncSatF32U => {
                    #[rustfmt::skip]
                    UNARY2!(F32, I32, I32TruncSatF32U, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32TruncSatF64S => {
                    #[rustfmt::skip]
                    UNARY2!(F64, I32, I32TruncSatF64S, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I32TruncSatF64U => {
                    #[rustfmt::skip]
                    UNARY2!(F64, I32, I32TruncSatF64U, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64TruncSatF32S => {
                    #[rustfmt::skip]
                    UNARY2!(F32, I64, I64TruncSatF32S, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64TruncSatF32U => {
                    #[rustfmt::skip]
                    UNARY2!(F32, I64, I64TruncSatF32U, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64TruncSatF64S => {
                    #[rustfmt::skip]
                    UNARY2!(F64, I64, I64TruncSatF64S, bytecode, position, int_codes, value_stack,);
                }
                WasmBytecode::I64TruncSatF64U => {
                    #[rustfmt::skip]
                    UNARY2!(F64, I64, I64TruncSatF64U, bytecode, position, int_codes, value_stack,);
                }

                WasmBytecode::MemoryCopy => {
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

                WasmBytecode::MemoryFill => {
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

                _ => return Err(WasmDecodeErrorKind::UnsupportedBytecode(bytecode.into())),
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
                    (I64Const(val), I64And) => {
                        fused2!(int_codes, i, FusedI64AndI(*val as u64));
                    }
                    (I64Const(val), I64Or) => {
                        fused2!(int_codes, i, FusedI64OrI(*val as u64));
                    }
                    (I64Const(val), I64Xor) => {
                        fused2!(int_codes, i, FusedI64XorI(*val as u64));
                    }
                    (I64Const(val), I64Shl) => {
                        fused2!(int_codes, i, FusedI64ShlI(*val as u32));
                    }
                    (I64Const(val), I64ShrS) => {
                        fused2!(int_codes, i, FusedI64ShrSI(*val as u32));
                    }
                    (I64Const(val), I64ShrU) => {
                        fused2!(int_codes, i, FusedI64ShrUI(*val as u32));
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

    #[inline]
    pub const unsafe fn succ(self, delta: usize) -> Self {
        StackLevel(self.0.wrapping_add(delta))
    }

    #[inline]
    pub const unsafe fn add(self, offset: StackOffset) -> Self {
        StackLevel(self.0.wrapping_add(offset.0))
    }

    #[inline]
    pub const unsafe fn sub(self, offset: StackOffset) -> Self {
        StackLevel(self.0.wrapping_sub(offset.0))
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
