use crate::hashmap::FastMap;

struct SymbolId(u32);
struct Symbol(String);
struct TypeDeclId(u32);
struct TypeDecl(String);

pub struct Binder {
    symbol_env: Vec<FastMap<Symbol, SymbolId>>,
    type_env: Vec<FastMap<TypeDecl, TypeDeclId>>,
}

impl Binder {
    pub fn new() -> Self {
        Self {
            symbol_env: Vec::new(),
            type_env: Vec::new(),
        }
    }
}

impl Binder {}
