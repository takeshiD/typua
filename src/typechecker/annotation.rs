use full_moon::ast;
use std::{borrow::Cow, collections::HashMap};

use super::types::{
    AnnotatedType, Annotation, AnnotationIndex, AnnotationUsage, ClassDeclaration, FunctionParam,
    FunctionType, TypeKind, TypeRegistry,
};

use full_moon::tokenizer::{Lexer, LexerResult, Token, TokenType};

impl AnnotationIndex {
    pub fn from_ast(ast: &ast::Ast, source: &str) -> (Self, TypeRegistry) {
        let _ = ast;
        let lexer = Lexer::new(source, ast::LuaVersion::new());
        let tokens = match lexer.collect() {
            LexerResult::Ok(tokens) | LexerResult::Recovered(tokens, _) => tokens,
            LexerResult::Fatal(_) => return Self::from_source(source),
        };

        build_index_from_tokens(tokens, source)
    }
    pub fn from_source(source: &str) -> (Self, TypeRegistry) {
        let mut by_line: HashMap<usize, Vec<Annotation>> = HashMap::new();
        let mut class_hints: HashMap<usize, Vec<String>> = HashMap::new();
        let mut pending: Vec<Annotation> = Vec::new();
        let mut pending_classes: Vec<String> = Vec::new();
        let mut registry = TypeRegistry::default();
        let mut current_class: Option<String> = None;

        for (idx, line) in source.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim_start();

            if let Some(decl) = parse_class_declaration(trimmed) {
                pending_classes.push(decl.name.clone());
                current_class = Some(decl.name.clone());
                registry.register_class(decl);
                continue;
            }

            if let Some(name) = parse_enum_declaration(trimmed) {
                registry.register_enum(&name);
                current_class = None;
                pending_classes.clear();
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

            if !pending_classes.is_empty() {
                class_hints
                    .entry(line_no)
                    .or_default()
                    .append(&mut pending_classes);
            }

            current_class = None;

            if !pending.is_empty() {
                by_line.entry(line_no).or_default().append(&mut pending);
            }
        }

        (
            Self {
                by_line,
                class_hints,
            },
            registry,
        )
    }
}

fn build_index_from_tokens(tokens: Vec<Token>, source: &str) -> (AnnotationIndex, TypeRegistry) {
    let mut by_line: HashMap<usize, Vec<Annotation>> = HashMap::new();
    let mut class_hints: HashMap<usize, Vec<String>> = HashMap::new();
    let mut pending_annotations: Vec<Annotation> = Vec::new();
    let mut pending_classes: Vec<String> = Vec::new();
    let mut registry = TypeRegistry::default();
    let mut current_class: Option<String> = None;
    let lines: Vec<&str> = source.lines().collect();

    for token in tokens {
        if let TokenType::SingleLineComment { comment } = token.token_type() {
            let line = token.start_position().line();
            if line == 0 || !is_annotation_leading(&lines, line, token.start_position().character())
            {
                continue;
            }

            let trimmed = comment.as_str().trim_start();
            let normalized: Cow<'_, str> = if trimmed.starts_with('-') {
                Cow::Owned(format!("--{trimmed}"))
            } else {
                Cow::Borrowed(trimmed)
            };

            if let Some(decl) = parse_class_declaration(&normalized) {
                pending_classes.push(decl.name.clone());
                current_class = Some(decl.name.clone());
                registry.register_class(decl);
                continue;
            }

            if let Some(name) = parse_enum_declaration(&normalized) {
                registry.register_enum(&name);
                current_class = None;
                pending_classes.clear();
                continue;
            }

            if let Some((field_name, field_ty)) = parse_field_declaration(&normalized) {
                if let Some(class_name) = current_class.clone() {
                    registry.register_field(&class_name, &field_name, field_ty);
                }
                continue;
            }

            if let Some(annotation) = parse_annotation(&normalized) {
                pending_annotations.push(annotation);
            }

            continue;
        }

        if matches!(token.token_type(), TokenType::Eof) {
            break;
        }

        if token.token_type().is_trivia() {
            continue;
        }

        let line = token.start_position().line();
        if line == 0 {
            continue;
        }

        if !pending_classes.is_empty() {
            class_hints
                .entry(line)
                .or_default()
                .append(&mut pending_classes);
        }
        current_class = None;

        if !pending_annotations.is_empty() {
            by_line
                .entry(line)
                .or_default()
                .append(&mut pending_annotations);
        }
    }

    (
        AnnotationIndex {
            by_line,
            class_hints,
        },
        registry,
    )
}

fn is_annotation_leading(lines: &[&str], line: usize, column: usize) -> bool {
    if line == 0 {
        return false;
    }
    match lines.get(line.saturating_sub(1)) {
        Some(text) => text
            .chars()
            .take(column.saturating_sub(1))
            .all(char::is_whitespace),
        None => true,
    }
}

pub(crate) fn parse_annotation(line: &str) -> Option<Annotation> {
    if let Some(rest) = line.strip_prefix("---@type") {
        let type_token = rest.trim();
        let ty = AnnotatedType::new(type_token.to_string(), parse_type(type_token));
        return Some(Annotation {
            usage: AnnotationUsage::Type,
            name: None,
            ty,
        });
    }

    if let Some(rest) = line.strip_prefix("---@param") {
        let mut parts = rest.split_whitespace();
        let name = parts.next()?.to_string();
        let type_token = parts.next().unwrap_or("any");
        let ty = AnnotatedType::new(type_token.to_string(), parse_type(type_token));
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
        let ty = AnnotatedType::new(type_token.to_string(), parse_type(type_token));
        return Some(Annotation {
            usage: AnnotationUsage::Return,
            name,
            ty,
        });
    }
    None
}

pub(crate) fn parse_type(raw: &str) -> Option<TypeKind> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Optional type sugar: Type? -> Type|nil
    if let Some(stripped) = trimmed.strip_suffix('?') {
        let base_type = parse_type(stripped.trim())?;
        return Some(make_union(vec![base_type, TypeKind::Nil]));
    }

    // Function signature: fun(<params>): <returns>
    if trimmed.starts_with("fun(") || trimmed.starts_with("fun<") {
        return parse_function_type(trimmed);
    }

    // Dictionary literal type: { [K]: V }
    if trimmed.starts_with('{')
        && trimmed.ends_with('}')
        && let Some((k, v)) = parse_dictionary_type(trimmed)
    {
        return Some(TypeKind::Applied {
            base: Box::new(TypeKind::Custom("table".to_string())),
            args: vec![k, v],
        });
    }

    // Tuple: [A, B, C]
    if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.contains(',') {
        let inner = &trimmed[1..trimmed.len() - 1];
        let mut members = Vec::new();
        for part in inner.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            if let Some(t) = parse_type(part) {
                members.push(t);
            }
        }
        if !members.is_empty() {
            return Some(TypeKind::Applied {
                base: Box::new(TypeKind::Custom("tuple".to_string())),
                args: members,
            });
        }
    }

    // Generic application: base<Arg1, Arg2, ...>
    if let Some((base, args)) = parse_applied_type(trimmed) {
        return Some(TypeKind::Applied {
            base: Box::new(base),
            args,
        });
    }

    if trimmed.contains('|') {
        let mut members = Vec::new();
        for part in trimmed.split('|').map(str::trim).filter(|p| !p.is_empty()) {
            members.push(parse_type(part)?);
        }
        return Some(make_union(members));
    }

    if let Some(stripped) = trimmed.strip_suffix("[]") {
        let base_type = parse_type(stripped.trim())?;
        return Some(TypeKind::Array(Box::new(base_type)));
    }

    parse_atomic_type(trimmed)
}

fn parse_atomic_type(raw: &str) -> Option<TypeKind> {
    if raw.starts_with('"') || raw.starts_with('\'') {
        return Some(TypeKind::String);
    }

    let lower = raw.to_ascii_lowercase();

    match lower.as_str() {
        "nil" => Some(TypeKind::Nil),
        "boolean" | "bool" => Some(TypeKind::Boolean),
        "string" => Some(TypeKind::String),
        "number" => Some(TypeKind::Number),
        "integer" | "int" => Some(TypeKind::Integer),
        "table" => Some(TypeKind::Table),
        "function" | "fun" => Some(TypeKind::Function),
        "thread" => Some(TypeKind::Thread),
        "any" => None,
        _ => Some(TypeKind::Custom(raw.to_string())),
    }
}

fn parse_applied_type(raw: &str) -> Option<(TypeKind, Vec<TypeKind>)> {
    // base<Arg, Arg2>
    let _chars = raw.chars();
    let mut depth = 0usize;
    let mut open_idx = None;
    for (i, ch) in raw.char_indices() {
        match ch {
            '<' => {
                if depth == 0 {
                    open_idx = Some(i);
                }
                depth += 1;
            }
            '>' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                if depth == 0 {
                    // base .. <args>
                    let base_str = raw[..open_idx?].trim();
                    let args_str = &raw[open_idx? + 1..i];
                    let base = TypeKind::Custom(base_str.to_string());
                    let mut args = Vec::new();
                    for part in args_str.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                        if let Some(t) = parse_type(part) {
                            args.push(t);
                        }
                    }
                    return Some((base, args));
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_dictionary_type(raw: &str) -> Option<(TypeKind, TypeKind)> {
    // very lightweight pattern matcher for: { [K]: V }
    let s = raw
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .trim();
    let open = s.find('[')?;
    let close = s[open + 1..].find(']')? + open + 1;
    let key_ty = s[open + 1..close].trim();
    let colon = s[close + 1..].find(':')? + close + 1;
    let val_ty = s[colon + 1..].trim();
    Some((parse_type(key_ty)?, parse_type(val_ty)?))
}

fn parse_function_type(raw: &str) -> Option<TypeKind> {
    // fun(a: number, b: string): boolean, string
    // optional generics: fun<T>(...)
    let mut rest = raw.trim_start_matches("fun");
    // strip optional generics <...>
    if rest.starts_with('<') {
        let mut depth = 0usize;
        let mut idx = 0usize;
        for (i, ch) in rest.char_indices() {
            match ch {
                '<' => depth += 1,
                '>' => {
                    if depth == 0 {
                        return None;
                    }
                    depth -= 1;
                    if depth == 0 {
                        idx = i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        rest = &rest[idx..];
    }
    let rest = rest.trim();
    if !rest.starts_with('(') {
        return None;
    }
    let mut depth = 0usize;
    let mut end = 0usize;
    for (i, ch) in rest.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    if end == 0 {
        return None;
    }
    let params_str = &rest[1..end];
    let after = rest[end + 1..].trim();
    let mut params: Vec<FunctionParam> = Vec::new();
    let mut vararg: Option<Box<TypeKind>> = None;
    if !params_str.trim().is_empty() {
        for p in params_str
            .split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
        {
            if let Some(t) = p.strip_suffix("...") {
                vararg = parse_type(t.trim()).map(Box::new);
                continue;
            }
            let ty = if let Some(col) = p.find(':') {
                parse_type(p[col + 1..].trim())
            } else {
                parse_type(p)
            };
            if let Some(kind) = ty {
                params.push(FunctionParam {
                    name: None,
                    ty: kind,
                    is_self: false,
                    is_vararg: false,
                });
            }
        }
    }
    let mut returns: Vec<TypeKind> = Vec::new();
    if let Some(after_ret) = after.strip_prefix(':') {
        for r in after_ret
            .split(',')
            .map(str::trim)
            .filter(|r| !r.is_empty())
        {
            if let Some(t) = parse_type(r) {
                returns.push(t);
            }
        }
    }
    let ft = FunctionType {
        generics: Vec::new(),
        params,
        returns,
        vararg,
    };
    Some(TypeKind::FunctionSig(Box::new(ft)))
}

pub(crate) fn make_union(types: Vec<TypeKind>) -> TypeKind {
    let mut flat: Vec<TypeKind> = Vec::new();
    for ty in types {
        match ty {
            TypeKind::Union(inner) => flat.extend(inner),
            other => flat.push(other),
        }
    }
    flat.sort_by_key(|a| a.to_string());
    flat.dedup_by(|a, b| a.matches(b));

    if flat.len() == 1 {
        flat.into_iter().next().unwrap()
    } else {
        TypeKind::Union(flat)
    }
}

pub(crate) fn parse_class_declaration(line: &str) -> Option<ClassDeclaration> {
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

pub(crate) fn parse_enum_declaration(line: &str) -> Option<String> {
    let rest = line.strip_prefix("---@enum")?.trim();
    let name = rest.split_whitespace().next()?;
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

pub(crate) fn parse_field_declaration(line: &str) -> Option<(String, AnnotatedType)> {
    let rest = line.strip_prefix("---@field")?.trim();
    let mut parts = rest.split_whitespace();
    let name = parts.next()?.trim();
    let type_token = parts.next().unwrap_or("any");
    let ty = AnnotatedType::new(type_token.to_string(), parse_type(type_token));
    Some((name.to_string(), ty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use full_moon::parse;
    use pretty_assertions::assert_eq;
    use unindent::Unindent;

    #[test]
    fn annotation_type_parsing() {
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
        assert_eq!(
            parse_annotation("---@type number[]").unwrap(),
            Annotation {
                usage: AnnotationUsage::Type,
                name: None,
                ty: AnnotatedType {
                    raw: "number[]".to_string(),
                    kind: Some(TypeKind::Array(Box::new(TypeKind::Number)))
                }
            }
        );
    }

    #[test]
    fn annotation_parsing_more_types() {
        // function type
        let ty = parse_type("fun(a: number, b: string): boolean").unwrap();
        match ty {
            TypeKind::FunctionSig(ft) => {
                assert_eq!(ft.params.len(), 2);
                assert_eq!(ft.returns.len(), 1);
            }
            _ => panic!("expected function type"),
        }

        // applied generic: table<string, number>
        let ty = parse_type("table<string, number>").unwrap();
        match ty {
            TypeKind::Applied { base, args } => {
                match *base {
                    TypeKind::Custom(ref s) if s == "table" => {}
                    _ => panic!("base should be table"),
                }
                assert_eq!(args.len(), 2);
            }
            _ => panic!("expected applied type"),
        }

        // dictionary literal: { [string]: number }
        let ty = parse_type("{ [string]: number }").unwrap();
        match ty {
            TypeKind::Applied { base, args } => {
                match *base {
                    TypeKind::Custom(ref s) if s == "table" => {}
                    _ => panic!("base should be table"),
                }
                assert_eq!(args.len(), 2);
            }
            _ => panic!("expected table applied type"),
        }

        // tuple literal
        let ty = parse_type("[number, string]").unwrap();
        match ty {
            TypeKind::Applied { base, args } => {
                match *base {
                    TypeKind::Custom(ref s) if s == "tuple" => {}
                    _ => panic!("base should be tuple"),
                }
                assert_eq!(args.len(), 2);
            }
            _ => panic!("expected tuple applied type"),
        }
    }

    #[test]
    fn from_ast_collects_annotations_with_padding() {
        let source = r#"
        ---@type number
        -- keep

        local value = 42
        "#;
        let ast = parse(source.unindent().as_str()).expect("parse failure");
        let (index, _) = AnnotationIndex::from_ast(&ast, source);
        let annotations = index.by_line.get(&5).expect("annotation attached");
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].usage, AnnotationUsage::Type);
        assert_eq!(annotations[0].ty.raw, "number");
    }

    #[test]
    fn from_ast_registers_classes_without_statements() {
        let source = r#"
        ---@class Foo
        ---@field bar string
        "#;
        let ast = parse(source.unindent().as_str()).expect("parse failure");
        let (index, registry) = AnnotationIndex::from_ast(&ast, source);
        assert!(index.by_line.is_empty());
        assert!(index.class_hints.is_empty());
        assert!(registry.resolve("Foo").is_some());
        let field = registry
            .field_annotation("Foo", "bar")
            .expect("field registered");
        assert_eq!(field.raw, "string");
    }

    #[test]
    fn from_ast_ignores_inline_annotation_comments() {
        let source = r#"
        local ignored = 0 ---@type string
        local actual = 1
        "#;
        let ast = parse(source.unindent().as_str()).expect("parse failure");
        let (index, _) = AnnotationIndex::from_ast(&ast, source);
        assert!(!index.by_line.contains_key(&1));
        assert!(!index.by_line.contains_key(&2));
    }

    #[test]
    fn from_ast_attaches_block_type_annotations_to_statement() {
        let source = r#"
        ---@type number
        local value = 0
        "#;
        let ast = parse(source.unindent().as_str()).expect("parse failure");
        let (index, _) = AnnotationIndex::from_ast(&ast, source);
        let annotations = index
            .by_line
            .get(&3)
            .expect("annotation attached to statement");
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].usage, AnnotationUsage::Type);
        assert_eq!(annotations[0].ty.raw, "number");
    }

    #[test]
    fn from_ast_attaches_block_class_annotations_to_statement() {
        let source = r#"
        ---@class Foo
        ---@field bar string
        local f1 = {}

        ---@type Foo
        local f2 = {}
        "#;
        let ast = parse(source.unindent().as_str()).expect("parse failure");
        let (index, _) = AnnotationIndex::from_ast(&ast, source);
        let class_ann = index
            .class_hints
            .get(&4)
            .expect("annotation attached to statement");
        assert_eq!(class_ann.len(), 1);
        assert_eq!(class_ann[0], "Foo");

        let line_ann = index
            .by_line
            .get(&7)
            .expect("annotation attached to statement");
        assert_eq!(line_ann[0].usage, AnnotationUsage::Type);
        assert_eq!(line_ann[0].ty.raw, "Foo");
    }
}
