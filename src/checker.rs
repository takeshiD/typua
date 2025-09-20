use std::{
    collections::HashMap,
    fmt, fs,
    path::{Path, PathBuf},
};

use full_moon::{
    self, Error as FullMoonError, ast,
    node::Node,
    tokenizer::{Position, Symbol, TokenReference, TokenType},
};

use crate::{
    cli::CheckOptions,
    diagnostics::{Diagnostic, Severity, TextPosition, TextRange},
    error::{Result, TypuaError},
    workspace,
};

#[derive(Debug, Default)]
pub struct CheckReport {
    pub files_checked: usize,
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diag| matches!(diag.severity, Severity::Error))
    }
}

pub fn run(options: &CheckOptions) -> Result<CheckReport> {
    let files = workspace::collect_source_files(&options.target, &options.config)?;

    let mut report = CheckReport {
        files_checked: files.len(),
        diagnostics: Vec::new(),
    };

    for path in &files {
        let source = read_source(path)?;
        match full_moon::parse(&source) {
            Ok(ast) => {
                let mut diagnostics = type_check_ast(path, &source, &ast);
                report.diagnostics.append(&mut diagnostics);
            }
            Err(errors) => {
                for error in errors {
                    report.diagnostics.push(to_syntax_diagnostic(path, error));
                }
            }
        }
    }

    Ok(report)
}

fn read_source(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|source| TypuaError::SourceRead {
        path: path.to_path_buf(),
        source,
    })
}

fn to_syntax_diagnostic(path: &Path, error: FullMoonError) -> Diagnostic {
    let range = error_range(&error);
    Diagnostic::error(path.to_path_buf(), error.error_message(), range)
}

fn error_range(error: &FullMoonError) -> Option<TextRange> {
    let (start, end) = error.range();
    let start = TextPosition::from(start);
    let end = TextPosition::from(end);
    Some(TextRange { start, end })
}

fn type_check_ast(path: &Path, source: &str, ast: &ast::Ast) -> Vec<Diagnostic> {
    let annotations = AnnotationIndex::from_source(source);
    TypeChecker::new(path, annotations).check(ast)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TypeKind {
    Unknown,
    Nil,
    Boolean,
    Number,
    String,
    Table,
    Function,
}

impl TypeKind {
    fn describe(self) -> &'static str {
        match self {
            TypeKind::Unknown => "unknown",
            TypeKind::Nil => "nil",
            TypeKind::Boolean => "boolean",
            TypeKind::Number => "number",
            TypeKind::String => "string",
            TypeKind::Table => "table",
            TypeKind::Function => "function",
        }
    }
}

impl fmt::Display for TypeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.describe())
    }
}

#[derive(Clone, Debug)]
struct AnnotatedType {
    raw: String,
    kind: Option<TypeKind>,
}

impl AnnotatedType {
    fn new(raw: String) -> Self {
        let kind = parse_type_name(&raw);
        Self { raw, kind }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AnnotationUsage {
    Type,
    Param,
}

#[derive(Clone, Debug)]
struct Annotation {
    usage: AnnotationUsage,
    name: Option<String>,
    ty: AnnotatedType,
}

#[derive(Default)]
struct AnnotationIndex {
    by_line: HashMap<usize, Vec<Annotation>>,
}

impl AnnotationIndex {
    fn from_source(source: &str) -> Self {
        let mut by_line: HashMap<usize, Vec<Annotation>> = HashMap::new();
        let mut pending: Vec<Annotation> = Vec::new();

        for (idx, line) in source.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim_start();

            if let Some(annotation) = parse_annotation(trimmed) {
                pending.push(annotation);
                continue;
            }

            if trimmed.is_empty() || (trimmed.starts_with("--") && !trimmed.starts_with("---@")) {
                continue;
            }

            if !pending.is_empty() {
                by_line
                    .entry(line_no)
                    .or_default()
                    .extend(pending.drain(..));
            }
        }

        Self { by_line }
    }

    fn take(&mut self, line: usize) -> Vec<Annotation> {
        self.by_line.remove(&line).unwrap_or_default()
    }
}

fn parse_annotation(line: &str) -> Option<Annotation> {
    if let Some(rest) = line.strip_prefix("---@type") {
        let mut parts = rest.trim().split_whitespace();
        let type_token = parts.next()?;
        let name = parts.next().map(|value| value.to_string());
        let ty = AnnotatedType::new(type_token.to_string());
        return Some(Annotation {
            usage: AnnotationUsage::Type,
            name,
            ty,
        });
    }

    if let Some(rest) = line.strip_prefix("---@param") {
        let mut parts = rest.trim().split_whitespace();
        let name = parts.next()?.to_string();
        let type_token = parts.next().unwrap_or("any");
        let ty = AnnotatedType::new(type_token.to_string());
        return Some(Annotation {
            usage: AnnotationUsage::Param,
            name: Some(name),
            ty,
        });
    }

    None
}

fn parse_type_name(raw: &str) -> Option<TypeKind> {
    let normalized = raw.trim();
    if normalized.is_empty() {
        return None;
    }

    let mut lower = normalized.to_ascii_lowercase();

    if lower.ends_with('?') {
        lower.pop();
    }

    if lower.ends_with("[]") {
        return Some(TypeKind::Table);
    }

    if lower.starts_with("table<") || lower == "table" {
        return Some(TypeKind::Table);
    }

    if lower.starts_with("fun") || lower == "function" {
        return Some(TypeKind::Function);
    }

    match lower.as_str() {
        "nil" => Some(TypeKind::Nil),
        "boolean" | "bool" => Some(TypeKind::Boolean),
        "string" => Some(TypeKind::String),
        "number" => Some(TypeKind::Number),
        "integer" | "int" => Some(TypeKind::Number),
        "table" => Some(TypeKind::Table),
        "function" => Some(TypeKind::Function),
        "any" => None,
        _ => None,
    }
}

struct TypeChecker<'a> {
    path: &'a Path,
    diagnostics: Vec<Diagnostic>,
    scopes: Vec<HashMap<String, TypeKind>>,
    annotations: AnnotationIndex,
}

impl<'a> TypeChecker<'a> {
    fn new(path: &'a Path, annotations: AnnotationIndex) -> Self {
        Self {
            path,
            diagnostics: Vec::new(),
            scopes: Vec::new(),
            annotations,
        }
    }

    fn check(mut self, ast: &ast::Ast) -> Vec<Diagnostic> {
        self.scopes.push(HashMap::new());
        self.check_block(ast.nodes());
        self.scopes.pop();
        self.diagnostics
    }

    fn check_block(&mut self, block: &ast::Block) {
        for stmt in block.stmts() {
            self.check_stmt(stmt);
        }

        if let Some(last_stmt) = block.last_stmt() {
            self.check_last_stmt(last_stmt);
        }
    }

    fn with_new_scope<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.scopes.push(HashMap::new());
        f(self);
        self.scopes.pop();
    }

    fn take_line_annotations(&mut self, line: usize) -> Vec<Annotation> {
        self.annotations.take(line)
    }

    fn extract_param_annotations(
        annotations: &mut Vec<Annotation>,
    ) -> HashMap<String, AnnotatedType> {
        let mut map = HashMap::new();
        let mut index = 0;
        while index < annotations.len() {
            if annotations[index].usage == AnnotationUsage::Param {
                if let Some(name) = annotations[index].name.clone() {
                    map.insert(name, annotations[index].ty.clone());
                }
                annotations.remove(index);
            } else {
                index += 1;
            }
        }
        map
    }

    fn apply_type_annotation(
        &mut self,
        name: &str,
        token: &TokenReference,
        inferred: TypeKind,
        annotations: &mut Vec<Annotation>,
    ) -> TypeKind {
        let idx = annotations
            .iter()
            .position(|annotation| {
                annotation.usage == AnnotationUsage::Type
                    && annotation.name.as_deref() == Some(name)
            })
            .or_else(|| {
                annotations.iter().position(|annotation| {
                    annotation.usage == AnnotationUsage::Type && annotation.name.is_none()
                })
            });

        if let Some(position) = idx {
            let annotation = annotations.remove(position);
            if let Some(expected) = annotation.ty.kind {
                if inferred != TypeKind::Unknown && inferred != expected {
                    let message = format!(
                        "variable '{name}' is annotated as type {} but inferred type is {}",
                        annotation.ty.raw, inferred
                    );
                    self.push_diagnostic(token, message);
                }
                expected
            } else {
                inferred
            }
        } else {
            inferred
        }
    }

    fn check_stmt(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::LocalAssignment(local) => self.check_local_assignment(local),
            ast::Stmt::Assignment(assignment) => self.check_assignment(assignment),
            ast::Stmt::LocalFunction(local_fn) => self.check_local_function(local_fn),
            ast::Stmt::FunctionDeclaration(function) => self.check_function_declaration(function),
            ast::Stmt::Do(do_block) => {
                self.with_new_scope(|checker| checker.check_block(do_block.block()))
            }
            ast::Stmt::If(if_stmt) => self.check_if(if_stmt),
            ast::Stmt::While(while_stmt) => self.check_while(while_stmt),
            ast::Stmt::Repeat(repeat_stmt) => self.check_repeat(repeat_stmt),
            ast::Stmt::NumericFor(numeric_for) => self.check_numeric_for(numeric_for),
            ast::Stmt::GenericFor(generic_for) => self.check_generic_for(generic_for),
            _ => {}
        }
    }

    fn check_last_stmt(&mut self, last_stmt: &ast::LastStmt) {
        if let ast::LastStmt::Return(ret) = last_stmt {
            for pair in ret.returns().pairs() {
                self.infer_expression(pair.value());
            }
        }
    }

    fn check_local_assignment(&mut self, assignment: &ast::LocalAssignment) {
        let line = assignment.local_token().token().start_position().line();
        let mut annotations = self.take_line_annotations(line);

        let expr_types: Vec<TypeKind> = assignment
            .expressions()
            .pairs()
            .map(|pair| self.infer_expression(pair.value()))
            .collect();

        for (index, pair) in assignment.names().pairs().enumerate() {
            let token = pair.value();
            if let Some(name) = token_identifier(token) {
                let inferred = expr_types.get(index).copied().unwrap_or(TypeKind::Nil);
                let ty = self.apply_type_annotation(&name, token, inferred, &mut annotations);
                self.assign_local(&name, token, ty);
            }
        }
    }

    fn check_assignment(&mut self, assignment: &ast::Assignment) {
        let line = assignment
            .variables()
            .pairs()
            .next()
            .and_then(|pair| pair.value().start_position())
            .map(|position| position.line())
            .unwrap_or(0);
        let mut annotations = if line > 0 {
            self.take_line_annotations(line)
        } else {
            Vec::new()
        };

        let expr_types: Vec<TypeKind> = assignment
            .expressions()
            .pairs()
            .map(|pair| self.infer_expression(pair.value()))
            .collect();

        for (index, pair) in assignment.variables().pairs().enumerate() {
            if let ast::Var::Name(token) = pair.value() {
                if let Some(name) = token_identifier(token) {
                    let inferred = expr_types.get(index).copied().unwrap_or(TypeKind::Nil);
                    let ty = self.apply_type_annotation(&name, token, inferred, &mut annotations);
                    self.assign_nonlocal(&name, token, ty);
                }
            }
        }
    }

    fn check_local_function(&mut self, local_fn: &ast::LocalFunction) {
        let line = local_fn.local_token().token().start_position().line();
        let mut annotations = self.take_line_annotations(line);
        let mut param_annotations = TypeChecker::extract_param_annotations(&mut annotations);

        if let Some(name) = token_identifier(local_fn.name()) {
            let inferred = TypeKind::Function;
            let ty = self.apply_type_annotation(&name, local_fn.name(), inferred, &mut annotations);
            self.assign_local(&name, local_fn.name(), ty);
        }

        self.with_new_scope(|checker| {
            checker.bind_function_parameters(local_fn.body(), &mut param_annotations);
            checker.check_block(local_fn.body().block());
        });
    }

    fn check_function_declaration(&mut self, function: &ast::FunctionDeclaration) {
        let line = function.function_token().token().start_position().line();
        let mut annotations = self.take_line_annotations(line);
        let mut param_annotations = TypeChecker::extract_param_annotations(&mut annotations);

        if let Some(token) = target_function_name(function.name()) {
            if let Some(name) = token_identifier(token) {
                let inferred = TypeKind::Function;
                let ty = self.apply_type_annotation(&name, token, inferred, &mut annotations);
                self.assign_nonlocal(&name, token, ty);
            }
        }

        self.with_new_scope(|checker| {
            checker.bind_function_parameters(function.body(), &mut param_annotations);
            checker.check_block(function.body().block());
        });
    }

    fn check_if(&mut self, if_stmt: &ast::If) {
        self.infer_expression(if_stmt.condition());
        self.with_new_scope(|checker| checker.check_block(if_stmt.block()));

        if let Some(elseifs) = if_stmt.else_if() {
            for elseif in elseifs {
                self.infer_expression(elseif.condition());
                self.with_new_scope(|checker| checker.check_block(elseif.block()));
            }
        }

        if let Some(block) = if_stmt.else_block() {
            self.with_new_scope(|checker| checker.check_block(block));
        }
    }

    fn check_while(&mut self, while_stmt: &ast::While) {
        self.infer_expression(while_stmt.condition());
        self.with_new_scope(|checker| checker.check_block(while_stmt.block()));
    }

    fn check_repeat(&mut self, repeat_stmt: &ast::Repeat) {
        self.with_new_scope(|checker| checker.check_block(repeat_stmt.block()));
        self.infer_expression(repeat_stmt.until());
    }

    fn check_numeric_for(&mut self, numeric_for: &ast::NumericFor) {
        self.infer_expression(numeric_for.start());
        self.infer_expression(numeric_for.end());
        if let Some(step) = numeric_for.step() {
            self.infer_expression(step);
        }

        self.with_new_scope(|checker| {
            if let Some(name) = token_identifier(numeric_for.index_variable()) {
                checker.assign_local(&name, numeric_for.index_variable(), TypeKind::Number);
            }
            checker.check_block(numeric_for.block());
        });
    }

    fn check_generic_for(&mut self, generic_for: &ast::GenericFor) {
        for pair in generic_for.expressions().pairs() {
            self.infer_expression(pair.value());
        }

        self.with_new_scope(|checker| {
            for pair in generic_for.names().pairs() {
                if let Some(name) = token_identifier(pair.value()) {
                    checker.assign_local(&name, pair.value(), TypeKind::Unknown);
                }
            }
            checker.check_block(generic_for.block());
        });
    }

    fn bind_function_parameters(
        &mut self,
        body: &ast::FunctionBody,
        param_annotations: &mut HashMap<String, AnnotatedType>,
    ) {
        for pair in body.parameters().pairs() {
            if let ast::Parameter::Name(token) = pair.value() {
                if let Some(name) = token_identifier(token) {
                    let mut ty = TypeKind::Unknown;
                    if let Some(annotation) = param_annotations.remove(&name) {
                        if let Some(expected) = annotation.kind {
                            ty = expected;
                        }
                    }
                    self.assign_local(&name, token, ty);
                }
            }
        }
    }

    fn assign_local(&mut self, name: &str, token: &TokenReference, ty: TypeKind) {
        self.emit_reassignment(name, token, ty);
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_owned(), ty);
        }
    }

    fn assign_nonlocal(&mut self, name: &str, token: &TokenReference, ty: TypeKind) {
        self.emit_reassignment(name, token, ty);

        if let Some(index) = self.lookup_scope_index(name) {
            if let Some(scope) = self.scopes.get_mut(index) {
                scope.insert(name.to_owned(), ty);
                return;
            }
        }

        if let Some(global) = self.scopes.first_mut() {
            global.insert(name.to_owned(), ty);
        }
    }

    fn emit_reassignment(&mut self, name: &str, token: &TokenReference, ty: TypeKind) {
        if ty == TypeKind::Unknown {
            return;
        }

        if let Some(existing) = self.lookup(name) {
            if existing != TypeKind::Unknown && existing != ty {
                let message = format!(
                    "variable '{name}' was previously inferred as type {existing} but is now assigned type {ty}"
                );
                self.push_diagnostic(token, message);
            }
        }
    }

    fn lookup(&self, name: &str) -> Option<TypeKind> {
        for scope in self.scopes.iter().rev() {
            if let Some(&ty) = scope.get(name) {
                return Some(ty);
            }
        }
        None
    }

    fn lookup_scope_index(&self, name: &str) -> Option<usize> {
        self.scopes
            .iter()
            .enumerate()
            .rev()
            .find(|(_, scope)| scope.contains_key(name))
            .map(|(idx, _)| idx)
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
            ast::Expression::FunctionCall(_) => TypeKind::Unknown,
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

    fn infer_var(&self, var: &ast::Var) -> TypeKind {
        match var {
            ast::Var::Name(token) => token_identifier(token)
                .and_then(|name| self.lookup(&name))
                .unwrap_or(TypeKind::Unknown),
            ast::Var::Expression(_) => TypeKind::Unknown,
            _ => TypeKind::Unknown,
        }
    }

    fn infer_binary(
        &mut self,
        lhs: &ast::Expression,
        binop: &ast::BinOp,
        rhs: &ast::Expression,
    ) -> TypeKind {
        let op_token = binop.token();
        let symbol = match op_token.token_type() {
            TokenType::Symbol { symbol } => symbol,
            _ => return TypeKind::Unknown,
        };

        match symbol {
            Symbol::Plus
            | Symbol::Minus
            | Symbol::Star
            | Symbol::Slash
            | Symbol::Percent
            | Symbol::Caret => {
                let left = self.infer_expression(lhs);
                let right = self.infer_expression(rhs);
                self.expect_type(op_token, left, TypeKind::Number, OperandSide::Left);
                self.expect_type(op_token, right, TypeKind::Number, OperandSide::Right);
                TypeKind::Number
            }
            Symbol::TwoDots => {
                let left = self.infer_expression(lhs);
                let right = self.infer_expression(rhs);
                self.expect_type(op_token, left, TypeKind::String, OperandSide::Left);
                self.expect_type(op_token, right, TypeKind::String, OperandSide::Right);
                TypeKind::String
            }
            Symbol::And | Symbol::Or => {
                let left = self.infer_expression(lhs);
                let right = self.infer_expression(rhs);
                self.expect_type(op_token, left, TypeKind::Boolean, OperandSide::Left);
                self.expect_type(op_token, right, TypeKind::Boolean, OperandSide::Right);
                TypeKind::Boolean
            }
            _ => {
                self.infer_expression(lhs);
                self.infer_expression(rhs);
                TypeKind::Unknown
            }
        }
    }

    fn expect_type(
        &mut self,
        token: &TokenReference,
        actual: TypeKind,
        expected: TypeKind,
        side: OperandSide,
    ) {
        if actual == TypeKind::Unknown || actual == expected {
            return;
        }

        let message = format!(
            "operator '{}' expected {} operand of type {}, but found {}",
            operator_label(token),
            side.describe(),
            expected,
            actual
        );
        self.push_diagnostic(token, message);
    }

    fn push_diagnostic(&mut self, token: &TokenReference, message: String) {
        let range = self.range_from_token(token);
        self.diagnostics
            .push(Diagnostic::error(self.path_buf(), message, Some(range)));
    }

    fn range_from_token(&self, token: &TokenReference) -> TextRange {
        let start: Position = token.token().start_position();
        let end: Position = token.token().end_position();
        TextRange {
            start: TextPosition::from(start),
            end: TextPosition::from(end),
        }
    }

    fn path_buf(&self) -> PathBuf {
        self.path.to_path_buf()
    }
}

#[derive(Clone, Copy)]
enum OperandSide {
    Left,
    Right,
}

impl OperandSide {
    fn describe(self) -> &'static str {
        match self {
            OperandSide::Left => "left",
            OperandSide::Right => "right",
        }
    }
}

fn token_identifier(token: &TokenReference) -> Option<String> {
    match token.token().token_type() {
        TokenType::Identifier { identifier } => Some(identifier.to_string()),
        _ => None,
    }
}

fn operator_label(token: &TokenReference) -> String {
    token.token().to_string()
}

fn target_function_name(name: &ast::FunctionName) -> Option<&TokenReference> {
    if name.method_name().is_some() {
        return None;
    }

    let mut iter = name.names().pairs();
    let first = iter.next()?;
    if iter.next().is_some() {
        return None;
    }
    Some(first.value())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::path::Path;

    fn run_type_check(source: &str) -> Vec<Diagnostic> {
        let ast = full_moon::parse(source).expect("failed to parse test source");
        TypeChecker::new(Path::new("test.lua"), AnnotationIndex::from_source(source)).check(&ast)
    }

    #[test]
    fn reports_variable_reassignment_type_conflict() {
        let diagnostics = run_type_check(
            r#"
            local x = 1
            x = "oops"
            "#,
        );

        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(
            diagnostic
                .message
                .contains("variable 'x' was previously inferred as type number")
        );
    }

    #[test]
    fn reports_arithmetic_operand_type_mismatch() {
        let diagnostics = run_type_check(
            r#"
            local a = "hello"
            local b = a + 1
            "#,
        );

        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(
            diagnostic
                .message
                .contains("operator '+' expected left operand of type number")
        );
    }

    #[test]
    fn allows_consistent_numeric_assignments() {
        let diagnostics = run_type_check(
            r#"
            local value = 1
            value = value + 2
            "#,
        );

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn reports_mismatch_with_type_annotation() {
        let diagnostics = run_type_check(
            r#"
            ---@type string
            local title = 10
            "#,
        );

        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert!(diagnostic.message.contains("annotated as type string"));
    }

    #[test]
    fn reports_mismatch_with_named_annotation() {
        let diagnostics = run_type_check(
            r#"
            ---@type number counter
            counter = "oops"
            "#,
        );

        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert!(diagnostic.message.contains("annotated as type number"));
    }

    #[test]
    fn param_annotation_enforces_type_in_body() {
        let diagnostics = run_type_check(
            r#"
            ---@param amount number
            local function charge(amount)
                amount = "free"
            end
            "#,
        );

        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert!(
            diagnostic
                .message
                .contains("variable 'amount' was previously inferred as type number")
        );
    }
}
