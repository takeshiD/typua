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
