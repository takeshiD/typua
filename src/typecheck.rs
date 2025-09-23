use crate::diagnostics::Diagnostic;
use std::collections::HashMap;
use std::path::Path;

use full_moon::ast;
use full_moon::tokenizer::TokenType;

#[derive(Clone, Debug, PartialEq, Eq)]
enum TypeKind {
    Unknown,
    Nil,
    Boolean,
    Number,
    String,
    Table,
    Function,
    Thread,
    Custom(String),
    Union(Vec<TypeKind>),
    Array(Box<TypeKind>),
    Generic(String),
    Applied {
        base: Box<TypeKind>,
        args: Vec<TypeKind>,
    },
    FunctionSig(Box<FunctionType>),
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct FunctionType {
    generics: Vec<String>,
    params: Vec<FunctionParam>,
    returns: Vec<TypeKind>,
    vararg: Option<Box<TypeKind>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FunctionParam {
    name: Option<String>,
    ty: TypeKind,
    is_self: bool,
    is_vararg: bool,
}

#[derive(Clone, Debug)]
struct CheckResult {
    pub diagnostics: Vec<Diagnostic>,
    // pub type_map: HashMap<DocumentPosition, TypeInfo>,
}

struct Environment {
    envs: Vec<HashMap<String, TypeKind>>, // x: TypeKind::Number
}

impl Environment {
    fn new() -> Self {
        Self {
            envs: Vec::new()
        }
    }
    fn 
}

struct TypeChecker<'a> {
    // The path target source code
    path: &'a Path,
    // Detected typecheck diagnostic results with position on source
    diagnostics: Vec<Diagnostic>,
    // Variables infomation on source
    envs: Vec<HashMap<String, TypeKind>>,
    // Detected type annotations on source
    // annotations: AnnotationIndex,
    //
    // type_registry: TypeRegistry,
    // return_expectations: Vec<Vec<AnnotatedType>>,
    // type_info: HashMap<DocumentPosition, TypeInfo>,
}

impl<'a> TypeChecker<'a> {
    fn new(path: &'a Path) -> Self {
        Self {
            path,
            diagnostics: Vec::new(),
            envs: Vec::new(),
        }
    }

    fn check(&mut self, ast: &ast::Ast) -> CheckResult {
        // top scope on source
        self.envs.push(HashMap::new());
        self.check_block(ast.nodes());
        self.envs.pop();
        CheckResult {
            diagnostics: self.diagnostics,
            // type_map: self.type_info,
        }
    }

    fn check_block(&mut self, block: &ast::Block) {
        for stmt in block.stmts() {
            self.check_stmt(stmt);
        }
        if let Some(last_stmt) = block.last_stmt() {
            self.check_last_stmt(last_stmt);
        }
    }

    fn check_stmt(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::LocalAssignment(local) => self.check_local_assignment(local),
            ast::Stmt::Assignment(assignment) => self.check_assignment(assignment),
            ast::Stmt::LocalFunction(local_fn) => {
                unimplemented!("LocalFunction is unimplemented yet");
                // self.check_local_function(local_fn)
            }
            ast::Stmt::FunctionDeclaration(function) => {
                unimplemented!("FunctionDeclaration is unimplemented yet");
                // self.check_function_declaration(function)
            }
            ast::Stmt::Do(do_block) => {
                unimplemented!("Do statement is unimplemented yet");
                // self.with_new_scope(|checker| checker.check_block(do_block.block()))
            }
            ast::Stmt::If(if_stmt) => {
                unimplemented!("If statement is implemented yet");
                // self.check_if(if_stmt)
            }
            ast::Stmt::While(while_stmt) => {
                unimplemented!("While statement is implemented yet");
                // self.check_while(while_stmt)
            }
            ast::Stmt::Repeat(repeat_stmt) => {
                unimplemented!("Repeat statement is implemented yet");
                // self.check_repeat(repeat_stmt)
            }
            ast::Stmt::NumericFor(numeric_for) => {
                unimplemented!("NumericFor statement is implemented yet");
                // self.check_numeric_for(numeric_for)
            }
            ast::Stmt::GenericFor(generic_for) => {
                unimplemented!("GenericFor statement is implemented yet");
                // self.check_generic_for(generic_for)
            }
            _ => {}
        }
    }

    fn check_last_stmt(&mut self, last_stmt: &ast::LastStmt) {
        unimplemented!("LastStatement is unimplemented yet");
        // if let ast::LastStmt::Return(ret) = last_stmt {
        //     self.validate_return(ret);
        // }
    }

    fn check_local_assignment(&mut self, assignment: &ast::LocalAssignment) {
        let line = assignment.local_token().token().start_position().line();
        // let mut annotations = self.anno

        let expr_types: Vec<TypeKind> = assignment
            .expressions()
            .pairs()
            .map(|pair| self.infer_expression(pair.value()))
            .collect();

        for (index, pair) in assignment.names().pairs().enumerate() {
            let token = pair.value();
            if let Some(name) = token_identifier(token) {
                let inferred = expr_types.get(index).cloned().unwrap_or(TypeKind::Nil);
                let ty = self.apply_type_annotation(&name, token, inferred, &mut annotations);
                self.assign_local(&name, token, ty);
            }
        }
    }

    fn infer_expression(&mut self, expression: &ast::Expression) -> TypeKind {
        match expression {
            ast::Expression::Number(_) => TypeKind::Number,
            ast::Expression::String(_) => TypeKind::String,
            ast::Expression::TableConstructor(_) => TypeKind::Table,
            ast::Expression::Function(_) => TypeKind::Function,
            ast::Expression::Parentheses { expression, .. } => self.infer_expression(expression),
            ast::Expression::UnaryOperator { expression, .. } => self.infer_expression(expression),
            ast::Expression::BinaryOperator { lhs, binop, rhs } => {
                self.infer_binary(lhs, binop, rhs)
            }
            ast::Expression::FunctionCall(call) => {
                self.try_record_function_call_type(call);
                TypeKind::Unknown
            }
            ast::Expression::Var(var) => self.infer_var(var),
            ast::Expression::Symbol(token) => match token.token().token_type() {
                TokenType::Symbol {
                    symbol: Symbol::True,
                }
                | TokenType::Symbol {
                    symbol: Symbol::False,
                } => TypeKind::Boolean,
                TokenType::Symbol {
                    symbol: Symbol::Nil,
                } => TypeKind::Nil,
                _ => TypeKind::Unknown,
            },
            _ => TypeKind::Unknown,
        }
    }
}
