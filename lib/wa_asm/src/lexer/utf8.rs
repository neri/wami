//! Simple utf-8 decoder

#[derive(Clone, Copy)]
pub struct Utf8StateMachine {
    buf: u32,
}

impl Utf8StateMachine {
    #[inline]
    pub const fn new() -> Self {
        Self { buf: 0 }
    }

    #[inline]
    pub fn force_clear(&mut self) {
        self.buf = 0;
    }

    #[inline]
    pub const fn len(&self) -> usize {
        match self.buf {
            0 => 0,
            0x00_00_00_01..=0x00_00_00_FF => 1,
            0x00_00_01_00..=0x00_00_FF_FF => 2,
            0x00_01_00_00..=0x00_FF_FF_FF => 3,
            0x01_00_00_00..=0xFF_FF_FF_FF => 4,
        }
    }

    #[inline]
    pub fn expected_len(&self) -> usize {
        match self.lead_byte() {
            0x01..=0x7F => 1,
            0xC2..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF4 => 4,
            _ => 0,
        }
    }

    pub fn push(&mut self, byte: u8) -> Result<(), u8> {
        if self.buf == 0 {
            match byte {
                0x01..=0x7F | 0xC2..=0xF4 => {
                    self.buf = byte as u32;
                    Ok(())
                }
                _ => Err(byte),
            }
        } else {
            match byte {
                0x80..=0xBF => {
                    self.buf |= (byte as u32).wrapping_shl(self.len().wrapping_mul(8) as u32);
                    Ok(())
                }
                _ => Err(byte),
            }
        }
    }

    #[inline]
    fn _get(&self, index: usize) -> u8 {
        self.buf.wrapping_shr(index.wrapping_mul(8) as u32) as u8
    }

    #[inline]
    pub fn lead_byte(&self) -> u8 {
        self._get(0)
    }

    #[inline]
    pub fn needs_trail_bytes(&self) -> bool {
        self.len() < self.expected_len()
    }

    pub fn valid_char(&self) -> Option<char> {
        match self.expected_len() {
            1 => char::from_u32(self.buf),
            2 => {
                let lead = self.lead_byte() as u32 & 0x1F;
                let t1 = self._get(1) as u32 & 0x3F;
                char::from_u32((lead << 6) | t1)
            }
            3 => {
                let lead = self.lead_byte() as u32 & 0x0F;
                let t1 = self._get(1) as u32 & 0x3F;
                let t2 = self._get(2) as u32 & 0x3F;
                let u = (lead << 12) | (t1 << 6) | t2;
                (u >= 0x0800).then(|| char::from_u32(u)).flatten()
            }
            4 => {
                let lead = self.lead_byte() as u32 & 0x07;
                let t1 = self._get(1) as u32 & 0x3F;
                let t2 = self._get(2) as u32 & 0x3F;
                let t3 = self._get(3) as u32 & 0x3F;
                let u = (lead << 16) | (t1 << 12) | (t2 << 6) | t3;
                (u >= 0x010000 && u <= 0x10FFFF)
                    .then(|| char::from_u32(u))
                    .flatten()
            }
            _ => None,
        }
    }

    #[inline]
    pub fn take_valid_char(&mut self) -> Option<char> {
        self.valid_char().map(|v| {
            self.force_clear();
            v
        })
    }
}
