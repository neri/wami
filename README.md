# WAMI

A WebAssembly Interpreter used in my os (https://github.com/neri/maystorm)

## Features

- Support for `no_std`
- A subset of WebAssembly 2.0
  - It can be used for most applications, but it does not support SIMD, reference, and some basic instructions.

## Requirements

- Rust nightly

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
use wami::prelude::*;

fn main() {
    let instance = WebAssembly::instantiate(include_bytes!("../hello.wasm"), &Env {}).unwrap();

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
