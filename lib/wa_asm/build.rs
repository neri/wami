use std::{
    collections::BTreeMap,
    fs::{File, read_to_string},
    io::*,
};

fn main() {
    {
        make_enum(
            "./src/misc/valtype.txt",
            "./src/_generated/valtype.rs",
            "ValType",
            "WebAssembly Value Types",
            &[],
        );
    }

    {
        let mut lines = Vec::new();
        for line in read_to_string("./src/misc/opcode.csv").unwrap().lines() {
            if !line.is_empty() && !line.starts_with("#") {
                lines.push(line.to_string());
            }
        }
        let mut os = File::create("./src/_generated/opcode.rs").unwrap();
        let opcodes = make_opcode(&mut os, lines.as_slice());
        println!("cargo:rerun-if-changed=./src/wasm/opcode.csv");

        make_enum(
            "./src/misc/keyword.txt",
            "./src/_generated/keyword.rs",
            "Keyword",
            "WebAssembly Reserved Keywords",
            &opcodes,
        );
    }
}

fn make_enum(
    src_path: &str,
    dest_path: &str,
    class_name: &str,
    comment: &str,
    appending: &[String],
) {
    let mut keywords = Vec::new();
    for line in read_to_string(src_path).unwrap().lines() {
        let keyword = line.trim().to_string();
        if !keyword.is_empty() && !keyword.starts_with("#") {
            if keywords.contains(&keyword) {
                panic!("redefined keyword: {keyword}");
            }
            keywords.push(keyword);
        }
    }
    keywords.extend_from_slice(appending);
    keywords.sort();

    let mut os = File::create(dest_path).unwrap();

    write!(
        os,
        "//! {comment}

/* This file is generated automatically. DO NOT EDIT DIRECTLY. */

/// {comment}
#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
            "            {:?} => Some(Self::{}),",
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
            "            Self::{} => {:?},",
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

impl core::fmt::Display for {class_name} {{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{
        write!(f, \"{{}}\", self.as_str())
    }}
}}

impl core::fmt::Debug for {class_name} {{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{
        write!(f, \"{{:?}}\", self.as_str())
    }}
}}
"
    )
    .unwrap();

    println!("cargo:rerun-if-changed={}", src_path);
}

#[allow(unused)]
struct WasmOpcode {
    opcode1: u8,
    opcode2: Option<u32>,
    binary: u64,
    mnemonic: String,
    identifier: String,
    params2: Vec<String>,
    comment: Option<String>,
}

impl WasmOpcode {
    fn common_comment(&self) -> String {
        let mut f = Vec::new();

        write!(f, "0x{:02X}", self.opcode1,).unwrap();
        if let Some(opcode2) = self.opcode2 {
            write!(f, " 0x{:02X}", opcode2,).unwrap();
        }

        write!(
            f,
            " {}{}",
            [format!("`{}`", self.mnemonic)]
                .iter()
                .chain(self.params_str())
                .map(|v| v.clone())
                .collect::<Vec<_>>()
                .join(" "),
            self.comment
                .as_ref()
                .map(|v| format!(" {}", v))
                .unwrap_or("".to_string()),
        )
        .unwrap();

        String::from_utf8(f).unwrap()
    }

    fn params_str(&self) -> impl Iterator<Item = &String> {
        self.params2.iter()
    }

    // fn match_id(&self) -> String {
    //     if self.params.is_empty() {
    //         self.identifier.clone()
    //     } else {
    //         format!(
    //             "{}({})",
    //             self.identifier,
    //             self.params
    //                 .iter()
    //                 .map(|_| "_")
    //                 .collect::<Vec<_>>()
    //                 .join(", ")
    //         )
    //     }
    // }

    // #[allow(dead_code)]
    // fn match_values(&self) -> String {
    //     if self.params.is_empty() {
    //         self.identifier.clone()
    //     } else {
    //         format!(
    //             "{}({})",
    //             self.identifier,
    //             self.params
    //                 .iter()
    //                 .map(|v| v.0.clone())
    //                 .collect::<Vec<_>>()
    //                 .join(", ")
    //         )
    //     }
    // }
}

fn make_opcode(os: &mut File, lines: &[String]) -> Vec<String> {
    let mut opcodes = BTreeMap::new();
    let mut opcode_order = Vec::new();
    let mut b2id = BTreeMap::new();
    let mut leading_ids = Vec::new();

    for line in lines.iter().skip(1) {
        let mut cols = line.split(",");
        let _dummy = cols.next().unwrap();
        let opcode1 = cols.next().unwrap();
        let opcode2 = cols.next().unwrap();
        let mnemonic = cols.next().unwrap().to_string();
        if opcode1.is_empty() || mnemonic.is_empty() {
            continue;
        }

        let opcode1 = match parse_with_prefix(opcode1) {
            Ok(v) => v as u8,
            Err(e) => panic!("{:?}\n {:?} {:?}", line, opcode1, e),
        };
        // .unwrap() as u8;
        let opcode2 = parse_with_prefix(opcode2).map(|v| v as u32).ok();
        let binary = ((opcode1 as u64) << 32) + (opcode2.unwrap_or_default() as u64);
        if opcode2.is_some() && !leading_ids.contains(&opcode1) {
            leading_ids.push(opcode1);
        }
        let identifier = to_camel_case_identifier(&mnemonic);

        // let mut params = Vec::new();
        // for param in cols {
        //     let mut cols = param.split(":");
        //     let var_identifier = cols.next().unwrap();
        //     let type_id = cols.next().unwrap();
        //     params.push((var_identifier.to_string(), type_id.to_string()));
        // }

        let opcode = WasmOpcode {
            opcode1: opcode1.clone(),
            opcode2: opcode2.clone(),
            binary,
            mnemonic: mnemonic.clone(),
            identifier: identifier.clone(),
            params2: Vec::new(),
            comment: None,
        };

        if opcodes.get(&identifier).is_some() {
            panic!("Duplicated mnemonic: {}", mnemonic);
        }
        opcodes.insert(identifier.clone(), opcode);
        b2id.insert(binary, identifier.clone());
        opcode_order.push(binary);
    }
    opcode_order.sort();

    write!(
        os,
        "//! WebAssembly opcodes

/* This file is generated automatically. DO NOT EDIT DIRECTLY. */

use crate::types::ValType;

/// WebAssembly opcodes
#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WasmOpcode {{
"
    )
    .unwrap();
    for opcode in opcode_order.iter() {
        let id = b2id.get(opcode).unwrap();
        let opcode = opcodes.get(id).unwrap();
        writeln!(os, "    /// {}", opcode.common_comment(),).unwrap();
        writeln!(os, "    {},", opcode.identifier,).unwrap();
        // writeln!(os, "    {} = 0x{:010x},", opcode.identifier, opcode.binary).unwrap();
    }
    write!(
        os,
        "}}

impl WasmOpcode {{
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

    pub fn from_str(v: &str) -> Option<Self> {{
        match v {{
"
    )
    .unwrap();

    for opcode in opcodes.values() {
        writeln!(
            os,
            "            \"{}\" => Some(Self::{}),",
            opcode.mnemonic, opcode.identifier,
        )
        .unwrap();
    }

    write!(
        os,
        "           _=> None,
        }}
    }}

    pub const fn leading_byte(&self) -> u8 {{
        match self {{
"
    )
    .unwrap();

    for opcode in opcodes.values() {
        writeln!(
            os,
            "            Self::{} => {:?},",
            opcode.identifier, opcode.opcode1,
        )
        .unwrap();
    }

    write!(
        os,
        "        }}
    }}

    pub const fn trailing_word(&self) -> Option<u32> {{
        match self {{
"
    )
    .unwrap();

    for opcode in opcodes.values() {
        if let Some(opcode2) = opcode.opcode2 {
            writeln!(
                os,
                "            Self::{} => Some({:?}),",
                opcode.identifier, opcode2,
            )
            .unwrap();
        }
    }

    write!(
        os,
        "            _ => None
        }}
    }}

    /// Returns stack I/O definitions for general data instructions that can define context-independent stack I/O.
    pub fn stack_io(&self) -> Option<(&[ValType], &[ValType])> {{
        use ValType::*;
        match self {{
"
    )
    .unwrap();

    let fq_i32 = "I32";
    for opcode in opcode_order.iter() {
        let id = b2id.get(opcode).unwrap();
        let opcode = opcodes.get(id).unwrap();
        if !opcode.mnemonic.contains(".") {
            continue;
        }
        let mut parts = opcode.mnemonic.split(".");
        let valtype = parts.next().unwrap();
        if !["i32", "i64", "f32", "f64"].contains(&valtype) {
            continue;
        }
        let insttype = parts.next().unwrap();

        let fqtype = valtype.to_uppercase();

        match insttype {
            // load
            "load" | "load8_s" | "load8_u" | "load16_s" | "load16_u" | "load32_s" | "load32_u" => {
                writeln!(
                    os,
                    "            Self::{} => Some((&[{}], &[{}])),",
                    opcode.identifier, fq_i32, fqtype,
                )
                .unwrap();
            }
            // store
            "store" | "store8" | "store16" | "store32" => {
                writeln!(
                    os,
                    "            Self::{} => Some((&[{}, {}], &[])),",
                    opcode.identifier, fq_i32, fqtype,
                )
                .unwrap();
            }
            // const
            "const" => {
                writeln!(
                    os,
                    "            Self::{} => Some((&[], &[{}])),",
                    opcode.identifier, fqtype,
                )
                .unwrap();
            }
            // testop
            "eqz" => {
                writeln!(
                    os,
                    "            Self::{} => Some((&[{}], &[{}])),",
                    opcode.identifier, fqtype, fq_i32,
                )
                .unwrap();
            }
            // relop
            "eq" | "ne" | "lt_s" | "lt_u" | "gt_s" | "gt_u" | "le_s" | "le_u" | "ge_s" | "ge_u"
            | "lt" | "gt" | "le" | "ge" => {
                writeln!(
                    os,
                    "            Self::{} => Some((&[{}, {}], &[{}])),",
                    opcode.identifier, fqtype, fqtype, fq_i32,
                )
                .unwrap();
            }
            // unop
            "clz" | "ctz" | "popcnt" | "abs" | "neg" | "sqrt" | "ceil" | "floor" | "trunc"
            | "nearest" | "extend8_s" | "extend16_s" | "extend32_s" => {
                writeln!(
                    os,
                    "            Self::{} => Some((&[{}], &[{}])),",
                    opcode.identifier, fqtype, fqtype,
                )
                .unwrap();
            }
            // binop
            "add" | "sub" | "mul" | "div_s" | "div_u" | "rem_s" | "rem_u" | "and" | "or"
            | "xor" | "shl" | "shr_s" | "shr_u" | "rotl" | "rotr" | "div" | "rem" | "min"
            | "max" | "copysign" => {
                writeln!(
                    os,
                    "            Self::{} => Some((&[{}, {}], &[{}])),",
                    opcode.identifier, fqtype, fqtype, fqtype,
                )
                .unwrap();
            }

            _ => {
                // cvtop
                for cvtop in [
                    "wrap",
                    "extend",
                    "trunc",
                    "trunc_sat",
                    "convert",
                    "demote",
                    "promote",
                    "reinterpret",
                ] {
                    if !insttype.starts_with(cvtop) {
                        continue;
                    }
                    let cvttype = &insttype[cvtop.len() + 1..];
                    if cvttype.len() < 3 {
                        continue;
                    }
                    let cvttype = &cvttype[..3];
                    if !["i32", "i64", "f32", "f64"].contains(&cvttype) {
                        continue;
                    }
                    let fqcvttype: String = cvttype.to_uppercase();

                    writeln!(
                        os,
                        "            Self::{} => Some((&[{}], &[{}])),",
                        opcode.identifier, fqcvttype, fqtype,
                    )
                    .unwrap();
                    break;
                }

                // writeln!(os, "// {} . {}", valtype, insttype).unwrap()
            }
        }
    }

    write!(
        os,
        "            _ => None
        }}
    }}

}}

impl core::fmt::Debug for WasmOpcode {{
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{
        f.write_str(self.as_str())
    }}
}}

impl core::fmt::Display for WasmOpcode {{
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{
        f.write_str(self.as_str())
    }}
}}
"
    )
    .unwrap();

    opcodes
        .values()
        .map(|v| v.mnemonic.clone())
        .collect::<Vec<_>>()
}

fn to_camel_case_identifier(s: &str) -> String {
    let mut output = Vec::new();
    let mut flip = true;
    for ch in s.chars() {
        match ch {
            'A'..='Z' | '0'..='9' => {
                output.push(ch);
                flip = false;
            }
            'a'..='z' => {
                if flip {
                    output.push(ch.to_ascii_uppercase())
                } else {
                    output.push(ch);
                }
                flip = false;
            }
            _ => {
                flip = true;
            }
        }
    }

    output.into_iter().collect::<String>()
}

fn parse_with_prefix(s: &str) -> core::result::Result<usize, core::num::ParseIntError> {
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
