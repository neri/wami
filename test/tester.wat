(module
  ;; test cases
  (import "env" "add" (func $env_add (param i32) (param i32) (result i32)))
  (import "env" "sub" (func $env_sub (param i32) (param i32) (result i32)))

  (memory 1)

  (global $stack_pointer (export "__stack_pointer") (mut i32) (i32.const 123))
  (global $global1 (export "global1") (mut i32) (i32.const 123))

  (table 10 funcref)
  (elem (i32.const 1) $elem1 $elem2 $elem3 $call_indirect_test)

  ;; fn local_test() -> i32
  (func $local_test (export "local_test") (result i32)
    (local $i i32)
    (local $j i32)
    i32.const 123
    local.set $i
    i32.const 456
    local.set $j
    local.get $i
  )

  ;; fn global_add(v: i32) -> i32
  (func $global_add (export "global_add") (param i32) (result i32)
    global.get $global1
    local.get 0
    i32.add
    global.set $global1
    global.get $global1
  )

  ;; fn fib(v: i32) -> i32
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

  ;; fn fact(v: i32) -> i32
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

  ;; fn test_unary_i32(v: i32) -> i32
  (func $test_unary_i32 (export "test_unary_i32") (param i32) (result i32)
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

  ;; fn test_unary_i64(v: i64) -> i64
  (func $test_unary_i64 (export "test_unary_i64") (param i64) (result i32)
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

  ;; fn test_bin_i32(lhs: i32, rhs: i32) -> i32
  (func $test_bin_i32 (export "test_bin_i32") (param $lhs i32) (param $rhs i32) (result i32)
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

  ;; fn test_bin_i64(lhs: i64, rhs: i64) -> i64
  (func $test_bin_i64 (export "test_bin_i64") (param $lhs i64) (param $rhs i64) (result i32)
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

  ;; fn call_test1(a1: i32, a2: i32, a3: i64, a4: i64) -> i32
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

  ;; fn call_test2(a1: i32, a2: i32, a2: i64, a4: i64) -> i32
  (func $call_test2 (export "call_test2") (param $a1 i32) (param $a2 i32) (param $a3 i64) (param $a4 i64) (result i32)
    (local $i i32)

    i64.const 0
    i64.const 0
    i64.const 0
    i64.const 0

    local.get $a2
    local.get $a1
    local.get $a4
    local.get $a3

    i64.const 123456789
    i64.const 987654321
    i64.const 0x5555aaaa5555aaaa
    i64.const 0xaaaa5555aaaa5555

    drop
    drop
    drop
    drop

    call $call_test1
    local.set $i

    drop
    drop
    drop
    drop

    local.get $i
    local.get $a1
    i32.add
  )

  ;; fn call_test3(a1: i32, a2: i32, a3: i64, a4: i64) -> i64
  (func $call_test3 (export "call_test3") (param $a1 i32) (param $a2 i32) (param $a3 i64) (param $a4 i64) (result i64)
    local.get $a2
    local.get $a1
    local.get $a4
    local.get $a3
    call $call_test2
    drop

    local.get $a3
  )

  ;; fn call_test4(a1: i32, a2: i32, a3: i64, a4: i64) -> i64
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

  ;; fn mem_test_u32u8(a1: &u8, a2: &mut u8) -> &mut u8
  (func $mem_test_u32u8 (export "mem_test_u32u8") (param $a1 i32) (param $a2 i32) (result i32)
    (local $temp i32)
    
    local.get $a2
    local.get $a1
    i32.load8_u
    local.tee $temp
    i32.store8

    local.get $temp
  )

  ;; fn mem_test_u32i8(a1: &i8, a2: &mut i8) -> &mut i8
  (func $mem_test_i32i8 (export "mem_test_i32i8") (param $a1 i32) (param $a2 i32) (result i32)
    (local $temp i32)
    
    local.get $a2
    local.get $a1
    i32.load8_s
    local.tee $temp
    i32.store8

    local.get $temp
  )

  ;; fn mem_test_u32u16(a1: &u16, a2: &mut u16) -> &mut u16
  (func $mem_test_u32u16 (export "mem_test_u32u16") (param $a1 i32) (param $a2 i32) (result i32)
    (local $temp i32)
    
    local.get $a2
    local.get $a1
    i32.load16_u
    local.tee $temp
    i32.store16

    local.get $temp
  )

  ;; fn mem_test_u32i16(a1: &i16, a2: &mut i16) -> &mut i16
  (func $mem_test_i32i16 (export "mem_test_i32i16") (param $a1 i32) (param $a2 i32) (result i32)
    (local $temp i32)
    
    local.get $a2
    local.get $a1
    i32.load16_s
    local.tee $temp
    i32.store16

    local.get $temp
  )

  ;; fn mem_test_u32(a1: &u32, a2: &mut u32) -> &mut u32
  (func $mem_test_u32 (export "mem_test_u32") (param $a1 i32) (param $a2 i32) (result i32)
    (local $temp i32)
    
    local.get $a2
    local.get $a1
    i32.load
    local.tee $temp
    i32.store

    local.get $temp
  )

  ;; fn mem_test_u64u8(a1: &u8, a2: &mut u8) -> &mut u8
  (func $mem_test_u64u8 (export "mem_test_u64u8") (param $a1 i32) (param $a2 i32) (result i64)
    (local $temp i64)
    
    local.get $a2
    local.get $a1
    i64.load8_u
    local.tee $temp
    i64.store8

    local.get $temp
  )

  ;; fn mem_test_u64i8(a1: &i8, a2: &mut i8) -> &mut i8
  (func $mem_test_i64i8 (export "mem_test_i64i8") (param $a1 i32) (param $a2 i32) (result i64)
    (local $temp i64)
    
    local.get $a2
    local.get $a1
    i64.load8_s
    local.tee $temp
    i64.store8

    local.get $temp
  )

  ;; fn mem_test_u64u16(a1: &u16, a2: &mut u16) -> &mut u16
  (func $mem_test_u64u16 (export "mem_test_u64u16") (param $a1 i32) (param $a2 i32) (result i64)
    (local $temp i64)
    
    local.get $a2
    local.get $a1
    i64.load16_u
    local.tee $temp
    i64.store16

    local.get $temp
  )

  ;; fn mem_test_u64i16(a1: &i16, a2: &mut i16) -> &mut i16
  (func $mem_test_i64i16 (export "mem_test_i64i16") (param $a1 i32) (param $a2 i32) (result i64)
    (local $temp i64)
    
    local.get $a2
    local.get $a1
    i64.load16_s
    local.tee $temp
    i64.store16

    local.get $temp
  )

  ;; fn mem_test_u64u32(a1: &u32, a2: &mut u32) -> &mut u32
  (func $mem_test_u64u32 (export "mem_test_u64u32") (param $a1 i32) (param $a2 i32) (result i64)
    (local $temp i64)
    
    local.get $a2
    local.get $a1
    i64.load32_u
    local.tee $temp
    i64.store32

    local.get $temp
  )

  ;; fn mem_test_u64i32(a1: &i32, a2: &mut i32) -> &mut i32
  (func $mem_test_i64i32 (export "mem_test_i64i32") (param $a1 i32) (param $a2 i32) (result i64)
    (local $temp i64)
    
    local.get $a2
    local.get $a1
    i64.load32_s
    local.tee $temp
    i64.store32

    local.get $temp
  )

  ;; fn mem_test_u64(a1: &u64, a2: &mut u64) -> &mut u64
  (func $mem_test_u64 (export "mem_test_u64") (param $a1 i32) (param $a2 i32) (result i64)
    (local $temp i64)
    
    local.get $a2
    local.get $a1
    i64.load
    local.tee $temp
    i64.store

    local.get $temp
  )

  ;; fn mem_test_size() -> i32
  (func $mem_test_size (export "mem_test_size") (result i32)
    memory.size
  )

  ;; fn mem_test_grow(v: i32) -> i32
  (func $mem_test_grow (export "mem_test_grow") (param $v i32) (result i32)
    local.get $v
    memory.grow
  )

  ;; fn mem_test_fill(d: *mut c_void, v: u8, n: usize)
  (func $mem_test_fill (export "mem_test_fill") (param $d i32) (param $v i32) (param $n i32)
    local.get $d
    local.get $v
    local.get $n
    memory.fill
  )

  ;; fn mem_test_copy(d: *mut c_void, s: *const c_void, n: usize)
  (func $mem_test_copy (export "mem_test_copy") (param $d i32) (param $s i32) (param $n i32)
    local.get $d
    local.get $s
    local.get $n
    memory.copy
  )

  ;; fn test_unary_f32(fval: f32, i32val: i32, u32val: u32, i64val: i64, u64val: u64) -> i32
  (func $test_unary_f32 (export "test_unary_f32") (param $fval f32) (param $i32val i32) (param $u32val i32) (param $i64val i64) (param $u64val i64) (result i32)
    (local $p i32)

    i32.const 0x10
    local.get $fval
    i32.trunc_f32_s
    i32.store

    i32.const 0x14
    local.get $fval
    i32.trunc_f32_u
    i32.store

    i32.const 0x18
    local.get $fval
    i32.trunc_sat_f32_s
    i32.store

    i32.const 0x1C
    local.get $fval
    i32.trunc_sat_f32_u
    i32.store

    i32.const 0x20
    local.get $fval
    i64.trunc_f32_s
    i64.store

    i32.const 0x28
    local.get $fval
    i64.trunc_f32_u
    i64.store

    i32.const 0x30
    local.get $fval
    i64.trunc_sat_f32_s
    i64.store

    i32.const 0x38
    local.get $fval
    i64.trunc_sat_f32_u
    i64.store

    i32.const 0x40
    local.get $i32val
    f32.convert_i32_s
    f32.store

    i32.const 0x44
    local.get $u32val
    f32.convert_i32_u
    f32.store

    i32.const 0x48
    local.get $i64val
    f32.convert_i64_s
    f32.store

    i32.const 0x4C
    local.get $u64val
    f32.convert_i64_u
    f32.store

    i32.const 0x50
    local.get $fval
    f64.promote_f32
    f64.store

    i32.const 0x80
    local.tee $p
    local.get $fval
    f32.abs
    f32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $fval
    f32.neg
    f32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $fval
    f32.ceil
    f32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $fval
    f32.floor
    f32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $fval
    f32.trunc
    f32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $fval
    f32.nearest
    f32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $fval
    f32.sqrt
    f32.store

    local.get $p
  )

  ;; fn test_bin_f32(lhs: f32, rhs: f32) -> i32
  (func $test_bin_f32 (export "test_bin_f32") (param $lhs f32) (param $rhs f32) (result i32)
    (local $p i32)

    i32.const 0x10
    local.tee $p
    local.get $lhs
    local.get $rhs
    f32.eq
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f32.ne
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f32.lt
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f32.gt
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f32.le
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f32.ge
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f32.add
    f32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f32.sub
    f32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f32.mul
    f32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f32.div
    f32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f32.copysign
    f32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f32.min
    f32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f32.max
    f32.store

    local.get $p
  )

  ;; fn test_unary_f64(fval: f64, i32val: i32, u32val: u32, i64val: i64, u64val: u64) -> i32
  (func $test_unary_f64 (export "test_unary_f64") (param $fval f64) (param $i32val i32) (param $u32val i32) (param $i64val i64) (param $u64val i64) (result i32)
    (local $p i32)

    i32.const 0x10
    local.get $fval
    i32.trunc_f64_s
    i32.store

    i32.const 0x14
    local.get $fval
    i32.trunc_f64_u
    i32.store

    i32.const 0x18
    local.get $fval
    i32.trunc_sat_f64_s
    i32.store

    i32.const 0x1C
    local.get $fval
    i32.trunc_sat_f64_u
    i32.store

    i32.const 0x20
    local.get $fval
    i64.trunc_f64_s
    i64.store

    i32.const 0x28
    local.get $fval
    i64.trunc_f64_u
    i64.store

    i32.const 0x30
    local.get $fval
    i64.trunc_sat_f64_s
    i64.store

    i32.const 0x38
    local.get $fval
    i64.trunc_sat_f64_u
    i64.store

    i32.const 0x40
    local.get $i32val
    f64.convert_i32_s
    f64.store

    i32.const 0x48
    local.get $u32val
    f64.convert_i32_u
    f64.store

    i32.const 0x50
    local.get $i64val
    f64.convert_i64_s
    f64.store

    i32.const 0x58
    local.get $u64val
    f64.convert_i64_u
    f64.store

    i32.const 0x60
    local.get $fval
    f32.demote_f64
    f32.store

    i32.const 0x80
    local.tee $p
    local.get $fval
    f64.abs
    f64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $fval
    f64.neg
    f64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $fval
    f64.ceil
    f64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $fval
    f64.floor
    f64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $fval
    f64.trunc
    f64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $fval
    f64.nearest
    f64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $fval
    f64.sqrt
    f64.store

    local.get $p
  )

  ;; fn test_bin_f64(lhs: f64, rhs: f64) -> i32
  (func $test_bin_f64 (export "test_bin_f64") (param $lhs f64) (param $rhs f64) (result i32)
    (local $p i32)

    i32.const 0x10
    local.tee $p
    local.get $lhs
    local.get $rhs
    f64.eq
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f64.ne
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f64.lt
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f64.gt
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f64.le
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f64.ge
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f64.add
    f64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f64.sub
    f64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f64.mul
    f64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f64.div
    f64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f64.copysign
    f64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f64.min
    f64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $lhs
    local.get $rhs
    f64.max
    f64.store

    local.get $p
  )

  ;; fn block_test(cc: i32, a1: i32, a2: i32, a3: i32) -> i32
  (func $block_test (export "block_test") (param $cc i32) (param $a1 i32) (param $a2 i32) (param $a3 i32) (result i32)
    block $top (result i32)
      local.get $a1
      local.get $cc
      i32.eqz
      br_if $top
      drop
      block (result i32)
        local.get $a2
        local.get $cc
        i32.const 1
        i32.eq
        br_if $top
        drop
        block (result i32)
          local.get $a3
          br $top
        end
      end
    end
  )

  ;; fn loop_test(a1: i32) -> i32
  (func $loop_test (export "loop_test") (param $a1 i32) (result i32)
    (local $i i32)
    (local $acc i32)
    i32.const 0
    local.set $i
    block $top (result i32)
      loop $loop (result i32)
        local.get $acc
        local.get $i
        local.get $a1
        i32.ge_s
        br_if $top
        local.get $i
        i32.const 1
        i32.add
        local.tee $i
        i32.add
        local.set $acc
        br $loop
      end
    end
  )

  ;; fn if_test1(lhs: i32, rhs: i32, cc: bool) -> i32
  (func $if_test1 (export "if_test1") (param $lhs i32) (param $rhs i32) (param $cc i32) (result i32)
    (local $i i32)
    local.get $cc
    if
      local.get $lhs
      local.set $i
    else
      local.get $rhs
      local.set $i
    end
    local.get $i
  )

  ;; fn if_test2(lhs: i32, rhs: i32, cc: bool) -> i32
  (func $if_test2 (export "if_test2") (param $lhs i32) (param $rhs i32) (param $cc i32) (result i32)
    local.get $cc
    if (result i32)
      local.get $lhs
    else
      local.get $rhs
    end
  )

  ;; fn import_test1(a0: i32, a1: i32) -> i32
  (func $import_test1 (export "import_test1") (param $a0 i32) (param $a1 i32) (result i32)
    local.get $a0
    local.get $a1
    call $env_add
  )

  ;; fn import_test2(a0: i32, a1: i32) -> i32
  (func $import_test2 (export "import_test2") (param $a0 i32) (param $a1 i32) (result i32)
    local.get $a0
    local.get $a1
    call $env_sub
  )

  ;; fn call_indirect_test(sel: i32, a1: i32) -> i32
  (func $call_indirect_test (export "call_indirect_test") (param $sel i32) (param $a1 i32) (result i32)
    local.get $a1
    local.get $sel
    call_indirect (param i32) (result i32)
  )

  (func $elem1 (param $a0 i32) (result i32)
    i32.const 123
    local.get $a0
    i32.add
  )

  (func $elem2 (param $a0 i32) (result i32)
    local.get $a0
    i32.const 456
    i32.sub
  )

  (func $elem3 (param $a0 i32) (result i32)
    i32.const 789
    local.get $a0
    i32.add
  )

  ;; fn test_fusion_unary_i32(a0: i32) -> i32
  (func $test_fusion_unary_i32 (export "test_fusion_unary_i32") (param $a0 i32) (result i32)
    (local $p i32)
    (local $i i32)

    ;; i32.const N; local.set -> FusedI32SetConst
    i32.const 0x10
    local.tee $p
    i32.const 0x12345678
    local.set $i
    i64.const 0xcccccccc
    drop
    local.get $i
    i32.store

    ;; i32.const N; i32.add -> FusedI32AddI
    ;; i32.const N; i32.sub -> FusedI32AddI (sign reversed)
    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $a0
    i32.const 1234
    i32.add
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $a0
    i32.const 1234
    i32.sub
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $a0
    i32.const 5678
    i32.add
    i32.store

    ;; i32.const N; i32.and -> FusedI32AndI
    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $a0
    i32.const 0x55555555
    i32.and
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $a0
    i32.const 0xaaaaaaaa
    i32.and
    i32.store

    ;; i32.const N; i32.or -> FusedI32OrI
    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $a0
    i32.const 0x55555555
    i32.or
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $a0
    i32.const 0xaaaaaaaa
    i32.or
    i32.store

    ;; i32.const N; i32.xor -> FusedI32XorI
    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $a0
    i32.const 0x55555555
    i32.xor
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $a0
    i32.const 0xaaaaaaaa
    i32.xor
    i32.store

    ;; i32.const N; i32.shl -> FusedI32ShlI
    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $a0
    i32.const 7
    i32.shl
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $a0
    i32.const 19
    i32.shl
    i32.store

    ;; i32.const N; i32.shr_s -> FusedI32ShrSI
    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $a0
    i32.const 5
    i32.shr_s
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $a0
    i32.const 17
    i32.shr_s
    i32.store

    ;; i32.const N; i32.shr_s -> FusedI32ShrUI
    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $a0
    i32.const 3
    i32.shr_u
    i32.store

    local.get $p
    i32.const 4
    i32.add
    local.tee $p
    local.get $a0
    i32.const 13
    i32.shr_u
    i32.store

    ;; i32.eqz; br_if -> FusedI32BrZ
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $a0
        i32.eqz
        br_if 0
        local.get $p
        i32.const 5678
        i32.store
        br 1
      end
      local.get $p
      i32.const 1234
      i32.store
    end

    local.get $p
  )

  ;; fn test_fusion_binary_i32(lhs: i32, rhs: i32) -> i32
  (func $test_fusion_binary_i32 (export "test_fusion_binary_i32") (param $lhs i32) (param $rhs i32) (result i32)
    (local $p i32)

    ;; i32.eq; br_if -> FusedI32BrEq
    i32.const 0x10
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i32.eq
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i32.ne; br_if -> FusedI32BrNe
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i32.ne
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i32.lt_s; br_if -> FusedI32BrLtS
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i32.lt_s
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i32.lt_u; br_if -> FusedI32BrLtU
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i32.lt_u
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i32.gt_s; br_if -> FusedI32BrGtS
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i32.gt_s
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i32.gt_u; br_if -> FusedI32BrGtU
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i32.gt_u
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i32.le_s; br_if -> FusedI32BrLeS
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i32.le_s
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i32.le_u; br_if -> FusedI32BrLeU
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i32.le_u
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i32.ge_s; br_if -> FusedI32BrGeS
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i32.ge_s
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i32.ge_u; br_if -> FusedI32BrGeU
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i32.ge_u
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    local.get $p
  )

  ;; fn test_fused_i64(a0: i64) -> i32
  (func $test_fused_i64 (export "test_fused_i64") (param $a0 i64) (result i32)
    (local $p i32)
    (local $l i64)

    ;; i64.const N; local.set -> FusedI64SetConst
    i32.const 0x10
    local.tee $p
    i64.const 0x12345678
    local.set $l
    i64.const 0xcccccccc
    drop
    local.get $l
    i64.store

    ;; i64.const N; i64.add -> FusedI64AddI
    ;; i64.const N; i64.sub -> FusedI64AddI (sign reversed)
    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $a0
    i64.const 12345678
    i64.add
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $a0
    i64.const 12345678
    i64.sub
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $a0
    i64.const 987654321
    i64.add
    i64.store

    ;; i64.const N; i64.and -> FusedI64AndI
    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $a0
    i64.const 0x5555555555555555
    i64.and
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $a0
    i64.const 0xaaaaaaaaaaaaaaaa
    i64.and
    i64.store

    ;; i64.const N; i64.or -> FusedI64OrI
    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $a0
    i64.const 0x5555555555555555
    i64.or
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $a0
    i64.const 0xaaaaaaaaaaaaaaaa
    i64.or
    i64.store

    ;; i64.const N; i64.xor -> FusedI64XorI
    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $a0
    i64.const 0x5555555555555555
    i64.xor
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $a0
    i64.const 0xaaaaaaaaaaaaaaaa
    i64.xor
    i64.store

    ;; i64.const N; i64.shl -> FusedI64ShlI
    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $a0
    i64.const 7
    i64.shl
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $a0
    i64.const 19
    i64.shl
    i64.store

    ;; i64.const N; i64.shr_s -> FusedI64ShrSI
    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $a0
    i64.const 5
    i64.shr_s
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $a0
    i64.const 17
    i64.shr_s
    i64.store

    ;; i64.const N; i64.shr_s -> FusedI64ShrUI
    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $a0
    i64.const 3
    i64.shr_u
    i64.store

    local.get $p
    i32.const 8
    i32.add
    local.tee $p
    local.get $a0
    i64.const 13
    i64.shr_u
    i64.store

    ;; i64.eqz; br_if -> FusedI64BrZ
    local.get $p
    i32.const 8
    i32.add
    local.set $p
    block
      block
        local.get $a0
        i64.eqz
        br_if 0
        local.get $p
        i32.const 5678
        i32.store
        br 1
      end
      local.get $p
      i32.const 1234
      i32.store
    end

    local.get $p
  )

  ;; fn test_fusion_binary_i64(lhs: i64, rhs: i64) -> i32
  (func $test_fusion_binary_i64 (export "test_fusion_binary_i64") (param $lhs i64) (param $rhs i64) (result i32)
    (local $p i32)

    ;; i64.eq; br_if -> FusedI64BrEq
    i32.const 0x10
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i64.eq
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i64.ne; br_if -> FusedI64BrNe
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i64.ne
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i64.lt_s; br_if -> FusedI64BrLtS
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i64.lt_s
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i64.lt_u; br_if -> FusedI64BrLtU
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i64.lt_u
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i64.gt_s; br_if -> FusedI64BrGtS
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i64.gt_s
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i64.gt_u; br_if -> FusedI64BrGtU
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i64.gt_u
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i64.le_s; br_if -> FusedI64BrLeS
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i64.le_s
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i64.le_u; br_if -> FusedI64BrLeU
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i64.le_u
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i64.ge_s; br_if -> FusedI64BrGeS
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i64.ge_s
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    ;; i64.ge_u; br_if -> FusedI64BrGeU
    local.get $p
    i32.const 4
    i32.add
    local.set $p
    block
      block
        local.get $lhs
        local.get $rhs
        i64.ge_u
        br_if 0
        local.get $p
        i32.const 456
        i32.store
        br 1
      end
      local.get $p
      i32.const 123
      i32.store
    end

    local.get $p
  )

)
