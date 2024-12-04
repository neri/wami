//! Indexes

macro_rules! decl_index {
    ($type_name:ident, $prefix:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $type_name(pub(super) u32);

        impl $type_name {
            #[inline]
            pub const fn as_usize(&self) -> usize {
                self.0 as usize
            }

            #[inline]
            pub const fn get(&self) -> u32 {
                self.0
            }
        }

        impl core::fmt::Debug for $type_name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}({})", stringify!($prefix), self.0)
            }
        }
    };
}

decl_index!(TypeIndex, Type);
decl_index!(FuncIndex, Func);
decl_index!(TableIndex, Table);
decl_index!(MemoryIndex, Memory);
decl_index!(GlobalIndex, Global);
decl_index!(LocalIndex, Local);
