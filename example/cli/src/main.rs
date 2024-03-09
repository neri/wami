// test cli application

use std::env;
use wami::WebAssembly;

fn main() {
    let mut args = env::args();
    let _ = args.next().unwrap();

    let input = args.next().expect("please specify the filename");

    let bytes = std::fs::read(&input).expect("cannot read input");

    let module = WebAssembly::compile(&bytes).unwrap();

    println!("Compile OK");

    if module.imports().count() > 0 {
        println!("Imports: ")
    }
    for import in module.imports() {
        println!("  {:?} {:?} {:?}", import.module, import.name, import.kind);
    }

    if module.exports().count() > 0 {
        println!("Exports: ")
    }
    for export in module.exports() {
        println!("  {:?} {:?}", export.name, export.kind);
    }
}
