use std::path::Path;

use full_moon::parse;
use typua::diagnostics::DiagnosticCode;
use typua::typechecker::{
    TypeRegistry, check_ast, check_ast_with_registry, types::AnnotationIndex,
};
use unindent::unindent;

fn parse_source(source: &str) -> full_moon::ast::Ast {
    parse(source).expect("failed to parse test source")
}

#[test]
fn local_assignment_type_mismatch_reports_diagnostic() {
    let source = unindent(
        r#"
    ---@type number
    local value = "oops"
    "#,
    );

    let ast = parse_source(&source);
    let result = check_ast(Path::new("single.lua"), &source, &ast);
    assert_eq!(result.diagnostics.len(), 1);
    let diagnostic = &result.diagnostics[0];
    assert!(
        diagnostic
            .message
            .contains("annotated as type number but inferred type is string"),
        "unexpected diagnostic message: {}",
        diagnostic.message
    );
    assert_eq!(diagnostic.code, Some(DiagnosticCode::AssignTypeMismatch));
}

#[test]
fn workspace_registry_tracks_class_fields_across_files() {
    let class_source = unindent(
        r#"
    ---@class Foo
    ---@field value number
    "#,
    );

    let class_ast = parse_source(&class_source);
    let (_, class_registry) = AnnotationIndex::from_ast(&class_ast, &class_source);

    let usage_source = unindent(
        r#"
    ---@type Foo
    local foo = {}
    foo.value = "invalid"
    "#,
    );

    let usage_ast = parse_source(&usage_source);
    let mut workspace_registry = TypeRegistry::default();
    workspace_registry.extend(&class_registry);

    let result = check_ast_with_registry(
        Path::new("usage.lua"),
        &usage_source,
        &usage_ast,
        Some(&workspace_registry),
    );

    assert_eq!(result.diagnostics.len(), 1);
    let diagnostic = &result.diagnostics[0];
    assert!(
        diagnostic
            .message
            .contains("field 'value' in class Foo expects type number"),
        "unexpected diagnostic message: {}",
        diagnostic.message
    );
    assert_eq!(diagnostic.code, Some(DiagnosticCode::ParamTypeMismatch));
}

#[test]
fn workspace_registry_allows_cross_file_class_usage() {
    let type_dec_source = unindent(
        r#"
    ---@class Foo
    ---@field value number
    "#,
    );

    let type_dec_ast = parse_source(&type_dec_source);
    let (_, type_dec_registry) = AnnotationIndex::from_ast(&type_dec_ast, &type_dec_source);

    let main_source = unindent(
        r#"
    ---@type Foo
    local foo = {}
    foo.value = 1
    "#,
    );

    let main_ast = parse_source(&main_source);
    let mut workspace_registry = TypeRegistry::default();
    workspace_registry.extend(&type_dec_registry);

    let result = check_ast_with_registry(
        Path::new("main.lua"),
        &main_source,
        &main_ast,
        Some(&workspace_registry),
    );

    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn multi_return_function_respects_annotations() {
    let source = include_str!("scripts/multi-return-ok.lua");
    let ast = parse_source(source);
    let result = check_ast(Path::new("multi-return-ok.lua"), source, &ast);
    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );
}

#[test]
fn multi_return_missing_value_reports_diagnostic() {
    let source = include_str!("scripts/multi-return-missing.lua");
    let ast = parse_source(source);
    let result = check_ast(Path::new("multi-return-missing.lua"), source, &ast);

    assert_eq!(result.diagnostics.len(), 1);
    let diagnostic = &result.diagnostics[0];
    assert!(
        diagnostic
            .message
            .contains("function annotated to return 2 value(s) (expected: result, err)"),
        "unexpected diagnostic message: {}",
        diagnostic.message
    );
    assert_eq!(diagnostic.code, Some(DiagnosticCode::ReturnTypeMismatch));
}

#[test]
fn multi_return_extra_value_reports_diagnostic() {
    let source = include_str!("scripts/multi-return-extra.lua");
    let ast = parse_source(source);
    let result = check_ast(Path::new("multi-return-extra.lua"), source, &ast);

    assert_eq!(result.diagnostics.len(), 1);
    let diagnostic = &result.diagnostics[0];
    assert!(
        diagnostic
            .message
            .contains("function returns 2 value(s) but only 1 annotated via @return"),
        "unexpected diagnostic message: {}",
        diagnostic.message
    );
    assert_eq!(diagnostic.code, Some(DiagnosticCode::ReturnTypeMismatch));
}

#[test]
fn multi_return_type_mismatch_reports_diagnostic() {
    let source = include_str!("scripts/multi-return-type-mismatch.lua");
    let ast = parse_source(source);
    let result = check_ast(Path::new("multi-return-type-mismatch.lua"), source, &ast);

    assert_eq!(result.diagnostics.len(), 1);
    let diagnostic = &result.diagnostics[0];
    assert!(
        diagnostic
            .message
            .contains("return value 'err' is annotated as type string but inferred type is number"),
        "unexpected diagnostic message: {}",
        diagnostic.message
    );
    assert_eq!(diagnostic.code, Some(DiagnosticCode::ReturnTypeMismatch));
}

#[test]
fn workspace_multi_return_mismatch_reports_diagnostic() {
    let lib_source = include_str!("scripts/multi-return-workspace-lib.lua");
    let lib_ast = parse_source(lib_source);
    let (_, lib_registry) = AnnotationIndex::from_ast(&lib_ast, lib_source);

    let mut workspace_registry = TypeRegistry::default();
    workspace_registry.extend(&lib_registry);

    let usage_source = include_str!("scripts/multi-return-extra.lua");
    let usage_ast = parse_source(usage_source);
    let result = check_ast_with_registry(
        Path::new("multi-return-extra.lua"),
        usage_source,
        &usage_ast,
        Some(&workspace_registry),
    );

    assert_eq!(result.diagnostics.len(), 1);
    let diagnostic = &result.diagnostics[0];
    assert!(
        diagnostic
            .message
            .contains("function returns 2 value(s) but only 1 annotated via @return"),
        "unexpected diagnostic message: {}",
        diagnostic.message
    );
    assert_eq!(diagnostic.code, Some(DiagnosticCode::ReturnTypeMismatch));
}

#[test]
fn function_return_annotation_propagates_function_type() {
    let source = unindent(
        r#"
    ---@param x number
    ---@return fun(y: number): number
    local function gen_const(x)
        return function(y)
            return y + x
        end
    end
    local const = gen_const(12)
    local result_value = const(3)
    "#,
    );

    let ast = parse_source(&source);
    let result = check_ast(Path::new("function-return.lua"), &source, &ast);

    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );

    let const_entry = result
        .type_map
        .iter()
        .find(|(pos, info)| pos.row == 8 && info.ty == "fun(number): number")
        .expect("missing type info for const");
    assert_eq!(const_entry.1.ty, "fun(number): number");

    let value_entry = result
        .type_map
        .iter()
        .find(|(pos, info)| pos.row == 9 && info.ty == "number")
        .expect("missing type info for result_value");
    assert_eq!(value_entry.1.ty, "number");
}

#[test]
fn class_field_function_annotation_sets_function_type() {
    let source = unindent(
        r#"
    ---@class ConstGenerator
    ---@field gen_const fun(y: number): number

    ---@type ConstGenerator
    local const = {}

    const.gen_const = function (y)
        return y
    end
    "#,
    );

    let ast = parse_source(&source);
    let result = check_ast(Path::new("const-generator.lua"), &source, &ast);

    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );

    let field_entry = result
        .type_map
        .iter()
        .find(|(pos, info)| pos.row == 7 && info.ty == "fun(number): number")
        .expect("missing type info for const.gen_const");
    assert_eq!(field_entry.1.ty, "fun(number): number");
}

#[test]
fn multi_return_scripts_match_expected_patterns() {
    let ok_source = include_str!("scripts/multi-return-ok.lua");
    assert!(ok_source.contains("---@return number? result"));
    assert!(ok_source.contains("---@return string? err"));
    assert!(ok_source.contains("return x, nil"));
    assert!(ok_source.contains("return nil, \"error\""));

    let missing_source = include_str!("scripts/multi-return-missing.lua");
    assert!(missing_source.contains("---@return number result"));
    assert!(missing_source.contains("---@return string? err"));
    assert!(missing_source.contains("return multi(1)"));

    let extra_source = include_str!("scripts/multi-return-extra.lua");
    assert!(extra_source.contains("---@return number result"));
    assert!(extra_source.contains("return x, \"extra\""));

    let mismatch_source = include_str!("scripts/multi-return-type-mismatch.lua");
    assert!(mismatch_source.contains("---@return number result"));
    assert!(mismatch_source.contains("---@return string err"));
    assert!(mismatch_source.contains("return x, 1"));

    let workspace_source = include_str!("scripts/multi-return-workspace-lib.lua");
    assert!(workspace_source.contains("---@class Wrapper"));
    assert!(workspace_source.contains("---@return Wrapper result"));
    assert!(workspace_source.contains("---@return string? err"));
}

#[test]
fn generic_function_instantiates_types() {
    let source = unindent(
        r#"
    ---@generics T
    ---@param x T
    ---@return T, T[]
    local function generic_func(x)
        return x, {x, x}
    end
    local a, b = generic_func(12)
    local f, fs = generic_func(function() return 12 end)
    local g, gs = generic_func(function(x) return 12 end)
    "#,
    );

    let ast = parse_source(&source);
    let result = check_ast(Path::new("function-generics.lua"), &source, &ast);

    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );

    let find_type = |row: usize, expected: &str| {
        result
            .type_map
            .iter()
            .find(|(pos, info)| pos.row == row && info.ty == expected)
            .unwrap_or_else(|| panic!("missing type {expected} at row {row}"));
    };

    find_type(7, "number");
    find_type(7, "number[]");
    find_type(8, "fun(): number");
    find_type(8, "function[]");
    find_type(9, "fun(any): number");
    find_type(9, "function[]");
}

#[test]
fn class_generics_specialize_array() {
    let source = unindent(
        r#"
    ---@class Array<T>: { [integer]: T }

    ---@type Array<string>
    local arr = {}
    "#,
    );

    let ast = parse_source(&source);
    let result = check_ast(Path::new("array-generics.lua"), &source, &ast);

    assert!(
        result.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        result.diagnostics
    );

    let entry = result
        .type_map
        .iter()
        .find(|(pos, info)| pos.row == 4 && info.ty == "string[]")
        .expect("missing type info for arr");
    assert_eq!(entry.1.ty, "string[]");
}
