//! Constant Expression

use crate::*;
use ast::literal::NumericLiteral;
use leb128::{Leb128Writer, WriteError, WriteLeb128};
use types::ValType;
use wasm::opcode::WasmOpcode;

#[derive(Debug)]
pub struct ConstExpr(Vec<ConstInstr>);

#[derive(Debug, Clone, Copy)]
pub enum ConstInstr {
    I32Const(i32),
    I64Const(i64),
    F32Const(f32),
    F64Const(f64),
}

impl ConstExpr {
    #[inline]
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    pub fn expect<KEYWORD>(
        tokens: &mut TokenStream<KEYWORD>,
        valtype: ValType,
    ) -> Result<ConstExpr, AssembleError>
    where
        KEYWORD: Copy + Clone + PartialEq + core::fmt::Debug + core::fmt::Display,
    {
        let mut start = 0;
        let mut paren_level = 0;

        let mut vt_stack = Vec::new();
        let mut codes = Vec::new();

        loop {
            let token = tokens.next_non_blank();
            if start == 0 {
                start = tokens.peek_last().unwrap().position().start();
            }

            match token.token_type() {
                TokenType::Eof => {
                    return Err(AssembleError::missing_token(
                        &[TokenType::CloseParenthesis],
                        &token,
                    ));
                }
                TokenType::OpenParenthesis => {
                    paren_level += 1;
                    continue;
                }
                TokenType::CloseParenthesis => {
                    if paren_level > 0 {
                        paren_level -= 1;
                        continue;
                    }

                    AssembleError::check_types(
                        &[valtype],
                        &vt_stack,
                        TokenPosition((start as u32, token.position().end() as u32)).into(),
                    )?;
                    return Ok(ConstExpr(codes));
                }
                _ => {}
            }

            let token = token
                .convert(WasmOpcode::from_str)
                .into_keyword()
                .map_err(|token| AssembleError::invalid_mnemonic(&token))?;

            match token.keyword() {
                WasmOpcode::I32Const => {
                    let num = NumericLiteral::<i32>::expect(tokens)?;
                    vt_stack.push(ValType::I32);
                    codes.push(ConstInstr::I32Const(num.get()));
                }
                WasmOpcode::I64Const => {
                    let num = NumericLiteral::<i64>::expect(tokens)?;
                    vt_stack.push(ValType::I64);
                    codes.push(ConstInstr::I64Const(num.get()));
                }
                // WasmOpcode::F32Const => {}
                // WasmOpcode::F64Const => {}
                WasmOpcode::End => {
                    for _ in 0..=paren_level {
                        expect(tokens, &[TokenType::CloseParenthesis])?;
                    }

                    AssembleError::check_types(
                        &[valtype],
                        &vt_stack,
                        TokenPosition((start as u32, token.position().end() as u32)).into(),
                    )?;
                    return Ok(ConstExpr(codes));
                }
                _ => return Err(AssembleError::invalid_mnemonic(&token.as_token())),
            }
        }
    }

    pub fn write_to_wasm(&self, writer: &mut Leb128Writer) -> Result<(), WriteError> {
        for instr in self.0.iter() {
            match instr {
                ConstInstr::I32Const(v) => {
                    writer.write_byte(WasmOpcode::I32Const.leading_byte())?;
                    writer.write(*v)?;
                }
                ConstInstr::I64Const(v) => {
                    writer.write_byte(WasmOpcode::I64Const.leading_byte())?;
                    writer.write(*v)?;
                }
                ConstInstr::F32Const(v) => {
                    writer.write_byte(WasmOpcode::F32Const.leading_byte())?;
                    writer.write(*v)?;
                }
                ConstInstr::F64Const(v) => {
                    writer.write_byte(WasmOpcode::F64Const.leading_byte())?;
                    writer.write(*v)?;
                }
            }
        }
        writer.write_byte(WasmOpcode::End.leading_byte())?;
        Ok(())
    }
}
