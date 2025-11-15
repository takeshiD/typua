use crate::{TypuaError, error::OperationError};

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Unknown, // top
    Never,   // bottom
    Any,
    Nil,
    Number,
    Boolean(BoolLiteral),
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

#[derive(Debug, Clone, PartialEq)]
pub enum BoolLiteral {
    True,
    False,
    Any,
}

impl TypeKind {
    /// sub_ty <: sup_ty
    ///   true  => sub_ty is subtype of sup_ty
    ///   false => sub_ty is not subtype of sup_ty
    pub fn subtype(sub_ty: &TypeKind, sup_ty: &TypeKind) -> bool {
        match sup_ty {
            TypeKind::Unknown => true,
            TypeKind::Never => sub_ty == sup_ty,
            TypeKind::Any => *sub_ty != TypeKind::Unknown,
            TypeKind::Nil => *sub_ty == TypeKind::Nil,
            TypeKind::Number => {
                matches!(
                    *sub_ty,
                    TypeKind::Number | TypeKind::Any | TypeKind::Unknown
                )
            }
            TypeKind::Boolean(_) => {
                matches!(
                    *sub_ty,
                    TypeKind::Boolean(_) | TypeKind::Any | TypeKind::Unknown
                )
            }
            TypeKind::String => {
                matches!(
                    *sub_ty,
                    TypeKind::String | TypeKind::Any | TypeKind::Unknown
                )
            }
            _ => unimplemented!(),
        }
    }
    pub fn try_add(sub_ty: &TypeKind, sup_ty: &TypeKind) -> Result<TypeKind, TypuaError> {
        match sup_ty {
            TypeKind::Any | TypeKind::Unknown => Ok(TypeKind::Any),
            TypeKind::Never => Err(TypuaError::Operation(OperationError::AddFailed(
                "never".to_string(),
            ))),
            TypeKind::Nil => Err(TypuaError::Operation(OperationError::AddFailed(
                "nil".to_string(),
            ))),
            TypeKind::Number => {
                if *sub_ty == TypeKind::Number {
                    Ok(TypeKind::Number)
                } else {
                    Err(TypuaError::Operation(OperationError::AddFailed(
                        "number".to_string(),
                    )))
                }
            }
            TypeKind::Boolean(_) => Err(TypuaError::Operation(OperationError::AddFailed(
                "boolean".to_string(),
            ))),
            TypeKind::String => Err(TypuaError::Operation(OperationError::AddFailed(
                "string".to_string(),
            ))),
            _ => unimplemented!(),
        }
    }
    pub fn try_sub(sub_ty: &TypeKind, sup_ty: &TypeKind) -> Result<TypeKind, TypuaError> {
        match sup_ty {
            TypeKind::Any | TypeKind::Unknown => Ok(TypeKind::Any),
            TypeKind::Never => Err(TypuaError::Operation(OperationError::SubFailed(
                "never".to_string(),
            ))),
            TypeKind::Nil => Err(TypuaError::Operation(OperationError::SubFailed(
                "nil".to_string(),
            ))),
            TypeKind::Number => {
                if *sub_ty == TypeKind::Number {
                    Ok(TypeKind::Number)
                } else {
                    Err(TypuaError::Operation(OperationError::SubFailed(
                        "number".to_string(),
                    )))
                }
            }
            TypeKind::Boolean(_) => Err(TypuaError::Operation(OperationError::SubFailed(
                "boolean".to_string(),
            ))),
            TypeKind::String => Err(TypuaError::Operation(OperationError::SubFailed(
                "string".to_string(),
            ))),
            _ => unimplemented!(),
        }
    }
    // `false` and `nil` are treated as `false`
    pub fn try_and(first_ty: &TypeKind, second_ty: &TypeKind) -> Result<TypeKind, TypuaError> {
        match first_ty {
            TypeKind::Nil => Ok(TypeKind::Nil),
            TypeKind::Boolean(b) => match b {
                BoolLiteral::True => Ok(second_ty.clone()),
                BoolLiteral::False => Ok(TypeKind::Boolean(BoolLiteral::False)),
                BoolLiteral::Any => Ok(TypeKind::Union(vec![
                    second_ty.clone(),
                    TypeKind::Boolean(BoolLiteral::False),
                ])),
            },
            _ => Ok(second_ty.clone()),
        }
    }
    // `false` and `nil` are treated as `false`
    pub fn try_or(first_ty: &TypeKind, second_ty: &TypeKind) -> Result<TypeKind, TypuaError> {
        match first_ty {
            TypeKind::Nil => Ok(second_ty.clone()),
            TypeKind::Boolean(b) => match b {
                BoolLiteral::True => Ok(TypeKind::Boolean(BoolLiteral::True)),
                BoolLiteral::False => Ok(second_ty.clone()),
                BoolLiteral::Any => Ok(TypeKind::Union(vec![
                    second_ty.clone(),
                    TypeKind::Boolean(BoolLiteral::True),
                ])),
            },
            _ => Ok(first_ty.clone()),
        }
    }
}

impl std::fmt::Display for TypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let s = match self {
            TypeKind::Unknown => "unknown".to_string(),
            TypeKind::Any => "any".to_string(),
            TypeKind::Never => "never".to_string(),
            TypeKind::Nil => "nil".to_string(),
            TypeKind::Number => "number".to_string(),
            TypeKind::Boolean(_) => "boolean".to_string(),
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
