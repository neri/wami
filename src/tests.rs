// test

use crate::{
    wasmintr::{WasmInterpreter, WasmInvocation},
    Leb128Stream, WasmCodeBlock, WasmLoader, WasmModule, WasmRuntimeErrorType, WasmValType,
};

#[test]
fn instantiate() {
    let minimal = [0, 97, 115, 109, 1, 0, 0, 0];
    WasmLoader::instantiate(&minimal, |_, _, _| unreachable!()).unwrap();
}

#[test]
#[should_panic(expected = "BadExecutable")]
fn instantiate_bad_exec1() {
    let too_small = [0, 97, 115, 109, 1, 0, 0];
    WasmLoader::instantiate(&too_small, |_, _, _| unreachable!()).unwrap();
}

#[test]
#[should_panic(expected = "BadExecutable")]
fn instantiate_bad_exec2() {
    let too_small = [0, 97, 115, 109, 2, 0, 0, 0];
    WasmLoader::instantiate(&too_small, |_, _, _| unreachable!()).unwrap();
}

#[test]
#[should_panic(expected = "UnexpectedEof")]
fn instantiate_unexpected_eof() {
    let minimal_bad = [0, 97, 115, 109, 1, 0, 0, 0, 1];
    WasmLoader::instantiate(&minimal_bad, |_, _, _| unreachable!()).unwrap();
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
fn add() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x6A, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let params = [1234.into(), 5678.into()];

    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 6912);

    let params = [0xDEADBEEFu32.into(), 0x55555555.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
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
        WasmCodeBlock::generate(0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let params = [1234_5678.into()];

    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 12345679);

    let params = [0xFFFF_FFFFu32.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);
}

#[test]
fn sub() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x6B, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let params = [1234.into(), 5678.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, -4444);

    let params = [0x55555555.into(), 0xDEADBEEFu32.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
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
        WasmCodeBlock::generate(0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let params = [1234.into(), 5678.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 7006652);

    let params = [0x55555555.into(), 0xDEADBEEFu32.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0x6070c05b);
}

#[test]
fn div_s() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x6D, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let params = [7006652.into(), 5678.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1234);

    let params = [42.into(), (-6).into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, -7);

    let params = [(-42).into(), (6).into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, -7);

    let params = [(-42).into(), (-6).into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 7);

    let params = [1234.into(), 0.into()];
    let result = interp.invoke(0, &info, &params, &result_types).unwrap_err();
    assert_eq!(WasmRuntimeErrorType::DivideByZero, result.kind());
}

#[test]
fn div_u() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x6E, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let params = [7006652.into(), 5678.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1234);

    let params = [42.into(), (-6).into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let params = [(-42).into(), (6).into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 715827875);

    let params = [1234.into(), 0.into()];
    let result = interp.invoke(0, &info, &params, &result_types).unwrap_err();
    assert_eq!(WasmRuntimeErrorType::DivideByZero, result.kind());
}

#[test]
fn select() {
    let slice = [0, 0x20, 0, 0x20, 1, 0x20, 2, 0x1B, 0x0B];
    let param_types = [WasmValType::I32, WasmValType::I32, WasmValType::I32];
    let result_types = [WasmValType::I32];
    let mut stream = Leb128Stream::from_slice(&slice);
    let module = WasmModule::new();
    let info =
        WasmCodeBlock::generate(0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let params = [123.into(), 456.into(), 789.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 123);

    let params = [123.into(), 456.into(), 0.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
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
        WasmCodeBlock::generate(0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let params = [123.into(), 456.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1);

    let params = [123.into(), 123.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let params = [456.into(), 123.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let params = [123.into(), (-456).into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let params = [456.into(), (-123).into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
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
        WasmCodeBlock::generate(0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let params = [123.into(), 456.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1);

    let params = [123.into(), 123.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let params = [456.into(), 123.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let params = [123.into(), (-456).into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1);

    let params = [456.into(), (-123).into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
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
        WasmCodeBlock::generate(0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let params = [123.into(), 456.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1);

    let params = [123.into(), 123.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1);

    let params = [456.into(), 123.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let params = [123.into(), (-456).into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);

    let params = [456.into(), (-123).into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 0);
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
        WasmCodeBlock::generate(0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let params = [0.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 123);

    let params = [1.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 456);

    let params = [2.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 789);

    let params = [3.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 789);

    let params = [4.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 789);

    let params = [5.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 789);

    let params = [(-1).into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
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
        WasmCodeBlock::generate(0, &mut stream, &param_types, &result_types, &module).unwrap();
    let mut interp = WasmInterpreter::new(&module);

    let params = [7.into(), 0.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 5040);

    let params = [10.into(), 0.into()];
    let result = interp
        .invoke(0, &info, &params, &result_types)
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 3628800);
}

#[test]
fn app_fibonacci() {
    let slice = [
        0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00, 0x01, 0x06, 0x01, 0x60, 0x01, 0x7F, 0x01,
        0x7F, 0x03, 0x02, 0x01, 0x00, 0x0A, 0x31, 0x01, 0x2F, 0x01, 0x01, 0x7F, 0x41, 0x00, 0x21,
        0x01, 0x02, 0x40, 0x03, 0x40, 0x20, 0x00, 0x41, 0x02, 0x49, 0x0D, 0x01, 0x20, 0x00, 0x41,
        0x7F, 0x6A, 0x10, 0x00, 0x20, 0x01, 0x6A, 0x21, 0x01, 0x20, 0x00, 0x41, 0x7E, 0x6A, 0x21,
        0x00, 0x0C, 0x00, 0x0B, 0x0B, 0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B,
    ];
    let module = WasmLoader::instantiate(&slice, |_, _, _| unreachable!()).unwrap();
    let runnable = module.func_by_index(0).unwrap();

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
fn global() {
    let slice = [
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x06, 0x01, 0x60, 0x01, 0x7f, 0x01,
        0x7f, 0x03, 0x02, 0x01, 0x00, 0x05, 0x03, 0x01, 0x00, 0x01, 0x06, 0x07, 0x01, 0x7f, 0x01,
        0x41, 0xfb, 0x00, 0x0b, 0x0a, 0x0d, 0x01, 0x0b, 0x00, 0x23, 0x00, 0x20, 0x00, 0x6a, 0x24,
        0x00, 0x23, 0x00, 0x0b,
    ];
    let module = WasmLoader::instantiate(&slice, |_, _, _| unreachable!()).unwrap();
    let runnable = module.func_by_index(0).unwrap();

    assert_eq!(module.global(0).unwrap().value().get_i32().unwrap(), 123);

    let result = runnable
        .invoke(&[456.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 579);

    assert_eq!(module.global(0).unwrap().value().get_i32().unwrap(), 579);

    let result = runnable
        .invoke(&[789.into()])
        .unwrap()
        .unwrap()
        .get_i32()
        .unwrap();
    assert_eq!(result, 1368);

    assert_eq!(module.global(0).unwrap().value().get_i32().unwrap(), 1368);
}
