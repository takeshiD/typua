use full_moon::ast;
use std::{borrow::Cow, collections::HashMap};

use super::types::{
    AnnotatedType, Annotation, AnnotationIndex, AnnotationUsage, ClassDeclaration, FunctionParam,
    FunctionType, TypeKind, TypeRegistry,
};

use full_moon::tokenizer::{Lexer, LexerResult, Token, TokenType};

#[derive(Debug)]
struct AliasSegment {
    raw: String,
    kind: Option<TypeKind>,
}

#[derive(Debug)]
struct AliasDeclaration {
    name: String,
    initial_segment: Option<AliasSegment>,
    comment: Option<String>,
}

#[derive(Debug)]
struct PendingAlias {
    name: String,
    comment: Option<String>,
    segments: Vec<AliasSegment>,
}

impl PendingAlias {
    fn new(name: String, comment: Option<String>) -> Self {
        Self {
            name,
            comment,
            segments: Vec::new(),
        }
    }

    fn push_segment(&mut self, segment: AliasSegment) {
        self.segments.push(segment);
    }

    fn build(self) -> Option<(String, AnnotatedType)> {
        let PendingAlias {
            name,
            comment,
            segments,
        } = self;

        if segments.is_empty() {
            return None;
        }

        let mut raw_parts: Vec<String> = Vec::new();
        let mut kinds: Vec<TypeKind> = Vec::new();
        for segment in segments {
            raw_parts.push(segment.raw);
            if let Some(kind) = segment.kind {
                kinds.push(kind);
            }
        }

        let raw = if raw_parts.len() == 1 {
            raw_parts.into_iter().next().unwrap()
        } else {
            raw_parts.join(" | ")
        };

        let kind = if kinds.is_empty() {
            None
        } else if kinds.len() == 1 {
            Some(kinds.pop().unwrap())
        } else {
            Some(make_union(kinds))
        };

        let ty = AnnotatedType::with_comment(raw, kind, comment);
        Some((name, ty))
    }
}

fn normalize_alias_comment(comment: Option<String>) -> Option<String> {
    comment.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        let without_hash = if let Some(rest) = trimmed.strip_prefix('#') {
            rest.trim_start()
        } else {
            trimmed
        };
        if without_hash.is_empty() {
            None
        } else {
            Some(without_hash.to_string())
        }
    })
}

fn parse_alias_declaration(line: &str) -> Option<AliasDeclaration> {
    let rest = match_annotation(line, "alias")?;
    let trimmed = rest.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut iter = trimmed.splitn(2, char::is_whitespace);
    let name = iter.next()?.trim();
    if name.is_empty() {
        return None;
    }

    let remainder = iter.next().unwrap_or("").trim();
    if remainder.is_empty() {
        return Some(AliasDeclaration {
            name: name.to_string(),
            initial_segment: None,
            comment: None,
        });
    }

    if remainder.starts_with('#') {
        let comment = normalize_alias_comment(Some(remainder.to_string()));
        return Some(AliasDeclaration {
            name: name.to_string(),
            initial_segment: None,
            comment,
        });
    }

    let (raw, kind, comment) = split_type_and_comment(remainder);
    let comment = normalize_alias_comment(comment);
    let initial_segment = if raw.is_empty() {
        None
    } else {
        Some(AliasSegment { raw, kind })
    };

    Some(AliasDeclaration {
        name: name.to_string(),
        initial_segment,
        comment,
    })
}

fn parse_alias_variant(line: &str) -> Option<AliasSegment> {
    let trimmed = line.trim_start();
    let mut stripped = trimmed.trim_start_matches('-');
    stripped = stripped.trim_start();
    let remainder = stripped.strip_prefix('|')?.trim();
    if remainder.is_empty() {
        return None;
    }
    let (raw, kind, _) = split_type_and_comment(remainder);
    if raw.is_empty() {
        return None;
    }
    Some(AliasSegment { raw, kind })
}

fn finalize_pending_alias(pending: &mut Option<PendingAlias>, registry: &mut TypeRegistry) {
    if let Some(alias) = pending.take()
        && let Some((name, ty)) = alias.build()
    {
        registry.register_alias(name, ty);
    }
}

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
        let mut pending_alias: Option<PendingAlias> = None;

        for (idx, line) in source.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim_start();

            if let Some(segment) = parse_alias_variant(trimmed) {
                if let Some(alias) = pending_alias.as_mut() {
                    alias.push_segment(segment);
                }
                continue;
            }

            if let Some(alias_decl) = parse_alias_declaration(trimmed) {
                finalize_pending_alias(&mut pending_alias, &mut registry);
                let AliasDeclaration {
                    name,
                    initial_segment,
                    comment,
                } = alias_decl;
                let mut alias = PendingAlias::new(name, comment);
                if let Some(segment) = initial_segment {
                    alias.push_segment(segment);
                }
                pending_alias = Some(alias);
                continue;
            }

            let stripped = trimmed.trim_start_matches('-').trim_start();
            if trimmed.is_empty() || (trimmed.starts_with("--") && !stripped.starts_with('@')) {
                continue;
            }

            finalize_pending_alias(&mut pending_alias, &mut registry);

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

        finalize_pending_alias(&mut pending_alias, &mut registry);

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
    let mut pending_alias: Option<PendingAlias> = None;
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
            let normalized_str = normalized.as_ref();

            if let Some(segment) = parse_alias_variant(normalized_str) {
                if let Some(alias) = pending_alias.as_mut() {
                    alias.push_segment(segment);
                }
                continue;
            }

            if let Some(alias_decl) = parse_alias_declaration(normalized_str) {
                finalize_pending_alias(&mut pending_alias, &mut registry);
                let AliasDeclaration {
                    name,
                    initial_segment,
                    comment,
                } = alias_decl;
                let mut alias = PendingAlias::new(name, comment);
                if let Some(segment) = initial_segment {
                    alias.push_segment(segment);
                }
                pending_alias = Some(alias);
                continue;
            }

            let stripped = normalized_str.trim_start_matches('-').trim_start();
            if normalized_str.trim_start().starts_with("--") && !stripped.starts_with('@') {
                continue;
            }

            finalize_pending_alias(&mut pending_alias, &mut registry);

            if let Some(decl) = parse_class_declaration(normalized_str) {
                pending_classes.push(decl.name.clone());
                current_class = Some(decl.name.clone());
                registry.register_class(decl);
                continue;
            }

            if let Some(name) = parse_enum_declaration(normalized_str) {
                registry.register_enum(&name);
                current_class = None;
                pending_classes.clear();
                continue;
            }

            if let Some((field_name, field_ty)) = parse_field_declaration(normalized_str) {
                if let Some(class_name) = current_class.clone() {
                    registry.register_field(&class_name, &field_name, field_ty);
                }
                continue;
            }

            if let Some(annotation) = parse_annotation(normalized_str) {
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

        finalize_pending_alias(&mut pending_alias, &mut registry);

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

    finalize_pending_alias(&mut pending_alias, &mut registry);

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

    if let Some(rest) = match_annotation(line, "param") {
        let trimmed = rest.trim();
        if trimmed.is_empty() {
            let ty = AnnotatedType::with_comment("any".to_string(), parse_type("any"), None);
            return Some(Annotation {
                usage: AnnotationUsage::Param,
                name: None,
                ty,
            });
        }

        let mut iter = trimmed.splitn(2, char::is_whitespace);
        let name_part = iter.next()?.trim();
        let rest = iter.next().unwrap_or("");
        let (type_raw, type_kind, comment) = split_type_and_comment(rest);
        let ty = AnnotatedType::with_comment(type_raw, type_kind, comment);
        return Some(Annotation {
            usage: AnnotationUsage::Param,
            name: Some(name_part.to_string()),
            ty,
        });
    }

    if let Some(rest) = match_annotation(line, "return") {
        let trimmed = rest.trim();
        if trimmed.is_empty() {
            let ty = AnnotatedType::with_comment("any".to_string(), parse_type("any"), None);
            return Some(Annotation {
                usage: AnnotationUsage::Return,
                name: None,
                ty,
            });
        }

        if split_top_level(trimmed, ',').len() > 1 {
            return Some(Annotation {
                usage: AnnotationUsage::Return,
                name: None,
                ty: AnnotatedType::with_comment(trimmed.to_string(), None, None),
            });
        }

        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        let (type_token, name) = if tokens.len() == 1 {
            (trimmed, None)
        } else if tokens.len() == 2
            && tokens[1]
                .chars()
                .all(|ch| ch == '_' || ch.is_alphanumeric())
        {
            (tokens[0], Some(tokens[1].to_string()))
        } else {
            (trimmed, None)
        };
        let ty = AnnotatedType::with_comment(type_token.to_string(), parse_type(type_token), None);
        return Some(Annotation {
            usage: AnnotationUsage::Return,
            name,
            ty,
        });
    }

    if let Some(rest) =
        match_annotation(line, "generics").or_else(|| match_annotation(line, "generic"))
    {
        let trimmed = rest.trim();
        if trimmed.is_empty() {
            return None;
        }

        return Some(Annotation {
            usage: AnnotationUsage::Generic,
            name: Some(trimmed.to_string()),
            ty: AnnotatedType::with_comment("any".to_string(), None, None),
        });
    }
    None
}

fn match_annotation<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix('@') {
        return rest.strip_prefix(key);
    }
    let stripped = trimmed.trim_start_matches('-');
    if stripped.len() == trimmed.len() {
        return None;
    }
    let stripped = stripped.trim_start();
    let rest = stripped.strip_prefix('@')?;
    rest.strip_prefix(key)
}

fn split_type_and_comment(input: &str) -> (String, Option<TypeKind>, Option<String>) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        let raw = "any".to_string();
        return (raw.clone(), parse_type(&raw), None);
    }

    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    for split in (1..parts.len()).rev() {
        let candidate = parts[..split].join(" ");
        if let Some(kind) = parse_type(&candidate) {
            if candidate.contains('#') {
                continue;
            }
            if candidate.trim_end().ends_with(':') {
                continue;
            }
            if candidate.contains(' ') && matches!(kind, TypeKind::Custom(_)) {
                continue;
            }
            let comment = parts[split..].join(" ");
            let comment = if comment.is_empty() {
                None
            } else {
                Some(comment)
            };
            return (candidate, Some(kind), comment);
        }
    }

    (trimmed.to_string(), parse_type(trimmed), None)
}

pub(crate) fn parse_type(raw: &str) -> Option<TypeKind> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(stripped) = trimmed.strip_suffix('?') {
        let base_type = parse_type(stripped.trim())?;
        return Some(make_union(vec![base_type, TypeKind::Nil]));
    }

    let (base_str, array_depth) = strip_array_suffixes(trimmed);
    let mut ty = parse_type_non_array(base_str)?;
    for _ in 0..array_depth {
        ty = TypeKind::Array(Box::new(ty));
    }
    Some(ty)
}

fn parse_type_non_array(raw: &str) -> Option<TypeKind> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.starts_with("fun(") || trimmed.starts_with("fun<") {
        return parse_function_type(trimmed);
    }

    if trimmed.starts_with('{')
        && trimmed.ends_with('}')
        && let Some((k, v)) = parse_dictionary_type(trimmed)
    {
        return Some(TypeKind::Applied {
            base: Box::new(TypeKind::Custom("table".to_string())),
            args: vec![k, v],
        });
    }

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

    if let Some((base, args)) = parse_applied_type(trimmed) {
        return Some(TypeKind::Applied {
            base: Box::new(base),
            args,
        });
    }

    if let Some(inner) = strip_enclosing_parens(trimmed) {
        return parse_type(inner);
    }

    let union_parts = split_top_level(trimmed, '|');
    if union_parts.len() > 1 {
        let mut members = Vec::new();
        for part in union_parts {
            members.push(parse_type(part)?);
        }
        return Some(make_union(members));
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

fn strip_array_suffixes(raw: &str) -> (&str, usize) {
    let mut depth = 0usize;
    let mut current = raw.trim_end();
    while let Some(stripped) = current.strip_suffix("[]") {
        depth += 1;
        current = stripped.trim_end();
    }
    (current.trim(), depth)
}

fn strip_enclosing_parens(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
        return None;
    }

    let mut depth = 0i32;
    for (idx, ch) in trimmed.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
                if depth == 0 && idx != trimmed.len() - 1 {
                    return None;
                }
            }
            _ => {}
        }
    }

    if depth == 0 {
        Some(&trimmed[1..trimmed.len() - 1])
    } else {
        None
    }
}

pub(crate) fn split_top_level(input: &str, delimiter: char) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut depth_paren = 0i32;
    let mut depth_brace = 0i32;
    let mut depth_bracket = 0i32;
    let mut depth_angle = 0i32;
    let mut start = 0usize;
    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => depth_paren += 1,
            ')' => depth_paren -= 1,
            '{' => depth_brace += 1,
            '}' => depth_brace -= 1,
            '[' => depth_bracket += 1,
            ']' => depth_bracket -= 1,
            '<' => depth_angle += 1,
            '>' => depth_angle -= 1,
            c if c == delimiter
                && depth_paren == 0
                && depth_brace == 0
                && depth_bracket == 0
                && depth_angle == 0 =>
            {
                let segment = input[start..idx].trim();
                if !segment.is_empty() {
                    segments.push(segment);
                }
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    let trailing = input[start..].trim();
    if !trailing.is_empty() {
        segments.push(trailing);
    }
    segments
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
    let rest = match_annotation(line, "class")?.trim();
    let (rest, exact) = if let Some(remaining) = rest.strip_prefix("(exact)") {
        (remaining.trim(), true)
    } else {
        (rest, false)
    };

    let mut parts = rest.splitn(2, ':');
    let signature = parts.next()?.trim();
    if signature.is_empty() {
        return None;
    }

    let mut name = signature;
    let mut generics = Vec::new();
    if let Some(start) = signature.find('<')
        && signature.ends_with('>')
    {
        name = signature[..start].trim();
        let generic_part = &signature[start + 1..signature.len() - 1];
        generics = generic_part
            .split(',')
            .map(|g| g.trim())
            .filter(|g| !g.is_empty())
            .map(|g| g.to_string())
            .collect();
    }

    let parent = parts
        .next()
        .and_then(|value| value.split_whitespace().next())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    Some(ClassDeclaration {
        name: name.to_string(),
        exact,
        parent,
        generics,
    })
}

pub(crate) fn parse_enum_declaration(line: &str) -> Option<String> {
    let rest = match_annotation(line, "enum")?.trim();
    let name = rest.split_whitespace().next()?;
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

pub(crate) fn parse_field_declaration(line: &str) -> Option<(String, AnnotatedType)> {
    let rest = match_annotation(line, "field")?.trim();
    let mut iter = rest.splitn(2, char::is_whitespace);
    let name = iter.next()?.trim();
    let remaining = iter.next().unwrap_or("any");
    let (type_raw, type_kind, comment) = split_type_and_comment(remaining);
    let ty = AnnotatedType::with_comment(type_raw, type_kind, comment);
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
                    kind: Some(TypeKind::Number),
                    comment: None,
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
                    kind: Some(make_union(vec![TypeKind::Number, TypeKind::Nil])),
                    comment: None,
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
                    kind: Some(make_union(vec![TypeKind::Number, TypeKind::String])),
                    comment: None,
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
                    kind: Some(TypeKind::Array(Box::new(TypeKind::Number))),
                    comment: None,
                }
            }
        );
        assert_eq!(
            parse_annotation("---@type (boolean|number)[]").unwrap(),
            Annotation {
                usage: AnnotationUsage::Type,
                name: None,
                ty: AnnotatedType {
                    raw: "(boolean|number)[]".to_string(),
                    kind: Some(TypeKind::Array(Box::new(make_union(vec![
                        TypeKind::Boolean,
                        TypeKind::Number,
                    ])))),
                    comment: None,
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
    fn param_annotation_captures_comment() {
        let annotation = parse_annotation("---@param id number this is userId").unwrap();
        assert_eq!(annotation.usage, AnnotationUsage::Param);
        assert_eq!(annotation.name.as_deref(), Some("id"));
        assert_eq!(annotation.ty.raw, "number");
        assert_eq!(annotation.ty.comment.as_deref(), Some("this is userId"));
    }

    #[test]
    fn field_annotation_captures_comment_with_spacing() {
        let (name, ty) =
            parse_field_declaration("---   @field id string this is container id").unwrap();
        assert_eq!(name, "id");
        assert_eq!(ty.raw, "string");
        assert_eq!(ty.comment.as_deref(), Some("this is container id"));
    }

    #[test]
    fn class_declaration_allows_spaced_prefix() {
        let decl = parse_class_declaration("---   @class Container").unwrap();
        assert_eq!(decl.name, "Container");
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
        assert!(field.comment.is_none());
    }

    #[test]
    fn from_ast_allows_comments_between_class_and_fields() {
        let source = r#"
        --- @class Container

        --- note about the class
        -- another comment

        ---@field id number
        --- Extra info about id
        ---@field info string detailed info
        local Container = {}
        "#;
        let ast = parse(source.unindent().as_str()).expect("parse failure");
        let (_, registry) = AnnotationIndex::from_ast(&ast, source);
        let id_field = registry
            .field_annotation("Container", "id")
            .expect("id field registered");
        assert_eq!(id_field.raw, "number");
        assert!(id_field.comment.is_none());
        let info_field = registry
            .field_annotation("Container", "info")
            .expect("info field registered");
        assert_eq!(info_field.raw, "string");
        assert_eq!(info_field.comment.as_deref(), Some("detailed info"));
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

    #[test]
    fn alias_single_line_registers_with_comment() {
        let source = r#"
        ---@alias userID integer The ID of a user
        "#;
        let ast = parse(source.unindent().as_str()).expect("parse failure");
        let (_, registry) = AnnotationIndex::from_ast(&ast, source);
        let alias = registry.alias("userID").expect("alias registered");
        assert_eq!(alias.raw, "integer");
        assert_eq!(alias.comment.as_deref(), Some("The ID of a user"));
        assert_eq!(registry.resolve("userID"), Some(TypeKind::Integer));
    }

    #[test]
    fn alias_multiline_union_registers() {
        let source = r#"
        ---@alias NumberOrString
        ---| number
        ---| string
        "#;
        let ast = parse(source.unindent().as_str()).expect("parse failure");
        let (_, registry) = AnnotationIndex::from_ast(&ast, source);
        let alias = registry.alias("NumberOrString").expect("alias registered");
        assert_eq!(alias.raw, "number | string");

        let resolved = registry.resolve("NumberOrString").expect("alias resolves");
        match resolved {
            TypeKind::Union(members) => {
                assert!(members.contains(&TypeKind::Number));
                assert!(members.contains(&TypeKind::String));
            }
            other => panic!("expected union, got {other:?}"),
        }
    }

    #[test]
    fn alias_resolves_nested_alias_references() {
        let source = r#"
        ---@alias UserID integer
        ---@alias UserIDList UserID[]
        "#;
        let ast = parse(source.unindent().as_str()).expect("parse failure");
        let (_, registry) = AnnotationIndex::from_ast(&ast, source);

        let resolved = registry.resolve("UserIDList").expect("alias resolves");
        match resolved {
            TypeKind::Array(inner) => assert_eq!(*inner, TypeKind::Integer),
            other => panic!("expected array alias, got {other:?}"),
        }
    }

    #[test]
    fn alias_comment_strips_hash_prefix() {
        let source = r#"
        ---@alias DeviceSide "left" # The left side
        "#;
        let ast = parse(source.unindent().as_str()).expect("parse failure");
        let (_, registry) = AnnotationIndex::from_ast(&ast, source);
        let alias = registry.alias("DeviceSide").expect("alias registered");
        assert_eq!(alias.comment.as_deref(), Some("The left side"));
        assert_eq!(registry.resolve("DeviceSide"), Some(TypeKind::String));
    }
}
