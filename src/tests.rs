// test

use crate::{
    cg::{
        intr::{WasmInterpreter, WasmInvocation},
        WasmCodeBlock,
    },
    opcode::WasmSingleOpcode,
    WasmValType, *,
};
use alloc::borrow::ToOwned;

#[cfg(feature = "float")]
use core::f64::consts::PI;

#[test]
fn instantiate_minimal() {
    let data = [0, 97, 115, 109, 1, 0, 0, 0];
    WasmLoader::instantiate(&data, |_, _, _| unreachable!()).unwrap();
}

#[test]
#[should_panic(expected = "BadExecutable")]
fn instantiate_bad_exec1() {
    let data = [0, 97, 115, 109, 1, 0, 0];
    WasmLoader::instantiate(&data, |_, _, _| unreachable!()).unwrap();
}

#[test]
#[should_panic(expected = "BadExecutable")]
fn instantiate_bad_exec2() {
    let data = [0, 97, 115, 109, 2, 0, 0, 0];
    WasmLoader::instantiate(&data, |_, _, _| unreachable!()).unwrap();
}

#[test]
#[should_panic(expected = "UnexpectedEof")]
fn instantiate_unexpected_eof() {
    let data = [0, 97, 115, 109, 1, 0, 0, 0, 1];
    WasmLoader::instantiate(&data, |_, _, _| unreachable!()).unwrap();
}

#[test]
fn instantiate_fibonacci() {
    let slice = [
        0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x01, 0x06, 0x01, 0x60, 0x01, 0x7F, 0x01,
        0x7F, 0x03, 0x02, 0x01, 0x00, 0x0A, 0x31, 0x01, 0x2F, 0x01, 0x01, 0x7F, 0x41, 0x00, 0x21,
        0x01, 0x02, 0x40, 0x03, 0x40, 0x20, 0x00, 0x41, 0x02, 0x49, 0x0D, 0x01, 0x20, 0x00, 0x41,
        0x7F, 0x6A, 0x10, 0x00, 0x20, 0x01, 0x6A, 0x21, 0x01, 0x20, 0x00, 0x41, 0x7E, 0x6A, 0x21,
        0x00, 0x0C, 0x00, 0x0B, 0x0B, 0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B,
    ];
    let module = WasmLoader::instantiate(&slice, |_, _, _| unreachable!()).unwrap();
    let _ = module.func_by_index(0).unwrap();
}

#[test]
fn leb128() {
    let data = [
        0x7F, 0xFF, 0x00, 0xEF, 0xFD, 0xB6, 0xF5, 0x0D, 0xEF, 0xFD, 0xB6, 0xF5, 0x7D,
    ];
    let mut stream = Leb128Stream::from_slice(&data);

    stream.reset();
    assert_eq!(stream.position(), 0);
    let test = stream.read_unsigned().unwrap();
    assert_eq!(test, 127);
    let test = stream.read_unsigned().unwrap();
    assert_eq!(test, 127);
    let test = stream.read_unsigned().unwrap();
    assert_eq!(test, 0xdeadbeef);
    let test = stream.read_unsigned().unwrap();
    assert_eq!(test, 0x7deadbeef);

    stream.reset();
    assert_eq!(stream.position(), 0);
    let test = stream.read_signed().unwrap();
    assert_eq!(test, -1);
    let test = stream.read_signed().unwrap();
    assert_eq!(test, 127);
    let test = stream.read_signed().unwrap();
    assert_eq!(test, 0xdeadbeef);
    let test = stream.read_signed().unwrap();
    assert_eq!(test, -559038737);
}

#[test]
fn i32_const() {
    let slice = [0, 0x41, 0xf8, 0xac, 0xd1, 0x91, 0x01, 0x0B];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info = WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0x12345678);
}

#[test]
#[should_panic(expected = "TypeMismatch")]
fn i32_const_type_mismatch1() {
    let slice = [0, 0x41, 0x00, 0x01, 0x0B];
    let result_types = [WasmValType::I64];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let _info = WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, &module).unwrap();
}

#[test]
fn i32_const_type_mismatch2() {
    let slice = [0, 0x41, 0x00, 0x01, 0x0B];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info = WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let result2 = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_i64();
    assert_eq!(result2, Err(WasmRuntimeErrorKind::TypeMismatch));
}

#[test]
fn i64_const() {
    let slice = [
        0, 0x42, 0xef, 0x9b, 0xaf, 0xcd, 0xf8, 0xac, 0xd1, 0x91, 0x01, 0x0B,
    ];
    let result_types = [WasmValType::I64];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info = WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_i64()
        .unwrap();
    assert_eq!(result, 0x123456789abcdef);
}

#[test]
#[should_panic(expected = "TypeMismatch")]
fn i64_const_type_mismatch1() {
    let slice = [0, 0x42, 0x00, 0x01, 0x0B];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let _info = WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, &module).unwrap();
}

#[test]
#[cfg(feature = "float")]
fn float_const() {
    let slice = [0, 0x43, 0, 0, 0, 0, 0x0B];
    let result_types = [WasmValType::F32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info = WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_f32()
        .unwrap();
    assert_eq!(result, 0.0);

    let slice = [0, 0x43, 0, 0, 0xc0, 0x7f, 0x0B];
    let result_types = [WasmValType::F32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info = WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_f32()
        .unwrap();
    assert!(result.is_nan());

    let slice = [0, 0x43, 0, 0, 0x80, 0x7f, 0x0B];
    let result_types = [WasmValType::F32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info = WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_f32()
        .unwrap();
    assert!(result.is_infinite());

    let slice = [0, 0x43, 0xdb, 0x0f, 0x49, 0x40, 0x0B];
    let result_types = [WasmValType::F32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info = WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_f32()
        .unwrap();
    assert_eq!(result, 3.1415927);
}

#[test]
#[cfg(feature = "float")]
fn float64_const() {
    let slice = [0, 0x44, 0, 0, 0, 0, 0, 0, 0, 0, 0x0B];
    let result_types = [WasmValType::F64];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info = WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_f64()
        .unwrap();
    assert_eq!(result, 0.0);

    let slice = [0, 0x44, 0, 0, 0, 0, 0, 0, 0xf8, 0x7f, 0x0B];
    let result_types = [WasmValType::F64];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info = WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_f64()
        .unwrap();
    assert!(result.is_nan());

    let slice = [0, 0x44, 0, 0, 0, 0, 0, 0, 0xf0, 0x7f, 0x0B];
    let result_types = [WasmValType::F64];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info = WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_f64()
        .unwrap();
    assert!(result.is_infinite());

    let slice = [
        0, 0x44, 0x18, 0x2d, 0x44, 0x54, 0xfb, 0x21, 0x09, 0x40, 0x0b,
    ];
    let result_types = [WasmValType::F64];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info = WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_f64()
        .unwrap();
    assert_eq!(result, PI);
}

#[test]
fn add() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x6A, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [1234.into(), 5678.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 6912);

    let mut locals = [0xDEADBEEFu32.into(), 0x55555555.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0x34031444);
}

#[test]
fn fused_add() {
    let slice = [0, 0x20, 0, 0x41, 1, 0x6A, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [1234_5678.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 12345679);

    let mut locals = [0xFFFF_FFFFu32.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);
}

#[test]
fn const_local() {
    let slice = [
        1, 1, 0x7F, 0x41, 0xFB, 0x00, 0x21, 0, 0x41, 0x12, 0x1A, 0x20, 0, 0x0B,
    ];
    let param_types = [];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 123);
}

#[test]
fn sub() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x6B, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [1234.into(), 5678.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, -4444);

    let mut locals = [0x55555555.into(), 0xDEADBEEFu32.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0x76a79666);
}

#[test]
fn mul() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x6C, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [1234.into(), 5678.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 7006652);

    let mut locals = [0x55555555.into(), 0xDEADBEEFu32.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0x6070c05b);
}

#[test]
fn div32_s() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x6D, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [7006652.into(), 5678.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1234);

    let mut locals = [42.into(), (-6).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, -7);

    let mut locals = [(-42).into(), (6).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, -7);

    let mut locals = [(-42).into(), (-6).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 7);

    let mut locals = [1234.into(), 0.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap_err();
    assert_eq!(WasmRuntimeErrorKind::DivideByZero, result.kind());
    assert_eq!(result.opcode(), WasmSingleOpcode::I32DivS.into());
    assert_eq!(result.position(), 5);
}

#[test]
fn div32_u() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x6E, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [7006652.into(), 5678.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1234);

    let mut locals = [42.into(), (-6).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let mut locals = [(-42).into(), (6).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 715827875);

    let mut locals = [1234.into(), 0.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap_err();
    assert_eq!(WasmRuntimeErrorKind::DivideByZero, result.kind());
    assert_eq!(result.opcode(), WasmSingleOpcode::I32DivU.into());
    assert_eq!(result.position(), 5);
}

#[test]
fn div64_s() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x7F, 0x0B];
    let param_types = [WasmValType::I64, WasmValType::I64];
    let result_types = [WasmValType::I64];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [7006652i64.into(), 5678i64.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i64()
        .unwrap();
    assert_eq!(result, 1234);

    let mut locals = [42i64.into(), (-6i64).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i64()
        .unwrap();
    assert_eq!(result, -7);

    let mut locals = [(-42i64).into(), (6i64).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i64()
        .unwrap();
    assert_eq!(result, -7);

    let mut locals = [(-42i64).into(), (-6i64).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i64()
        .unwrap();
    assert_eq!(result, 7);

    let mut locals = [1234i64.into(), 0i64.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap_err();
    assert_eq!(WasmRuntimeErrorKind::DivideByZero, result.kind());
    assert_eq!(result.opcode(), WasmSingleOpcode::I64DivS.into());
    assert_eq!(result.position(), 5);
}

#[test]
fn div64_u() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x80, 0x0B];
    let param_types = [WasmValType::I64, WasmValType::I64];
    let result_types = [WasmValType::I64];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [7006652i64.into(), 5678i64.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i64()
        .unwrap();
    assert_eq!(result, 1234);

    let mut locals = [42i64.into(), (-6i64).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i64()
        .unwrap();
    assert_eq!(result, 0);

    let mut locals = [(-42i64).into(), (6i64).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i64()
        .unwrap();
    assert_eq!(result, 3074457345618258595);

    let mut locals = [1234i64.into(), 0i64.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap_err();
    assert_eq!(WasmRuntimeErrorKind::DivideByZero, result.kind());
    assert_eq!(result.opcode(), WasmSingleOpcode::I64DivU.into());
    assert_eq!(result.position(), 5);
}

#[test]
fn select() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x20, 2, 0x1B, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [123.into(), 456.into(), 789.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 123);

    let mut locals = [123.into(), 456.into(), 0.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 456);
}

#[test]
fn lts() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x48, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [123.into(), 456.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1);

    let mut locals = [123.into(), 123.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let mut locals = [456.into(), 123.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let mut locals = [123.into(), (-456).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let mut locals = [456.into(), (-123).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);
}

#[test]
fn ltu() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x49, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [123.into(), 456.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1);

    let mut locals = [123.into(), 123.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let mut locals = [456.into(), 123.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let mut locals = [123.into(), (-456).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1);

    let mut locals = [456.into(), (-123).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1);
}

#[test]
fn les() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x4C, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [123.into(), 456.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1);

    let mut locals = [123.into(), 123.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1);

    let mut locals = [456.into(), 123.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let mut locals = [123.into(), (-456).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let mut locals = [456.into(), (-123).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);
}

#[test]
fn br_if() {
    let slice = [
        0, 0x02, 0x40, 0x20, 0, 0x20, 1, 0x4C, 0x0d, 0, 0x41, 1, 0x0f, 0x0b, 0x41, 2, 0x0B,
    ];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [123.into(), 456.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 2);

    let mut locals = [123.into(), 123.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 2);

    let mut locals = [456.into(), 123.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1);

    let mut locals = [123.into(), (-456).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1);

    let mut locals = [456.into(), (-123).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1);
}

#[test]
fn br_table() {
    let slice = [
        0, 0x02, 0x40, 0x02, 0x40, 0x0b, 0x0b, 0x02, 0x40, 0x02, 0x40, 0x02, 0x40, 0x20, 0x00,
        0x0e, 0x02, 0x00, 0x01, 0x02, 0x0b, 0x41, 0xfb, 0x00, 0x0f, 0x0b, 0x41, 0xc8, 0x03, 0x0f,
        0x0b, 0x41, 0x95, 0x06, 0x0b,
    ];
    let param_types = [WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [0.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 123);

    let mut locals = [1.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 456);

    let mut locals = [2.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 789);

    let mut locals = [3.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 789);

    let mut locals = [4.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 789);

    let mut locals = [5.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 789);

    let mut locals = [(-1).into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 789);
}

#[test]
fn app_factorial() {
    #[rustfmt::skip]
    let slice = [
        1, 1, WasmValType::I32 as u8,
        0x41, 0x01, 0x21, 0x01, 0x02, 0x40, 0x03, 0x40, 0x20, 0x00, 0x45, 0x0d, 0x01, 0x20, 0x01,
        0x20, 0x00, 0x6c, 0x21, 0x01, 0x20, 0x00, 0x41, 0x01, 0x6b, 0x21, 0x00, 0x0c, 0x00, 0x0b,
        0x0b, 0x20, 0x01, 0x0b, 
    ];
    let param_types = [WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [7.into(), 0.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 5040);

    let mut locals = [10.into(), 0.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 3628800);
}

#[test]
fn app_fib() {
    let module =
        WasmLoader::instantiate(include_bytes!("../test/fib.wasm"), |_, _, _| unreachable!())
            .unwrap();
    let runnable = module.func("fib").unwrap();

    let result = runnable
        .invoke(&[5.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 5);

    let result = runnable
        .invoke(&[10.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 55);

    let result = runnable
        .invoke(&[20.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 6765);
}

#[test]
fn opr_test_i32() {
    let module =
        WasmLoader::instantiate(include_bytes!("../test/opr.wasm"), |_, _, _| unreachable!())
            .unwrap();

    for val in [
        0i32,
        1,
        -1,
        0x1234_5678,
        0x5555_5555,
        0xAAAA_AAAAu32 as i32,
        0x0000_FFFF,
        0xFFFF_0000u32 as i32,
    ] {
        module.memory(0).unwrap().memset(0, 0xCC, 0x1_0000).unwrap();
        let result = module
            .func("test_unary_i32")
            .unwrap()
            .invoke(&[val.into()])
            .unwrap()
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 28);

        let memory = module.memory(0).unwrap();

        assert_eq!(memory.read_u64(0, 0).unwrap(), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8, 0).unwrap(), 0xCCCC_CCCC_CCCC_CCCC);

        assert_eq!(memory.read_u32(0x10, 0).unwrap(), (val == 0) as u32);

        assert_eq!(memory.read_u32(0x14, 0).unwrap(), val.trailing_zeros());
        assert_eq!(memory.read_u32(0x18, 0).unwrap(), val.leading_zeros());
        assert_eq!(memory.read_u32(0x1C, 0).unwrap(), val.count_ones());

        assert_eq!(memory.read_u64(0x20, 0).unwrap(), 0xCCCC_CCCC_CCCC_CCCC);
    }

    for (lhs, rhs) in [
        (1i32, 1i32),
        (-1, -1),
        (1234, 1234),
        (-5678, -5678),
        (1234, 5678),
        (5678, 1234),
        (1234, -1234),
        (-1234, 1234),
        (0x1234_5678, 0x1234_5678),
        (0x7FFF_FFFF, 0x8000_0000u32 as i32),
        (0x8000_0000u32 as i32, 0x7FFF_FFFF),
        (0x1234_5678, 0xFEDC_BA98u32 as i32),
        (0x5555_5555, 0xAAAA_AAAAu32 as i32),
    ] {
        module.memory(0).unwrap().memset(0, 0xCC, 0x1_0000).unwrap();
        let result = module
            .func("test_bin_i32")
            .unwrap()
            .invoke(&[lhs.into(), rhs.into()])
            .unwrap()
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 112);

        let memory = module.memory(0).unwrap();

        assert_eq!(memory.read_u64(0, 0).unwrap(), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8, 0).unwrap(), 0xCCCC_CCCC_CCCC_CCCC);

        assert_eq!(memory.read_u32(0x10, 0).unwrap(), (lhs == rhs) as u32);
        assert_eq!(memory.read_u32(0x14, 0).unwrap(), (lhs != rhs) as u32);
        assert_eq!(memory.read_u32(0x18, 0).unwrap(), (lhs < rhs) as u32);
        assert_eq!(
            memory.read_u32(0x1c, 0).unwrap(),
            ((lhs as u32) < (rhs as u32)) as u32
        );
        assert_eq!(memory.read_u32(0x20, 0).unwrap(), (lhs > rhs) as u32);
        assert_eq!(
            memory.read_u32(0x24, 0).unwrap(),
            ((lhs as u32) > (rhs as u32)) as u32
        );
        assert_eq!(memory.read_u32(0x28, 0).unwrap(), (lhs <= rhs) as u32);
        assert_eq!(
            memory.read_u32(0x2c, 0).unwrap(),
            ((lhs as u32) <= (rhs as u32)) as u32
        );
        assert_eq!(memory.read_u32(0x30, 0).unwrap(), (lhs >= rhs) as u32);
        assert_eq!(
            memory.read_u32(0x34, 0).unwrap(),
            ((lhs as u32) >= (rhs as u32)) as u32
        );

        assert_eq!(
            memory.read_u32(0x38, 0).unwrap() as i32,
            lhs.wrapping_add(rhs)
        );
        assert_eq!(
            memory.read_u32(0x3c, 0).unwrap() as i32,
            lhs.wrapping_sub(rhs)
        );
        assert_eq!(
            memory.read_u32(0x40, 0).unwrap() as i32,
            lhs.wrapping_mul(rhs)
        );
        assert_eq!(
            memory.read_u32(0x44, 0).unwrap() as i32,
            lhs.wrapping_div(rhs)
        );
        assert_eq!(
            memory.read_u32(0x48, 0).unwrap(),
            (lhs as u32).wrapping_div(rhs as u32)
        );
        assert_eq!(
            memory.read_u32(0x4c, 0).unwrap() as i32,
            lhs.wrapping_rem(rhs)
        );
        assert_eq!(
            memory.read_u32(0x50, 0).unwrap(),
            (lhs as u32).wrapping_rem(rhs as u32)
        );

        assert_eq!(memory.read_u32(0x54, 0).unwrap() as i32, lhs & rhs);
        assert_eq!(memory.read_u32(0x58, 0).unwrap() as i32, lhs | rhs);
        assert_eq!(memory.read_u32(0x5c, 0).unwrap() as i32, lhs ^ rhs);

        assert_eq!(
            memory.read_u32(0x60, 0).unwrap() as i32,
            lhs.wrapping_shl(rhs as u32)
        );
        assert_eq!(
            memory.read_u32(0x64, 0).unwrap() as i32,
            lhs.wrapping_shr(rhs as u32)
        );
        assert_eq!(
            memory.read_u32(0x68, 0).unwrap(),
            (lhs as u32).wrapping_shr(rhs as u32)
        );
        assert_eq!(
            memory.read_u32(0x6c, 0).unwrap(),
            (lhs as u32).rotate_left(rhs as u32)
        );
        assert_eq!(
            memory.read_u32(0x70, 0).unwrap(),
            (lhs as u32).rotate_right(rhs as u32)
        );

        assert_eq!(memory.read_u32(0x74, 0).unwrap(), 0xCCCCCCCC);
        assert_eq!(memory.read_u64(0x78, 0).unwrap(), 0xCCCC_CCCC_CCCC_CCCC);
    }
}

#[test]
fn opr_test_i64() {
    let module =
        WasmLoader::instantiate(include_bytes!("../test/opr.wasm"), |_, _, _| unreachable!())
            .unwrap();

    for val in [
        0i64,
        1,
        -1,
        0x1234_5678_ABCD_DEF0,
        0x5555_5555_5555_5555,
        0xAAAA_AAAA_AAAA_AAAAu64 as i64,
        0x0000_0000_FFFF_FFFF,
        0xFFFF_FFFF_0000_0000u64 as i64,
    ] {
        module.memory(0).unwrap().memset(0, 0xCC, 0x1_0000).unwrap();
        let result = module
            .func("test_unary_i64")
            .unwrap()
            .invoke(&[val.into()])
            .unwrap()
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 40);

        let memory = module.memory(0).unwrap();

        assert_eq!(memory.read_u64(0, 0).unwrap(), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8, 0).unwrap(), 0xCCCC_CCCC_CCCC_CCCC);

        assert_eq!(memory.read_u32(0x10, 0).unwrap(), (val == 0) as u32);

        assert_eq!(
            memory.read_u64(0x18, 0).unwrap(),
            val.trailing_zeros() as u64
        );
        assert_eq!(
            memory.read_u64(0x20, 0).unwrap(),
            val.leading_zeros() as u64
        );
        assert_eq!(memory.read_u64(0x28, 0).unwrap(), val.count_ones() as u64);

        assert_eq!(memory.read_u64(0x30, 0).unwrap(), 0xCCCC_CCCC_CCCC_CCCC);
    }

    for (lhs, rhs) in [
        (1i64, 1i64),
        (-1, -1),
        (1234, 1234),
        (-5678, -5678),
        (1234, 5678),
        (5678, 1234),
        (1234, -1234),
        (-1234, 1234),
        (0x1234_5678_9ABC_DEF0, 0x1234_5678_9ABC_DEF0),
        (0x7FFF_FFFF_FFFF_FFFF, 0x8000_0000_0000_0000u64 as i64),
        (0x8000_0000_0000_0000u64 as i64, 0x7FFF_FFFF_FFFF_FFFF),
        (0x1234_5678_9ABC_DEF0, 0xFEDC_BA98_7654_3210u64 as i64),
        (0x5555_5555_5555_5555, 0xAAAA_AAAA_AAAA_AAAAu64 as i64),
    ] {
        module.memory(0).unwrap().memset(0, 0xCC, 0x1_0000).unwrap();
        let result = module
            .func("test_bin_i64")
            .unwrap()
            .invoke(&[lhs.into(), rhs.into()])
            .unwrap()
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 168);

        let memory = module.memory(0).unwrap();

        assert_eq!(memory.read_u64(0, 0).unwrap(), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8, 0).unwrap(), 0xCCCC_CCCC_CCCC_CCCC);

        assert_eq!(memory.read_u32(0x10, 0).unwrap(), (lhs == rhs) as u32);
        assert_eq!(memory.read_u32(0x14, 0).unwrap(), (lhs != rhs) as u32);
        assert_eq!(memory.read_u32(0x18, 0).unwrap(), (lhs < rhs) as u32);
        assert_eq!(
            memory.read_u32(0x1c, 0).unwrap(),
            ((lhs as u64) < (rhs as u64)) as u32
        );
        assert_eq!(memory.read_u32(0x20, 0).unwrap(), (lhs > rhs) as u32);
        assert_eq!(
            memory.read_u32(0x24, 0).unwrap(),
            ((lhs as u64) > (rhs as u64)) as u32
        );
        assert_eq!(memory.read_u32(0x28, 0).unwrap(), (lhs <= rhs) as u32);
        assert_eq!(
            memory.read_u32(0x2c, 0).unwrap(),
            ((lhs as u64) <= (rhs as u64)) as u32
        );
        assert_eq!(memory.read_u32(0x30, 0).unwrap(), (lhs >= rhs) as u32);
        assert_eq!(
            memory.read_u32(0x34, 0).unwrap(),
            ((lhs as u64) >= (rhs as u64)) as u32
        );

        assert_eq!(
            memory.read_u64(0x38, 0).unwrap() as i64,
            lhs.wrapping_add(rhs)
        );
        assert_eq!(
            memory.read_u64(0x40, 0).unwrap() as i64,
            lhs.wrapping_sub(rhs)
        );
        assert_eq!(
            memory.read_u64(0x48, 0).unwrap() as i64,
            lhs.wrapping_mul(rhs)
        );
        assert_eq!(
            memory.read_u64(0x50, 0).unwrap() as i64,
            lhs.wrapping_div(rhs)
        );
        assert_eq!(
            memory.read_u64(0x58, 0).unwrap(),
            (lhs as u64).wrapping_div(rhs as u64)
        );
        assert_eq!(
            memory.read_u64(0x60, 0).unwrap() as i64,
            lhs.wrapping_rem(rhs)
        );
        assert_eq!(
            memory.read_u64(0x68, 0).unwrap(),
            (lhs as u64).wrapping_rem(rhs as u64)
        );

        assert_eq!(memory.read_u64(0x70, 0).unwrap() as i64, lhs & rhs);
        assert_eq!(memory.read_u64(0x78, 0).unwrap() as i64, lhs | rhs);
        assert_eq!(memory.read_u64(0x80, 0).unwrap() as i64, lhs ^ rhs);

        assert_eq!(
            memory.read_u64(0x88, 0).unwrap() as i64,
            lhs.wrapping_shl(rhs as u32)
        );
        assert_eq!(
            memory.read_u64(0x90, 0).unwrap() as i64,
            lhs.wrapping_shr(rhs as u32)
        );
        assert_eq!(
            memory.read_u64(0x98, 0).unwrap(),
            (lhs as u64).wrapping_shr(rhs as u32)
        );
        assert_eq!(
            memory.read_u64(0xA0, 0).unwrap(),
            (lhs as u64).rotate_left(rhs as u32)
        );
        assert_eq!(
            memory.read_u64(0xA8, 0).unwrap(),
            (lhs as u64).rotate_right(rhs as u32)
        );

        assert_eq!(memory.read_u64(0xB0, 0).unwrap(), 0xCCCC_CCCC_CCCC_CCCC);
    }
}

#[test]
fn global() {
    let slice = include_bytes!("../test/global.wasm");
    let module = WasmLoader::instantiate(slice, |_, _, _| unreachable!()).unwrap();
    let runnable = module.func_by_index(0).unwrap();

    assert_eq!(module.global_get(0).unwrap().value().get_i32(), Ok(123));

    let result = runnable
        .invoke(&[456.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 579);

    assert_eq!(module.global_get(0).unwrap().value().get_i32(), Ok(579));

    let result = runnable
        .invoke(&[789.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1368);

    assert_eq!(module.global_get(0).unwrap().value().get_i32(), Ok(1368));
}

#[test]
fn name() {
    let slice = [
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x00, 0x1F, 0x04, 0x6E,
        0x61, 0x6D, 0x65, 0x00, 0x06, 0x05, 0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x01, 0x0E, 0x02, 0x01,
        0x04, 0x77, 0x61, 0x73, 0x6D, 0xB4, 0x24, 0x04, 0x74, 0x65, 0x73, 0x74, 0x7F, 0x00,
    ];
    let module = WasmLoader::instantiate(&slice, |_, _, _| unreachable!()).unwrap();
    let names = module.names().unwrap();

    assert_eq!(names.module().unwrap(), "Hello");

    assert_eq!(names.functions()[0], (1, "wasm".to_owned()));

    assert_eq!(names.func_by_index(0x1234).unwrap(), "test");
}

#[test]
#[cfg(feature = "float")]
fn float_reinterpret() {
    let slice = [1, 1, 0x7F, 0x20, 0x01, 0xbe, 0x0B];
    let param_types = [WasmValType::I32];
    let result_types = [WasmValType::F32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [0x40490fdb.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_f32()
        .unwrap();
    assert_eq!(result, 3.1415927);

    let slice = [1, 1, 0x7D, 0x20, 0x01, 0xbc, 0x0B];
    let param_types = [WasmValType::F32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [3.1415927f32.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_u32()
        .unwrap();
    assert_eq!(result, 0x40490fdb);
}

#[test]
#[cfg(feature = "float")]
fn float64_reinterpret() {
    let slice = [1, 1, 0x7E, 0x20, 0x01, 0xbf, 0x0B];
    let param_types = [WasmValType::I64];
    let result_types = [WasmValType::F64];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [0x400921fb54442d18u64.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_f64()
        .unwrap();
    assert_eq!(result, PI);

    let slice = [1, 1, 0x7C, 0x20, 0x01, 0xbd, 0x0B];
    let param_types = [WasmValType::F64];
    let result_types = [WasmValType::I64];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let mut locals = [PI.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_u64()
        .unwrap();
    assert_eq!(result, 0x400921fb54442d18u64);
}
