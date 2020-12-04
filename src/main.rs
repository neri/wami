// Wasm Runner

use std::env;
use std::fs::File;
use std::io::Read;
use wasm_o::wasm::*;

fn usage() {
    let arg = env::args().next().unwrap();
    println!("usage: {} WASMFILE [FUNCTION]", arg);
}

fn main() {
    let mut args = env::args();
    let _ = args.next().unwrap();

    let mut option_d = false;

    let in_file = loop {
        match args.next() {
            Some(v) => {
                if v.starts_with("-") {
                    match &*v {
                        "-d" => option_d = true,
                        _ => {
                            usage();
                            return;
                        }
                    }
                } else {
                    break v;
                }
            }
            None => {
                usage();
                return;
            }
        }
    };

    let function_name = args.next().unwrap_or("_start".to_string());

    let mut is = File::open(in_file).unwrap();
    let mut blob = Vec::new();
    let _ = is.read_to_end(&mut blob).unwrap();

    let mut module =
        WasmLoader::instantiate(blob.as_slice(), &|_mod_name, name, _type_ref| match name {
            "fd_write" => Ok(Box::new(FdWrite::new()) as Box<dyn WasmInvocation>),
            _ => Err(WasmDecodeError::DynamicLinkError),
        })
        .unwrap();

    if option_d {
        module.print_stat();
    } else {
        match module
            .function(&function_name)
            .and_then(|v| v.invoke(&[7.into(), 1.into()]))
        {
            Ok(v) => {
                println!("result: {}", v);
            }
            Err(err) => {
                println!("error: {:?}", err);
            }
        }
    }
}

struct FdWrite {}

impl FdWrite {
    const fn new() -> Self {
        Self {}
    }
}

impl WasmInvocation for FdWrite {
    fn invoke(
        &self,
        module: &WasmModule,
        params: &[WasmValue],
    ) -> Result<WasmValue, WasmRuntimeError> {
        // fd_write (i32 i32 i32 i32) -> i32

        let memory = module.memory(0).unwrap();

        let iovs = params
            .get(1)
            .ok_or(WasmRuntimeError::InvalidParameter)
            .and_then(|v| v.get_u32())? as usize;
        // let iovs_len = params
        //     .get(2)
        //     .ok_or(WasmRuntimeError::InvalidParameter)
        //     .and_then(|v| v.get_i32())?;

        let iov_base = memory.read_u32(iovs)? as usize;
        let iov_len = memory.read_u32(iovs + 4)? as usize;

        let slice = memory.read_bytes(iov_base, iov_len)?;
        let s = core::str::from_utf8(slice).map_err(|_| WasmRuntimeError::InvalidParameter)?;
        print!("{}", s);

        Ok(WasmValue::I32(s.len() as i32))
    }
}
