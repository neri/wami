#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmSectionId {
    Custom,
    Type,
    Import,
    Function,
    Table,
    Memory,
    Global,
    Export,
    Start,
    Element,
    Code,
    Data,
    DataCount,
}
