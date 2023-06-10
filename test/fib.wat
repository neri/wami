(module
  (func (export "fib") (param i32) (result i32)
    (local i32)
    i32.const 0
    local.set 1
    block  ;; label = @1
      loop  ;; label = @2
        local.get 0
        i32.const 2
        i32.lt_u
        br_if 1 (;@1;)
        local.get 0
        i32.const -1
        i32.add
        call 0
        local.get 1
        i32.add
        local.set 1
        local.get 0
        i32.const -2
        i32.add
        local.set 0
        br 0 (;@2;)
      end
    end
    local.get 0
    local.get 1
    i32.add
  )
)
