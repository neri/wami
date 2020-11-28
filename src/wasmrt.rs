// WebAssembly Runtime (pre-alpha)

use super::opcode::*;
use super::wasm::*;
use crate::*;
use alloc::vec::Vec;
// use core::cell::RefCell;

#[allow(dead_code)]
pub struct WasmRuntimeContext<'a> {
    call_stack: Vec<WasmCodeBlock<'a>>,
}

impl<'a> WasmRuntimeContext<'a> {
    pub fn new() -> Self {
        Self {
            call_stack: Vec::new(),
        }
    }

    pub fn run(
        &mut self,
        code_block: &mut WasmCodeBlock,
        locals: &mut [WasmValue],
        result_types: &[WasmValType],
    ) -> Result<WasmValue, WasmRuntimeError> {
        let mut locals = {
            let mut output = Vec::new();
            for local in locals {
                output.push(WasmStackValue::from(*local));
            }
            output
        };
        let mut value_stack: Vec<WasmStackValue> = Vec::new();
        code_block.reset();
        loop {
            // let position = code_block.position();
            let opcode = code_block.read_opcode()?;

            // println!("{:04x} {:02x} {}", position, opcode as u8, opcode.to_str());

            match opcode {
                WasmOpcode::Nop => (),

                WasmOpcode::End => {
                    break;
                }

                WasmOpcode::Drop => {
                    let _ = value_stack.pop();
                }
                WasmOpcode::Select => {
                    let cc = value_stack.pop().unwrap();
                    let b = value_stack.pop().unwrap();
                    let a = value_stack.pop().unwrap();
                    let c = if cc.get_i32() != 0 { a } else { b };
                    value_stack.push(c);
                }

                WasmOpcode::LocalGet => {
                    let local_ref = code_block.read_uint()? as usize;
                    let val = *locals.get(local_ref).unwrap();
                    value_stack.push(val.into());
                }
                WasmOpcode::LocalSet => {
                    let local_ref = code_block.read_uint()? as usize;
                    let var = locals.get_mut(local_ref).unwrap();
                    let val = value_stack.pop().unwrap();
                    *var = val;
                }
                WasmOpcode::LocalTee => {
                    let local_ref = code_block.read_uint()? as usize;
                    let var = locals.get_mut(local_ref).unwrap();
                    let val = value_stack.last().unwrap();
                    *var = *val;
                }

                WasmOpcode::I32Const => {
                    let val = code_block.read_sint()? as i32;
                    value_stack.push(WasmStackValue { i32: val });
                }
                WasmOpcode::I64Const => {
                    let val = code_block.read_sint()?;
                    value_stack.push(WasmStackValue { i64: val });
                }

                WasmOpcode::I32Eqz => {
                    let last = value_stack.last_mut().unwrap();
                    *last = WasmStackValue::bool(last.get_i32() == 0);
                }
                WasmOpcode::I32Eq => {
                    let b = value_stack.pop().unwrap().get_i32();
                    let a = value_stack.pop().unwrap().get_i32();
                    value_stack.push(WasmStackValue::bool(a == b));
                }
                WasmOpcode::I32Ne => {
                    let b = value_stack.pop().unwrap().get_i32();
                    let a = value_stack.pop().unwrap().get_i32();
                    value_stack.push(WasmStackValue::bool(a != b));
                }
                WasmOpcode::I32LtS => {
                    let b = value_stack.pop().unwrap().get_i32();
                    let a = value_stack.pop().unwrap().get_i32();
                    value_stack.push(WasmStackValue::bool(a < b));
                }
                WasmOpcode::I32LtU => {
                    let b = value_stack.pop().unwrap().get_u32();
                    let a = value_stack.pop().unwrap().get_u32();
                    value_stack.push(WasmStackValue::bool(a < b));
                }
                WasmOpcode::I32LeS => {
                    let b = value_stack.pop().unwrap().get_i32();
                    let a = value_stack.pop().unwrap().get_i32();
                    value_stack.push(WasmStackValue::bool(a <= b));
                }
                WasmOpcode::I32LeU => {
                    let b = value_stack.pop().unwrap().get_u32();
                    let a = value_stack.pop().unwrap().get_u32();
                    value_stack.push(WasmStackValue::bool(a <= b));
                }
                WasmOpcode::I32GtS => {
                    let b = value_stack.pop().unwrap().get_i32();
                    let a = value_stack.pop().unwrap().get_i32();
                    value_stack.push(WasmStackValue::bool(a > b));
                }
                WasmOpcode::I32GtU => {
                    let b = value_stack.pop().unwrap().get_u32();
                    let a = value_stack.pop().unwrap().get_u32();
                    value_stack.push(WasmStackValue::bool(a > b));
                }
                WasmOpcode::I32GeS => {
                    let b = value_stack.pop().unwrap().get_i32();
                    let a = value_stack.pop().unwrap().get_i32();
                    value_stack.push(WasmStackValue::bool(a >= b));
                }
                WasmOpcode::I32GeU => {
                    let b = value_stack.pop().unwrap().get_u32();
                    let a = value_stack.pop().unwrap().get_u32();
                    value_stack.push(WasmStackValue::bool(a >= b));
                }

                WasmOpcode::I64Eqz => {
                    let last = value_stack.last_mut().unwrap();
                    *last = WasmStackValue::bool(last.get_i64() == 0);
                }
                WasmOpcode::I64Eq => {
                    let b = value_stack.pop().unwrap().get_i64();
                    let a = value_stack.pop().unwrap().get_i64();
                    value_stack.push(WasmStackValue::bool(a == b));
                }
                WasmOpcode::I64Ne => {
                    let b = value_stack.pop().unwrap().get_i64();
                    let a = value_stack.pop().unwrap().get_i64();
                    value_stack.push(WasmStackValue::bool(a != b));
                }
                WasmOpcode::I64LtS => {
                    let b = value_stack.pop().unwrap().get_i64();
                    let a = value_stack.pop().unwrap().get_i64();
                    value_stack.push(WasmStackValue::bool(a < b));
                }
                WasmOpcode::I64LtU => {
                    let b = value_stack.pop().unwrap().get_u64();
                    let a = value_stack.pop().unwrap().get_u64();
                    value_stack.push(WasmStackValue::bool(a < b));
                }
                WasmOpcode::I64LeS => {
                    let b = value_stack.pop().unwrap().get_i64();
                    let a = value_stack.pop().unwrap().get_i64();
                    value_stack.push(WasmStackValue::bool(a <= b));
                }
                WasmOpcode::I64LeU => {
                    let b = value_stack.pop().unwrap().get_u64();
                    let a = value_stack.pop().unwrap().get_u64();
                    value_stack.push(WasmStackValue::bool(a <= b));
                }
                WasmOpcode::I64GtS => {
                    let b = value_stack.pop().unwrap().get_i64();
                    let a = value_stack.pop().unwrap().get_i64();
                    value_stack.push(WasmStackValue::bool(a > b));
                }
                WasmOpcode::I64GtU => {
                    let b = value_stack.pop().unwrap().get_u64();
                    let a = value_stack.pop().unwrap().get_u64();
                    value_stack.push(WasmStackValue::bool(a > b));
                }
                WasmOpcode::I64GeS => {
                    let b = value_stack.pop().unwrap().get_i64();
                    let a = value_stack.pop().unwrap().get_i64();
                    value_stack.push(WasmStackValue::bool(a >= b));
                }
                WasmOpcode::I64GeU => {
                    let b = value_stack.pop().unwrap().get_u64();
                    let a = value_stack.pop().unwrap().get_u64();
                    value_stack.push(WasmStackValue::bool(a >= b));
                }

                WasmOpcode::I32Clz => {
                    let last = value_stack.last_mut().unwrap();
                    *last = WasmStackValue::u32(last.get_i32().leading_zeros());
                }
                WasmOpcode::I32Ctz => {
                    let last = value_stack.last_mut().unwrap();
                    *last = WasmStackValue::u32(last.get_i32().trailing_zeros());
                }
                WasmOpcode::I32Popcnt => {
                    let last = value_stack.last_mut().unwrap();
                    *last = WasmStackValue::u32(last.get_i32().count_ones());
                }

                WasmOpcode::I32Add => {
                    let b = value_stack.pop().unwrap().get_u32();
                    let a = value_stack.pop().unwrap().get_u32();
                    value_stack.push(WasmStackValue::u32(a.wrapping_add(b)));
                }
                WasmOpcode::I32Sub => {
                    let b = value_stack.pop().unwrap().get_u32();
                    let a = value_stack.pop().unwrap().get_u32();
                    value_stack.push(WasmStackValue::u32(a.wrapping_sub(b)));
                }
                WasmOpcode::I32Mul => {
                    let b = value_stack.pop().unwrap().get_u32();
                    let a = value_stack.pop().unwrap().get_u32();
                    value_stack.push(WasmStackValue::u32(a.wrapping_mul(b)));
                }
                WasmOpcode::I32DivS => {
                    let b = value_stack.pop().unwrap().get_i32();
                    let a = value_stack.pop().unwrap().get_i32();
                    if b == 0 {
                        return Err(WasmRuntimeError::DivideByZero);
                    }
                    value_stack.push(WasmStackValue::i32(a.wrapping_div(b)));
                }
                WasmOpcode::I32DivU => {
                    let b = value_stack.pop().unwrap().get_u32();
                    let a = value_stack.pop().unwrap().get_u32();
                    if b == 0 {
                        return Err(WasmRuntimeError::DivideByZero);
                    }
                    value_stack.push(WasmStackValue::u32(a.wrapping_div(b)));
                }
                WasmOpcode::I32RemS => {
                    let b = value_stack.pop().unwrap().get_i32();
                    let a = value_stack.pop().unwrap().get_i32();
                    if b == 0 {
                        return Err(WasmRuntimeError::DivideByZero);
                    }
                    value_stack.push(WasmStackValue::i32(a.wrapping_rem(b)));
                }
                WasmOpcode::I32RemU => {
                    let b = value_stack.pop().unwrap().get_u32();
                    let a = value_stack.pop().unwrap().get_u32();
                    if b == 0 {
                        return Err(WasmRuntimeError::DivideByZero);
                    }
                    value_stack.push(WasmStackValue::u32(a.wrapping_rem(b)));
                }

                WasmOpcode::I32And => {
                    let b = value_stack.pop().unwrap().get_u32();
                    let a = value_stack.pop().unwrap().get_u32();
                    value_stack.push(WasmStackValue::u32(a & b));
                }
                WasmOpcode::I32Or => {
                    let b = value_stack.pop().unwrap().get_u32();
                    let a = value_stack.pop().unwrap().get_u32();
                    value_stack.push(WasmStackValue::u32(a | b));
                }
                WasmOpcode::I32Xor => {
                    let b = value_stack.pop().unwrap().get_u32();
                    let a = value_stack.pop().unwrap().get_u32();
                    value_stack.push(WasmStackValue::u32(a ^ b));
                }

                WasmOpcode::I32Shl => {
                    let b = value_stack.pop().unwrap().get_u32();
                    let a = value_stack.pop().unwrap().get_u32();
                    value_stack.push(WasmStackValue::u32(a << b));
                }
                WasmOpcode::I32ShrS => {
                    let b = value_stack.pop().unwrap().get_i32();
                    let a = value_stack.pop().unwrap().get_i32();
                    value_stack.push(WasmStackValue::i32(a >> b));
                }
                WasmOpcode::I32ShrU => {
                    let b = value_stack.pop().unwrap().get_u32();
                    let a = value_stack.pop().unwrap().get_u32();
                    value_stack.push(WasmStackValue::u32(a >> b));
                }
                WasmOpcode::I32Rotl => {
                    let b = value_stack.pop().unwrap().get_u32();
                    let a = value_stack.pop().unwrap().get_u32();
                    value_stack.push(WasmStackValue::u32(a.rotate_left(b)));
                }
                WasmOpcode::I32Rotr => {
                    let b = value_stack.pop().unwrap().get_u32();
                    let a = value_stack.pop().unwrap().get_u32();
                    value_stack.push(WasmStackValue::u32(a.rotate_right(b)));
                }

                WasmOpcode::I64Clz => {
                    let last = value_stack.last_mut().unwrap();
                    *last = WasmStackValue::i64(last.get_i64().leading_zeros() as i64);
                }
                WasmOpcode::I64Ctz => {
                    let last = value_stack.last_mut().unwrap();
                    *last = WasmStackValue::i64(last.get_i64().trailing_zeros() as i64);
                }
                WasmOpcode::I64Popcnt => {
                    let last = value_stack.last_mut().unwrap();
                    *last = WasmStackValue::i64(last.get_i64().count_ones() as i64);
                }

                WasmOpcode::I64Add => {
                    let b = value_stack.pop().unwrap().get_u64();
                    let a = value_stack.pop().unwrap().get_u64();
                    value_stack.push(WasmStackValue::u64(a.wrapping_add(b)));
                }
                WasmOpcode::I64Sub => {
                    let b = value_stack.pop().unwrap().get_u64();
                    let a = value_stack.pop().unwrap().get_u64();
                    value_stack.push(WasmStackValue::u64(a.wrapping_sub(b)));
                }
                WasmOpcode::I64Mul => {
                    let b = value_stack.pop().unwrap().get_u64();
                    let a = value_stack.pop().unwrap().get_u64();
                    value_stack.push(WasmStackValue::u64(a.wrapping_mul(b)));
                }
                WasmOpcode::I64DivS => {
                    let b = value_stack.pop().unwrap().get_i64();
                    let a = value_stack.pop().unwrap().get_i64();
                    if b == 0 {
                        return Err(WasmRuntimeError::DivideByZero);
                    }
                    value_stack.push(WasmStackValue::i64(a.wrapping_div(b)));
                }
                WasmOpcode::I64DivU => {
                    let b = value_stack.pop().unwrap().get_u64();
                    let a = value_stack.pop().unwrap().get_u64();
                    if b == 0 {
                        return Err(WasmRuntimeError::DivideByZero);
                    }
                    value_stack.push(WasmStackValue::u64(a.wrapping_div(b)));
                }
                WasmOpcode::I64RemS => {
                    let b = value_stack.pop().unwrap().get_i64();
                    let a = value_stack.pop().unwrap().get_i64();
                    if b == 0 {
                        return Err(WasmRuntimeError::DivideByZero);
                    }
                    value_stack.push(WasmStackValue::i64(a.wrapping_rem(b)));
                }
                WasmOpcode::I64RemU => {
                    let b = value_stack.pop().unwrap().get_u64();
                    let a = value_stack.pop().unwrap().get_u64();
                    if b == 0 {
                        return Err(WasmRuntimeError::DivideByZero);
                    }
                    value_stack.push(WasmStackValue::u64(a.wrapping_rem(b)));
                }

                WasmOpcode::I64And => {
                    let b = value_stack.pop().unwrap().get_u64();
                    let a = value_stack.pop().unwrap().get_u64();
                    value_stack.push(WasmStackValue::u64(a & b));
                }
                WasmOpcode::I64Or => {
                    let b = value_stack.pop().unwrap().get_u64();
                    let a = value_stack.pop().unwrap().get_u64();
                    value_stack.push(WasmStackValue::u64(a | b));
                }
                WasmOpcode::I64Xor => {
                    let b = value_stack.pop().unwrap().get_u64();
                    let a = value_stack.pop().unwrap().get_u64();
                    value_stack.push(WasmStackValue::u64(a ^ b));
                }

                WasmOpcode::I64Shl => {
                    let b = value_stack.pop().unwrap().get_u64();
                    let a = value_stack.pop().unwrap().get_u64();
                    value_stack.push(WasmStackValue::u64(a << b));
                }
                WasmOpcode::I64ShrS => {
                    let b = value_stack.pop().unwrap().get_i64();
                    let a = value_stack.pop().unwrap().get_i64();
                    value_stack.push(WasmStackValue::i64(a >> b));
                }
                WasmOpcode::I64ShrU => {
                    let b = value_stack.pop().unwrap().get_u64();
                    let a = value_stack.pop().unwrap().get_u64();
                    value_stack.push(WasmStackValue::u64(a >> b));
                }
                WasmOpcode::I64Rotl => {
                    let b = value_stack.pop().unwrap().get_u64();
                    let a = value_stack.pop().unwrap().get_u64();
                    value_stack.push(WasmStackValue::u64(a.rotate_left(b as u32)));
                }
                WasmOpcode::I64Rotr => {
                    let b = value_stack.pop().unwrap().get_u64();
                    let a = value_stack.pop().unwrap().get_u64();
                    value_stack.push(WasmStackValue::u64(a.rotate_right(b as u32)));
                }

                WasmOpcode::I32WrapI64 => {
                    // NOP
                }
                WasmOpcode::I64ExtendI32S => {
                    let last = value_stack.last_mut().unwrap();
                    *last = WasmStackValue::i64(last.get_i32() as i64);
                }
                WasmOpcode::I64ExtendI32U => {
                    let last = value_stack.last_mut().unwrap();
                    *last = WasmStackValue::u64(last.get_u32() as u64);
                }

                WasmOpcode::I32Extend8S => {
                    let last = value_stack.last_mut().unwrap();
                    *last = WasmStackValue::i32((last.get_i32() as i8) as i32);
                }
                WasmOpcode::I32Extend16S => {
                    let last = value_stack.last_mut().unwrap();
                    *last = WasmStackValue::i32((last.get_i32() as i16) as i32);
                }

                WasmOpcode::I64Extend8S => {
                    let last = value_stack.last_mut().unwrap();
                    *last = WasmStackValue::i64((last.get_i64() as i8) as i64);
                }
                WasmOpcode::I64Extend16S => {
                    let last = value_stack.last_mut().unwrap();
                    *last = WasmStackValue::i64((last.get_i64() as i16) as i64);
                }
                WasmOpcode::I64Extend32S => {
                    let last = value_stack.last_mut().unwrap();
                    *last = WasmStackValue::i64((last.get_i64() as i32) as i64);
                }

                _ => return Err(WasmRuntimeError::InvalidBytecode),
            }
        }
        if let Some(result_type) = result_types.first() {
            let val = value_stack.pop().unwrap();
            match result_type {
                WasmValType::I32 => Ok(WasmValue::I32(val.get_i32())),
                WasmValType::I64 => Ok(WasmValue::I64(val.get_i64())),
                // WasmValType::F32 => {}
                // WasmValType::F64 => {}
                _ => Err(WasmRuntimeError::InvalidParameter),
            }
        } else {
            Ok(WasmValue::Empty)
        }
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
union WasmStackValue {
    i32: i32,
    u32: u32,
    i64: i64,
    u64: u64,
    f32: f32,
    f64: f64,
}

impl WasmStackValue {
    pub const fn bool(v: bool) -> Self {
        if v {
            Self::i64(1)
        } else {
            Self::i64(0)
        }
    }

    pub fn get_i32(&self) -> i32 {
        unsafe { self.i32 }
    }

    pub fn get_u32(&self) -> u32 {
        unsafe { self.u32 }
    }

    pub fn get_i64(&self) -> i64 {
        unsafe { self.i64 }
    }

    pub fn get_u64(&self) -> u64 {
        unsafe { self.u64 }
    }

    pub const fn i32(v: i32) -> Self {
        Self { i64: v as i64 }
    }

    pub const fn u32(v: u32) -> Self {
        Self { u64: v as u64 }
    }

    pub const fn i64(v: i64) -> Self {
        Self { i64: v }
    }

    pub const fn u64(v: u64) -> Self {
        Self { u64: v }
    }
}

impl From<WasmValue> for WasmStackValue {
    fn from(v: WasmValue) -> Self {
        match v {
            WasmValue::Empty => Self::i64(0),
            WasmValue::I32(v) => Self::i64(v as i64),
            WasmValue::I64(v) => Self::i64(v),
            _ => todo!(),
        }
    }
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

        let mut params = [0xDEADBEEFu32.into(), 0x55555555.into()];
        let result = code_block
            .invoke(&mut params, &[crate::wasm::WasmValType::I32])
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 0x34031444);
    }

    #[test]
    fn sub() {
        let slice = [0x20, 0, 0x20, 1, 0x6B, 0x0B];
        let mut code_block = super::WasmCodeBlock::from_slice(&slice);

        let mut params = [1234.into(), 5678.into()];
        let result = code_block
            .invoke(&mut params, &[crate::wasm::WasmValType::I32])
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, -4444);

        let mut params = [0x55555555.into(), 0xDEADBEEFu32.into()];
        let result = code_block
            .invoke(&mut params, &[crate::wasm::WasmValType::I32])
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 0x76a79666);
    }
}
