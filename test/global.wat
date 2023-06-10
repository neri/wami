(module
  (global $g1 (mut i32) (i32.const 123))
  (func (export "global_add") (param i32) (result i32)
    global.get $g1
    local.get 0
    i32.add
    global.set $g1
    global.get $g1
  )
)
