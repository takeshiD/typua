use std::collections::HashMap;

use full_moon::ast;
use full_moon::ast::punctuated::Punctuated;
use full_moon::node::Node;

use super::types::{AnnotatedType, AnnotationIndex, AnnotationUsage};
use crate::diagnostics::TextRange;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub block: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    LocalAssign(LocalAssign),
    Assign(Assign),
    Function(Function),
    LocalFunction(LocalFunction),
    FunctionCall(FunctionCallStmt),
    If(IfStmt),
    While(WhileStmt),
    Repeat(RepeatStmt),
    Do(DoStmt),
    NumericFor(NumericForStmt),
    GenericFor(GenericForStmt),
    Return(ReturnStmt),
    Break(TextRange),
    Unknown(TextRange),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalAssign {
    pub names: Vec<String>,
    pub values: Vec<Expr>,
    pub ann: HashMap<String, AnnotatedType>,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assign {
    pub targets: Vec<Expr>,
    pub values: Vec<Expr>,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: FunctionName,
    pub params: Vec<FunctionParam>,
    pub param_types: HashMap<String, AnnotatedType>,
    pub returns: Vec<AnnotatedType>,
    pub body: Block,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionName {
    pub path: Vec<String>,
    pub method: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionParam {
    pub name: Option<String>,
    pub is_vararg: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalFunction {
    pub name: String,
    pub params: Vec<FunctionParam>,
    pub param_types: HashMap<String, AnnotatedType>,
    pub returns: Vec<AnnotatedType>,
    pub body: Block,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCallStmt {
    pub expression: Expr,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub branches: Vec<IfBranch>,
    pub else_branch: Option<Block>,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfBranch {
    pub condition: Expr,
    pub block: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt {
    pub condition: Expr,
    pub block: Block,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatStmt {
    pub block: Block,
    pub condition: Expr,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoStmt {
    pub block: Block,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumericForStmt {
    pub index: String,
    pub start: Expr,
    pub end: Expr,
    pub step: Option<Expr>,
    pub body: Block,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericForStmt {
    pub names: Vec<String>,
    pub generators: Vec<Expr>,
    pub body: Block,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    pub values: Vec<Expr>,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Nil,
    Boolean(bool),
    Number(String),
    String(String),
    VarArgs,
    TableConstructor(Vec<TableField>),
    Name(String),
    Field(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    BinaryOp(Box<Expr>, String, Box<Expr>),
    UnaryOp(String, Box<Expr>),
    Call(Box<CallExpr>),
    MethodCall(Box<MethodCallExpr>),
    Function(FunctionExpr),
    Parentheses(Box<Expr>),
    Unknown(TextRange),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallExpr {
    pub function: Box<Expr>,
    pub args: CallArgs,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodCallExpr {
    pub receiver: Box<Expr>,
    pub method: String,
    pub args: CallArgs,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallArgs {
    Parentheses(Vec<Expr>),
    String(String),
    Table(Vec<TableField>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionExpr {
    pub params: Vec<FunctionParam>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TableField {
    Array(Expr),
    NameValue { name: String, value: Expr },
    ExpressionKey { key: Expr, value: Expr },
}

pub fn build_typed_ast(_source: &str, ast: &ast::Ast, annotations: &AnnotationIndex) -> Program {
    let block = to_block(ast.nodes(), annotations);
    Program { block }
}

fn to_block(block: &ast::Block, annotations: &AnnotationIndex) -> Block {
    let mut stmts = Vec::new();
    for stmt in block.stmts() {
        stmts.push(to_stmt(stmt, annotations));
    }

    if let Some(last) = block.last_stmt() {
        match last {
            ast::LastStmt::Return(ret) => {
                let values = ret
                    .returns()
                    .iter()
                    .map(|expr| to_expr(expr, annotations))
                    .collect();
                stmts.push(Stmt::Return(ReturnStmt {
                    values,
                    range: token_range(last),
                }));
            }
            ast::LastStmt::Break(_) => {
                stmts.push(Stmt::Break(token_range(last)));
            }
            _ => {}
        }
    }

    Block { stmts }
}

fn to_stmt(stmt: &ast::Stmt, annotations: &AnnotationIndex) -> Stmt {
    match stmt {
        ast::Stmt::LocalAssignment(assign) => {
            let names: Vec<String> = assign
                .names()
                .iter()
                .map(|token| token.token().to_string())
                .collect();

            let mut ann = HashMap::new();
            if let Some(pos) = assign.start_position()
                && let Some(vec) = annotations.by_line.get(&pos.line())
            {
                for a in vec.iter() {
                    if let (AnnotationUsage::Param | AnnotationUsage::Type, Some(name)) =
                        (&a.usage, &a.name)
                    {
                        ann.insert(name.clone(), a.ty.clone());
                    }
                }
            }

            let values = assign
                .expressions()
                .pairs()
                .map(|pair| to_expr(pair.value(), annotations))
                .collect();

            Stmt::LocalAssign(LocalAssign {
                names,
                values,
                ann,
                range: token_range(assign),
            })
        }
        ast::Stmt::Assignment(assign) => {
            let targets = assign
                .variables()
                .pairs()
                .map(|pair| to_expr_var(pair.value(), annotations))
                .collect();
            let values = assign
                .expressions()
                .pairs()
                .map(|pair| to_expr(pair.value(), annotations))
                .collect();
            Stmt::Assign(Assign {
                targets,
                values,
                range: token_range(assign),
            })
        }
        ast::Stmt::FunctionDeclaration(function) => {
            let name = to_function_name(function.name());
            let params = to_function_params(function.body().parameters());
            let (param_types, returns) = function_annotations(
                function.function_token().token().start_position().line(),
                annotations,
            );
            let body = to_block(function.body().block(), annotations);
            Stmt::Function(Function {
                name,
                params,
                param_types,
                returns,
                body,
                range: token_range(function),
            })
        }
        ast::Stmt::LocalFunction(function) => {
            let name = function.name().token().to_string();
            let params = to_function_params(function.body().parameters());
            let (param_types, returns) = function_annotations(
                function.function_token().token().start_position().line(),
                annotations,
            );
            let body = to_block(function.body().block(), annotations);
            Stmt::LocalFunction(LocalFunction {
                name,
                params,
                param_types,
                returns,
                body,
                range: token_range(function),
            })
        }
        ast::Stmt::FunctionCall(call) => {
            let expr = to_function_call(call, annotations);
            Stmt::FunctionCall(FunctionCallStmt {
                expression: expr,
                range: token_range(call),
            })
        }
        ast::Stmt::If(if_stmt) => {
            let mut branches = Vec::new();
            branches.push(IfBranch {
                condition: to_expr(if_stmt.condition(), annotations),
                block: to_block(if_stmt.block(), annotations),
            });
            if let Some(else_ifs) = if_stmt.else_if() {
                for branch in else_ifs {
                    branches.push(IfBranch {
                        condition: to_expr(branch.condition(), annotations),
                        block: to_block(branch.block(), annotations),
                    });
                }
            }
            let else_branch = if_stmt
                .else_block()
                .map(|block| to_block(block, annotations));
            Stmt::If(IfStmt {
                branches,
                else_branch,
                range: token_range(if_stmt),
            })
        }
        ast::Stmt::While(while_stmt) => Stmt::While(WhileStmt {
            condition: to_expr(while_stmt.condition(), annotations),
            block: to_block(while_stmt.block(), annotations),
            range: token_range(while_stmt),
        }),
        ast::Stmt::Repeat(repeat_stmt) => Stmt::Repeat(RepeatStmt {
            block: to_block(repeat_stmt.block(), annotations),
            condition: to_expr(repeat_stmt.until(), annotations),
            range: token_range(repeat_stmt),
        }),
        ast::Stmt::Do(do_stmt) => Stmt::Do(DoStmt {
            block: to_block(do_stmt.block(), annotations),
            range: token_range(do_stmt),
        }),
        ast::Stmt::NumericFor(numeric_for) => {
            let index = numeric_for.index_variable().token().to_string();
            let start = to_expr(numeric_for.start(), annotations);
            let end = to_expr(numeric_for.end(), annotations);
            let step = numeric_for.step().map(|expr| to_expr(expr, annotations));
            let body = to_block(numeric_for.block(), annotations);
            Stmt::NumericFor(NumericForStmt {
                index,
                start,
                end,
                step,
                body,
                range: token_range(numeric_for),
            })
        }
        ast::Stmt::GenericFor(generic_for) => {
            let names = generic_for
                .names()
                .iter()
                .map(|token| token.token().to_string())
                .collect();
            let generators = generic_for
                .expressions()
                .pairs()
                .map(|pair| to_expr(pair.value(), annotations))
                .collect();
            let body = to_block(generic_for.block(), annotations);
            Stmt::GenericFor(GenericForStmt {
                names,
                generators,
                body,
                range: token_range(generic_for),
            })
        }
        _ => Stmt::Unknown(token_range(stmt)),
    }
}

fn to_expr(expr: &ast::Expression, annotations: &AnnotationIndex) -> Expr {
    match expr {
        ast::Expression::Number(token) => Expr::Number(token.to_string()),
        ast::Expression::String(token) => Expr::String(token.to_string()),
        ast::Expression::Symbol(token) => match token.to_string().as_str() {
            "nil" => Expr::Nil,
            "true" => Expr::Boolean(true),
            "false" => Expr::Boolean(false),
            "..." => Expr::VarArgs,
            other => Expr::Name(other.to_string()),
        },
        ast::Expression::BinaryOperator { lhs, binop, rhs } => Expr::BinaryOp(
            Box::new(to_expr(lhs, annotations)),
            binop.token().to_string(),
            Box::new(to_expr(rhs, annotations)),
        ),
        ast::Expression::UnaryOperator { unop, expression } => Expr::UnaryOp(
            unop.token().to_string(),
            Box::new(to_expr(expression, annotations)),
        ),
        ast::Expression::Parentheses { expression, .. } => {
            Expr::Parentheses(Box::new(to_expr(expression, annotations)))
        }
        ast::Expression::TableConstructor(table) => {
            Expr::TableConstructor(to_table_fields(table, annotations))
        }
        ast::Expression::FunctionCall(call) => to_function_call(call, annotations),
        ast::Expression::Var(var) => to_expr_var(var, annotations),
        ast::Expression::Function(function) => Expr::Function(FunctionExpr {
            params: to_function_params(function.body().parameters()),
            body: to_block(function.body().block(), annotations),
        }),
        _ => Expr::Unknown(token_range(expr)),
    }
}

fn to_expr_var(var: &ast::Var, annotations: &AnnotationIndex) -> Expr {
    match var {
        ast::Var::Name(token) => Expr::Name(token.token().to_string()),
        ast::Var::Expression(expr) => to_prefixed_expression(expr, annotations),
        _ => Expr::Unknown(token_range(var)),
    }
}

fn to_function_call(call: &ast::FunctionCall, annotations: &AnnotationIndex) -> Expr {
    to_prefixed_chain(call.prefix(), call.suffixes(), annotations)
}

fn to_prefixed_expression(expr: &ast::VarExpression, annotations: &AnnotationIndex) -> Expr {
    to_prefixed_chain(expr.prefix(), expr.suffixes(), annotations)
}

fn to_prefixed_chain<'a, I>(
    prefix: &ast::Prefix,
    suffixes: I,
    annotations: &AnnotationIndex,
) -> Expr
where
    I: Iterator<Item = &'a ast::Suffix>,
{
    let mut current = match prefix {
        ast::Prefix::Name(token) => Expr::Name(token.token().to_string()),
        ast::Prefix::Expression(expr) => to_expr(expr, annotations),
        _ => Expr::Unknown(token_range(prefix)),
    };

    for suffix in suffixes {
        current = apply_suffix(current, suffix, annotations);
    }

    current
}

fn apply_suffix(expr: Expr, suffix: &ast::Suffix, annotations: &AnnotationIndex) -> Expr {
    match suffix {
        ast::Suffix::Index(index) => match index {
            ast::Index::Dot { name, .. } => Expr::Field(Box::new(expr), name.token().to_string()),
            ast::Index::Brackets {
                expression: key, ..
            } => {
                let key_expr = to_expr(key, annotations);
                Expr::Index(Box::new(expr), Box::new(key_expr))
            }
            _ => Expr::Unknown(token_range(index)),
        },
        ast::Suffix::Call(call) => match call {
            ast::Call::AnonymousCall(args) => Expr::Call(Box::new(CallExpr {
                function: Box::new(expr),
                args: to_call_args(args, annotations),
            })),
            ast::Call::MethodCall(method) => Expr::MethodCall(Box::new(MethodCallExpr {
                receiver: Box::new(expr),
                method: method.name().token().to_string(),
                args: to_call_args(method.args(), annotations),
            })),
            _ => Expr::Unknown(token_range(call)),
        },
        _ => Expr::Unknown(token_range(suffix)),
    }
}

fn to_call_args(args: &ast::FunctionArgs, annotations: &AnnotationIndex) -> CallArgs {
    match args {
        ast::FunctionArgs::Parentheses { arguments, .. } => CallArgs::Parentheses(
            arguments
                .pairs()
                .map(|pair| to_expr(pair.value(), annotations))
                .collect(),
        ),
        ast::FunctionArgs::String(token) => CallArgs::String(token.to_string()),
        ast::FunctionArgs::TableConstructor(table) => {
            CallArgs::Table(to_table_fields(table, annotations))
        }
        _ => CallArgs::Parentheses(Vec::new()),
    }
}

fn to_table_fields(
    table: &ast::TableConstructor,
    annotations: &AnnotationIndex,
) -> Vec<TableField> {
    table
        .fields()
        .pairs()
        .map(|pair| match pair.value() {
            ast::Field::NoKey(expr) => TableField::Array(to_expr(expr, annotations)),
            ast::Field::NameKey { key, value, .. } => TableField::NameValue {
                name: key.token().to_string(),
                value: to_expr(value, annotations),
            },
            ast::Field::ExpressionKey { key, value, .. } => TableField::ExpressionKey {
                key: to_expr(key, annotations),
                value: to_expr(value, annotations),
            },
            _ => TableField::Array(Expr::Unknown(token_range(pair.value()))),
        })
        .collect()
}

fn to_function_name(name: &ast::FunctionName) -> FunctionName {
    let path = name
        .names()
        .iter()
        .map(|token| token.token().to_string())
        .collect();
    let method = name.method_name().map(|token| token.token().to_string());
    FunctionName { path, method }
}

fn to_function_params(parameters: &Punctuated<ast::Parameter>) -> Vec<FunctionParam> {
    parameters
        .iter()
        .map(|param| match param {
            ast::Parameter::Name(token) => FunctionParam {
                name: Some(token.token().to_string()),
                is_vararg: false,
            },
            ast::Parameter::Ellipsis(_) => FunctionParam {
                name: None,
                is_vararg: true,
            },
            _ => FunctionParam {
                name: None,
                is_vararg: false,
            },
        })
        .collect()
}

fn function_annotations(
    line: usize,
    annotations: &AnnotationIndex,
) -> (HashMap<String, AnnotatedType>, Vec<AnnotatedType>) {
    let mut params = HashMap::new();
    let mut returns = Vec::new();

    if let Some(list) = annotations.by_line.get(&line) {
        for ann in list.iter() {
            match ann.usage {
                AnnotationUsage::Param => {
                    if let Some(name) = &ann.name {
                        params.insert(name.clone(), ann.ty.clone());
                    }
                }
                AnnotationUsage::Return => returns.push(ann.ty.clone()),
                AnnotationUsage::Type => {}
            }
        }
    }

    (params, returns)
}

fn token_range<T: Node>(node: &T) -> TextRange {
    let (start, end) = (node.start_position(), node.end_position());
    let start = start
        .map(Into::into)
        .unwrap_or(crate::diagnostics::TextPosition {
            line: 0,
            character: 0,
        });
    let end = end.map(Into::into).unwrap_or(start);
    TextRange { start, end }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typechecker::types::AnnotationIndex;
    use pretty_assertions::assert_eq;
    use unindent::unindent;

    fn parse(source: &str) -> (ast::Ast, AnnotationIndex) {
        let ast = full_moon::parse(source).expect("parse");
        let (ann, _) = AnnotationIndex::from_source(source);
        (ast, ann)
    }

    #[test]
    fn capture_function_declaration_with_annotations() {
        let source = unindent(
            r#"
            ---@param a number
            ---@param b string
            ---@return boolean
            function mod.example:run(a, b)
                local sum = a
                return true
            end
            "#,
        );

        let (ast, annotations) = parse(&source);
        let program = build_typed_ast(&source, &ast, &annotations);
        assert_eq!(program.block.stmts.len(), 1);

        let Stmt::Function(func) = &program.block.stmts[0] else {
            panic!("expected function stmt");
        };

        assert_eq!(
            func.name.path,
            vec!["mod".to_string(), "example".to_string()]
        );
        assert_eq!(func.name.method.as_deref(), Some("run"));
        assert_eq!(func.params.len(), 2);
        assert_eq!(
            func.params[0],
            FunctionParam {
                name: Some("a".into()),
                is_vararg: false
            }
        );
        assert_eq!(func.returns.len(), 1);
        assert!(func.param_types.contains_key("a"));
        assert!(matches!(func.body.stmts.last(), Some(Stmt::Return(_))));
    }

    #[test]
    fn convert_control_flow_and_calls() {
        let source = unindent(
            r#"
            local total = 0
            for i = 1, 3, 1 do
                print(i)
            end
            for k, v in pairs(t) do
                print(k, v)
            end
            if total > 0 then
                total = total - 1
            elseif total == 0 then
                total = 10
            else
                total = -total
            end
            while total < 5 do
                total = total + 1
            end
            repeat
                total = total - 2
            until total == 0
            do
                local inner = 1
            end
            foo:bar(1, 2)
            return total
            "#,
        );

        let (ast, annotations) = parse(&source);
        let program = build_typed_ast(&source, &ast, &annotations);
        assert!(
            program
                .block
                .stmts
                .iter()
                .any(|stmt| matches!(stmt, Stmt::NumericFor(_)))
        );
        assert!(
            program
                .block
                .stmts
                .iter()
                .any(|stmt| matches!(stmt, Stmt::GenericFor(_)))
        );
        assert!(
            program
                .block
                .stmts
                .iter()
                .any(|stmt| matches!(stmt, Stmt::If(_)))
        );
        assert!(
            program
                .block
                .stmts
                .iter()
                .any(|stmt| matches!(stmt, Stmt::While(_)))
        );
        assert!(
            program
                .block
                .stmts
                .iter()
                .any(|stmt| matches!(stmt, Stmt::Repeat(_)))
        );
        assert!(
            program
                .block
                .stmts
                .iter()
                .any(|stmt| matches!(stmt, Stmt::Do(_)))
        );
        assert!(
            program
                .block
                .stmts
                .iter()
                .any(|stmt| matches!(stmt, Stmt::FunctionCall(_)))
        );
        assert!(matches!(program.block.stmts.last(), Some(Stmt::Return(_))));
    }

    #[test]
    fn convert_anonymous_function_and_table_constructor() {
        let source = unindent(
            r#"
            local mapper = function(x)
                return { value = x, [x] = true }
            end
            "#,
        );

        let (ast, annotations) = parse(&source);
        let program = build_typed_ast(&source, &ast, &annotations);
        let Stmt::LocalAssign(local) = &program.block.stmts[0] else {
            panic!("expected local assignment");
        };
        assert_eq!(local.names, vec!["mapper".to_string()]);
        let Expr::Function(function_expr) = &local.values[0] else {
            panic!("expected anonymous function");
        };
        assert_eq!(function_expr.params.len(), 1);
        let Stmt::Return(return_stmt) = &function_expr.body.stmts[0] else {
            panic!("expected return inside function body");
        };
        let Expr::TableConstructor(fields) = &return_stmt.values[0] else {
            panic!("expected table constructor");
        };
        assert_eq!(fields.len(), 2);
    }
}
