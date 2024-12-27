use leb128::{Leb128Writer, WriteLeb128};
use opcode::WasmOpcode;

#[path = "../_generated/opcode.rs"]
pub mod opcode;

pub mod code;
pub mod expr;
pub mod section_id;

impl WriteLeb128<WasmOpcode> for Leb128Writer {
    fn write(&mut self, value: WasmOpcode) -> Result<(), leb128::WriteError> {
        self.write_byte(value.leading_byte())?;
        if let Some(trailing_word) = value.trailing_word() {
            self.write(trailing_word)
        } else {
            Ok(())
        }
    }
}
