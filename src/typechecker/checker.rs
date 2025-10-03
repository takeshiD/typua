use std::collections::VecDeque;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use full_moon::{self, Error as FullMoonError, ast};

// use crate::typing::{infer::expr as infer_expr, types as tty};
use crate::{
    cli::CheckOptions,
    diagnostics::{Diagnostic, DiagnosticCode, TextPosition, TextRange},
    error::{Result, TypuaError},
    lsp::DocumentPosition,
    workspace,
};

use super::typed_ast;
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

    let typed = crate::typechecker::typed_ast::build_typed_ast(source, ast, &annotations);

    TypeChecker::new(path, registry).check_program(&typed)
}

struct TypeChecker<'a> {
    path: &'a Path,
    diagnostics: Vec<Diagnostic>,
    scopes: Vec<HashMap<String, VariableEntry>>,
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
    RequireType(String, TypeKind),
    ExcludeType(String, TypeKind),
}

impl<'a> TypeChecker<'a> {
    fn new(path: &'a Path, type_registry: TypeRegistry) -> Self {
        Self {
            path,
            diagnostics: Vec::new(),
            scopes: Vec::new(),
            type_registry,
            return_expectations: Vec::new(),
            type_info: HashMap::new(),
        }
    }

    fn check_program(mut self, program: &typed_ast::Program) -> CheckResult {
        self.scopes.push(HashMap::new());
        self.check_block(&program.block);
        self.scopes.pop();
        CheckResult {
            diagnostics: self.diagnostics,
            type_map: self.type_info,
        }
    }

    fn check_block(&mut self, block: &typed_ast::Block) {
        for stmt in &block.stmts {
            self.check_stmt(stmt);
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

    fn apply_type_annotation(
        &mut self,
        identifier: &typed_ast::Identifier,
        inferred: TypeKind,
        annotations: &mut Vec<Annotation>,
    ) -> (TypeKind, bool) {
        let name = identifier.name.as_str();
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
                    self.push_diagnostic(
                        Some(identifier.range),
                        message,
                        Some(DiagnosticCode::AssignTypeMismatch),
                    );
                }
                self.record_type(identifier.range, expected.clone());
                (expected, annotated)
            } else {
                self.record_type(identifier.range, inferred.clone());
                (inferred, annotated)
            }
        } else {
            let annotated = self
                .lookup_entry(name)
                .map(|entry| entry.annotated)
                .unwrap_or(false);
            self.record_type(identifier.range, inferred.clone());
            (inferred, annotated)
        }
    }

    fn resolve_annotation_kind(&self, annotation: &AnnotatedType) -> Option<TypeKind> {
        if let Some(resolved) = self.type_registry.resolve(&annotation.raw) {
            return Some(resolved);
        }
        annotation.kind.clone()
    }

    fn check_stmt(&mut self, stmt: &typed_ast::Stmt) {
        match stmt {
            typed_ast::Stmt::LocalAssign(local) => self.check_local_assignment(local),
            typed_ast::Stmt::Assign(assign) => self.check_assignment(assign),
            typed_ast::Stmt::LocalFunction(local_fn) => self.check_local_function(local_fn),
            typed_ast::Stmt::Function(function) => self.check_function_declaration(function),
            typed_ast::Stmt::Do(do_block) => {
                self.with_new_scope(|checker| checker.check_block(&do_block.block))
            }
            typed_ast::Stmt::If(if_stmt) => self.check_if(if_stmt),
            typed_ast::Stmt::While(while_stmt) => self.check_while(while_stmt),
            typed_ast::Stmt::Repeat(repeat_stmt) => self.check_repeat(repeat_stmt),
            typed_ast::Stmt::NumericFor(numeric_for) => self.check_numeric_for(numeric_for),
            typed_ast::Stmt::GenericFor(generic_for) => self.check_generic_for(generic_for),
            typed_ast::Stmt::Return(ret) => self.validate_return(ret),
            typed_ast::Stmt::FunctionCall(_)
            | typed_ast::Stmt::Label(_)
            | typed_ast::Stmt::Goto(_)
            | typed_ast::Stmt::Break(_)
            | typed_ast::Stmt::Unknown(_) => {}
        }
    }

    fn validate_return(&mut self, ret: &typed_ast::ReturnStmt) {
        let expr_info: Vec<TypeKind> = ret
            .values
            .iter()
            .map(|expr| self.infer_expression(expr))
            .collect();

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
                Some(ret.range),
                message,
                Some(DiagnosticCode::ReturnTypeMismatch),
            );
        }

        if actual_len < expected_len {
            let message = format!(
                "function annotated to return {expected_len} value(s) but this return statement provides {actual_len}"
            );
            self.push_diagnostic(
                Some(ret.range),
                message,
                Some(DiagnosticCode::ReturnTypeMismatch),
            );
        }

        for (idx, annotation) in expectations.iter().enumerate() {
            if idx >= expr_info.len() {
                break;
            }

            let actual = expr_info[idx].clone();
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
                    Some(ret.range),
                    message,
                    Some(DiagnosticCode::ReturnTypeMismatch),
                );
            }
        }
    }

    fn validate_field_assignment(
        &mut self,
        class_name: &str,
        field: &typed_ast::Identifier,
        value_type: &TypeKind,
    ) {
        if let Some(annotation) = self.type_registry.field_annotation(class_name, &field.name) {
            if let Some(expected) = self.resolve_annotation_kind(annotation) {
                let expected_clone = expected.clone();
                let annotation_message = annotation.raw.clone();
                self.record_type(field.range, expected_clone);
                if !expected.matches(value_type) {
                    let message = format!(
                        "field '{}' in class {class_name} expects type {} but inferred type is {}",
                        field.name, annotation_message, value_type
                    );
                    self.push_diagnostic(
                        Some(field.range),
                        message,
                        Some(DiagnosticCode::ParamTypeMismatch),
                    );
                }
                return;
            }
        } else if self.type_registry.is_exact(class_name) {
            let message = format!(
                "class {class_name} is declared exact; field '{}' is not defined",
                field.name
            );
            self.push_diagnostic(
                Some(field.range),
                message,
                Some(DiagnosticCode::UndefinedField),
            );
            return;
        }

        self.record_type(field.range, value_type.clone());
    }

    fn check_local_assignment(&mut self, assignment: &typed_ast::LocalAssign) {
        let mut annotations = assignment.annotations.clone();
        let mut class_hints: VecDeque<String> = VecDeque::from(assignment.class_hints.clone());

        let expr_types: Vec<TypeKind> = assignment
            .values
            .iter()
            .map(|expr| self.infer_expression(expr))
            .collect();

        for (index, identifier) in assignment.names.iter().enumerate() {
            let inferred = expr_types.get(index).cloned().unwrap_or(TypeKind::Nil);
            let is_table_literal = matches!(inferred, TypeKind::Table);
            let before_len = annotations.len();
            let (mut ty, mut annotated) =
                self.apply_type_annotation(identifier, inferred, &mut annotations);
            let used_annotation = before_len != annotations.len();

            if !used_annotation
                && is_table_literal
                && let Some(class_name) = class_hints.pop_front()
            {
                ty = TypeKind::Custom(class_name);
                annotated = true;
            }

            self.assign_local(&identifier.name, identifier.range, ty, annotated);
        }
    }

    fn check_assignment(&mut self, assignment: &typed_ast::Assign) {
        let mut annotations = assignment.annotations.clone();
        let mut class_hints: VecDeque<String> = VecDeque::from(assignment.class_hints.clone());

        let expr_types: Vec<TypeKind> = assignment
            .values
            .iter()
            .map(|expr| self.infer_expression(expr))
            .collect();

        for (index, target) in assignment.targets.iter().enumerate() {
            let inferred = expr_types.get(index).cloned().unwrap_or(TypeKind::Nil);

            match &target.kind {
                typed_ast::ExprKind::Name(identifier) => {
                    let is_table_literal = matches!(inferred, TypeKind::Table);
                    let before_len = annotations.len();
                    let (mut ty, mut annotated) =
                        self.apply_type_annotation(identifier, inferred, &mut annotations);
                    let used_annotation = before_len != annotations.len();

                    if !used_annotation
                        && is_table_literal
                        && let Some(class_name) = class_hints.pop_front()
                    {
                        ty = TypeKind::Custom(class_name);
                        annotated = true;
                    }

                    self.assign_nonlocal(&identifier.name, identifier.range, ty, annotated);
                }
                typed_ast::ExprKind::Field { target: base, name } => {
                    if let Some(base_name) = expression_identifier(base)
                        && let Some(TypeKind::Custom(class_name)) = self.lookup(&base_name)
                    {
                        let value_type =
                            expr_types.get(index).cloned().unwrap_or(TypeKind::Unknown);
                        self.validate_field_assignment(&class_name, name, &value_type);
                    }
                }
                _ => {}
            }
        }
    }

    fn check_local_function(&mut self, local_fn: &typed_ast::LocalFunction) {
        let mut annotations = local_fn.annotations.clone();
        let mut param_annotations = local_fn.param_types.clone();

        let inferred = TypeKind::Function;
        let (ty, annotated) =
            self.apply_type_annotation(&local_fn.name, inferred, &mut annotations);
        self.assign_local(&local_fn.name.name, local_fn.name.range, ty, annotated);
        self.clear_type_info(local_fn.name.range);

        let enforce_returns = !local_fn.returns.is_empty();
        if enforce_returns {
            self.return_expectations.push(local_fn.returns.clone());
        }
        self.with_new_scope(|checker| {
            checker.bind_function_parameters(&local_fn.params, &mut param_annotations);
            checker.check_block(&local_fn.body);
        });
        if enforce_returns {
            self.return_expectations.pop();
        }
    }

    fn check_function_declaration(&mut self, function: &typed_ast::Function) {
        let mut annotations = function.annotations.clone();
        let mut param_annotations = function.param_types.clone();

        if let Some(identifier) = function.name.last_component() {
            let inferred = TypeKind::Function;
            let (ty, annotated) =
                self.apply_type_annotation(identifier, inferred, &mut annotations);
            self.assign_nonlocal(&identifier.name, identifier.range, ty, annotated);
            self.clear_type_info(identifier.range);
        }

        let enforce_returns = !function.returns.is_empty();
        if enforce_returns {
            self.return_expectations.push(function.returns.clone());
        }
        self.with_new_scope(|checker| {
            checker.bind_function_parameters(&function.params, &mut param_annotations);
            checker.check_block(&function.body);
        });
        if enforce_returns {
            self.return_expectations.pop();
        }
    }

    fn check_if(&mut self, if_stmt: &typed_ast::IfStmt) {
        let base_scope = self.current_scope_snapshot();
        let mut branch_scopes: Vec<HashMap<String, VariableEntry>> = Vec::new();
        let mut remaining_env = base_scope.clone();

        let mut branches: Vec<(Option<&typed_ast::Expr>, &typed_ast::Block)> = Vec::new();
        for branch in &if_stmt.branches {
            branches.push((Some(&branch.condition), &branch.block));
        }
        if let Some(block) = &if_stmt.else_branch {
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

        if if_stmt.else_branch.is_none() {
            branch_scopes.push(remaining_env);
        }

        let merged = Self::merge_branch_scopes(&base_scope, branch_scopes);
        self.replace_current_scope(merged);
    }

    fn check_while(&mut self, while_stmt: &typed_ast::WhileStmt) {
        self.infer_expression(&while_stmt.condition);
        let base_scope = self.current_scope_snapshot();
        let effect = Self::analyze_condition(&while_stmt.condition);
        let mut loop_env = base_scope.clone();
        Self::apply_narrowing(&mut loop_env, &effect.truthy);

        self.push_scope_with(loop_env);
        self.check_block(&while_stmt.block);
        let loop_scope = self.pop_scope_map();

        let merged = Self::merge_branch_scopes(&base_scope, vec![loop_scope, base_scope.clone()]);
        self.replace_current_scope(merged);
    }

    fn check_repeat(&mut self, repeat_stmt: &typed_ast::RepeatStmt) {
        let base_scope = self.current_scope_snapshot();
        self.push_scope_with(base_scope.clone());
        self.check_block(&repeat_stmt.block);
        let body_scope = self.pop_scope_map();
        self.infer_expression(&repeat_stmt.condition);

        let merged = Self::merge_branch_scopes(&base_scope, vec![body_scope, base_scope.clone()]);
        self.replace_current_scope(merged);
    }

    fn check_numeric_for(&mut self, numeric_for: &typed_ast::NumericForStmt) {
        self.infer_expression(&numeric_for.start);
        self.infer_expression(&numeric_for.end);
        if let Some(step) = &numeric_for.step {
            self.infer_expression(step);
        }

        self.with_new_scope(|checker| {
            checker.assign_local(
                &numeric_for.index.name,
                numeric_for.index.range,
                TypeKind::Number,
                false,
            );
            checker.check_block(&numeric_for.body);
        });
    }

    fn check_generic_for(&mut self, generic_for: &typed_ast::GenericForStmt) {
        for expr in &generic_for.generators {
            self.infer_expression(expr);
        }

        self.with_new_scope(|checker| {
            for identifier in &generic_for.names {
                checker.assign_local(&identifier.name, identifier.range, TypeKind::Unknown, false);
            }
            checker.check_block(&generic_for.body);
        });
    }

    fn bind_function_parameters(
        &mut self,
        params: &[typed_ast::FunctionParam],
        param_annotations: &mut HashMap<String, AnnotatedType>,
    ) {
        for param in params {
            if let Some(identifier) = &param.name {
                let mut ty = TypeKind::Unknown;
                let mut annotated_param = false;
                if let Some(annotation) = param_annotations.remove(&identifier.name) {
                    annotated_param = true;
                    if let Some(expected) = self.resolve_annotation_kind(&annotation) {
                        ty = expected;
                    }
                }
                self.assign_local(&identifier.name, identifier.range, ty, annotated_param);
            }
        }
    }

    fn analyze_condition(expr: &typed_ast::Expr) -> ConditionEffect {
        match &expr.kind {
            typed_ast::ExprKind::Name(identifier) => {
                let mut effect = ConditionEffect::default();
                effect
                    .truthy
                    .push(NarrowRule::ExcludeNil(identifier.name.clone()));
                effect
                    .falsy
                    .push(NarrowRule::RequireNil(identifier.name.clone()));
                effect
            }
            typed_ast::ExprKind::UnaryOp {
                operator,
                expression,
            } => {
                if operator.symbol == "not" {
                    let inner = Self::analyze_condition(expression);
                    ConditionEffect {
                        truthy: inner.falsy,
                        falsy: inner.truthy,
                    }
                } else {
                    ConditionEffect::default()
                }
            }
            typed_ast::ExprKind::BinaryOp {
                left,
                operator,
                right,
            } => match operator.symbol.as_str() {
                "==" => Self::analyze_equality(left, right, true),
                "~=" => Self::analyze_equality(left, right, false),
                _ => ConditionEffect::default(),
            },
            typed_ast::ExprKind::Parentheses(inner) => Self::analyze_condition(inner),
            _ => ConditionEffect::default(),
        }
    }

    fn analyze_equality(
        lhs: &typed_ast::Expr,
        rhs: &typed_ast::Expr,
        is_equal: bool,
    ) -> ConditionEffect {
        if let Some(effect) = Self::analyze_type_comparison(lhs, rhs, is_equal) {
            return effect;
        }

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

    fn analyze_type_comparison(
        lhs: &typed_ast::Expr,
        rhs: &typed_ast::Expr,
        is_equal: bool,
    ) -> Option<ConditionEffect> {
        if let Some(name) = type_call_variable(lhs)
            && let Some(kind) = type_literal_kind(rhs)
        {
            return Some(Self::build_type_comparison(name, kind, is_equal));
        }

        if let Some(name) = type_call_variable(rhs)
            && let Some(kind) = type_literal_kind(lhs)
        {
            return Some(Self::build_type_comparison(name, kind, is_equal));
        }

        None
    }

    fn build_type_comparison(name: String, kind: TypeKind, is_equal: bool) -> ConditionEffect {
        let mut effect = ConditionEffect::default();
        if is_equal {
            effect
                .truthy
                .push(NarrowRule::RequireType(name.clone(), kind.clone()));
            effect.falsy.push(NarrowRule::ExcludeType(name, kind));
        } else {
            effect
                .truthy
                .push(NarrowRule::ExcludeType(name.clone(), kind.clone()));
            effect.falsy.push(NarrowRule::RequireType(name, kind));
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
                NarrowRule::RequireType(name, target) => {
                    if let Some(entry) = scope.get_mut(name) {
                        entry.ty = type_only_kind(&entry.ty, target);
                    }
                }
                NarrowRule::ExcludeType(name, target) => {
                    if let Some(entry) = scope.get_mut(name) {
                        entry.ty = type_without_kind(&entry.ty, target);
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

    fn assign_local(&mut self, name: &str, range: TextRange, ty: TypeKind, annotated: bool) {
        let prev_annotated = self
            .lookup_entry(name)
            .map(|entry| entry.annotated)
            .unwrap_or(false);
        let merged_annotated = prev_annotated || annotated;
        self.emit_reassignment(name, range, &ty, merged_annotated);
        self.record_type(range, ty.clone());
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

    fn assign_nonlocal(&mut self, name: &str, range: TextRange, ty: TypeKind, annotated: bool) {
        let prev_annotated = self
            .lookup_entry(name)
            .map(|entry| entry.annotated)
            .unwrap_or(false);
        let merged_annotated = prev_annotated || annotated;
        self.emit_reassignment(name, range, &ty, merged_annotated);
        self.record_type(range, ty.clone());

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

    fn emit_reassignment(&mut self, name: &str, range: TextRange, ty: &TypeKind, annotated: bool) {
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
            self.push_diagnostic(
                Some(range),
                message,
                Some(DiagnosticCode::AssignTypeMismatch),
            );
        }
    }

    fn record_type(&mut self, range: TextRange, ty: TypeKind) {
        if matches!(ty, TypeKind::Unknown) {
            return;
        }

        let start = range.start;
        let end = range.end;
        self.type_info.insert(
            DocumentPosition {
                row: start.line,
                col: start.character,
            },
            TypeInfo {
                ty: ty.to_string(),
                end_line: end.line,
                end_character: end.character,
            },
        );
    }

    fn clear_type_info(&mut self, range: TextRange) {
        let start = range.start;
        self.type_info.remove(&DocumentPosition {
            row: start.line,
            col: start.character,
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

    fn infer_expression(&mut self, expression: &typed_ast::Expr) -> TypeKind {
        match &expression.kind {
            typed_ast::ExprKind::Number(_) => TypeKind::Number,
            typed_ast::ExprKind::String(_) => TypeKind::String,
            typed_ast::ExprKind::TableConstructor(fields) => self.infer_table_constructor(fields),
            typed_ast::ExprKind::Function(_) => TypeKind::Function,
            typed_ast::ExprKind::Parentheses(inner) => self.infer_expression(inner),
            typed_ast::ExprKind::UnaryOp { expression, .. } => self.infer_expression(expression),
            typed_ast::ExprKind::BinaryOp {
                left,
                operator,
                right,
            } => self.infer_binary(left, operator, right),
            typed_ast::ExprKind::Call(_) | typed_ast::ExprKind::MethodCall(_) => TypeKind::Unknown,
            typed_ast::ExprKind::Name(identifier) => {
                self.lookup(&identifier.name).unwrap_or(TypeKind::Unknown)
            }
            typed_ast::ExprKind::Boolean(_) => TypeKind::Boolean,
            typed_ast::ExprKind::Nil => TypeKind::Nil,
            _ => TypeKind::Unknown,
        }
    }

    fn infer_binary(
        &mut self,
        lhs: &typed_ast::Expr,
        operator: &typed_ast::Operator,
        rhs: &typed_ast::Expr,
    ) -> TypeKind {
        match operator.symbol.as_str() {
            "+" | "-" | "*" | "/" | "%" | "^" => {
                let left = self.infer_expression(lhs);
                let right = self.infer_expression(rhs);
                self.expect_type(operator, left, TypeKind::Number, OperandSide::Left);
                self.expect_type(operator, right, TypeKind::Number, OperandSide::Right);
                TypeKind::Number
            }
            ".." => {
                let left = self.infer_expression(lhs);
                let right = self.infer_expression(rhs);
                self.expect_type(operator, left, TypeKind::String, OperandSide::Left);
                self.expect_type(operator, right, TypeKind::String, OperandSide::Right);
                TypeKind::String
            }
            "and" | "or" => {
                let left = self.infer_expression(lhs);
                let right = self.infer_expression(rhs);
                self.expect_type(operator, left, TypeKind::Boolean, OperandSide::Left);
                self.expect_type(operator, right, TypeKind::Boolean, OperandSide::Right);
                TypeKind::Boolean
            }
            _ => {
                self.infer_expression(lhs);
                self.infer_expression(rhs);
                TypeKind::Unknown
            }
        }
    }

    fn infer_table_constructor(&mut self, fields: &[typed_ast::TableField]) -> TypeKind {
        if let Some(array_type) = self.try_infer_array_literal(fields) {
            return array_type;
        }

        TypeKind::Table
    }

    fn try_infer_array_literal(&mut self, fields: &[typed_ast::TableField]) -> Option<TypeKind> {
        if fields.is_empty() {
            return None;
        }

        let mut element_types = Vec::new();
        for field in fields {
            match field {
                typed_ast::TableField::Array { value, .. } => {
                    let ty = self.infer_expression(value);
                    element_types.push(ty);
                }
                _ => return None,
            }
        }

        if element_types.is_empty() {
            return None;
        }

        let mut flattened = Vec::new();
        for ty in element_types {
            flatten_union(&ty, &mut flattened);
        }

        let element_type = build_union(flattened);
        Some(TypeKind::Array(Box::new(element_type)))
    }

    fn expect_type(
        &mut self,
        operator: &typed_ast::Operator,
        actual: TypeKind,
        expected: TypeKind,
        side: OperandSide,
    ) {
        if actual == TypeKind::Unknown || expected.matches(&actual) {
            return;
        }

        let message = format!(
            "operator '{}' expected {} operand of type {}, but found {}",
            operator.symbol,
            side.describe(),
            expected,
            actual
        );
        self.push_diagnostic(
            Some(operator.range),
            message,
            Some(DiagnosticCode::AssignTypeMismatch),
        );
    }

    fn push_diagnostic(
        &mut self,
        range: Option<TextRange>,
        message: String,
        code: Option<DiagnosticCode>,
    ) {
        self.diagnostics
            .push(Diagnostic::error(self.path_buf(), message, range, code));
    }

    fn path_buf(&self) -> PathBuf {
        self.path.to_path_buf()
    }
}

fn expression_identifier(expr: &typed_ast::Expr) -> Option<String> {
    match &expr.kind {
        typed_ast::ExprKind::Name(identifier) => Some(identifier.name.clone()),
        typed_ast::ExprKind::Parentheses(inner) => expression_identifier(inner),
        _ => None,
    }
}

fn expression_is_nil(expr: &typed_ast::Expr) -> bool {
    matches!(expr.kind, typed_ast::ExprKind::Nil)
}

fn type_call_variable(expr: &typed_ast::Expr) -> Option<String> {
    let typed_ast::ExprKind::Call(call) = &expr.kind else {
        return None;
    };

    if !matches!(call.function.kind, typed_ast::ExprKind::Name(ref ident) if ident.name == "type") {
        return None;
    }

    let typed_ast::CallArgs::Parentheses(args) = &call.args else {
        return None;
    };

    if args.len() != 1 {
        return None;
    }

    expression_identifier(&args[0])
}

fn type_literal_kind(expr: &typed_ast::Expr) -> Option<TypeKind> {
    match &expr.kind {
        typed_ast::ExprKind::String(raw) => {
            let trimmed = raw.trim();
            let literal = trimmed.trim_matches(|c| c == '"' || c == '\'');
            if literal.is_empty() {
                return None;
            }
            match literal {
                "number" => Some(TypeKind::Number),
                "string" => Some(TypeKind::String),
                "table" => Some(TypeKind::Table),
                "boolean" => Some(TypeKind::Boolean),
                "function" => Some(TypeKind::Function),
                "thread" => Some(TypeKind::Thread),
                "nil" => Some(TypeKind::Nil),
                other => Some(TypeKind::Custom(other.to_string())),
            }
        }
        _ => None,
    }
}

fn type_only_nil(ty: &TypeKind) -> TypeKind {
    type_only_kind(ty, &TypeKind::Nil)
}

fn type_without_nil(ty: &TypeKind) -> TypeKind {
    type_without_kind(ty, &TypeKind::Nil)
}

fn type_only_kind(ty: &TypeKind, target: &TypeKind) -> TypeKind {
    if type_contains_kind(ty, target) {
        target.clone()
    } else {
        TypeKind::Unknown
    }
}

fn type_without_kind(ty: &TypeKind, target: &TypeKind) -> TypeKind {
    match ty {
        TypeKind::Union(items) => {
            let mut kept = Vec::new();
            for item in items {
                let filtered = type_without_kind(item, target);
                if !matches!(filtered, TypeKind::Unknown) {
                    flatten_union(&filtered, &mut kept);
                }
            }
            build_union(kept)
        }
        other if other == target => TypeKind::Unknown,
        _ => ty.clone(),
    }
}

fn type_contains_kind(ty: &TypeKind, target: &TypeKind) -> bool {
    match ty {
        other if other == target => true,
        TypeKind::Union(items) => items.iter().any(|item| type_contains_kind(item, target)),
        _ => false,
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
    use unindent::unindent;

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
        let source = unindent(
            r##"
            local x = 1
            x = "oops"
            "##,
        );
        let result = run_type_check(&source);
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
        let source = unindent(
            r##"
            ---@type number
            local x = 1
            x = "oops"
            "##,
        );
        let result = run_type_check(&source);
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
    fn array_annotation_inlay_hint_uses_full_type() {
        let source = unindent(
            r#"
            ---@type (boolean|number)[]
            local t = { true, 1 }
            "#,
        );

        let result = run_type_check(&source);
        let info = result
            .type_map
            .get(&DocumentPosition { row: 2, col: 7 })
            .expect("missing type info for array annotation");

        assert_eq!(info.ty, "(boolean|number)[]");
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn array_annotation_reports_element_type_mismatch() {
        let source = unindent(
            r#"
            ---@type boolean[]
            local t = {1, 2, 3}
            "#,
        );

        let result = run_type_check(&source);

        assert_eq!(result.diagnostics.len(), 1);
        let diagnostic = &result.diagnostics[0];
        assert_eq!(diagnostic.code, Some(DiagnosticCode::AssignTypeMismatch));
        assert!(
            diagnostic
                .message
                .contains("annotated as type boolean[] but inferred type is number[]")
        );

        let info = result
            .type_map
            .get(&DocumentPosition { row: 2, col: 7 })
            .expect("missing type info for boolean[] annotation");
        assert_eq!(info.ty, "boolean[]");
    }

    #[test]
    fn reports_variable_reassignment_type_conflict() {
        let source = unindent(
            r#"
            local x = 1
            x = "oops"
            "#,
        );
        let result = run_type_check(&source);

        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn reports_arithmetic_operand_type_mismatch() {
        let source = unindent(
            r#"
            local a = "hello"
            local b = a + 1
            "#,
        );
        let result = run_type_check(&source);

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
        let source = unindent(
            r#"
            ---@type number|nil
            local value = nil
            if value ~= nil then
                value = value
            else
                value = value
            end
        "#,
        );

        let result = run_type_check(&source);
        assert!(result.diagnostics.is_empty());

        let position = DocumentPosition { row: 4, col: 5 };
        let info = result
            .type_map
            .get(&position)
            .expect("type info for narrowed assignment");
        assert_eq!(info.ty, "number");

        let position = DocumentPosition { row: 6, col: 5 };
        let info = result
            .type_map
            .get(&position)
            .expect("type info for narrowed assignment");
        assert_eq!(info.ty, "nil");
    }

    #[test]
    fn narrowing_exclude_builting_type_in_not_equals() {
        let source = unindent(
            r#"
            ---@type number|string|boolean
            local value = "hello"
            if type(value) ~= "string" then
                local num_or_bool = value
            elseif type(value) ~= "boolean" then
                local num = value
            end
        "#,
        );

        let result = run_type_check(&source);
        assert!(result.diagnostics.is_empty());

        // num_or_bool
        let position = DocumentPosition { row: 4, col: 11 };
        let info = result
            .type_map
            .get(&position)
            .expect("type info for narrowed assignment");
        assert_eq!(info.ty, "boolean|number");

        // num
        let position = DocumentPosition { row: 6, col: 11 };
        let info = result
            .type_map
            .get(&position)
            .expect("type info for narrowed assignment");
        assert_eq!(info.ty, "string");
    }

    #[test]
    fn narrowing_exclude_builting_type_in_equals() {
        let source = unindent(
            r#"
            ---@type number|string|boolean
            local value = "hello"
            if type(value) == "string" then
                local s = value
            elseif type(value) == "boolean" then
                local b = value
            else
                local n = value
            end
        "#,
        );

        let result = run_type_check(&source);
        assert!(result.diagnostics.is_empty());

        // string
        let position = DocumentPosition { row: 4, col: 11 };
        let info = result
            .type_map
            .get(&position)
            .expect("type info for narrowed assignment");
        assert_eq!(info.ty, "string");

        // boolean
        let position = DocumentPosition { row: 6, col: 11 };
        let info = result
            .type_map
            .get(&position)
            .expect("type info for narrowed assignment");
        assert_eq!(info.ty, "boolean");

        // number
        let position = DocumentPosition { row: 8, col: 11 };
        let info = result
            .type_map
            .get(&position)
            .expect("type info for narrowed assignment");
        assert_eq!(info.ty, "number");
    }

    #[test]
    fn mismatch_type_annotation() {
        let source = unindent(
            r#"
            ---@type string
            local title = 10
            "#,
        );
        let result = run_type_check(&source);
        assert_eq!(result.diagnostics.len(), 1);
        let diagnostic = &result.diagnostics[0];
        assert!(diagnostic.message.contains("annotated as type string"));
    }

    #[test]
    fn param_annotation_enforces_type_in_body() {
        let source = unindent(
            r#"
            ---@param amount number
            local function charge(amount)
                amount = "free"
            end
            "#,
        );
        let result = run_type_check(&source);

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
        let source = unindent(
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
        let result = run_type_check(&source);
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
        let a_source = unindent(
            r##"
            ---@class Point
            ---@field x number
            ---@field y number
        "##,
        );
        let (_, registry_a) = AnnotationIndex::from_source(&a_source);

        let mut workspace_registry = TypeRegistry::default();
        workspace_registry.extend(&registry_a);

        let b_source = unindent(
            r##"
            ---@type Point
            local p = {}
        "##,
        );
        let ast = full_moon::parse(&b_source).expect("failed to parse reference source");
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
        let source = unindent(
            r#"
            ---@return number
            local function value()
                return "oops"
            end
            "#,
        );
        let result = run_type_check(&source);

        assert_eq!(result.diagnostics.len(), 1);
        let diagnostic = &result.diagnostics[0];
        assert!(diagnostic.message.contains("return value #1"));
    }

    #[test]
    fn return_annotation_accepts_correct_type() {
        let source = unindent(
            r#"
            ---@return number
            local function value()
                return 42
            end
            "#,
        );
        let result = run_type_check(&source);

        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn class_annotation_maps_to_table() {
        let source = unindent(
            r#"
            ---@class Person
            ---@type Person
            local person = {}
            "#,
        );
        let result = run_type_check(&source);

        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn class_annotation_infers_type_for_following_local_assignment() {
        let source = unindent(
            r##"
            ---@class Container
            local C = {}
            "##,
        );
        let result = run_type_check(&source);

        assert!(result.diagnostics.is_empty());
        let info = result
            .type_map
            .get(&DocumentPosition { row: 2, col: 7 })
            .expect("missing type info for local assignment");
        assert_eq!(info.ty, "Container");
    }

    #[test]
    fn class_annotation_infers_type_for_following_assignment() {
        let source = unindent(
            r##"
            ---@class Container
            Container = {}
            "##,
        );
        let result = run_type_check(&source);

        assert!(result.diagnostics.is_empty());
        let info = result
            .type_map
            .get(&DocumentPosition { row: 2, col: 1 })
            .expect("missing type info for assignment");
        assert_eq!(info.ty, "Container");
    }

    #[test]
    fn enum_annotation_treated_as_string() {
        let source = unindent(
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
        let result = run_type_check(&source);
        assert!(result.diagnostics.is_empty());
    }
}
