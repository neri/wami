(module
  (memory 1)

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
)
