use thiserror::Error;

use super::types::{Params, Subst, Table, Type, apply, union};
use super::unify::{UnifyError, unify};

#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    Eq(Type, Type),
    // Subtyping placeholder for将来拡張
    Sub(Type, Type),
    // Callable(f, args, rets)
    Callable(Type, Vec<Type>, Vec<Type>),
    HasField(Type, String, Type),
    Index(Type, Type, Type),
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Constraints(pub Vec<Constraint>);

impl Constraints {
    pub fn push(&mut self, c: Constraint) {
        self.0.push(c);
    }
}

#[derive(Debug, Error)]
pub enum SolveError {
    #[error(transparent)]
    Unify(#[from] UnifyError),
}

#[derive(Debug, Default, Clone)]
pub struct Solver {
    pub subst: Subst,
}

impl Solver {
    pub fn new() -> Self {
        Self {
            subst: Subst::default(),
        }
    }

    pub fn solve(mut self, mut cs: Constraints) -> Result<Subst, SolveError> {
        // 単純なワークリストで Eq と Callable を処理
        while let Some(c) = cs.0.pop() {
            match c {
                Constraint::Eq(a, b) => {
                    let s2 = unify(self.subst.clone(), a, b)?;
                    self.subst = s2;
                }
                Constraint::Callable(f, args, rets) => {
                    let fun = Type::Fun {
                        params: Params::Fixed(args),
                        returns: Params::Fixed(rets),
                    };
                    let s2 = unify(self.subst.clone(), f, fun)?;
                    self.subst = s2;
                }
                Constraint::Sub(_a, _b) => {
                    // ひとまず未実装: 将来的にUnion/Optionalの縮約に分解
                    // 現段階は保留
                }
                Constraint::HasField(tab, name, ty) => {
                    let t = apply(&self.subst, tab);
                    match t {
                        Type::Table(Table::Record { mut fields, exact }) => {
                            if let Some(ft) = fields.remove(&name) {
                                let s2 = unify(self.subst.clone(), ft, ty)?;
                                self.subst = s2;
                            } else {
                                // openでもHasFieldは存在を要求する
                                return Err(SolveError::Unify(
                                    super::unify::UnifyError::Mismatch {
                                        expected: format!("field {name} exists"),
                                        actual: "missing".into(),
                                    },
                                ));
                            }
                        }
                        _ => {
                            return Err(SolveError::Unify(super::unify::UnifyError::Mismatch {
                                expected: "table(record)".into(),
                                actual: format!("{:?}", t),
                            }));
                        }
                    }
                }
                Constraint::Index(tab, key, val) => {
                    let t = apply(&self.subst, tab);
                    match t {
                        Type::Table(Table::Map { key: k, value: v }) => {
                            let s2 = unify(self.subst.clone(), *k, key)?;
                            let s3 = unify(s2, *v, val)?;
                            self.subst = s3;
                        }
                        // array sugar も Map(integer, T) として扱うため、他形式はエラー
                        _ => {
                            return Err(SolveError::Unify(super::unify::UnifyError::Mismatch {
                                expected: "table(map)".into(),
                                actual: format!("{:?}", t),
                            }));
                        }
                    }
                }
            }
        }
        Ok(self.subst)
    }
}
