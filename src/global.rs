//! WebAssembly global variables
use crate::*;
use crate::{sync::rwlock_nb::RwLockNb, WasmRuntimeErrorKind};
use core::sync::atomic::{AtomicI32, AtomicI64, Ordering};

pub trait WasmGlobalProp<T>
where
    T: Copy,
{
    fn get(&self) -> Result<T, WasmRuntimeErrorKind>;
}

pub trait WasmGlobalPropMut<T>: WasmGlobalProp<T>
where
    T: Copy,
{
    fn set(&self, value: T) -> Result<(), WasmRuntimeErrorKind>;
}

pub struct WasmGlobalFixedValue<T> {
    value: T,
}

impl<T> WasmGlobalFixedValue<T> {
    #[inline]
    pub const fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T: Copy> WasmGlobalProp<T> for WasmGlobalFixedValue<T> {
    #[inline]
    fn get(&self) -> Result<T, WasmRuntimeErrorKind> {
        Ok(self.value)
    }
}

pub struct WasmGlobalMutableValue<T> {
    value: RwLockNb<T>,
}

impl<T> WasmGlobalMutableValue<T> {
    #[inline]
    pub const fn new(value: T) -> Self {
        Self {
            value: RwLockNb::new(value),
        }
    }
}

impl<T: Copy> WasmGlobalProp<T> for WasmGlobalMutableValue<T> {
    #[inline]
    fn get(&self) -> Result<T, WasmRuntimeErrorKind> {
        self.value
            .try_read()
            .map(|v| *v)
            .map_err(|_| WasmRuntimeErrorKind::WouldBlock)
    }
}

impl<T: Copy> WasmGlobalPropMut<T> for WasmGlobalMutableValue<T> {
    #[inline]
    fn set(&self, value: T) -> Result<(), WasmRuntimeErrorKind> {
        self.value
            .try_write()
            .map(|mut v| {
                *v = value;
            })
            .map_err(|_| WasmRuntimeErrorKind::WouldBlock)
    }
}

macro_rules! decl_wasm_global_atomics {
    ($class_name:ident, $val_type:ident, $atomic_type:ident) => {
        pub struct $class_name {
            value: $atomic_type,
        }

        impl $class_name {
            #[inline]
            pub const fn new(value: $val_type) -> Self {
                Self {
                    value: $atomic_type::new(value),
                }
            }
        }

        impl WasmGlobalProp<$val_type> for $class_name {
            #[inline]
            fn get(&self) -> Result<$val_type, WasmRuntimeErrorKind> {
                Ok(self.value.load(Ordering::Relaxed))
            }
        }

        impl WasmGlobalPropMut<$val_type> for $class_name {
            #[inline]
            fn set(&self, value: $val_type) -> Result<(), WasmRuntimeErrorKind> {
                self.value.store(value, Ordering::SeqCst);
                Ok(())
            }
        }
    };
}

decl_wasm_global_atomics!(WasmGlobalI32, i32, AtomicI32);
decl_wasm_global_atomics!(WasmGlobalI64, i64, AtomicI64);

/// WebAssembly global variable
pub enum WasmGlobal {
    I32(Box<dyn WasmGlobalProp<i32>>),
    I64(Box<dyn WasmGlobalProp<i64>>),
    F32(Box<dyn WasmGlobalProp<f32>>),
    F64(Box<dyn WasmGlobalProp<f64>>),
    I32Mut(Box<dyn WasmGlobalPropMut<i32>>),
    I64Mut(Box<dyn WasmGlobalPropMut<i64>>),
    F32Mut(Box<dyn WasmGlobalPropMut<f32>>),
    F64Mut(Box<dyn WasmGlobalPropMut<f64>>),
}

impl WasmGlobal {
    #[inline]
    pub fn new(val: WasmValue, is_mutable: bool) -> Self {
        if is_mutable {
            Self::with_mut(val)
        } else {
            Self::with_const(val)
        }
    }

    #[inline]
    pub fn with_const(val: WasmValue) -> Self {
        match val {
            WasmValue::I32(v) => Self::I32(Box::new(WasmGlobalI32::new(v))),
            WasmValue::I64(v) => Self::I64(Box::new(WasmGlobalI64::new(v))),
            WasmValue::F32(v) => Self::F32(Box::new(WasmGlobalFixedValue::new(v))),
            WasmValue::F64(v) => Self::F64(Box::new(WasmGlobalFixedValue::new(v))),
        }
    }

    #[inline]
    pub fn with_mut(val: WasmValue) -> Self {
        match val {
            WasmValue::I32(v) => Self::I32Mut(Box::new(WasmGlobalI32::new(v))),
            WasmValue::I64(v) => Self::I64Mut(Box::new(WasmGlobalI64::new(v))),
            WasmValue::F32(v) => Self::F32Mut(Box::new(WasmGlobalMutableValue::new(v))),
            WasmValue::F64(v) => Self::F64Mut(Box::new(WasmGlobalMutableValue::new(v))),
        }
    }

    #[inline]
    pub const fn val_type(&self) -> WasmValType {
        match self {
            Self::I32(_) | Self::I32Mut(_) => WasmValType::I32,
            Self::I64(_) | Self::I64Mut(_) => WasmValType::I64,
            Self::F32(_) | Self::F32Mut(_) => WasmValType::F32,
            Self::F64(_) | Self::F64Mut(_) => WasmValType::F64,
        }
    }

    #[inline]
    pub const fn is_mutable(&self) -> bool {
        match self {
            Self::I32(_) | Self::I64(_) | Self::F32(_) | Self::F64(_) => false,
            Self::I32Mut(_) | Self::I64Mut(_) | Self::F32Mut(_) | Self::F64Mut(_) => true,
        }
    }

    #[inline]
    pub fn get_i32(&self) -> Result<i32, WasmRuntimeErrorKind> {
        self.get()
    }

    #[inline]
    pub fn get_i64(&self) -> Result<i64, WasmRuntimeErrorKind> {
        self.get()
    }

    #[inline]
    pub fn get_f32(&self) -> Result<f32, WasmRuntimeErrorKind> {
        self.get()
    }

    #[inline]
    pub fn get_f64(&self) -> Result<f64, WasmRuntimeErrorKind> {
        self.get()
    }
}

pub trait Get<T> {
    fn get(&self) -> Result<T, WasmRuntimeErrorKind>;
}

pub trait GetMut<T> {
    fn get_mut(&self) -> Option<&dyn WasmGlobalPropMut<T>>;
}

macro_rules! impl_get_set {
    ($type:ident, $arm_const:ident, $arm_mut:ident) => {
        impl Get<$type> for WasmGlobal {
            fn get(&self) -> Result<$type, WasmRuntimeErrorKind> {
                match self {
                    Self::$arm_const(v) => v.get(),
                    Self::$arm_mut(v) => v.get(),
                    _ => Err(WasmRuntimeErrorKind::TypeMismatch),
                }
            }
        }

        impl GetMut<$type> for WasmGlobal {
            fn get_mut(&self) -> Option<&dyn WasmGlobalPropMut<$type>> {
                match self {
                    Self::$arm_mut(v) => Some(v.as_ref()),
                    _ => None,
                }
            }
        }
    };
}

impl_get_set!(i32, I32, I32Mut);
impl_get_set!(i64, I64, I64Mut);
impl_get_set!(f32, F32, F32Mut);
impl_get_set!(f64, F64, F64Mut);
