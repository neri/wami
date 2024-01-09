//! WebAssembly Intermediate Code Interpreter

use super::{intcode::*, *};
use crate::{
    memory::WasmMemory,
    opcode::{WasmOpcode, WasmOpcodeFC, WasmSingleOpcode},
    stack::*,
    wasm::*,
};
use alloc::{borrow::ToOwned, string::String, vec::Vec};
use core::{
    fmt, iter,
    mem::{size_of, transmute},
    ops::Neg,
};

const INITIAL_VALUE_STACK_SIZE: usize = 512;

/// Wasm Intermediate Code Interpreter
pub struct WasmInterpreter<'a> {
    module: &'a WasmModule,
    func_index: usize,
}

impl<'a> WasmInterpreter<'a> {
    #[inline]
    pub fn new(module: &'a WasmModule) -> Self {
        Self {
            module,
            func_index: 0,
        }
    }
}

impl WasmInterpreter<'_> {
    #[inline]
    fn error(
        &mut self,
        kind: WasmRuntimeErrorKind,
        opcode: WasmOpcode,
        ex_position: ExceptionPosition,
    ) -> WasmRuntimeError {
        let function_name = self
            .module
            .names()
            .and_then(|v| v.func_by_index(self.func_index))
            .map(|v| v.to_owned());
        let file_position =
            self.module.func_position(self.func_index).unwrap_or(0) + ex_position.position();

        WasmRuntimeError {
            kind,
            file_position,
            function: self.func_index,
            function_name,
            position: ex_position.position(),
            opcode,
        }
    }

    #[inline]
    pub fn invoke(
        &mut self,
        func_index: usize,
        code_block: &WasmCodeBlock,
        locals: &[WasmUnionValue],
        result_types: &[WasmValType],
    ) -> Result<Option<WasmValue>, WasmRuntimeError> {
        if locals.len() < code_block.local_types().len() {
            return Err(WasmRuntimeErrorKind::InvalidParameter.into());
        }
        let mut heap = StackHeap::with_capacity(0x10000);
        let local2 = heap.alloc_slice::<WasmUnionValue>(locals.len());
        local2.copy_from_slice(locals);
        self._interpret(
            func_index,
            code_block,
            LocalVariables::new(local2),
            result_types,
            &mut heap,
        )
    }

    fn _interpret(
        &mut self,
        func_index: usize,
        code_block: &WasmCodeBlock,
        mut locals: LocalVariables,
        result_types: &[WasmValType],
        heap: &mut StackHeap,
    ) -> Result<Option<WasmValue>, WasmRuntimeError> {
        macro_rules! GET_MEMORY {
            ($self:ident) => {
                $self
                    .module
                    .memories()
                    .get(0)
                    .ok_or(WasmRuntimeErrorKind::OutOfMemory)
            };
        }

        macro_rules! BORROW_MEMORY {
            ($self:ident) => {
                GET_MEMORY!($self).and_then(|v| v.try_borrow())
            };
        }

        self.func_index = func_index;

        let mut codes = WasmIntermediateCodeStream::from_codes(code_block.intermediate_codes())
            .ok_or(WasmRuntimeError::from(
                WasmRuntimeErrorKind::InternalInconsistency,
            ))?;

        let mut value_stack = StackFrame::new(heap.alloc_slice(code_block.max_value_stack()));

        let mut result_stack_level = StackLevel::zero();

        let mut memory = BORROW_MEMORY!(self)?;

        macro_rules! MEM_LOAD {
            ($stor_type:ident, $data_type:ident, $opcode:ident, $offset:ident, $ex_position:ident, $code:ident, $value_stack:ident, $memory:ident, ) => {
                #[cfg(test)]
                assert!(matches!($code.mnemonic(), WasmIntMnemonic::$opcode(_, _)));

                let var = $value_stack.get_mut($code.base_stack_level());
                let index = unsafe { var.get_u32() };
                let ea = WasmMemory::effective_address::<$data_type>($offset, index, $memory.len())
                    .map_err(|e| self.error(e, WasmSingleOpcode::$opcode.into(), $ex_position))?;

                unsafe {
                    let p = $memory.as_ptr().byte_add(ea) as *const $data_type;
                    let data = p.read_volatile() as $stor_type;
                    *var = data.into();
                }
            };
        }

        macro_rules! MEM_STORE {
            ($stor_type:ident, $data_type:ident, $opcode:ident, $offset:ident, $ex_position:ident, $code:ident, $value_stack:ident, $memory:ident, ) => {
                #[cfg(test)]
                assert!(matches!($code.mnemonic(), WasmIntMnemonic::$opcode(_, _)));

                let stack_level = $code.base_stack_level();
                let index = unsafe { $value_stack.get(stack_level).get_u32() };
                let storage: $stor_type =
                    unsafe { $value_stack.get(stack_level.succ(1)).unsafe_into() };
                let ea = WasmMemory::effective_address::<$data_type>($offset, index, $memory.len())
                    .map_err(|e| self.error(e, WasmSingleOpcode::$opcode.into(), $ex_position))?;
                unsafe {
                    let p = $memory.as_mut_ptr().byte_add(ea) as *mut $data_type;
                    p.write_volatile(storage as $data_type);
                }
            };
        }

        macro_rules! DIV_OP {
            ($map_lhs:ident, $get_rhs:ident, $opcode:ident, $opr:ident, $ex_position:ident, $code:ident, $value_stack:ident, ) => {
                #[cfg(test)]
                assert!(matches!($code.mnemonic(), WasmIntMnemonic::$opcode(_)));

                let stack_level = $code.base_stack_level();
                let rhs = unsafe { $value_stack.get(stack_level.succ(1)).$get_rhs() };
                let lhs = $value_stack.get_mut(stack_level);
                if rhs == 0 {
                    return Err(self.error(
                        WasmRuntimeErrorKind::DivideByZero,
                        WasmSingleOpcode::$opcode.into(),
                        $ex_position,
                    ));
                }
                unsafe {
                    lhs.$map_lhs(|lhs| lhs.$opr(rhs));
                }
            };
        }

        loop {
            let code = codes.fetch();

            match *code.mnemonic() {
                WasmIntMnemonic::Unreachable(position) => {
                    return Err(self.error(
                        WasmRuntimeErrorKind::Unreachable,
                        WasmOpcode::UNREACHABLE,
                        position,
                    ));
                }

                WasmIntMnemonic::Undefined(opcode, position) => {
                    return Err(self.error(WasmRuntimeErrorKind::NotSupprted, opcode, position));
                }

                WasmIntMnemonic::Nop | WasmIntMnemonic::Block(_) | WasmIntMnemonic::End(_) => {
                    return Err(self.error(
                        WasmRuntimeErrorKind::InternalInconsistency,
                        WasmOpcode::UNREACHABLE,
                        ExceptionPosition::UNKNOWN,
                    ));
                }

                WasmIntMnemonic::Br(target) => {
                    codes.set_position(target)?;
                }
                WasmIntMnemonic::BrIf(target) => {
                    let cc = unsafe { value_stack.get(code.base_stack_level()).get_bool() };
                    if cc {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::BrTable(ref table) => {
                    let table_len = table.len() - 1;
                    let index = usize::min(table_len, unsafe {
                        value_stack.get(code.base_stack_level()).get_u32() as usize
                    });
                    let target = unsafe { *table.get_unchecked(index) };
                    codes.set_position(target)?;
                }

                WasmIntMnemonic::ReturnN => {
                    break;
                }

                WasmIntMnemonic::ReturnI | WasmIntMnemonic::ReturnF => {
                    result_stack_level = code.base_stack_level();
                    break;
                }

                WasmIntMnemonic::Call(func_index, ex_position) => {
                    let func = unsafe { self.module.functions().get_unchecked(func_index) };
                    drop(memory);
                    self.call(
                        WasmSingleOpcode::Call.into(),
                        ex_position,
                        code.base_stack_level(),
                        func,
                        &mut value_stack,
                        heap,
                    )?;
                    memory = BORROW_MEMORY!(self)?;
                }
                WasmIntMnemonic::CallIndirect(type_index, ex_position) => {
                    let opcode = WasmOpcode::Single(WasmSingleOpcode::CallIndirect);
                    let index =
                        unsafe { value_stack.get(code.base_stack_level()).get_i32() as usize };
                    let func = self.module.elem_get(index).ok_or(self.error(
                        WasmRuntimeErrorKind::NoMethod,
                        opcode,
                        ex_position,
                    ))?;
                    if func.type_index() != type_index {
                        return Err(self.error(
                            WasmRuntimeErrorKind::TypeMismatch,
                            opcode,
                            ex_position,
                        ));
                    }
                    drop(memory);
                    self.call(
                        opcode,
                        ex_position,
                        code.base_stack_level(),
                        func,
                        &mut value_stack,
                        heap,
                    )?;
                    memory = BORROW_MEMORY!(self)?;
                }

                WasmIntMnemonic::SelectI | WasmIntMnemonic::SelectF => {
                    let stack_level = code.base_stack_level();
                    let cc = unsafe { value_stack.get(stack_level.succ(2)).get_bool() };
                    if !cc {
                        unsafe {
                            let b = *value_stack.get(stack_level.succ(1));
                            let ref_a = value_stack.get_mut(stack_level);
                            *ref_a = b;
                        }
                    }
                }

                WasmIntMnemonic::LocalGetI(local_index)
                | WasmIntMnemonic::LocalGetF(local_index) => {
                    let local = locals.get_local(local_index);
                    let ref_a = value_stack.get_mut(code.base_stack_level());
                    *ref_a = *local;
                }
                WasmIntMnemonic::LocalSetI(local_index)
                | WasmIntMnemonic::LocalTeeI(local_index)
                | WasmIntMnemonic::LocalSetF(local_index)
                | WasmIntMnemonic::LocalTeeF(local_index) => {
                    let local = locals.get_local_mut(local_index);
                    let ref_a = value_stack.get(code.base_stack_level());
                    *local = *ref_a;
                }

                WasmIntMnemonic::GlobalGetI(global_ref)
                | WasmIntMnemonic::GlobalGetF(global_ref) => {
                    let global = self.module.global_get(global_ref);
                    let ref_a = value_stack.get_mut(code.base_stack_level());
                    *ref_a = global.value().into();
                }
                WasmIntMnemonic::GlobalSetI(global_ref)
                | WasmIntMnemonic::GlobalSetF(global_ref) => {
                    let global = self.module.global_get(global_ref);
                    let ref_a = value_stack.get(code.base_stack_level());
                    global.set_value(*ref_a);
                }

                WasmIntMnemonic::I32Load(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(u32, u32, I32Load, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::I32Load8S(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(i32, i8, I32Load8S, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::I32Load8U(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(u32, u8, I32Load8U, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::I32Load16S(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(i32, i16, I32Load16S, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::I32Load16U(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(u32, u16, I32Load16U, offset, ex_position, code, value_stack, memory, );
                }

                WasmIntMnemonic::I64Load(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(u64, u64, I64Load, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::I64Load8S(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(i64, i8, I64Load8S, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::I64Load8U(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(u64, u8, I64Load8U, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::I64Load16S(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(i64, i16, I64Load16S, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::I64Load16U(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(u64, u16, I64Load16U, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::I64Load32S(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(i64, i32, I64Load32S, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::I64Load32U(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(u64, u32, I64Load32U, offset, ex_position, code, value_stack, memory, );
                }

                WasmIntMnemonic::I32Store(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(u32, u32, I32Store, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::I32Store8(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(u32, u8, I32Store8, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::I32Store16(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(u32, u16, I32Store16, offset, ex_position, code, value_stack, memory, );
                }

                WasmIntMnemonic::I64Store(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(u64, u64, I64Store, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::I64Store8(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(u64, u8, I64Store8, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::I64Store16(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(u64, u16, I64Store16, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::I64Store32(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(u64, u32, I64Store32, offset, ex_position, code, value_stack, memory, );
                }

                WasmIntMnemonic::F32Load(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(f32, f32, F32Load, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::F32Store(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(f32, f32, F32Store, offset, ex_position, code, value_stack, memory, );
                }

                WasmIntMnemonic::F64Load(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(f64, f64, F64Load, offset, ex_position, code, value_stack, memory, );
                }
                WasmIntMnemonic::F64Store(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(f64, f64, F64Store, offset, ex_position, code, value_stack, memory, );
                }

                WasmIntMnemonic::MemorySize => {
                    let ref_a = value_stack.get_mut(code.base_stack_level());
                    ref_a.write_i32((memory.len() / WebAssembly::PAGE_SIZE) as i32);
                }
                WasmIntMnemonic::MemoryGrow => {
                    let ref_a = value_stack.get_mut(code.base_stack_level());
                    drop(memory);

                    let mem = GET_MEMORY!(self)?;
                    unsafe {
                        ref_a.map_u32(|v| mem.grow(v).unwrap_or(u32::MAX));
                    }

                    memory = BORROW_MEMORY!(self)?;
                }
                WasmIntMnemonic::MemoryCopy(ex_position) => {
                    let stack_level = code.base_stack_level();
                    let dest = unsafe { value_stack.get(stack_level).get_u32() } as usize;
                    let src = unsafe { value_stack.get(stack_level.succ(1)).get_u32() } as usize;
                    let count = unsafe { value_stack.get(stack_level.succ(2)).get_u32() } as usize;

                    WasmMemory::check_bound(dest as u64, count, memory.len())
                        .map_err(|k| self.error(k, WasmOpcodeFC::MemoryCopy.into(), ex_position))?;
                    WasmMemory::check_bound(src as u64, count, memory.len())
                        .map_err(|k| self.error(k, WasmOpcodeFC::MemoryCopy.into(), ex_position))?;

                    if count > 0 {
                        unsafe {
                            memory
                                .as_mut_ptr()
                                .add(dest)
                                .copy_from(memory.as_ptr().add(src), count);
                        }
                    }
                }
                WasmIntMnemonic::MemoryFill(ex_position) => {
                    let stack_level = code.base_stack_level();
                    let base = unsafe { value_stack.get(stack_level).get_u32() } as usize;
                    let val = unsafe { value_stack.get(stack_level.succ(1)).get_u8() };
                    let count = unsafe { value_stack.get(stack_level.succ(2)).get_u32() } as usize;

                    WasmMemory::check_bound(base as u64, count, memory.len())
                        .map_err(|k| self.error(k, WasmOpcodeFC::MemoryFill.into(), ex_position))?;

                    if count > 0 {
                        unsafe {
                            memory.as_mut_ptr().add(base).write_bytes(val, count);
                        }
                    }
                }

                WasmIntMnemonic::I32Const(val) => {
                    let ref_a = value_stack.get_mut(code.base_stack_level());
                    ref_a.write_i32(val);
                }
                WasmIntMnemonic::I64Const(val) => {
                    let ref_a = value_stack.get_mut(code.base_stack_level());
                    ref_a.write_i64(val);
                }
                WasmIntMnemonic::F32Const(val) => {
                    let ref_a = value_stack.get_mut(code.base_stack_level());
                    ref_a.write_f32(val);
                }
                WasmIntMnemonic::F64Const(val) => {
                    let ref_a = value_stack.get_mut(code.base_stack_level());
                    ref_a.write_f64(val);
                }

                WasmIntMnemonic::I32Eqz => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_bool(var.get_i32() == 0)
                    });
                }
                WasmIntMnemonic::I32Eq => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u32() == rhs.get_u32())
                    });
                }
                WasmIntMnemonic::I32Ne => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u32() != rhs.get_u32())
                    });
                }
                WasmIntMnemonic::I32LtS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_i32() < rhs.get_i32())
                    });
                }
                WasmIntMnemonic::I32LtU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u32() < rhs.get_u32())
                    });
                }
                WasmIntMnemonic::I32GtS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_i32() > rhs.get_i32())
                    });
                }
                WasmIntMnemonic::I32GtU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u32() > rhs.get_u32())
                    });
                }
                WasmIntMnemonic::I32LeS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_i32() <= rhs.get_i32())
                    });
                }
                WasmIntMnemonic::I32LeU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u32() <= rhs.get_u32())
                    });
                }
                WasmIntMnemonic::I32GeS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_i32() >= rhs.get_i32())
                    });
                }
                WasmIntMnemonic::I32GeU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u32() >= rhs.get_u32())
                    });
                }

                WasmIntMnemonic::I32Clz => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.map_u32(|v| v.leading_zeros())
                    });
                }
                WasmIntMnemonic::I32Ctz => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.map_u32(|v| v.trailing_zeros())
                    });
                }
                WasmIntMnemonic::I32Popcnt => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.map_u32(|v| v.count_ones())
                    });
                }

                WasmIntMnemonic::I32Add => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_add(rhs.get_i32()));
                    });
                }
                WasmIntMnemonic::I32Sub => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_sub(rhs.get_i32()));
                    });
                }
                WasmIntMnemonic::I32Mul => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_mul(rhs.get_i32()));
                    });
                }

                WasmIntMnemonic::I32DivS(ex_position) => {
                    #[rustfmt::skip]
                    DIV_OP!(map_i32, get_i32, I32DivS, wrapping_div, ex_position, code, value_stack, );
                }
                WasmIntMnemonic::I32DivU(ex_position) => {
                    #[rustfmt::skip]
                    DIV_OP!(map_u32, get_u32, I32DivU, wrapping_div, ex_position, code, value_stack, );
                }
                WasmIntMnemonic::I32RemS(ex_position) => {
                    #[rustfmt::skip]
                    DIV_OP!(map_i32, get_i32, I32RemS, wrapping_rem, ex_position, code, value_stack, );
                }
                WasmIntMnemonic::I32RemU(ex_position) => {
                    #[rustfmt::skip]
                    DIV_OP!(map_u32, get_u32, I32RemU, wrapping_rem, ex_position, code, value_stack, );
                }

                WasmIntMnemonic::I32And => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u32(|lhs| lhs & rhs.get_u32());
                    });
                }
                WasmIntMnemonic::I32Or => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u32(|lhs| lhs | rhs.get_u32());
                    });
                }
                WasmIntMnemonic::I32Xor => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u32(|lhs| lhs ^ rhs.get_u32());
                    });
                }
                WasmIntMnemonic::I32Shl => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u32(|lhs| lhs.wrapping_shl(rhs.get_u32()));
                    });
                }
                WasmIntMnemonic::I32ShrS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_shr(rhs.get_u32()));
                    });
                }
                WasmIntMnemonic::I32ShrU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u32(|lhs| lhs.wrapping_shr(rhs.get_u32()));
                    });
                }
                WasmIntMnemonic::I32Rotl => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u32(|lhs| lhs.rotate_left(rhs.get_u32()));
                    });
                }
                WasmIntMnemonic::I32Rotr => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u32(|lhs| lhs.rotate_right(rhs.get_u32()));
                    });
                }

                WasmIntMnemonic::I64Eqz => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_bool(var.get_i64() == 0)
                    });
                }
                WasmIntMnemonic::I64Eq => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u64() == rhs.get_u64())
                    });
                }
                WasmIntMnemonic::I64Ne => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u64() != rhs.get_u64())
                    });
                }
                WasmIntMnemonic::I64LtS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_i64() < rhs.get_i64())
                    });
                }
                WasmIntMnemonic::I64LtU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u64() < rhs.get_u64())
                    });
                }
                WasmIntMnemonic::I64GtS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_i64() > rhs.get_i64())
                    });
                }
                WasmIntMnemonic::I64GtU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u64() > rhs.get_u64())
                    });
                }
                WasmIntMnemonic::I64LeS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_i64() <= rhs.get_i64())
                    });
                }
                WasmIntMnemonic::I64LeU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u64() <= rhs.get_u64())
                    });
                }
                WasmIntMnemonic::I64GeS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_i64() >= rhs.get_i64())
                    });
                }
                WasmIntMnemonic::I64GeU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u64() >= rhs.get_u64())
                    });
                }

                WasmIntMnemonic::I64Clz => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.map_u64(|v| v.leading_zeros() as u64);
                    });
                }
                WasmIntMnemonic::I64Ctz => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.map_u64(|v| v.trailing_zeros() as u64);
                    });
                }
                WasmIntMnemonic::I64Popcnt => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.map_u64(|v| v.count_ones() as u64);
                    });
                }
                WasmIntMnemonic::I64Add => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_add(rhs.get_i64()));
                    });
                }
                WasmIntMnemonic::I64Sub => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_sub(rhs.get_i64()));
                    });
                }
                WasmIntMnemonic::I64Mul => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_mul(rhs.get_i64()));
                    });
                }

                WasmIntMnemonic::I64DivS(ex_position) => {
                    #[rustfmt::skip]
                    DIV_OP!(map_i64, get_i64, I64DivS, wrapping_div, ex_position, code, value_stack, );
                }
                WasmIntMnemonic::I64DivU(ex_position) => {
                    #[rustfmt::skip]
                    DIV_OP!(map_u64, get_u64, I64DivU, wrapping_div, ex_position, code, value_stack, );
                }
                WasmIntMnemonic::I64RemS(ex_position) => {
                    #[rustfmt::skip]
                    DIV_OP!(map_i64, get_i64, I64RemS, wrapping_rem, ex_position, code, value_stack, );
                }
                WasmIntMnemonic::I64RemU(ex_position) => {
                    #[rustfmt::skip]
                    DIV_OP!(map_u64, get_u64, I64RemU, wrapping_rem, ex_position, code, value_stack, );
                }

                WasmIntMnemonic::I64And => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u64(|lhs| lhs & rhs.get_u64());
                    });
                }
                WasmIntMnemonic::I64Or => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u64(|lhs| lhs | rhs.get_u64());
                    });
                }
                WasmIntMnemonic::I64Xor => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u64(|lhs| lhs ^ rhs.get_u64());
                    });
                }
                WasmIntMnemonic::I64Shl => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u64(|lhs| lhs.wrapping_shl(rhs.get_u64() as u32));
                    });
                }
                WasmIntMnemonic::I64ShrS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_shr(rhs.get_u64() as u32));
                    });
                }
                WasmIntMnemonic::I64ShrU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u64(|lhs| lhs.wrapping_shr(rhs.get_u64() as u32));
                    });
                }
                WasmIntMnemonic::I64Rotl => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u64(|lhs| lhs.rotate_left(rhs.get_u64() as u32));
                    });
                }
                WasmIntMnemonic::I64Rotr => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u64(|lhs| lhs.rotate_right(rhs.get_u64() as u32));
                    });
                }

                WasmIntMnemonic::I32WrapI64 => {
                    Self::unary_op(code, &mut value_stack, |var| {
                        *var = StackTop::from_i32(unsafe { var.get_i64() as i32 });
                    });
                }
                WasmIntMnemonic::I32Extend8S => {
                    Self::unary_op(code, &mut value_stack, |var| {
                        *var = StackTop::from_i32(unsafe { var.get_i8() as i32 });
                    });
                }
                WasmIntMnemonic::I32Extend16S => {
                    Self::unary_op(code, &mut value_stack, |var| {
                        *var = StackTop::from_i32(unsafe { var.get_i16() as i32 });
                    });
                }
                WasmIntMnemonic::I64Extend8S => {
                    Self::unary_op(code, &mut value_stack, |var| {
                        *var = StackTop::from_i64(unsafe { var.get_i8() as i64 });
                    });
                }
                WasmIntMnemonic::I64Extend16S => {
                    Self::unary_op(code, &mut value_stack, |var| {
                        *var = StackTop::from_i64(unsafe { var.get_i16() as i64 });
                    });
                }
                WasmIntMnemonic::I64Extend32S | WasmIntMnemonic::I64ExtendI32S => {
                    Self::unary_op(code, &mut value_stack, |var| {
                        *var = StackTop::from_i64(unsafe { var.get_i32() as i64 });
                    });
                }
                WasmIntMnemonic::I64ExtendI32U => {
                    Self::unary_op(code, &mut value_stack, |var| {
                        *var = StackTop::from_u64(unsafe { var.get_u32() as u64 });
                    });
                }

                WasmIntMnemonic::F32Eq => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f32() == rhs.get_f32())
                    });
                }
                WasmIntMnemonic::F32Ne => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f32() != rhs.get_f32())
                    });
                }
                WasmIntMnemonic::F32Lt => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f32() < rhs.get_f32())
                    });
                }
                WasmIntMnemonic::F32Gt => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f32() > rhs.get_f32())
                    });
                }
                WasmIntMnemonic::F32Le => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f32() <= rhs.get_f32())
                    });
                }
                WasmIntMnemonic::F32Ge => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f32() >= rhs.get_f32())
                    });
                }

                WasmIntMnemonic::F32Abs => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f32(|v| core::intrinsics::fabsf32(v));
                    });
                }
                WasmIntMnemonic::F32Neg => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f32(|v| v.neg());
                    });
                }
                WasmIntMnemonic::F32Ceil => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f32(|v| core::intrinsics::ceilf32(v));
                    });
                }
                WasmIntMnemonic::F32Floor => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f32(|v| core::intrinsics::floorf32(v));
                    });
                }
                WasmIntMnemonic::F32Trunc => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f32(|v| core::intrinsics::truncf32(v));
                    });
                }
                WasmIntMnemonic::F32Nearest => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f32(|v| core::intrinsics::rintf32(v));
                    });
                }
                WasmIntMnemonic::F32Sqrt => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f32(|v| core::intrinsics::sqrtf32(v));
                    });
                }

                WasmIntMnemonic::F32Add => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f32(|lhs| lhs + rhs.get_f32());
                    });
                }
                WasmIntMnemonic::F32Sub => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f32(|lhs| lhs - rhs.get_f32());
                    });
                }
                WasmIntMnemonic::F32Mul => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f32(|lhs| lhs * rhs.get_f32());
                    });
                }
                WasmIntMnemonic::F32Div => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f32(|lhs| lhs / rhs.get_f32());
                    });
                }
                WasmIntMnemonic::F32Min => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f32(|lhs| lhs.minimum(rhs.get_f32()));
                    });
                }
                WasmIntMnemonic::F32Max => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f32(|lhs| lhs.maximum(rhs.get_f32()));
                    });
                }
                WasmIntMnemonic::F32Copysign => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f32(|lhs| core::intrinsics::copysignf32(lhs, rhs.get_f32()));
                    });
                }

                WasmIntMnemonic::F64Eq => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f64() == rhs.get_f64())
                    });
                }
                WasmIntMnemonic::F64Ne => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f64() != rhs.get_f64())
                    });
                }
                WasmIntMnemonic::F64Lt => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f64() < rhs.get_f64())
                    });
                }
                WasmIntMnemonic::F64Gt => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f64() > rhs.get_f64())
                    });
                }
                WasmIntMnemonic::F64Le => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f64() <= rhs.get_f64())
                    });
                }
                WasmIntMnemonic::F64Ge => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f64() >= rhs.get_f64())
                    });
                }

                WasmIntMnemonic::F64Abs => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f64(|v| core::intrinsics::fabsf64(v));
                    });
                }
                WasmIntMnemonic::F64Neg => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f64(|v| v.neg());
                    });
                }
                WasmIntMnemonic::F64Ceil => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f64(|v| core::intrinsics::ceilf64(v));
                    });
                }
                WasmIntMnemonic::F64Floor => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f64(|v| core::intrinsics::floorf64(v));
                    });
                }
                WasmIntMnemonic::F64Trunc => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f64(|v| core::intrinsics::truncf64(v));
                    });
                }
                WasmIntMnemonic::F64Nearest => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f64(|v| core::intrinsics::rintf64(v));
                    });
                }
                WasmIntMnemonic::F64Sqrt => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f64(|v| core::intrinsics::sqrtf64(v));
                    });
                }

                WasmIntMnemonic::F64Add => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f64(|lhs| lhs + rhs.get_f64());
                    });
                }
                WasmIntMnemonic::F64Sub => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f64(|lhs| lhs - rhs.get_f64());
                    });
                }
                WasmIntMnemonic::F64Mul => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f64(|lhs| lhs * rhs.get_f64());
                    });
                }
                WasmIntMnemonic::F64Div => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f64(|lhs| lhs / rhs.get_f64());
                    });
                }
                WasmIntMnemonic::F64Min => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f64(|lhs| lhs.minimum(rhs.get_f64()));
                    });
                }
                WasmIntMnemonic::F64Max => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f64(|lhs| lhs.maximum(rhs.get_f64()));
                    });
                }
                WasmIntMnemonic::F64Copysign => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f64(|lhs| core::intrinsics::copysignf64(lhs, rhs.get_f64()));
                    });
                }

                WasmIntMnemonic::I32TruncF32S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_i32(var.get_f32().to_int_unchecked());
                    });
                }
                WasmIntMnemonic::I32TruncF32U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_u32(var.get_f32().to_int_unchecked());
                    });
                }
                WasmIntMnemonic::I32TruncF64S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_i32(var.get_f64().to_int_unchecked());
                    });
                }
                WasmIntMnemonic::I32TruncF64U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_u32(var.get_f64().to_int_unchecked());
                    });
                }

                WasmIntMnemonic::I64TruncF32S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_i64(var.get_f32().to_int_unchecked());
                    });
                }
                WasmIntMnemonic::I64TruncF32U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_u64(var.get_f32().to_int_unchecked());
                    });
                }
                WasmIntMnemonic::I64TruncF64S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_i64(var.get_f64().to_int_unchecked());
                    });
                }
                WasmIntMnemonic::I64TruncF64U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_u64(var.get_f64().to_int_unchecked());
                    });
                }

                WasmIntMnemonic::I32TruncSatF32S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_i32(var.get_f32() as i32);
                    });
                }
                WasmIntMnemonic::I32TruncSatF32U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_u32(var.get_f32() as u32);
                    });
                }
                WasmIntMnemonic::I32TruncSatF64S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_i32(var.get_f64() as i32);
                    });
                }
                WasmIntMnemonic::I32TruncSatF64U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_u32(var.get_f64() as u32);
                    });
                }
                WasmIntMnemonic::I64TruncSatF32S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_i64(var.get_f32() as i64);
                    });
                }
                WasmIntMnemonic::I64TruncSatF32U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_u64(var.get_f32() as u64);
                    });
                }
                WasmIntMnemonic::I64TruncSatF64S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_i64(var.get_f64() as i64);
                    });
                }
                WasmIntMnemonic::I64TruncSatF64U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_u64(var.get_f64() as u64);
                    });
                }

                WasmIntMnemonic::F32ConvertI32S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f32(var.get_i32() as f32);
                    });
                }
                WasmIntMnemonic::F32ConvertI32U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f32(var.get_u32() as f32);
                    });
                }
                WasmIntMnemonic::F32ConvertI64S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f32(var.get_i64() as f32);
                    });
                }
                WasmIntMnemonic::F32ConvertI64U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f32(var.get_u64() as f32);
                    });
                }
                WasmIntMnemonic::F32DemoteF64 => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f32(var.get_f64() as f32);
                    });
                }

                WasmIntMnemonic::F64ConvertI32S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f64(var.get_i32() as f64);
                    });
                }
                WasmIntMnemonic::F64ConvertI32U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f64(var.get_u32() as f64);
                    });
                }
                WasmIntMnemonic::F64ConvertI64S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f64(var.get_i64() as f64);
                    });
                }
                WasmIntMnemonic::F64ConvertI64U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f64(var.get_u64() as f64);
                    });
                }
                WasmIntMnemonic::F64PromoteF32 => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f64(var.get_f32().into());
                    });
                }

                WasmIntMnemonic::I32ReinterpretF32 => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.write_u32(v.get_f32().to_bits());
                    })
                }
                WasmIntMnemonic::I64ReinterpretF64 => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.write_u64(v.get_f64().to_bits());
                    })
                }
                WasmIntMnemonic::F32ReinterpretI32 => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.write_f32(f32::from_bits(v.get_u32()));
                    })
                }
                WasmIntMnemonic::F64ReinterpretI64 => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.write_f64(f64::from_bits(v.get_u64()));
                    })
                }

                WasmIntMnemonic::FusedI32SetConst(local_index, val) => {
                    let local = locals.get_local_mut(local_index);
                    local.write_i32(val);
                }
                WasmIntMnemonic::FusedI64SetConst(local_index, val) => {
                    let local = locals.get_local_mut(local_index);
                    *local = val.into();
                }

                WasmIntMnemonic::FusedI32AddI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_add(val));
                    });
                }
                WasmIntMnemonic::FusedI32SubI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_sub(val));
                    });
                }
                WasmIntMnemonic::FusedI32AndI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u32(|lhs| lhs & val);
                    });
                }
                WasmIntMnemonic::FusedI32OrI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u32(|lhs| lhs | val);
                    });
                }
                WasmIntMnemonic::FusedI32XorI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u32(|lhs| lhs ^ val);
                    });
                }
                WasmIntMnemonic::FusedI32ShlI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_shl(val));
                    });
                }
                WasmIntMnemonic::FusedI32ShrSI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_shr(val));
                    });
                }
                WasmIntMnemonic::FusedI32ShrUI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u32(|lhs| lhs.wrapping_shl(val));
                    });
                }

                WasmIntMnemonic::FusedI64AddI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_add(val));
                    });
                }
                WasmIntMnemonic::FusedI64SubI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_sub(val));
                    });
                }
                WasmIntMnemonic::FusedI64AndI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u64(|lhs| lhs & val);
                    });
                }
                WasmIntMnemonic::FusedI64OrI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u64(|lhs| lhs | val);
                    });
                }
                WasmIntMnemonic::FusedI64XorI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u64(|lhs| lhs ^ val);
                    });
                }
                WasmIntMnemonic::FusedI64ShlI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u64(|lhs| lhs.wrapping_shl(val));
                    });
                }
                WasmIntMnemonic::FusedI64ShrSI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_shr(val));
                    });
                }
                WasmIntMnemonic::FusedI64ShrUI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u64(|lhs| lhs.wrapping_shr(val));
                    });
                }

                WasmIntMnemonic::FusedI32BrZ(target) => {
                    let cc = unsafe { value_stack.get_mut(code.base_stack_level()).get_i32() == 0 };
                    if cc {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrEq(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u32() == rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrNe(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u32() != rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrLtS(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_i32() < rhs.get_i32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrLtU(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u32() < rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrGtS(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_i32() > rhs.get_i32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrGtU(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u32() > rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrLeS(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_i32() <= rhs.get_i32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrLeU(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u32() <= rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrGeS(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_i32() >= rhs.get_i32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrGeU(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u32() >= rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }

                WasmIntMnemonic::FusedI64BrZ(target) => {
                    let cc = unsafe { value_stack.get_mut(code.base_stack_level()).get_i64() == 0 };
                    if cc {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI64BrEq(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u64() == rhs.get_u64() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI64BrNe(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u64() != rhs.get_u64() } {
                        codes.set_position(target)?;
                    }
                }
            }
        }
        if let Some(result_type) = result_types.first() {
            let val = value_stack.get(result_stack_level);
            Ok(Some(unsafe { val.get_by_type(*result_type) }))
        } else {
            Ok(None)
        }
    }

    #[inline(always)]
    fn unary_op<F, R>(code: &WasmImc, value_stack: &mut StackFrame, kernel: F) -> R
    where
        F: FnOnce(&mut StackTop) -> R,
    {
        let var = StackTop::from_union(value_stack.get_mut(code.base_stack_level()));
        kernel(var)
    }

    #[inline(always)]
    fn binary_op<F, R>(code: &WasmImc, value_stack: &mut StackFrame, kernel: F) -> R
    where
        F: FnOnce(&mut StackTop, WasmUnionValue) -> R,
    {
        let stack_level = code.base_stack_level();
        let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
        let lhs = StackTop::from_union(value_stack.get_mut(stack_level));
        kernel(lhs, rhs)
    }

    #[inline]
    fn call(
        &mut self,
        opcode: WasmOpcode,
        ex_position: ExceptionPosition,
        stack_pointer: StackLevel,
        target: &WasmFunction,
        value_stack: &mut StackFrame,
        heap: &mut StackHeap,
    ) -> Result<(), WasmRuntimeError> {
        let current_function = self.func_index;
        let module = self.module;
        let result_types = target.result_types();

        let param_len = target.param_types().len();
        // if stack_pointer < param_len {
        //     return Err(self.error(WasmRuntimeError::InternalInconsistency, code));
        // }
        let stack_under = unsafe { stack_pointer.sub(StackOffset::new(param_len)) };

        match target.content() {
            WasmFunctionContent::CodeBlock(code_block) => heap.snapshot(|heap| {
                let local_len = param_len + code_block.local_types().len();

                let mut locals = if value_stack.len()
                    >= (unsafe { stack_under.add(StackOffset::new(local_len)) }).as_usize()
                {
                    let (_, locals) = unsafe { value_stack.split_at_mut_unchecked(stack_under) };
                    locals
                } else {
                    let mut locals = StackFrame::new(
                        heap.alloc_slice(usize::max(INITIAL_VALUE_STACK_SIZE, local_len)),
                    );
                    for (local, value) in locals
                        .iter_mut()
                        .skip(stack_under.as_usize())
                        .zip(value_stack.iter())
                    {
                        *local = *value;
                    }
                    locals
                };

                for (_, local) in
                    (0..code_block.local_types().len()).zip(locals.iter_mut().skip(param_len))
                {
                    *local = WasmUnionValue::zero();
                }

                self._interpret(
                    target.index(),
                    code_block,
                    locals.as_locals(),
                    result_types,
                    heap,
                )
                .and_then(|v| {
                    if let Some(result) = v {
                        let var = value_stack.get_mut(stack_under);
                        *var = WasmUnionValue::from(result);
                    }
                    self.func_index = current_function;
                    Ok(())
                })
            }),
            WasmFunctionContent::Dynamic(function) => {
                let locals = unsafe { value_stack.get_range(stack_under, param_len) };
                let result = match function(module, locals) {
                    Ok(v) => v,
                    Err(e) => return Err(self.error(e, opcode, ex_position)),
                };

                if let Some(t) = result_types.first() {
                    if result.is_valid_type(*t) {
                        let var = value_stack.get_mut(stack_under);
                        *var = WasmUnionValue::from(result);
                    } else {
                        return Err(self.error(
                            WasmRuntimeErrorKind::TypeMismatch,
                            opcode,
                            ex_position,
                        ));
                    }
                }
                Ok(())
            }

            WasmFunctionContent::Unresolved => {
                Err(self.error(WasmRuntimeErrorKind::NoMethod, opcode, ex_position))
            }
        }
    }
}

struct WasmIntermediateCodeStream<'a> {
    codes: &'a [WasmImc],
    position: usize,
}

impl<'a> WasmIntermediateCodeStream<'a> {
    #[inline]
    fn from_codes(codes: &'a [WasmImc]) -> Option<Self> {
        if let Some(last) = codes.last() {
            if matches!(last.mnemonic(), WasmIntMnemonic::Unreachable(_)) {
                return Some(Self { codes, position: 0 });
            }
        }
        None
    }
}

impl WasmIntermediateCodeStream<'_> {
    #[inline]
    fn fetch(&mut self) -> &WasmImc {
        let code = unsafe { self.codes.get_unchecked(self.position) };
        self.position += 1;
        code
    }

    #[inline]
    fn set_position(&mut self, val: usize) -> Result<(), WasmRuntimeErrorKind> {
        if val < self.codes.len() {
            self.position = val;
            Ok(())
        } else {
            Err(WasmRuntimeErrorKind::InternalInconsistency)
        }
    }
}

pub trait WasmInvocation {
    fn invoke(&self, params: &[WasmValue]) -> Result<Option<WasmValue>, WasmRuntimeError>;
}

impl WasmInvocation for WasmRunnable<'_> {
    fn invoke(&self, params: &[WasmValue]) -> Result<Option<WasmValue>, WasmRuntimeError> {
        let function = self.function();

        let code_block = match function.content() {
            WasmFunctionContent::CodeBlock(v) => Ok(v),
            WasmFunctionContent::Dynamic(_) => Err(WasmRuntimeErrorKind::InvalidParameter),
            WasmFunctionContent::Unresolved => Err(WasmRuntimeErrorKind::NoMethod),
        }?;

        let mut locals =
            Vec::with_capacity(function.param_types().len() + code_block.local_types().len());

        for (index, param_type) in function.param_types().iter().enumerate() {
            let param = params.get(index).ok_or(WasmRuntimeError::from(
                WasmRuntimeErrorKind::InvalidParameter,
            ))?;
            if !param.is_valid_type(*param_type) {
                return Err(WasmRuntimeErrorKind::InvalidParameter.into());
            }
            locals.push(WasmUnionValue::from(*param));
        }

        locals.extend(iter::repeat(WasmUnionValue::zero()).take(code_block.local_types().len()));

        let result_types = function.result_types();

        let mut interp = WasmInterpreter::new(self.module());
        interp.invoke(
            function.index(),
            code_block,
            locals.as_slice(),
            result_types,
        )
    }
}

pub struct WasmRuntimeError {
    kind: WasmRuntimeErrorKind,
    file_position: usize,
    function: usize,
    function_name: Option<String>,
    position: usize,
    opcode: WasmOpcode,
}

impl WasmRuntimeError {
    #[inline]
    pub const fn kind(&self) -> &WasmRuntimeErrorKind {
        &self.kind
    }

    #[inline]
    pub const fn file_position(&self) -> usize {
        self.file_position
    }

    #[inline]
    pub const fn function(&self) -> usize {
        self.function
    }

    #[inline]
    pub fn function_name(&self) -> Option<&str> {
        self.function_name.as_ref().map(|v| v.as_str())
    }

    #[inline]
    pub const fn position(&self) -> usize {
        self.position
    }

    #[inline]
    pub const fn opcode(&self) -> WasmOpcode {
        self.opcode
    }
}

impl From<WasmRuntimeErrorKind> for WasmRuntimeError {
    #[inline]
    fn from(kind: WasmRuntimeErrorKind) -> Self {
        Self {
            kind,
            file_position: 0,
            function: 0,
            function_name: None,
            position: 0,
            opcode: WasmSingleOpcode::Unreachable.into(),
        }
    }
}

impl fmt::Debug for WasmRuntimeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let opcode = self.opcode();
        write!(f, "{:?} at", self.kind())?;
        if let Some(function_name) = self.function_name() {
            write!(
                f,
                " {}(${}):{}",
                function_name,
                self.function(),
                self.position(),
            )?;
        } else {
            write!(f, " ${}:{}", self.function(), self.position(),)?;
        }

        write!(f, ", 0x{:x}: {:?}", self.file_position(), opcode)
    }
}

#[repr(transparent)]
pub struct StackFrame<'a>(&'a mut [WasmUnionValue]);

impl<'a> StackFrame<'a> {
    #[inline]
    pub fn new(slice: &'a mut [WasmUnionValue]) -> Self {
        slice.fill(WasmUnionValue::zero());
        Self(slice)
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &WasmUnionValue> {
        self.0.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut WasmUnionValue> {
        self.0.iter_mut()
    }

    #[inline]
    pub fn as_locals(&'a mut self) -> LocalVariables<'a> {
        LocalVariables::new(self.0)
    }

    /// # Safety
    ///
    /// Since stack-level verification is guaranteed by the code verifier
    #[inline]
    pub unsafe fn split_at_mut_unchecked(&mut self, index: StackLevel) -> (StackFrame, StackFrame) {
        let (l, r) = unsafe { self.0.split_at_mut_unchecked(index.as_usize()) };
        (StackFrame::new(l), StackFrame::new(r))
    }

    /// # Safety
    ///
    /// Since stack-level verification is guaranteed by the code verifier
    #[inline]
    pub fn get(&self, index: StackLevel) -> &WasmUnionValue {
        unsafe { self.0.get_unchecked(index.as_usize()) }
    }

    /// # Safety
    ///
    /// Since stack-level verification is guaranteed by the code verifier
    #[inline]
    pub fn get_mut(&mut self, index: StackLevel) -> &mut WasmUnionValue {
        unsafe { self.0.get_unchecked_mut(index.as_usize()) }
    }

    /// # Safety
    ///
    /// Since stack-level verification is guaranteed by the code verifier
    pub unsafe fn get_range(&mut self, offset: StackLevel, size: usize) -> &[WasmUnionValue] {
        let offset = offset.as_usize();
        unsafe { self.0.get_unchecked_mut(offset..offset + size) }
    }
}

#[repr(transparent)]
pub struct LocalVariables<'a>(&'a mut [WasmUnionValue]);

impl<'a> LocalVariables<'a> {
    #[inline]
    pub fn new(slice: &'a mut [WasmUnionValue]) -> Self {
        Self(slice)
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// # Safety
    ///
    /// Because the range is guaranteed by the code verifier
    #[inline]
    pub fn get_local(&self, index: LocalVarIndex) -> &WasmUnionValue {
        unsafe { self.0.get_unchecked(index.as_usize()) }
    }

    /// # Safety
    ///
    /// Because the range is guaranteed by the code verifier
    #[inline]
    pub fn get_local_mut(&mut self, index: LocalVarIndex) -> &mut WasmUnionValue {
        unsafe { self.0.get_unchecked_mut(index.as_usize()) }
    }
}

#[derive(Clone, Copy)]
pub union StackTop {
    usize: usize,
    u32: u32,
    i32: i32,
    u64: u64,
    i64: i64,
    f32: f32,
    f64: f64,
}

impl StackTop {
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
        Self { usize: v as usize }
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
    pub const unsafe fn get_u32(&self) -> u32 {
        unsafe { self.u32 }
    }

    #[inline]
    pub const unsafe fn get_i32(&self) -> i32 {
        unsafe { self.i32 }
    }

    #[inline]
    pub unsafe fn write_i32(&mut self, val: i32) {
        unsafe {
            self.copy_from_i32(&Self::from(val));
        }
    }

    #[inline]
    pub unsafe fn write_u32(&mut self, val: u32) {
        unsafe {
            self.copy_from_i32(&Self::from(val));
        }
    }

    #[inline]
    pub const unsafe fn get_u64(&self) -> u64 {
        unsafe { self.u64 }
    }

    #[inline]
    pub const unsafe fn get_i64(&self) -> i64 {
        unsafe { self.i64 }
    }

    #[inline]
    pub unsafe fn write_i64(&mut self, val: i64) {
        *self = Self::from(val);
    }

    #[inline]
    pub unsafe fn write_u64(&mut self, val: u64) {
        *self = Self::from(val);
    }

    #[inline]
    pub const unsafe fn get_f32(&self) -> f32 {
        unsafe { self.f32 }
    }

    #[inline]
    pub const unsafe fn get_f64(&self) -> f64 {
        unsafe { self.f64 }
    }

    #[inline]
    pub unsafe fn write_f32(&mut self, val: f32) {
        if Self::_is_32bit_env() {
            self.f32 = val;
        } else {
            *self = Self::from(val);
        }
    }

    #[inline]
    pub unsafe fn write_f64(&mut self, val: f64) {
        *self = Self::from(val);
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

    #[inline]
    pub unsafe fn copy_from_i32(&mut self, other: &Self) {
        if Self::_is_32bit_env() {
            self.u32 = unsafe { other.u32 };
        } else {
            *self = *other;
        }
    }

    #[inline]
    pub fn from_union(v: &mut WasmUnionValue) -> &mut Self {
        unsafe { transmute(v) }
    }

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

    #[inline]
    pub unsafe fn map_u32<F>(&mut self, f: F)
    where
        F: FnOnce(u32) -> u32,
    {
        let val = unsafe { self.u32 };
        unsafe { self.copy_from_i32(&Self::from(f(val))) };
    }

    #[inline]
    pub unsafe fn map_i64<F>(&mut self, f: F)
    where
        F: FnOnce(i64) -> i64,
    {
        let val = unsafe { self.i64 };
        *self = Self::from(f(val));
    }

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
        unsafe {
            self.write_f32(f(val));
        }
    }

    #[inline]
    pub unsafe fn map_f64<F>(&mut self, f: F)
    where
        F: FnOnce(f64) -> f64,
    {
        let val = unsafe { self.f64 };
        *self = Self::from(f(val));
    }
}

impl From<bool> for StackTop {
    #[inline]
    fn from(v: bool) -> Self {
        Self::from_bool(v)
    }
}

impl From<u32> for StackTop {
    #[inline]
    fn from(v: u32) -> Self {
        Self::from_u32(v)
    }
}

impl From<i32> for StackTop {
    #[inline]
    fn from(v: i32) -> Self {
        Self::from_i32(v)
    }
}

impl From<u64> for StackTop {
    #[inline]
    fn from(v: u64) -> Self {
        Self::from_u64(v)
    }
}

impl From<i64> for StackTop {
    #[inline]
    fn from(v: i64) -> Self {
        Self::from_i64(v)
    }
}

impl From<f32> for StackTop {
    #[inline]
    fn from(v: f32) -> Self {
        Self::from_f32(v)
    }
}

impl From<f64> for StackTop {
    #[inline]
    fn from(v: f64) -> Self {
        Self::from_f64(v)
    }
}
