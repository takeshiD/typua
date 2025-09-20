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
    let (annotations, type_registry) = AnnotationIndex::from_source(source);
    TypeChecker::new(path, annotations, type_registry).check(ast)
}

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
}

impl TypeKind {
    fn describe(&self) -> &'static str {
        match self {
            TypeKind::Unknown => "unknown",
            TypeKind::Nil => "nil",
            TypeKind::Boolean => "boolean",
            TypeKind::Number => "number",
            TypeKind::String => "string",
            TypeKind::Table => "table",
            TypeKind::Function => "function",
            TypeKind::Thread => "thread",
            TypeKind::Custom(_) => "custom",
        }
    }
}

impl fmt::Display for TypeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeKind::Custom(name) => f.write_str(name),
            _ => f.write_str(self.describe()),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct ClassInfo {
    exact: bool,
    parent: Option<String>,
    fields: HashMap<String, AnnotatedType>,
}

impl ClassInfo {
    fn new(exact: bool, parent: Option<String>) -> Self {
        Self {
            exact,
            parent,
            fields: HashMap::new(),
        }
    }
}

#[derive(Default)]
struct TypeRegistry {
    classes: HashMap<String, ClassInfo>,
    enums: HashMap<String, ()>,
}

impl TypeRegistry {
    fn register_class(&mut self, decl: ClassDeclaration) {
        let name = decl.name.clone();
        let entry = self
            .classes
            .entry(name)
            .or_insert_with(|| ClassInfo::new(decl.exact, decl.parent.clone()));
        entry.exact = decl.exact;
        entry.parent = decl.parent;
    }

    fn register_enum(&mut self, name: &str) {
        self.enums.insert(name.to_string(), ());
    }

    fn register_field(&mut self, class: &str, field: &str, ty: AnnotatedType) {
        let entry = self
            .classes
            .entry(class.to_string())
            .or_insert_with(|| ClassInfo::new(false, None));
        entry.fields.insert(field.to_string(), ty);
    }

    fn resolve(&self, name: &str) -> Option<TypeKind> {
        if self.classes.contains_key(name) {
            Some(TypeKind::Custom(name.to_string()))
        } else if self.enums.contains_key(name) {
            Some(TypeKind::String)
        } else {
            None
        }
    }

    fn field_annotation(&self, class: &str, field: &str) -> Option<&AnnotatedType> {
        let mut current = Some(class);
        while let Some(name) = current {
            if let Some(info) = self.classes.get(name) {
                if let Some(annotation) = info.fields.get(field) {
                    return Some(annotation);
                }
                current = info.parent.as_deref();
            } else {
                break;
            }
        }
        None
    }

    fn is_exact(&self, class: &str) -> bool {
        self.classes
            .get(class)
            .map(|info| info.exact)
            .unwrap_or(false)
    }
}

#[derive(Clone, Debug)]
struct AnnotatedType {
    raw: String,
    kind: Option<TypeKind>,
}

impl AnnotatedType {
    fn new(raw: String) -> Self {
        let kind = parse_builtin_type(&raw);
        Self { raw, kind }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AnnotationUsage {
    Type,
    Param,
    Return,
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
    fn from_source(source: &str) -> (Self, TypeRegistry) {
        let mut by_line: HashMap<usize, Vec<Annotation>> = HashMap::new();
        let mut pending: Vec<Annotation> = Vec::new();
        let mut registry = TypeRegistry::default();
        let mut current_class: Option<String> = None;

        for (idx, line) in source.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim_start();

            if let Some(decl) = parse_class_declaration(trimmed) {
                current_class = Some(decl.name.clone());
                registry.register_class(decl);
                continue;
            }

            if let Some(name) = parse_enum_declaration(trimmed) {
                registry.register_enum(&name);
                current_class = None;
                continue;
            }

            if let Some((field_name, field_ty)) = parse_field_declaration(trimmed) {
                if let Some(class_name) = current_class.clone() {
                    registry.register_field(&class_name, &field_name, field_ty);
                }
                continue;
            }

            if let Some(annotation) = parse_annotation(trimmed) {
                pending.push(annotation);
                continue;
            }

            if trimmed.is_empty() || (trimmed.starts_with("--") && !trimmed.starts_with("---@")) {
                continue;
            }

            current_class = None;

            if !pending.is_empty() {
                by_line.entry(line_no).or_default().append(&mut pending);
            }
        }

        (Self { by_line }, registry)
    }

    fn take(&mut self, line: usize) -> Vec<Annotation> {
        self.by_line.remove(&line).unwrap_or_default()
    }
}

fn parse_annotation(line: &str) -> Option<Annotation> {
    if let Some(rest) = line.strip_prefix("---@type") {
        let mut parts = rest.split_whitespace();
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
        let mut parts = rest.split_whitespace();
        let name = parts.next()?.to_string();
        let type_token = parts.next().unwrap_or("any");
        let ty = AnnotatedType::new(type_token.to_string());
        return Some(Annotation {
            usage: AnnotationUsage::Param,
            name: Some(name),
            ty,
        });
    }

    if let Some(rest) = line.strip_prefix("---@return") {
        let mut parts = rest.split_whitespace();
        let type_token = parts.next().unwrap_or("any");
        let name = parts.next().map(|value| value.to_string());
        let ty = AnnotatedType::new(type_token.to_string());
        return Some(Annotation {
            usage: AnnotationUsage::Return,
            name,
            ty,
        });
    }

    None
}

#[derive(Clone, Debug)]
struct ClassDeclaration {
    name: String,
    exact: bool,
    parent: Option<String>,
}

fn parse_builtin_type(raw: &str) -> Option<TypeKind> {
    let normalized = raw.trim();
    if normalized.is_empty() {
        return None;
    }

    if normalized.starts_with('"')
        || normalized.starts_with('\'')
        || normalized.ends_with('"')
        || normalized.ends_with('\'')
    {
        return Some(TypeKind::String);
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
        "thread" => Some(TypeKind::Thread),
        "any" => None,
        _ => None,
    }
}

fn parse_class_declaration(line: &str) -> Option<ClassDeclaration> {
    let rest = line.strip_prefix("---@class")?.trim();
    let (rest, exact) = if let Some(remaining) = rest.strip_prefix("(exact)") {
        (remaining.trim(), true)
    } else {
        (rest, false)
    };

    let mut parts = rest.splitn(2, ':');
    let name_part = parts.next()?.trim();
    if name_part.is_empty() {
        return None;
    }

    let parent = parts
        .next()
        .and_then(|value| value.split_whitespace().next())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    Some(ClassDeclaration {
        name: name_part.to_string(),
        exact,
        parent,
    })
}

fn parse_enum_declaration(line: &str) -> Option<String> {
    let rest = line.strip_prefix("---@enum")?.trim();
    let name = rest.split_whitespace().next()?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn parse_field_declaration(line: &str) -> Option<(String, AnnotatedType)> {
    let rest = line.strip_prefix("---@field")?.trim();
    let mut parts = rest.split_whitespace();
    let name = parts.next()?.to_string();
    let type_part = parts.next().unwrap_or("any");
    Some((name, AnnotatedType::new(type_part.to_string())))
}

struct TypeChecker<'a> {
    path: &'a Path,
    diagnostics: Vec<Diagnostic>,
    scopes: Vec<HashMap<String, TypeKind>>,
    annotations: AnnotationIndex,
    type_registry: TypeRegistry,
    return_expectations: Vec<Vec<AnnotatedType>>,
}

impl<'a> TypeChecker<'a> {
    fn new(path: &'a Path, annotations: AnnotationIndex, type_registry: TypeRegistry) -> Self {
        Self {
            path,
            diagnostics: Vec::new(),
            scopes: Vec::new(),
            annotations,
            type_registry,
            return_expectations: Vec::new(),
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

    fn extract_function_annotations(
        annotations: &mut Vec<Annotation>,
    ) -> (HashMap<String, AnnotatedType>, Vec<AnnotatedType>) {
        let mut params = HashMap::new();
        let mut returns = Vec::new();
        let mut index = 0;
        while index < annotations.len() {
            match annotations[index].usage {
                AnnotationUsage::Param => {
                    if let Some(name) = annotations[index].name.clone() {
                        params.insert(name, annotations[index].ty.clone());
                    }
                    annotations.remove(index);
                }
                AnnotationUsage::Return => {
                    returns.push(annotations[index].ty.clone());
                    annotations.remove(index);
                }
                _ => {
                    index += 1;
                }
            }
        }
        (params, returns)
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
            if let Some(expected) = self.resolve_annotation_kind(&annotation.ty) {
                let compatible = match &expected {
                    TypeKind::Custom(_) => {
                        inferred == TypeKind::Unknown
                            || inferred == TypeKind::Table
                            || inferred == expected
                    }
                    _ => inferred == TypeKind::Unknown || inferred == expected,
                };

                if !compatible {
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

    fn resolve_annotation_kind(&self, annotation: &AnnotatedType) -> Option<TypeKind> {
        if let Some(kind) = annotation.kind.clone() {
            Some(kind)
        } else {
            self.type_registry.resolve(&annotation.raw)
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
            self.validate_return(ret);
        }
    }

    fn validate_return(&mut self, ret: &ast::Return) {
        let mut expr_info: Vec<(TypeKind, &ast::Expression)> = Vec::new();
        for pair in ret.returns().pairs() {
            let ty = self.infer_expression(pair.value());
            expr_info.push((ty, pair.value()));
        }

        let Some(expectations) = self.return_expectations.last() else {
            return;
        };
        let expectations = expectations.clone();

        let expected_len = expectations.len();
        let actual_len = expr_info.len();

        if actual_len > expected_len {
            let message = format!(
                "function returns {actual_len} value(s) but only {expected_len} annotated via @return"
            );
            self.push_diagnostic(ret.token(), message);
        }

        if actual_len < expected_len {
            let message = format!(
                "function annotated to return {expected_len} value(s) but this return statement provides {actual_len}"
            );
            self.push_diagnostic(ret.token(), message);
        }

        for (idx, annotation) in expectations.iter().enumerate() {
            if idx >= expr_info.len() {
                break;
            }

            let actual = expr_info[idx].0.clone();
            if let Some(expected) = self.resolve_annotation_kind(annotation)
                && actual != TypeKind::Unknown
                && actual != expected
            {
                let message = format!(
                    "return value #{} is annotated as type {} but inferred type is {}",
                    idx + 1,
                    annotation.raw,
                    actual
                );
                self.push_diagnostic(ret.token(), message);
            }
        }
    }

    fn validate_field_assignment(
        &mut self,
        class_name: &str,
        field_token: &TokenReference,
        value_type: &TypeKind,
    ) {
        let Some(field_name) = token_identifier(field_token) else {
            return;
        };

        if let Some(annotation) = self.type_registry.field_annotation(class_name, &field_name) {
            if let Some(expected) = self.resolve_annotation_kind(annotation)
                && value_type != &TypeKind::Unknown
                && value_type != &expected
            {
                let message = format!(
                    "field '{field_name}' in class {class_name} expects type {} but inferred type is {}",
                    annotation.raw, value_type
                );
                self.push_diagnostic(field_token, message);
            }
        } else if self.type_registry.is_exact(class_name) {
            let message = format!(
                "class {class_name} is declared exact; field '{field_name}' is not defined"
            );
            self.push_diagnostic(field_token, message);
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
                let inferred = expr_types.get(index).cloned().unwrap_or(TypeKind::Nil);
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
                    let inferred = expr_types.get(index).cloned().unwrap_or(TypeKind::Nil);
                    let ty = self.apply_type_annotation(&name, token, inferred, &mut annotations);
                    self.assign_nonlocal(&name, token, ty);
                }
            } else if let Some((base_token, field_token)) = extract_field_assignment(pair.value())
                && let Some(base_name) = token_identifier(base_token)
                && let Some(TypeKind::Custom(class_name)) = self.lookup(&base_name)
            {
                let value_type = expr_types.get(index).cloned().unwrap_or(TypeKind::Unknown);
                self.validate_field_assignment(&class_name, field_token, &value_type);
            }
        }
    }

    fn check_local_function(&mut self, local_fn: &ast::LocalFunction) {
        let line = local_fn.local_token().token().start_position().line();
        let mut annotations = self.take_line_annotations(line);
        let (mut param_annotations, return_annotations) =
            TypeChecker::extract_function_annotations(&mut annotations);

        if let Some(name) = token_identifier(local_fn.name()) {
            let inferred = TypeKind::Function;
            let ty = self.apply_type_annotation(&name, local_fn.name(), inferred, &mut annotations);
            self.assign_local(&name, local_fn.name(), ty);
        }

        let enforce_returns = !return_annotations.is_empty();
        if enforce_returns {
            self.return_expectations.push(return_annotations);
        }
        self.with_new_scope(|checker| {
            checker.bind_function_parameters(local_fn.body(), &mut param_annotations);
            checker.check_block(local_fn.body().block());
        });
        if enforce_returns {
            self.return_expectations.pop();
        }
    }

    fn check_function_declaration(&mut self, function: &ast::FunctionDeclaration) {
        let line = function.function_token().token().start_position().line();
        let mut annotations = self.take_line_annotations(line);
        let (mut param_annotations, return_annotations) =
            TypeChecker::extract_function_annotations(&mut annotations);

        if let Some(token) = target_function_name(function.name())
            && let Some(name) = token_identifier(token)
        {
            let inferred = TypeKind::Function;
            let ty = self.apply_type_annotation(&name, token, inferred, &mut annotations);
            self.assign_nonlocal(&name, token, ty);
        }

        let enforce_returns = !return_annotations.is_empty();
        if enforce_returns {
            self.return_expectations.push(return_annotations);
        }
        self.with_new_scope(|checker| {
            checker.bind_function_parameters(function.body(), &mut param_annotations);
            checker.check_block(function.body().block());
        });
        if enforce_returns {
            self.return_expectations.pop();
        }
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
            if let ast::Parameter::Name(token) = pair.value()
                && let Some(name) = token_identifier(token)
            {
                let mut ty = TypeKind::Unknown;
                if let Some(annotation) = param_annotations.remove(&name)
                    && let Some(expected) = self.resolve_annotation_kind(&annotation)
                {
                    ty = expected;
                }
                self.assign_local(&name, token, ty);
            }
        }
    }

    fn assign_local(&mut self, name: &str, token: &TokenReference, ty: TypeKind) {
        self.emit_reassignment(name, token, &ty);
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_owned(), ty);
        }
    }

    fn assign_nonlocal(&mut self, name: &str, token: &TokenReference, ty: TypeKind) {
        self.emit_reassignment(name, token, &ty);

        if let Some(index) = self.lookup_scope_index(name)
            && let Some(scope) = self.scopes.get_mut(index)
        {
            scope.insert(name.to_owned(), ty);
            return;
        }

        if let Some(global) = self.scopes.first_mut() {
            global.insert(name.to_owned(), ty);
        }
    }

    fn emit_reassignment(&mut self, name: &str, token: &TokenReference, ty: &TypeKind) {
        if ty == &TypeKind::Unknown {
            return;
        }

        if let Some(existing) = self.lookup(name)
            && existing != TypeKind::Unknown
            && existing != *ty
        {
            let message = format!(
                "variable '{name}' was previously inferred as type {existing} but is now assigned type {ty}"
            );
            self.push_diagnostic(token, message);
        }
    }

    fn lookup(&self, name: &str) -> Option<TypeKind> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
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

fn extract_field_assignment(var: &ast::Var) -> Option<(&TokenReference, &TokenReference)> {
    if let ast::Var::Expression(expression) = var
        && let ast::Prefix::Name(base) = expression.prefix()
        && let Some(ast::Suffix::Index(ast::Index::Dot { name, .. })) = expression.suffixes().next()
    {
        return Some((base, name));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::path::Path;

    fn run_type_check(source: &str) -> Vec<Diagnostic> {
        let ast = full_moon::parse(source).expect("failed to parse test source");
        type_check_ast(Path::new("test.lua"), source, &ast)
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

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn reports_mismatch_with_type_annotation() {
        let diagnostics = run_type_check(
            r#"
            ---@type string
            local title = 10
            "#,
        );

        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
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

    #[test]
    fn class_field_annotations_cover_builtin_types() {
        let diagnostics = run_type_check(
            r#"
            ---@class Data
            ---@field nothing nil
            ---@field anything any
            ---@field flag boolean
            ---@field name string
            ---@field size integer
            ---@field callback function
            ---@field bucket table
            ---@field co thread

            ---@type Data
            local data = {}
            data.nothing = nil
            data.anything = 1
            data.flag = true
            data.name = "alice"
            data.size = 1
            data.callback = function() end
            data.bucket = {}
            data.co = coroutine.create(function() end)
            "#,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn exact_class_rejects_unknown_fields() {
        let diagnostics = run_type_check(
            r#"
            ---@class (exact) Point
            ---@field x number
            ---@field y number

            ---@type Point
            local Point = {}
            Point.x = 1
            Point.y = 2
            Point.z = 3
            "#,
        );

        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert!(diagnostic.message.contains("Point"));
        assert!(diagnostic.message.contains("field 'z'"));
    }

    #[test]
    fn class_inheritance_allows_parent_fields() {
        let diagnostics = run_type_check(
            r#"
            ---@class Vehicle
            ---@field speed number
            local Vehicle = {}

            ---@class Plane: Vehicle
            ---@field altitude number

            ---@type Plane
            local plane = {}
            plane.speed = 100
            plane.altitude = 1000
            "#,
        );

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn return_annotation_detects_mismatch() {
        let diagnostics = run_type_check(
            r#"
            ---@return number
            local function value()
                return "oops"
            end
            "#,
        );

        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert!(diagnostic.message.contains("return value #1"));
    }

    #[test]
    fn return_annotation_accepts_correct_type() {
        let diagnostics = run_type_check(
            r#"
            ---@return number
            local function value()
                return 42
            end
            "#,
        );

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn class_annotation_maps_to_table() {
        let diagnostics = run_type_check(
            r#"
            ---@class Person
            ---@type Person
            local person = {}
            "#,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn enum_annotation_treated_as_string() {
        let diagnostics = run_type_check(
            r#"
            ---@enum Mode
            ---@field Immediate '"immediate"'
            ---@field Deferred '"deferred"'

            ---@param mode Mode
            local function set_mode(mode)
                mode = "immediate"
            end
            "#,
        );

        assert!(diagnostics.is_empty());
    }
}
