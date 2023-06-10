//! WebAssembly Intermediate Code Interpreter

use super::{
    intcode::{ExceptionPosition, WasmImc, WasmIntMnemonic},
    LocalVarIndex, StackLevel, StackOffset, WasmCodeBlock,
};
use crate::{
    opcode::WasmOpcode,
    opcode::{WasmOpcodeFC, WasmSingleOpcode},
    stack::*,
    wasm::*,
};
use alloc::{borrow::ToOwned, string::String, vec::Vec};
use core::fmt;

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
        &self,
        kind: WasmRuntimeErrorKind,
        opcode: WasmOpcode,
        ex_position: ExceptionPosition,
    ) -> WasmRuntimeError {
        let function_name = self
            .module
            .names()
            .and_then(|v| v.func_by_index(self.func_index))
            .map(|v| v.to_owned());
        let file_position = self
            .module
            .codeblock(self.func_index)
            .map(|v| v.file_position())
            .unwrap_or(0)
            + ex_position.position();
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
        locals: &mut [WasmUnsafeValue],
        result_types: &[WasmValType],
    ) -> Result<Option<WasmValue>, WasmRuntimeError> {
        let mut heap = StackHeap::with_capacity(0x10000);
        self._interpret(
            func_index,
            code_block,
            LocalVariables::new(locals),
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
        self.func_index = func_index;
        let mut codes = WasmIntermediateCodeStream::from_codes(code_block.intermediate_codes())
            .ok_or(WasmRuntimeError::from(
                WasmRuntimeErrorKind::InternalInconsistency,
            ))?;

        let mut value_stack = ValueStack::new(heap.alloc_slice(code_block.max_value_stack()));
        value_stack.clear();

        let mut result_stack_level = StackLevel::zero();

        let memory = self
            .module
            .memory(0)
            .ok_or(WasmRuntimeError::from(WasmRuntimeErrorKind::OutOfBounds))?;

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
                    let cc = unsafe {
                        value_stack
                            .get_unchecked(code.base_stack_level())
                            .get_bool()
                    };
                    if cc {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::BrTable(ref table) => {
                    let table_len = table.len() - 1;
                    let index = usize::min(table_len, unsafe {
                        value_stack.get_unchecked(code.base_stack_level()).get_u32() as usize
                    });
                    let target = unsafe { *table.get_unchecked(index) };
                    codes.set_position(target)?;
                }

                WasmIntMnemonic::ReturnV => {
                    break;
                }

                WasmIntMnemonic::ReturnI => {
                    result_stack_level = code.base_stack_level();
                    break;
                }
                WasmIntMnemonic::ReturnF => {
                    todo!();
                }

                WasmIntMnemonic::Call(func_index, ex_position) => {
                    let func = unsafe { self.module.functions().get_unchecked(func_index) };
                    self.call(
                        WasmSingleOpcode::Call.into(),
                        ex_position,
                        code.base_stack_level(),
                        func,
                        &mut value_stack,
                        heap,
                    )?;
                }
                WasmIntMnemonic::CallIndirect(type_index, ex_position) => {
                    let opcode = WasmOpcode::Single(WasmSingleOpcode::CallIndirect);
                    let index = unsafe {
                        value_stack.get_unchecked(code.base_stack_level()).get_i32() as usize
                    };
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
                    self.call(
                        opcode,
                        ex_position,
                        code.base_stack_level(),
                        func,
                        &mut value_stack,
                        heap,
                    )?;
                }

                WasmIntMnemonic::SelectI => {
                    let stack_level = code.base_stack_level();
                    let cc = unsafe { value_stack.get_unchecked(stack_level + 2).get_bool() };
                    if !cc {
                        unsafe {
                            let b = *value_stack.get_unchecked(stack_level + 1);
                            let ref_a = value_stack.get_unchecked_mut(stack_level);
                            *ref_a = b;
                        }
                    }
                }

                WasmIntMnemonic::SelectF => todo!(),

                WasmIntMnemonic::LocalGetI(local_index) => {
                    let local = unsafe { locals.get_unchecked(local_index) };
                    let ref_a = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *ref_a = *local;
                }
                WasmIntMnemonic::LocalSetI(local_index)
                | WasmIntMnemonic::LocalTeeI(local_index) => {
                    let local = unsafe { locals.get_unchecked_mut(local_index) };
                    let ref_a = unsafe { value_stack.get_unchecked(code.base_stack_level()) };
                    *local = *ref_a;
                }

                WasmIntMnemonic::LocalGetF(_local_index) => todo!(),
                WasmIntMnemonic::LocalSetF(_local_index) => todo!(),
                WasmIntMnemonic::LocalTeeF(_local_index) => todo!(),

                WasmIntMnemonic::GlobalGetI(global_ref) => {
                    let global =
                        unsafe { self.module.globals().get_unchecked(global_ref.as_usize()) };
                    let ref_a = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *ref_a = global.value().into();
                }
                WasmIntMnemonic::GlobalSetI(global_ref) => {
                    let global =
                        unsafe { self.module.globals().get_unchecked(global_ref.as_usize()) };
                    let ref_a = unsafe { value_stack.get_unchecked(code.base_stack_level()) };
                    global.set_value(*ref_a);
                }

                WasmIntMnemonic::GlobalGetF(_global_ref) => todo!(),
                WasmIntMnemonic::GlobalSetF(_global_ref) => todo!(),

                WasmIntMnemonic::I32Load(offset, ex_position) => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    let index = unsafe { var.get_u32() };
                    *var = memory
                        .read_u32(offset, index)
                        .map(|v| WasmUnsafeValue::from(v))
                        .map_err(|e| {
                            self.error(e, WasmSingleOpcode::I32Load.into(), ex_position)
                        })?;
                }
                WasmIntMnemonic::I32Load8S(offset, ex_position) => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = memory
                        .read_u8(offset, unsafe { var.get_u32() })
                        .map(|v| WasmUnsafeValue::from(v as i8 as i32))
                        .map_err(|e| {
                            self.error(e, WasmSingleOpcode::I32Load8S.into(), ex_position)
                        })?;
                }
                WasmIntMnemonic::I32Load8U(offset, ex_position) => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = memory
                        .read_u8(offset, unsafe { var.get_u32() })
                        .map(|v| WasmUnsafeValue::from(v as u32))
                        .map_err(|e| {
                            self.error(e, WasmSingleOpcode::I32Load8U.into(), ex_position)
                        })?;
                }
                WasmIntMnemonic::I32Load16S(offset, ex_position) => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = memory
                        .read_u16(offset, unsafe { var.get_u32() })
                        .map(|v| WasmUnsafeValue::from(v as i16 as i32))
                        .map_err(|e| {
                            self.error(e, WasmSingleOpcode::I32Load16S.into(), ex_position)
                        })?;
                }
                WasmIntMnemonic::I32Load16U(offset, ex_position) => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = memory
                        .read_u16(offset, unsafe { var.get_u32() })
                        .map(|v| WasmUnsafeValue::from(v as u32))
                        .map_err(|e| {
                            self.error(e, WasmSingleOpcode::I32Load16U.into(), ex_position)
                        })?;
                }

                WasmIntMnemonic::I64Load(offset, ex_position) => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = memory
                        .read_u64(offset, unsafe { var.get_u32() })
                        .map(|v| WasmUnsafeValue::from(v))
                        .map_err(|e| {
                            self.error(e, WasmSingleOpcode::I64Load.into(), ex_position)
                        })?;
                }
                WasmIntMnemonic::I64Load8S(offset, ex_position) => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = memory
                        .read_u8(offset, unsafe { var.get_u32() })
                        .map(|v| WasmUnsafeValue::from(v as i8 as i64))
                        .map_err(|e| {
                            self.error(e, WasmSingleOpcode::I64Load8S.into(), ex_position)
                        })?;
                }
                WasmIntMnemonic::I64Load8U(offset, ex_position) => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = memory
                        .read_u8(offset, unsafe { var.get_u32() })
                        .map(|v| WasmUnsafeValue::from(v as u64))
                        .map_err(|e| {
                            self.error(e, WasmSingleOpcode::I64Load8U.into(), ex_position)
                        })?;
                }
                WasmIntMnemonic::I64Load16S(offset, ex_position) => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = memory
                        .read_u16(offset, unsafe { var.get_u32() })
                        .map(|v| WasmUnsafeValue::from(v as i16 as i64))
                        .map_err(|e| {
                            self.error(e, WasmSingleOpcode::I64Load16S.into(), ex_position)
                        })?;
                }
                WasmIntMnemonic::I64Load16U(offset, ex_position) => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = memory
                        .read_u16(offset, unsafe { var.get_u32() })
                        .map(|v| WasmUnsafeValue::from(v as u64))
                        .map_err(|e| {
                            self.error(e, WasmSingleOpcode::I64Load16U.into(), ex_position)
                        })?;
                }
                WasmIntMnemonic::I64Load32S(offset, ex_position) => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = memory
                        .read_u32(offset, unsafe { var.get_u32() })
                        .map(|v| WasmUnsafeValue::from(v as i32 as i64))
                        .map_err(|e| {
                            self.error(e, WasmSingleOpcode::I64Load32S.into(), ex_position)
                        })?;
                }
                WasmIntMnemonic::I64Load32U(offset, ex_position) => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = memory
                        .read_u32(offset, unsafe { var.get_u32() })
                        .map(|v| WasmUnsafeValue::from(v as u64))
                        .map_err(|e| {
                            self.error(e, WasmSingleOpcode::I64Load32U.into(), ex_position)
                        })?;
                }

                WasmIntMnemonic::I32Store(offset, ex_position) => {
                    let stack_level = code.base_stack_level();
                    let index = unsafe { value_stack.get_unchecked(stack_level).get_u32() };
                    let data = unsafe { value_stack.get_unchecked(stack_level + 1).get_u32() };
                    memory.write_u32(offset, index, data).map_err(|e| {
                        self.error(e, WasmSingleOpcode::I32Store.into(), ex_position)
                    })?;
                }
                WasmIntMnemonic::I32Store8(offset, ex_position) => {
                    let stack_level = code.base_stack_level();
                    let index = unsafe { value_stack.get_unchecked(stack_level).get_u32() };
                    let data = unsafe { value_stack.get_unchecked(stack_level + 1).get_u8() };
                    memory.write_u8(offset, index, data).map_err(|e| {
                        self.error(e, WasmSingleOpcode::I32Store8.into(), ex_position)
                    })?;
                }
                WasmIntMnemonic::I32Store16(offset, ex_position) => {
                    let stack_level = code.base_stack_level();
                    let index = unsafe { value_stack.get_unchecked(stack_level).get_u32() };
                    let data = unsafe { value_stack.get_unchecked(stack_level + 1).get_u16() };
                    memory.write_u16(offset, index, data).map_err(|e| {
                        self.error(e, WasmSingleOpcode::I32Store16.into(), ex_position)
                    })?;
                }

                WasmIntMnemonic::I64Store8(offset, ex_position) => {
                    let stack_level = code.base_stack_level();
                    let index = unsafe { value_stack.get_unchecked(stack_level).get_u32() };
                    let data = unsafe { value_stack.get_unchecked(stack_level + 1).get_u8() };
                    memory.write_u8(offset, index, data).map_err(|e| {
                        self.error(e, WasmSingleOpcode::I64Store8.into(), ex_position)
                    })?;
                }
                WasmIntMnemonic::I64Store16(offset, ex_position) => {
                    let stack_level = code.base_stack_level();
                    let index = unsafe { value_stack.get_unchecked(stack_level).get_u32() };
                    let data = unsafe { value_stack.get_unchecked(stack_level + 1).get_u16() };
                    memory.write_u16(offset, index, data).map_err(|e| {
                        self.error(e, WasmSingleOpcode::I64Store16.into(), ex_position)
                    })?;
                }
                WasmIntMnemonic::I64Store32(offset, ex_position) => {
                    let stack_level = code.base_stack_level();
                    let index = unsafe { value_stack.get_unchecked(stack_level).get_u32() };
                    let data = unsafe { value_stack.get_unchecked(stack_level + 1).get_u32() };
                    memory.write_u32(offset, index, data).map_err(|e| {
                        self.error(e, WasmSingleOpcode::I64Store32.into(), ex_position)
                    })?;
                }
                WasmIntMnemonic::I64Store(offset, ex_position) => {
                    let stack_level = code.base_stack_level();
                    let index = unsafe { value_stack.get_unchecked(stack_level).get_u32() };
                    let data = unsafe { value_stack.get_unchecked(stack_level + 1).get_u64() };
                    memory.write_u64(offset, index, data).map_err(|e| {
                        self.error(e, WasmSingleOpcode::I64Store.into(), ex_position)
                    })?;
                }

                #[cfg(feature = "float")]
                WasmIntMnemonic::F32Load(offset, ex_position) => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = memory
                        .read_u32(offset, unsafe { var.get_u32() })
                        .map(|v| WasmUnsafeValue::from(v))
                        .map_err(|e| {
                            self.error(e, WasmSingleOpcode::F32Load.into(), ex_position)
                        })?;
                }
                #[cfg(feature = "float")]
                WasmIntMnemonic::F32Store(offset, ex_position) => {
                    let stack_level = code.base_stack_level();
                    let index = unsafe { value_stack.get_unchecked(stack_level).get_u32() };
                    let data = unsafe { value_stack.get_unchecked(stack_level + 1).get_u32() };
                    memory.write_u32(offset, index, data).map_err(|e| {
                        self.error(e, WasmSingleOpcode::F32Store.into(), ex_position)
                    })?;
                }

                #[cfg(feature = "float")]
                WasmIntMnemonic::F64Load(offset, ex_position) => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = memory
                        .read_u64(offset, unsafe { var.get_u32() })
                        .map(|v| WasmUnsafeValue::from(v))
                        .map_err(|e| {
                            self.error(e, WasmSingleOpcode::F64Load.into(), ex_position)
                        })?;
                }
                #[cfg(feature = "float")]
                WasmIntMnemonic::F64Store(offset, ex_position) => {
                    let stack_level = code.base_stack_level();
                    let index = unsafe { value_stack.get_unchecked(stack_level).get_u32() };
                    let data = unsafe { value_stack.get_unchecked(stack_level + 1).get_u64() };
                    memory.write_u64(offset, index, data).map_err(|e| {
                        self.error(e, WasmSingleOpcode::F64Store.into(), ex_position)
                    })?;
                }

                WasmIntMnemonic::MemorySize => {
                    let ref_a = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *ref_a = WasmUnsafeValue::from(memory.size());
                }
                WasmIntMnemonic::MemoryGrow => {
                    let ref_a = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *ref_a = WasmUnsafeValue::from(memory.grow(unsafe { ref_a.get_i32() }));
                }
                WasmIntMnemonic::MemoryCopy(ex_position) => {
                    let stack_level = code.base_stack_level();
                    let dest = unsafe { value_stack.get_unchecked(stack_level).get_u32() };
                    let src = unsafe { value_stack.get_unchecked(stack_level + 1).get_u32() };
                    let count = unsafe { value_stack.get_unchecked(stack_level + 2).get_u32() };
                    memory
                        .memcpy(dest, src, count)
                        .map_err(|k| self.error(k, WasmOpcodeFC::MemoryCopy.into(), ex_position))?;
                }
                WasmIntMnemonic::MemoryFill(ex_position) => {
                    let stack_level = code.base_stack_level();
                    let base = unsafe { value_stack.get_unchecked(stack_level).get_u32() };
                    let val = unsafe { value_stack.get_unchecked(stack_level + 1).get_u32() };
                    let count = unsafe { value_stack.get_unchecked(stack_level + 2).get_u32() };
                    memory
                        .memset(base, val as u8, count)
                        .map_err(|k| self.error(k, WasmOpcodeFC::MemoryFill.into(), ex_position))?;
                }

                WasmIntMnemonic::I32Const(val) => {
                    let ref_a = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        ref_a.write_i32(val);
                    }
                }
                WasmIntMnemonic::I64Const(val) => {
                    let ref_a = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        ref_a.write_i64(val);
                    }
                }
                #[cfg(feature = "float")]
                WasmIntMnemonic::F32Const(val) => {
                    let ref_a = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        ref_a.write_f32(val);
                    }
                }
                #[cfg(feature = "float")]
                WasmIntMnemonic::F64Const(val) => {
                    let ref_a = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        ref_a.write_f64(val);
                    }
                }

                WasmIntMnemonic::I32Eqz => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe { var.write_bool(var.get_i32() == 0) }
                }
                WasmIntMnemonic::I32Eq => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe { lhs.write_bool(lhs.get_u32() == rhs.get_u32()) }
                }
                WasmIntMnemonic::I32Ne => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe { lhs.write_bool(lhs.get_u32() != rhs.get_u32()) }
                }
                WasmIntMnemonic::I32LtS => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe { lhs.write_bool(lhs.get_i32() < rhs.get_i32()) }
                }
                WasmIntMnemonic::I32LtU => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe { lhs.write_bool(lhs.get_u32() < rhs.get_u32()) }
                }
                WasmIntMnemonic::I32GtS => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe { lhs.write_bool(lhs.get_i32() > rhs.get_i32()) }
                }
                WasmIntMnemonic::I32GtU => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe { lhs.write_bool(lhs.get_u32() > rhs.get_u32()) }
                }
                WasmIntMnemonic::I32LeS => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe { lhs.write_bool(lhs.get_i32() <= rhs.get_i32()) }
                }
                WasmIntMnemonic::I32LeU => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe { lhs.write_bool(lhs.get_u32() <= rhs.get_u32()) }
                }
                WasmIntMnemonic::I32GeS => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe { lhs.write_bool(lhs.get_i32() >= rhs.get_i32()) }
                }
                WasmIntMnemonic::I32GeU => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe { lhs.write_bool(lhs.get_u32() >= rhs.get_u32()) }
                }

                WasmIntMnemonic::I32Clz => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        var.map_u32(|v| v.leading_zeros());
                    }
                }
                WasmIntMnemonic::I32Ctz => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        var.map_u32(|v| v.trailing_zeros());
                    }
                }
                WasmIntMnemonic::I32Popcnt => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        var.map_u32(|v| v.count_ones());
                    }
                }

                WasmIntMnemonic::I32Add => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_add(rhs.get_i32()));
                    }
                }
                WasmIntMnemonic::I32Sub => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_sub(rhs.get_i32()));
                    }
                }
                WasmIntMnemonic::I32Mul => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_mul(rhs.get_i32()));
                    }
                }

                WasmIntMnemonic::I32DivS(position) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { value_stack.get_unchecked(stack_level + 1).get_i32() };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    if rhs == 0 {
                        return Err(self.error(
                            WasmRuntimeErrorKind::DivideByZero,
                            WasmSingleOpcode::I32DivS.into(),
                            position,
                        ));
                    }
                    unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_div(rhs));
                    }
                }
                WasmIntMnemonic::I32DivU(position) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { value_stack.get_unchecked(stack_level + 1).get_u32() };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    if rhs == 0 {
                        return Err(self.error(
                            WasmRuntimeErrorKind::DivideByZero,
                            WasmSingleOpcode::I32DivU.into(),
                            position,
                        ));
                    }
                    unsafe {
                        lhs.map_u32(|lhs| lhs.wrapping_div(rhs));
                    }
                }
                WasmIntMnemonic::I32RemS(position) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { value_stack.get_unchecked(stack_level + 1).get_i32() };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    if rhs == 0 {
                        return Err(self.error(
                            WasmRuntimeErrorKind::DivideByZero,
                            WasmSingleOpcode::I32RemS.into(),
                            position,
                        ));
                    }
                    unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_rem(rhs));
                    }
                }
                WasmIntMnemonic::I32RemU(position) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { value_stack.get_unchecked(stack_level + 1).get_u32() };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    if rhs == 0 {
                        return Err(self.error(
                            WasmRuntimeErrorKind::DivideByZero,
                            WasmSingleOpcode::I32RemU.into(),
                            position,
                        ));
                    }
                    unsafe {
                        lhs.map_u32(|lhs| lhs.wrapping_rem(rhs));
                    }
                }

                WasmIntMnemonic::I32And => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_u32(|lhs| lhs & rhs.get_u32());
                    }
                }
                WasmIntMnemonic::I32Or => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_u32(|lhs| lhs | rhs.get_u32());
                    }
                }
                WasmIntMnemonic::I32Xor => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_u32(|lhs| lhs ^ rhs.get_u32());
                    }
                }
                WasmIntMnemonic::I32Shl => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_u32(|lhs| lhs.wrapping_shl(rhs.get_u32()));
                    }
                }
                WasmIntMnemonic::I32ShrS => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_shr(rhs.get_u32()));
                    }
                }
                WasmIntMnemonic::I32ShrU => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_u32(|lhs| lhs.wrapping_shr(rhs.get_u32()));
                    }
                }
                WasmIntMnemonic::I32Rotl => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_u32(|lhs| lhs.rotate_left(rhs.get_u32()));
                    }
                }
                WasmIntMnemonic::I32Rotr => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_u32(|lhs| lhs.rotate_right(rhs.get_u32()));
                    }
                }

                WasmIntMnemonic::I64Eqz => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = WasmUnsafeValue::from_bool(unsafe { var.get_i64() == 0 });
                }
                WasmIntMnemonic::I64Eq => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    *lhs = WasmUnsafeValue::from(unsafe { lhs.get_u64() == rhs.get_u64() });
                }
                WasmIntMnemonic::I64Ne => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    *lhs = WasmUnsafeValue::from(unsafe { lhs.get_u64() != rhs.get_u64() });
                }
                WasmIntMnemonic::I64LtS => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    *lhs = WasmUnsafeValue::from(unsafe { lhs.get_i64() < rhs.get_i64() });
                }
                WasmIntMnemonic::I64LtU => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    *lhs = WasmUnsafeValue::from(unsafe { lhs.get_u64() < rhs.get_u64() });
                }
                WasmIntMnemonic::I64GtS => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    *lhs = WasmUnsafeValue::from(unsafe { lhs.get_i64() > rhs.get_i64() });
                }
                WasmIntMnemonic::I64GtU => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    *lhs = WasmUnsafeValue::from(unsafe { lhs.get_u64() > rhs.get_u64() });
                }
                WasmIntMnemonic::I64LeS => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    *lhs = WasmUnsafeValue::from(unsafe { lhs.get_i64() <= rhs.get_i64() });
                }
                WasmIntMnemonic::I64LeU => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    *lhs = WasmUnsafeValue::from(unsafe { lhs.get_u64() <= rhs.get_u64() });
                }
                WasmIntMnemonic::I64GeS => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    *lhs = WasmUnsafeValue::from(unsafe { lhs.get_i64() >= rhs.get_i64() });
                }
                WasmIntMnemonic::I64GeU => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    *lhs = WasmUnsafeValue::from(unsafe { lhs.get_u64() >= rhs.get_u64() });
                }

                WasmIntMnemonic::I64Clz => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        var.map_u64(|v| v.leading_zeros() as u64);
                    }
                }
                WasmIntMnemonic::I64Ctz => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        var.map_u64(|v| v.trailing_zeros() as u64);
                    }
                }
                WasmIntMnemonic::I64Popcnt => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        var.map_u64(|v| v.count_ones() as u64);
                    }
                }
                WasmIntMnemonic::I64Add => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_add(rhs.get_i64()));
                    }
                }
                WasmIntMnemonic::I64Sub => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_sub(rhs.get_i64()));
                    }
                }
                WasmIntMnemonic::I64Mul => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_mul(rhs.get_i64()));
                    }
                }

                WasmIntMnemonic::I64DivS(position) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { value_stack.get_unchecked(stack_level + 1).get_i64() };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    if rhs == 0 {
                        return Err(self.error(
                            WasmRuntimeErrorKind::DivideByZero,
                            WasmSingleOpcode::I64DivS.into(),
                            position,
                        ));
                    }
                    unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_div(rhs));
                    }
                }
                WasmIntMnemonic::I64DivU(position) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { value_stack.get_unchecked(stack_level + 1).get_u64() };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    if rhs == 0 {
                        return Err(self.error(
                            WasmRuntimeErrorKind::DivideByZero,
                            WasmSingleOpcode::I64DivU.into(),
                            position,
                        ));
                    }
                    unsafe {
                        lhs.map_u64(|lhs| lhs.wrapping_div(rhs));
                    }
                }
                WasmIntMnemonic::I64RemS(position) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { value_stack.get_unchecked(stack_level + 1).get_i64() };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    if rhs == 0 {
                        return Err(self.error(
                            WasmRuntimeErrorKind::DivideByZero,
                            WasmSingleOpcode::I64RemS.into(),
                            position,
                        ));
                    }
                    unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_rem(rhs));
                    }
                }
                WasmIntMnemonic::I64RemU(position) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { value_stack.get_unchecked(stack_level + 1).get_u64() };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    if rhs == 0 {
                        return Err(self.error(
                            WasmRuntimeErrorKind::DivideByZero,
                            WasmSingleOpcode::I64RemU.into(),
                            position,
                        ));
                    }
                    unsafe {
                        lhs.map_u64(|lhs| lhs.wrapping_rem(rhs));
                    }
                }

                WasmIntMnemonic::I64And => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_u64(|lhs| lhs & rhs.get_u64());
                    }
                }
                WasmIntMnemonic::I64Or => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_u64(|lhs| lhs | rhs.get_u64());
                    }
                }
                WasmIntMnemonic::I64Xor => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_u64(|lhs| lhs ^ rhs.get_u64());
                    }
                }
                WasmIntMnemonic::I64Shl => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_u64(|lhs| lhs.wrapping_shl(rhs.get_u64() as u32));
                    }
                }
                WasmIntMnemonic::I64ShrS => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_shr(rhs.get_u64() as u32));
                    }
                }
                WasmIntMnemonic::I64ShrU => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_u64(|lhs| lhs.wrapping_shr(rhs.get_u64() as u32));
                    }
                }
                WasmIntMnemonic::I64Rotl => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_u64(|lhs| lhs.rotate_left(rhs.get_u32()));
                    }
                }
                WasmIntMnemonic::I64Rotr => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { value_stack.get_unchecked_mut(stack_level) };
                    unsafe {
                        lhs.map_u64(|lhs| lhs.rotate_right(rhs.get_u32()));
                    }
                }

                WasmIntMnemonic::I64Extend8S => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = WasmUnsafeValue::from_i64(unsafe { var.get_i8() as i64 });
                }
                WasmIntMnemonic::I64Extend16S => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = WasmUnsafeValue::from_i64(unsafe { var.get_i16() as i64 });
                }
                WasmIntMnemonic::I64Extend32S | WasmIntMnemonic::I64ExtendI32S => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = WasmUnsafeValue::from_i64(unsafe { var.get_i32() as i64 });
                }
                WasmIntMnemonic::I64ExtendI32U => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = WasmUnsafeValue::from_u64(unsafe { var.get_u32() as u64 });
                }
                WasmIntMnemonic::I32WrapI64 => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = WasmUnsafeValue::from_i32(unsafe { var.get_i64() as i32 });
                }
                WasmIntMnemonic::I32Extend8S => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = WasmUnsafeValue::from_i32(unsafe { var.get_i8() as i32 });
                }
                WasmIntMnemonic::I32Extend16S => {
                    let var = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    *var = WasmUnsafeValue::from_i32(unsafe { var.get_i16() as i32 });
                }

                WasmIntMnemonic::FusedI32SetConst(local_index, val) => {
                    let local = unsafe { locals.get_unchecked_mut(local_index) };
                    unsafe {
                        local.write_i32(val);
                    }
                }
                WasmIntMnemonic::FusedI32AddI(val) => {
                    let lhs = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_add(val));
                    }
                }
                WasmIntMnemonic::FusedI32SubI(val) => {
                    let lhs = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        lhs.map_i32(|lhs| lhs.wrapping_sub(val));
                    }
                }
                WasmIntMnemonic::FusedI32AndI(val) => {
                    let lhs = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        lhs.map_i32(|lhs| lhs & val);
                    }
                }
                WasmIntMnemonic::FusedI32OrI(val) => {
                    let lhs = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        lhs.map_i32(|lhs| lhs | val);
                    }
                }
                WasmIntMnemonic::FusedI32XorI(val) => {
                    let lhs = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        lhs.map_i32(|lhs| lhs ^ val);
                    }
                }
                WasmIntMnemonic::FusedI32ShlI(val) => {
                    let lhs = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        lhs.map_u32(|lhs| lhs << (val));
                    }
                }
                WasmIntMnemonic::FusedI32ShrUI(val) => {
                    let lhs = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        lhs.map_u32(|lhs| lhs >> (val as u32));
                    }
                }
                WasmIntMnemonic::FusedI32ShrSI(val) => {
                    let lhs = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        lhs.map_i32(|lhs| lhs >> val);
                    }
                }

                WasmIntMnemonic::FusedI64SetConst(local_index, val) => {
                    let local = unsafe { locals.get_unchecked_mut(local_index) };
                    *local = val.into();
                }
                WasmIntMnemonic::FusedI64AddI(val) => {
                    let lhs = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_add(val));
                    }
                }
                WasmIntMnemonic::FusedI64SubI(val) => {
                    let lhs = unsafe { value_stack.get_unchecked_mut(code.base_stack_level()) };
                    unsafe {
                        lhs.map_i64(|lhs| lhs.wrapping_sub(val));
                    }
                }

                WasmIntMnemonic::FusedI32BrZ(target) => {
                    let cc = unsafe {
                        value_stack
                            .get_unchecked_mut(code.base_stack_level())
                            .get_i32()
                            == 0
                    };
                    if cc {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrEq(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { *value_stack.get_unchecked(stack_level) };
                    if unsafe { lhs.get_u32() == rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrNe(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { *value_stack.get_unchecked(stack_level) };
                    if unsafe { lhs.get_u32() != rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrLtS(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { *value_stack.get_unchecked(stack_level) };
                    if unsafe { lhs.get_i32() < rhs.get_i32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrLtU(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { *value_stack.get_unchecked(stack_level) };
                    if unsafe { lhs.get_u32() < rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrGtS(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { *value_stack.get_unchecked(stack_level) };
                    if unsafe { lhs.get_i32() > rhs.get_i32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrGtU(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { *value_stack.get_unchecked(stack_level) };
                    if unsafe { lhs.get_u32() > rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrLeS(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { *value_stack.get_unchecked(stack_level) };
                    if unsafe { lhs.get_i32() <= rhs.get_i32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrLeU(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { *value_stack.get_unchecked(stack_level) };
                    if unsafe { lhs.get_u32() <= rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrGeS(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { *value_stack.get_unchecked(stack_level) };
                    if unsafe { lhs.get_i32() >= rhs.get_i32() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI32BrGeU(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { *value_stack.get_unchecked(stack_level) };
                    if unsafe { lhs.get_u32() >= rhs.get_u32() } {
                        codes.set_position(target)?;
                    }
                }

                WasmIntMnemonic::FusedI64BrZ(target) => {
                    let cc = unsafe {
                        value_stack
                            .get_unchecked_mut(code.base_stack_level())
                            .get_i64()
                            == 0
                    };
                    if cc {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI64BrEq(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { *value_stack.get_unchecked(stack_level) };
                    if unsafe { lhs.get_u64() == rhs.get_u64() } {
                        codes.set_position(target)?;
                    }
                }
                WasmIntMnemonic::FusedI64BrNe(target) => {
                    let stack_level = code.base_stack_level();
                    let rhs = unsafe { *value_stack.get_unchecked(stack_level + 1) };
                    let lhs = unsafe { *value_stack.get_unchecked(stack_level) };
                    if unsafe { lhs.get_u64() != rhs.get_u64() } {
                        codes.set_position(target)?;
                    }
                }
            }
        }
        if let Some(result_type) = result_types.first() {
            let val = unsafe { value_stack.get_unchecked(result_stack_level) };
            Ok(Some(unsafe { val.get_by_type(*result_type) }))
        } else {
            Ok(None)
        }
    }

    #[inline]
    fn call(
        &mut self,
        opcode: WasmOpcode,
        ex_position: ExceptionPosition,
        stack_pointer: StackLevel,
        target: &WasmFunction,
        value_stack: &mut ValueStack,
        heap: &mut StackHeap,
    ) -> Result<(), WasmRuntimeError> {
        let current_function = self.func_index;
        let module = self.module;
        let result_types = target.result_types();

        let param_len = target.param_types().len();
        // if stack_pointer < param_len {
        //     return Err(self.error(WasmRuntimeError::InternalInconsistency, code));
        // }
        let stack_under = stack_pointer - StackOffset::new(param_len);

        if let Some(code_block) = target.code_block() {
            heap.snapshot(|heap| {
                let local_len = param_len + code_block.local_types().len();

                let mut locals = if value_stack.len()
                    >= (stack_under + StackOffset::new(local_len)).as_usize()
                {
                    let (_, locals) = unsafe { value_stack.split_at_mut_unchecked(stack_under) };
                    locals
                } else {
                    let mut locals = ValueStack::new(
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
                    *local = WasmUnsafeValue::zero();
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
                        let var = unsafe { value_stack.get_unchecked_mut(stack_under) };
                        *var = WasmUnsafeValue::from(result);
                    }
                    self.func_index = current_function;
                    Ok(())
                })
            })
        } else if let Some(function) = target.dlink() {
            let locals = unsafe { value_stack.get_range(stack_under, param_len) };
            let result = match function(module, locals) {
                Ok(v) => v,
                Err(e) => return Err(self.error(e, opcode, ex_position)),
            };

            if let Some(t) = result_types.first() {
                if result.is_valid_type(*t) {
                    let var = unsafe { value_stack.get_unchecked_mut(stack_under) };
                    *var = WasmUnsafeValue::from(result);
                } else {
                    return Err(self.error(
                        WasmRuntimeErrorKind::TypeMismatch,
                        opcode,
                        ex_position,
                    ));
                }
            }
            Ok(())
        } else {
            Err(self.error(WasmRuntimeErrorKind::NoMethod, opcode, ex_position))
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
        if let Some(last) = codes.last()
            && matches!(last.mnemonic(), WasmIntMnemonic::Unreachable(_))
        {
            Some(Self { codes, position: 0 })
        } else {
            None
        }
    }
}

impl WasmIntermediateCodeStream<'_> {
    #[inline]
    fn fetch(&mut self) -> &WasmImc {
        let code = unsafe { self.codes.get_unchecked(self.position) };
        self.position += 1;
        code
    }

    #[allow(dead_code)]
    #[inline]
    const fn position(&self) -> usize {
        self.position
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
        let code_block = function
            .code_block()
            .ok_or(WasmRuntimeError::from(WasmRuntimeErrorKind::NoMethod))?;

        let local_len = usize::max(
            INITIAL_VALUE_STACK_SIZE,
            function.param_types().len() + code_block.local_types().len(),
        );
        let mut locals = Vec::with_capacity(local_len);
        locals.resize(local_len, WasmUnsafeValue::zero());

        for (index, param_type) in function.param_types().iter().enumerate() {
            let param = params.get(index).ok_or(WasmRuntimeError::from(
                WasmRuntimeErrorKind::InvalidParameter,
            ))?;
            if !param.is_valid_type(*param_type) {
                return Err(WasmRuntimeErrorKind::InvalidParameter.into());
            }
            unsafe {
                *locals.get_unchecked_mut(index) = WasmUnsafeValue::from(param.clone());
            }
        }

        let result_types = function.result_types();

        let mut interp = WasmInterpreter::new(self.module());
        interp.invoke(
            function.index(),
            code_block,
            locals.as_mut_slice(),
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
    pub const fn kind(&self) -> WasmRuntimeErrorKind {
        self.kind
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
pub struct ValueStack<'a>(&'a mut [WasmUnsafeValue]);

impl<'a> ValueStack<'a> {
    #[inline]
    pub fn new(slice: &'a mut [WasmUnsafeValue]) -> Self {
        Self(slice)
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.0.fill(WasmUnsafeValue::zero());
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &WasmUnsafeValue> {
        self.0.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut WasmUnsafeValue> {
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
    pub unsafe fn split_at_mut_unchecked(&mut self, index: StackLevel) -> (ValueStack, ValueStack) {
        let (l, r) = unsafe { self.0.split_at_mut_unchecked(index.as_usize()) };
        (ValueStack::new(l), ValueStack::new(r))
    }

    /// # Safety
    ///
    /// Since stack-level verification is guaranteed by the code verifier
    #[inline]
    pub unsafe fn get_unchecked(&self, index: StackLevel) -> &WasmUnsafeValue {
        unsafe { self.0.get_unchecked(index.as_usize()) }
    }

    /// # Safety
    ///
    /// Since stack-level verification is guaranteed by the code verifier
    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, index: StackLevel) -> &mut WasmUnsafeValue {
        unsafe { self.0.get_unchecked_mut(index.as_usize()) }
    }

    /// # Safety
    ///
    /// Since stack-level verification is guaranteed by the code verifier
    pub unsafe fn get_range(&mut self, offset: StackLevel, size: usize) -> &[WasmUnsafeValue] {
        let offset = offset.as_usize();
        unsafe { self.0.get_unchecked_mut(offset..offset + size) }
    }
}

#[repr(transparent)]
pub struct LocalVariables<'a>(&'a mut [WasmUnsafeValue]);

impl<'a> LocalVariables<'a> {
    #[inline]
    pub fn new(slice: &'a mut [WasmUnsafeValue]) -> Self {
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
    pub unsafe fn get_unchecked(&self, index: LocalVarIndex) -> &WasmUnsafeValue {
        unsafe { self.0.get_unchecked(index.as_usize()) }
    }

    /// # Safety
    ///
    /// Because the range is guaranteed by the code verifier
    #[inline]
    pub unsafe fn get_unchecked_mut(&mut self, index: LocalVarIndex) -> &mut WasmUnsafeValue {
        unsafe { self.0.get_unchecked_mut(index.as_usize()) }
    }
}
