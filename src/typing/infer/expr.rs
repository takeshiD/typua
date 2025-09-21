use crate::typing::constraints::{Constraint, Constraints, Solver};
use crate::typing::types::{apply, generalize, instantiate, Params, Prim, Scheme, Subst, Type, TyVarId};

#[derive(Debug, thiserror::Error)]
pub enum InferError {
    #[error(transparent)]
    Solve(#[from] crate::typing::constraints::SolveError),
}

pub struct InferCtx {
    pub next_id: u32,
}

impl InferCtx {
    pub fn new() -> Self { Self { next_id: 0 } }
    pub fn fresh(&mut self) -> TyVarId { let id = self.next_id; self.next_id += 1; TyVarId(id) }
}

// 関数呼び出しの推論: callee の型と引数の型から戻り型を推論
// callee はすでにインスタンス化済みの型を想定
pub fn infer_call_return(mut callee: Type, args: Vec<Type>) -> Result<(Type, Subst), InferError> {
    let mut ctx = InferCtx::new();
    let ret = Type::Var(ctx.fresh());
    let mut cs = Constraints::default();
    cs.push(Constraint::Callable(callee, args, vec![ret.clone()]));
    let s = Solver::new().solve(cs)?;
    Ok((apply(&s, ret), s))
}

// フィールドアクセス: rec.name の型を推論
pub fn infer_field_access(record: Type, field: &str) -> Result<(Type, Subst), InferError> {
    let mut ctx = InferCtx::new();
    let out = Type::Var(ctx.fresh());
    let mut cs = Constraints::default();
    cs.push(Constraint::HasField(record, field.to_string(), out.clone()));
    let s = Solver::new().solve(cs)?;
    Ok((apply(&s, out), s))
}

// 添字アクセス: tab[key] の型を推論
pub fn infer_index_access(table: Type, key: Type) -> Result<(Type, Subst), InferError> {
    let mut ctx = InferCtx::new();
    let out = Type::Var(ctx.fresh());
    let mut cs = Constraints::default();
    cs.push(Constraint::Index(table, key, out.clone()));
    let s = Solver::new().solve(cs)?;
    Ok((apply(&s, out), s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn prim(p: Prim) -> Type { Type::Prim(p) }
    fn v(id: u32) -> Type { Type::Var(TyVarId(id)) }

    #[test]
    fn infer_identity_call() {
        // id : ∀T. fun(T): T
        let t_fun = Type::Fun { params: Params::Fixed(vec![v(0)]), returns: Params::Fixed(vec![v(0)]) };
        let scheme = Scheme::new([TyVarId(0)], t_fun);
        let mut next = 10u32;
        let mut fresh = || { let id = TyVarId(next); next += 1; id };
        let callee = instantiate(&mut fresh, &scheme);
        let (ret, _s) = infer_call_return(callee, vec![prim(Prim::Number)]).unwrap();
        assert_eq!(format!("{}", crate::typing::types::TypeDisplay::new(&ret)), "number");
    }

    #[test]
    fn infer_map_call() {
        // map : ∀T,U. fun(fun(T):U, T[]): U[]  
        let t = Type::Var(TyVarId(0));
        let u = Type::Var(TyVarId(1));
        let f = Type::Fun { params: Params::Fixed(vec![t.clone()]), returns: Params::Fixed(vec![u.clone()]) };
        let arr_t = Type::Table(crate::typing::types::Table::Map { key: Box::new(prim(Prim::Integer)), value: Box::new(t.clone()) });
        let arr_u = Type::Table(crate::typing::types::Table::Map { key: Box::new(prim(Prim::Integer)), value: Box::new(u.clone()) });
        let map_ty = Type::Fun { params: Params::Fixed(vec![f, arr_t.clone()]), returns: Params::Fixed(vec![arr_u.clone()]) };
        let scheme = Scheme::new([TyVarId(0), TyVarId(1)], map_ty);
        // instantiate
        let mut next = 20u32;
        let mut fresh = || { let id = TyVarId(next); next += 1; id };
        let callee = instantiate(&mut fresh, &scheme);
        // argument types: fun(number): string  と number[]
        let fn_arg = Type::Fun { params: Params::Fixed(vec![prim(Prim::Number)]), returns: Params::Fixed(vec![prim(Prim::String)]) };
        let arr_num = Type::Table(crate::typing::types::Table::Map { key: Box::new(prim(Prim::Integer)), value: Box::new(prim(Prim::Number)) });
        let (ret, _s) = infer_call_return(callee, vec![fn_arg, arr_num]).unwrap();
        assert_eq!(format!("{}", crate::typing::types::TypeDisplay::new(&ret)), "{ [integer]: string }");
    }

    #[test]
    fn infer_record_field() {
        // { x: number, y: string } . x => number
        let rec = Type::Table(crate::typing::types::Table::Record {
            fields: [
                ("x".to_string(), prim(Prim::Number)),
                ("y".to_string(), prim(Prim::String)),
            ]
            .into_iter()
            .collect(),
            exact: true,
        });
        let (ret, _s) = infer_field_access(rec, "x").unwrap();
        assert_eq!(format!("{}", crate::typing::types::TypeDisplay::new(&ret)), "number");
    }

    #[test]
    fn infer_index_array() {
        // number[] の index(integer) は number
        let arr_num = Type::Table(crate::typing::types::Table::Map { key: Box::new(prim(Prim::Integer)), value: Box::new(prim(Prim::Number)) });
        let (ret, _s) = infer_index_access(arr_num, prim(Prim::Integer)).unwrap();
        assert_eq!(format!("{}", crate::typing::types::TypeDisplay::new(&ret)), "number");
    }
}
