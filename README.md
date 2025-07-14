# WAMI

A WebAssembly Interpreter

## Features

- Support for `no_std`
- A subset of WebAssembly 2.0
- This library by itself does not support execution environments such as WASI.

## Supported WebAssembly 2.0 Features

|Proposals|Status|
|-|-|
|Sign extension instructions|✅|
|Non-trapping float-to-int conversions|✅|
|Multiple values|-|
|Reference types|-|
|Table instructions|-|
|Multiple tables|-|
|Bulk memory and table instructions|`memory.fill`, `memory.copy`|
|Vector instructions|-|

## Requirements

- Rust nightly

### MSRV

- The latest version is recommended whenever possible.

## Test

```
# cargo test
```

## Example of use

* The actual sample can be found in `/example/hello`.
* First there is WebAssembly like this.

```wat
(module
  (import "env" "println" (func $println (param i32) (param i32)))

  (memory 1)

  (data (i32.const 16) "hello world!")

  (func $main (export "main")
    i32.const 12
    i32.const 16
    call $println
  )
)
```

* To run this, we create the following Rust code.

```rust
use wa_asm::WasmAssembler;
use wami::prelude::*;

fn main() {
    let src = r#"
(module
  (import "env" "println" (func $println (param i32) (param i32)))

  (memory 1)

  (data (i32.const 16) "hello world!")

  (func $main (export "main")
    i32.const 12
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
```

## License

MIT License

(C) 2020 Nerry
