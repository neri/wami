(module

  (import "env" "println" (func $println (param i32) (param i32)))

  (memory 1)

  (data (i32.const 16) "hello world!")

  (func $main (export "main") (result i32)
    (local $i i32)

    i32.const 12
    i32.const 16
    call $println

    i32.const 42
  )

)
