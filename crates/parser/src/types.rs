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
