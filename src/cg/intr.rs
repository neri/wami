//! WebAssembly Intermediate Code Interpreter

use super::intcode::*;
use super::*;
use crate::memory::WasmMemory;
use crate::stack::*;
use crate::wasm::*;
use crate::*;
use core::error::Error;
use core::iter;
use core::mem::{size_of, transmute};
use core::ops::Neg;
use libm::{ceil, ceilf, floor, floorf, rint, rintf, trunc, truncf};

const INITIAL_VALUE_STACK_SIZE: usize = 512;

/// Wasm Intermediate Code Interpreter
pub struct WasmInterpreter<'a> {
    instance: &'a WasmInstance,
    func_index: usize,
}

impl<'a> WasmInterpreter<'a> {
    #[inline]
    pub fn new(instance: &'a WasmInstance) -> Self {
        Self {
            instance,
            func_index: 0,
        }
    }
}

impl WasmInterpreter<'_> {
    #[inline]
    fn error(
        &mut self,
        kind: WasmRuntimeErrorKind,
        mnemonic: WasmMnemonic,
        ex_position: ExceptionPosition,
    ) -> Box<dyn Error> {
        let function_name = self
            .instance
            .module()
            .names()
            .and_then(|v| v.func_by_index(self.func_index))
            .map(|v| v.to_owned());
        let file_position = self
            .instance
            .module()
            .func_position(self.func_index)
            .unwrap_or(0)
            + ex_position.position();

        Box::new(WasmRuntimeError {
            kind,
            file_position,
            function: self.func_index,
            function_name,
            position: ex_position.position(),
            mnemonic,
        })
    }

    #[inline]
    pub fn invoke(
        &mut self,
        func_index: usize,
        code_block: &WasmCodeBlock,
        locals: &[WasmUnionValue],
        result_types: &[WasmValType],
    ) -> Result<Option<WasmValue>, Box<dyn Error>> {
        if locals.len() < code_block.local_types().len() {
            return Err(WasmRuntimeErrorKind::InvalidParameter.into());
        }
        let mut heap = StackHeap::with_capacity(0x10000);
        let local2 = heap.alloc_slice(locals.len(), WasmUnionValue::zero());
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
    ) -> Result<Option<WasmValue>, Box<dyn Error>> {
        macro_rules! GET_MEMORY {
            ($self:ident) => {
                $self
                    .instance
                    .module()
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
            .ok_or(WasmRuntimeErrorKind::InternalInconsistency)?;

        let mut value_stack = StackFrame::new(heap.alloc_slice(
            code_block.max_value_stack().as_usize(),
            WasmUnionValue::zero(),
        ));

        let mut result_stack_level = StackLevel::zero();

        let mut memory = BORROW_MEMORY!(self)?;

        macro_rules! MEM_LOAD {
            ($stor_type:ident, $data_type:ident, $mnemonic:ident, $offset:ident, $ex_position:ident, $code:ident, $value_stack:ident, $memory:ident, ) => {
                #[cfg(test)]
                assert_matches!($code.instruction(), WasmImInstruction::$mnemonic(_, _));

                let var = $value_stack.get_mut($code.base_stack_level());
                let index = unsafe { var.get_u32() };
                let ea = WasmMemory::effective_address::<$data_type>($offset, index, $memory.len())
                    .map_err(|e| self.error(e, WasmMnemonic::$mnemonic, $ex_position))?;

                unsafe {
                    let p = $memory.as_ptr().byte_add(ea) as *const $data_type;
                    let data = p.read_volatile() as $stor_type;
                    *var = data.into();
                }
            };
        }

        macro_rules! MEM_STORE {
            ($stor_type:ident, $data_type:ident, $mnemonic:ident, $offset:ident, $ex_position:ident, $code:ident, $value_stack:ident, $memory:ident, ) => {
                #[cfg(test)]
                assert_matches!($code.instruction(), WasmImInstruction::$mnemonic(_, _));

                let stack_level = $code.base_stack_level();
                let index = unsafe { $value_stack.get(stack_level).get_u32() };
                let storage: $stor_type =
                    unsafe { $value_stack.get(stack_level.succ(1)).unsafe_into() };
                let ea = WasmMemory::effective_address::<$data_type>($offset, index, $memory.len())
                    .map_err(|e| self.error(e, WasmMnemonic::$mnemonic, $ex_position))?;
                unsafe {
                    let p = $memory.as_mut_ptr().byte_add(ea) as *mut $data_type;
                    p.write_volatile(storage as $data_type);
                }
            };
        }

        macro_rules! DIV_OP {
            ($map_lhs:ident, $get_rhs:ident, $mnemonic:ident, $opr:ident, $ex_position:ident, $code:ident, $value_stack:ident, ) => {
                #[cfg(test)]
                assert_matches!($code.instruction(), WasmImInstruction::$mnemonic(_));

                let stack_level = $code.base_stack_level();
                let rhs = unsafe { $value_stack.get(stack_level.succ(1)).$get_rhs() };
                let lhs = $value_stack.get_mut(stack_level);
                if rhs == 0 {
                    return Err(self.error(
                        WasmRuntimeErrorKind::DivideByZero,
                        WasmMnemonic::$mnemonic,
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

            match *code.instruction() {
                WasmImInstruction::Unreachable(position) => {
                    return Err(self.error(
                        WasmRuntimeErrorKind::Unreachable,
                        WasmMnemonic::Unreachable,
                        position,
                    ));
                }

                WasmImInstruction::NotSupported(mnemonic, position) => {
                    return Err(self.error(WasmRuntimeErrorKind::NotSupported, mnemonic, position));
                }

                WasmImInstruction::Marker(_, _) => {
                    return Err(self.error(
                        WasmRuntimeErrorKind::InternalInconsistency,
                        WasmMnemonic::Nop,
                        ExceptionPosition::UNKNOWN,
                    ));
                }

                WasmImInstruction::If(target) => {
                    let cc = unsafe { value_stack.get(code.base_stack_level()).get_bool() };
                    if !cc {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::Br(target) => {
                    codes.set_position(target)?;
                }
                WasmImInstruction::BrIf(target) => {
                    let cc = unsafe { value_stack.get(code.base_stack_level()).get_bool() };
                    if cc {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::BrTable(ref table) => {
                    let index = (table.len() - 1).min(unsafe {
                        value_stack.get(code.base_stack_level()).get_u32() as usize
                    });
                    let target = unsafe { *table.get_unchecked(index) };
                    codes.set_position(target)?;
                }

                WasmImInstruction::BrUnwind(target, target_stack_level) => {
                    let source_stack_level = code.base_stack_level();
                    value_stack.set(target_stack_level, *value_stack.get(source_stack_level));
                    codes.set_position(target)?;
                }
                WasmImInstruction::BrIfUnwind(target, target_stack_level) => {
                    let cc = unsafe { value_stack.get(code.base_stack_level().succ(1)).get_bool() };
                    if cc {
                        let source_stack_level = code.base_stack_level();
                        value_stack.set(target_stack_level, *value_stack.get(source_stack_level));
                        codes.set_position(target)?;
                    }
                }

                WasmImInstruction::ReturnN => {
                    break;
                }
                WasmImInstruction::ReturnI | WasmImInstruction::ReturnF => {
                    result_stack_level = code.base_stack_level();
                    break;
                }

                WasmImInstruction::Call(func_index, ex_position) => {
                    let func =
                        unsafe { self.instance.module().functions().get_unchecked(func_index) };
                    drop(memory);
                    self.call(
                        WasmMnemonic::Call,
                        ex_position,
                        code.base_stack_level(),
                        func,
                        &mut value_stack,
                        heap,
                    )?;
                    memory = BORROW_MEMORY!(self)?;
                }
                WasmImInstruction::CallIndirect(type_index, ex_position) => {
                    let opcode = WasmMnemonic::CallIndirect;
                    let index =
                        unsafe { value_stack.get(code.base_stack_level()).get_i32() as usize };
                    let func = self.instance.module().elem_get(index).ok_or(self.error(
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

                WasmImInstruction::SelectI | WasmImInstruction::SelectF => {
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

                WasmImInstruction::LocalGetI(local_index)
                | WasmImInstruction::LocalGetF(local_index) => {
                    let local = locals.get_local(local_index);
                    let ref_a = value_stack.get_mut(code.base_stack_level());
                    *ref_a = *local;
                }
                WasmImInstruction::LocalSetI(local_index)
                | WasmImInstruction::LocalTeeI(local_index)
                | WasmImInstruction::LocalSetF(local_index)
                | WasmImInstruction::LocalTeeF(local_index) => {
                    let local = locals.get_local_mut(local_index);
                    let ref_a = value_stack.get(code.base_stack_level());
                    *local = *ref_a;
                }

                WasmImInstruction::GlobalGetI(global_ref)
                | WasmImInstruction::GlobalGetF(global_ref) => {
                    let global = self.instance.module().global_get(global_ref);
                    let ref_a = value_stack.get_mut(code.base_stack_level());
                    *ref_a = global.raw_value();
                }
                WasmImInstruction::GlobalSetI(global_ref)
                | WasmImInstruction::GlobalSetF(global_ref) => {
                    let global = self.instance.module().global_get(global_ref);
                    let ref_a = value_stack.get(code.base_stack_level());
                    global.set_raw_value(*ref_a);
                }

                WasmImInstruction::I32Load(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(u32, u32, I32Load, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::I32Load8S(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(i32, i8, I32Load8S, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::I32Load8U(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(u32, u8, I32Load8U, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::I32Load16S(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(i32, i16, I32Load16S, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::I32Load16U(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(u32, u16, I32Load16U, offset, ex_position, code, value_stack, memory, );
                }

                WasmImInstruction::I64Load(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(u64, u64, I64Load, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::I64Load8S(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(i64, i8, I64Load8S, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::I64Load8U(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(u64, u8, I64Load8U, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::I64Load16S(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(i64, i16, I64Load16S, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::I64Load16U(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(u64, u16, I64Load16U, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::I64Load32S(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(i64, i32, I64Load32S, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::I64Load32U(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(u64, u32, I64Load32U, offset, ex_position, code, value_stack, memory, );
                }

                WasmImInstruction::I32Store(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(u32, u32, I32Store, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::I32Store8(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(u32, u8, I32Store8, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::I32Store16(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(u32, u16, I32Store16, offset, ex_position, code, value_stack, memory, );
                }

                WasmImInstruction::I64Store(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(u64, u64, I64Store, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::I64Store8(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(u64, u8, I64Store8, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::I64Store16(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(u64, u16, I64Store16, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::I64Store32(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(u64, u32, I64Store32, offset, ex_position, code, value_stack, memory, );
                }

                WasmImInstruction::F32Load(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(f32, f32, F32Load, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::F32Store(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(f32, f32, F32Store, offset, ex_position, code, value_stack, memory, );
                }

                WasmImInstruction::F64Load(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_LOAD!(f64, f64, F64Load, offset, ex_position, code, value_stack, memory, );
                }
                WasmImInstruction::F64Store(offset, ex_position) => {
                    #[rustfmt::skip]
                    MEM_STORE!(f64, f64, F64Store, offset, ex_position, code, value_stack, memory, );
                }

                WasmImInstruction::MemorySize => {
                    let ref_a = value_stack.get_mut(code.base_stack_level());
                    ref_a.write_i32((memory.len() / WebAssembly::PAGE_SIZE) as i32);
                }
                WasmImInstruction::MemoryGrow => {
                    let ref_a = value_stack.get_mut(code.base_stack_level());
                    drop(memory);

                    let mem = GET_MEMORY!(self)?;
                    unsafe {
                        ref_a.map_u32(|v| mem.grow(v).unwrap_or(u32::MAX));
                    }

                    memory = BORROW_MEMORY!(self)?;
                }
                WasmImInstruction::MemoryCopy(ex_position) => {
                    let stack_level = code.base_stack_level();
                    let dest = unsafe { value_stack.get(stack_level).get_u32() } as usize;
                    let src = unsafe { value_stack.get(stack_level.succ(1)).get_u32() } as usize;
                    let count = unsafe { value_stack.get(stack_level.succ(2)).get_u32() } as usize;

                    WasmMemory::check_bound(dest as u64, count, memory.len())
                        .map_err(|k| self.error(k, WasmMnemonic::MemoryCopy, ex_position))?;
                    WasmMemory::check_bound(src as u64, count, memory.len())
                        .map_err(|k| self.error(k, WasmMnemonic::MemoryCopy, ex_position))?;

                    if count > 0 {
                        unsafe {
                            memory
                                .as_mut_ptr()
                                .add(dest)
                                .copy_from(memory.as_ptr().add(src), count);
                        }
                    }
                }
                WasmImInstruction::MemoryFill(ex_position) => {
                    let stack_level = code.base_stack_level();
                    let base = unsafe { value_stack.get(stack_level).get_u32() } as usize;
                    let val = unsafe { value_stack.get(stack_level.succ(1)).get_u8() };
                    let count = unsafe { value_stack.get(stack_level.succ(2)).get_u32() } as usize;

                    WasmMemory::check_bound(base as u64, count, memory.len())
                        .map_err(|k| self.error(k, WasmMnemonic::MemoryFill, ex_position))?;

                    if count > 0 {
                        unsafe {
                            memory.as_mut_ptr().add(base).write_bytes(val, count);
                        }
                    }
                }

                WasmImInstruction::I32Const(val) => {
                    let ref_a = value_stack.get_mut(code.base_stack_level());
                    ref_a.write_i32(val);
                }
                WasmImInstruction::I64Const(val) => {
                    let ref_a = value_stack.get_mut(code.base_stack_level());
                    ref_a.write_i64(val);
                }
                WasmImInstruction::F32Const(val) => {
                    let ref_a = value_stack.get_mut(code.base_stack_level());
                    ref_a.write_f32(val);
                }
                WasmImInstruction::F64Const(val) => {
                    let ref_a = value_stack.get_mut(code.base_stack_level());
                    ref_a.write_f64(val);
                }

                WasmImInstruction::I32Eqz => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_bool(var.get_i32() == 0)
                    });
                }
                WasmImInstruction::I32Eq => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u32() == rhs.get_u32())
                    });
                }
                WasmImInstruction::I32Ne => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u32() != rhs.get_u32())
                    });
                }
                WasmImInstruction::I32LtS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_i32() < rhs.get_i32())
                    });
                }
                WasmImInstruction::I32LtU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u32() < rhs.get_u32())
                    });
                }
                WasmImInstruction::I32GtS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_i32() > rhs.get_i32())
                    });
                }
                WasmImInstruction::I32GtU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u32() > rhs.get_u32())
                    });
                }
                WasmImInstruction::I32LeS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_i32() <= rhs.get_i32())
                    });
                }
                WasmImInstruction::I32LeU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u32() <= rhs.get_u32())
                    });
                }
                WasmImInstruction::I32GeS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_i32() >= rhs.get_i32())
                    });
                }
                WasmImInstruction::I32GeU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u32() >= rhs.get_u32())
                    });
                }

                WasmImInstruction::I32Clz => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.map_u32(|v| v.leading_zeros())
                    });
                }
                WasmImInstruction::I32Ctz => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.map_u32(|v| v.trailing_zeros())
                    });
                }
                WasmImInstruction::I32Popcnt => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.map_u32(|v| v.count_ones())
                    });
                }

                WasmImInstruction::I32Add => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_add(rhs.get_i32()));
                    });
                }
                WasmImInstruction::I32Sub => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_sub(rhs.get_i32()));
                    });
                }
                WasmImInstruction::I32Mul => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_mul(rhs.get_i32()));
                    });
                }

                WasmImInstruction::I32DivS(ex_position) => {
                    #[rustfmt::skip]
                    DIV_OP!(map_i32, get_i32, I32DivS, wrapping_div, ex_position, code, value_stack, );
                }
                WasmImInstruction::I32DivU(ex_position) => {
                    #[rustfmt::skip]
                    DIV_OP!(map_u32, get_u32, I32DivU, wrapping_div, ex_position, code, value_stack, );
                }
                WasmImInstruction::I32RemS(ex_position) => {
                    #[rustfmt::skip]
                    DIV_OP!(map_i32, get_i32, I32RemS, wrapping_rem, ex_position, code, value_stack, );
                }
                WasmImInstruction::I32RemU(ex_position) => {
                    #[rustfmt::skip]
                    DIV_OP!(map_u32, get_u32, I32RemU, wrapping_rem, ex_position, code, value_stack, );
                }

                WasmImInstruction::I32And => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u32(|lhs| lhs & rhs.get_u32());
                    });
                }
                WasmImInstruction::I32Or => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u32(|lhs| lhs | rhs.get_u32());
                    });
                }
                WasmImInstruction::I32Xor => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u32(|lhs| lhs ^ rhs.get_u32());
                    });
                }
                WasmImInstruction::I32Shl => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u32(|lhs| lhs.wrapping_shl(rhs.get_u32()));
                    });
                }
                WasmImInstruction::I32ShrS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_shr(rhs.get_u32()));
                    });
                }
                WasmImInstruction::I32ShrU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u32(|lhs| lhs.wrapping_shr(rhs.get_u32()));
                    });
                }
                WasmImInstruction::I32Rotl => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u32(|lhs| lhs.rotate_left(rhs.get_u32()));
                    });
                }
                WasmImInstruction::I32Rotr => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u32(|lhs| lhs.rotate_right(rhs.get_u32()));
                    });
                }

                WasmImInstruction::I64Eqz => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_bool(var.get_i64() == 0)
                    });
                }
                WasmImInstruction::I64Eq => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u64() == rhs.get_u64())
                    });
                }
                WasmImInstruction::I64Ne => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u64() != rhs.get_u64())
                    });
                }
                WasmImInstruction::I64LtS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_i64() < rhs.get_i64())
                    });
                }
                WasmImInstruction::I64LtU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u64() < rhs.get_u64())
                    });
                }
                WasmImInstruction::I64GtS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_i64() > rhs.get_i64())
                    });
                }
                WasmImInstruction::I64GtU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u64() > rhs.get_u64())
                    });
                }
                WasmImInstruction::I64LeS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_i64() <= rhs.get_i64())
                    });
                }
                WasmImInstruction::I64LeU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u64() <= rhs.get_u64())
                    });
                }
                WasmImInstruction::I64GeS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_i64() >= rhs.get_i64())
                    });
                }
                WasmImInstruction::I64GeU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_u64() >= rhs.get_u64())
                    });
                }

                WasmImInstruction::I64Clz => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.map_u64(|v| v.leading_zeros() as u64);
                    });
                }
                WasmImInstruction::I64Ctz => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.map_u64(|v| v.trailing_zeros() as u64);
                    });
                }
                WasmImInstruction::I64Popcnt => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.map_u64(|v| v.count_ones() as u64);
                    });
                }
                WasmImInstruction::I64Add => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_add(rhs.get_i64()));
                    });
                }
                WasmImInstruction::I64Sub => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_sub(rhs.get_i64()));
                    });
                }
                WasmImInstruction::I64Mul => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_mul(rhs.get_i64()));
                    });
                }

                WasmImInstruction::I64DivS(ex_position) => {
                    #[rustfmt::skip]
                    DIV_OP!(map_i64, get_i64, I64DivS, wrapping_div, ex_position, code, value_stack, );
                }
                WasmImInstruction::I64DivU(ex_position) => {
                    #[rustfmt::skip]
                    DIV_OP!(map_u64, get_u64, I64DivU, wrapping_div, ex_position, code, value_stack, );
                }
                WasmImInstruction::I64RemS(ex_position) => {
                    #[rustfmt::skip]
                    DIV_OP!(map_i64, get_i64, I64RemS, wrapping_rem, ex_position, code, value_stack, );
                }
                WasmImInstruction::I64RemU(ex_position) => {
                    #[rustfmt::skip]
                    DIV_OP!(map_u64, get_u64, I64RemU, wrapping_rem, ex_position, code, value_stack, );
                }

                WasmImInstruction::I64And => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u64(|lhs| lhs & rhs.get_u64());
                    });
                }
                WasmImInstruction::I64Or => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u64(|lhs| lhs | rhs.get_u64());
                    });
                }
                WasmImInstruction::I64Xor => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u64(|lhs| lhs ^ rhs.get_u64());
                    });
                }
                WasmImInstruction::I64Shl => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u64(|lhs| lhs.wrapping_shl(rhs.get_u64() as u32));
                    });
                }
                WasmImInstruction::I64ShrS => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_shr(rhs.get_u64() as u32));
                    });
                }
                WasmImInstruction::I64ShrU => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u64(|lhs| lhs.wrapping_shr(rhs.get_u64() as u32));
                    });
                }
                WasmImInstruction::I64Rotl => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u64(|lhs| lhs.rotate_left(rhs.get_u64() as u32));
                    });
                }
                WasmImInstruction::I64Rotr => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_u64(|lhs| lhs.rotate_right(rhs.get_u64() as u32));
                    });
                }

                WasmImInstruction::I32WrapI64 => {
                    Self::unary_op(code, &mut value_stack, |var| {
                        *var = StackTop::from_i32(unsafe { var.get_i64() as i32 });
                    });
                }
                WasmImInstruction::I32Extend8S => {
                    Self::unary_op(code, &mut value_stack, |var| {
                        *var = StackTop::from_i32(unsafe { var.get_i8() as i32 });
                    });
                }
                WasmImInstruction::I32Extend16S => {
                    Self::unary_op(code, &mut value_stack, |var| {
                        *var = StackTop::from_i32(unsafe { var.get_i16() as i32 });
                    });
                }
                WasmImInstruction::I64Extend8S => {
                    Self::unary_op(code, &mut value_stack, |var| {
                        *var = StackTop::from_i64(unsafe { var.get_i8() as i64 });
                    });
                }
                WasmImInstruction::I64Extend16S => {
                    Self::unary_op(code, &mut value_stack, |var| {
                        *var = StackTop::from_i64(unsafe { var.get_i16() as i64 });
                    });
                }
                WasmImInstruction::I64Extend32S | WasmImInstruction::I64ExtendI32S => {
                    Self::unary_op(code, &mut value_stack, |var| {
                        *var = StackTop::from_i64(unsafe { var.get_i32() as i64 });
                    });
                }
                WasmImInstruction::I64ExtendI32U => {
                    Self::unary_op(code, &mut value_stack, |var| {
                        *var = StackTop::from_u64(unsafe { var.get_u32() as u64 });
                    });
                }

                WasmImInstruction::F32Eq => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f32() == rhs.get_f32())
                    });
                }
                WasmImInstruction::F32Ne => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f32() != rhs.get_f32())
                    });
                }
                WasmImInstruction::F32Lt => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f32() < rhs.get_f32())
                    });
                }
                WasmImInstruction::F32Gt => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f32() > rhs.get_f32())
                    });
                }
                WasmImInstruction::F32Le => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f32() <= rhs.get_f32())
                    });
                }
                WasmImInstruction::F32Ge => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f32() >= rhs.get_f32())
                    });
                }

                WasmImInstruction::F32Abs => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f32(|v| core::intrinsics::fabsf32(v));
                    });
                }
                WasmImInstruction::F32Neg => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f32(|v| v.neg());
                    });
                }
                WasmImInstruction::F32Ceil => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f32(|v| ceilf(v));
                    });
                }
                WasmImInstruction::F32Floor => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f32(|v| floorf(v));
                    });
                }
                WasmImInstruction::F32Trunc => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f32(|v| truncf(v));
                    });
                }
                WasmImInstruction::F32Nearest => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f32(|v| rintf(v));
                    });
                }
                WasmImInstruction::F32Sqrt => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f32(|v| core::intrinsics::sqrtf32(v));
                    });
                }

                WasmImInstruction::F32Add => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f32(|lhs| lhs + rhs.get_f32());
                    });
                }
                WasmImInstruction::F32Sub => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f32(|lhs| lhs - rhs.get_f32());
                    });
                }
                WasmImInstruction::F32Mul => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f32(|lhs| lhs * rhs.get_f32());
                    });
                }
                WasmImInstruction::F32Div => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f32(|lhs| lhs / rhs.get_f32());
                    });
                }
                WasmImInstruction::F32Min => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f32(|lhs| lhs.minimum(rhs.get_f32()));
                    });
                }
                WasmImInstruction::F32Max => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f32(|lhs| lhs.maximum(rhs.get_f32()));
                    });
                }
                WasmImInstruction::F32Copysign => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f32(|lhs| core::intrinsics::copysignf32(lhs, rhs.get_f32()));
                    });
                }

                WasmImInstruction::F64Eq => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f64() == rhs.get_f64())
                    });
                }
                WasmImInstruction::F64Ne => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f64() != rhs.get_f64())
                    });
                }
                WasmImInstruction::F64Lt => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f64() < rhs.get_f64())
                    });
                }
                WasmImInstruction::F64Gt => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f64() > rhs.get_f64())
                    });
                }
                WasmImInstruction::F64Le => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f64() <= rhs.get_f64())
                    });
                }
                WasmImInstruction::F64Ge => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.write_bool(lhs.get_f64() >= rhs.get_f64())
                    });
                }

                WasmImInstruction::F64Abs => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f64(|v| core::intrinsics::fabsf64(v));
                    });
                }
                WasmImInstruction::F64Neg => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f64(|v| v.neg());
                    });
                }
                WasmImInstruction::F64Ceil => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f64(|v| ceil(v));
                    });
                }
                WasmImInstruction::F64Floor => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f64(|v| floor(v));
                    });
                }
                WasmImInstruction::F64Trunc => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f64(|v| trunc(v));
                    });
                }
                WasmImInstruction::F64Nearest => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f64(|v| rint(v));
                    });
                }
                WasmImInstruction::F64Sqrt => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.map_f64(|v| core::intrinsics::sqrtf64(v));
                    });
                }

                WasmImInstruction::F64Add => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f64(|lhs| lhs + rhs.get_f64());
                    });
                }
                WasmImInstruction::F64Sub => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f64(|lhs| lhs - rhs.get_f64());
                    });
                }
                WasmImInstruction::F64Mul => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f64(|lhs| lhs * rhs.get_f64());
                    });
                }
                WasmImInstruction::F64Div => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f64(|lhs| lhs / rhs.get_f64());
                    });
                }
                WasmImInstruction::F64Min => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f64(|lhs| lhs.minimum(rhs.get_f64()));
                    });
                }
                WasmImInstruction::F64Max => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f64(|lhs| lhs.maximum(rhs.get_f64()));
                    });
                }
                WasmImInstruction::F64Copysign => {
                    Self::binary_op(code, &mut value_stack, |lhs, rhs| unsafe {
                        lhs.map_f64(|lhs| core::intrinsics::copysignf64(lhs, rhs.get_f64()));
                    });
                }

                WasmImInstruction::I32TruncF32S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_i32(var.get_f32().to_int_unchecked());
                    });
                }
                WasmImInstruction::I32TruncF32U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_u32(var.get_f32().to_int_unchecked());
                    });
                }
                WasmImInstruction::I32TruncF64S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_i32(var.get_f64().to_int_unchecked());
                    });
                }
                WasmImInstruction::I32TruncF64U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_u32(var.get_f64().to_int_unchecked());
                    });
                }

                WasmImInstruction::I64TruncF32S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_i64(var.get_f32().to_int_unchecked());
                    });
                }
                WasmImInstruction::I64TruncF32U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_u64(var.get_f32().to_int_unchecked());
                    });
                }
                WasmImInstruction::I64TruncF64S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_i64(var.get_f64().to_int_unchecked());
                    });
                }
                WasmImInstruction::I64TruncF64U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_u64(var.get_f64().to_int_unchecked());
                    });
                }

                WasmImInstruction::I32TruncSatF32S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_i32(var.get_f32() as i32);
                    });
                }
                WasmImInstruction::I32TruncSatF32U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_u32(var.get_f32() as u32);
                    });
                }
                WasmImInstruction::I32TruncSatF64S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_i32(var.get_f64() as i32);
                    });
                }
                WasmImInstruction::I32TruncSatF64U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_u32(var.get_f64() as u32);
                    });
                }
                WasmImInstruction::I64TruncSatF32S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_i64(var.get_f32() as i64);
                    });
                }
                WasmImInstruction::I64TruncSatF32U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_u64(var.get_f32() as u64);
                    });
                }
                WasmImInstruction::I64TruncSatF64S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_i64(var.get_f64() as i64);
                    });
                }
                WasmImInstruction::I64TruncSatF64U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_u64(var.get_f64() as u64);
                    });
                }

                WasmImInstruction::F32ConvertI32S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f32(var.get_i32() as f32);
                    });
                }
                WasmImInstruction::F32ConvertI32U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f32(var.get_u32() as f32);
                    });
                }
                WasmImInstruction::F32ConvertI64S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f32(var.get_i64() as f32);
                    });
                }
                WasmImInstruction::F32ConvertI64U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f32(var.get_u64() as f32);
                    });
                }
                WasmImInstruction::F32DemoteF64 => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f32(var.get_f64() as f32);
                    });
                }

                WasmImInstruction::F64ConvertI32S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f64(var.get_i32() as f64);
                    });
                }
                WasmImInstruction::F64ConvertI32U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f64(var.get_u32() as f64);
                    });
                }
                WasmImInstruction::F64ConvertI64S => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f64(var.get_i64() as f64);
                    });
                }
                WasmImInstruction::F64ConvertI64U => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f64(var.get_u64() as f64);
                    });
                }
                WasmImInstruction::F64PromoteF32 => {
                    Self::unary_op(code, &mut value_stack, |var| unsafe {
                        var.write_f64(var.get_f32().into());
                    });
                }

                WasmImInstruction::I32ReinterpretF32 => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.write_u32(v.get_f32().to_bits());
                    })
                }
                WasmImInstruction::I64ReinterpretF64 => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.write_u64(v.get_f64().to_bits());
                    })
                }
                WasmImInstruction::F32ReinterpretI32 => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.write_f32(f32::from_bits(v.get_u32()));
                    })
                }
                WasmImInstruction::F64ReinterpretI64 => {
                    Self::unary_op(code, &mut value_stack, |v| unsafe {
                        v.write_f64(f64::from_bits(v.get_u64()));
                    })
                }

                WasmImInstruction::FusedI32AddConst(local_index, val) => {
                    let local = locals.get_local_mut(local_index);
                    unsafe {
                        local.map_i32(|a| a.wrapping_add(val));
                    }
                }

                WasmImInstruction::FusedI32SetConst(local_index, val) => {
                    let local = locals.get_local_mut(local_index);
                    local.write_i32(val);
                }
                WasmImInstruction::FusedI64SetConst(local_index, val) => {
                    let local = locals.get_local_mut(local_index);
                    *local = val.into();
                }

                WasmImInstruction::FusedI32AddI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_add(val));
                    });
                }
                WasmImInstruction::FusedI32AndI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u32(|lhs| lhs & val);
                    });
                }
                WasmImInstruction::FusedI32OrI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u32(|lhs| lhs | val);
                    });
                }
                WasmImInstruction::FusedI32XorI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u32(|lhs| lhs ^ val);
                    });
                }
                WasmImInstruction::FusedI32ShlI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_shl(val));
                    });
                }
                WasmImInstruction::FusedI32ShrSI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_shr(val));
                    });
                }
                WasmImInstruction::FusedI32ShrUI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u32(|lhs| lhs.wrapping_shr(val));
                    });
                }

                WasmImInstruction::FusedI64AddI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_add(val));
                    });
                }
                WasmImInstruction::FusedI64AndI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u64(|lhs| lhs & val);
                    });
                }
                WasmImInstruction::FusedI64OrI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u64(|lhs| lhs | val);
                    });
                }
                WasmImInstruction::FusedI64XorI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u64(|lhs| lhs ^ val);
                    });
                }
                WasmImInstruction::FusedI64ShlI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u64(|lhs| lhs.wrapping_shl(val));
                    });
                }
                WasmImInstruction::FusedI64ShrSI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_shr(val));
                    });
                }
                WasmImInstruction::FusedI64ShrUI(val) => {
                    Self::unary_op(code, &mut value_stack, |lhs| unsafe {
                        lhs.map_u64(|lhs| lhs.wrapping_shr(val));
                    });
                }

                WasmImInstruction::FusedI32BrZ(target) => {
                    let cc = unsafe { value_stack.get_mut(code.base_stack_level()).get_i32() == 0 };
                    if cc {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI32BrEq(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u32() == rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI32BrNe(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u32() != rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI32BrLtS(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_i32() < rhs.get_i32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI32BrLtU(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u32() < rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI32BrGtS(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_i32() > rhs.get_i32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI32BrGtU(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u32() > rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI32BrLeS(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_i32() <= rhs.get_i32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI32BrLeU(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u32() <= rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI32BrGeS(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_i32() >= rhs.get_i32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI32BrGeU(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u32() >= rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }

                WasmImInstruction::FusedI64BrZ(target) => {
                    let cc = unsafe { value_stack.get_mut(code.base_stack_level()).get_i64() == 0 };
                    if cc {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI64BrEq(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u64() == rhs.get_u64() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI64BrNe(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u64() != rhs.get_u64() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI64BrLtS(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_i64() < rhs.get_i64() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI64BrLtU(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u64() < rhs.get_u64() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI64BrGtS(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_i64() > rhs.get_i64() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI64BrGtU(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u64() > rhs.get_u64() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI64BrLeS(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_i64() <= rhs.get_i64() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI64BrLeU(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u64() <= rhs.get_u64() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI64BrGeS(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_i64() >= rhs.get_i64() } {
                        codes.set_position(target)?;
                    }
                }
                WasmImInstruction::FusedI64BrGeU(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get(stack_level.succ(1)) };
                    let lhs = *value_stack.get(stack_level);
                    if unsafe { lhs.get_u64() >= rhs.get_u64() } {
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
        opcode: WasmMnemonic,
        ex_position: ExceptionPosition,
        stack_pointer: StackLevel,
        target: &WasmFunction,
        value_stack: &mut StackFrame,
        heap: &mut StackHeap,
    ) -> Result<(), Box<dyn Error>> {
        let current_function = self.func_index;
        let result_types = target.result_types();

        let param_len = target.param_types().len();
        let stack_under = unsafe { stack_pointer.sub(StackOffset::new(param_len)) };

        match target.content() {
            WasmFunctionContent::CodeBlock(code_block) => heap.snapshot(|heap| {
                let local_len = code_block.local_types().len();

                let mut locals = if value_stack.len() >= (stack_under.as_usize() + local_len) {
                    let (_, mut locals) =
                        unsafe { value_stack.split_at_mut_unchecked(stack_under) };

                    for local in locals.iter_mut().skip(param_len) {
                        *local = WasmUnionValue::zero();
                    }

                    locals
                } else {
                    let mut locals = StackFrame::new(heap.alloc_slice(
                        local_len.max(INITIAL_VALUE_STACK_SIZE),
                        WasmUnionValue::zero(),
                    ));

                    for (local, value) in locals
                        .iter_mut()
                        .zip(unsafe { value_stack.get_range(stack_under, param_len) }.iter())
                    {
                        *local = *value;
                    }

                    locals
                };

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
            WasmFunctionContent::Dynamic(func) => {
                let locals = unsafe { value_stack.get_range(stack_under, param_len) };
                func(self.instance, WasmArgs::new(&locals)).and_then(|val| {
                    match (val, result_types.first()) {
                        (None, None) => Ok(()),
                        (Some(val), Some(t)) => {
                            if val.is_valid_type(*t) {
                                let var = value_stack.get_mut(stack_under);
                                *var = val.into();
                                Ok(())
                            } else {
                                Err(self.error(
                                    WasmRuntimeErrorKind::TypeMismatch,
                                    opcode,
                                    ex_position,
                                ))
                            }
                        }
                        _ => {
                            Err(self.error(WasmRuntimeErrorKind::TypeMismatch, opcode, ex_position))
                        }
                    }
                })
            }
            WasmFunctionContent::Unresolved => {
                Err(self.error(WasmRuntimeErrorKind::NoMethod, opcode, ex_position))
            }
        }
    }
}

struct WasmIntermediateCodeStream<'a> {
    codes: &'a [WasmImc],
    position: u32,
}

impl<'a> WasmIntermediateCodeStream<'a> {
    #[inline]
    fn from_codes(codes: &'a [WasmImc]) -> Option<Self> {
        if let Some(last) = codes.last() {
            if matches!(last.instruction(), WasmImInstruction::Unreachable(_)) {
                return Some(Self { codes, position: 0 });
            }
        }
        None
    }
}

impl WasmIntermediateCodeStream<'_> {
    #[inline]
    fn fetch(&mut self) -> &WasmImc {
        let code = unsafe { self.codes.get_unchecked(self.position as usize) };
        self.position += 1;
        code
    }

    #[inline]
    fn set_position(&mut self, val: u32) -> Result<(), WasmRuntimeErrorKind> {
        if (val as usize) < self.codes.len() {
            self.position = val;
            Ok(())
        } else {
            Err(WasmRuntimeErrorKind::InternalInconsistency)
        }
    }
}

impl WasmInvocation for WasmRunnable<'_> {
    fn invoke(&self, params: &[WasmValue]) -> Result<Option<WasmValue>, Box<dyn Error>> {
        let function = self.function();

        let code_block = match function.content() {
            WasmFunctionContent::CodeBlock(v) => Ok(v),
            WasmFunctionContent::Dynamic(_) => Err(WasmRuntimeErrorKind::InvalidParameter),
            WasmFunctionContent::Unresolved => Err(WasmRuntimeErrorKind::NoMethod),
        }?;

        let mut locals =
            Vec::with_capacity(function.param_types().len() + code_block.local_types().len());

        for (index, param_type) in function.param_types().iter().enumerate() {
            let param = params
                .get(index)
                .ok_or(WasmRuntimeErrorKind::InvalidParameter)?;
            if !param.is_valid_type(*param_type) {
                return Err(WasmRuntimeErrorKind::InvalidParameter.into());
            }
            locals.push(WasmUnionValue::from(*param));
        }

        locals.extend(iter::repeat(WasmUnionValue::zero()).take(code_block.local_types().len()));

        let result_types = function.result_types();

        let mut interp = WasmInterpreter::new(self.instance());
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
    mnemonic: WasmMnemonic,
}

impl WasmRuntimeError {
    #[inline]
    pub fn try_from_error(e: Box<dyn Error>) -> Option<Box<Self>> {
        e.downcast::<Self>().ok()
    }

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
    pub const fn mnemonic(&self) -> WasmMnemonic {
        self.mnemonic
    }
}

// impl From<WasmRuntimeErrorKind> for WasmRuntimeError {
//     #[inline]
//     fn from(kind: WasmRuntimeErrorKind) -> Self {
//         Self {
//             kind,
//             file_position: 0,
//             function: 0,
//             function_name: None,
//             position: 0,
//             mnemonic: WasmMnemonic::Unreachable,
//         }
//     }
// }

impl From<WasmRuntimeErrorKind> for Box<dyn Error> {
    #[inline]
    fn from(value: WasmRuntimeErrorKind) -> Self {
        Box::new(WasmRuntimeError {
            kind: value,
            file_position: 0,
            function: 0,
            function_name: None,
            position: 0,
            mnemonic: WasmMnemonic::Unreachable,
        })
    }
}

impl fmt::Display for WasmRuntimeError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mnemonic = self.mnemonic();
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

        write!(f, ", 0x{:x}: {:?}", self.file_position(), mnemonic)
    }
}

impl fmt::Debug for WasmRuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self as &dyn fmt::Display).fmt(f)
    }
}

impl core::error::Error for WasmRuntimeError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        Some(self)
    }
}

#[repr(transparent)]
pub struct StackFrame<'a>(&'a mut [WasmUnionValue]);

impl<'a> StackFrame<'a> {
    #[inline]
    pub fn new(slice: &'a mut [WasmUnionValue]) -> Self {
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
        #[cfg(test)]
        let _ = self.0.split_at(index.as_usize());

        let (l, r) = unsafe { self.0.split_at_mut_unchecked(index.as_usize()) };
        (StackFrame(l), StackFrame(r))
    }

    /// # Safety
    ///
    /// Since stack-level verification is guaranteed by the code verifier
    #[inline]
    pub fn get(&self, index: StackLevel) -> &WasmUnionValue {
        #[cfg(test)]
        let _ = self.0[index.as_usize()];

        unsafe { self.0.get_unchecked(index.as_usize()) }
    }

    /// # Safety
    ///
    /// Since stack-level verification is guaranteed by the code verifier
    #[inline]
    pub fn get_mut(&mut self, index: StackLevel) -> &mut WasmUnionValue {
        #[cfg(test)]
        let _ = self.0[index.as_usize()];

        unsafe { self.0.get_unchecked_mut(index.as_usize()) }
    }

    #[inline]
    pub fn set(&mut self, index: StackLevel, value: WasmUnionValue) {
        *(self.get_mut(index)) = value;
    }

    /// # Safety
    ///
    /// Since stack-level verification is guaranteed by the code verifier
    pub unsafe fn get_range(&mut self, offset: StackLevel, size: usize) -> &[WasmUnionValue] {
        let offset = offset.as_usize();

        #[cfg(test)]
        let _ = self.0[offset..offset + size];

        unsafe { self.0.get_unchecked(offset..offset + size) }
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
        #[cfg(test)]
        let _ = self.0[index.as_usize()];

        unsafe { self.0.get_unchecked(index.as_usize()) }
    }

    /// # Safety
    ///
    /// Because the range is guaranteed by the code verifier
    #[inline]
    pub fn get_local_mut(&mut self, index: LocalVarIndex) -> &mut WasmUnionValue {
        #[cfg(test)]
        let _ = self.0[index.as_usize()];

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
