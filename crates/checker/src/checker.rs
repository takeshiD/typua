use crate::result::{CheckResult, EvalErr, EvalType};
use typua_binder::{Symbol, TypeEnv};
use typua_parser::ast::{BinOp, Block, Expression, Stmt, TypeAst};
use typua_span::{Position, Span};
use typua_ty::{
    diagnostic::{Diagnostic, DiagnosticKind},
    kind::TypeKind,
};

/// entry point typechcking
pub fn typecheck(ast: &TypeAst, env: &TypeEnv) {}

fn typecheck_block(block: &Block, env: &TypeEnv) {}

fn typecheck_stmt(stmt: &Stmt, env: &TypeEnv) -> CheckResult {
    match stmt {
        Stmt::LocalAssign(local_assign) => {
            let mut diags: Vec<Diagnostic> = Vec::new();
            for (var, expr) in local_assign.vars.iter().zip(local_assign.exprs.iter()) {
                match eval_expr(expr, env) {
                    Ok(eval_ty) => {
                        let maybe_ann_ty = env.get(&Symbol::from(var.name.clone()));
                        if let Some(ann_ty) = maybe_ann_ty
                            && ann_ty != eval_ty.ty
                        {
                            diags.push(Diagnostic {
                                message: format!("cannot assign `{}` to `{}`", eval_ty.ty, ann_ty),
                                kind: DiagnosticKind::TypeMismatch,
                                span: eval_ty.span,
                            })
                        }
                    }
                    Err(eval_err) => {
                        diags.push(eval_err.diagnostic);
                    }
                }
            }
            CheckResult { diagnostics: diags }
        }
        _ => unimplemented!(),
    }
}

fn eval_expr(expr: &Expression, env: &TypeEnv) -> Result<EvalType, EvalErr> {
    match expr {
        Expression::Number { span } => Ok(EvalType {
            span: span.clone(),
            ty: TypeKind::Number,
        }),
        Expression::Boolean { span } => Ok(EvalType {
            span: span.clone(),
            ty: TypeKind::Boolean,
        }),
        Expression::BinaryOperator { lhs, binop, rhs } => {
            let lhs_eval = eval_expr(lhs, env);
            let rhs_eval = eval_expr(rhs, env);
            match binop {
                BinOp::Add(_) => match (lhs_eval, rhs_eval) {
                    (
                        Ok(EvalType {
                            span: left_span,
                            ty: left_ty,
                        }),
                        Ok(EvalType {
                            span: right_span,
                            ty: right_ty,
                        }),
                    ) => {
                        if left_ty == right_ty {
                            Ok(EvalType {
                                span: Span::new(left_span.start, right_span.end),
                                ty: left_ty,
                            })
                        } else {
                            Err(EvalErr {
                                span: Span::new(left_span.start.clone(), right_span.end.clone()),
                                diagnostic: Diagnostic {
                                    message: format!("cannot add `{}` and `{}`", left_ty, right_ty),
                                    kind: DiagnosticKind::TypeMismatch,
                                    span: Span::new(left_span.start, right_span.end),
                                },
                            })
                        }
                    }
                    (_, _) => unimplemented!(),
                },
                _ => unimplemented!(),
            }
        }
        Expression::Var(symbol) => {
            match env.get(&Symbol::new(*symbol)) {
                Some(ty) => {
                    Ok(EvalType {
                        span: 
                        ty,
                    })
                }
            }
        }
        _ => unimplemented!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use typua_span::{Position, Span};
    #[test]
    fn eval_expr_literal() {
        let env = TypeEnv::new();
        let expr = Expression::Number {
            span: Span {
                start: Position::new(0, 0),
                end: Position::new(0, 0),
            },
        };
        let ret = eval_expr(&expr, &env);
        assert_eq!(ret.is_ok(), true);
        assert_eq!(
            ret.unwrap(),
            EvalType {
                span: Span::new(Position::new(0, 0), Position::new(0, 0)),
                ty: TypeKind::Number
            }
        );
    }
    #[test]
    fn eval_expr_binop() {
        // normal test: number + number
        let env = TypeEnv::new();
        let expr = Expression::BinaryOperator {
            lhs: Box::new(Expression::Number {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 0),
                },
            }),
            rhs: Box::new(Expression::Number {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 10),
                },
            }),
            binop: BinOp::Add(Span::new(Position::new(0, 0), Position::new(0, 0))),
        };
        let ret = eval_expr(&expr, &env);
        assert_eq!(ret.is_ok(), true);
        assert_eq!(
            ret.unwrap(),
            EvalType {
                span: Span::new(Position::new(0, 0), Position::new(0, 10)),
                ty: TypeKind::Number,
            }
        );

        // typecheck diagnostic test: number + bool
        let env = TypeEnv::new();
        let expr = Expression::BinaryOperator {
            lhs: Box::new(Expression::Boolean {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 0),
                },
            }),
            rhs: Box::new(Expression::Number {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 10),
                },
            }),
            binop: BinOp::Add(Span::new(Position::new(0, 0), Position::new(0, 0))),
        };
        let ret = eval_expr(&expr, &env);
        assert_eq!(ret.is_err(), true);
        assert_eq!(
            ret.unwrap_err(),
            EvalErr {
                span: Span::new(Position::new(0, 0), Position::new(0, 10)),
                diagnostic: Diagnostic {
                    message: "cannot add `boolean` and `number`".to_string(),
                    kind: DiagnosticKind::TypeMismatch,
                    span: Span::new(Position::new(0, 0), Position::new(0, 10)),
                }
            }
        );
    }
}
