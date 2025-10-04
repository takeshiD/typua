use std::collections::HashMap;

use full_moon::ast;
use full_moon::ast::punctuated::Punctuated;
use full_moon::node::Node;
use full_moon::tokenizer::{Token, TokenReference};

use super::types::{AnnotatedType, Annotation, AnnotationIndex, AnnotationUsage, ReturnAnnotation};
use crate::diagnostics::{TextPosition, TextRange};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub block: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
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
    Goto(GotoStmt),
    Label(LabelStmt),
    Return(ReturnStmt),
    Break(TextRange),
    Unknown(TextRange),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: String,
    pub range: TextRange,
}

impl Identifier {
    fn new(name: impl Into<String>, range: TextRange) -> Self {
        Self {
            name: name.into(),
            range,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalAssign {
    pub names: Vec<Identifier>,
    pub values: Vec<Expr>,
    pub annotations: Vec<Annotation>,
    pub class_hints: Vec<String>,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assign {
    pub targets: Vec<Expr>,
    pub values: Vec<Expr>,
    pub annotations: Vec<Annotation>,
    pub class_hints: Vec<String>,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: FunctionName,
    pub params: Vec<FunctionParam>,
    pub param_types: HashMap<String, AnnotatedType>,
    pub returns: Vec<ReturnAnnotation>,
    pub annotations: Vec<Annotation>,
    pub body: Block,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionName {
    pub path: Vec<Identifier>,
    pub method: Option<Identifier>,
}

impl FunctionName {
    pub fn last_component(&self) -> Option<&Identifier> {
        if let Some(method) = &self.method {
            Some(method)
        } else {
            self.path.last()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionParam {
    pub name: Option<Identifier>,
    pub is_vararg: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalFunction {
    pub name: Identifier,
    pub params: Vec<FunctionParam>,
    pub param_types: HashMap<String, AnnotatedType>,
    pub returns: Vec<ReturnAnnotation>,
    pub annotations: Vec<Annotation>,
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
    pub index: Identifier,
    pub start: Expr,
    pub end: Expr,
    pub step: Option<Expr>,
    pub body: Block,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericForStmt {
    pub names: Vec<Identifier>,
    pub generators: Vec<Expr>,
    pub body: Block,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GotoStmt {
    pub name: Identifier,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LabelStmt {
    pub name: Identifier,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    pub values: Vec<Expr>,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub range: TextRange,
}

impl Expr {
    fn new(kind: ExprKind, range: TextRange) -> Self {
        Self { kind, range }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Nil,
    Boolean(bool),
    Number(String),
    String(String),
    VarArgs,
    TableConstructor(Vec<TableField>),
    Name(Identifier),
    Field {
        target: Box<Expr>,
        name: Identifier,
    },
    Index {
        target: Box<Expr>,
        key: Box<Expr>,
    },
    BinaryOp {
        left: Box<Expr>,
        operator: Operator,
        right: Box<Expr>,
    },
    UnaryOp {
        operator: Operator,
        expression: Box<Expr>,
    },
    Call(CallExpr),
    MethodCall(MethodCallExpr),
    Function(FunctionExpr),
    Parentheses(Box<Expr>),
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Operator {
    pub symbol: String,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallExpr {
    pub function: Box<Expr>,
    pub args: CallArgs,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodCallExpr {
    pub receiver: Box<Expr>,
    pub method: Identifier,
    pub args: CallArgs,
    pub range: TextRange,
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
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TableField {
    Array {
        value: Expr,
        range: TextRange,
    },
    NameValue {
        name: Identifier,
        value: Expr,
        range: TextRange,
    },
    ExpressionKey {
        key: Expr,
        value: Expr,
        range: TextRange,
    },
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
            let line = assign.local_token().token().start_position().line();
            let names = assign
                .names()
                .iter()
                .map(|token| identifier_from_token(token.token()))
                .collect();
            let annotations_for_line = annotations.line_annotations(line);
            let class_hints = annotations.line_class_hints(line);
            let values = assign
                .expressions()
                .pairs()
                .map(|pair| to_expr(pair.value(), annotations))
                .collect();
            Stmt::LocalAssign(LocalAssign {
                names,
                values,
                annotations: annotations_for_line,
                class_hints,
                range: token_range(assign),
            })
        }
        ast::Stmt::Assignment(assign) => {
            let line = assign
                .variables()
                .pairs()
                .next()
                .and_then(|pair| pair.value().start_position())
                .map(|pos| pos.line())
                .unwrap_or(0);
            let annotations_for_line = if line > 0 {
                annotations.line_annotations(line)
            } else {
                Vec::new()
            };
            let class_hints = if line > 0 {
                annotations.line_class_hints(line)
            } else {
                Vec::new()
            };
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
                annotations: annotations_for_line,
                class_hints,
                range: token_range(assign),
            })
        }
        ast::Stmt::FunctionDeclaration(function) => {
            let line = function.function_token().token().start_position().line();
            let annotations_for_line = annotations.line_annotations(line);
            let (param_types, returns, remaining_annotations) =
                function_annotations(annotations_for_line);
            let name = to_function_name(function.name());
            let params = to_function_params(function.body().parameters());
            let body = to_block(function.body().block(), annotations);
            Stmt::Function(Function {
                name,
                params,
                param_types,
                returns,
                annotations: remaining_annotations,
                body,
                range: token_range(function),
            })
        }
        ast::Stmt::LocalFunction(function) => {
            let line = function.function_token().token().start_position().line();
            let annotations_for_line = annotations.line_annotations(line);
            let (param_types, returns, remaining_annotations) =
                function_annotations(annotations_for_line);
            let name = identifier_from_token(function.name().token());
            let params = to_function_params(function.body().parameters());
            let body = to_block(function.body().block(), annotations);
            Stmt::LocalFunction(LocalFunction {
                name,
                params,
                param_types,
                returns,
                annotations: remaining_annotations,
                body,
                range: token_range(function),
            })
        }
        ast::Stmt::FunctionCall(call) => {
            let expr = to_function_call(call, annotations);
            Stmt::FunctionCall(FunctionCallStmt {
                range: token_range(call),
                expression: expr,
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
            let index = identifier_from_token_ref(numeric_for.index_variable());
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
                .map(|token| identifier_from_token(token.token()))
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
        ast::Stmt::Goto(goto) => {
            let name = identifier_from_token(goto.label_name().token());
            Stmt::Goto(GotoStmt {
                name,
                range: token_range(goto),
            })
        }
        ast::Stmt::Label(label) => {
            let name = identifier_from_token(label.name().token());
            Stmt::Label(LabelStmt {
                name,
                range: token_range(label),
            })
        }
        _ => Stmt::Unknown(token_range(stmt)),
    }
}

fn to_expr(expr: &ast::Expression, annotations: &AnnotationIndex) -> Expr {
    let range = token_range(expr);
    match expr {
        ast::Expression::Number(token) => Expr::new(ExprKind::Number(token.to_string()), range),
        ast::Expression::String(token) => Expr::new(ExprKind::String(token.to_string()), range),
        ast::Expression::Symbol(token) => {
            let symbol = token.to_string();
            match symbol.trim() {
                "nil" => Expr::new(ExprKind::Nil, range),
                "true" => Expr::new(ExprKind::Boolean(true), range),
                "false" => Expr::new(ExprKind::Boolean(false), range),
                "..." => Expr::new(ExprKind::VarArgs, range),
                other => Expr::new(
                    ExprKind::Name(Identifier::new(other.to_string(), range)),
                    range,
                ),
            }
        }
        ast::Expression::BinaryOperator { lhs, binop, rhs } => {
            let left = to_expr(lhs, annotations);
            let right = to_expr(rhs, annotations);
            let operator = Operator {
                symbol: binop.token().to_string().trim().to_string(),
                range: token_range(binop),
            };
            Expr::new(
                ExprKind::BinaryOp {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                },
                range,
            )
        }
        ast::Expression::UnaryOperator { unop, expression } => {
            let expr = to_expr(expression, annotations);
            let operator = Operator {
                symbol: unop.token().to_string().trim().to_string(),
                range: token_range(unop),
            };
            Expr::new(
                ExprKind::UnaryOp {
                    operator,
                    expression: Box::new(expr),
                },
                range,
            )
        }
        ast::Expression::Parentheses { expression, .. } => Expr::new(
            ExprKind::Parentheses(Box::new(to_expr(expression, annotations))),
            range,
        ),
        ast::Expression::TableConstructor(table) => Expr::new(
            ExprKind::TableConstructor(to_table_fields(table, annotations)),
            range,
        ),
        ast::Expression::FunctionCall(call) => to_function_call(call, annotations),
        ast::Expression::Var(var) => to_expr_var(var, annotations),
        ast::Expression::Function(function) => Expr::new(
            ExprKind::Function(FunctionExpr {
                params: to_function_params(function.body().parameters()),
                body: to_block(function.body().block(), annotations),
                range: token_range(function),
            }),
            range,
        ),
        _ => Expr::new(ExprKind::Unknown, range),
    }
}

fn to_expr_var(var: &ast::Var, annotations: &AnnotationIndex) -> Expr {
    let range = token_range(var);
    match var {
        ast::Var::Name(token) => Expr::new(ExprKind::Name(identifier_from_token_ref(token)), range),
        ast::Var::Expression(expr) => to_prefixed_expression(expr, annotations),
        _ => Expr::new(ExprKind::Unknown, range),
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
        ast::Prefix::Name(token) => Expr::new(
            ExprKind::Name(identifier_from_token(token.token())),
            token_range(token),
        ),
        ast::Prefix::Expression(expr) => to_expr(expr, annotations),
        _ => Expr::new(ExprKind::Unknown, token_range(prefix)),
    };

    for suffix in suffixes {
        let prev = current;
        let prev_range = prev.range;
        current = match suffix {
            ast::Suffix::Index(index) => match index {
                ast::Index::Dot { name, .. } => {
                    let ident = identifier_from_token(name.token());
                    let range = merge_ranges(prev_range, token_range(index));
                    Expr::new(
                        ExprKind::Field {
                            target: Box::new(prev),
                            name: ident,
                        },
                        range,
                    )
                }
                ast::Index::Brackets {
                    expression: key, ..
                } => {
                    let key_expr = to_expr(key, annotations);
                    let range = merge_ranges(prev_range, token_range(index));
                    Expr::new(
                        ExprKind::Index {
                            target: Box::new(prev),
                            key: Box::new(key_expr),
                        },
                        range,
                    )
                }
                _ => Expr::new(
                    ExprKind::Unknown,
                    merge_ranges(prev_range, token_range(index)),
                ),
            },
            ast::Suffix::Call(call) => match call {
                ast::Call::AnonymousCall(args) => {
                    let args = to_call_args(args, annotations);
                    let range = merge_ranges(prev_range, token_range(call));
                    Expr::new(
                        ExprKind::Call(CallExpr {
                            function: Box::new(prev),
                            args,
                            range,
                        }),
                        range,
                    )
                }
                ast::Call::MethodCall(method) => {
                    let args = to_call_args(method.args(), annotations);
                    let method_ident = identifier_from_token(method.name().token());
                    let range = merge_ranges(prev_range, token_range(method));
                    Expr::new(
                        ExprKind::MethodCall(MethodCallExpr {
                            receiver: Box::new(prev),
                            method: method_ident,
                            args,
                            range,
                        }),
                        range,
                    )
                }
                _ => Expr::new(
                    ExprKind::Unknown,
                    merge_ranges(prev_range, token_range(call)),
                ),
            },
            _ => Expr::new(
                ExprKind::Unknown,
                merge_ranges(prev_range, token_range(suffix)),
            ),
        };
    }

    current
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
            ast::Field::NoKey(expr) => TableField::Array {
                value: to_expr(expr, annotations),
                range: token_range(expr),
            },
            ast::Field::NameKey { key, value, .. } => TableField::NameValue {
                name: identifier_from_token(key.token()),
                value: to_expr(value, annotations),
                range: token_range(pair.value()),
            },
            ast::Field::ExpressionKey { key, value, .. } => TableField::ExpressionKey {
                key: to_expr(key, annotations),
                value: to_expr(value, annotations),
                range: token_range(pair.value()),
            },
            _ => TableField::Array {
                value: Expr::new(ExprKind::Unknown, token_range(pair.value())),
                range: token_range(pair.value()),
            },
        })
        .collect()
}

fn to_function_name(name: &ast::FunctionName) -> FunctionName {
    let path = name
        .names()
        .iter()
        .map(|token| identifier_from_token(token.token()))
        .collect();
    let method = name
        .method_name()
        .map(|token| identifier_from_token(token.token()));
    FunctionName { path, method }
}

fn to_function_params(parameters: &Punctuated<ast::Parameter>) -> Vec<FunctionParam> {
    parameters
        .iter()
        .map(|param| match param {
            ast::Parameter::Name(token) => FunctionParam {
                name: Some(identifier_from_token_ref(token)),
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
    annotations: Vec<Annotation>,
) -> (
    HashMap<String, AnnotatedType>,
    Vec<ReturnAnnotation>,
    Vec<Annotation>,
) {
    let mut params = HashMap::new();
    let mut returns = Vec::new();
    let mut leftover = Vec::new();

    for ann in annotations {
        match ann.usage {
            AnnotationUsage::Param => {
                if let Some(name) = ann.name.clone() {
                    params.insert(name, ann.ty.clone());
                }
            }
            AnnotationUsage::Return => returns.push(ReturnAnnotation {
                name: ann.name.clone(),
                ty: ann.ty.clone(),
            }),
            AnnotationUsage::Type => leftover.push(ann.clone()),
        }
    }

    (params, returns, leftover)
}

fn identifier_from_token(token: &Token) -> Identifier {
    let start = TextPosition::from(token.start_position());
    let end = TextPosition::from(token.end_position());
    let name = token.to_string().trim().to_string();
    Identifier::new(name, TextRange { start, end })
}

fn identifier_from_token_ref(token: &TokenReference) -> Identifier {
    identifier_from_token(token.token())
}

fn merge_ranges(a: TextRange, b: TextRange) -> TextRange {
    match (is_valid_range(&a), is_valid_range(&b)) {
        (true, true) => TextRange {
            start: min_position(a.start, b.start),
            end: max_position(a.end, b.end),
        },
        (true, false) => a,
        (false, true) => b,
        (false, false) => a,
    }
}

fn min_position(a: TextPosition, b: TextPosition) -> TextPosition {
    if (a.line, a.character) <= (b.line, b.character) {
        a
    } else {
        b
    }
}

fn max_position(a: TextPosition, b: TextPosition) -> TextPosition {
    if (a.line, a.character) >= (b.line, b.character) {
        a
    } else {
        b
    }
}

fn is_valid_range(range: &TextRange) -> bool {
    is_valid_position(range.start) || is_valid_position(range.end)
}

fn is_valid_position(position: TextPosition) -> bool {
    position.line != 0 || position.character != 0
}

fn token_range<T: Node>(node: &T) -> TextRange {
    let (start, end) = (node.start_position(), node.end_position());
    let start = start.map(Into::into);
    let end = end.map(Into::into);
    match (start, end) {
        (Some(start), Some(end)) => TextRange { start, end },
        (Some(start), None) => TextRange { start, end: start },
        (None, Some(end)) => TextRange { start: end, end },
        (None, None) => TextRange {
            start: TextPosition {
                line: 0,
                character: 0,
            },
            end: TextPosition {
                line: 0,
                character: 0,
            },
        },
    }
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
            func.name
                .path
                .iter()
                .map(|id| id.name.clone())
                .collect::<Vec<_>>(),
            vec!["mod".to_string(), "example".to_string()]
        );
        assert_eq!(
            func.name.method.as_ref().map(|id| id.name.clone()),
            Some("run".into())
        );
        assert_eq!(func.params.len(), 2);
        assert!(func.param_types.contains_key("a"));
        assert_eq!(func.returns.len(), 1);
        assert!(func.returns[0].name.is_none());
        assert_eq!(func.returns[0].ty.raw, "boolean");
        assert!(matches!(func.body.stmts.last(), Some(Stmt::Return(_))));
    }

    #[test]
    fn capture_multiple_return_annotations_with_names() {
        let source = unindent(
            r#"
            ---@return number result
            ---@return string? err
            local function multi()
                return 1, "ok"
            end
            "#,
        );

        let (ast, annotations) = parse(&source);
        let program = build_typed_ast(&source, &ast, &annotations);

        let Stmt::LocalFunction(func) = &program.block.stmts[0] else {
            panic!("expected local function stmt");
        };

        assert_eq!(func.returns.len(), 2);
        assert_eq!(func.returns[0].name.as_deref(), Some("result"));
        assert_eq!(func.returns[0].ty.raw, "number");
        assert_eq!(func.returns[1].name.as_deref(), Some("err"));
        assert_eq!(func.returns[1].ty.raw, "string?");
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
    fn convert_label_and_goto_statements() {
        let source = unindent(
            r#"
            ::continue::
            sum = sum + 1
            goto continue
            "#,
        );

        let (ast, annotations) = parse(&source);
        let program = build_typed_ast(&source, &ast, &annotations);

        assert_eq!(program.block.stmts.len(), 3);
        let Stmt::Label(label) = &program.block.stmts[0] else {
            panic!("expected label statement");
        };
        assert_eq!(label.name.name, "continue");

        let Stmt::Goto(goto) = &program.block.stmts[2] else {
            panic!("expected goto statement");
        };
        assert_eq!(goto.name.name, "continue");
    }

    #[test]
    fn merge_ranges_prefers_valid_ranges() {
        let invalid = TextRange {
            start: TextPosition {
                line: 0,
                character: 0,
            },
            end: TextPosition {
                line: 0,
                character: 0,
            },
        };
        let valid = TextRange {
            start: TextPosition {
                line: 2,
                character: 1,
            },
            end: TextPosition {
                line: 2,
                character: 5,
            },
        };

        let merged = merge_ranges(invalid, valid);
        assert_eq!(merged.start.line, 2);
        assert_eq!(merged.end.character, 5);

        let merged_reverse = merge_ranges(valid, invalid);
        assert_eq!(merged_reverse.start.line, 2);
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
        assert_eq!(local.names.len(), 1);
        assert_eq!(local.names[0].name, "mapper");
        let ExprKind::Function(function_expr) = &local.values[0].kind else {
            panic!("expected anonymous function");
        };
        assert_eq!(function_expr.params.len(), 1);
        let Stmt::Return(return_stmt) = &function_expr.body.stmts[0] else {
            panic!("expected return inside function body");
        };
        let ExprKind::TableConstructor(fields) = &return_stmt.values[0].kind else {
            panic!("expected table constructor");
        };
        assert_eq!(fields.len(), 2);
    }
}
