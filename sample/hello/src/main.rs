// Hello world

use wami::prelude::*;

fn main() {
    let bin = include_bytes!("../hello.wasm");
    let instance = WebAssembly::instantiate(bin, &Env {}).unwrap();

    assert_eq!(instance.exports().add(123, 456).unwrap(), 123 + 456);
}

struct Env;

#[wasm_env]
impl Env {
    pub fn print(value: i32) {
        println!("{}", value);
    }
}

#[wasm_exports]
trait Hello {
    fn add(lhs: i32, rhs: i32) -> i32;
}
