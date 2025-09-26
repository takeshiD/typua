use super::types::{Params, Subst, Table, TyVarId, Type, apply, union};

#[derive(Debug, thiserror::Error)]
pub enum UnifyError {
    #[error("type mismatch: expected {expected}, actual {actual}")]
    Mismatch { expected: String, actual: String },
    #[error("occurs check failed: {var} occurs in {in_type}")]
    Occurs { var: String, in_type: String },
}

fn occurs(v: TyVarId, t: &Type) -> bool {
    match t {
        Type::Var(x) => *x == v,
        Type::Prim(_) => false,
        Type::Optional(t1) => occurs(v, t1),
        Type::Tuple(vs) | Type::Union(vs) => vs.iter().any(|t| occurs(v, t)),
        Type::Fun { params, returns } => {
            occurs_in_params(v, params) || occurs_in_params(v, returns)
        }
        Type::Table(Table::Record { fields, .. }) => fields.values().any(|t| occurs(v, t)),
        Type::Table(Table::Map { key, value }) => occurs(v, key) || occurs(v, value),
    }
}

fn occurs_in_params(v: TyVarId, p: &Params) -> bool {
    match p {
        Params::Fixed(vs) => vs.iter().any(|t| occurs(v, t)),
        Params::VarArg(vs, t) => vs.iter().any(|x| occurs(v, x)) || occurs(v, t),
    }
}

pub fn unify(s: Subst, a: Type, b: Type) -> Result<Subst, UnifyError> {
    match (a, b) {
        (Type::Var(v), t) | (t, Type::Var(v)) => bind(s, v, t),
        (Type::Prim(p), Type::Prim(q)) if p == q => Ok(s),
        (Type::Optional(x), Type::Optional(y)) => unify(s, *x, *y),
        (Type::Tuple(xs), Type::Tuple(ys)) => unify_vec(s, xs, ys),
        (Type::Union(xs), Type::Union(ys)) => unify_vec(s, xs, ys),
        (
            Type::Fun {
                params: p1,
                returns: r1,
            },
            Type::Fun {
                params: p2,
                returns: r2,
            },
        ) => {
            let s = unify_params(s, p1, p2)?;
            // 戻り値は右側に現れる型変数へ束縛が流れるように順序を反転
            unify_params(s, r2, r1)
        }
        (
            Type::Table(Table::Record {
                fields: f1,
                exact: e1,
            }),
            Type::Table(Table::Record {
                fields: f2,
                exact: e2,
            }),
        ) => {
            // exact 同士は同名フィールド一致。open が混じる場合は共通フィールドのみ検査。
            let mut s_acc = s;
            for (k, v1) in f1.iter() {
                if let Some(v2) = f2.get(k) {
                    s_acc = unify(s_acc, v1.clone(), v2.clone())?;
                } else if e1 {
                    return Err(UnifyError::Mismatch {
                        expected: format!("field {k} exists"),
                        actual: "missing".to_string(),
                    });
                }
            }
            if e2 {
                for (k, _) in f2.iter() {
                    if !f1.contains_key(k) {
                        return Err(UnifyError::Mismatch {
                            expected: format!("field {k} exists"),
                            actual: "missing".to_string(),
                        });
                    }
                }
            }
            Ok(s_acc)
        }
        (
            Type::Table(Table::Map { key: k1, value: v1 }),
            Type::Table(Table::Map { key: k2, value: v2 }),
        ) => {
            let s = unify(s, *k1, *k2)?;
            unify(s, *v1, *v2)
        }
        // Optional sugar: unify(T, U?) => unify(T|nil, U|nil)
        (Type::Optional(t), other) | (other, Type::Optional(t)) => unify(
            s,
            union(vec![*t, Type::Prim(super::types::Prim::Nil)]),
            union(vec![other, Type::Prim(super::types::Prim::Nil)]),
        ),
        (x, y) => Err(UnifyError::Mismatch {
            expected: format!("{:?}", x),
            actual: format!("{:?}", y),
        }),
    }
}

fn unify_vec(mut s: Subst, xs: Vec<Type>, ys: Vec<Type>) -> Result<Subst, UnifyError> {
    if xs.len() != ys.len() {
        return Err(UnifyError::Mismatch {
            expected: format!("len {}", xs.len()),
            actual: format!("len {}", ys.len()),
        });
    }
    let mut it = xs.into_iter().zip(ys.into_iter());
    while let Some((a, b)) = it.next() {
        s = unify(s, a, b)?;
    }
    Ok(s)
}

fn unify_params(s: Subst, a: Params, b: Params) -> Result<Subst, UnifyError> {
    use Params::*;
    match (a, b) {
        (Fixed(xs), Fixed(ys)) => unify_vec(s, xs, ys),
        (VarArg(xs, xv), VarArg(ys, yv)) => {
            let s = unify_vec(s, xs, ys)?;
            unify(s, *xv, *yv)
        }
        (Fixed(xs), VarArg(mut ys, yv)) | (VarArg(mut ys, yv), Fixed(xs)) => {
            // expand vararg to match length (last element repeats)
            if ys.is_empty() {
                return Err(UnifyError::Mismatch {
                    expected: "vararg non-empty".into(),
                    actual: "empty".into(),
                });
            }
            let mut ys_full = ys;
            if xs.len() >= ys_full.len() {
                let last = yv.clone();
                ys_full.extend(std::iter::repeat((*last).clone()).take(xs.len() - ys_full.len()));
            }
            unify_vec(s, xs, ys_full)
        }
    }
}

fn bind(mut s: Subst, v: TyVarId, t: Type) -> Result<Subst, UnifyError> {
    if let Type::Var(w) = &t {
        if *w == v {
            return Ok(s);
        }
    }
    if occurs(v, &t) {
        return Err(UnifyError::Occurs {
            var: format!("{:?}", v),
            in_type: format!("{:?}", t),
        });
    }
    // apply current substitution before binding to keep canonical
    let t_applied = apply(&s, t);
    s.insert(v, t_applied);
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typing::types::{TypeDisplay, ftv};
    use pretty_assertions::assert_eq;

    fn v(i: u32) -> Type {
        Type::Var(TyVarId(i))
    }

    #[test]
    fn unify_vars() {
        let s = Subst::default();
        let s = unify(s, v(0), v(1)).unwrap();
        assert!(s.get(&TyVarId(0)).is_some() || s.get(&TyVarId(1)).is_some());
    }

    #[test]
    fn occurs_error() {
        let s = Subst::default();
        let t = Type::Tuple(vec![v(0)]);
        let err = unify(s, v(0), t).unwrap_err();
        match err {
            UnifyError::Occurs { .. } => {}
            e => panic!("unexpected {e:?}"),
        }
    }
}
