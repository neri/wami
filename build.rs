use std::{
    collections::BTreeMap,
    fs::{read_to_string, File},
    io::*,
    num::ParseIntError,
    result,
};

fn main() {
    {
        let mut lines = Vec::new();
        for line in read_to_string("./misc/bytecode.csv").unwrap().lines() {
            if !line.is_empty() && !line.starts_with("#") {
                lines.push(line.to_string());
            }
        }

        let mut os = File::create("./src/_generated/bytecode.rs").unwrap();

        make_opcode(&mut os, lines.as_slice());

        println!("cargo:rerun-if-changed=./misc/bytecode.csv");
    }
}

struct Opcode {
    leading: u8,
    trailing: Option<u32>,
    mnemonic: String,
    identifier: String,
    params: Vec<String>,
    proposal: Option<String>,
    comment: Option<String>,
}

impl Opcode {
    fn common_comment(&self) -> String {
        let mut f = Vec::new();

        write!(f, "0x{:02X}", self.leading,).unwrap();
        if let Some(trailing) = self.trailing {
            write!(f, " 0x{:02X}", trailing,).unwrap();
        }

        write!(
            f,
            " {} ({}){}",
            [format!("`{}`", self.mnemonic)]
                .iter()
                .chain(self.params_str())
                .map(|v| v.clone())
                .collect::<Vec<_>>()
                .join(" "),
            self.proposal.as_ref().unwrap_or(&"MVP".to_string()),
            self.comment
                .as_ref()
                .map(|v| format!(" {}", v))
                .unwrap_or("".to_string()),
        )
        .unwrap();

        String::from_utf8(f).unwrap()
    }

    fn declare_id(&self) -> String {
        if self.params.is_empty() {
            self.identifier.clone()
        } else {
            format!(
                "{:}({})",
                self.identifier,
                self.params
                    .iter()
                    .map(|v| type_convert(&v))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    fn match_id(&self) -> String {
        if self.params.is_empty() {
            self.identifier.clone()
        } else {
            format!(
                "{:}({})",
                self.identifier,
                self.params
                    .iter()
                    .map(|_| "_".to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    fn params_str(&self) -> impl Iterator<Item = &String> {
        self.params.iter()
    }
}

fn make_opcode(os: &mut File, lines: &[String]) {
    let mut opcodes = BTreeMap::new();
    let mut opcode_order = Vec::new();
    let mut b2id = BTreeMap::new();
    let mut single2id = BTreeMap::new();
    let mut leading_ids = Vec::new();
    let mut proposals = vec!["Mvp".to_string()];

    for line in lines.iter().skip(1) {
        let mut cols = line.split(",");

        let leading = cols.next().unwrap();
        let trailing = cols.next().unwrap();
        let mnemonic = cols.next().unwrap();
        if mnemonic.is_empty() {
            continue;
        }

        let leading = parse_with_prefix(leading).unwrap() as u8;
        let trailing = parse_with_prefix(trailing).map(|v| v as u32).ok();
        let binary = ((leading as u64) << 32) + (trailing.unwrap_or_default() as u64);
        if trailing.is_some() && !leading_ids.contains(&leading) {
            leading_ids.push(leading);
        }

        let mnemonic = mnemonic.to_string();
        let identifier = to_camel_case_identifier(&mnemonic);

        let mut params = Vec::new();
        let param1 = cols.next().filter(|v| !v.is_empty());
        let param2 = cols.next().filter(|v| !v.is_empty());
        let _param3 = cols.next().filter(|v| !v.is_empty());
        if let Some(param1) = param1 {
            params.push(param1.to_string());
            if let Some(param2) = param2 {
                params.push(param2.to_string());
            }
        }

        let proposal = cols
            .next()
            .filter(|v| !v.is_empty())
            .map(|v| to_camel_case_identifier(v));
        if let Some(proposal) = proposal.clone() {
            if !proposals.contains(&proposal) {
                proposals.push(proposal);
            }
        }
        let comment = cols.next().filter(|v| !v.is_empty()).map(|v| v.to_string());

        let opcode = Opcode {
            leading,
            trailing,
            mnemonic: mnemonic.clone(),
            identifier: identifier.clone(),
            params,
            proposal,
            comment,
        };

        if opcodes.get(&identifier).is_some() {
            panic!("Duplicated mnemonic: {}", mnemonic);
        }
        if opcode.trailing.is_none() {
            single2id.insert(opcode.leading.clone(), identifier.clone());
        }
        b2id.insert(binary, identifier.clone());
        opcodes.insert(identifier.clone(), opcode);
        opcode_order.push(binary);
    }
    leading_ids.sort();
    opcode_order.sort();
    proposals.sort_by(|a, b| (proposal_release(a), a).cmp(&(proposal_release(b), b)));

    write!(
        os,
        "//
// This file is automatically generated at build time. DO NOT EDIT DIRECTLY.
//
use crate::{{leb128::*, CompileErrorKind, WasmMemArg, WasmBlockType, BrTableVec}};
use core::fmt;

/// WebAssembly Bytecode
#[non_exhaustive]
pub enum WasmBytecode {{
"
    )
    .unwrap();
    for opcode in opcode_order.iter() {
        let id = b2id.get(opcode).unwrap();
        let opcode: &Opcode = opcodes.get(id).unwrap();

        writeln!(os, "    /// {}", opcode.common_comment(),).unwrap();
        writeln!(os, "    {},", opcode.declare_id()).unwrap();
    }
    write!(
        os,
        "}}

impl WasmBytecode {{
    pub fn fetch(reader: &mut Leb128Reader) -> Result<Self, CompileErrorKind> {{
        let leading = reader.read_byte()?;
        match leading {{
"
    )
    .unwrap();

    for leading in single2id.values() {
        let opcode = opcodes.get(leading).unwrap();
        writeln!(
            os,
            "            // {}\n            0x{:02X} => {{",
            opcode.common_comment(),
            opcode.leading,
        )
        .unwrap();

        match opcode.params.len() {
            2 => {
                writeln!(
                    os,
                    "                let a1 = reader.read()?;
                let a2 = reader.read()?;
                Ok(Self::{}(a1, a2))",
                    opcode.identifier,
                )
                .unwrap();
            }
            1 => match opcode.params[0].as_str() {
                _ => {
                    writeln!(
                        os,
                        "                let a1 = reader.read()?;
                Ok(Self::{}(a1))",
                        opcode.identifier,
                    )
                    .unwrap();
                }
            },

            _ => writeln!(os, "                Ok(Self::{})", opcode.identifier,).unwrap(),
        }

        writeln!(os, "            }}",).unwrap();
    }

    for leading in leading_ids {
        write!(
            os,
            "            0x{:02X} => {{
                let trailing: u32 = reader.read()?;
                match trailing {{
",
            leading
        )
        .unwrap();

        for (binary, identifier) in b2id.iter() {
            let base = (*binary >> 32) as u8;
            if base < leading {
                continue;
            } else if base > leading {
                break;
            }
            let opcode = opcodes.get(identifier).unwrap();
            let trailing = opcode.trailing.unwrap();

            writeln!(
                os,
                "                    // {}\n                    0x{:02x} => {{",
                opcode.common_comment(),
                trailing,
            )
            .unwrap();

            match opcode.params.len() {
                2 => {
                    writeln!(
                        os,
                        "                        let a1 = reader.read()?;
                        let a2 = reader.read()?;
                        Ok(Self::{}(a1, a2))",
                        opcode.identifier,
                    )
                    .unwrap();
                }
                1 => match opcode.params[0].as_str() {
                    _ => {
                        writeln!(
                            os,
                            "                        let a1 = reader.read()?;
                        Ok(Self::{}(a1))",
                            opcode.identifier,
                        )
                        .unwrap();
                    }
                },
                _ => writeln!(
                    os,
                    "                        Ok(Self::{})",
                    opcode.identifier,
                )
                .unwrap(),
            }

            writeln!(os, "                    }},",).unwrap();
        }

        writeln!(
            os,
            "                    _ => Err(CompileErrorKind::InvalidBytecode2(leading, trailing))
                }}
            }}",
        )
        .unwrap();
    }

    write!(
        os,
        "            _ => Err(CompileErrorKind::InvalidBytecode(leading))
        }}
    }}

    #[inline]
    pub const fn as_str(&self) -> &'static str {{
        self.mnemonic().as_str()
    }}

    pub const fn mnemonic(&self) -> WasmMnemonic {{
        match self {{
"
    )
    .unwrap();

    for opcode in opcodes.values() {
        writeln!(
            os,
            "            Self::{} => WasmMnemonic::{},",
            opcode.match_id(),
            opcode.identifier,
        )
        .unwrap();
    }

    write!(
        os,
        "
        }}
    }}

}}

impl fmt::Display for WasmBytecode {{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {{
        match self {{
            _ => f.write_str(self.as_str())
        }}
    }}
}}

impl fmt::Debug for WasmBytecode {{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {{
        match self {{
            _ => f.write_str(self.as_str())
        }}
    }}
}}

#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WasmMnemonic {{
"
    )
    .unwrap();
    for opcode in opcode_order.iter() {
        let id = b2id.get(opcode).unwrap();
        let opcode: &Opcode = opcodes.get(id).unwrap();

        writeln!(os, "    /// {}", opcode.common_comment(),).unwrap();
        writeln!(os, "    {},", opcode.identifier).unwrap();
    }
    write!(
        os,
        "}}

impl WasmMnemonic {{
    pub const fn as_str(&self) -> &'static str {{
        match self {{
"
    )
    .unwrap();

    for opcode in opcodes.values() {
        writeln!(
            os,
            "            Self::{} => \"{}\",",
            opcode.identifier, opcode.mnemonic,
        )
        .unwrap();
    }

    write!(
        os,
        "        }}
    }}
    
    pub const fn proposal(&self) -> WasmProposal {{
        match self {{
"
    )
    .unwrap();

    for opcode in opcodes.values() {
        if let Some(proposal) = opcode.proposal.clone() {
            if proposal == "Mvp" {
                continue;
            }
            writeln!(
                os,
                "            Self::{} => WasmProposal::{},",
                opcode.identifier, proposal,
            )
            .unwrap();
        }
    }

    write!(
        os,
        "            _ => WasmProposal::Mvp,
        }}
    }}
}}

impl fmt::Display for WasmMnemonic {{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {{
        f.write_str(self.as_str())
    }}
}}

impl fmt::Debug for WasmMnemonic {{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {{
        f.write_str(self.as_str())
    }}
}}

impl From<WasmBytecode> for WasmMnemonic {{
    #[inline]
    fn from(val: WasmBytecode) -> WasmMnemonic {{
        val.mnemonic()
    }}
}}

#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WasmProposal {{
"
    )
    .unwrap();

    for proposal in proposals {
        writeln!(os, "    {},", proposal).unwrap();
    }

    write!(
        os,
        "}}
"
    )
    .unwrap();
}

#[allow(dead_code)]
fn make_enum(os: &mut File, class_name: &str, comment: &str, keywords: &[String]) {
    write!(
        os,
        "//
// This file is automatically generated at build time. DO NOT EDIT DIRECTLY.
//

/// {comment}
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum {class_name} {{
"
    )
    .unwrap();
    for keyword in keywords.iter() {
        writeln!(os, "    /// \"{}\"", keyword).unwrap();
        writeln!(os, "    {},", to_camel_case_identifier(keyword)).unwrap();
    }
    write!(
        os,
        "}}

impl {class_name} {{
    pub fn all_values() -> &'static [Self] {{
        &[
"
    )
    .unwrap();

    for keyword in keywords.iter() {
        writeln!(
            os,
            "            Self::{},",
            to_camel_case_identifier(keyword),
        )
        .unwrap();
    }

    write!(
        os,
        "        ]
    }}

    pub fn from_str(v: &str) -> Option<Self> {{
        match v {{
"
    )
    .unwrap();

    for keyword in keywords.iter() {
        writeln!(
            os,
            "            \"{}\" => Some(Self::{}),",
            keyword,
            to_camel_case_identifier(keyword),
        )
        .unwrap();
    }

    write!(
        os,
        "            _ => None,
        }}
    }}

    pub fn as_str(&self) -> &'static str {{
        match self {{
"
    )
    .unwrap();

    for keyword in keywords.iter() {
        writeln!(
            os,
            "            Self::{} => \"{}\",",
            to_camel_case_identifier(keyword),
            keyword,
        )
        .unwrap();
    }

    write!(
        os,
        "        }}
    }}
}}
"
    )
    .unwrap();
}

fn to_camel_case_identifier(s: &str) -> String {
    let mut output = Vec::new();
    let mut next_upcase = true;
    for ch in s.chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => {
                if next_upcase {
                    output.push(ch.to_ascii_uppercase())
                } else {
                    output.push(ch.to_ascii_lowercase())
                }
                next_upcase = false;
            }
            _ => {
                next_upcase = true;
            }
        }
    }

    output.into_iter().collect::<String>()
}

fn parse_with_prefix(s: &str) -> result::Result<usize, ParseIntError> {
    if s.len() >= 3 && s.starts_with("0") {
        let radix = match s.chars().nth(1).unwrap().to_ascii_lowercase() {
            'b' => 2,
            'o' => 8,
            'x' => 16,
            _ => 0,
        };
        usize::from_str_radix(&s[2..], radix)
    } else {
        usize::from_str_radix(s, 10)
    }
}

fn type_convert(src: &str) -> String {
    match src {
        "bt" => "WasmBlockType".to_string(),
        "br_table" => "BrTableVec".to_string(),
        "memarg" => "WasmMemArg".to_string(),
        _ => src.to_string(),
    }
}

fn proposal_release(val: &str) -> usize {
    match val {
        // WASM v1.0 MVP
        "Mvp" => 1,
        // WASM v2.0
        "NonTrappingFloatToIntConversion"
        | "SignExtension"
        | "ReferenceTypes"
        | "BulkMemoryOperations"
        | "Simd" => 2,
        // otherwise
        _ => usize::MAX,
    }
}
