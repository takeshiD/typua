use std::collections::BTreeMap;

use crate::typing::types::{Scheme, TyVarId, Type, generalize, instantiate};

#[derive(Debug, Default, Clone)]
pub struct TypeEnv {
    pub bindings: BTreeMap<String, Scheme>,
    next_id: u32,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            bindings: BTreeMap::new(),
            next_id: 0,
        }
    }
    pub fn fresh(&mut self) -> TyVarId {
        let id = self.next_id;
        self.next_id += 1;
        TyVarId(id)
    }
    pub fn get(&mut self, name: &str) -> Option<Type> {
        let s = self.bindings.get(name).cloned()?;
        Some(instantiate(&mut || self.fresh(), &s))
    }
    pub fn insert_mono(&mut self, name: impl Into<String>, ty: Type) {
        self.bindings.insert(
            name.into(),
            Scheme {
                vars: Default::default(),
                body: ty,
            },
        );
    }
    pub fn insert_gen(&mut self, name: impl Into<String>, ty: Type) {
        let scheme = generalize(&self.bindings, ty);
        self.bindings.insert(name.into(), scheme);
    }
}
