use crate::cg::{
    intr::{WasmInterpreter, WasmInvocation},
    WasmCodeBlock,
};
use crate::leb128::*;
use crate::opcode::WasmMnemonic;
use crate::*;
use core::f64::consts::PI;
use num_traits::Zero;
use std::assert_matches::assert_matches;

struct Env;

#[wasm_env]
impl Env {
    pub fn add(a: i32, b: i32) -> i32 {
        a.wrapping_add(b)
    }

    pub fn sub(a: i32, b: i32) -> i32 {
        a.wrapping_sub(b)
    }
}

#[test]
fn instantiate() {
    let data = [0, 97, 115, 109, 1, 0, 0, 0];
    WebAssembly::instantiate(&data, &Env {}).unwrap();

    let data = [0, 97, 115, 109, 1, 0, 0];
    assert_matches!(
        CompileErrorKind::downcast_ref(&WebAssembly::instantiate(&data, &Env {}).unwrap_err())
            .unwrap(),
        CompileErrorKind::BadExecutable
    );

    let data = [0, 97, 115, 109, 2, 0, 0, 0];
    assert_matches!(
        CompileErrorKind::downcast_ref(&WebAssembly::instantiate(&data, &Env {}).unwrap_err())
            .unwrap(),
        CompileErrorKind::BadExecutable
    );

    let data = [0, 97, 115, 109, 1, 0, 0, 0, 1];
    assert_matches!(
        CompileErrorKind::downcast_ref(&WebAssembly::instantiate(&data, &Env {}).unwrap_err())
            .unwrap(),
        CompileErrorKind::UnexpectedEof
    );

    let instance =
        WebAssembly::instantiate(include_bytes!("../test/tester.wasm"), &Env {}).unwrap();
    let _ = instance.function("fib").unwrap();
}

#[test]
fn section_order() {
    let data = [
        0, 97, 115, 109, 1, 0, 0, 0, 1, 1, 0, 2, 1, 0, 2, 1, 0, 3, 1, 0, 4, 1, 0, 5, 1, 0,
    ];
    WebAssembly::instantiate(&data, &Env {}).unwrap();

    let data = [
        0, 97, 115, 109, 1, 0, 0, 0, 1, 1, 0, 2, 1, 0, 2, 1, 0, 3, 1, 0, 4, 1, 0, 3, 1, 0,
    ];
    assert_matches!(
        CompileErrorKind::downcast_ref(&WebAssembly::instantiate(&data, &Env {}).unwrap_err())
            .unwrap(),
        CompileErrorKind::InvalidSectionOrder(WasmSectionId::Function)
    );
}

#[test]
fn i32_const() {
    let slice = [0, 0x41, 0xf8, 0xac, 0xd1, 0x91, 0x01, 0x0B];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, instance.module()).unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0x12345678);
}

#[test]
fn i32_const_type_mismatch() {
    let slice = [0, 0x41, 0x00, 0x01, 0x0B];
    let result_types = [WasmValType::I64];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    assert_matches!(
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, instance.module())
            .unwrap_err()
            .kind(),
        CompileErrorKind::TypeMismatch
    );

    let slice = [0, 0x41, 0x00, 0x01, 0x0B];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, instance.module()).unwrap();
    let mut interp = WasmInterpreter::new(&instance);

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
    assert_matches!(result2, Err(WasmRuntimeErrorKind::TypeMismatch));
}

#[test]
fn i64_const() {
    let slice = [
        0, 0x42, 0xef, 0x9b, 0xaf, 0xcd, 0xf8, 0xac, 0xd1, 0x91, 0x01, 0x0B,
    ];
    let result_types = [WasmValType::I64];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, instance.module()).unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_i64()
        .unwrap();
    assert_eq!(result, 0x123456789abcdef);
}

#[test]
fn i64_const_type_mismatch() {
    let slice = [0, 0x42, 0x00, 0x01, 0x0B];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    assert_matches!(
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, instance.module())
            .unwrap_err()
            .kind(),
        CompileErrorKind::TypeMismatch
    );
}

#[test]
fn float_const() {
    let slice = [0, 0x43, 0, 0, 0, 0, 0x0B];
    let result_types = [WasmValType::F32];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, instance.module()).unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_f32()
        .unwrap();
    assert_eq!(result, 0.0);

    let slice = [0, 0x43, 0, 0, 0xc0, 0x7f, 0x0B];
    let result_types = [WasmValType::F32];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, instance.module()).unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_f32()
        .unwrap();
    assert!(result.is_nan());

    let slice = [0, 0x43, 0, 0, 0x80, 0x7f, 0x0B];
    let result_types = [WasmValType::F32];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, instance.module()).unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_f32()
        .unwrap();
    assert!(result.is_infinite());

    let slice = [0, 0x43, 0xdb, 0x0f, 0x49, 0x40, 0x0B];
    let result_types = [WasmValType::F32];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, instance.module()).unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_f32()
        .unwrap();
    assert_eq!(result, 3.1415927);
}

#[test]
fn float64_const() {
    let slice = [0, 0x44, 0, 0, 0, 0, 0, 0, 0, 0, 0x0B];
    let result_types = [WasmValType::F64];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, instance.module()).unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_f64()
        .unwrap();
    assert_eq!(result, 0.0);

    let slice = [0, 0x44, 0, 0, 0, 0, 0, 0, 0xf8, 0x7f, 0x0B];
    let result_types = [WasmValType::F64];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, instance.module()).unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_f64()
        .unwrap();
    assert!(result.is_nan());

    let slice = [0, 0x44, 0, 0, 0, 0, 0, 0, 0xf0, 0x7f, 0x0B];
    let result_types = [WasmValType::F64];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, instance.module()).unwrap();
    let mut interp = WasmInterpreter::new(&instance);

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
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info =
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &result_types, instance.module()).unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let result = interp
        .invoke(0, &info, &mut [], &result_types)
        .unwrap()
        .unwrap()
        .get_f64()
        .unwrap();
    assert_eq!(result, PI);
}

#[test]
fn const_local() {
    let slice = [
        1, 1, 0x7F, 0x41, 0xFB, 0x00, 0x21, 0, 0x41, 0x12, 0x1A, 0x20, 0, 0x0B,
    ];
    let param_types = [];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info = WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &param_types,
        &result_types,
        instance.module(),
    )
    .unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let result = interp
        .invoke(0, &info, &[0.into()], &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 123);
}

#[test]
fn div32_s() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x6D, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info = WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &param_types,
        &result_types,
        instance.module(),
    )
    .unwrap();
    let mut interp = WasmInterpreter::new(&instance);

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
    let err: Box<WasmRuntimeError> = result.downcast().unwrap();
    assert_matches!(*err.kind(), WasmRuntimeErrorKind::DivideByZero);
    assert_eq!(err.mnemonic(), WasmMnemonic::I32DivS);
    assert_eq!(err.position(), 5);
}

#[test]
fn div32_u() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x6E, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info = WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &param_types,
        &result_types,
        instance.module(),
    )
    .unwrap();
    let mut interp = WasmInterpreter::new(&instance);

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
    let err: Box<WasmRuntimeError> = result.downcast().unwrap();
    assert_matches!(*err.kind(), WasmRuntimeErrorKind::DivideByZero);
    assert_eq!(err.mnemonic(), WasmMnemonic::I32DivU);
    assert_eq!(err.position(), 5);
}

#[test]
fn div64_s() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x7F, 0x0B];
    let param_types = [WasmValType::I64, WasmValType::I64];
    let result_types = [WasmValType::I64];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info = WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &param_types,
        &result_types,
        instance.module(),
    )
    .unwrap();
    let mut interp = WasmInterpreter::new(&instance);

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
    let err: Box<WasmRuntimeError> = result.downcast().unwrap();
    assert_matches!(*err.kind(), WasmRuntimeErrorKind::DivideByZero);
    assert_eq!(err.mnemonic(), WasmMnemonic::I64DivS);
    assert_eq!(err.position(), 5);
}

#[test]
fn div64_u() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x80, 0x0B];
    let param_types = [WasmValType::I64, WasmValType::I64];
    let result_types = [WasmValType::I64];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info = WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &param_types,
        &result_types,
        instance.module(),
    )
    .unwrap();
    let mut interp = WasmInterpreter::new(&instance);

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
    let err: Box<WasmRuntimeError> = result.downcast().unwrap();
    assert_matches!(*err.kind(), WasmRuntimeErrorKind::DivideByZero);
    assert_eq!(err.mnemonic(), WasmMnemonic::I64DivU);
    assert_eq!(err.position(), 5);
}

#[test]
fn select_int() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x20, 2, 0x1B, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info = WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &param_types,
        &result_types,
        instance.module(),
    )
    .unwrap();
    let mut interp = WasmInterpreter::new(&instance);

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
fn select_float() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x20, 2, 0x1B, 0x0B];
    let param_types = [WasmValType::F64, WasmValType::F64, WasmValType::I32];
    let result_types = [WasmValType::F64];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info = WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &param_types,
        &result_types,
        instance.module(),
    )
    .unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let mut locals = [0.0.into(), PI.into(), 789.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_f64()
        .unwrap();
    assert_eq!(result, 0.0);

    let mut locals = [0.0.into(), PI.into(), 0.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_f64()
        .unwrap();
    assert_eq!(result, PI);
}

#[test]
fn br_if() {
    let slice = [
        0, 0x02, 0x40, 0x20, 0, 0x20, 1, 0x4C, 0x0d, 0, 0x41, 1, 0x0f, 0x0b, 0x41, 2, 0x0B,
    ];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info = WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &param_types,
        &result_types,
        instance.module(),
    )
    .unwrap();
    let mut interp = WasmInterpreter::new(&instance);

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
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info = WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &param_types,
        &result_types,
        instance.module(),
    )
    .unwrap();
    let mut interp = WasmInterpreter::new(&instance);

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
fn app_fact() {
    let instance =
        WebAssembly::instantiate(include_bytes!("../test/tester.wasm"), &Env {}).unwrap();
    let runnable = instance.function("fact").unwrap();

    let result = runnable
        .invoke(&[7.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 5040);

    let result = runnable
        .invoke(&[10.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 3628800);
}

#[test]
fn app_fib() {
    let instance =
        WebAssembly::instantiate(include_bytes!("../test/tester.wasm"), &Env {}).unwrap();
    let runnable = instance.function("fib").unwrap();

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
    let instance =
        WebAssembly::instantiate(include_bytes!("../test/tester.wasm"), &Env {}).unwrap();

    let memory = instance.memory(0).unwrap().try_borrow().unwrap();

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
        memory.fill(0xCC);
        let result = instance
            .function("test_unary_i32")
            .unwrap()
            .invoke(&[val.into()])
            .unwrap()
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 0x24);

        assert_eq!(memory.read_u64(0), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8), 0xCCCC_CCCC_CCCC_CCCC);

        assert_eq!(memory.read_u32(0x10), (val == 0) as u32);
        assert_eq!(memory.read_u32(0x14), val.trailing_zeros());
        assert_eq!(memory.read_u32(0x18), val.leading_zeros());
        assert_eq!(memory.read_u32(0x1C), val.count_ones());
        assert_eq!(memory.read_u32(0x20), ((val as i8) as i32) as u32);
        assert_eq!(memory.read_u32(0x24), ((val as i16) as i32) as u32);

        assert_eq!(memory.read_u64(0x28), 0xCCCC_CCCC_CCCC_CCCC);

        memory.fill(0xCC);
        let result = instance
            .function("test_fusion_unary_i32")
            .unwrap()
            .invoke(&[val.into()])
            .unwrap()
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 0x50);

        assert_eq!(memory.read_u64(0), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8), 0xCCCC_CCCC_CCCC_CCCC);

        let valu = val as u32;
        assert_eq!(memory.read_i32(0x10), 0x12345678);
        assert_eq!(memory.read_i32(0x14), val.wrapping_add(1234));
        assert_eq!(memory.read_i32(0x18), val.wrapping_sub(1234));
        assert_eq!(memory.read_i32(0x1C), val.wrapping_add(5678));

        assert_eq!(memory.read_u32(0x20), valu & 0x5555_5555);
        assert_eq!(memory.read_u32(0x24), valu & 0xaaaa_aaaa);
        assert_eq!(memory.read_u32(0x28), valu | 0x5555_5555);
        assert_eq!(memory.read_u32(0x2C), valu | 0xaaaa_aaaa);
        assert_eq!(memory.read_u32(0x30), valu ^ 0x5555_5555);
        assert_eq!(memory.read_u32(0x34), valu ^ 0xaaaa_aaaa);

        assert_eq!(memory.read_u32(0x38), valu.wrapping_shl(7));
        assert_eq!(memory.read_u32(0x3C), valu.wrapping_shl(19));
        assert_eq!(memory.read_i32(0x40), val.wrapping_shr(5));
        assert_eq!(memory.read_i32(0x44), val.wrapping_shr(17));
        assert_eq!(memory.read_u32(0x48), valu.wrapping_shr(3));
        assert_eq!(memory.read_u32(0x4C), valu.wrapping_shr(13));

        assert_eq!(memory.read_i32(0x50), if val == 0 { 1234 } else { 5678 });

        assert_eq!(memory.read_u32(0x54), 0xCCCC_CCCC);
        assert_eq!(memory.read_u64(0x58), 0xCCCC_CCCC_CCCC_CCCC);
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
        let lhsu = lhs as u32;
        let rhsu = rhs as u32;

        memory.fill(0xCC);
        let result = instance
            .function("test_bin_i32")
            .unwrap()
            .invoke(&[lhs.into(), rhs.into()])
            .unwrap()
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 112);

        assert_eq!(memory.read_u64(0), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8), 0xCCCC_CCCC_CCCC_CCCC);

        assert_eq!(memory.read_u32(0x10), (lhs == rhs) as u32);
        assert_eq!(memory.read_u32(0x14), (lhs != rhs) as u32);
        assert_eq!(memory.read_u32(0x18), (lhs < rhs) as u32);
        assert_eq!(memory.read_u32(0x1c), (lhsu < rhsu) as u32);
        assert_eq!(memory.read_u32(0x20), (lhs > rhs) as u32);
        assert_eq!(memory.read_u32(0x24), (lhsu > rhsu) as u32);
        assert_eq!(memory.read_u32(0x28), (lhs <= rhs) as u32);
        assert_eq!(memory.read_u32(0x2c), (lhsu <= rhsu) as u32);
        assert_eq!(memory.read_u32(0x30), (lhs >= rhs) as u32);
        assert_eq!(memory.read_u32(0x34), (lhsu >= rhsu) as u32);

        assert_eq!(memory.read_u32(0x38) as i32, lhs.wrapping_add(rhs));
        assert_eq!(memory.read_u32(0x3c) as i32, lhs.wrapping_sub(rhs));
        assert_eq!(memory.read_u32(0x40) as i32, lhs.wrapping_mul(rhs));
        assert_eq!(memory.read_u32(0x44) as i32, lhs.wrapping_div(rhs));
        assert_eq!(memory.read_u32(0x48), lhsu.wrapping_div(rhsu));
        assert_eq!(memory.read_u32(0x4c) as i32, lhs.wrapping_rem(rhs));
        assert_eq!(memory.read_u32(0x50), lhsu.wrapping_rem(rhsu));

        assert_eq!(memory.read_u32(0x54) as i32, lhs & rhs);
        assert_eq!(memory.read_u32(0x58) as i32, lhs | rhs);
        assert_eq!(memory.read_u32(0x5c) as i32, lhs ^ rhs);

        assert_eq!(memory.read_u32(0x60) as i32, lhs.wrapping_shl(rhsu));
        assert_eq!(memory.read_u32(0x64) as i32, lhs.wrapping_shr(rhsu));
        assert_eq!(memory.read_u32(0x68), lhsu.wrapping_shr(rhsu));
        assert_eq!(memory.read_u32(0x6c), lhsu.rotate_left(rhsu));
        assert_eq!(memory.read_u32(0x70), lhsu.rotate_right(rhsu));

        assert_eq!(memory.read_u32(0x74), 0xCCCCCCCC);
        assert_eq!(memory.read_u64(0x78), 0xCCCC_CCCC_CCCC_CCCC);

        memory.fill(0xCC);
        let result = instance
            .function("test_fusion_binary_i32")
            .unwrap()
            .invoke(&[lhs.into(), rhs.into()])
            .unwrap()
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 0x34);

        assert_eq!(memory.read_u64(0), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8), 0xCCCC_CCCC_CCCC_CCCC);

        assert_eq!(memory.read_u32(0x10), if lhs == rhs { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x14), if lhs != rhs { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x18), if lhs < rhs { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x1c), if lhsu < rhsu { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x20), if lhs > rhs { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x24), if lhsu > rhsu { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x28), if lhs <= rhs { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x2c), if lhsu <= rhsu { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x30), if lhs >= rhs { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x34), if lhsu >= rhsu { 123 } else { 456 });

        assert_eq!(memory.read_u64(0x38), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(0x40), 0xCCCC_CCCC_CCCC_CCCC);
    }
}

#[test]
fn opr_test_i64() {
    let instance =
        WebAssembly::instantiate(include_bytes!("../test/tester.wasm"), &Env {}).unwrap();

    let memory = instance.memory(0).unwrap().try_borrow().unwrap();

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
        memory.fill(0xCC);
        let result = instance
            .function("test_unary_i64")
            .unwrap()
            .invoke(&[val.into()])
            .unwrap()
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 0x48);

        assert_eq!(memory.read_u64(0), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8), 0xCCCC_CCCC_CCCC_CCCC);

        assert_eq!(memory.read_u32(0x10), (val == 0) as u32);
        assert_eq!(memory.read_u64(0x18), val.trailing_zeros() as u64);
        assert_eq!(memory.read_u64(0x20), val.leading_zeros() as u64);
        assert_eq!(memory.read_u64(0x28), val.count_ones() as u64);
        assert_eq!(memory.read_u64(0x30), ((val as i8) as i64) as u64);
        assert_eq!(memory.read_u64(0x38), ((val as i16) as i64) as u64);
        assert_eq!(memory.read_u64(0x40), ((val as i32) as i64) as u64);

        assert_eq!(memory.read_u32(0x48), (val as u32));
        assert_eq!(memory.read_u32(0x4C), 0xCCCC_CCCC);

        assert_eq!(memory.read_u64(0x50), 0xCCCC_CCCC_CCCC_CCCC);

        memory.fill(0xCC);
        let result = instance
            .function("test_fused_i64")
            .unwrap()
            .invoke(&[val.into()])
            .unwrap()
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 0x90);

        assert_eq!(memory.read_u64(0), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8), 0xCCCC_CCCC_CCCC_CCCC);

        let valu = val as u64;
        assert_eq!(memory.read_i64(0x10), 0x12345678);
        assert_eq!(memory.read_i64(0x18), val.wrapping_add(12345678));
        assert_eq!(memory.read_i64(0x20), val.wrapping_sub(12345678));
        assert_eq!(memory.read_i64(0x28), val.wrapping_add(987654321));

        assert_eq!(memory.read_u64(0x30), valu & 0x5555_5555_5555_5555);
        assert_eq!(memory.read_u64(0x38), valu & 0xaaaa_aaaa_aaaa_aaaa);
        assert_eq!(memory.read_u64(0x40), valu | 0x5555_5555_5555_5555);
        assert_eq!(memory.read_u64(0x48), valu | 0xaaaa_aaaa_aaaa_aaaa);
        assert_eq!(memory.read_u64(0x50), valu ^ 0x5555_5555_5555_5555);
        assert_eq!(memory.read_u64(0x58), valu ^ 0xaaaa_aaaa_aaaa_aaaa);

        assert_eq!(memory.read_u64(0x60), valu.wrapping_shl(7));
        assert_eq!(memory.read_u64(0x68), valu.wrapping_shl(19));
        assert_eq!(memory.read_i64(0x70), val.wrapping_shr(5));
        assert_eq!(memory.read_i64(0x78), val.wrapping_shr(17));
        assert_eq!(memory.read_u64(0x80), valu.wrapping_shr(3));
        assert_eq!(memory.read_u64(0x88), valu.wrapping_shr(13));

        assert_eq!(memory.read_i32(0x90), if val == 0 { 1234 } else { 5678 });

        assert_eq!(memory.read_u32(0x94), 0xCCCC_CCCC);
        assert_eq!(memory.read_u64(0x98), 0xCCCC_CCCC_CCCC_CCCC);
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
        (0x1111_1111_1234_5678, 0x0000_0000_1234_5678),
        (0x1234_5678_9ABC_DEF0, 0x1234_5678_9ABC_DEF0),
        (0x7FFF_FFFF_FFFF_FFFF, 0x8000_0000_0000_0000u64 as i64),
        (0x8000_0000_0000_0000u64 as i64, 0x7FFF_FFFF_FFFF_FFFF),
        (0x1234_5678_9ABC_DEF0, 0xFEDC_BA98_7654_3210u64 as i64),
        (0x5555_5555_5555_5555, 0xAAAA_AAAA_AAAA_AAAAu64 as i64),
    ] {
        let lhsu = lhs as u64;
        let rhsu = rhs as u64;

        memory.fill(0xCC);
        let result = instance
            .function("test_bin_i64")
            .unwrap()
            .invoke(&[lhs.into(), rhs.into()])
            .unwrap()
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 168);

        assert_eq!(memory.read_u64(0), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8), 0xCCCC_CCCC_CCCC_CCCC);

        assert_eq!(memory.read_u32(0x10), (lhs == rhs) as u32);
        assert_eq!(memory.read_u32(0x14), (lhs != rhs) as u32);
        assert_eq!(memory.read_u32(0x18), (lhs < rhs) as u32);
        assert_eq!(memory.read_u32(0x1c), (lhsu < rhsu) as u32);
        assert_eq!(memory.read_u32(0x20), (lhs > rhs) as u32);
        assert_eq!(memory.read_u32(0x24), (lhsu > rhsu) as u32);
        assert_eq!(memory.read_u32(0x28), (lhs <= rhs) as u32);
        assert_eq!(memory.read_u32(0x2c), (lhsu <= rhsu) as u32);
        assert_eq!(memory.read_u32(0x30), (lhs >= rhs) as u32);
        assert_eq!(memory.read_u32(0x34), (lhsu >= rhsu) as u32);

        assert_eq!(memory.read_u64(0x38) as i64, lhs.wrapping_add(rhs));
        assert_eq!(memory.read_u64(0x40) as i64, lhs.wrapping_sub(rhs));
        assert_eq!(memory.read_u64(0x48) as i64, lhs.wrapping_mul(rhs));
        assert_eq!(memory.read_u64(0x50) as i64, lhs.wrapping_div(rhs));
        assert_eq!(memory.read_u64(0x58), lhsu.wrapping_div(rhsu));
        assert_eq!(memory.read_u64(0x60) as i64, lhs.wrapping_rem(rhs));
        assert_eq!(memory.read_u64(0x68), lhsu.wrapping_rem(rhsu));

        assert_eq!(memory.read_u64(0x70) as i64, lhs & rhs);
        assert_eq!(memory.read_u64(0x78) as i64, lhs | rhs);
        assert_eq!(memory.read_u64(0x80) as i64, lhs ^ rhs);

        assert_eq!(memory.read_u64(0x88) as i64, lhs.wrapping_shl(rhs as u32));
        assert_eq!(memory.read_u64(0x90) as i64, lhs.wrapping_shr(rhs as u32));
        assert_eq!(memory.read_u64(0x98), lhsu.wrapping_shr(rhs as u32));
        assert_eq!(memory.read_u64(0xA0), lhsu.rotate_left(rhs as u32));
        assert_eq!(memory.read_u64(0xA8), lhsu.rotate_right(rhs as u32));

        assert_eq!(memory.read_u64(0xB0), 0xCCCC_CCCC_CCCC_CCCC);

        memory.fill(0xCC);
        let result = instance
            .function("test_fusion_binary_i64")
            .unwrap()
            .invoke(&[lhs.into(), rhs.into()])
            .unwrap()
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 0x34);

        assert_eq!(memory.read_u64(0), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8), 0xCCCC_CCCC_CCCC_CCCC);

        assert_eq!(memory.read_u32(0x10), if lhs == rhs { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x14), if lhs != rhs { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x18), if lhs < rhs { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x1c), if lhsu < rhsu { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x20), if lhs > rhs { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x24), if lhsu > rhsu { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x28), if lhs <= rhs { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x2c), if lhsu <= rhsu { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x30), if lhs >= rhs { 123 } else { 456 });
        assert_eq!(memory.read_u32(0x34), if lhsu >= rhsu { 123 } else { 456 });

        assert_eq!(memory.read_u64(0x38), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(0x40), 0xCCCC_CCCC_CCCC_CCCC);
    }
}

#[test]
fn call_test() {
    let instance =
        WebAssembly::instantiate(include_bytes!("../test/tester.wasm"), &Env {}).unwrap();

    let memory = instance.memory(0).unwrap().try_borrow().unwrap();

    for (a1, a2, a3, a4) in [
        (1u32, 2u32, 3u64, 4u64),
        (
            0x1234_5678,
            0xFEDC_BA98,
            0x1234_5678_9ABC_DEF0,
            0xFEDC_BA98_7654_3210,
        ),
    ] {
        memory.fill(0xCC);
        let result = instance
            .function("call_test1")
            .unwrap()
            .invoke(&[a1.into(), a2.into(), a3.into(), a4.into()])
            .unwrap()
            .unwrap()
            .get_u32()
            .unwrap();
        assert_eq!(result, a1);

        assert_eq!(memory.read_u64(0), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8), 0xCCCC_CCCC_CCCC_CCCC);

        assert_eq!(memory.read_u32(0x10), a1);
        assert_eq!(memory.read_u32(0x14), a2);
        assert_eq!(memory.read_u64(0x18), a3);
        assert_eq!(memory.read_u64(0x20), a4);

        assert_eq!(memory.read_u64(0x28), 0xCCCC_CCCC_CCCC_CCCC);

        memory.fill(0xCC);
        let result = instance
            .function("call_test2")
            .unwrap()
            .invoke(&[a1.into(), a2.into(), a3.into(), a4.into()])
            .unwrap()
            .unwrap()
            .get_u32()
            .unwrap();
        assert_eq!(result, a1.wrapping_add(a2));

        assert_eq!(memory.read_u64(0), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8), 0xCCCC_CCCC_CCCC_CCCC);

        assert_eq!(memory.read_u32(0x10), a2);
        assert_eq!(memory.read_u32(0x14), a1);
        assert_eq!(memory.read_u64(0x18), a4);
        assert_eq!(memory.read_u64(0x20), a3);

        assert_eq!(memory.read_u64(0x28), 0xCCCC_CCCC_CCCC_CCCC);

        memory.fill(0xCC);
        let result = instance
            .function("call_test3")
            .unwrap()
            .invoke(&[a1.into(), a2.into(), a3.into(), a4.into()])
            .unwrap()
            .unwrap()
            .get_u64()
            .unwrap();
        assert_eq!(result, a3);

        assert_eq!(memory.read_u64(0), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8), 0xCCCC_CCCC_CCCC_CCCC);

        assert_eq!(memory.read_u32(0x10), a1);
        assert_eq!(memory.read_u32(0x14), a2);
        assert_eq!(memory.read_u64(0x18), a3);
        assert_eq!(memory.read_u64(0x20), a4);

        assert_eq!(memory.read_u64(0x28), 0xCCCC_CCCC_CCCC_CCCC);

        memory.fill(0xCC);
        let result = instance
            .function("call_test4")
            .unwrap()
            .invoke(&[a1.into(), a2.into(), a3.into(), a4.into()])
            .unwrap()
            .unwrap()
            .get_u64()
            .unwrap();
        assert_eq!(result, a4);

        assert_eq!(memory.read_u64(0), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8), 0xCCCC_CCCC_CCCC_CCCC);

        assert_eq!(memory.read_u32(0x10), a1.wrapping_add(a2));
        assert_eq!(memory.read_u32(0x14), a1.wrapping_sub(a2));
        assert_eq!(memory.read_u64(0x18), a3.wrapping_add(a4));
        assert_eq!(memory.read_u64(0x20), a3.wrapping_sub(a4));

        assert_eq!(memory.read_u64(0x28), 0xCCCC_CCCC_CCCC_CCCC);
    }
}

#[test]
fn call_indirect_test() {
    let instance =
        WebAssembly::instantiate(include_bytes!("../test/tester.wasm"), &Env {}).unwrap();

    // let memory = instance.memory(0).unwrap().try_borrow().unwrap();

    for a1 in [0i32, 0x12345678, 0x55555555, 0xAAAAAAAAu32 as i32] {
        for i in 1..=3 {
            let base = [0i32, 123, -456, 789][i as usize];
            let result = instance
                .function("call_indirect_test")
                .unwrap()
                .invoke(&[(i as i32).into(), a1.into()])
                .unwrap()
                .unwrap()
                .get_i32()
                .unwrap();
            assert_eq!(result, a1.wrapping_add(base));
        }

        let err: Box<WasmRuntimeError> = instance
            .function("call_indirect_test")
            .unwrap()
            .invoke(&[4.into(), a1.into()])
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_matches!(err.kind(), WasmRuntimeErrorKind::TypeMismatch);

        let err: Box<WasmRuntimeError> = instance
            .function("call_indirect_test")
            .unwrap()
            .invoke(&[100.into(), a1.into()])
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_matches!(err.kind(), WasmRuntimeErrorKind::NoMethod);
    }
}

#[test]
fn mem_load_store() {
    let instance =
        WebAssembly::instantiate(include_bytes!("../test/tester.wasm"), &Env {}).unwrap();

    let mut src = Vec::new();
    for i in 0..65536 {
        src.push(0xFF ^ i as u8);
    }
    let src = src.as_slice();

    #[inline]
    #[track_caller]
    fn reset_memory(instance: &WasmInstance, src: &[u8]) {
        instance
            .memory(0)
            .unwrap()
            .borrowing(|memory| {
                memory[0..65536].copy_from_slice(src);
            })
            .unwrap();
    }

    macro_rules! test_memory {
        ($instance:ident, $expected:expr) => {
            $instance
                .memory(0)
                .unwrap()
                .borrowing(|memory| {
                    let mut vec = Vec::new();
                    for index in 0..65536 {
                        let lhs = memory[index];
                        let rhs = $expected[index];
                        if lhs != rhs {
                            vec.push((index, lhs, rhs));
                        }
                    }
                    if vec.len() > 0 {
                        panic!("MEMORY ERROR (index, actual, expected): {:?}", vec);
                    }
                })
                .unwrap();
        };
    }

    for (a1, a2) in [(0x12u32, 0x34u32), (0x87, 0x65), (0xAA, 0x55), (0x55, 0xAA)] {
        // u32u8
        reset_memory(&instance, src);
        let expected = 0xFF ^ a1;
        let result = instance
            .function("mem_test_u32u8")
            .unwrap()
            .invoke(&[a1.into(), a2.into()])
            .unwrap()
            .unwrap()
            .get_u32()
            .unwrap();
        assert_eq!(result, expected);

        let mut expected = Vec::new();
        expected.extend_from_slice(src);
        expected[a2 as usize] = expected[a1 as usize];
        test_memory!(instance, &expected);

        // i32i8
        reset_memory(&instance, src);
        let expected = (-1 ^ a1 as i8) as i32;
        let result = instance
            .function("mem_test_i32i8")
            .unwrap()
            .invoke(&[a1.into(), a2.into()])
            .unwrap()
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, expected);

        let mut expected = Vec::new();
        expected.extend_from_slice(src);
        expected[a2 as usize] = expected[a1 as usize];
        test_memory!(instance, &expected);

        // u64u8
        reset_memory(&instance, src);
        let expected = 0xFF ^ a1 as u64;
        let result = instance
            .function("mem_test_u64u8")
            .unwrap()
            .invoke(&[a1.into(), a2.into()])
            .unwrap()
            .unwrap()
            .get_u64()
            .unwrap();
        assert_eq!(result, expected);

        let mut expected = Vec::new();
        expected.extend_from_slice(src);
        expected[a2 as usize] = expected[a1 as usize];
        test_memory!(instance, &expected);

        // i64i8
        reset_memory(&instance, src);
        let expected = (-1 ^ a1 as i8) as i64;
        let result = instance
            .function("mem_test_i64i8")
            .unwrap()
            .invoke(&[a1.into(), a2.into()])
            .unwrap()
            .unwrap()
            .get_i64()
            .unwrap();
        assert_eq!(result, expected);

        let mut expected = Vec::new();
        expected.extend_from_slice(src);
        expected[a2 as usize] = expected[a1 as usize];
        test_memory!(instance, &expected);

        let a1 = a1 * 2;
        let a2 = a2 * 2;
        let a1b = a1 & 0xFF;

        // u32u16
        reset_memory(&instance, src);
        let expected = 0xFFFF ^ (a1b + (a1b + 1) * 0x100);
        let result = instance
            .function("mem_test_u32u16")
            .unwrap()
            .invoke(&[a1.into(), a2.into()])
            .unwrap()
            .unwrap()
            .get_u32()
            .unwrap();
        assert_eq!(result, expected);

        let mut expected = Vec::new();
        expected.extend_from_slice(src);
        expected[a2 as usize] = expected[a1 as usize];
        expected[a2 as usize + 1] = expected[a1 as usize + 1];
        test_memory!(instance, &expected);

        // i32i16
        reset_memory(&instance, src);
        let expected = -1 ^ ((a1b + (a1b + 1) * 0x100) as i16) as i32;
        let result = instance
            .function("mem_test_i32i16")
            .unwrap()
            .invoke(&[a1.into(), a2.into()])
            .unwrap()
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, expected);

        let mut expected = Vec::new();
        expected.extend_from_slice(src);
        expected[a2 as usize] = expected[a1 as usize];
        expected[a2 as usize + 1] = expected[a1 as usize + 1];
        test_memory!(instance, &expected);

        // u64u16
        reset_memory(&instance, src);
        let expected = 0xFFFF ^ (a1b as u64 + (a1b as u64 + 1) * 0x100);
        let result = instance
            .function("mem_test_u64u16")
            .unwrap()
            .invoke(&[a1.into(), a2.into()])
            .unwrap()
            .unwrap()
            .get_u64()
            .unwrap();
        assert_eq!(result, expected);

        let mut expected = Vec::new();
        expected.extend_from_slice(src);
        expected[a2 as usize] = expected[a1 as usize];
        expected[a2 as usize + 1] = expected[a1 as usize + 1];
        test_memory!(instance, &expected);

        // i64i16
        reset_memory(&instance, src);
        let expected = -1 ^ ((a1b as u64 + (a1b as u64 + 1) * 0x100) as i16) as i64;
        let result = instance
            .function("mem_test_i64i16")
            .unwrap()
            .invoke(&[a1.into(), a2.into()])
            .unwrap()
            .unwrap()
            .get_i64()
            .unwrap();
        assert_eq!(result, expected);

        let mut expected = Vec::new();
        expected.extend_from_slice(src);
        expected[a2 as usize] = expected[a1 as usize];
        expected[a2 as usize + 1] = expected[a1 as usize + 1];
        test_memory!(instance, &expected);

        let a1 = a1 * 2;
        let a2 = a2 * 2;
        let a1b = a1 & 0xFF;

        // u32
        reset_memory(&instance, src);
        let expected = 0xFFFFFFFF ^ ((a1b * 0x1_01_01_01) + 0x03_02_01_00);
        let result = instance
            .function("mem_test_u32")
            .unwrap()
            .invoke(&[a1.into(), a2.into()])
            .unwrap()
            .unwrap()
            .get_u32()
            .unwrap();
        assert_eq!(result, expected);

        let mut expected = Vec::new();
        expected.extend_from_slice(src);
        for i in 0..4 {
            expected[a2 as usize + i] = expected[a1 as usize + i];
        }
        test_memory!(instance, &expected);

        // u64u32
        reset_memory(&instance, src);
        let expected = 0xFFFFFFFF ^ ((a1b * 0x1_01_01_01) + 0x03_02_01_00) as u64;
        let result = instance
            .function("mem_test_u64u32")
            .unwrap()
            .invoke(&[a1.into(), a2.into()])
            .unwrap()
            .unwrap()
            .get_u64()
            .unwrap();
        assert_eq!(result, expected);

        let mut expected = Vec::new();
        expected.extend_from_slice(src);
        for i in 0..4 {
            expected[a2 as usize + i] = expected[a1 as usize + i];
        }
        test_memory!(instance, &expected);

        // i64i32
        reset_memory(&instance, src);
        let expected = -1 ^ (((a1b * 0x1_01_01_01) + 0x03_02_01_00) as i32) as i64;
        let result = instance
            .function("mem_test_i64i32")
            .unwrap()
            .invoke(&[a1.into(), a2.into()])
            .unwrap()
            .unwrap()
            .get_i64()
            .unwrap();
        assert_eq!(result, expected);

        let mut expected = Vec::new();
        expected.extend_from_slice(src);
        for i in 0..4 {
            expected[a2 as usize + i] = expected[a1 as usize + i];
        }
        test_memory!(instance, &expected);

        let a1 = a1 * 2;
        let a2 = a2 * 2;
        let a1b = a1 & 0xFF;

        // u64
        reset_memory(&instance, src);
        let expected = 0xFFFF_FFFF_FFFF_FFFF
            ^ ((a1b as u64 * 0x1_01_01_01_01_01_01_01) + 0x07_06_05_04_03_02_01_00);
        let result = instance
            .function("mem_test_u64")
            .unwrap()
            .invoke(&[a1.into(), a2.into()])
            .unwrap()
            .unwrap()
            .get_u64()
            .unwrap();
        assert_eq!(result, expected);

        let mut expected = Vec::new();
        expected.extend_from_slice(src);
        for i in 0..8 {
            expected[a2 as usize + i] = expected[a1 as usize + i];
        }
        test_memory!(instance, &expected);
    }
}

#[test]
fn memory() {
    let instance =
        WebAssembly::instantiate(include_bytes!("../test/tester.wasm"), &Env {}).unwrap();

    let mut src = Vec::new();
    for i in 0..256 {
        src.push(0xFF ^ i as u8);
    }
    let src = src.as_slice();

    #[inline]
    #[track_caller]
    fn reset_memory(instance: &WasmInstance, src: &[u8]) {
        instance
            .memory(0)
            .unwrap()
            .borrowing(|memory| {
                memory[0..256].copy_from_slice(src);
            })
            .unwrap();
    }

    macro_rules! test_memory {
        ($instance:ident, $expected:expr) => {
            $instance
                .memory(0)
                .unwrap()
                .borrowing(|memory| {
                    let mut vec = Vec::new();
                    for index in 0..256 {
                        let lhs = memory[index];
                        let rhs = $expected[index];
                        if lhs != rhs {
                            vec.push((index, lhs, rhs));
                        }
                    }
                    if vec.len() > 0 {
                        panic!("MEMORY ERROR (index, actual, expected): {:?}", vec);
                    }
                })
                .unwrap();
        };
    }

    #[inline]
    #[track_caller]
    fn memset(memory: &mut [u8], d: i32, v: u8, n: i32) {
        let d = d as usize;
        let n = n as usize;
        for i in 0..n {
            memory[d + i] = v;
        }
    }

    #[inline]
    #[track_caller]
    fn memcpy(memory: &mut [u8], d: i32, s: i32, n: i32) {
        let d = d as usize;
        let s = s as usize;
        let n = n as usize;
        for i in 0..n {
            let v = memory[s + i];
            memory[d + i] = v;
        }
    }

    let mem_size = instance
        .function("mem_test_size")
        .unwrap()
        .invoke(&[])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(mem_size, 1);

    let mem_size = instance
        .function("mem_test_grow")
        .unwrap()
        .invoke(&[0.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(mem_size, 1);

    let mem_size = instance
        .function("mem_test_size")
        .unwrap()
        .invoke(&[])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(mem_size, 1);

    let mem_size = instance
        .function("mem_test_grow")
        .unwrap()
        .invoke(&[10.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(mem_size, 1);

    let mem_size = instance
        .function("mem_test_size")
        .unwrap()
        .invoke(&[])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(mem_size, 11);

    let mem_size = instance
        .function("mem_test_grow")
        .unwrap()
        .invoke(&[0x1_0000.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(mem_size, -1);

    let mem_size = instance
        .function("mem_test_size")
        .unwrap()
        .invoke(&[])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(mem_size, 11);

    // memmory fill
    let (p_dest, p_src, count) = (12, 34, 5);
    reset_memory(&instance, src);

    assert!(instance
        .function("mem_test_fill")
        .unwrap()
        .invoke(&[p_dest.into(), p_src.into(), count.into()])
        .unwrap()
        .is_none());

    let mut expected = Vec::new();
    expected.extend_from_slice(src);
    memset(&mut expected, p_dest, p_src as u8, count);
    test_memory!(instance, &expected);

    // memmory copy
    reset_memory(&instance, src);

    assert!(instance
        .function("mem_test_copy")
        .unwrap()
        .invoke(&[p_dest.into(), p_src.into(), count.into()])
        .unwrap()
        .is_none());

    let mut expected = Vec::new();
    expected.extend_from_slice(src);
    memcpy(&mut expected, p_dest, p_src, count);
    test_memory!(instance, &expected);
}

#[test]
fn global() {
    let instance =
        WebAssembly::instantiate(include_bytes!("../test/tester.wasm"), &Env {}).unwrap();
    let runnable = instance.function("global_add").unwrap();

    assert_eq!(
        instance
            .global("global1")
            .unwrap()
            .value()
            .get_i32()
            .unwrap(),
        123
    );

    let result = runnable
        .invoke(&[456.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 579);

    assert_eq!(
        instance
            .global("global1")
            .unwrap()
            .value()
            .get_i32()
            .unwrap(),
        579
    );

    let result = runnable
        .invoke(&[789.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1368);

    assert_eq!(
        instance
            .global("global1")
            .unwrap()
            .value()
            .get_i32()
            .unwrap(),
        1368
    );
}

#[test]
fn name() {
    let slice = [
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x00, 0x1F, 0x04, 0x6E,
        0x61, 0x6D, 0x65, 0x00, 0x06, 0x05, 0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x01, 0x0E, 0x02, 0x01,
        0x04, 0x77, 0x61, 0x73, 0x6D, 0xB4, 0x24, 0x04, 0x74, 0x65, 0x73, 0x74, 0x7F, 0x00,
    ];
    let instance = WebAssembly::instantiate(&slice, &Env {}).unwrap();
    let names = instance.module().names().unwrap();

    assert_eq!(names.module().unwrap(), "Hello");

    assert_eq!(names.functions()[0], (1, "wasm".to_owned()));

    assert_eq!(names.func_by_index(0x1234).unwrap(), "test");
}

macro_rules! assert_sign_eq {
    ($lhs:expr, $rhs:expr) => {
        assert_eq!($lhs.is_sign_positive(), $rhs.is_sign_positive())
    };
}

macro_rules! assert_sign_ne {
    ($lhs:expr, $rhs:expr) => {
        assert_eq!($lhs.is_sign_positive(), $rhs.is_sign_negative())
    };
}

#[test]
fn float32() {
    let slice = [0, 0x43, 0xdb, 0x0f, 0x49, 0x40, 0x0B];
    let param_types = [];
    let result_types = [WasmValType::F32];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info = WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &param_types,
        &result_types,
        instance.module(),
    )
    .unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let mut locals = [];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_f32()
        .unwrap();
    assert_eq!(result, 3.1415927);

    let slice = [0, 0x20, 0x00, 0xbe, 0x0B];
    let param_types = [WasmValType::I32];
    let result_types = [WasmValType::F32];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info = WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &param_types,
        &result_types,
        instance.module(),
    )
    .unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let mut locals = [0x40490fdb.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_f32()
        .unwrap();
    assert_eq!(result, 3.1415927);

    let slice = [0, 0x20, 0x00, 0xbc, 0x0B];
    let param_types = [WasmValType::F32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info = WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &param_types,
        &result_types,
        instance.module(),
    )
    .unwrap();
    let mut interp = WasmInterpreter::new(&instance);

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
fn float32_opr() {
    let instance =
        WebAssembly::instantiate(include_bytes!("../test/tester.wasm"), &Env {}).unwrap();

    let memory = instance.memory(0).unwrap().try_borrow().unwrap();

    const SIGN_BITS: u32 = 0x8000_0000;
    const ZERO_BITS: u32 = 0;
    const NEG_ZERO_BITS: u32 = SIGN_BITS;

    for val in [
        0.0f64,
        -0.0f64,
        0.24,
        0.5,
        0.75,
        1.0,
        1.25,
        1.5,
        1.75,
        2.0,
        2.5,
        3.0,
        3.5,
        4.0,
        -0.25,
        -0.5,
        -0.75,
        -1.0,
        -1.25,
        -1.5,
        -1.75,
        -2.0,
        -2.5,
        -3.0,
        -3.5,
        -4.0,
        1.1920929E-7,
        1.17549435E-38,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        PI,
        2_147_483_647.0,
        2_147_483_648.0,
        -2_147_483_647.0,
        -2_147_483_648.0,
        -2_147_483_649.0,
        4_294_967_295.0,
        4_294_967_296.0,
        -3_000_000_000.0,
        5_000_000_000.0,
    ] {
        let fval = val as f32;
        let i32val = fval as i32;
        let u32val = fval as u32;
        let i64val = fval as i64;
        let u64val = fval as u64;

        memory.fill(0xCC);
        let result = instance
            .function("test_unary_f32")
            .unwrap()
            .invoke(&[
                fval.into(),
                i32val.into(),
                u32val.into(),
                i64val.into(),
                u64val.into(),
            ])
            .unwrap()
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 0x98);

        assert_eq!(memory.read_u64(0), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8), 0xCCCC_CCCC_CCCC_CCCC);

        if val.is_nan() {
        } else if val <= i32::MIN as f64 {
            assert_eq!(memory.read_i32(0x18), i32::MIN);
        } else if val >= i32::MAX as f64 {
            assert_eq!(memory.read_i32(0x18), i32::MAX);
        } else {
            assert_eq!(memory.read_i32(0x10), i32val);
            assert_eq!(memory.read_i32(0x18), i32val);
        }

        if val.is_nan() {
        } else if val <= 0.0 {
            assert_eq!(memory.read_u32(0x1C), 0);
        } else if val >= u32::MAX as f64 {
            assert_eq!(memory.read_u32(0x1C), u32::MAX);
        } else {
            assert_eq!(memory.read_u32(0x14), u32val);
            assert_eq!(memory.read_u32(0x1C), u32val);
        }

        if val.is_nan() {
        } else if val <= i64::MIN as f64 {
            assert_eq!(memory.read_i64(0x30), i64::MIN);
        } else if val >= i64::MAX as f64 {
            assert_eq!(memory.read_i64(0x30), i64::MAX);
        } else {
            assert_eq!(memory.read_i64(0x20), i64val);
            assert_eq!(memory.read_i64(0x30), i64val);
        }

        if val.is_nan() {
        } else if val <= 0.0 {
            assert_eq!(memory.read_u64(0x38), 0);
        } else if val >= u64::MAX as f64 {
            assert_eq!(memory.read_u64(0x38), u64::MAX);
        } else {
            assert_eq!(memory.read_u64(0x28), u64val);
            assert_eq!(memory.read_u64(0x38), u64val);
        }

        assert_eq!(memory.read_f32(0x40), i32val as f32);
        assert_eq!(memory.read_f32(0x44), u32val as f32);
        assert_eq!(memory.read_f32(0x48), i64val as f32);
        assert_eq!(memory.read_f32(0x4C), u64val as f32);

        if fval.is_nan() {
            assert!(memory.read_f64(0x50).is_nan());
        } else {
            assert_eq!(memory.read_f64(0x50), fval as f64);
        }

        // fabs
        let test = memory.read_f32(0x80);
        assert!(test.is_sign_positive());
        if fval.is_nan() {
            assert!(test.is_nan());
        } else if fval.is_infinite() {
            assert_eq!(test, f32::INFINITY);
        } else {
            assert_eq!(test, fval.abs());
        }

        // fneg
        let test = memory.read_f32(0x84);
        assert_sign_ne!(test, fval);
        if fval.is_nan() {
            assert!(test.is_nan());
        } else if fval.is_infinite() {
            assert!(test.is_infinite());
        } else {
            assert_eq!(test, 0.0 - fval);
        }

        // fceil
        let test = memory.read_f32(0x88);
        if fval.is_nan() {
            assert!(test.is_nan());
        } else if fval.is_infinite() {
            assert!(test.is_infinite());
        } else if fval.is_zero() {
            assert_eq!(test, fval);
        } else if fval > 0.0 && fval <= 1.0 {
            assert_eq!(test, 1.0);
        } else if fval > 1.0 && fval <= 2.0 {
            assert_eq!(test, 2.0);
        } else if fval > 2.0 && fval <= 3.0 {
            assert_eq!(test, 3.0);
        } else if fval > 3.0 && fval <= 4.0 {
            assert_eq!(test, 4.0);
        } else if fval > -1.0 && fval < 0.0 {
            assert_eq!(test, 0.0);
        } else if fval > -2.0 && fval <= -1.0 {
            assert_eq!(test, -1.0);
        } else if fval > -3.0 && fval <= -2.0 {
            assert_eq!(test, -2.0);
        } else if fval > -4.0 && fval <= -3.0 {
            assert_eq!(test, -3.0);
        } else {
            assert_eq!(test, fval.ceil());
        }

        // ffloor
        let test = memory.read_f32(0x8C);
        if fval.is_nan() {
            assert!(test.is_nan());
        } else if fval.is_infinite() {
            assert!(test.is_infinite());
        } else if fval.is_zero() {
            assert_eq!(test, fval);
        } else if fval > 0.0 && fval < 1.0 {
            assert_eq!(test, 0.0);
        } else if fval >= 1.0 && fval < 2.0 {
            assert_eq!(test, 1.0);
        } else if fval >= 2.0 && fval < 3.0 {
            assert_eq!(test, 2.0);
        } else if fval >= 3.0 && fval < 4.0 {
            assert_eq!(test, 3.0);
        } else if fval < 0.0 && fval > -1.0 {
            assert_eq!(test, -1.0);
        } else if fval < -1.0 && fval >= -2.0 {
            assert_eq!(test, -2.0);
        } else if fval < -2.0 && fval >= -3.0 {
            assert_eq!(test, -3.0);
        } else if fval < -3.0 && fval >= -4.0 {
            assert_eq!(test, -4.0);
        } else {
            assert_eq!(test, fval.floor());
        }

        // ftrunc
        let test = memory.read_f32(0x90);
        if fval.is_nan() {
            assert!(test.is_nan());
        } else if fval.is_infinite() {
            assert!(test.is_infinite());
        } else if fval.is_zero() {
            assert_eq!(test, fval);
        } else if fval > 0.0 && fval < 1.0 {
            assert_eq!(test, 0.0);
        } else if fval >= 1.0 && fval < 2.0 {
            assert_eq!(test, 1.0);
        } else if fval >= 2.0 && fval < 3.0 {
            assert_eq!(test, 2.0);
        } else if fval >= 3.0 && fval < 4.0 {
            assert_eq!(test, 3.0);
        } else if fval < 0.0 && fval > -1.0 {
            assert_eq!(test, -0.0);
        } else if fval <= -1.0 && fval > -2.0 {
            assert_eq!(test, -1.0);
        } else if fval <= -2.0 && fval > -3.0 {
            assert_eq!(test, -2.0);
        } else if fval <= -3.0 && fval > -4.0 {
            assert_eq!(test, -3.0);
        } else {
            assert_eq!(test, fval.trunc());
        }

        // fnearest
        let test = memory.read_f32(0x94);
        if fval.is_nan() {
            assert!(test.is_nan());
        } else if fval.is_infinite() {
            assert!(test.is_infinite());
        } else if fval.is_zero() {
            assert_eq!(test, fval);
        } else if fval > 0.0 && fval <= 0.5 {
            assert_eq!(test, 0.0);
        } else if fval > 0.5 && fval < 1.5 {
            assert_eq!(test, 1.0);
        } else if fval >= 1.5 && fval <= 2.5 {
            assert_eq!(test, 2.0);
        } else if fval > 2.5 && fval < 3.5 {
            assert_eq!(test, 3.0);
        } else if fval >= 3.5 && fval <= 4.5 {
            assert_eq!(test, 4.0);
        } else if fval < 0.0 && fval >= -0.5 {
            assert_eq!(test, -0.0);
        } else if fval < -0.5 && fval > -1.5 {
            assert_eq!(test, -1.0);
        } else if fval <= -1.5 && fval >= -2.5 {
            assert_eq!(test, -2.0);
        } else if fval < -2.5 && fval > -3.5 {
            assert_eq!(test, -3.0);
        } else if fval <= -3.5 && fval >= -4.5 {
            assert_eq!(test, -4.0);
        } else {
            // TODO: not stable
            // assert_eq!(test, fval.round_ties_even());
        }

        // fsqrt
        let test = memory.read_f32(0x98);
        if fval.is_nan() {
            assert!(test.is_nan());
        } else if fval.is_zero() {
            assert_eq!(test.to_bits(), fval.to_bits());
        } else if fval.is_sign_negative() {
            assert!(test.is_nan());
        } else if fval.is_infinite() {
            assert!(test.is_sign_positive());
            assert!(test.is_infinite());
        } else {
            assert!(test.is_sign_positive());
            assert_eq!(test, fval.sqrt());
        }

        assert_eq!(memory.read_u32(0x9C), 0xCCCC_CCCC);
        assert_eq!(memory.read_u64(0xA0), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(0xA8), 0xCCCC_CCCC_CCCC_CCCC);
    }

    let values = [
        0.0,
        -0.0,
        0.5,
        1.0,
        1.5,
        2.0,
        -0.5,
        -1.0,
        -1.5,
        -2.0,
        1.1920929E-7,
        1.17549435E-38,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        core::f32::consts::PI,
    ];

    for lhs in &values {
        for rhs in &values {
            let lhs = *lhs;
            let rhs = *rhs;

            memory.fill(0xCC);
            let result = instance
                .function("test_bin_f32")
                .unwrap()
                .invoke(&[lhs.into(), rhs.into()])
                .unwrap()
                .unwrap()
                .get_i32()
                .unwrap();
            assert_eq!(result, 0x40);

            assert_eq!(memory.read_u64(0), 0xCCCC_CCCC_CCCC_CCCC);
            assert_eq!(memory.read_u64(8), 0xCCCC_CCCC_CCCC_CCCC);

            // feq
            let test = memory.read_u32(0x10);
            if lhs.is_nan() || rhs.is_nan() {
                assert_eq!(test, 0);
            } else if lhs.is_zero() && rhs.is_zero() {
                assert_eq!(test, 1);
            } else {
                assert_eq!(test, (lhs == rhs) as u32);
            }

            // fne
            let test = memory.read_u32(0x14);
            if lhs.is_nan() || rhs.is_nan() {
                assert_eq!(test, 1);
            } else if lhs.is_zero() && rhs.is_zero() {
                assert_eq!(test, 0);
            } else {
                assert_eq!(test, (lhs != rhs) as u32);
            }

            // flt
            let test = memory.read_u32(0x18);
            if lhs.is_nan() || rhs.is_nan() {
                assert_eq!(test, 0);
            } else if lhs == rhs {
                assert_eq!(test, 0);
            } else if lhs == f32::INFINITY {
                assert_eq!(test, 0);
            } else if lhs == f32::NEG_INFINITY {
                assert_eq!(test, 1);
            } else if rhs == f32::INFINITY {
                assert_eq!(test, 1);
            } else if rhs == f32::NEG_INFINITY {
                assert_eq!(test, 0);
            } else if lhs.is_zero() && rhs.is_zero() {
                assert_eq!(test, 0);
            } else {
                assert_eq!(test, (lhs < rhs) as u32);
            }

            // fgt
            let test = memory.read_u32(0x1C);
            if lhs.is_nan() || rhs.is_nan() {
                assert_eq!(test, 0);
            } else if lhs == rhs {
                assert_eq!(test, 0);
            } else if lhs == f32::INFINITY {
                assert_eq!(test, 1);
            } else if lhs == f32::NEG_INFINITY {
                assert_eq!(test, 0);
            } else if rhs == f32::INFINITY {
                assert_eq!(test, 0);
            } else if rhs == f32::NEG_INFINITY {
                assert_eq!(test, 1);
            } else if lhs.is_zero() && rhs.is_zero() {
                assert_eq!(test, 0);
            } else {
                assert_eq!(test, (lhs > rhs) as u32);
            }

            // fle
            let test = memory.read_u32(0x20);
            if lhs.is_nan() || rhs.is_nan() {
                assert_eq!(test, 0);
            } else if lhs == rhs {
                assert_eq!(test, 1);
            } else if lhs == f32::INFINITY {
                assert_eq!(test, 0);
            } else if lhs == f32::NEG_INFINITY {
                assert_eq!(test, 1);
            } else if rhs == f32::INFINITY {
                assert_eq!(test, 1);
            } else if rhs == f32::NEG_INFINITY {
                assert_eq!(test, 0);
            } else if lhs.is_zero() && rhs.is_zero() {
                assert_eq!(test, 1);
            } else {
                assert_eq!(test, (lhs <= rhs) as u32);
            }

            // fgt
            let test = memory.read_u32(0x24);
            if lhs.is_nan() || rhs.is_nan() {
                assert_eq!(test, 0);
            } else if lhs == rhs {
                assert_eq!(test, 1);
            } else if lhs == f32::INFINITY {
                assert_eq!(test, 1);
            } else if lhs == f32::NEG_INFINITY {
                assert_eq!(test, 0);
            } else if rhs == f32::INFINITY {
                assert_eq!(test, 0);
            } else if rhs == f32::NEG_INFINITY {
                assert_eq!(test, 1);
            } else if lhs.is_zero() && rhs.is_zero() {
                assert_eq!(test, 1);
            } else {
                assert_eq!(test, (lhs >= rhs) as u32);
            }

            // fadd
            let test = memory.read_f32(0x28);
            if lhs.is_nan() || rhs.is_nan() {
                assert!(test.is_nan());
            } else if lhs == f32::INFINITY && rhs == f32::INFINITY {
                assert_eq!(test, f32::INFINITY);
            } else if lhs == f32::INFINITY && rhs == f32::NEG_INFINITY {
                assert!(test.is_nan());
            } else if lhs == f32::NEG_INFINITY && rhs == f32::INFINITY {
                assert!(test.is_nan());
            } else if lhs == f32::NEG_INFINITY && rhs == f32::NEG_INFINITY {
                assert_eq!(test, f32::NEG_INFINITY);
            } else if rhs.is_infinite() {
                assert_eq!(test, rhs);
            } else if lhs.is_infinite() {
                assert_eq!(test, lhs);
            } else if lhs.is_zero() && rhs.is_zero() {
                if lhs.is_sign_positive() == rhs.is_sign_positive() {
                    assert_sign_eq!(test, lhs);
                } else {
                    assert!(test.is_sign_positive());
                }
                assert!(test.is_zero());
            } else if rhs.is_zero() {
                assert_eq!(test, lhs);
            } else if lhs.is_zero() {
                assert_eq!(test, rhs);
            } else if lhs.abs() == rhs.abs() && lhs.is_sign_positive() == rhs.is_sign_negative() {
                assert_eq!(test.to_bits(), ZERO_BITS);
            } else {
                assert_eq!(test, lhs + rhs);
            }

            // fsub
            let test = memory.read_f32(0x2C);
            if lhs.is_nan() || rhs.is_nan() {
                assert!(test.is_nan());
            } else if lhs == f32::INFINITY && rhs == f32::INFINITY {
                assert!(test.is_nan());
            } else if lhs == f32::INFINITY && rhs == f32::NEG_INFINITY {
                assert_eq!(test, f32::INFINITY);
            } else if lhs == f32::NEG_INFINITY && rhs == f32::INFINITY {
                assert_eq!(test, f32::NEG_INFINITY);
            } else if lhs == f32::NEG_INFINITY && rhs == f32::NEG_INFINITY {
                assert!(test.is_nan());
            } else if lhs == f32::INFINITY {
                assert_eq!(test, f32::INFINITY);
            } else if lhs == f32::NEG_INFINITY {
                assert_eq!(test, f32::NEG_INFINITY);
            } else if rhs == f32::INFINITY {
                assert_eq!(test, f32::NEG_INFINITY);
            } else if rhs == f32::NEG_INFINITY {
                assert_eq!(test, f32::INFINITY);
            } else if lhs.is_zero() && rhs.is_zero() {
                if lhs.is_sign_positive() == rhs.is_sign_positive() {
                    assert!(test.is_sign_positive());
                } else {
                    assert_sign_eq!(test, lhs);
                }
                assert!(test.is_zero());
            } else if rhs.is_zero() {
                assert_eq!(test, lhs);
            } else if lhs.is_zero() {
                assert_sign_ne!(test, rhs);
                assert_eq!(test.abs(), rhs.abs());
            } else if lhs.abs() == rhs.abs() && lhs.is_sign_positive() == rhs.is_sign_positive() {
                assert_eq!(test.to_bits(), ZERO_BITS);
            } else {
                assert_eq!(test, lhs - rhs);
            }

            // fmul
            let test = memory.read_f32(0x30);
            if lhs.is_nan() || rhs.is_nan() {
                assert!(test.is_nan());
            } else if lhs.is_zero() && rhs.is_infinite() || lhs.is_infinite() && rhs.is_zero() {
                assert!(test.is_nan());
            } else if lhs == f32::INFINITY && rhs == f32::INFINITY {
                assert_eq!(test, f32::INFINITY);
            } else if lhs == f32::INFINITY && rhs == f32::NEG_INFINITY {
                assert_eq!(test, f32::NEG_INFINITY);
            } else if lhs == f32::NEG_INFINITY && rhs == f32::INFINITY {
                assert_eq!(test, f32::NEG_INFINITY);
            } else if lhs == f32::NEG_INFINITY && rhs == f32::NEG_INFINITY {
                assert_eq!(test, f32::INFINITY);
            } else if lhs.is_infinite() && lhs.is_sign_positive() == rhs.is_sign_positive()
                || rhs.is_infinite() && lhs.is_sign_positive() == rhs.is_sign_positive()
            {
                assert_eq!(test, f32::INFINITY);
            } else if lhs.is_infinite() && lhs.is_sign_positive() == rhs.is_sign_negative()
                || rhs.is_infinite() && lhs.is_sign_positive() == rhs.is_sign_negative()
            {
                assert_eq!(test, f32::NEG_INFINITY);
            } else if lhs.is_zero() && rhs.is_zero() {
                if lhs.is_sign_positive() == rhs.is_sign_positive() {
                    assert!(test.is_sign_positive());
                } else {
                    assert!(test.is_sign_negative());
                }
                assert!(test.is_zero());
            } else {
                assert_eq!(test, lhs * rhs);
            }

            // fdiv
            let test = memory.read_f32(0x34);
            if lhs.is_nan() || rhs.is_nan() {
                assert!(test.is_nan());
            } else if lhs.is_infinite() && rhs.is_infinite() {
                assert!(test.is_nan());
            } else if lhs.is_zero() && rhs.is_zero() {
                assert!(test.is_nan());
            } else if lhs == f32::INFINITY && lhs.is_sign_positive() == rhs.is_sign_positive() {
                assert_eq!(test, f32::INFINITY);
            } else if lhs == f32::INFINITY && lhs.is_sign_positive() == rhs.is_sign_negative() {
                assert_eq!(test, f32::NEG_INFINITY);
            } else if rhs == f32::INFINITY && lhs.is_sign_positive() == rhs.is_sign_positive() {
                assert_eq!(test.to_bits(), ZERO_BITS);
            } else if rhs == f32::INFINITY && lhs.is_sign_positive() == rhs.is_sign_negative() {
                assert_eq!(test.to_bits(), NEG_ZERO_BITS);
            } else if lhs.is_zero() && lhs.is_sign_positive() == rhs.is_sign_positive() {
                assert_eq!(test.to_bits(), ZERO_BITS);
            } else if lhs.is_zero() && lhs.is_sign_positive() == rhs.is_sign_negative() {
                assert_eq!(test.to_bits(), NEG_ZERO_BITS);
            } else if rhs.is_zero() && lhs.is_sign_positive() == rhs.is_sign_positive() {
                assert_eq!(test, f32::INFINITY);
            } else if rhs.is_zero() && lhs.is_sign_positive() == rhs.is_sign_negative() {
                assert_eq!(test, f32::NEG_INFINITY);
            } else {
                assert_eq!(test, lhs / rhs);
            }

            // fcopysign
            let test = memory.read_f32(0x38);
            if lhs.is_sign_positive() == rhs.is_sign_positive() {
                assert_eq!(test.to_bits(), lhs.to_bits());
            } else {
                assert_eq!(test.to_bits(), lhs.to_bits() ^ SIGN_BITS);
            }

            // fmin
            let test = memory.read_f32(0x3C);
            if lhs.is_nan() || rhs.is_nan() {
                assert!(test.is_nan());
            } else if lhs == f32::NEG_INFINITY || rhs == f32::NEG_INFINITY {
                assert_eq!(test, f32::NEG_INFINITY);
            } else if lhs == f32::INFINITY {
                assert_eq!(test.to_bits(), rhs.to_bits());
            } else if rhs == f32::INFINITY {
                assert_eq!(test.to_bits(), lhs.to_bits());
            } else if lhs.is_zero()
                && rhs.is_zero()
                && lhs.is_sign_positive() == rhs.is_sign_negative()
            {
                assert_eq!(test.to_bits(), NEG_ZERO_BITS);
            } else if lhs < rhs {
                assert_eq!(test, lhs);
            } else {
                assert_eq!(test, rhs);
            }

            // fmax
            let test = memory.read_f32(0x40);
            if lhs.is_nan() || rhs.is_nan() {
                assert!(test.is_nan());
            } else if lhs == f32::INFINITY || rhs == f32::INFINITY {
                assert_eq!(test, f32::INFINITY);
            } else if lhs == f32::NEG_INFINITY {
                assert_eq!(test.to_bits(), rhs.to_bits());
            } else if rhs == f32::NEG_INFINITY {
                assert_eq!(test.to_bits(), lhs.to_bits());
            } else if lhs.is_zero()
                && rhs.is_zero()
                && lhs.is_sign_positive() == rhs.is_sign_negative()
            {
                assert_eq!(test.to_bits(), ZERO_BITS);
            } else if lhs > rhs {
                assert_eq!(test, lhs);
            } else {
                assert_eq!(test, rhs);
            }

            assert_eq!(memory.read_u32(0x44), 0xCCCC_CCCC);
            assert_eq!(memory.read_u64(0x48), 0xCCCC_CCCC_CCCC_CCCC);
            assert_eq!(memory.read_u64(0x50), 0xCCCC_CCCC_CCCC_CCCC);
        }
    }
}

#[test]
fn float64() {
    let slice = [
        0, 0x44, 0x18, 0x2d, 0x44, 0x54, 0xfb, 0x21, 0x09, 0x40, 0x0B,
    ];
    let param_types = [];
    let result_types = [WasmValType::F64];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info = WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &param_types,
        &result_types,
        instance.module(),
    )
    .unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let mut locals = [];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_f64()
        .unwrap();
    assert_eq!(result, PI);

    let slice = [0, 0x20, 0x00, 0xbf, 0x0B];
    let param_types = [WasmValType::I64];
    let result_types = [WasmValType::F64];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info = WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &param_types,
        &result_types,
        instance.module(),
    )
    .unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let mut locals = [0x400921fb54442d18u64.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_f64()
        .unwrap();
    assert_eq!(result, PI);

    let slice = [0, 0x20, 0x00, 0xbd, 0x0B];
    let param_types = [WasmValType::F64];
    let result_types = [WasmValType::I64];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    let info = WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &param_types,
        &result_types,
        instance.module(),
    )
    .unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let mut locals = [PI.into()];
    let result = interp
        .invoke(0, &info, &mut locals, &result_types)
        .unwrap()
        .unwrap()
        .get_u64()
        .unwrap();
    assert_eq!(result, 0x400921fb54442d18u64);
}

#[test]
fn float64_opr() {
    let instance =
        WebAssembly::instantiate(include_bytes!("../test/tester.wasm"), &Env {}).unwrap();

    let memory = instance.memory(0).unwrap().try_borrow().unwrap();

    const SIGN_BITS: u64 = 0x8000_0000_0000_0000;
    const ZERO_BITS: u64 = 0;
    const NEG_ZERO_BITS: u64 = SIGN_BITS;

    for val in [
        0.0f64,
        -0.0f64,
        0.24,
        0.5,
        0.75,
        1.0,
        1.25,
        1.5,
        1.75,
        2.0,
        2.5,
        3.0,
        3.5,
        4.0,
        -0.25,
        -0.5,
        -0.75,
        -1.0,
        -1.25,
        -1.5,
        -1.75,
        -2.0,
        -2.5,
        -3.0,
        -3.5,
        -4.0,
        1.1920929E-7,
        1.17549435E-38,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        PI,
        2_147_483_647.0,
        2_147_483_648.0,
        -2_147_483_647.0,
        -2_147_483_648.0,
        -2_147_483_649.0,
        4_294_967_295.0,
        4_294_967_296.0,
        -3_000_000_000.0,
        5_000_000_000.0,
    ] {
        let fval = val;
        let i32val = fval as i32;
        let u32val = fval as u32;
        let i64val = fval as i64;
        let u64val = fval as u64;

        memory.fill(0xCC);
        let result = instance
            .function("test_unary_f64")
            .unwrap()
            .invoke(&[
                fval.into(),
                i32val.into(),
                u32val.into(),
                i64val.into(),
                u64val.into(),
            ])
            .unwrap()
            .unwrap()
            .get_i32()
            .unwrap();
        assert_eq!(result, 0xB0);

        assert_eq!(memory.read_u64(0), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(8), 0xCCCC_CCCC_CCCC_CCCC);

        if val.is_nan() {
        } else if val <= i32::MIN as f64 {
            assert_eq!(memory.read_i32(0x18), i32::MIN);
        } else if val >= i32::MAX as f64 {
            assert_eq!(memory.read_i32(0x18), i32::MAX);
        } else {
            assert_eq!(memory.read_i32(0x10), i32val);
            assert_eq!(memory.read_i32(0x18), i32val);
        }

        if val.is_nan() {
        } else if val <= 0.0 {
            assert_eq!(memory.read_u32(0x1C), 0);
        } else if val >= u32::MAX as f64 {
            assert_eq!(memory.read_u32(0x1C), u32::MAX);
        } else {
            assert_eq!(memory.read_u32(0x14), u32val);
            assert_eq!(memory.read_u32(0x1C), u32val);
        }

        if val.is_nan() {
        } else if val <= i64::MIN as f64 {
            assert_eq!(memory.read_i64(0x30), i64::MIN);
        } else if val >= i64::MAX as f64 {
            assert_eq!(memory.read_i64(0x30), i64::MAX);
        } else {
            assert_eq!(memory.read_i64(0x20), i64val);
            assert_eq!(memory.read_i64(0x30), i64val);
        }

        if val.is_nan() {
        } else if val <= 0.0 {
            assert_eq!(memory.read_u64(0x38), 0);
        } else if val >= u64::MAX as f64 {
            assert_eq!(memory.read_u64(0x38), u64::MAX);
        } else {
            assert_eq!(memory.read_u64(0x28), u64val);
            assert_eq!(memory.read_u64(0x38), u64val);
        }

        assert_eq!(memory.read_f64(0x40), i32val as f64);
        assert_eq!(memory.read_f64(0x48), u32val as f64);
        assert_eq!(memory.read_f64(0x50), i64val as f64);
        assert_eq!(memory.read_f64(0x58), u64val as f64);

        if fval.is_nan() {
            assert!(memory.read_f32(0x60).is_nan());
        } else {
            assert_eq!(memory.read_f32(0x60), fval as f32);
        }

        // fabs
        let test = memory.read_f64(0x80);
        assert!(test.is_sign_positive());
        if fval.is_nan() {
            assert!(test.is_nan());
        } else if fval.is_infinite() {
            assert_eq!(test, f64::INFINITY);
        } else {
            assert_eq!(test, fval.abs());
        }

        // fneg
        let test = memory.read_f64(0x88);
        assert_sign_ne!(test, fval);
        if fval.is_nan() {
            assert!(test.is_nan());
        } else if fval.is_infinite() {
            assert!(test.is_infinite());
        } else {
            assert_eq!(test, 0.0 - fval);
        }

        // fceil
        let test = memory.read_f64(0x90);
        if fval.is_nan() {
            assert!(test.is_nan());
        } else if fval.is_infinite() {
            assert!(test.is_infinite());
        } else if fval.is_zero() {
            assert_eq!(test, fval);
        } else if fval > 0.0 && fval <= 1.0 {
            assert_eq!(test, 1.0);
        } else if fval > 1.0 && fval <= 2.0 {
            assert_eq!(test, 2.0);
        } else if fval > 2.0 && fval <= 3.0 {
            assert_eq!(test, 3.0);
        } else if fval > 3.0 && fval <= 4.0 {
            assert_eq!(test, 4.0);
        } else if fval > -1.0 && fval < 0.0 {
            assert_eq!(test, 0.0);
        } else if fval > -2.0 && fval <= -1.0 {
            assert_eq!(test, -1.0);
        } else if fval > -3.0 && fval <= -2.0 {
            assert_eq!(test, -2.0);
        } else if fval > -4.0 && fval <= -3.0 {
            assert_eq!(test, -3.0);
        } else {
            assert_eq!(test, fval.ceil());
        }

        // ffloor
        let test = memory.read_f64(0x98);
        if fval.is_nan() {
            assert!(test.is_nan());
        } else if fval.is_infinite() {
            assert!(test.is_infinite());
        } else if fval.is_zero() {
            assert_eq!(test, fval);
        } else if fval > 0.0 && fval < 1.0 {
            assert_eq!(test, 0.0);
        } else if fval >= 1.0 && fval < 2.0 {
            assert_eq!(test, 1.0);
        } else if fval >= 2.0 && fval < 3.0 {
            assert_eq!(test, 2.0);
        } else if fval >= 3.0 && fval < 4.0 {
            assert_eq!(test, 3.0);
        } else if fval < 0.0 && fval > -1.0 {
            assert_eq!(test, -1.0);
        } else if fval < -1.0 && fval >= -2.0 {
            assert_eq!(test, -2.0);
        } else if fval < -2.0 && fval >= -3.0 {
            assert_eq!(test, -3.0);
        } else if fval < -3.0 && fval >= -4.0 {
            assert_eq!(test, -4.0);
        } else {
            assert_eq!(test, fval.floor());
        }

        // ftrunc
        let test = memory.read_f64(0xA0);
        if fval.is_nan() {
            assert!(test.is_nan());
        } else if fval.is_infinite() {
            assert!(test.is_infinite());
        } else if fval.is_zero() {
            assert_eq!(test, fval);
        } else if fval > 0.0 && fval < 1.0 {
            assert_eq!(test, 0.0);
        } else if fval >= 1.0 && fval < 2.0 {
            assert_eq!(test, 1.0);
        } else if fval >= 2.0 && fval < 3.0 {
            assert_eq!(test, 2.0);
        } else if fval >= 3.0 && fval < 4.0 {
            assert_eq!(test, 3.0);
        } else if fval < 0.0 && fval > -1.0 {
            assert_eq!(test, -0.0);
        } else if fval <= -1.0 && fval > -2.0 {
            assert_eq!(test, -1.0);
        } else if fval <= -2.0 && fval > -3.0 {
            assert_eq!(test, -2.0);
        } else if fval <= -3.0 && fval > -4.0 {
            assert_eq!(test, -3.0);
        } else {
            assert_eq!(test, fval.trunc());
        }

        // fnearest
        let test = memory.read_f64(0xA8);
        if fval.is_nan() {
            assert!(test.is_nan());
        } else if fval.is_infinite() {
            assert!(test.is_infinite());
        } else if fval.is_zero() {
            assert_eq!(test, fval);
        } else if fval > 0.0 && fval <= 0.5 {
            assert_eq!(test, 0.0);
        } else if fval > 0.5 && fval < 1.5 {
            assert_eq!(test, 1.0);
        } else if fval >= 1.5 && fval <= 2.5 {
            assert_eq!(test, 2.0);
        } else if fval > 2.5 && fval < 3.5 {
            assert_eq!(test, 3.0);
        } else if fval >= 3.5 && fval <= 4.5 {
            assert_eq!(test, 4.0);
        } else if fval < 0.0 && fval >= -0.5 {
            assert_eq!(test, -0.0);
        } else if fval < -0.5 && fval > -1.5 {
            assert_eq!(test, -1.0);
        } else if fval <= -1.5 && fval >= -2.5 {
            assert_eq!(test, -2.0);
        } else if fval < -2.5 && fval > -3.5 {
            assert_eq!(test, -3.0);
        } else if fval <= -3.5 && fval >= -4.5 {
            assert_eq!(test, -4.0);
        } else {
            // TODO: not stable
            // assert_eq!(fnearest, fval.round_ties_even());
        }

        // fsqrt
        let test = memory.read_f64(0xB0);
        if fval.is_nan() {
            assert!(test.is_nan());
        } else if fval.is_zero() {
            assert_eq!(test.to_bits(), fval.to_bits());
        } else if fval.is_sign_negative() {
            assert!(test.is_nan());
        } else if fval.is_infinite() {
            assert!(test.is_sign_positive());
            assert!(test.is_infinite());
        } else {
            assert!(test.is_sign_positive());
            assert_eq!(test, fval.sqrt());
        }

        assert_eq!(memory.read_u64(0xB8), 0xCCCC_CCCC_CCCC_CCCC);
        assert_eq!(memory.read_u64(0xC0), 0xCCCC_CCCC_CCCC_CCCC);
    }

    let values = [
        0.0,
        -0.0,
        0.5,
        1.0,
        1.5,
        2.0,
        -0.5,
        -1.0,
        -1.5,
        -2.0,
        1.1920929E-7,
        1.17549435E-38,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        PI,
    ];

    for lhs in &values {
        for rhs in &values {
            let lhs = *lhs;
            let rhs = *rhs;

            memory.fill(0xCC);
            let result = instance
                .function("test_bin_f64")
                .unwrap()
                .invoke(&[lhs.into(), rhs.into()])
                .unwrap()
                .unwrap()
                .get_i32()
                .unwrap();
            assert_eq!(result, 0x58);

            assert_eq!(memory.read_u64(0), 0xCCCC_CCCC_CCCC_CCCC);
            assert_eq!(memory.read_u64(8), 0xCCCC_CCCC_CCCC_CCCC);

            // feq
            let test = memory.read_u32(0x10);
            if lhs.is_nan() || rhs.is_nan() {
                assert_eq!(test, 0);
            } else if lhs.is_zero() && rhs.is_zero() {
                assert_eq!(test, 1);
            } else {
                assert_eq!(test, (lhs == rhs) as u32);
            }

            // fne
            let test = memory.read_u32(0x14);
            if lhs.is_nan() || rhs.is_nan() {
                assert_eq!(test, 1);
            } else if lhs.is_zero() && rhs.is_zero() {
                assert_eq!(test, 0);
            } else {
                assert_eq!(test, (lhs != rhs) as u32);
            }

            // flt
            let test = memory.read_u32(0x18);
            if lhs.is_nan() || rhs.is_nan() {
                assert_eq!(test, 0);
            } else if lhs == rhs {
                assert_eq!(test, 0);
            } else if lhs == f64::INFINITY {
                assert_eq!(test, 0);
            } else if lhs == f64::NEG_INFINITY {
                assert_eq!(test, 1);
            } else if rhs == f64::INFINITY {
                assert_eq!(test, 1);
            } else if rhs == f64::NEG_INFINITY {
                assert_eq!(test, 0);
            } else if lhs.is_zero() && rhs.is_zero() {
                assert_eq!(test, 0);
            } else {
                assert_eq!(test, (lhs < rhs) as u32);
            }

            // fgt
            let test = memory.read_u32(0x1C);
            if lhs.is_nan() || rhs.is_nan() {
                assert_eq!(test, 0);
            } else if lhs == rhs {
                assert_eq!(test, 0);
            } else if lhs == f64::INFINITY {
                assert_eq!(test, 1);
            } else if lhs == f64::NEG_INFINITY {
                assert_eq!(test, 0);
            } else if rhs == f64::INFINITY {
                assert_eq!(test, 0);
            } else if rhs == f64::NEG_INFINITY {
                assert_eq!(test, 1);
            } else if lhs.is_zero() && rhs.is_zero() {
                assert_eq!(test, 0);
            } else {
                assert_eq!(test, (lhs > rhs) as u32);
            }

            // fle
            let test = memory.read_u32(0x20);
            if lhs.is_nan() || rhs.is_nan() {
                assert_eq!(test, 0);
            } else if lhs == rhs {
                assert_eq!(test, 1);
            } else if lhs == f64::INFINITY {
                assert_eq!(test, 0);
            } else if lhs == f64::NEG_INFINITY {
                assert_eq!(test, 1);
            } else if rhs == f64::INFINITY {
                assert_eq!(test, 1);
            } else if rhs == f64::NEG_INFINITY {
                assert_eq!(test, 0);
            } else if lhs.is_zero() && rhs.is_zero() {
                assert_eq!(test, 1);
            } else {
                assert_eq!(test, (lhs <= rhs) as u32);
            }

            // fgt
            let test = memory.read_u32(0x24);
            if lhs.is_nan() || rhs.is_nan() {
                assert_eq!(test, 0);
            } else if lhs == rhs {
                assert_eq!(test, 1);
            } else if lhs == f64::INFINITY {
                assert_eq!(test, 1);
            } else if lhs == f64::NEG_INFINITY {
                assert_eq!(test, 0);
            } else if rhs == f64::INFINITY {
                assert_eq!(test, 0);
            } else if rhs == f64::NEG_INFINITY {
                assert_eq!(test, 1);
            } else if lhs.is_zero() && rhs.is_zero() {
                assert_eq!(test, 1);
            } else {
                assert_eq!(test, (lhs >= rhs) as u32);
            }

            // fadd
            let test = memory.read_f64(0x28);
            if lhs.is_nan() || rhs.is_nan() {
                assert!(test.is_nan());
            } else if lhs == f64::INFINITY && rhs == f64::INFINITY {
                assert_eq!(test, f64::INFINITY);
            } else if lhs == f64::INFINITY && rhs == f64::NEG_INFINITY {
                assert!(test.is_nan());
            } else if lhs == f64::NEG_INFINITY && rhs == f64::INFINITY {
                assert!(test.is_nan());
            } else if lhs == f64::NEG_INFINITY && rhs == f64::NEG_INFINITY {
                assert_eq!(test, f64::NEG_INFINITY);
            } else if rhs.is_infinite() {
                assert_eq!(test, rhs);
            } else if lhs.is_infinite() {
                assert_eq!(test, lhs);
            } else if lhs.is_zero() && rhs.is_zero() {
                if lhs.is_sign_positive() == rhs.is_sign_positive() {
                    assert_sign_eq!(test, lhs);
                } else {
                    assert!(test.is_sign_positive());
                }
                assert!(test.is_zero());
            } else if rhs.is_zero() {
                assert_eq!(test, lhs);
            } else if lhs.is_zero() {
                assert_eq!(test, rhs);
            } else if lhs.abs() == rhs.abs() && lhs.is_sign_positive() == rhs.is_sign_negative() {
                assert_eq!(test.to_bits(), ZERO_BITS);
            } else {
                assert_eq!(test, lhs + rhs);
            }

            // fsub
            let test = memory.read_f64(0x30);
            if lhs.is_nan() || rhs.is_nan() {
                assert!(test.is_nan());
            } else if lhs == f64::INFINITY && rhs == f64::INFINITY {
                assert!(test.is_nan());
            } else if lhs == f64::INFINITY && rhs == f64::NEG_INFINITY {
                assert_eq!(test, f64::INFINITY);
            } else if lhs == f64::NEG_INFINITY && rhs == f64::INFINITY {
                assert_eq!(test, f64::NEG_INFINITY);
            } else if lhs == f64::NEG_INFINITY && rhs == f64::NEG_INFINITY {
                assert!(test.is_nan());
            } else if lhs == f64::INFINITY {
                assert_eq!(test, f64::INFINITY);
            } else if lhs == f64::NEG_INFINITY {
                assert_eq!(test, f64::NEG_INFINITY);
            } else if rhs == f64::INFINITY {
                assert_eq!(test, f64::NEG_INFINITY);
            } else if rhs == f64::NEG_INFINITY {
                assert_eq!(test, f64::INFINITY);
            } else if lhs.is_zero() && rhs.is_zero() {
                if lhs.is_sign_positive() == rhs.is_sign_positive() {
                    assert!(test.is_sign_positive());
                } else {
                    assert_sign_eq!(test, lhs);
                }
                assert!(test.is_zero());
            } else if rhs.is_zero() {
                assert_eq!(test, lhs);
            } else if lhs.is_zero() {
                assert_sign_ne!(test, rhs);
                assert_eq!(test.abs(), rhs.abs());
            } else if lhs.abs() == rhs.abs() && lhs.is_sign_positive() == rhs.is_sign_positive() {
                assert_eq!(test.to_bits(), ZERO_BITS);
            } else {
                assert_eq!(test, lhs - rhs);
            }

            // fmul
            let test = memory.read_f64(0x38);
            if lhs.is_nan() || rhs.is_nan() {
                assert!(test.is_nan());
            } else if lhs.is_zero() && rhs.is_infinite() || lhs.is_infinite() && rhs.is_zero() {
                assert!(test.is_nan());
            } else if lhs == f64::INFINITY && rhs == f64::INFINITY {
                assert_eq!(test, f64::INFINITY);
            } else if lhs == f64::INFINITY && rhs == f64::NEG_INFINITY {
                assert_eq!(test, f64::NEG_INFINITY);
            } else if lhs == f64::NEG_INFINITY && rhs == f64::INFINITY {
                assert_eq!(test, f64::NEG_INFINITY);
            } else if lhs == f64::NEG_INFINITY && rhs == f64::NEG_INFINITY {
                assert_eq!(test, f64::INFINITY);
            } else if lhs.is_infinite() && lhs.is_sign_positive() == rhs.is_sign_positive()
                || rhs.is_infinite() && lhs.is_sign_positive() == rhs.is_sign_positive()
            {
                assert_eq!(test, f64::INFINITY);
            } else if lhs.is_infinite() && lhs.is_sign_positive() == rhs.is_sign_negative()
                || rhs.is_infinite() && lhs.is_sign_positive() == rhs.is_sign_negative()
            {
                assert_eq!(test, f64::NEG_INFINITY);
            } else if lhs.is_zero() && rhs.is_zero() {
                if lhs.is_sign_positive() == rhs.is_sign_positive() {
                    assert!(test.is_sign_positive());
                } else {
                    assert!(test.is_sign_negative());
                }
                assert!(test.is_zero());
            } else {
                assert_eq!(test, lhs * rhs);
            }

            // fdiv
            let test = memory.read_f64(0x40);
            if lhs.is_nan() || rhs.is_nan() {
                assert!(test.is_nan());
            } else if lhs.is_infinite() && rhs.is_infinite() {
                assert!(test.is_nan());
            } else if lhs.is_zero() && rhs.is_zero() {
                assert!(test.is_nan());
            } else if lhs == f64::INFINITY && lhs.is_sign_positive() == rhs.is_sign_positive() {
                assert_eq!(test, f64::INFINITY);
            } else if lhs == f64::INFINITY && lhs.is_sign_positive() == rhs.is_sign_negative() {
                assert_eq!(test, f64::NEG_INFINITY);
            } else if rhs == f64::INFINITY && lhs.is_sign_positive() == rhs.is_sign_positive() {
                assert_eq!(test.to_bits(), ZERO_BITS);
            } else if rhs == f64::INFINITY && lhs.is_sign_positive() == rhs.is_sign_negative() {
                assert_eq!(test.to_bits(), NEG_ZERO_BITS);
            } else if lhs.is_zero() && lhs.is_sign_positive() == rhs.is_sign_positive() {
                assert_eq!(test.to_bits(), ZERO_BITS);
            } else if lhs.is_zero() && lhs.is_sign_positive() == rhs.is_sign_negative() {
                assert_eq!(test.to_bits(), NEG_ZERO_BITS);
            } else if rhs.is_zero() && lhs.is_sign_positive() == rhs.is_sign_positive() {
                assert_eq!(test, f64::INFINITY);
            } else if rhs.is_zero() && lhs.is_sign_positive() == rhs.is_sign_negative() {
                assert_eq!(test, f64::NEG_INFINITY);
            } else {
                assert_eq!(test, lhs / rhs);
            }

            // fcopysign
            let test = memory.read_f64(0x48);
            if lhs.is_sign_positive() == rhs.is_sign_positive() {
                assert_eq!(test.to_bits(), lhs.to_bits());
            } else {
                assert_eq!(test.to_bits(), lhs.to_bits() ^ SIGN_BITS);
            }

            // fmin
            let test = memory.read_f64(0x50);
            if lhs.is_nan() || rhs.is_nan() {
                assert!(test.is_nan());
            } else if lhs == f64::NEG_INFINITY || rhs == f64::NEG_INFINITY {
                assert_eq!(test, f64::NEG_INFINITY);
            } else if lhs == f64::INFINITY {
                assert_eq!(test.to_bits(), rhs.to_bits());
            } else if rhs == f64::INFINITY {
                assert_eq!(test.to_bits(), lhs.to_bits());
            } else if lhs.is_zero()
                && rhs.is_zero()
                && lhs.is_sign_positive() == rhs.is_sign_negative()
            {
                assert_eq!(test.to_bits(), NEG_ZERO_BITS);
            } else if lhs < rhs {
                assert_eq!(test, lhs);
            } else {
                assert_eq!(test, rhs);
            }

            // fmax
            let test = memory.read_f64(0x58);
            if lhs.is_nan() || rhs.is_nan() {
                assert!(test.is_nan());
            } else if lhs == f64::INFINITY || rhs == f64::INFINITY {
                assert_eq!(test, f64::INFINITY);
            } else if lhs == f64::NEG_INFINITY {
                assert_eq!(test.to_bits(), rhs.to_bits());
            } else if rhs == f64::NEG_INFINITY {
                assert_eq!(test.to_bits(), lhs.to_bits());
            } else if lhs.is_zero()
                && rhs.is_zero()
                && lhs.is_sign_positive() == rhs.is_sign_negative()
            {
                assert_eq!(test.to_bits(), ZERO_BITS);
            } else if lhs > rhs {
                assert_eq!(test, lhs);
            } else {
                assert_eq!(test, rhs);
            }

            assert_eq!(memory.read_u64(0x60), 0xCCCC_CCCC_CCCC_CCCC);
            assert_eq!(memory.read_u64(0x68), 0xCCCC_CCCC_CCCC_CCCC);
        }
    }
}

#[test]
fn block_nest() {
    let slice = [0, 0x02, 0x40, 0x02, 0x40, 0x01, 0x0B, 0x0B, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    WasmCodeBlock::generate(0, 0, &mut stream, &[], &[], instance.module()).unwrap();

    let slice = [0, 0x02, 0x40, 0x02, 0x40, 0x01, 0x0B, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    assert_matches!(
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &[], instance.module())
            .unwrap_err()
            .kind(),
        CompileErrorKind::UnexpectedEof
    );

    let slice = [0, 0x02, 0x7F, 0x02, 0x7F, 0x41, 0x01, 0x0B, 0x0B, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &[],
        &[WasmValType::I32],
        instance.module(),
    )
    .unwrap();

    let slice = [0, 0x02, 0x7F, 0x02, 0x7F, 0x01, 0x0B, 0x0B, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    assert_matches!(
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &[], instance.module())
            .unwrap_err()
            .kind(),
        CompileErrorKind::OutOfStack
    );

    let slice = [0, 0x02, 0x7F, 0x02, 0x7F, 0x41, 0x01, 0x0B, 0x0B, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    assert_matches!(
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &[], instance.module())
            .unwrap_err()
            .kind(),
        CompileErrorKind::InvalidStackLevel
    );

    let slice = [0, 0x02, 0x7F, 0x02, 0x7F, 0x41, 0x01, 0x0B, 0x0B, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    assert_matches!(
        WasmCodeBlock::generate(
            0,
            0,
            &mut stream,
            &[],
            &[WasmValType::I64],
            instance.module()
        )
        .unwrap_err()
        .kind(),
        CompileErrorKind::TypeMismatch
    );

    let slice = [0, 0x02, 0x7F, 0x02, 0x7F, 0x42, 0x01, 0x0B, 0x0B, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    assert_matches!(
        WasmCodeBlock::generate(
            0,
            0,
            &mut stream,
            &[],
            &[WasmValType::I32],
            instance.module()
        )
        .unwrap_err()
        .kind(),
        CompileErrorKind::TypeMismatch
    );

    let slice = [0, 0x02, 0x7F, 0x02, 0x7E, 0x41, 0x01, 0x0B, 0x0B, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    assert_matches!(
        WasmCodeBlock::generate(
            0,
            0,
            &mut stream,
            &[],
            &[WasmValType::I32],
            instance.module()
        )
        .unwrap_err()
        .kind(),
        CompileErrorKind::TypeMismatch
    );

    let slice = [0, 0x02, 0x7E, 0x02, 0x7F, 0x41, 0x01, 0x0B, 0x0B, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    assert_matches!(
        WasmCodeBlock::generate(
            0,
            0,
            &mut stream,
            &[],
            &[WasmValType::I32],
            instance.module()
        )
        .unwrap_err()
        .kind(),
        CompileErrorKind::TypeMismatch
    );

    let slice = [0, 0x02, 0x7E, 0x02, 0x7F, 0x41, 0x01, 0x0B, 0x0B, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    assert_matches!(
        WasmCodeBlock::generate(
            0,
            0,
            &mut stream,
            &[],
            &[WasmValType::I64],
            instance.module()
        )
        .unwrap_err()
        .kind(),
        CompileErrorKind::TypeMismatch
    );

    let slice = [0, 0x02, 0x7F, 0x02, 0x7F, 0x20, 0x00, 0x0B, 0x0B, 0x0B];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Reader::from_slice(&slice);
    let info = WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &[WasmValType::I32],
        &result_types,
        instance.module(),
    )
    .unwrap();
    let mut interp = WasmInterpreter::new(&instance);

    let result = interp
        .invoke(0, &info, &mut [123.into()], &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 123);
}

#[test]
fn block_test() {
    let instance =
        WebAssembly::instantiate(include_bytes!("../test/tester.wasm"), &Env {}).unwrap();
    let runnable = instance.function("block_test").unwrap();

    let result = runnable
        .invoke(&[1.into(), 123.into(), 456.into(), 789.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 456);

    let result = runnable
        .invoke(&[2.into(), 123.into(), 456.into(), 789.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 789);
}

#[test]
fn loop_nest() {
    let slice = [0, 0x02, 0x40, 0x03, 0x40, 0x01, 0x0B, 0x0B, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    WasmCodeBlock::generate(0, 0, &mut stream, &[], &[], instance.module()).unwrap();

    let slice = [0, 0x02, 0x40, 0x03, 0x40, 0x01, 0x0B, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    assert_matches!(
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &[], instance.module())
            .unwrap_err()
            .kind(),
        CompileErrorKind::UnexpectedEof
    );

    let slice = [0, 0x02, 0x7F, 0x03, 0x7F, 0x41, 0x01, 0x0B, 0x0B, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    WasmCodeBlock::generate(
        0,
        0,
        &mut stream,
        &[],
        &[WasmValType::I32],
        instance.module(),
    )
    .unwrap();

    let slice = [0, 0x02, 0x7F, 0x03, 0x7F, 0x01, 0x0B, 0x0B, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    assert_matches!(
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &[], instance.module())
            .unwrap_err()
            .kind(),
        CompileErrorKind::OutOfStack
    );
}

#[test]
fn loop_test() {
    let instance =
        WebAssembly::instantiate(include_bytes!("../test/tester.wasm"), &Env {}).unwrap();
    let runnable = instance.function("loop_test").unwrap();

    let result = runnable
        .invoke(&[10.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 55);

    let result = runnable
        .invoke(&[100.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 5050);
}

#[test]
fn if_nest() {
    let slice = [0, 0x41, 0x01, 0x04, 0x40, 0x01, 0x0B, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    WasmCodeBlock::generate(0, 0, &mut stream, &[], &[], instance.module()).unwrap();

    let slice = [0, 0x41, 0x01, 0x04, 0x40, 0x01, 0x05, 0x01, 0x0B, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    WasmCodeBlock::generate(0, 0, &mut stream, &[], &[], instance.module()).unwrap();

    let slice = [
        0, 0x41, 0x01, 0x04, 0x7F, 0x41, 0x01, 0x05, 0x41, 0x01, 0x0B, 0x1A, 0x0B,
    ];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    WasmCodeBlock::generate(0, 0, &mut stream, &[], &[], instance.module()).unwrap();

    let slice = [0, 0x41, 0x01, 0x04, 0x7F, 0x41, 0x01, 0x0B, 0x1A, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    assert_matches!(
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &[], instance.module())
            .unwrap_err()
            .kind(),
        CompileErrorKind::ElseNotExists
    );

    let slice = [0, 0x05, 0x01, 0x0B];
    let mut stream = Leb128Reader::from_slice(&slice);
    let instance = WasmInstance::empty();
    assert_matches!(
        WasmCodeBlock::generate(0, 0, &mut stream, &[], &[], instance.module())
            .unwrap_err()
            .kind(),
        CompileErrorKind::ElseWithoutIf
    );
}

#[test]
fn if_test() {
    let instance =
        WebAssembly::instantiate(include_bytes!("../test/tester.wasm"), &Env {}).unwrap();
    let runnable = instance.function("if_test1").unwrap();

    let result = runnable
        .invoke(&[123.into(), 456.into(), 1.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 123);

    let result = runnable
        .invoke(&[123.into(), 456.into(), 2.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 123);

    let result = runnable
        .invoke(&[123.into(), 456.into(), 0.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 456);

    let runnable = instance.function("if_test2").unwrap();

    let result = runnable
        .invoke(&[123.into(), 456.into(), 1.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 123);

    let result = runnable
        .invoke(&[123.into(), 456.into(), 2.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 123);

    let result = runnable
        .invoke(&[123.into(), 456.into(), 0.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 456);
}

#[test]
fn import_test() {
    let instance =
        WebAssembly::instantiate(include_bytes!("../test/tester.wasm"), &Env {}).unwrap();
    let runnable = instance.function("import_test1").unwrap();

    let result = runnable
        .invoke(&[123.into(), 456.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 123 + 456);

    let runnable = instance.function("import_test2").unwrap();

    let result = runnable
        .invoke(&[987.into(), 654.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 987 - 654);
}
