use crate::result::CheckResult;
use typua_binder::{Symbol, TypeEnv};
use typua_parser::ast::{BinOp, Block, Expression, Stmt, TypeAst};
use typua_ty::{
    diagnostic::{Diagnostic, DiagnosticKind},
    kind::TypeKind,
};

/// entry point typechcking
pub fn typecheck(ast: &TypeAst, env: &TypeEnv) {}

fn typecheck_block(block: &Block, env: &TypeEnv) {}

fn typecheck_stmt(stmt: &Stmt, env: &mut TypeEnv) -> CheckResult {
    match stmt {
        Stmt::LocalAssign(local_assign) => {
            let mut diags: Vec<Diagnostic> = Vec::new();
            for (var, expr) in local_assign.vars.iter().zip(local_assign.exprs.iter()) {
                // let ty = eval_expr(expr, env);
                match eval_expr(expr, env) {
                    Ok(ty) => {
                        let maybe_ann_ty = env.get(&Symbol::from(var.name.clone()));
                        if let Some(ann_ty) = maybe_ann_ty
                            && ann_ty != ty
                        {
                            diags.push(Diagnostic {
                                message: format!("cannot assign `{}` to `{}`", ty, ann_ty),
                                kind: DiagnosticKind::TypeMismatch,
                            })
                        }
                    }
                    Err(diagnostic) => {
                        diags.push(diagnostic);
                    }
                }
            }
            CheckResult { diagnostics: diags }
        }
        _ => unimplemented!(),
    }
}

fn eval_expr(expr: &Expression, env: &TypeEnv) -> Result<TypeKind, Diagnostic> {
    match expr {
        Expression::Number(_) => Ok(TypeKind::Number),
        Expression::Boolean(_) => Ok(TypeKind::Boolean),
        Expression::BinaryOperator { lhs, binop, rhs } => {
            let left_ty = eval_expr(lhs, env)?;
            let right_ty = eval_expr(rhs, env)?;
            match binop {
                BinOp::Add => {
                    if left_ty == right_ty {
                        Ok(left_ty)
                    } else {
                        Err(Diagnostic {
                            message: format!("cannot add `{}` and `{}`", left_ty, right_ty),
                            kind: DiagnosticKind::TypeMismatch,
                        })
                    }
                }
                BinOp::Sub => {
                    if left_ty == right_ty {
                        Ok(left_ty)
                    } else {
                        Err(Diagnostic {
                            message: format!("cannot sub `{}` and `{}`", left_ty, right_ty),
                            kind: DiagnosticKind::TypeMismatch,
                        })
                    }
                }
                BinOp::Mul => {
                    if left_ty == right_ty {
                        Ok(left_ty)
                    } else {
                        Err(Diagnostic {
                            message: format!("cannot times `{}` and `{}`", left_ty, right_ty),
                            kind: DiagnosticKind::TypeMismatch,
                        })
                    }
                }
                BinOp::Div => {
                    if left_ty == right_ty {
                        Ok(left_ty)
                    } else {
                        Err(Diagnostic {
                            message: format!("cannot divide `{}` and `{}`", left_ty, right_ty),
                            kind: DiagnosticKind::TypeMismatch,
                        })
                    }
                }
                _ => unimplemented!(),
            }
        }
        _ => unimplemented!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use typua_parser::ast::{LuaBoolean, LuaNumber};
    use typua_span::{Position, Span};
    #[test]
    fn eval_expr_literal() {
        let env = TypeEnv::new();
        let expr = Expression::Number(LuaNumber {
            span: Span {
                start: Position::new(0, 0),
                end: Position::new(0, 0),
            },
        });
        let ret = eval_expr(&expr, &env);
        assert_eq!(ret.is_ok(), true);
        assert_eq!(ret.unwrap(), TypeKind::Number);
    }
    #[test]
    fn eval_expr_binop() {
        // normal test: number + number
        let env = TypeEnv::new();
        let expr = Expression::BinaryOperator {
            lhs: Box::new(Expression::Number(LuaNumber {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 0),
                },
            })),
            rhs: Box::new(Expression::Number(LuaNumber {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 0),
                },
            })),
            binop: BinOp::Add,
        };
        let ret = eval_expr(&expr, &env);
        assert_eq!(ret.is_ok(), true);
        assert_eq!(ret.unwrap(), TypeKind::Number);

        // abnormal test: number + bool
        let env = TypeEnv::new();
        let expr = Expression::BinaryOperator {
            lhs: Box::new(Expression::Boolean(LuaBoolean {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 0),
                },
            })),
            rhs: Box::new(Expression::Number(LuaNumber {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 0),
                },
            })),
            binop: BinOp::Add,
        };
        let ret = eval_expr(&expr, &env);
        assert_eq!(ret.is_err(), true);
        assert_eq!(
            ret.unwrap_err(),
            Diagnostic {
                message: "cannot add `boolean` and `number`".to_string(),
                kind: DiagnosticKind::TypeMismatch,
            }
        );
    }
}
