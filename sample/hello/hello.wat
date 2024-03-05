(module

  (import "env" "print" (func $print (param i32)))

  (memory 0)

  (func $add (export "add") (param $a i32) (param $b i32) (result i32)
    (local $i i32)

    local.get $a
    local.get $b
    i32.add
    local.tee $i
    call $print
    local.get $i
  )

)
