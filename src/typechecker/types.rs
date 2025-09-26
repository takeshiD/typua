use std::{cmp::Ordering, collections::HashMap, fmt};

use crate::{
    diagnostics::{Diagnostic, Severity},
    lsp::DocumentPosition,
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

#[derive(Clone, Debug)]
pub struct CheckResult {
    pub diagnostics: Vec<Diagnostic>,
    pub type_map: HashMap<DocumentPosition, TypeInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeKind {
    Unknown,
    Nil,
    Boolean,
    Number,
    Integer,
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

impl TypeKind {
    pub fn describe(&self) -> &'static str {
        match self {
            TypeKind::Unknown => "unknown",
            TypeKind::Nil => "nil",
            TypeKind::Boolean => "boolean",
            TypeKind::Number => "number",
            TypeKind::Integer => "integer",
            TypeKind::String => "string",
            TypeKind::Table => "table",
            TypeKind::Function => "function",
            TypeKind::Thread => "thread",
            TypeKind::Custom(_) => "custom",
            TypeKind::Union(_) => "union",
            TypeKind::Array(_) => "array",
            TypeKind::Generic(_) => "generic",
            TypeKind::Applied { .. } => "applied",
            TypeKind::FunctionSig(_) => "function",
        }
    }

    pub fn matches(&self, other: &TypeKind) -> bool {
        if matches!(self, TypeKind::Unknown) || matches!(other, TypeKind::Unknown) {
            return true;
        }

        match self {
            TypeKind::Union(types) => types.iter().any(|t| t.matches(other)),
            TypeKind::Custom(_) => match other {
                TypeKind::Union(types) => types.iter().any(|t| self.matches(t)),
                TypeKind::Table => true,
                _ => self == other,
            },
            TypeKind::Integer => match other {
                TypeKind::Union(types) => types.iter().any(|t| self.matches(t)),
                TypeKind::Number => true,
                _ => self == other,
            },
            TypeKind::Table => match other {
                TypeKind::Union(types) => types.iter().any(|t| self.matches(t)),
                TypeKind::Custom(_) => true,
                _ => self == other,
            },
            TypeKind::Number => match other {
                TypeKind::Union(types) => types.iter().any(|t| self.matches(t)),
                TypeKind::Integer => true,
                _ => self == other,
            },
            _ => match other {
                TypeKind::Union(types) => types.iter().any(|t| self.matches(t)),
                _ => self == other,
            },
        }
    }
}
impl fmt::Display for TypeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeKind::Custom(name) => f.write_str(name),
            TypeKind::Union(types) => {
                if types.is_empty() {
                    return f.write_str("unknown");
                }

                let mut rendered: Vec<(bool, String)> = types
                    .iter()
                    .map(|ty| (matches!(ty, TypeKind::Nil), ty.to_string()))
                    .collect();

                rendered.sort_by(|(is_nil_a, text_a), (is_nil_b, text_b)| {
                    match is_nil_a.cmp(is_nil_b) {
                        Ordering::Equal => text_a.cmp(text_b),
                        other => other,
                    }
                });

                for (index, (_, text)) in rendered.iter().enumerate() {
                    if index > 0 {
                        write!(f, "|{text}")?;
                    } else {
                        write!(f, "{text}")?;
                    }
                }
                Ok(())
            }
            _ => f.write_str(self.describe()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TypeInfo {
    pub ty: String,
    pub end_line: usize,
    pub end_character: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FunctionType {
    pub generics: Vec<String>,
    pub params: Vec<FunctionParam>,
    pub returns: Vec<TypeKind>,
    pub vararg: Option<Box<TypeKind>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionParam {
    pub name: Option<String>,
    pub ty: TypeKind,
    pub is_self: bool,
    pub is_vararg: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ClassInfo {
    pub exact: bool,
    pub parent: Option<String>,
    pub fields: HashMap<String, AnnotatedType>,
}

impl ClassInfo {
    pub fn new(exact: bool, parent: Option<String>) -> Self {
        Self {
            exact,
            parent,
            fields: HashMap::new(),
        }
    }
}

#[derive(Default, Clone)]
pub struct TypeRegistry {
    pub classes: HashMap<String, ClassInfo>,
    pub enums: HashMap<String, ()>,
}

impl TypeRegistry {
    pub fn register_class(&mut self, decl: ClassDeclaration) {
        let name = decl.name.clone();
        let entry = self
            .classes
            .entry(name)
            .or_insert_with(|| ClassInfo::new(decl.exact, decl.parent.clone()));
        entry.exact = decl.exact;
        entry.parent = decl.parent;
    }

    pub fn register_enum(&mut self, name: &str) {
        self.enums.insert(name.to_string(), ());
    }

    pub fn register_field(&mut self, class: &str, field: &str, ty: AnnotatedType) {
        let entry = self
            .classes
            .entry(class.to_string())
            .or_insert_with(|| ClassInfo::new(false, None));
        entry.fields.insert(field.to_string(), ty);
    }

    pub fn resolve(&self, name: &str) -> Option<TypeKind> {
        if self.classes.contains_key(name) {
            Some(TypeKind::Custom(name.to_string()))
        } else if self.enums.contains_key(name) {
            Some(TypeKind::String)
        } else {
            None
        }
    }

    pub fn field_annotation(&self, class: &str, field: &str) -> Option<&AnnotatedType> {
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

    pub fn is_exact(&self, class: &str) -> bool {
        self.classes
            .get(class)
            .map(|info| info.exact)
            .unwrap_or(false)
    }

    pub fn extend(&mut self, other: &TypeRegistry) {
        for (name, info) in &other.classes {
            let entry = self
                .classes
                .entry(name.clone())
                .or_insert_with(ClassInfo::default);
            entry.exact = info.exact;
            entry.parent = info.parent.clone();
            for (field, ty) in &info.fields {
                entry.fields.insert(field.clone(), ty.clone());
            }
        }

        for (name, ()) in &other.enums {
            self.enums.insert(name.clone(), ());
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnnotatedType {
    pub raw: String,
    pub kind: Option<TypeKind>,
}

impl AnnotatedType {
    pub fn new(raw: String, kind: Option<TypeKind>) -> Self {
        Self { raw, kind }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnotationUsage {
    Type,
    Param,
    Return,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Annotation {
    pub usage: AnnotationUsage,
    pub name: Option<String>,
    pub ty: AnnotatedType,
}

#[derive(Default, Clone)]
pub struct AnnotationIndex {
    pub by_line: HashMap<usize, Vec<Annotation>>,
    pub class_hints: HashMap<usize, Vec<String>>,
}

impl AnnotationIndex {
    pub fn take(&mut self, line: usize) -> Vec<Annotation> {
        self.by_line.remove(&line).unwrap_or_default()
    }

    pub fn take_class_hint(&mut self, line: usize) -> Vec<String> {
        self.class_hints.remove(&line).unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
pub struct ClassDeclaration {
    pub name: String,
    pub exact: bool,
    pub parent: Option<String>,
}

#[derive(Clone, Copy)]
pub enum OperandSide {
    Left,
    Right,
}

impl OperandSide {
    pub fn describe(self) -> &'static str {
        match self {
            OperandSide::Left => "left",
            OperandSide::Right => "right",
        }
    }
}
