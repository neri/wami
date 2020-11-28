// Wasm Runner

use std::env;
use std::fs::File;
use std::io::Read;
use wasm_o::wasm::*;

fn main() {
    let mut args = env::args();
    let self_name = args.next().unwrap();

    let in_file = match args.next() {
        Some(v) => v,
        None => {
            println!("usage: {} WASM", self_name);
            return;
        }
    };
    let mut is = File::open(in_file).unwrap();
    let mut blob = Vec::new();
    let _ = is.read_to_end(&mut blob).unwrap();

    let module = WasmLoader::instantiate(blob.as_slice()).unwrap();

    match module
        .function("add")
        .and_then(|v| v.invoke(&[123.into(), 456.into()]))
    {
        Ok(v) => {
            println!("[result: {}]", v);
        }
        Err(err) => {
            println!("error: {:?}", err);
        }
    }
}
