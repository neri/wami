use crate::sync::rwlock_nb::*;
use crate::*;
use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::mem::{size_of, transmute};
use core::ops::{Deref, DerefMut};
use core::slice;
use core::sync::atomic::{fence, AtomicU32, Ordering};

/// WebAssembly memory object
pub struct WasmMemory {
    data: RwLockNb<SharedDataStore>,
    size: AtomicU32,
    limit: u32,
}

impl WasmMemory {
    pub const MAX_LIMIT: u32 = 0x1_0000;

    #[inline]
    pub const fn zero() -> Self {
        Self {
            data: RwLockNb::new(SharedDataStore::new()),
            size: AtomicU32::new(0),
            limit: 0,
        }
    }

    #[inline]
    pub fn new(limit: WasmLimit) -> Result<Self, WasmCompileErrorKind> {
        let memory = Self {
            data: RwLockNb::new(SharedDataStore::new()),
            size: AtomicU32::new(0),
            limit: limit.max().unwrap_or(u32::MAX).min(Self::MAX_LIMIT),
        };

        if limit.is_zero() {
            return Ok(memory);
        }

        memory
            .grow(limit.min())
            .map(|_| memory)
            .map_err(|_| WasmCompileErrorKind::OutOfMemory)
    }

    #[inline]
    pub fn try_borrow(
        &self,
    ) -> Result<RwLockNbReadGuard<'_, SharedDataStore>, WasmRuntimeErrorKind> {
        self.data
            .try_read()
            .map_err(|_| WasmRuntimeErrorKind::MemoryBorrowError)
    }

    #[inline]
    pub fn borrowing<F, R>(&self, kernel: F) -> Result<R, WasmRuntimeErrorKind>
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let memory = self.try_borrow()?;
        let result = kernel(memory.as_mut_slice());
        drop(memory);
        Ok(result)
    }

    /// memory.size
    #[inline]
    pub fn size(&self) -> u32 {
        self.size.load(Ordering::Acquire)
    }

    /// memory.grow
    pub fn grow(&self, delta: u32) -> Result<u32, WasmRuntimeErrorKind> {
        if delta > 0 {
            let mut memory = self
                .data
                .try_write()
                .map_err(|_| WasmRuntimeErrorKind::MemoryBorrowError)?;

            let old_len = self.size();
            let new_len = old_len.saturating_add(delta);
            if new_len as u32 > self.limit {
                return Err(WasmRuntimeErrorKind::InvalidParameter);
            }

            let additional = (delta as usize)
                .checked_mul(WebAssembly::PAGE_SIZE)
                .ok_or(WasmRuntimeErrorKind::InvalidParameter)?;
            memory.try_grow(additional)?;

            self.size.store(new_len, Ordering::Release);
            Ok(old_len)
        } else {
            Ok(self.size())
        }
    }

    /// Write slice to memory
    pub fn write_slice(&self, offset: usize, src: &[u8]) -> Result<(), WasmRuntimeErrorKind> {
        let memory = self.try_borrow()?;
        let count = src.len();
        let limit = memory.len();
        let Some(end) = offset.checked_add(count) else {
            return Err(WasmRuntimeErrorKind::OutOfBounds);
        };
        if offset < limit && end <= limit {
            unsafe {
                memory
                    .as_mut_ptr()
                    .add(offset)
                    .copy_from_nonoverlapping(src.as_ptr(), count);
            }
            Ok(())
        } else {
            Err(WasmRuntimeErrorKind::OutOfBounds)
        }
    }

    #[inline]
    pub fn check_bound(base: u64, count: usize, limit: usize) -> Result<(), WasmRuntimeErrorKind> {
        if base.saturating_add(count as u64) <= limit as u64 {
            Ok(())
        } else {
            Err(WasmRuntimeErrorKind::OutOfBounds)
        }
    }

    #[inline]
    pub fn effective_address<T: Sized>(
        offset: u32,
        index: u32,
        limit: usize,
    ) -> Result<usize, WasmRuntimeErrorKind> {
        let base = (offset as u64).wrapping_add(index as u64);
        Self::check_bound(base, size_of::<T>(), limit).map(|_| base as usize)
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct SharedDataStore(UnsafeCell<Vec<u8>>);

impl SharedDataStore {
    #[inline]
    pub const fn new() -> Self {
        Self(UnsafeCell::new(Vec::new()))
    }

    #[inline]
    pub fn as_slice<'a>(&'a self) -> &'a [u8] {
        unsafe { &*self.0.get() }.as_slice()
    }

    #[inline]
    pub fn as_mut_slice<'a>(&'a self) -> &'a mut [u8] {
        unsafe { &mut *self.0.get() }.as_mut_slice()
    }

    pub fn slice<'a, T: Sized>(
        &self,
        base: usize,
        count: usize,
    ) -> Result<&'a [T], WasmRuntimeErrorKind> {
        let slice = self.as_slice();
        let limit = slice.len();
        let size_of_t = size_of::<T>();
        WasmMemory::check_bound(base as u64, count * size_of_t, limit)?;
        Ok(unsafe { slice::from_raw_parts(slice.as_ptr().add(base as usize) as *const T, count) })
    }

    pub fn slice_mut<'a, T: Sized>(
        &self,
        base: usize,
        count: usize,
    ) -> Result<&'a mut [T], WasmRuntimeErrorKind> {
        let slice = self.as_mut_slice();
        let limit = slice.len();
        let size_of_t = size_of::<T>();
        WasmMemory::check_bound(base as u64, count * size_of_t, limit)?;
        Ok(unsafe { slice::from_raw_parts_mut(slice.as_mut_ptr().add(base) as *mut T, count) })
    }

    pub unsafe fn transmute<'a, T: Sized>(
        &self,
        offset: u64,
    ) -> Result<&'a T, WasmRuntimeErrorKind> {
        let slice = self.as_slice();
        let limit = slice.len();
        WasmMemory::check_bound(offset, size_of::<T>(), limit)?;
        Ok(unsafe { transmute(slice.as_ptr().add(offset as usize)) })
    }

    pub unsafe fn transmute_mut<'a, T: Sized>(
        &self,
        offset: u64,
    ) -> Result<&'a mut T, WasmRuntimeErrorKind> {
        let slice = self.as_mut_slice();
        let limit = slice.len();
        WasmMemory::check_bound(offset, size_of::<T>(), limit)?;
        Ok(unsafe { transmute(slice.as_mut_ptr().add(offset as usize)) })
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        unsafe { (&*self.0.get()).as_ptr() }
    }

    #[inline]
    pub fn as_mut_ptr(&self) -> *mut u8 {
        unsafe { (&mut *self.0.get()).as_mut_ptr() }
    }

    #[inline]
    pub fn fence(&self, order: Ordering) {
        fence(order)
    }

    #[inline]
    pub fn len(&self) -> usize {
        unsafe { &*self.0.get() }.len()
    }

    pub fn try_grow(&mut self, additional: usize) -> Result<(), WasmRuntimeErrorKind> {
        let vec = self.0.get_mut();

        let old_size = vec.len();
        let new_size = old_size
            .checked_add(additional)
            .ok_or(WasmRuntimeErrorKind::InvalidParameter)?;

        if vec.try_reserve(additional).is_err() {
            return Err(WasmRuntimeErrorKind::OutOfMemory);
        }
        vec.resize(new_size, 0);

        Ok(())
    }

    #[inline]
    pub fn fill(&self, value: u8) {
        self.as_mut_slice().fill(value);
    }

    #[cfg(test)]
    pub(crate) fn read_i32(&self, offset: usize) -> i32 {
        self.read_u32(offset) as i32
    }

    #[cfg(test)]
    pub(crate) fn read_u32(&self, offset: usize) -> u32 {
        let slice = &self.as_slice()[offset..offset + 4].try_into().unwrap();
        u32::from_le_bytes(*slice)
    }

    #[cfg(test)]
    pub(crate) fn read_f32(&self, offset: usize) -> f32 {
        let slice = &self.as_slice()[offset..offset + 4].try_into().unwrap();
        f32::from_le_bytes(*slice)
    }

    #[cfg(test)]
    pub(crate) fn read_i64(&self, offset: usize) -> i64 {
        self.read_u64(offset) as i64
    }

    #[cfg(test)]
    pub(crate) fn read_u64(&self, offset: usize) -> u64 {
        let slice = &self.as_slice()[offset..offset + 8].try_into().unwrap();
        u64::from_le_bytes(*slice)
    }

    #[cfg(test)]
    pub(crate) fn read_f64(&self, offset: usize) -> f64 {
        let slice = &self.as_slice()[offset..offset + 8].try_into().unwrap();
        f64::from_le_bytes(*slice)
    }
}

impl Deref for SharedDataStore {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for SharedDataStore {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WasmPtr<T> {
    repr: u32,
    _phantom: PhantomData<*const T>,
}

impl<T> WasmPtr<T> {
    #[inline]
    pub const fn from_u32(value: u32) -> Self {
        Self {
            repr: value,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub const fn as_u32(&self) -> u32 {
        self.repr
    }
}

impl<T> From<WasmPtr<T>> for WasmValue {
    #[inline]
    fn from(value: WasmPtr<T>) -> Self {
        WasmValue::I32(value.repr as i32)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WasmPtrMut<T> {
    repr: u32,
    _phantom: PhantomData<*mut T>,
}

impl<T> WasmPtrMut<T> {
    #[inline]
    pub const fn from_u32(value: u32) -> Self {
        Self {
            repr: value,
            _phantom: PhantomData,
        }
    }

    #[inline]
    pub const fn as_u32(&self) -> u32 {
        self.repr
    }
}

impl<T> From<WasmPtrMut<T>> for WasmPtr<T> {
    #[inline]
    fn from(value: WasmPtrMut<T>) -> Self {
        Self {
            repr: value.repr,
            _phantom: PhantomData,
        }
    }
}

impl<T> From<WasmPtrMut<T>> for WasmValue {
    #[inline]
    fn from(value: WasmPtrMut<T>) -> Self {
        WasmValue::I32(value.repr as i32)
    }
}
