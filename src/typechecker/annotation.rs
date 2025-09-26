use std::collections::HashMap;

use super::types::{
    AnnotatedType, Annotation, AnnotationIndex, AnnotationUsage, ClassDeclaration, TypeKind,
    TypeRegistry,
};

impl AnnotationIndex {
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

    if let Some(stripped) = trimmed.strip_suffix('?') {
        let base_type = parse_type(stripped.trim())?;
        return Some(make_union(vec![base_type, TypeKind::Nil]));
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
    use pretty_assertions::assert_eq;

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
}
