#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct FileId {
    id: i32,
}

#[derive(Debug, Clone)]
pub struct FileText {
    text: String,
}
