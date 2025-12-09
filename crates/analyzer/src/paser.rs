use typua_parser::{ast::TypeAst, parse};

use crate::files::FileText;

#[tracked]
pub fn parse_query(db: &salsa::Database, file_text: FileText) -> (SemanticAst, SourceMap) {
    let text = file_text.text(&db);
    let parser = Parser::new(&db);
    let (sema_ast, source_map) = parser.parse(text);
}

pub struct Parser {
    db: &dyn salsa::Database,
}

impl Parser {
    fn new(db: &salsa::Database) -> Self {
        Self { db }
    }
    fn parse(&self, text: &str) -> (SemanticAst, SourceMap) {
        unimplemented!()
    }
}

#[salsa::tracked]
struct SemanticAst {}

#[salsa::tracked]
struct SourceMap {
    map: DashMap<NodeId, Range>,
}

#[salsa::interned]
struct NodeId {}
