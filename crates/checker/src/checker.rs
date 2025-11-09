use crate::result::{CheckResult, EvalErr, EvalType};
use typua_binder::{Symbol, TypeEnv};
use typua_parser::ast::{BinOp, Block, Expression, Stmt, TypeAst};
use typua_span::Span;
use typua_ty::{
    diagnostic::{Diagnostic, DiagnosticKind},
    kind::TypeKind,
};

/// entry point typechcking
pub fn typecheck(ast: &TypeAst, env: &TypeEnv) -> CheckResult {
    typecheck_block(&ast.block, env)
}

fn typecheck_block(block: &Block, env: &TypeEnv) -> CheckResult {
    let mut result = CheckResult::new();
    for stmt in block.stmts.iter() {
        result = CheckResult::merge(&result, &typecheck_stmt(stmt, env));
    }
    result
}

fn typecheck_stmt(stmt: &Stmt, env: &TypeEnv) -> CheckResult {
    match stmt {
        Stmt::LocalAssign(local_assign) => {
            let mut diags: Vec<Diagnostic> = Vec::new();
            for (var, expr) in local_assign.vars.iter().zip(local_assign.exprs.iter()) {
                match eval_expr(expr, env) {
                    Ok(eval_ty) => {
                        let maybe_ann_ty = env.get(&Symbol::from(var.name.clone()));
                        if let Some(ann_ty) = maybe_ann_ty
                            && !TypeKind::subtype(&eval_ty.ty, &ann_ty)
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
                    ) => match TypeKind::can_add(&left_ty, &right_ty) {
                        Ok(ty) => Ok(EvalType {
                            span: Span::new(left_span.start, right_span.end),
                            ty,
                        }),
                        Err(_e) => Err(EvalErr {
                            span: Span::new(left_span.start.clone(), right_span.end.clone()),
                            diagnostic: Diagnostic {
                                message: format!("cannot add `{}` and `{}`", left_ty, right_ty),
                                kind: DiagnosticKind::TypeMismatch,
                                span: Span::new(left_span.start, right_span.end),
                            },
                        }),
                    },
                    (_, _) => unimplemented!(),
                },
                _ => unimplemented!(),
            }
        }
        Expression::Var { span, symbol } => match env.get(&Symbol::new(symbol.clone())) {
            Some(ty) => Ok(EvalType {
                span: span.clone(),
                ty,
            }),
            None => Err(EvalErr {
                span: span.clone(),
                diagnostic: Diagnostic {
                    span: span.clone(),
                    kind: DiagnosticKind::NotDeclaredVariable,
                    message: format!("'{}' is not declared", *symbol),
                },
            }),
        },
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

        // TypeMismatch: number + bool
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
        // normal test: binop vars
        let mut env = TypeEnv::new();
        let _ = env.insert(&Symbol::new("x".to_string()), &TypeKind::Number);
        let _ = env.insert(&Symbol::new("y".to_string()), &TypeKind::Number);
        let expr = Expression::BinaryOperator {
            lhs: Box::new(Expression::Var {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 0),
                },
                symbol: "x".to_string(),
            }),
            rhs: Box::new(Expression::Var {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 10),
                },
                symbol: "y".to_string(),
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
        // abnormal test: binop vars
        let mut env = TypeEnv::new();
        let _ = env.insert(&Symbol::new("x".to_string()), &TypeKind::Number);
        let _ = env.insert(&Symbol::new("y".to_string()), &TypeKind::Boolean);
        let expr = Expression::BinaryOperator {
            lhs: Box::new(Expression::Var {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 0),
                },
                symbol: "x".to_string(),
            }),
            rhs: Box::new(Expression::Var {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 10),
                },
                symbol: "y".to_string(),
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
                    message: "cannot add `number` and `boolean`".to_string(),
                    kind: DiagnosticKind::TypeMismatch,
                    span: Span::new(Position::new(0, 0), Position::new(0, 10)),
                }
            }
        );
    }
    #[test]
    fn eval_expr_var() {
        let mut env = TypeEnv::new();
        let _ = env.insert(&Symbol::new("x".to_string()), &TypeKind::Number);
        // normal test: number
        let expr = Expression::Var {
            span: Span::new(Position::new(0, 0), Position::new(0, 10)),
            symbol: "x".to_string(),
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
        // abnormal test: number
        let expr = Expression::Var {
            span: Span::new(Position::new(0, 0), Position::new(0, 10)),
            symbol: "y".to_string(),
        };
        let ret = eval_expr(&expr, &env);
        assert_eq!(ret.is_err(), true);
        assert_eq!(
            ret.unwrap_err(),
            EvalErr {
                span: Span::new(Position::new(0, 0), Position::new(0, 10)),
                diagnostic: Diagnostic {
                    span: Span::new(Position::new(0, 0), Position::new(0, 10)),
                    kind: DiagnosticKind::NotDeclaredVariable,
                    message: "'y' is not declared".to_string()
                }
            }
        );
    }
}
