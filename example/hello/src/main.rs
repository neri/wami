//! Hello world
use wami::prelude::*;

fn main() {
    let src = r#"
(module
  (import "env" "println" (func $println (param i32) (param i32)))

  (memory 1)

  (data (i32.const 16) "Hello, World!")

  (func $main (export "main")
    i32.const 13
    i32.const 16
    call $println
  )
)
"#;
    let bin = WebAssembly::from_wat("hello.wat", src.as_bytes().to_vec()).unwrap();
    let instance = WebAssembly::instantiate(&bin, &Env {}).unwrap();
    instance.exports().main().unwrap();
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
    fn main();
}
