use std::collections::VecDeque;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use full_moon::{
    self, Error as FullMoonError, ast,
    node::Node,
    tokenizer::{Position, Symbol, TokenReference, TokenType},
};

// use crate::typing::{infer::expr as infer_expr, types as tty};
use crate::{
    cli::CheckOptions,
    diagnostics::{Diagnostic, DiagnosticCode, TextPosition, TextRange},
    error::{Result, TypuaError},
    lsp::DocumentPosition,
    workspace,
};

use super::types::{
    AnnotatedType, Annotation, AnnotationIndex, AnnotationUsage, OperandSide, TypeKind,
    TypeRegistry,
};

pub use super::types::{CheckReport, CheckResult, TypeInfo};

pub fn run(options: &CheckOptions) -> Result<CheckReport> {
    let files = workspace::collect_source_files(&options.target, &options.config)?;

    let mut sources = Vec::new();
    for path in &files {
        let source = read_source(path)?;
        sources.push((path.clone(), source));
    }

    let mut workspace_registry = TypeRegistry::default();
    for (_, source) in &sources {
        let (_, registry) = AnnotationIndex::from_source(source);
        workspace_registry.extend(&registry);
    }

    let mut report = CheckReport {
        files_checked: files.len(),
        diagnostics: Vec::new(),
    };

    for (path, source) in sources {
        match full_moon::parse(&source) {
            Ok(ast) => {
                let mut result = check_ast_with_registry(
                    path.as_path(),
                    &source,
                    &ast,
                    Some(&workspace_registry),
                );
                report.diagnostics.append(&mut result.diagnostics);
            }
            Err(errors) => {
                for error in errors {
                    report
                        .diagnostics
                        .push(to_syntax_diagnostic(path.as_path(), error));
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
    let code = Some(DiagnosticCode::SyntaxError);
    Diagnostic::error(path.to_path_buf(), error.error_message(), range, code)
}

fn error_range(error: &FullMoonError) -> Option<TextRange> {
    let (start, end) = error.range();
    let start = TextPosition::from(start);
    let end = TextPosition::from(end);
    Some(TextRange { start, end })
}

pub fn check_ast(path: &Path, source: &str, ast: &ast::Ast) -> CheckResult {
    check_ast_with_registry(path, source, ast, None)
}

pub fn check_ast_with_registry(
    path: &Path,
    source: &str,
    ast: &ast::Ast,
    workspace_registry: Option<&TypeRegistry>,
) -> CheckResult {
    let (annotations, local_registry) = AnnotationIndex::from_ast(ast, source);
    let registry = if let Some(global) = workspace_registry {
        let mut combined = global.clone();
        combined.extend(&local_registry);
        combined
    } else {
        local_registry
    };

    // Build TypedAST for future incremental analysis pipeline.
    // 現状の型検査はfull_moon ASTに対して行うが、
    // 仕様に基づきTypedASTを生成しておく（将来的にこちらに切替）。
    let _typed = crate::typechecker::typed_ast::build_typed_ast(source, ast, &annotations);

    TypeChecker::new(path, annotations, registry).check(ast)
}

struct TypeChecker<'a> {
    path: &'a Path,
    diagnostics: Vec<Diagnostic>,
    scopes: Vec<HashMap<String, VariableEntry>>,
    annotations: AnnotationIndex,
    type_registry: TypeRegistry,
    return_expectations: Vec<Vec<AnnotatedType>>,
    type_info: HashMap<DocumentPosition, TypeInfo>,
}

#[derive(Clone)]
struct VariableEntry {
    ty: TypeKind,
    annotated: bool,
}

#[derive(Clone, Default)]
struct ConditionEffect {
    truthy: Vec<NarrowRule>,
    falsy: Vec<NarrowRule>,
}

#[derive(Clone)]
enum NarrowRule {
    RequireNil(String),
    ExcludeNil(String),
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
            type_info: HashMap::new(),
        }
    }

    fn check(mut self, ast: &ast::Ast) -> CheckResult {
        self.scopes.push(HashMap::new());
        self.check_block(ast.nodes());
        self.scopes.pop();
        CheckResult {
            diagnostics: self.diagnostics,
            type_map: self.type_info,
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

    fn with_new_scope<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.scopes.push(HashMap::new());
        f(self);
        self.scopes.pop();
    }

    fn current_scope_snapshot(&self) -> HashMap<String, VariableEntry> {
        self.scopes.last().cloned().unwrap_or_default()
    }

    fn replace_current_scope(&mut self, scope: HashMap<String, VariableEntry>) {
        if let Some(current) = self.scopes.last_mut() {
            *current = scope;
        }
    }

    fn push_scope_with(&mut self, scope: HashMap<String, VariableEntry>) {
        self.scopes.push(scope);
    }

    fn pop_scope_map(&mut self) -> HashMap<String, VariableEntry> {
        self.scopes.pop().unwrap_or_default()
    }

    fn take_line_annotations(&mut self, line: usize) -> Vec<Annotation> {
        self.annotations.take(line)
    }

    fn take_class_hints(&mut self, line: usize) -> Vec<String> {
        self.annotations.take_class_hint(line)
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
    ) -> (TypeKind, bool) {
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
            let annotated = true;
            if let Some(expected) = self.resolve_annotation_kind(&annotation.ty) {
                if !expected.matches(&inferred) {
                    let message = format!(
                        "variable '{name}' is annotated as type {} but inferred type is {}",
                        annotation.ty.raw, inferred
                    );
                    self.push_diagnostic(token, message, Some(DiagnosticCode::AssignTypeMismatch));
                }
                self.record_type(token, expected.clone());
                (expected, annotated)
            } else {
                self.record_type(token, inferred.clone());
                (inferred, annotated)
            }
        } else {
            let annotated = self
                .lookup_entry(name)
                .map(|entry| entry.annotated)
                .unwrap_or(false);
            self.record_type(token, inferred.clone());
            (inferred, annotated)
        }
    }

    fn resolve_annotation_kind(&self, annotation: &AnnotatedType) -> Option<TypeKind> {
        if let Some(resolved) = self.type_registry.resolve(&annotation.raw) {
            return Some(resolved);
        }
        annotation.kind.clone()
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
            self.push_diagnostic(
                ret.token(),
                message,
                Some(DiagnosticCode::ReturnTypeMismatch),
            );
        }

        if actual_len < expected_len {
            let message = format!(
                "function annotated to return {expected_len} value(s) but this return statement provides {actual_len}"
            );
            self.push_diagnostic(
                ret.token(),
                message,
                Some(DiagnosticCode::ReturnTypeMismatch),
            );
        }

        for (idx, annotation) in expectations.iter().enumerate() {
            if idx >= expr_info.len() {
                break;
            }

            let actual = expr_info[idx].0.clone();
            if let Some(expected) = self.resolve_annotation_kind(annotation)
                && !expected.matches(&actual)
            {
                let message = format!(
                    "return value #{} is annotated as type {} but inferred type is {}",
                    idx + 1,
                    annotation.raw,
                    actual
                );
                self.push_diagnostic(
                    ret.token(),
                    message,
                    Some(DiagnosticCode::ReturnTypeMismatch),
                );
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
            if let Some(expected) = self.resolve_annotation_kind(annotation) {
                let expected_clone = expected.clone();
                let annotation_message = annotation.raw.clone();
                self.record_type(field_token, expected_clone);
                if !expected.matches(value_type) {
                    let message = format!(
                        "field '{field_name}' in class {class_name} expects type {} but inferred type is {}",
                        annotation_message, value_type
                    );
                    self.push_diagnostic(
                        field_token,
                        message,
                        Some(DiagnosticCode::ParamTypeMismatch),
                    );
                }
                return;
            }
        } else if self.type_registry.is_exact(class_name) {
            let message = format!(
                "class {class_name} is declared exact; field '{field_name}' is not defined"
            );
            self.push_diagnostic(field_token, message, Some(DiagnosticCode::UndefinedField));
            return;
        }

        self.record_type(field_token, value_type.clone());
    }

    fn check_local_assignment(&mut self, assignment: &ast::LocalAssignment) {
        let line = assignment.local_token().token().start_position().line();
        let mut annotations = self.take_line_annotations(line);
        let mut class_hints: VecDeque<String> = VecDeque::from(self.take_class_hints(line));

        let expr_types: Vec<TypeKind> = assignment
            .expressions()
            .pairs()
            .map(|pair| self.infer_expression(pair.value()))
            .collect();

        for (index, pair) in assignment.names().pairs().enumerate() {
            let token = pair.value();
            if let Some(name) = token_identifier(token) {
                let inferred = expr_types.get(index).cloned().unwrap_or(TypeKind::Nil);
                let is_table_literal = matches!(inferred, TypeKind::Table);
                let before_len = annotations.len();
                let (mut ty, mut annotated) =
                    self.apply_type_annotation(&name, token, inferred, &mut annotations);
                let used_annotation = before_len != annotations.len();

                if !used_annotation
                    && is_table_literal
                    && let Some(class_name) = class_hints.pop_front()
                {
                    ty = TypeKind::Custom(class_name);
                    annotated = true;
                }

                self.assign_local(&name, token, ty, annotated);
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
        let mut class_hints = if line > 0 {
            VecDeque::from(self.take_class_hints(line))
        } else {
            VecDeque::new()
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
                    let is_table_literal = matches!(inferred, TypeKind::Table);
                    let before_len = annotations.len();
                    let (mut ty, mut annotated) =
                        self.apply_type_annotation(&name, token, inferred, &mut annotations);
                    let used_annotation = before_len != annotations.len();

                    if !used_annotation
                        && is_table_literal
                        && let Some(class_name) = class_hints.pop_front()
                    {
                        ty = TypeKind::Custom(class_name);
                        annotated = true;
                    }

                    self.assign_nonlocal(&name, token, ty, annotated);
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

        let name_token = local_fn.name();
        if let Some(name) = token_identifier(name_token) {
            let inferred = TypeKind::Function;
            let (ty, annotated) =
                self.apply_type_annotation(&name, name_token, inferred, &mut annotations);
            self.assign_local(&name, name_token, ty, annotated);
            self.clear_type_info(name_token);
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
            let (ty, annotated) =
                self.apply_type_annotation(&name, token, inferred, &mut annotations);
            self.assign_nonlocal(&name, token, ty, annotated);
            self.clear_type_info(token);
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
        let base_scope = self.current_scope_snapshot();
        let mut branch_scopes: Vec<HashMap<String, VariableEntry>> = Vec::new();
        let mut remaining_env = base_scope.clone();

        let mut branches: Vec<(Option<&ast::Expression>, &ast::Block)> = Vec::new();
        branches.push((Some(if_stmt.condition()), if_stmt.block()));
        if let Some(elseifs) = if_stmt.else_if() {
            for elseif in elseifs {
                branches.push((Some(elseif.condition()), elseif.block()));
            }
        }
        if let Some(block) = if_stmt.else_block() {
            branches.push((None, block));
        }

        for (condition, block) in branches {
            let mut branch_env = remaining_env.clone();
            if let Some(expr) = condition {
                self.infer_expression(expr);
                let effect = Self::analyze_condition(expr);
                Self::apply_narrowing(&mut branch_env, &effect.truthy);

                let mut next_env = remaining_env.clone();
                Self::apply_narrowing(&mut next_env, &effect.falsy);
                remaining_env = next_env;
            }

            self.push_scope_with(branch_env);
            self.check_block(block);
            let scope_result = self.pop_scope_map();
            branch_scopes.push(scope_result);
        }

        if if_stmt.else_block().is_none() {
            branch_scopes.push(remaining_env);
        }

        let merged = Self::merge_branch_scopes(&base_scope, branch_scopes);
        self.replace_current_scope(merged);
    }

    fn check_while(&mut self, while_stmt: &ast::While) {
        self.infer_expression(while_stmt.condition());
        let base_scope = self.current_scope_snapshot();
        let effect = Self::analyze_condition(while_stmt.condition());
        let mut loop_env = base_scope.clone();
        Self::apply_narrowing(&mut loop_env, &effect.truthy);

        self.push_scope_with(loop_env);
        self.check_block(while_stmt.block());
        let loop_scope = self.pop_scope_map();

        let merged = Self::merge_branch_scopes(&base_scope, vec![loop_scope, base_scope.clone()]);
        self.replace_current_scope(merged);
    }

    fn check_repeat(&mut self, repeat_stmt: &ast::Repeat) {
        let base_scope = self.current_scope_snapshot();
        self.push_scope_with(base_scope.clone());
        self.check_block(repeat_stmt.block());
        let body_scope = self.pop_scope_map();
        self.infer_expression(repeat_stmt.until());

        let merged = Self::merge_branch_scopes(&base_scope, vec![body_scope, base_scope.clone()]);
        self.replace_current_scope(merged);
    }

    fn check_numeric_for(&mut self, numeric_for: &ast::NumericFor) {
        self.infer_expression(numeric_for.start());
        self.infer_expression(numeric_for.end());
        if let Some(step) = numeric_for.step() {
            self.infer_expression(step);
        }

        self.with_new_scope(|checker| {
            if let Some(name) = token_identifier(numeric_for.index_variable()) {
                checker.assign_local(&name, numeric_for.index_variable(), TypeKind::Number, false);
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
                    checker.assign_local(&name, pair.value(), TypeKind::Unknown, false);
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
                let mut annotated_param = false;
                if let Some(annotation) = param_annotations.remove(&name) {
                    annotated_param = true;
                    if let Some(expected) = self.resolve_annotation_kind(&annotation) {
                        ty = expected;
                    }
                }
                self.assign_local(&name, token, ty, annotated_param);
            }
        }
    }

    fn analyze_condition(expr: &ast::Expression) -> ConditionEffect {
        match expr {
            ast::Expression::Var(_) => {
                if let Some(name) = expression_identifier(expr) {
                    let mut effect = ConditionEffect::default();
                    effect.truthy.push(NarrowRule::ExcludeNil(name.clone()));
                    effect.falsy.push(NarrowRule::RequireNil(name));
                    effect
                } else {
                    ConditionEffect::default()
                }
            }
            ast::Expression::UnaryOperator { unop, expression } => {
                if matches!(unop, ast::UnOp::Not(_)) {
                    let inner = Self::analyze_condition(expression);
                    ConditionEffect {
                        truthy: inner.falsy,
                        falsy: inner.truthy,
                    }
                } else {
                    ConditionEffect::default()
                }
            }
            ast::Expression::BinaryOperator { lhs, binop, rhs } => {
                let symbol = match binop.token().token_type() {
                    TokenType::Symbol { symbol } => symbol,
                    _ => return ConditionEffect::default(),
                };
                match symbol {
                    Symbol::TwoEqual => Self::analyze_equality(lhs, rhs, true),
                    Symbol::TildeEqual => Self::analyze_equality(lhs, rhs, false),
                    _ => ConditionEffect::default(),
                }
            }
            ast::Expression::Parentheses { expression, .. } => Self::analyze_condition(expression),
            _ => ConditionEffect::default(),
        }
    }

    fn analyze_equality(
        lhs: &ast::Expression,
        rhs: &ast::Expression,
        is_equal: bool,
    ) -> ConditionEffect {
        if expression_is_nil(rhs)
            && let Some(name) = expression_identifier(lhs)
        {
            return Self::build_nil_comparison(name, is_equal);
        }

        if expression_is_nil(lhs)
            && let Some(name) = expression_identifier(rhs)
        {
            return Self::build_nil_comparison(name, is_equal);
        }

        ConditionEffect::default()
    }

    fn build_nil_comparison(name: String, is_equal: bool) -> ConditionEffect {
        let mut effect = ConditionEffect::default();
        if is_equal {
            effect.truthy.push(NarrowRule::RequireNil(name.clone()));
            effect.falsy.push(NarrowRule::ExcludeNil(name));
        } else {
            effect.truthy.push(NarrowRule::ExcludeNil(name.clone()));
            effect.falsy.push(NarrowRule::RequireNil(name));
        }
        effect
    }

    fn apply_narrowing(scope: &mut HashMap<String, VariableEntry>, rules: &[NarrowRule]) {
        for rule in rules {
            match rule {
                NarrowRule::RequireNil(name) => {
                    if let Some(entry) = scope.get_mut(name) {
                        entry.ty = type_only_nil(&entry.ty);
                    }
                }
                NarrowRule::ExcludeNil(name) => {
                    if let Some(entry) = scope.get_mut(name) {
                        entry.ty = type_without_nil(&entry.ty);
                    }
                }
            }
        }
    }

    fn merge_branch_scopes(
        base: &HashMap<String, VariableEntry>,
        branches: Vec<HashMap<String, VariableEntry>>,
    ) -> HashMap<String, VariableEntry> {
        let mut merged = base.clone();
        for key in base.keys() {
            let mut ty: Option<TypeKind> = None;
            let mut annotated = base.get(key).map(|entry| entry.annotated).unwrap_or(false);

            for branch in &branches {
                if let Some(entry) = branch.get(key) {
                    annotated |= entry.annotated;
                    ty = Some(match ty {
                        None => entry.ty.clone(),
                        Some(ref acc) => union_type(acc, &entry.ty),
                    });
                }
            }

            if let Some(entry) = merged.get_mut(key) {
                if let Some(new_ty) = ty {
                    entry.ty = new_ty;
                }
                entry.annotated |= annotated;
            }
        }
        merged
    }

    fn assign_local(&mut self, name: &str, token: &TokenReference, ty: TypeKind, annotated: bool) {
        let prev_annotated = self
            .lookup_entry(name)
            .map(|entry| entry.annotated)
            .unwrap_or(false);
        let merged_annotated = prev_annotated || annotated;
        self.emit_reassignment(name, token, &ty, merged_annotated);
        self.record_type(token, ty.clone());
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(
                name.to_owned(),
                VariableEntry {
                    ty,
                    annotated: merged_annotated,
                },
            );
        }
    }

    fn assign_nonlocal(
        &mut self,
        name: &str,
        token: &TokenReference,
        ty: TypeKind,
        annotated: bool,
    ) {
        let prev_annotated = self
            .lookup_entry(name)
            .map(|entry| entry.annotated)
            .unwrap_or(false);
        let merged_annotated = prev_annotated || annotated;
        self.emit_reassignment(name, token, &ty, merged_annotated);
        self.record_type(token, ty.clone());

        if let Some(index) = self.lookup_scope_index(name)
            && let Some(scope) = self.scopes.get_mut(index)
        {
            scope.insert(
                name.to_owned(),
                VariableEntry {
                    ty,
                    annotated: merged_annotated,
                },
            );
            return;
        }

        if let Some(global) = self.scopes.first_mut() {
            global.insert(
                name.to_owned(),
                VariableEntry {
                    ty,
                    annotated: merged_annotated,
                },
            );
        }
    }

    fn emit_reassignment(
        &mut self,
        name: &str,
        token: &TokenReference,
        ty: &TypeKind,
        annotated: bool,
    ) {
        if ty == &TypeKind::Unknown {
            return;
        }

        if let Some(existing) = self.lookup_entry(name)
            && (existing.annotated || annotated)
            && !existing.ty.matches(ty)
        {
            let message = format!(
                "variable '{name}' was previously inferred as type {} but is now assigned type {ty}",
                existing.ty
            );
            self.push_diagnostic(token, message, Some(DiagnosticCode::AssignTypeMismatch));
        }
    }

    fn record_type(&mut self, token: &TokenReference, ty: TypeKind) {
        if matches!(ty, TypeKind::Unknown) {
            return;
        }

        let start = token.token().start_position();
        let end = token.token().end_position();
        self.type_info.insert(
            DocumentPosition {
                row: start.line(),
                col: start.character(),
            },
            TypeInfo {
                ty: ty.to_string(),
                end_line: end.line(),
                end_character: end.character(),
            },
        );
    }

    fn clear_type_info(&mut self, token: &TokenReference) {
        let start = token.token().start_position();
        self.type_info.remove(&DocumentPosition {
            row: start.line(),
            col: start.character(),
        });
    }

    fn lookup(&self, name: &str) -> Option<TypeKind> {
        self.lookup_entry(name).map(|entry| entry.ty.clone())
    }

    fn lookup_entry(&self, name: &str) -> Option<&VariableEntry> {
        for scope in self.scopes.iter().rev() {
            if let Some(entry) = scope.get(name) {
                return Some(entry);
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
            ast::Expression::FunctionCall(_call) => {
                // self.try_record_function_call_type(call);
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
        if actual == TypeKind::Unknown || expected.matches(&actual) {
            return;
        }

        let message = format!(
            "operator '{}' expected {} operand of type {}, but found {}",
            operator_label(token),
            side.describe(),
            expected,
            actual
        );
        self.push_diagnostic(token, message, Some(DiagnosticCode::AssignTypeMismatch));
    }

    fn push_diagnostic(
        &mut self,
        token: &TokenReference,
        message: String,
        code: Option<DiagnosticCode>,
    ) {
        let range = self.range_from_token(token);
        self.diagnostics.push(Diagnostic::error(
            self.path_buf(),
            message,
            Some(range),
            code,
        ));
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

fn expression_identifier(expr: &ast::Expression) -> Option<String> {
    match expr {
        ast::Expression::Var(ast::Var::Name(token)) => token_identifier(token),
        ast::Expression::Parentheses { expression, .. } => expression_identifier(expression),
        _ => None,
    }
}

fn expression_is_nil(expr: &ast::Expression) -> bool {
    match expr {
        ast::Expression::Symbol(token) => matches!(
            token.token().token_type(),
            TokenType::Symbol {
                symbol: Symbol::Nil
            }
        ),
        ast::Expression::Parentheses { expression, .. } => expression_is_nil(expression),
        _ => false,
    }
}

fn type_only_nil(ty: &TypeKind) -> TypeKind {
    if contains_nil(ty) {
        TypeKind::Nil
    } else {
        TypeKind::Unknown
    }
}

fn type_without_nil(ty: &TypeKind) -> TypeKind {
    match ty {
        TypeKind::Nil => TypeKind::Unknown,
        TypeKind::Union(items) => {
            let mut kept = Vec::new();
            for item in items {
                if matches!(item, TypeKind::Nil) {
                    continue;
                }
                let filtered = type_without_nil(item);
                if !matches!(filtered, TypeKind::Unknown) {
                    flatten_union(&filtered, &mut kept);
                }
            }
            build_union(kept)
        }
        _ => ty.clone(),
    }
}

fn union_type(a: &TypeKind, b: &TypeKind) -> TypeKind {
    if matches!(a, TypeKind::Unknown) || matches!(b, TypeKind::Unknown) {
        return TypeKind::Unknown;
    }
    if a == b {
        return a.clone();
    }
    let mut items = Vec::new();
    flatten_union(a, &mut items);
    flatten_union(b, &mut items);
    build_union(items)
}

fn contains_nil(ty: &TypeKind) -> bool {
    match ty {
        TypeKind::Nil => true,
        TypeKind::Union(items) => items.iter().any(contains_nil),
        _ => false,
    }
}

fn flatten_union(ty: &TypeKind, out: &mut Vec<TypeKind>) {
    match ty {
        TypeKind::Union(items) => {
            for item in items {
                flatten_union(item, out);
            }
        }
        other => {
            if !out.iter().any(|existing| existing == other) {
                out.push(other.clone());
            }
        }
    }
}

fn build_union(mut items: Vec<TypeKind>) -> TypeKind {
    if items.is_empty() {
        TypeKind::Unknown
    } else if items.len() == 1 {
        items.pop().unwrap()
    } else {
        TypeKind::Union(items)
    }
}

#[cfg(test)]
mod tests {
    use super::super::annotation::{make_union, parse_annotation};
    use super::super::types::{AnnotationIndex, TypeRegistry};
    use super::*;
    use crate::diagnostics::Severity;
    use pretty_assertions::assert_eq;
    use std::path::Path;
    use unindent::Unindent;

    fn run_type_check(source: &str) -> CheckResult {
        let ast = full_moon::parse(source).expect("failed to parse test source");
        check_ast(Path::new("test.lua"), source, &ast)
    }
    #[test]
    fn annotation_type() {
        // normal single type
        assert_eq!(
            parse_annotation("---@type number").unwrap(),
            Annotation {
                usage: AnnotationUsage::Type,
                name: None,
                ty: AnnotatedType {
                    raw: "number".to_string(),
                    kind: Some(TypeKind::Number)
                }
            }
        );
        // normal: optional
        assert_eq!(
            parse_annotation("---@type number?").unwrap(),
            Annotation {
                usage: AnnotationUsage::Type,
                name: None,
                ty: AnnotatedType {
                    raw: "number?".to_string(),
                    kind: Some(make_union(vec![TypeKind::Number, TypeKind::Nil]))
                }
            }
        );
        // normal: union
        assert_eq!(
            parse_annotation("---@type number | string").unwrap(),
            Annotation {
                usage: AnnotationUsage::Type,
                name: None,
                ty: AnnotatedType {
                    raw: "number | string".to_string(),
                    kind: Some(make_union(vec![TypeKind::Number, TypeKind::String]))
                }
            }
        );
        // normal: array
        assert_eq!(
            parse_annotation("---@type number[]").unwrap(),
            Annotation {
                usage: AnnotationUsage::Type,
                name: None,
                ty: AnnotatedType {
                    raw: "number[]".to_string(),
                    kind: Some(TypeKind::Array(Box::new(TypeKind::Number))),
                }
            }
        );
    }
    #[test]
    fn local_assignment_non_annotated() {
        let result = run_type_check(
            r##"
            local x = 1
            x = "oops"
            "##
            .unindent()
            .as_str(),
        );
        let actual = result
            .type_map
            .get(&DocumentPosition { row: 1, col: 7 })
            .unwrap();
        assert_eq!(
            actual,
            &TypeInfo {
                ty: "number".to_string(),
                end_line: 1,
                end_character: 8
            }
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn local_assignment_annotated() {
        let result = run_type_check(
            r##"
            ---@type number
            local x = 1
            x = "oops"
            "##
            .unindent()
            .as_str(),
        );
        let actual = result
            .type_map
            .get(&DocumentPosition { row: 2, col: 7 })
            .unwrap();
        assert_eq!(
            actual,
            &TypeInfo {
                ty: "number".to_string(),
                end_line: 2,
                end_character: 8
            }
        );
        assert_eq!(result.diagnostics.len(), 1);
        let diagnostic = &result.diagnostics[0];
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(
            diagnostic.code.clone().unwrap(),
            DiagnosticCode::AssignTypeMismatch
        );
    }

    #[test]
    fn reports_variable_reassignment_type_conflict() {
        let result = run_type_check(
            r#"
            local x = 1
            x = "oops"
            "#,
        );

        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn reports_arithmetic_operand_type_mismatch() {
        let result = run_type_check(
            r#"
            local a = "hello"
            local b = a + 1
            "#,
        );

        let diagnostic = &result.diagnostics[0];
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(
            diagnostic
                .message
                .contains("operator '+' expected left operand of type number")
        );
    }

    #[test]
    fn allows_consistent_numeric_assignments() {
        let result = run_type_check(
            r#"
            local value = 1
            value = value + 2
            "#,
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn narrowing_excludes_nil_in_truthy_branch() {
        let source = r#"
            ---@type number|nil
            local value = nil
            value = 1
            if value ~= nil then
            value = value
            end
        "#
        .unindent();

        let result = run_type_check(source.as_str());
        assert!(result.diagnostics.is_empty());

        let position = DocumentPosition { row: 5, col: 1 };
        let info = result
            .type_map
            .get(&position)
            .expect("type info for narrowed assignment");
        assert_eq!(info.ty, "number");
    }

    #[test]
    fn mismatch_type_annotation() {
        let result = run_type_check(
            r#"
            ---@type string
            local title = 10
            "#,
        );
        assert_eq!(result.diagnostics.len(), 1);
        let diagnostic = &result.diagnostics[0];
        assert!(diagnostic.message.contains("annotated as type string"));
    }

    #[test]
    fn param_annotation_enforces_type_in_body() {
        let result = run_type_check(
            r#"
            ---@param amount number
            local function charge(amount)
                amount = "free"
            end
            "#,
        );

        assert_eq!(result.diagnostics.len(), 1);
        let diagnostic = &result.diagnostics[0];
        assert!(
            diagnostic
                .message
                .contains("variable 'amount' was previously inferred as type number")
        );
    }

    #[test]
    fn class_field_annotations_cover_builtin_types() {
        let result = run_type_check(
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
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn exact_class_rejects_unknown_fields() {
        let result = run_type_check(
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

        assert_eq!(result.diagnostics.len(), 1);
        let diagnostic = &result.diagnostics[0];
        assert!(diagnostic.message.contains("Point"));
        assert!(diagnostic.message.contains("field 'z'"));
    }

    #[test]
    fn class_inheritance_allows_parent_fields() {
        let result = run_type_check(
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

        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn resolves_type_annotation_from_other_file() {
        let a_source = r##"
            ---@class Point
            ---@field x number
            ---@field y number
        "##
        .unindent();
        let (_, registry_a) = AnnotationIndex::from_source(a_source.as_str());

        let mut workspace_registry = TypeRegistry::default();
        workspace_registry.extend(&registry_a);

        let b_source = r##"
            ---@type Point
            local p = {}
        "##
        .unindent();
        let ast = full_moon::parse(b_source.as_str()).expect("failed to parse reference source");
        let result = check_ast_with_registry(
            Path::new("b.lua"),
            b_source.as_str(),
            &ast,
            Some(&workspace_registry),
        );

        let position = DocumentPosition { row: 2, col: 7 };
        let info = result
            .type_map
            .get(&position)
            .expect("missing type info for cross-file annotation");
        assert_eq!(info.ty, "Point");
    }

    #[test]
    fn return_annotation_detects_mismatch() {
        let result = run_type_check(
            r#"
            ---@return number
            local function value()
                return "oops"
            end
            "#,
        );

        assert_eq!(result.diagnostics.len(), 1);
        let diagnostic = &result.diagnostics[0];
        assert!(diagnostic.message.contains("return value #1"));
    }

    #[test]
    fn return_annotation_accepts_correct_type() {
        let result = run_type_check(
            r#"
            ---@return number
            local function value()
                return 42
            end
            "#,
        );

        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn class_annotation_maps_to_table() {
        let result = run_type_check(
            r#"
            ---@class Person
            ---@type Person
            local person = {}
            "#,
        );

        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn class_annotation_infers_type_for_following_local_assignment() {
        let result = run_type_check(
            r##"
            ---@class Container
            local C = {}
            "##
            .unindent()
            .as_str(),
        );

        assert!(result.diagnostics.is_empty());
        let info = result
            .type_map
            .get(&DocumentPosition { row: 2, col: 7 })
            .expect("missing type info for local assignment");
        assert_eq!(info.ty, "Container");
    }

    #[test]
    fn class_annotation_infers_type_for_following_assignment() {
        let result = run_type_check(
            r##"
            ---@class Container
            Container = {}
            "##
            .unindent()
            .as_str(),
        );

        assert!(result.diagnostics.is_empty());
        let info = result
            .type_map
            .get(&DocumentPosition { row: 2, col: 1 })
            .expect("missing type info for assignment");
        assert_eq!(info.ty, "Container");
    }

    #[test]
    fn enum_annotation_treated_as_string() {
        let result = run_type_check(
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
        assert!(result.diagnostics.is_empty());
    }
}
