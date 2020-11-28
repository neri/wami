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

    let function_name = args.next().unwrap_or("start".to_string());

    let mut is = File::open(in_file).unwrap();
    let mut blob = Vec::new();
    let _ = is.read_to_end(&mut blob).unwrap();

    let mut module = WasmLoader::instantiate(blob.as_slice()).unwrap();

    if option_d {
        module.print_stat();
    } else {
        match module
            .function(function_name.as_ref())
            .and_then(|v| v.invoke(&[123.into(), 456.into()]))
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
