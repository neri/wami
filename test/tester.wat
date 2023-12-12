(module
  ;; test cases
  (memory 1)
  (global $stack_pointer (export "__stack_pointer") (mut i32) (i32.const 123))
  (global $global1 (export "global1") (mut i32) (i32.const 123))

  (func $local_test (export "local_test") (result i32)
    (local $i i32)
    (local $j i32)
    i32.const 123
    local.set $i
    i32.const 456
    local.set $j
    local.get $i
  )

  (func $global_add (export "global_add") (param i32) (result i32)
    global.get $global1
    local.get 0
    i32.add
    global.set $global1
    global.get $global1
  )

  (func $fib (export "fib") (param i32) (result i32)
    (local i32)
    i32.const 0
    local.set 1
    block $1
      loop $2
        local.get 0
        i32.const 2
        i32.lt_u
        br_if $1
        local.get 0
        i32.const -1
        i32.add
        call $fib
        local.get 1
        i32.add
        local.set 1
        local.get 0
        i32.const -2
        i32.add
        local.set 0
        br $2
      end
    end
    local.get 0
    local.get 1
    i32.add
  )

  (func $fact (export "fact") (param i32) (result i32)
    (local i32)
    i32.const 1
    local.set 1
    block $1
      loop $2
        local.get 0
        i32.eqz
        br_if $1
        local.get 1
        local.get 0
        i32.mul
        local.set 1
        local.get 0
        i32.const 1
        i32.sub
        local.set 0
        br $2
      end
    end
    local.get 1
  )

  (func (export "test_unary_i32") (param i32) (result i32)
    (local $p i32)

    i32.const 0x10
    local.tee $p
    local.get 0
    i32.eqz
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get 0
    i32.ctz
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get 0
    i32.clz
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get 0
    i32.popcnt
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get 0
    i32.extend8_s
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get 0
    i32.extend16_s
    i32.store

    local.get $p
  )

  (func (export "test_unary_i64") (param i64) (result i32)
    (local $p i32)

    i32.const 0x10
    local.tee $p
    local.get 0
    i64.eqz
    i32.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get 0
    i64.ctz
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get 0
    i64.clz
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get 0
    i64.popcnt
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get 0
    i64.extend8_s
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get 0
    i64.extend16_s
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get 0
    i64.extend32_s
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get 0
    i32.wrap_i64
    i32.store

    local.get $p
  )

  (func (export "test_bin_i32") (param $lhs i32) (param $rhs i32) (result i32)
    (local $p i32)

    i32.const 0x10
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.eq
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.ne
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.lt_s
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.lt_u
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.gt_s
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.gt_u
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.le_s
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.le_u
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.ge_s
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.ge_u
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.add
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.sub
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.mul
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.div_s
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.div_u
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.rem_s
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.rem_u
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.and
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.or
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.xor
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.shl
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.shr_s
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.shr_u
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.rotl
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i32.rotr
    i32.store

    local.get $p
  )

  (func (export "test_bin_i64") (param $lhs i64) (param $rhs i64) (result i32)
    (local $p i32)

    i32.const 0x10
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.eq
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.ne
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.lt_s
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.lt_u
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.gt_s
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.gt_u
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.le_s
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.le_u
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.ge_s
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.ge_u
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.add
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.sub
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.mul
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.div_s
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.div_u
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.rem_s
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.rem_u
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.and
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.or
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.xor
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.shl
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.shr_s
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.shr_u
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.rotl
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    i64.rotr
    i64.store

    local.get $p
  )

  (func $call_test1 (export "call_test1") (param $a1 i32) (param $a2 i32) (param $a3 i64) (param $a4 i64) (result i32)
    i32.const 0x10
    local.get $a1
    i32.store

    i32.const 0x14
    local.get $a2
    i32.store

    i32.const 0x18
    local.get $a3
    i64.store

    i32.const 0x20
    local.get $a4
    i64.store

    local.get $a1
  )

  (func $call_test2 (export "call_test2") (param $a1 i32) (param $a2 i32) (param $a3 i64) (param $a4 i64) (result i32)
    local.get $a2
    local.get $a1
    local.get $a4
    local.get $a3
    call $call_test1

    local.get $a1
    i32.add
  )

  (func $call_test3 (export "call_test3") (param $a1 i32) (param $a2 i32) (param $a3 i64) (param $a4 i64) (result i64)
    local.get $a2
    local.get $a1
    local.get $a4
    local.get $a3
    call $call_test2
    drop

    local.get $a3
  )

  (func $call_test4 (export "call_test4") (param $a1 i32) (param $a2 i32) (param $a3 i64) (param $a4 i64) (result i64)
    local.get $a1
    local.get $a2
    i32.add
    local.get $a1
    local.get $a2
    i32.sub
    local.get $a3
    local.get $a4
    i64.add
    local.get $a3
    local.get $a4
    i64.sub
    call $call_test3

    local.get $a3
    i64.sub
  )

  (func (export "mem_test_u32u8") (param $a1 i32) (param $a2 i32) (result i32)
    (local $temp i32)
    
    local.get $a2
    local.get $a1
    i32.load8_u
    local.tee $temp
    i32.store8

    local.get $temp
  )

  (func (export "mem_test_i32i8") (param $a1 i32) (param $a2 i32) (result i32)
    (local $temp i32)
    
    local.get $a2
    local.get $a1
    i32.load8_s
    local.tee $temp
    i32.store8

    local.get $temp
  )

  (func (export "mem_test_u32u16") (param $a1 i32) (param $a2 i32) (result i32)
    (local $temp i32)
    
    local.get $a2
    local.get $a1
    i32.load16_u
    local.tee $temp
    i32.store16

    local.get $temp
  )

  (func (export "mem_test_i32i16") (param $a1 i32) (param $a2 i32) (result i32)
    (local $temp i32)
    
    local.get $a2
    local.get $a1
    i32.load16_s
    local.tee $temp
    i32.store16

    local.get $temp
  )

  (func (export "mem_test_u32") (param $a1 i32) (param $a2 i32) (result i32)
    (local $temp i32)
    
    local.get $a2
    local.get $a1
    i32.load
    local.tee $temp
    i32.store

    local.get $temp
  )

  (func (export "mem_test_u64u8") (param $a1 i32) (param $a2 i32) (result i64)
    (local $temp i64)
    
    local.get $a2
    local.get $a1
    i64.load8_u
    local.tee $temp
    i64.store8

    local.get $temp
  )

  (func (export "mem_test_i64i8") (param $a1 i32) (param $a2 i32) (result i64)
    (local $temp i64)
    
    local.get $a2
    local.get $a1
    i64.load8_s
    local.tee $temp
    i64.store8

    local.get $temp
  )

  (func (export "mem_test_u64u16") (param $a1 i32) (param $a2 i32) (result i64)
    (local $temp i64)
    
    local.get $a2
    local.get $a1
    i64.load16_u
    local.tee $temp
    i64.store16

    local.get $temp
  )

  (func (export "mem_test_i64i16") (param $a1 i32) (param $a2 i32) (result i64)
    (local $temp i64)
    
    local.get $a2
    local.get $a1
    i64.load16_s
    local.tee $temp
    i64.store16

    local.get $temp
  )

  (func (export "mem_test_u64u32") (param $a1 i32) (param $a2 i32) (result i64)
    (local $temp i64)
    
    local.get $a2
    local.get $a1
    i64.load32_u
    local.tee $temp
    i64.store32

    local.get $temp
  )

  (func (export "mem_test_i64i32") (param $a1 i32) (param $a2 i32) (result i64)
    (local $temp i64)
    
    local.get $a2
    local.get $a1
    i64.load32_s
    local.tee $temp
    i64.store32

    local.get $temp
  )

  (func (export "mem_test_u64") (param $a1 i32) (param $a2 i32) (result i64)
    (local $temp i64)
    
    local.get $a2
    local.get $a1
    i64.load
    local.tee $temp
    i64.store

    local.get $temp
  )
)
