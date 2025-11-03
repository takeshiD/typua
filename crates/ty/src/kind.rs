use std::fmt::write;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Any,
    Never,
    Nil,
    Number,
    Boolean,
    String,
    Table,
    Function {
        params: Vec<TypeKind>,
        returns: Vec<TypeKind>,
    },
    Class,
    Generic(String),
    Union(Vec<TypeKind>),
    Array(Box<TypeKind>),
    Dict {
        key: Box<TypeKind>,
        val: Box<TypeKind>,
    },
    KVTable {
        key: Box<TypeKind>,
        val: Box<TypeKind>,
    },
}

impl std::fmt::Display for TypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            TypeKind::Any => "any".to_string(),
            TypeKind::Never => "never".to_string(),
            TypeKind::Nil => "nil".to_string(),
            TypeKind::Number => "number".to_string(),
            TypeKind::Boolean => "boolean".to_string(),
            TypeKind::String => "string".to_string(),
            TypeKind::Table => "table".to_string(),
            TypeKind::Function { params, returns } => {
                let params_string: Vec<String> = params.iter().map(|ty| ty.to_string()).collect();
                let returns_string: Vec<String> = returns.iter().map(|ty| ty.to_string()).collect();
                format!(
                    "fun({})->{}",
                    params_string.join(","),
                    returns_string.join(",")
                )
            }
            TypeKind::Class => "class".to_string(),
            TypeKind::Generic(s) => s.clone(),
            TypeKind::Union(types) => {
                let types_string: Vec<String> = types.iter().map(|ty| ty.to_string()).collect();
                types_string.join("|")
            }
            TypeKind::Array(ty) => {
                format!("{}[]", ty)
            }
            TypeKind::Dict { key, val } => {
                format!("{{ [{}]: {} }}", key, val)
            }
            TypeKind::KVTable { key, val } => {
                format!("table<{}, {}>", key, val)
            }
        };
        write!(f, "{}", s)
    }
}
