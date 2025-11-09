// use std::collections::HashMap;
use im::HashMap;
use typua_ty::TypeKind;
use typua_ty::{BindError, TypuaError};

#[derive(Debug, Clone)]
pub struct TypeEnv {
    vars: HashMap<Symbol, TypeKind>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }
    pub fn insert(&mut self, symbol: &Symbol, ty: &TypeKind) -> Result<(), TypuaError> {
        match self.vars.insert(symbol.clone(), ty.clone()) {
            Some(_) => Ok(()),
            None => Err(TypuaError::Bind(BindError::InsertionFailed(format!(
                "{}",
                symbol
            )))),
        }
    }
    pub fn get(&self, symbol: &Symbol) -> Option<TypeKind> {
        self.vars.get(symbol).cloned()
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct Symbol {
    pub val: String,
}

impl Symbol {
    pub fn new(val: String) -> Self {
        Self {
            val
        }
    }
}

impl From<String> for Symbol {
    fn from(s: String) -> Self {
        Symbol { val: s }
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.val)
    }
}
