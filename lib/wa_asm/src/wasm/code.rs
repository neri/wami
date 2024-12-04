//! Normal code block

use crate::*;
use ast::{
    identifier::{Identifier, IndexToken},
    literal::NumericLiteral,
    try_expect_module,
    types::TypeUse,
};
use ir::{Module, index::*};
use leb128::{Leb128Writer, WriteLeb128};
use types::ValType;
use wasm::opcode::WasmOpcode;

#[derive(Debug)]
pub enum Code {
    Source(Text),
    Binary(Binary),
}

#[derive(Debug)]
pub struct Text {
    tokens: TokenStream<Keyword>,
}

pub struct Binary {
    bytes: Vec<u8>,
}

impl Binary {
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl core::fmt::Debug for Binary {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        DumpHex(&self.bytes).fmt(f)
    }
}

impl Code {
    pub fn expect(tokens: &mut TokenStream<Keyword>) -> Result<Code, AssembleError> {
        let snapshot = tokens.snapshot();
        let mut paren_level = 0;

        let token = tokens.next_non_blank();
        match token.token_type() {
            TokenType::Eof => {
                return Err(AssembleError::missing_token(
                    &[TokenType::CloseParenthesis],
                    &token,
                ));
            }
            TokenType::CloseParenthesis => {
                return Ok(Code::Source(Text {
                    tokens: tokens.make_replay(snapshot),
                }));
            }
            _ => {}
        }

        loop {
            let token = tokens.next_non_blank();
            if token.token_type() == &TokenType::Eof {
                return Err(AssembleError::missing_token(
                    &[TokenType::CloseParenthesis],
                    &token,
                ));
            } else if token.token_type() == &TokenType::OpenParenthesis {
                paren_level += 1;
            } else if token.token_type() == &TokenType::CloseParenthesis {
                if paren_level > 0 {
                    paren_level -= 1;
                    continue;
                }
                return Ok(Code::Source(Text {
                    tokens: tokens.make_replay(snapshot),
                }));
            }
        }
    }

    pub fn assemble(
        &mut self,
        module: &Module,
        results: &[ValType],
        locals: &[ValType],
        local_ids: &BTreeMap<String, LocalIndex>,
        params_and_locals: &[ValType],
    ) -> Result<(), AssembleError> {
        match self {
            Code::Binary(_) => Ok(()),
            Code::Source(src) => Self::assemble_from_source(
                &mut src.tokens,
                module,
                results,
                locals,
                local_ids,
                params_and_locals,
            )
            .map(|bytes| *self = Code::Binary(Binary { bytes })),
        }
    }

    /// Perform assembly from source code
    fn assemble_from_source(
        tokens: &mut TokenStream<Keyword>,
        module: &Module,
        results: &[ValType],
        locals: &[ValType],
        local_ids: &BTreeMap<String, LocalIndex>,
        params_and_locals: &[ValType],
    ) -> Result<Vec<u8>, AssembleError> {
        let _ = results;
        let mut writer = Leb128Writer::new();

        Self::assemble_locals(locals, &mut writer);

        let mut block_stack = BlockStack::default();

        loop {
            if let Ok(end) = tokens.expect(&[TokenType::CloseParenthesis, TokenType::Eof]) {
                if block_stack.len() > 0 {
                    return Err(AssembleError::out_of_bounds(
                        "missing 'end'",
                        end.position().into(),
                    ));
                }

                writer.write_byte(WasmOpcode::End.leading_byte()).unwrap();

                return Ok(writer.into_vec());
            }

            let instr = tokens
                .expect_any_keyword()
                .map_err(|token| AssembleError::invalid_mnemonic(&token))?;
            let instr = instr
                .as_token()
                .convert(WasmOpcode::from_str)
                .into_keyword()
                .unwrap();

            // TODO: more validations
            let mnemonic = instr.keyword();
            match mnemonic {
                WasmOpcode::Block | WasmOpcode::Loop | WasmOpcode::If => {
                    let inst_type = match mnemonic {
                        WasmOpcode::Block => BlockInstType::Block,
                        WasmOpcode::Loop => BlockInstType::Loop,
                        WasmOpcode::If => BlockInstType::If,
                        _ => unreachable!(),
                    };

                    let label = Identifier::try_expect(tokens)?;

                    let block_type = try_expect_module::<ast::types::FuncTypeResult>(tokens)?
                        .map(|v| v.0.first().map(|v| *v))
                        .flatten();

                    block_stack.push(label.as_ref(), BlockStackEntry {
                        inst_type,
                        block_type,
                    })?;

                    writer.write_byte(mnemonic.leading_byte()).unwrap();
                    if let Some(block_type) = block_type {
                        writer.write(block_type.as_bytecode()).unwrap();
                    } else {
                        writer.write_byte(0x40).unwrap();
                    }
                }

                WasmOpcode::End => match block_stack.pop() {
                    Some(_) => writer.write_byte(mnemonic.leading_byte()).unwrap(),
                    None => {
                        return Err(AssembleError::out_of_bounds(
                            "Too many 'end'",
                            instr.position().into(),
                        ));
                    }
                },

                WasmOpcode::Else => match block_stack.pop() {
                    Some(block) => match block.inst_type {
                        BlockInstType::If => {
                            block_stack.push(None, BlockStackEntry {
                                inst_type: BlockInstType::Else,
                                block_type: block.block_type,
                            })?;
                            writer.write_byte(mnemonic.leading_byte()).unwrap();
                        }
                        _ => {
                            return Err(AssembleError::out_of_bounds(
                                "'else' without 'if'",
                                instr.position().into(),
                            ));
                        }
                    },
                    None => {
                        return Err(AssembleError::out_of_bounds(
                            "'else' without 'if'",
                            instr.position().into(),
                        ));
                    }
                },

                WasmOpcode::Br | WasmOpcode::BrIf => {
                    let index = IndexToken::expect(tokens)?;
                    let target = match index {
                        IndexToken::Num(num) => {
                            let index = num.get();
                            AssembleError::check_index(
                                index,
                                0..(block_stack.len() as u32),
                                num.position().into(),
                            )?;
                            index
                        }
                        IndexToken::Id(id) => block_stack.solve(&id)?,
                    };
                    writer.write_byte(mnemonic.leading_byte()).unwrap();
                    writer.write(target).unwrap();
                }

                WasmOpcode::BrTable => {
                    let mut targets = Vec::new();
                    while let Some(index) = IndexToken::try_expect(tokens)? {
                        match index {
                            IndexToken::Num(num) => {
                                let index = num.get();
                                AssembleError::check_index(
                                    index,
                                    0..(block_stack.len() as u32),
                                    num.position().into(),
                                )?;
                                targets.push(index);
                            }
                            IndexToken::Id(id) => {
                                targets.push(block_stack.solve(&id)?);
                            }
                        }
                    }
                    if targets.len() < 1 {
                        return Err(AssembleError::missing_token(
                            &[TokenType::Identifier, TokenType::NumericLiteral],
                            &tokens.next().unwrap(),
                        ));
                    }
                    writer.write_byte(mnemonic.leading_byte()).unwrap();
                    writer.write(targets.len() - 1).unwrap();
                    for target in targets {
                        writer.write(target).unwrap();
                    }
                }

                WasmOpcode::CallIndirect => {
                    let typeuse = TypeUse::expect(tokens)?;
                    let typeidx = module.find_typeuse(&typeuse)?;
                    writer.write_byte(mnemonic.leading_byte()).unwrap();
                    writer.write(typeidx.as_usize()).unwrap();
                    writer.write(0usize).unwrap();
                }

                WasmOpcode::Call => {
                    let index = IndexToken::expect(tokens)?;
                    let funcidx = module.get_funcidx(&index)?;
                    writer.write_byte(mnemonic.leading_byte()).unwrap();
                    writer.write(funcidx.as_usize()).unwrap();
                }

                WasmOpcode::I32Const => {
                    let num = NumericLiteral::<i32>::expect(tokens)?;
                    writer.write_byte(mnemonic.leading_byte()).unwrap();
                    writer.write(num.get()).unwrap();
                }
                WasmOpcode::I64Const => {
                    let num = NumericLiteral::<i64>::expect(tokens)?;
                    writer.write_byte(mnemonic.leading_byte()).unwrap();
                    writer.write(num.get()).unwrap();
                }
                // WasmOpcode::F32Const => {}
                // WasmOpcode::F64Const => {}
                WasmOpcode::LocalGet | WasmOpcode::LocalSet | WasmOpcode::LocalTee => {
                    let index = IndexToken::expect(tokens)?;
                    let localidx = match index {
                        IndexToken::Num(num) => {
                            let index = num.get();
                            AssembleError::check_index(
                                index,
                                0..(params_and_locals.len() as u32),
                                num.position().into(),
                            )?;
                            index
                        }
                        IndexToken::Id(id) => {
                            let localidx = local_ids.get(id.name()).ok_or(
                                AssembleError::undefined_identifier(
                                    id.name(),
                                    id.position().into(),
                                ),
                            )?;
                            localidx.as_usize() as u32
                        }
                    };
                    writer.write_byte(mnemonic.leading_byte()).unwrap();
                    writer.write(localidx).unwrap();
                }

                WasmOpcode::GlobalGet | WasmOpcode::GlobalSet => {
                    let index = IndexToken::expect(tokens)?;
                    let globalidx = module.get_globalidx(&index)?;
                    writer.write_byte(mnemonic.leading_byte()).unwrap();
                    writer.write(globalidx.as_usize()).unwrap();
                }

                WasmOpcode::Unreachable
                | WasmOpcode::Nop
                | WasmOpcode::Return
                | WasmOpcode::Drop
                | WasmOpcode::Select
                | WasmOpcode::I32Eqz
                | WasmOpcode::I32Eq
                | WasmOpcode::I32Ne
                | WasmOpcode::I32LtS
                | WasmOpcode::I32LtU
                | WasmOpcode::I32GtS
                | WasmOpcode::I32GtU
                | WasmOpcode::I32LeS
                | WasmOpcode::I32LeU
                | WasmOpcode::I32GeS
                | WasmOpcode::I32GeU
                | WasmOpcode::I64Eqz
                | WasmOpcode::I64Eq
                | WasmOpcode::I64Ne
                | WasmOpcode::I64LtS
                | WasmOpcode::I64LtU
                | WasmOpcode::I64GtS
                | WasmOpcode::I64GtU
                | WasmOpcode::I64LeS
                | WasmOpcode::I64LeU
                | WasmOpcode::I64GeS
                | WasmOpcode::I64GeU
                | WasmOpcode::F32Eq
                | WasmOpcode::F32Ne
                | WasmOpcode::F32Lt
                | WasmOpcode::F32Gt
                | WasmOpcode::F32Le
                | WasmOpcode::F32Ge
                | WasmOpcode::F64Eq
                | WasmOpcode::F64Ne
                | WasmOpcode::F64Lt
                | WasmOpcode::F64Gt
                | WasmOpcode::F64Le
                | WasmOpcode::F64Ge
                | WasmOpcode::I32Clz
                | WasmOpcode::I32Ctz
                | WasmOpcode::I32Popcnt
                | WasmOpcode::I32Add
                | WasmOpcode::I32Sub
                | WasmOpcode::I32Mul
                | WasmOpcode::I32DivS
                | WasmOpcode::I32DivU
                | WasmOpcode::I32RemS
                | WasmOpcode::I32RemU
                | WasmOpcode::I32And
                | WasmOpcode::I32Or
                | WasmOpcode::I32Xor
                | WasmOpcode::I32Shl
                | WasmOpcode::I32ShrS
                | WasmOpcode::I32ShrU
                | WasmOpcode::I32Rotl
                | WasmOpcode::I32Rotr
                | WasmOpcode::I64Clz
                | WasmOpcode::I64Ctz
                | WasmOpcode::I64Popcnt
                | WasmOpcode::I64Add
                | WasmOpcode::I64Sub
                | WasmOpcode::I64Mul
                | WasmOpcode::I64DivS
                | WasmOpcode::I64DivU
                | WasmOpcode::I64RemS
                | WasmOpcode::I64RemU
                | WasmOpcode::I64And
                | WasmOpcode::I64Or
                | WasmOpcode::I64Xor
                | WasmOpcode::I64Shl
                | WasmOpcode::I64ShrS
                | WasmOpcode::I64ShrU
                | WasmOpcode::I64Rotl
                | WasmOpcode::I64Rotr
                | WasmOpcode::F32Abs
                | WasmOpcode::F32Neg
                | WasmOpcode::F32Ceil
                | WasmOpcode::F32Floor
                | WasmOpcode::F32Trunc
                | WasmOpcode::F32Nearest
                | WasmOpcode::F32Sqrt
                | WasmOpcode::F32Add
                | WasmOpcode::F32Sub
                | WasmOpcode::F32Mul
                | WasmOpcode::F32Div
                | WasmOpcode::F32Min
                | WasmOpcode::F32Max
                | WasmOpcode::F32Copysign
                | WasmOpcode::F64Abs
                | WasmOpcode::F64Neg
                | WasmOpcode::F64Ceil
                | WasmOpcode::F64Floor
                | WasmOpcode::F64Trunc
                | WasmOpcode::F64Nearest
                | WasmOpcode::F64Sqrt
                | WasmOpcode::F64Add
                | WasmOpcode::F64Sub
                | WasmOpcode::F64Mul
                | WasmOpcode::F64Div
                | WasmOpcode::F64Min
                | WasmOpcode::F64Max
                | WasmOpcode::F64Copysign
                | WasmOpcode::I32WrapI64
                | WasmOpcode::I32TruncF32S
                | WasmOpcode::I32TruncF32U
                | WasmOpcode::I32TruncF64S
                | WasmOpcode::I32TruncF64U
                | WasmOpcode::I64ExtendI32S
                | WasmOpcode::I64ExtendI32U
                | WasmOpcode::I64TruncF32S
                | WasmOpcode::I64TruncF32U
                | WasmOpcode::I64TruncF64S
                | WasmOpcode::I64TruncF64U
                | WasmOpcode::F32ConvertI32S
                | WasmOpcode::F32ConvertI32U
                | WasmOpcode::F32ConvertI64S
                | WasmOpcode::F32ConvertI64U
                | WasmOpcode::F32DemoteF64
                | WasmOpcode::F64ConvertI32S
                | WasmOpcode::F64ConvertI32U
                | WasmOpcode::F64ConvertI64S
                | WasmOpcode::F64ConvertI64U
                | WasmOpcode::F64PromoteF32
                | WasmOpcode::I32ReinterpretF32
                | WasmOpcode::I64ReinterpretF64
                | WasmOpcode::F32ReinterpretI32
                | WasmOpcode::F64ReinterpretI64
                | WasmOpcode::I32Extend8S
                | WasmOpcode::I32Extend16S
                | WasmOpcode::I64Extend8S
                | WasmOpcode::I64Extend16S
                | WasmOpcode::I64Extend32S
                | WasmOpcode::I32TruncSatF32S
                | WasmOpcode::I32TruncSatF32U
                | WasmOpcode::I32TruncSatF64S
                | WasmOpcode::I32TruncSatF64U
                | WasmOpcode::I64TruncSatF32S
                | WasmOpcode::I64TruncSatF32U
                | WasmOpcode::I64TruncSatF64S
                | WasmOpcode::I64TruncSatF64U => {
                    writer.write_byte(mnemonic.leading_byte()).unwrap();
                    if let Some(trailing) = mnemonic.trailing_word() {
                        writer.write(trailing).unwrap();
                    }
                }

                WasmOpcode::I32Load
                | WasmOpcode::I64Load
                | WasmOpcode::F32Load
                | WasmOpcode::F64Load
                | WasmOpcode::I32Load8S
                | WasmOpcode::I32Load8U
                | WasmOpcode::I32Load16S
                | WasmOpcode::I32Load16U
                | WasmOpcode::I64Load8S
                | WasmOpcode::I64Load8U
                | WasmOpcode::I64Load16S
                | WasmOpcode::I64Load16U
                | WasmOpcode::I64Load32S
                | WasmOpcode::I64Load32U
                | WasmOpcode::I32Store
                | WasmOpcode::I64Store
                | WasmOpcode::F32Store
                | WasmOpcode::F64Store
                | WasmOpcode::I32Store8
                | WasmOpcode::I32Store16
                | WasmOpcode::I64Store8
                | WasmOpcode::I64Store16
                | WasmOpcode::I64Store32 => {
                    let mut offset = 0;
                    let mut align = 0;

                    if let Some(_token) = try_expect_free_keyword(tokens, "offset")? {
                        expect_immed(tokens, &[TokenType::Symbol('=')])?;
                        let num = NumericLiteral::<u32>::expect(tokens)?;
                        offset = num.get();
                    }

                    if let Some(_token) = try_expect_free_keyword(tokens, "align")? {
                        expect_immed(tokens, &[TokenType::Symbol('=')])?;
                        let num = NumericLiteral::<u32>::expect(tokens)?;
                        align = num.get();
                    }

                    writer.write_byte(mnemonic.leading_byte()).unwrap();
                    writer.write(align).unwrap();
                    writer.write(offset).unwrap();
                }

                WasmOpcode::MemorySize | WasmOpcode::MemoryGrow | WasmOpcode::MemoryFill => {
                    writer.write_byte(mnemonic.leading_byte()).unwrap();
                    if let Some(trailing) = mnemonic.trailing_word() {
                        writer.write(trailing).unwrap();
                    }
                    writer.write(0).unwrap();
                }

                WasmOpcode::MemoryCopy => {
                    writer.write_byte(mnemonic.leading_byte()).unwrap();
                    if let Some(trailing) = mnemonic.trailing_word() {
                        writer.write(trailing).unwrap();
                    }
                    writer.write(0).unwrap();
                    writer.write(0).unwrap();
                }

                _ => {
                    return Err(AssembleError::internal_inconsistency(
                        "We are sorry, not yet supported",
                        instr.position().into(),
                    ));
                }
            }
        }
    }

    #[inline]
    fn assemble_locals(locals: &[ValType], writer: &mut Leb128Writer) {
        let mut locals = locals.iter().map(|v| v.as_bytecode());
        let mut total_local_num = 0;
        let mut local_writer = Leb128Writer::new();
        if let Some(first) = locals.next() {
            let mut current_num = 1usize;
            let mut current_type = first;
            for next in locals {
                if current_type == next {
                    current_num += 1;
                } else {
                    local_writer.write(current_num).unwrap();
                    local_writer.write(current_type).unwrap();
                    total_local_num += 1;
                    current_num = 1;
                    current_type = next;
                }
            }
            local_writer.write(current_num).unwrap();
            local_writer.write(current_type).unwrap();
            total_local_num += 1;
        }
        writer.write(total_local_num).unwrap();
        writer.write_bytes(&local_writer.into_vec()).unwrap();
    }
}

#[derive(Debug, PartialEq)]
enum BlockInstType {
    Block,
    Loop,
    If,
    Else,
}

#[derive(Debug)]
struct BlockStackEntry {
    inst_type: BlockInstType,
    block_type: Option<ValType>,
    // stack_level: usize,
}

#[derive(Debug, Default)]
struct BlockStack {
    items: Vec<BlockStackEntry>,
    labels: Vec<Option<String>>,
}

impl BlockStack {
    pub fn push(
        &mut self,
        label: Option<&ast::identifier::Identifier>,
        value: BlockStackEntry,
    ) -> Result<(), AssembleError> {
        if let Some(label) = label {
            if self.labels.contains(&Some(label.name().to_string())) {
                return Err(AssembleError::duplicated_identifier(label));
            }
        }
        self.items.push(value);
        self.labels.push(label.map(|v| v.name().to_string()));
        Ok(())
    }

    #[inline]
    pub fn pop(&mut self) -> Option<BlockStackEntry> {
        self.items.pop().map(|v| {
            let _ = self.labels.pop();
            v
        })
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn solve(&self, label: &ast::identifier::Identifier) -> Result<u32, AssembleError> {
        for (index, target) in self.labels.iter().rev().enumerate() {
            let Some(target) = target else { continue };
            if target == label.name() {
                return Ok(index as u32);
            }
        }
        Err(AssembleError::undefined_identifier(
            label.name(),
            label.position().into(),
        ))
    }
}
