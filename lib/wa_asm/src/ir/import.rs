use crate::*;
use ast::global::GlobalType;
use ir::{
    Module, WasmSectionId,
    index::{FuncIndex, GlobalIndex, MemoryIndex, TableIndex, TypeIndex},
    types::IdType,
};
use leb128::{Leb128Writer, WriteError, WriteLeb128};

#[derive(Default)]
pub struct Imports(pub(super) Vec<Import>);

impl Imports {
    pub(super) fn convert(
        module: &mut Module,
        ast_module: &ast::AstModule,
    ) -> Result<(), AssembleError> {
        for ast_import in ast_module.imports() {
            let mod_name = ast_import.mod_name().get();
            let name = ast_import.name().get();
            match ast_import.desc() {
                ast::import::ImportDescriptor::Func(func) => {
                    Self::add_ast_func(module, func.identifier(), mod_name, name, func.typeuse())?
                }
                _ => todo!(),
            }
        }
        for ast_func in ast_module.functions() {
            if let Some(&ast::types::ExtVis::Import(ref ast_import)) = ast_func.vis() {
                Self::add_ast_func(
                    module,
                    ast_func.identifier(),
                    ast_import.mod_name().get(),
                    ast_import.name().get(),
                    ast_func.typeuse(),
                )?;
            }
        }
        for ast_global in ast_module.globals() {
            if let Some(ast::types::ExtVis::Import(ref ast_import)) = ast_global.vis {
                Self::add_ast_global(
                    module,
                    ast_global.id.as_ref(),
                    ast_import.mod_name().get(),
                    ast_import.name().get(),
                    &ast_global.global_type,
                )?;
            }
        }
        Ok(())
    }

    pub fn write_to_wasm(&self, writer: &mut Leb128Writer) -> Result<WasmSectionId, WriteError> {
        if self.0.len() > 0 {
            writer.write(self.0.len())?;
            for item in self.0.iter() {
                writer.write(&item.mod_name)?;
                writer.write(&item.name)?;
                match item.desc {
                    ImportDesc::Func(v) => {
                        writer.write(0)?;
                        writer.write(v.as_usize())?;
                    }
                    ImportDesc::Table(v) => {
                        writer.write(1)?;
                        writer.write(v.as_usize())?;
                    }
                    ImportDesc::Memory(v) => {
                        writer.write(2)?;
                        writer.write(v.as_usize())?;
                    }
                    ImportDesc::Global(v) => {
                        writer.write(3)?;
                        v.write_to_wasm(writer)?;
                    }
                }
            }
        }
        Ok(WasmSectionId::Import)
    }

    fn add_ast_func(
        module: &mut Module,
        identifier: Option<&ast::identifier::Identifier>,
        mod_name: &str,
        name: &str,
        typeuse: &ast::types::TypeUse,
    ) -> Result<(), AssembleError> {
        let mod_name = mod_name.to_owned();
        let name = name.to_owned();
        let typeidx = module.define_typeuse(typeuse)?;

        if let Some(id) = identifier {
            module.register_ast_name(
                id,
                IdType::Func(FuncIndex(module.imports.num_import_funcs() as u32)),
            )?;
        }

        module.import_funcs.push(typeidx);
        module.imports.0.push(Import {
            mod_name,
            name,
            desc: ImportDesc::Func(typeidx),
        });

        Ok(())
    }

    fn add_ast_global(
        module: &mut Module,
        identifier: Option<&ast::identifier::Identifier>,
        mod_name: &str,
        name: &str,
        global_type: &ast::global::GlobalType,
    ) -> Result<(), AssembleError> {
        let mod_name = mod_name.to_owned();
        let name = name.to_owned();

        if let Some(id) = identifier {
            module.register_ast_name(
                id,
                IdType::Global(GlobalIndex(module.imports.num_import_globals() as u32)),
            )?;
        }

        module.imports.0.push(Import {
            mod_name,
            name,
            desc: ImportDesc::Global(*global_type),
        });

        Ok(())
    }

    pub fn num_import_funcs(&self) -> usize {
        self.0
            .iter()
            .filter(|v| matches!(v.desc, ImportDesc::Func(_)))
            .count()
    }

    pub fn num_import_globals(&self) -> usize {
        self.0
            .iter()
            .filter(|v| matches!(v.desc, ImportDesc::Global(_)))
            .count()
    }
}

impl core::fmt::Debug for Imports {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}

#[derive(Debug)]
pub struct Import {
    pub mod_name: String,
    pub name: String,
    pub desc: ImportDesc,
}

#[derive(Debug, Clone, Copy)]
pub enum ImportDesc {
    Func(TypeIndex),
    Table(TableIndex),
    Memory(MemoryIndex),
    Global(GlobalType),
}
