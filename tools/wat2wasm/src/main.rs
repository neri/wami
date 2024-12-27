//! WebAssembly Assembler CLI Frontend

use libwat2wasm::*;
use std::{
    env::{self, args},
    fs::{File, read_to_string},
    io::{self, Write},
    process,
};

fn usage() -> ! {
    let mut args = env::args_os();
    let arg = args.next().unwrap();
    eprintln!(
        "WebAssembly Assembler\nusage: {} [OPTIONS] INPUT",
        arg.to_str().unwrap()
    );
    process::exit(1);
}

fn main() {
    let mut args = args();
    let _ = args.next().unwrap();

    // let mut to_run = false;
    let mut path_input = None;
    let mut path_output = None;

    while let Some(arg) = args.next() {
        if arg.starts_with("-") {
            match arg.as_str() {
                // "-run" => {
                //     to_run = true;
                // }
                "-o" => match args.next() {
                    Some(v) => path_output = Some(v),
                    None => usage(),
                },
                "--" => {
                    path_input = args.next();
                    break;
                }
                _ => panic!("unknown option: {}", arg),
            }
        } else {
            path_input = Some(arg);
            break;
        }
    }

    let path_input = match path_input {
        Some(v) => v,
        None => usage(),
    };

    let src = read_to_string(path_input.as_str()).unwrap();
    let binary = match WasmAssembler::assemble(path_input.as_str(), src.into_bytes()) {
        Ok(v) => v,
        Err(e) => {
            panic!("{}", e);
        }
    };

    if let Some(path_output) = path_output {
        let mut os = File::create(path_output).unwrap();
        os.write(&binary).unwrap();
    } else {
        io::stdout().write(binary.as_slice()).unwrap();
    }
}
