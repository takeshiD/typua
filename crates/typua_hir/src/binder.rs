use crate::hashmap::FastMap;
use crate::hir;
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
struct ScopeId(u32);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolId(u32);

#[derive(Clone)]
pub struct BoundSymbol {
    name: String,
    ann: hir::TypeAnnotation,
    scope: ScopeId,
}

struct TypeDeclId(u32);
struct TypeDecl {
    name: String,
}

struct SymbolEnv {
    parent: Option<ScopeId>,
    env: FastMap<SymbolId, BoundSymbol>,
}

impl SymbolEnv {
    pub fn new(parent: Option<ScopeId>) -> Self {
        Self {
            parent,
            env: FastMap::default(),
        }
    }
    pub fn set_parent(&mut self, new_parent: ScopeId) -> Option<ScopeId> {
        let ret = self.parent.take();
        self.parent = Some(new_parent);
        ret
    }
    pub fn insert(&mut self, symbol: BoundSymbol) {
        let new_id = SymbolId(self.env.len() as u32);
        self.env.insert(new_id, symbol);
    }
}

struct TypeEnv {
    parent: Option<ScopeId>,
    env: FastMap<TypeDecl, TypeDeclId>,
}

pub struct Binder {
    cur_scope: ScopeId,
    symbol_envs: BTreeMap<ScopeId, SymbolEnv>,
    // TODO
    // type_envs: BTreeMap<ScopeId, TypeEnv>,
}
// ```lua
// ---@class Container
// ---@field x number
// ---@field y fun(number, number): string
//
// ---@type number
// local x = 1
// local y = x
// local double = function(x)
//      return x * 2
// end
// ---@param a number
// ---@param b number
// local maybe_add = function(a, b)
//      return a + b
// end
// ```
// symbol_envs = {
//  Scope(0): {
//    parent: None,
//    env: {
//      Symbol{name: "x",      ann: Type::Number,     }: SymbolId(0),
//      Symbol{name: "y",      ann: Type::Unannotated }: SymbolId(1),
//      Symbol{name: "double", ann: Type::Unannotated }: SymbolId(2),
//      Symbol{name: "maybe_add",
//             ann: Type::Function{
//                   params: [Param("a", Number), Param("b", Number)],
//                   return: Type::Unannotated
//                  }
//             }: SymbolId(3),
//    },
//  },
//  Scope(1): {
//    parent: Some(Scope(0)),
//    env: {
//      Symbol{name: "x",      ann: Type::Number,     }: SymbolId(3),
//    },
//  },
//  Scope(2): {
//    parent: Some(Scope(0)),
//    env: {
//      Symbol{name: "a",      ann: Type::Number,     }: SymbolId(4),
//    },
//  }
// }
// typedecls = [
//  TypeDecl(0) { kind: Class, fields: [Field {name: "x", ty: Number}, Field{name:"y", ty: Function}] },
// ]
impl Binder {
    pub fn new() -> Self {
        let mut symbol_envs = BTreeMap::new();
        symbol_envs.insert(ScopeId(0), SymbolEnv::new(None));
        Self {
            cur_scope: ScopeId(0),
            symbol_envs: BTreeMap::new(),
            // TODO
            // type_envs: BTreeMap::new(),
        }
    }
    pub fn binding(&mut self, body: &hir::HirBody) {
        body.roots()
            .for_each(|stmt_id| match body.find_stmt(stmt_id.clone()) {
                hir::Stmt::LocalAssign { binds, inits } => self.binding_local(binds, inits),
                _ => todo!(),
            });
    }
    fn binding_local(&mut self, binds: &[hir::LocalBinding], _: &[hir::ExprId]) {
        if let Some(env) = self.symbol_envs.get_mut(&self.cur_scope) {
            binds.iter().for_each(|b| {
                let symbol = BoundSymbol {
                    name: b.name.to_string(),
                    ann: b.ann.clone(),
                    scope: self.cur_scope,
                };
                env.insert(symbol);
            });
        };
    }
}
