//! Little Endian Base 128
#![cfg_attr(not(test), no_std)]

extern crate alloc;

#[cfg(test)]
mod tests;

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;
use core::mem::size_of_val;
use core::str;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WriteError {
    OutOfMemory,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReadError {
    InvalidData,
    UnexpectedEof,
    OutOfBounds,
}

pub struct Leb128Writer {
    inner: Vec<u8>,
}

impl Leb128Writer {
    #[inline]
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        self.inner.as_slice()
    }

    #[inline]
    pub fn into_vec(self) -> Vec<u8> {
        self.inner
    }

    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), WriteError> {
        let additional: usize = bytes.len();
        if self.inner.capacity() - self.inner.len() < additional {
            self.inner
                .try_reserve(additional)
                .map_err(|_| WriteError::OutOfMemory)?;
        }

        self.inner.extend_from_slice(bytes);

        Ok(())
    }

    pub fn write_unsigned(&mut self, value: u64) -> Result<(), WriteError> {
        let bits = (size_of_val(&value) * 8) - value.leading_zeros() as usize;

        if bits <= 7 {
            let bytes = [value as u8];
            self.write_bytes(&bytes)
        } else if bits <= 14 {
            let bytes = [(value | 0x80) as u8, (value >> 7) as u8];
            self.write_bytes(&bytes)
        } else if bits <= 21 {
            let bytes = [
                (value | 0x80) as u8,
                ((value >> 7) | 0x80) as u8,
                (value >> 14) as u8,
            ];
            self.write_bytes(&bytes)
        } else if bits <= 28 {
            let bytes = [
                (value | 0x80) as u8,
                ((value >> 7) | 0x80) as u8,
                ((value >> 14) | 0x80) as u8,
                (value >> 21) as u8,
            ];
            self.write_bytes(&bytes)
        } else if bits <= 35 {
            let bytes = [
                (value | 0x80) as u8,
                ((value >> 7) | 0x80) as u8,
                ((value >> 14) | 0x80) as u8,
                ((value >> 21) | 0x80) as u8,
                (value >> 28) as u8,
            ];
            self.write_bytes(&bytes)
        } else if bits <= 42 {
            let bytes = [
                (value | 0x80) as u8,
                ((value >> 7) | 0x80) as u8,
                ((value >> 14) | 0x80) as u8,
                ((value >> 21) | 0x80) as u8,
                ((value >> 28) | 0x80) as u8,
                (value >> 35) as u8,
            ];
            self.write_bytes(&bytes)
        } else if bits <= 49 {
            let bytes = [
                (value | 0x80) as u8,
                ((value >> 7) | 0x80) as u8,
                ((value >> 14) | 0x80) as u8,
                ((value >> 21) | 0x80) as u8,
                ((value >> 28) | 0x80) as u8,
                ((value >> 35) | 0x80) as u8,
                (value >> 42) as u8,
            ];
            self.write_bytes(&bytes)
        } else if bits <= 56 {
            let bytes = [
                (value | 0x80) as u8,
                ((value >> 7) | 0x80) as u8,
                ((value >> 14) | 0x80) as u8,
                ((value >> 21) | 0x80) as u8,
                ((value >> 28) | 0x80) as u8,
                ((value >> 35) | 0x80) as u8,
                ((value >> 42) | 0x80) as u8,
                (value >> 49) as u8,
            ];
            self.write_bytes(&bytes)
        } else if bits <= 63 {
            let bytes = [
                (value | 0x80) as u8,
                ((value >> 7) | 0x80) as u8,
                ((value >> 14) | 0x80) as u8,
                ((value >> 21) | 0x80) as u8,
                ((value >> 28) | 0x80) as u8,
                ((value >> 35) | 0x80) as u8,
                ((value >> 42) | 0x80) as u8,
                ((value >> 49) | 0x80) as u8,
                (value >> 56) as u8,
            ];
            self.write_bytes(&bytes)
        } else {
            let bytes = [
                (value | 0x80) as u8,
                ((value >> 7) | 0x80) as u8,
                ((value >> 14) | 0x80) as u8,
                ((value >> 21) | 0x80) as u8,
                ((value >> 28) | 0x80) as u8,
                ((value >> 35) | 0x80) as u8,
                ((value >> 42) | 0x80) as u8,
                ((value >> 49) | 0x80) as u8,
                ((value >> 56) | 0x80) as u8,
                (value >> 63) as u8,
            ];
            self.write_bytes(&bytes)
        }
    }

    pub fn write_fixed_u32(&mut self, value: u32) -> Result<(), WriteError> {
        let bytes = [
            (value | 0x80) as u8,
            ((value >> 7) | 0x80) as u8,
            ((value >> 14) | 0x80) as u8,
            ((value >> 21) | 0x80) as u8,
            (value >> 28) as u8,
        ];
        self.write_bytes(&bytes)
    }

    pub fn write_fixed_u64(&mut self, value: u64) -> Result<(), WriteError> {
        let bytes = [
            (value | 0x80) as u8,
            ((value >> 7) | 0x80) as u8,
            ((value >> 14) | 0x80) as u8,
            ((value >> 21) | 0x80) as u8,
            ((value >> 28) | 0x80) as u8,
            ((value >> 35) | 0x80) as u8,
            ((value >> 42) | 0x80) as u8,
            ((value >> 49) | 0x80) as u8,
            ((value >> 56) | 0x80) as u8,
            (value >> 63) as u8,
        ];
        self.write_bytes(&bytes)
    }

    pub fn write_signed(&mut self, value: i64) -> Result<(), WriteError> {
        let bits = (size_of_val(&value) * 8)
            - if value < 0 {
                value.leading_ones() as usize
            } else {
                value.leading_zeros() as usize
            };

        if bits < 7 {
            let bytes = [(value & 0x7F) as u8];
            self.write_bytes(&bytes)
        } else if bits < 14 {
            let bytes = [(value | 0x80) as u8, ((value >> 7) & 0x7F) as u8];
            self.write_bytes(&bytes)
        } else if bits < 21 {
            let bytes = [
                (value | 0x80) as u8,
                ((value >> 7) | 0x80) as u8,
                ((value >> 14) & 0x7F) as u8,
            ];
            self.write_bytes(&bytes)
        } else if bits < 28 {
            let bytes = [
                (value | 0x80) as u8,
                ((value >> 7) | 0x80) as u8,
                ((value >> 14) | 0x80) as u8,
                ((value >> 21) & 0x7F) as u8,
            ];
            self.write_bytes(&bytes)
        } else if bits < 35 {
            let bytes = [
                (value | 0x80) as u8,
                ((value >> 7) | 0x80) as u8,
                ((value >> 14) | 0x80) as u8,
                ((value >> 21) | 0x80) as u8,
                ((value >> 28) & 0x7F) as u8,
            ];
            self.write_bytes(&bytes)
        } else if bits < 42 {
            let bytes = [
                (value | 0x80) as u8,
                ((value >> 7) | 0x80) as u8,
                ((value >> 14) | 0x80) as u8,
                ((value >> 21) | 0x80) as u8,
                ((value >> 28) | 0x80) as u8,
                ((value >> 35) & 0x7F) as u8,
            ];
            self.write_bytes(&bytes)
        } else if bits < 49 {
            let bytes = [
                (value | 0x80) as u8,
                ((value >> 7) | 0x80) as u8,
                ((value >> 14) | 0x80) as u8,
                ((value >> 21) | 0x80) as u8,
                ((value >> 28) | 0x80) as u8,
                ((value >> 35) | 0x80) as u8,
                ((value >> 42) & 0x7F) as u8,
            ];
            self.write_bytes(&bytes)
        } else if bits < 56 {
            let bytes = [
                (value | 0x80) as u8,
                ((value >> 7) | 0x80) as u8,
                ((value >> 14) | 0x80) as u8,
                ((value >> 21) | 0x80) as u8,
                ((value >> 28) | 0x80) as u8,
                ((value >> 35) | 0x80) as u8,
                ((value >> 42) | 0x80) as u8,
                ((value >> 49) & 0x7F) as u8,
            ];
            self.write_bytes(&bytes)
        } else if bits < 63 {
            let bytes = [
                (value | 0x80) as u8,
                ((value >> 7) | 0x80) as u8,
                ((value >> 14) | 0x80) as u8,
                ((value >> 21) | 0x80) as u8,
                ((value >> 28) | 0x80) as u8,
                ((value >> 35) | 0x80) as u8,
                ((value >> 42) | 0x80) as u8,
                ((value >> 49) | 0x80) as u8,
                ((value >> 56) & 0x7F) as u8,
            ];
            self.write_bytes(&bytes)
        } else {
            let bytes = [
                (value | 0x80) as u8,
                ((value >> 7) | 0x80) as u8,
                ((value >> 14) | 0x80) as u8,
                ((value >> 21) | 0x80) as u8,
                ((value >> 28) | 0x80) as u8,
                ((value >> 35) | 0x80) as u8,
                ((value >> 42) | 0x80) as u8,
                ((value >> 49) | 0x80) as u8,
                ((value >> 56) | 0x80) as u8,
                ((value >> 63) & 0x7F) as u8,
            ];
            self.write_bytes(&bytes)
        }
    }

    #[inline]
    pub fn write_byte(&mut self, byte: u8) -> Result<(), WriteError> {
        self.write_bytes(&[byte])
    }

    #[inline]
    pub fn write_f32(&mut self, value: f32) -> Result<(), WriteError> {
        self.write_bytes(&value.to_le_bytes())
    }

    #[inline]
    pub fn write_f64(&mut self, value: f64) -> Result<(), WriteError> {
        self.write_bytes(&value.to_le_bytes())
    }

    /// blob:
    /// size: leb
    /// payload: array(u8)
    pub fn write_blob(&mut self, payload: &[u8]) -> Result<(), WriteError> {
        self.write(payload.len())?;
        self.write_bytes(payload)
    }

    /// tagged:
    /// tag: u8
    /// payload: blob
    pub fn write_tagged_payload(&mut self, tag: u8, payload: &[u8]) -> Result<(), WriteError> {
        self.write_byte(tag)?;
        self.write_blob(payload)
    }
}

pub struct Leb128Reader<'a> {
    slice: &'a [u8],
    position: usize,
}

impl<'a> Leb128Reader<'a> {
    #[inline]
    pub const fn from_slice(slice: &'a [u8]) -> Self {
        Self { slice, position: 0 }
    }

    #[inline]
    pub fn cloned(&self) -> Self {
        Self {
            slice: self.slice,
            position: self.position,
        }
    }

    pub fn sub_slice(&mut self, len: usize) -> Option<Self> {
        self.position
            .checked_add(len)
            .and_then(|end| self.slice.get(self.position..end))
            .map(|slice| {
                self.position += len;
                Self { slice, position: 0 }
            })
    }

    pub fn read_bytes<'b>(&'b mut self, size: usize) -> Result<&'a [u8], ReadError> {
        self.slice
            .get(self.position..self.position + size)
            .map(|v| {
                self.position += size;
                v
            })
            .ok_or(ReadError::UnexpectedEof)
    }

    pub fn read_slice<'b, const N: usize>(&'b mut self) -> Result<&'a [u8; N], ReadError> {
        self.slice
            .get(self.position..self.position + N)
            .map(|v| {
                self.position += N;
                v.try_into().unwrap()
            })
            .ok_or(ReadError::UnexpectedEof)
    }

    #[inline]
    pub fn read_blob<'b>(&'b mut self) -> Result<&'a [u8], ReadError> {
        self.read_unsigned()
            .and_then(move |size| self.read_bytes(size as usize))
    }
}

impl Leb128Reader<'_> {
    #[inline]
    pub fn reset(&mut self) {
        self.position = 0;
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.slice.len()
    }

    #[inline]
    pub const fn position(&self) -> usize {
        self.position
    }

    #[inline]
    pub fn set_position(&mut self, val: usize) {
        self.position = val;
    }

    #[inline]
    pub const fn is_eof(&self) -> bool {
        self.position >= self.slice.len()
    }

    #[inline]
    pub fn read_byte(&mut self) -> Result<u8, ReadError> {
        self.slice
            .get(self.position)
            .map(|v| {
                self.position += 1;
                *v
            })
            .ok_or(ReadError::UnexpectedEof)
    }

    pub fn read_unsigned(&mut self) -> Result<u64, ReadError> {
        let mut value: u64 = 0;
        let mut scale = 0;
        let mut cursor = self.position;
        loop {
            let d = match self.slice.get(cursor) {
                Some(v) => *v,
                None => return Err(ReadError::UnexpectedEof),
            };
            cursor += 1;

            value |= (d as u64 & 0x7F) << scale;
            scale += 7;
            if (d & 0x80) == 0 {
                break;
            }
        }
        self.position = cursor;
        Ok(value)
    }

    pub fn read_signed(&mut self) -> Result<i64, ReadError> {
        let mut value: u64 = 0;
        let mut scale = 0;
        let mut cursor = self.position;
        let signed = loop {
            let d = match self.slice.get(cursor) {
                Some(v) => *v,
                None => return Err(ReadError::UnexpectedEof),
            };
            cursor += 1;

            value |= (d as u64 & 0x7F) << scale;
            let signed = (d & 0x40) != 0;
            if (d & 0x80) == 0 {
                break signed;
            }
            scale += 7;
        };
        self.position = cursor;
        if signed {
            Ok((value | 0xFFFF_FFFF_FFFF_FFC0 << scale) as i64)
        } else {
            Ok(value as i64)
        }
    }

    pub fn read_f32(&mut self) -> Result<f32, ReadError> {
        self.read_slice().map(|v| f32::from_le_bytes(*v))
    }

    pub fn read_f64(&mut self) -> Result<f64, ReadError> {
        self.read_slice().map(|v| f64::from_le_bytes(*v))
    }

    pub fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize, ReadError> {
        match self.slice.get(self.position..) {
            Some(v) => {
                let size = v.len();
                buf.extend_from_slice(v);
                self.position = self.slice.len();
                Ok(size)
            }
            None => Ok(0),
        }
    }
}

impl<'b> Leb128Reader<'b> {
    pub fn get_string<'a>(&'a mut self) -> Result<String, ReadError> {
        self.read().map(|v: &str| v.to_owned())
    }
}

pub trait ReadLeb128<'a, T> {
    fn read(&'a mut self) -> Result<T, ReadError>;
}

pub trait WriteLeb128<T> {
    fn write(&mut self, value: T) -> Result<(), WriteError>;
}

impl<'a, 'b> ReadLeb128<'a, &'b str> for Leb128Reader<'b> {
    #[inline]
    fn read(&'a mut self) -> Result<&'b str, ReadError> {
        self.read_blob()
            .and_then(|v| str::from_utf8(v).map_err(|_| ReadError::InvalidData))
    }
}

impl WriteLeb128<&str> for Leb128Writer {
    #[inline]
    fn write(&mut self, value: &str) -> Result<(), WriteError> {
        self.write(value.len())?;
        self.write_bytes(value.as_bytes())
    }
}

impl WriteLeb128<&String> for Leb128Writer {
    #[inline]
    fn write(&mut self, value: &String) -> Result<(), WriteError> {
        self.write(value.len())?;
        self.write_bytes(value.as_bytes())
    }
}

macro_rules! leb128_serialize_u {
    ($type:ident) => {
        impl<'a> ReadLeb128<'a, $type> for Leb128Reader<'_> {
            #[inline]
            fn read(&'a mut self) -> Result<$type, ReadError> {
                self.read_unsigned()
                    .and_then(|v| v.try_into().map_err(|_| ReadError::OutOfBounds))
            }
        }

        impl WriteLeb128<$type> for Leb128Writer {
            #[inline]
            fn write(&mut self, value: $type) -> Result<(), WriteError> {
                self.write_unsigned(value as u64)
            }
        }
    };
}

macro_rules! leb128_serialize_s {
    ($type:ident) => {
        impl<'a> ReadLeb128<'a, $type> for Leb128Reader<'_> {
            #[inline]
            fn read(&'a mut self) -> Result<$type, ReadError> {
                self.read_signed()
                    .and_then(|v| v.try_into().map_err(|_| ReadError::OutOfBounds))
            }
        }

        impl WriteLeb128<$type> for Leb128Writer {
            #[inline]
            fn write(&mut self, value: $type) -> Result<(), WriteError> {
                self.write_signed(value as i64)
            }
        }
    };
}

leb128_serialize_u!(u16);
leb128_serialize_u!(u32);
leb128_serialize_u!(u64);
leb128_serialize_u!(usize);

leb128_serialize_s!(i16);
leb128_serialize_s!(i32);
leb128_serialize_s!(i64);
leb128_serialize_s!(isize);

impl<'a> ReadLeb128<'a, u8> for Leb128Reader<'_> {
    #[inline]
    fn read(&'a mut self) -> Result<u8, ReadError> {
        self.read_byte()
    }
}

impl<'a> ReadLeb128<'a, i8> for Leb128Reader<'_> {
    #[inline]
    fn read(&'a mut self) -> Result<i8, ReadError> {
        self.read_byte().map(|v| v as i8)
    }
}

impl WriteLeb128<f32> for Leb128Writer {
    #[inline]
    fn write(&mut self, value: f32) -> Result<(), WriteError> {
        self.write_f32(value)
    }
}

impl<'a> ReadLeb128<'a, f32> for Leb128Reader<'_> {
    #[inline]
    fn read(&'a mut self) -> Result<f32, ReadError> {
        self.read_f32()
    }
}

impl WriteLeb128<f64> for Leb128Writer {
    #[inline]
    fn write(&mut self, value: f64) -> Result<(), WriteError> {
        self.write_f64(value)
    }
}

impl<'a> ReadLeb128<'a, f64> for Leb128Reader<'_> {
    #[inline]
    fn read(&'a mut self) -> Result<f64, ReadError> {
        self.read_f64()
    }
}

pub trait WriteFixed<T> {
    fn write_fixed(&mut self, value: T) -> Result<(), WriteError>;
}

impl WriteFixed<u8> for Leb128Writer {
    #[inline]
    fn write_fixed(&mut self, value: u8) -> Result<(), WriteError> {
        self.write_byte(value)
    }
}

impl WriteFixed<u32> for Leb128Writer {
    #[inline]
    fn write_fixed(&mut self, value: u32) -> Result<(), WriteError> {
        self.write_fixed_u32(value)
    }
}

impl WriteFixed<u64> for Leb128Writer {
    #[inline]
    fn write_fixed(&mut self, value: u64) -> Result<(), WriteError> {
        self.write_fixed_u64(value)
    }
}
