// Hello world

use wami::prelude::*;

fn main() {
    let instance = WebAssembly::instantiate(include_bytes!("../hello.wasm"), &Env {}).unwrap();

    assert_eq!(instance.exports().main().unwrap(), 42);
}

struct Env;

#[wasm_env]
impl Env {
    pub fn println(s: &str) {
        println!("{}", s)
    }
}

#[wasm_exports]
trait Hello {
    fn main() -> i32;
}
