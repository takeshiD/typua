use typua_binder::{Symbol, TypeEnv};
use typua_parser::ast::{BinOp, Block, Expression, Stmt, TypeAst, Var};
use typua_ty::{
    BoolLiteral,
    diagnostic::{Diagnostic, DiagnosticKind},
    kind::TypeKind,
};

use crate::result::CheckResult;

#[derive(Debug)]
pub struct Checker {
    env: TypeEnv,
    diagnostics: Vec<Diagnostic>,
}

impl Checker {
    pub fn new(env: TypeEnv) -> Self {
        Self {
            env,
            diagnostics: Vec::new(),
        }
    }
    pub fn typecheck(mut self, ast: &TypeAst) -> CheckResult {
        self.typecheck_block(&ast.block);
        CheckResult {
            diagnostics: self.diagnostics,
        }
    }
    fn typecheck_block(&mut self, block: &Block) {
        for stmt in block.stmts.iter() {
            self.typecheck_stmt(stmt);
        }
    }
    fn typecheck_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::LocalAssign(local_assign) => {
                for (var, expr) in local_assign.vars.iter().zip(local_assign.exprs.iter()) {
                    let eval_ty = self.eval_expr(expr);
                    let maybe_ann_ty = self.env.get(&Symbol::from(var.name.clone()));
                    if let Some(ann_ty) = maybe_ann_ty
                        && !TypeKind::subtype(&eval_ty, &ann_ty)
                    {
                        self.diagnostics.push(Diagnostic {
                            message: format!("cannot assign `{}` to `{}`", eval_ty, ann_ty),
                            kind: DiagnosticKind::TypeMismatch,
                            span: var.span.clone(),
                        });
                    }
                }
            }
            _ => unimplemented!(),
        }
    }
    fn eval_expr(&mut self, expr: &Expression) -> TypeKind {
        match expr {
            Expression::Number { .. } => TypeKind::Number,
            Expression::Boolean { val, .. } => match val.as_str() {
                "false" => TypeKind::Boolean(BoolLiteral::False),
                "true" => TypeKind::Boolean(BoolLiteral::True),
                _ => unimplemented!("invalid bool literal"),
            },
            Expression::Nil { .. } => TypeKind::Nil,
            Expression::String { .. } => TypeKind::String,
            Expression::BinaryOperator { lhs, binop, rhs } => self.eval_binop(binop, lhs, rhs),
            Expression::Var { var } => self.eval_var(var),
            _ => unimplemented!(),
        }
    }
    fn eval_binop(&mut self, binop: &BinOp, lhs: &Expression, rhs: &Expression) -> TypeKind {
        let lhs_ty = self.eval_expr(lhs);
        let rhs_ty = self.eval_expr(rhs);
        match binop {
            BinOp::Add(op_span) => match TypeKind::try_add(&lhs_ty, &rhs_ty) {
                Ok(ty) => ty,
                Err(_) => {
                    let diagnostic = Diagnostic {
                        span: op_span.clone(),
                        kind: DiagnosticKind::TypeMismatch,
                        message: format!("cannot add `{}` and `{}`", lhs_ty, rhs_ty),
                    };
                    self.diagnostics.push(diagnostic);
                    TypeKind::Unknown
                }
            },
            BinOp::Sub(op_span) => match TypeKind::try_sub(&lhs_ty, &rhs_ty) {
                Ok(ty) => ty,
                Err(_) => {
                    let diagnostic = Diagnostic {
                        span: op_span.clone(),
                        kind: DiagnosticKind::TypeMismatch,
                        message: format!("cannot subtract `{}` and `{}`", lhs_ty, rhs_ty),
                    };
                    self.diagnostics.push(diagnostic);
                    TypeKind::Unknown
                }
            },
            BinOp::And(op_span) => match TypeKind::try_and(&lhs_ty, &rhs_ty) {
                Ok(ty) => ty,
                Err(_) => {
                    let diagnostic = Diagnostic {
                        span: op_span.clone(),
                        kind: DiagnosticKind::TypeMismatch,
                        message: format!("cannot and `{}` and `{}`", lhs_ty, rhs_ty),
                    };
                    self.diagnostics.push(diagnostic);
                    TypeKind::Unknown
                }
            },
            BinOp::Or(op_span) => match TypeKind::try_or(&lhs_ty, &rhs_ty) {
                Ok(ty) => ty,
                Err(_) => {
                    let diagnostic = Diagnostic {
                        span: op_span.clone(),
                        kind: DiagnosticKind::TypeMismatch,
                        message: format!("cannot and `{}` and `{}`", lhs_ty, rhs_ty),
                    };
                    self.diagnostics.push(diagnostic);
                    TypeKind::Unknown
                }
            },
            _ => unimplemented!(),
        }
    }
    fn eval_var(&mut self, var: &Var) -> TypeKind {
        match self.env.get(&Symbol::new(var.symbol.clone())) {
            Some(ty) => ty,
            None => {
                let diagnostic = Diagnostic {
                    span: var.span.clone(),
                    kind: DiagnosticKind::NotDeclaredVariable,
                    message: format!("'{}' is not declared", var.symbol.clone()),
                };
                self.diagnostics.push(diagnostic);
                TypeKind::Unknown
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use typua_span::{Position, Span};
    #[test]
    fn eval_literal() {
        let env = TypeEnv::new();
        let mut checker = Checker::new(env);
        let expr = Expression::Number {
            span: Span {
                start: Position::new(0, 0),
                end: Position::new(0, 0),
            },
            val: "12".to_string(),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::Number);
        assert_eq!(checker.diagnostics.is_empty(), true);
    }
    #[test]
    fn eval_binop_add() {
        // Normal Test: number + number
        let env = TypeEnv::new();
        let mut checker = Checker::new(env);
        let expr = Expression::BinaryOperator {
            binop: BinOp::Add(Span::new(Position::new(0, 0), Position::new(0, 0))),
            lhs: Box::new(Expression::Number {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 0),
                },
                val: "12".to_string(),
            }),
            rhs: Box::new(Expression::Number {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 10),
                },
                val: "12".to_string(),
            }),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::Number);
        assert_eq!(checker.diagnostics.is_empty(), true);

        // TypeMismatch: number + bool
        // (e.g) false + 12
        let env = TypeEnv::new();
        let mut checker = Checker::new(env);
        let expr = Expression::BinaryOperator {
            lhs: Box::new(Expression::Boolean {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 6),
                },
                val: "false".to_string(),
            }),
            binop: BinOp::Add(Span::new(Position::new(0, 7), Position::new(0, 8))),
            rhs: Box::new(Expression::Number {
                span: Span {
                    start: Position::new(0, 9),
                    end: Position::new(0, 11),
                },
                val: "12".to_string(),
            }),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::Unknown);
        assert_eq!(checker.diagnostics.is_empty(), false);
        assert_eq!(
            checker.diagnostics[0],
            Diagnostic {
                message: "cannot add `boolean` and `number`".to_string(),
                kind: DiagnosticKind::TypeMismatch,
                span: Span::new(Position::new(0, 7), Position::new(0, 8)),
            }
        );
        // Normal Test: binop vars
        let mut env = TypeEnv::new();
        let _ = env.insert(&Symbol::new("x".to_string()), &TypeKind::Number);
        let _ = env.insert(&Symbol::new("y".to_string()), &TypeKind::Number);
        let mut checker = Checker::new(env);
        let expr = Expression::BinaryOperator {
            lhs: Box::new(Expression::Var {
                var: Var {
                    span: Span {
                        start: Position::new(0, 0),
                        end: Position::new(0, 0),
                    },
                    symbol: "x".to_string(),
                },
            }),
            rhs: Box::new(Expression::Var {
                var: Var {
                    span: Span {
                        start: Position::new(0, 0),
                        end: Position::new(0, 10),
                    },
                    symbol: "y".to_string(),
                },
            }),
            binop: BinOp::Add(Span::new(Position::new(0, 0), Position::new(0, 0))),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::Number);
        assert_eq!(checker.diagnostics.is_empty(), true);
    }
    #[test]
    fn eval_binop_or() {
        // NormalTest: true or 12 => true
        let env = TypeEnv::new();
        let mut checker = Checker::new(env);
        let expr = Expression::BinaryOperator {
            binop: BinOp::Or(Span::new(Position::new(0, 0), Position::new(0, 0))),
            lhs: Box::new(Expression::Boolean {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 0),
                },
                val: "true".to_string(),
            }),
            rhs: Box::new(Expression::Number {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 10),
                },
                val: "12".to_string(),
            }),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::Boolean(BoolLiteral::True));
        assert_eq!(checker.diagnostics.is_empty(), true);

        // NormalTest: "hello" or 12 => string
        let env = TypeEnv::new();
        let mut checker = Checker::new(env);
        let expr = Expression::BinaryOperator {
            binop: BinOp::Or(Span::new(Position::new(0, 0), Position::new(0, 0))),
            lhs: Box::new(Expression::String {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 0),
                },
                val: "hello".to_string(),
            }),
            rhs: Box::new(Expression::Number {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 10),
                },
                val: "12".to_string(),
            }),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::String);
        assert_eq!(checker.diagnostics.is_empty(), true);

        // NormalTest: false or 12 => number
        let env = TypeEnv::new();
        let mut checker = Checker::new(env);
        let expr = Expression::BinaryOperator {
            binop: BinOp::Or(Span::new(Position::new(0, 7), Position::new(0, 8))),
            lhs: Box::new(Expression::Boolean {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 6),
                },
                val: "false".to_string(),
            }),
            rhs: Box::new(Expression::Number {
                span: Span {
                    start: Position::new(0, 9),
                    end: Position::new(0, 11),
                },
                val: "12".to_string(),
            }),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::Number);
        assert_eq!(checker.diagnostics.is_empty(), true);

        // NormalTest: nil or 12 => number
        let env = TypeEnv::new();
        let mut checker = Checker::new(env);
        let expr = Expression::BinaryOperator {
            binop: BinOp::Or(Span::new(Position::new(0, 7), Position::new(0, 8))),
            lhs: Box::new(Expression::Nil {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 6),
                },
            }),
            rhs: Box::new(Expression::Number {
                span: Span {
                    start: Position::new(0, 9),
                    end: Position::new(0, 11),
                },
                val: "12".to_string(),
            }),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::Number);
        assert_eq!(checker.diagnostics.is_empty(), true);

        // NormalTest: x and 12 => number|true
        let mut env = TypeEnv::new();
        let _ = env.insert(
            &Symbol::new("x".to_string()),
            &TypeKind::Boolean(BoolLiteral::Any),
        );
        let _ = env.insert(&Symbol::new("y".to_string()), &TypeKind::Number);
        let mut checker = Checker::new(env);
        let expr = Expression::BinaryOperator {
            binop: BinOp::Or(Span::new(Position::new(0, 0), Position::new(0, 0))),
            lhs: Box::new(Expression::Var {
                var: Var {
                    span: Span {
                        start: Position::new(0, 0),
                        end: Position::new(0, 0),
                    },
                    symbol: "x".to_string(),
                },
            }),
            rhs: Box::new(Expression::Var {
                var: Var {
                    span: Span {
                        start: Position::new(0, 0),
                        end: Position::new(0, 10),
                    },
                    symbol: "y".to_string(),
                },
            }),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(
            ty,
            TypeKind::Union(vec![TypeKind::Number, TypeKind::Boolean(BoolLiteral::True)])
        );
        assert_eq!(checker.diagnostics.is_empty(), true);
    }
    #[test]
    fn eval_binop_and() {
        // NormalTest: true and 12 => 12
        let env = TypeEnv::new();
        let mut checker = Checker::new(env);
        let expr = Expression::BinaryOperator {
            binop: BinOp::And(Span::new(Position::new(0, 0), Position::new(0, 0))),
            lhs: Box::new(Expression::Boolean {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 0),
                },
                val: "true".to_string(),
            }),
            rhs: Box::new(Expression::Number {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 10),
                },
                val: "12".to_string(),
            }),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::Number);
        assert_eq!(checker.diagnostics.is_empty(), true);

        // NormalTest: "hello" and 12 => 12
        let env = TypeEnv::new();
        let mut checker = Checker::new(env);
        let expr = Expression::BinaryOperator {
            binop: BinOp::And(Span::new(Position::new(0, 0), Position::new(0, 0))),
            lhs: Box::new(Expression::String {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 0),
                },
                val: "hello".to_string(),
            }),
            rhs: Box::new(Expression::Number {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 10),
                },
                val: "12".to_string(),
            }),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::Number);
        assert_eq!(checker.diagnostics.is_empty(), true);

        // NormalTest: false and 12 => false
        let env = TypeEnv::new();
        let mut checker = Checker::new(env);
        let expr = Expression::BinaryOperator {
            binop: BinOp::And(Span::new(Position::new(0, 7), Position::new(0, 8))),
            lhs: Box::new(Expression::Boolean {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 6),
                },
                val: "false".to_string(),
            }),
            rhs: Box::new(Expression::Number {
                span: Span {
                    start: Position::new(0, 9),
                    end: Position::new(0, 11),
                },
                val: "12".to_string(),
            }),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::Boolean(BoolLiteral::False));
        assert_eq!(checker.diagnostics.is_empty(), true);

        // NormalTest: nil and 12 => nil
        let env = TypeEnv::new();
        let mut checker = Checker::new(env);
        let expr = Expression::BinaryOperator {
            binop: BinOp::And(Span::new(Position::new(0, 7), Position::new(0, 8))),
            lhs: Box::new(Expression::Nil {
                span: Span {
                    start: Position::new(0, 0),
                    end: Position::new(0, 6),
                },
            }),
            rhs: Box::new(Expression::Number {
                span: Span {
                    start: Position::new(0, 9),
                    end: Position::new(0, 11),
                },
                val: "12".to_string(),
            }),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::Nil);
        assert_eq!(checker.diagnostics.is_empty(), true);

        // NormalTest: x = false, y = 12, x and y => false
        let mut env = TypeEnv::new();
        let _ = env.insert(
            &Symbol::new("x".to_string()),
            &TypeKind::Boolean(BoolLiteral::False),
        );
        let _ = env.insert(&Symbol::new("y".to_string()), &TypeKind::Number);
        let mut checker = Checker::new(env);
        let expr = Expression::BinaryOperator {
            binop: BinOp::And(Span::new(Position::new(0, 0), Position::new(0, 0))),
            lhs: Box::new(Expression::Var {
                var: Var {
                    span: Span {
                        start: Position::new(0, 0),
                        end: Position::new(0, 0),
                    },
                    symbol: "x".to_string(),
                },
            }),
            rhs: Box::new(Expression::Var {
                var: Var {
                    span: Span {
                        start: Position::new(0, 0),
                        end: Position::new(0, 10),
                    },
                    symbol: "y".to_string(),
                },
            }),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::Boolean(BoolLiteral::False));
        assert_eq!(checker.diagnostics.is_empty(), true);

        // NormalTest: x = true, y = 12, x and y => number
        let mut env = TypeEnv::new();
        let _ = env.insert(
            &Symbol::new("x".to_string()),
            &TypeKind::Boolean(BoolLiteral::True),
        );
        let _ = env.insert(&Symbol::new("y".to_string()), &TypeKind::Number);
        let mut checker = Checker::new(env);
        let expr = Expression::BinaryOperator {
            binop: BinOp::And(Span::new(Position::new(0, 0), Position::new(0, 0))),
            lhs: Box::new(Expression::Var {
                var: Var {
                    span: Span {
                        start: Position::new(0, 0),
                        end: Position::new(0, 0),
                    },
                    symbol: "x".to_string(),
                },
            }),
            rhs: Box::new(Expression::Var {
                var: Var {
                    span: Span {
                        start: Position::new(0, 0),
                        end: Position::new(0, 10),
                    },
                    symbol: "y".to_string(),
                },
            }),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::Number,);
        assert_eq!(checker.diagnostics.is_empty(), true);

        // NormalTest: x is nil but only annotated, x and 12 => number|false
        let mut env = TypeEnv::new();
        let _ = env.insert(
            &Symbol::new("x".to_string()),
            &TypeKind::Boolean(BoolLiteral::Any),
        );
        let _ = env.insert(&Symbol::new("y".to_string()), &TypeKind::Number);
        let mut checker = Checker::new(env);
        let expr = Expression::BinaryOperator {
            binop: BinOp::And(Span::new(Position::new(0, 0), Position::new(0, 0))),
            lhs: Box::new(Expression::Var {
                var: Var {
                    span: Span {
                        start: Position::new(0, 0),
                        end: Position::new(0, 0),
                    },
                    symbol: "x".to_string(),
                },
            }),
            rhs: Box::new(Expression::Var {
                var: Var {
                    span: Span {
                        start: Position::new(0, 0),
                        end: Position::new(0, 10),
                    },
                    symbol: "y".to_string(),
                },
            }),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(
            ty,
            TypeKind::Union(vec![
                TypeKind::Number,
                TypeKind::Boolean(BoolLiteral::False)
            ])
        );
        assert_eq!(checker.diagnostics.is_empty(), true);
    }
    #[test]
    fn eval_expr_var() {
        let mut env = TypeEnv::new();
        let _ = env.insert(&Symbol::new("x".to_string()), &TypeKind::Number);
        let mut checker = Checker::new(env);
        // normal test: number
        let expr = Expression::Var {
            var: Var {
                span: Span::new(Position::new(0, 0), Position::new(0, 10)),
                symbol: "x".to_string(),
            },
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::Number);
        assert_eq!(checker.diagnostics.is_empty(), true);

        // NotDeclaredVariable: number
        // (e.g)
        // local x = 12
        // y
        let mut env = TypeEnv::new();
        let _ = env.insert(&Symbol::new("x".to_string()), &TypeKind::Number);
        let mut checker = Checker::new(env);
        let expr = Expression::Var {
            var: Var {
                span: Span::new(Position::new(1, 0), Position::new(1, 1)),
                symbol: "y".to_string(),
            },
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::Unknown);
        assert_eq!(checker.diagnostics.is_empty(), false);
        assert_eq!(
            checker.diagnostics[0],
            Diagnostic {
                message: "'y' is not declared".to_string(),
                kind: DiagnosticKind::NotDeclaredVariable,
                span: Span::new(Position::new(1, 0), Position::new(1, 1))
            }
        );
        // TypeMismatch: binop vars
        // (e.g)
        // local x = 12
        // local y = false
        // x + y
        let mut env = TypeEnv::new();
        let _ = env.insert(&Symbol::new("x".to_string()), &TypeKind::Number);
        let _ = env.insert(
            &Symbol::new("y".to_string()),
            &TypeKind::Boolean(BoolLiteral::False),
        );
        let mut checker = Checker::new(env);
        let expr = Expression::BinaryOperator {
            lhs: Box::new(Expression::Var {
                var: Var {
                    span: Span {
                        start: Position::new(2, 0),
                        end: Position::new(2, 1),
                    },
                    symbol: "x".to_string(),
                },
            }),
            binop: BinOp::Add(Span::new(Position::new(2, 2), Position::new(2, 3))),
            rhs: Box::new(Expression::Var {
                var: Var {
                    span: Span {
                        start: Position::new(2, 4),
                        end: Position::new(2, 5),
                    },
                    symbol: "y".to_string(),
                },
            }),
        };
        let ty = checker.eval_expr(&expr);
        assert_eq!(ty, TypeKind::Unknown);
        assert_eq!(checker.diagnostics.len(), 1);
        assert_eq!(
            checker.diagnostics[0],
            Diagnostic {
                message: "cannot add `number` and `boolean`".to_string(),
                kind: DiagnosticKind::TypeMismatch,
                span: Span::new(Position::new(2, 2), Position::new(2, 3))
            }
        );
    }
}
