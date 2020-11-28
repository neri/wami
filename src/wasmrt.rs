// WebAssembly Runtime (pre-alpha)

use super::opcode::*;
use super::wasm::*;
use crate::*;
use alloc::vec::Vec;
// use super::*;
// use alloc::sync::Arc;
// use core::cell::RefCell;

#[allow(dead_code)]
pub struct WasmRuntimeContext<'a> {
    value_stack: Vec<WasmValue>,
    call_stack: Vec<WasmCodeBlock<'a>>,
}

impl<'a> WasmRuntimeContext<'a> {
    pub fn new() -> Self {
        Self {
            value_stack: Vec::new(),
            call_stack: Vec::new(),
        }
    }

    pub fn run(
        &mut self,
        code_block: &mut WasmCodeBlock,
        locals: &mut [WasmValue],
        result_types: &[WasmValType],
    ) -> Result<WasmValue, WasmRuntimeError> {
        code_block.reset();
        loop {
            // let position = code_block.position();
            let opcode = code_block.get_opcode()?;

            // println!("{:04x} {:02x} {}", position, opcode as u8, opcode.to_str());

            match opcode {
                WasmOpcode::Nop => (),

                WasmOpcode::End => {
                    break;
                }

                WasmOpcode::Drop => {
                    let _ = self.value_stack.pop().ok_or(WasmRuntimeError::OutOfStack)?;
                }
                WasmOpcode::Select => {
                    let cc = self.pop().and_then(|v| v.get_i32())?;
                    let b = self.pop()?;
                    let a = self.pop()?;
                    let c = if cc != 0 { a } else { b };
                    self.push(c)?;
                }

                WasmOpcode::LocalGet => {
                    let local_ref = code_block.get_uint()? as usize;
                    let val = locals
                        .get(local_ref)
                        .ok_or(WasmRuntimeError::InvalidLocal)?;
                    self.value_stack.push(*val);
                }
                WasmOpcode::LocalSet => {
                    let local_ref = code_block.get_uint()? as usize;
                    let var = locals
                        .get_mut(local_ref)
                        .ok_or(WasmRuntimeError::InvalidLocal)?;
                    let val = self.value_stack.pop().ok_or(WasmRuntimeError::OutOfStack)?;
                    *var = val;
                }
                WasmOpcode::LocalTee => {
                    let local_ref = code_block.get_uint()? as usize;
                    let var = locals
                        .get_mut(local_ref)
                        .ok_or(WasmRuntimeError::InvalidLocal)?;
                    let val = self
                        .value_stack
                        .last()
                        .ok_or(WasmRuntimeError::OutOfStack)?;
                    *var = *val;
                }

                WasmOpcode::I32Const => {
                    let val = code_block.get_sint()? as i32;
                    self.value_stack.push(val.into())
                }
                WasmOpcode::I64Const => {
                    let val = code_block.get_sint()?;
                    self.value_stack.push(val.into())
                }

                WasmOpcode::I32Eqz => {
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push((a == 0).into())?;
                }
                WasmOpcode::I32Eq => {
                    let b = self.pop().and_then(|v| v.get_i32())?;
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push((a == b).into())?;
                }
                WasmOpcode::I32Ne => {
                    let b = self.pop().and_then(|v| v.get_i32())?;
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push((a != b).into())?;
                }
                WasmOpcode::I32LtS => {
                    let b = self.pop().and_then(|v| v.get_i32())?;
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push((a < b).into())?;
                }
                WasmOpcode::I32LtU => {
                    let b = self.pop().and_then(|v| v.get_u32())?;
                    let a = self.pop().and_then(|v| v.get_u32())?;
                    self.push((a < b).into())?;
                }
                WasmOpcode::I32LeS => {
                    let b = self.pop().and_then(|v| v.get_i32())?;
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push((a <= b).into())?;
                }
                WasmOpcode::I32LeU => {
                    let b = self.pop().and_then(|v| v.get_u32())?;
                    let a = self.pop().and_then(|v| v.get_u32())?;
                    self.push((a <= b).into())?;
                }
                WasmOpcode::I32GtS => {
                    let b = self.pop().and_then(|v| v.get_i32())?;
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push((a > b).into())?;
                }
                WasmOpcode::I32GtU => {
                    let b = self.pop().and_then(|v| v.get_u32())?;
                    let a = self.pop().and_then(|v| v.get_u32())?;
                    self.push((a > b).into())?;
                }
                WasmOpcode::I32GeS => {
                    let b = self.pop().and_then(|v| v.get_i32())?;
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push((a >= b).into())?;
                }
                WasmOpcode::I32GeU => {
                    let b = self.pop().and_then(|v| v.get_u32())?;
                    let a = self.pop().and_then(|v| v.get_u32())?;
                    self.push((a >= b).into())?;
                }

                WasmOpcode::I64Eqz => {
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push((a == 0).into())?;
                }
                WasmOpcode::I64Eq => {
                    let b = self.pop().and_then(|v| v.get_i64())?;
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push((a == b).into())?;
                }
                WasmOpcode::I64Ne => {
                    let b = self.pop().and_then(|v| v.get_i64())?;
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push((a != b).into())?;
                }
                WasmOpcode::I64LtS => {
                    let b = self.pop().and_then(|v| v.get_i64())?;
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push((a < b).into())?;
                }
                WasmOpcode::I64LtU => {
                    let b = self.pop().and_then(|v| v.get_u64())?;
                    let a = self.pop().and_then(|v| v.get_u64())?;
                    self.push((a < b).into())?;
                }
                WasmOpcode::I64LeS => {
                    let b = self.pop().and_then(|v| v.get_i64())?;
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push((a <= b).into())?;
                }
                WasmOpcode::I64LeU => {
                    let b = self.pop().and_then(|v| v.get_u64())?;
                    let a = self.pop().and_then(|v| v.get_u64())?;
                    self.push((a <= b).into())?;
                }
                WasmOpcode::I64GtS => {
                    let b = self.pop().and_then(|v| v.get_i64())?;
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push((a > b).into())?;
                }
                WasmOpcode::I64GtU => {
                    let b = self.pop().and_then(|v| v.get_u64())?;
                    let a = self.pop().and_then(|v| v.get_u64())?;
                    self.push((a > b).into())?;
                }
                WasmOpcode::I64GeS => {
                    let b = self.pop().and_then(|v| v.get_i64())?;
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push((a >= b).into())?;
                }
                WasmOpcode::I64GeU => {
                    let b = self.pop().and_then(|v| v.get_u64())?;
                    let a = self.pop().and_then(|v| v.get_u64())?;
                    self.push((a >= b).into())?;
                }

                WasmOpcode::I32Clz => {
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push(a.leading_zeros().into())?;
                }
                WasmOpcode::I32Ctz => {
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push(a.trailing_zeros().into())?;
                }
                WasmOpcode::I32Popcnt => {
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push(a.count_ones().into())?;
                }
                WasmOpcode::I32Add => {
                    let b = self.pop().and_then(|v| v.get_i32())?;
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push(a.wrapping_add(b).into())?;
                }
                WasmOpcode::I32Sub => {
                    let b = self.pop().and_then(|v| v.get_i32())?;
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push(a.wrapping_sub(b).into())?;
                }
                WasmOpcode::I32Mul => {
                    let b = self.pop().and_then(|v| v.get_i32())?;
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push(a.wrapping_mul(b).into())?;
                }
                WasmOpcode::I32DivS => {
                    let b = self.pop().and_then(|v| v.get_i32())?;
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    if b == 0 {
                        return Err(WasmRuntimeError::DivideByZero);
                    }
                    self.push(a.wrapping_div(b).into())?;
                }
                WasmOpcode::I32DivU => {
                    let b = self.pop().and_then(|v| v.get_u32())?;
                    let a = self.pop().and_then(|v| v.get_u32())?;
                    if b == 0 {
                        return Err(WasmRuntimeError::DivideByZero);
                    }
                    self.push((a.wrapping_div(b)).into())?;
                }
                WasmOpcode::I32RemS => {
                    let b = self.pop().and_then(|v| v.get_i32())?;
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    if b == 0 {
                        return Err(WasmRuntimeError::DivideByZero);
                    }
                    self.push(a.wrapping_rem(b).into())?;
                }
                WasmOpcode::I32RemU => {
                    let b = self.pop().and_then(|v| v.get_u32())?;
                    let a = self.pop().and_then(|v| v.get_u32())?;
                    if b == 0 {
                        return Err(WasmRuntimeError::DivideByZero);
                    }
                    self.push((a.wrapping_rem(b)).into())?;
                }
                WasmOpcode::I32And => {
                    let b = self.pop().and_then(|v| v.get_i32())?;
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push((a & b).into())?;
                }
                WasmOpcode::I32Or => {
                    let b = self.pop().and_then(|v| v.get_i32())?;
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push((a | b).into())?;
                }
                WasmOpcode::I32Xor => {
                    let b = self.pop().and_then(|v| v.get_i32())?;
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push((a ^ b).into())?;
                }

                WasmOpcode::I32Shl => {
                    let b = self.pop().and_then(|v| v.get_i32())?;
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push((a << b).into())?;
                }
                WasmOpcode::I32ShrS => {
                    let b = self.pop().and_then(|v| v.get_i32())?;
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    self.push((a >> b).into())?;
                }
                WasmOpcode::I32ShrU => {
                    let b = self.pop().and_then(|v| v.get_u32())?;
                    let a = self.pop().and_then(|v| v.get_u32())?;
                    self.push((a >> b).into())?;
                }
                WasmOpcode::I32Rotl => {
                    let b = self.pop().and_then(|v| v.get_u32())?;
                    let a = self.pop().and_then(|v| v.get_u32())?;
                    self.push(a.rotate_left(b).into())?;
                }
                WasmOpcode::I32Rotr => {
                    let b = self.pop().and_then(|v| v.get_u32())?;
                    let a = self.pop().and_then(|v| v.get_u32())?;
                    self.push(a.rotate_right(b).into())?;
                }

                WasmOpcode::I64Clz => {
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push(a.leading_zeros().into())?;
                }
                WasmOpcode::I64Ctz => {
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push(a.trailing_zeros().into())?;
                }
                WasmOpcode::I64Popcnt => {
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push(a.count_ones().into())?;
                }
                WasmOpcode::I64Add => {
                    let b = self.pop().and_then(|v| v.get_i64())?;
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push(a.wrapping_add(b).into())?;
                }
                WasmOpcode::I64Sub => {
                    let b = self.pop().and_then(|v| v.get_i64())?;
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push(a.wrapping_sub(b).into())?;
                }
                WasmOpcode::I64Mul => {
                    let b = self.pop().and_then(|v| v.get_i64())?;
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push(a.wrapping_mul(b).into())?;
                }
                WasmOpcode::I64DivS => {
                    let b = self.pop().and_then(|v| v.get_i64())?;
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    if b == 0 {
                        return Err(WasmRuntimeError::DivideByZero);
                    }
                    self.push(a.wrapping_div(b).into())?;
                }
                WasmOpcode::I64DivU => {
                    let b = self.pop().and_then(|v| v.get_u64())?;
                    let a = self.pop().and_then(|v| v.get_u64())?;
                    if b == 0 {
                        return Err(WasmRuntimeError::DivideByZero);
                    }
                    self.push((a.wrapping_div(b)).into())?;
                }
                WasmOpcode::I64RemS => {
                    let b = self.pop().and_then(|v| v.get_i64())?;
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    if b == 0 {
                        return Err(WasmRuntimeError::DivideByZero);
                    }
                    self.push(a.wrapping_rem(b).into())?;
                }
                WasmOpcode::I64RemU => {
                    let b = self.pop().and_then(|v| v.get_u64())?;
                    let a = self.pop().and_then(|v| v.get_u64())?;
                    if b == 0 {
                        return Err(WasmRuntimeError::DivideByZero);
                    }
                    self.push((a.wrapping_rem(b)).into())?;
                }
                WasmOpcode::I64And => {
                    let b = self.pop().and_then(|v| v.get_i64())?;
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push((a & b).into())?;
                }
                WasmOpcode::I64Or => {
                    let b = self.pop().and_then(|v| v.get_i64())?;
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push((a | b).into())?;
                }
                WasmOpcode::I64Xor => {
                    let b = self.pop().and_then(|v| v.get_i64())?;
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push((a ^ b).into())?;
                }

                WasmOpcode::I64Shl => {
                    let b = self.pop().and_then(|v| v.get_i64())?;
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push((a << b).into())?;
                }
                WasmOpcode::I64ShrS => {
                    let b = self.pop().and_then(|v| v.get_i64())?;
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    self.push((a >> b).into())?;
                }
                WasmOpcode::I64ShrU => {
                    let b = self.pop().and_then(|v| v.get_u64())?;
                    let a = self.pop().and_then(|v| v.get_u64())?;
                    self.push((a >> b).into())?;
                }
                WasmOpcode::I64Rotl => {
                    let b = self.pop().and_then(|v| v.get_u64())? as u32;
                    let a = self.pop().and_then(|v| v.get_u64())?;
                    self.push(a.rotate_left(b).into())?;
                }
                WasmOpcode::I64Rotr => {
                    let b = self.pop().and_then(|v| v.get_u64())? as u32;
                    let a = self.pop().and_then(|v| v.get_u64())?;
                    self.push(a.rotate_right(b).into())?;
                }

                WasmOpcode::I32WrapI64 => {
                    let a = self.pop().and_then(|v| v.get_i64())?;
                    let c = a as i32;
                    self.push(c.into())?;
                }
                WasmOpcode::I64ExtendI32S => {
                    let a = self.pop().and_then(|v| v.get_i32())?;
                    let c = a as i64;
                    self.push(c.into())?;
                }
                WasmOpcode::I64ExtendI32U => {
                    let a = self.pop().and_then(|v| v.get_u32())?;
                    let c = a as u64;
                    self.push(c.into())?;
                }
                WasmOpcode::I32Extend8S => {
                    let a = self.pop().and_then(|v| v.get_i32())? as i8;
                    let c = a as i32;
                    self.push(c.into())?;
                }
                WasmOpcode::I32Extend16S => {
                    let a = self.pop().and_then(|v| v.get_i32())? as i16;
                    let c = a as i32;
                    self.push(c.into())?;
                }
                WasmOpcode::I64Extend8S => {
                    let a = self.pop().and_then(|v| v.get_i64())? as i8;
                    let c = a as i64;
                    self.push(c.into())?;
                }
                WasmOpcode::I64Extend16S => {
                    let a = self.pop().and_then(|v| v.get_i64())? as i16;
                    let c = a as i64;
                    self.push(c.into())?;
                }
                WasmOpcode::I64Extend32S => {
                    let a = self.pop().and_then(|v| v.get_i64())? as i32;
                    let c = a as i64;
                    self.push(c.into())?;
                }

                _ => return Err(WasmRuntimeError::InvalidBytecode),
            }
        }
        if result_types.len() > 0 {
            let val = self.pop()?;
            Ok(val)
        } else {
            if self.value_stack.first().is_none() {
                Ok(WasmValue::Empty)
            } else {
                Err(WasmRuntimeError::InvalidStackLevel)
            }
        }
    }

    #[inline]
    pub fn push(&mut self, value: WasmValue) -> Result<(), WasmRuntimeError> {
        Ok(self.value_stack.push(value))
    }

    #[inline]
    pub fn pop(&mut self) -> Result<WasmValue, WasmRuntimeError> {
        self.value_stack.pop().ok_or(WasmRuntimeError::OutOfStack)
    }
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
enum BlockInstType {
    Block,
    Loop,
    If,
    Else,
    End,
}

#[allow(dead_code)]
struct BlockContext {
    inst_type: BlockInstType,
    expr_type: WasmBlockType,
    stack_level: usize,
    start_position: usize,
    end_position: usize,
}

#[cfg(test)]
mod tests {

    #[test]
    fn add() {
        let slice = [0x20, 0, 0x20, 1, 0x6A, 0x0B];
        let mut code_block = super::WasmCodeBlock::from_slice(&slice);

        let mut params = [1234.into(), 5678.into()];
        let result = code_block
            .invoke(&mut params, &[crate::wasm::WasmValType::I32])
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 6912);

        let mut params = [0xdeadbeefu32.into(), 0x55555555.into()];
        let result = code_block
            .invoke(&mut params, &[crate::wasm::WasmValType::I32])
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 0x34031444);

        let mut params = [1234.into(), 5678.into()];
        match code_block.invoke(&mut params, &[]) {
            Err(super::WasmRuntimeError::InvalidStackLevel) => (),
            Ok(v) => panic!("expected: Err, actual: Ok({})", v),
            Err(err) => panic!("unexpected: {:?}", err),
        }

        let mut params = [0.into(), 0u64.into()];
        match code_block.invoke(&mut params, &[crate::wasm::WasmValType::I32]) {
            Err(super::WasmRuntimeError::TypeMismatch) => (),
            Ok(v) => panic!("expected: Err, actual: Ok({})", v),
            Err(err) => panic!("unexpected: {:?}", err),
        }
    }
}
