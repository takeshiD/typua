use typua_config::LuaVersion;

use crate::ast::TypeAst;
use crate::diagnostic::Diagnostic;
use typua_span::{Position, Span};

#[salsa::input(debug)]
struct SourceFile {
    #[returns(ref)]
    text: String,
}

#[salsa::tracked(debug)]
struct ParseResult<'db> {
    #[tracked]
    #[returns(ref)]
    type_ast: TypeAst,
    #[tracked]
    #[returns(ref)]
    diagnostics: Vec<Diagnostic>,
}

#[salsa::tracked]
pub fn parse(db: &dyn salsa::Database, file: SourceFile) -> ParseResult<'_> {
    let code = file.text(db);
    let (ast, diag) = _parse(code, LuaVersion::Lua51);
    ParseResult::new(db, ast, diag)
}

/// entry point for parsing lua script
fn _parse(code: &str, lua_version: LuaVersion) -> (TypeAst, Vec<Diagnostic>) {
    let result = full_moon::parse_fallible(code, lua_version.into());
    (
        TypeAst::from(result.ast().clone()),
        result
            .errors()
            .iter()
            .map(|e| {
                let (start, end) = e.range().clone();
                Diagnostic::new(
                    Span::new(Position::from(start), Position::from(end)),
                    format!("{}", e),
                )
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::{AnnotationInfo, AnnotationTag};
    use crate::ast::{Expression, LocalAssign, Stmt, Variable};
    use pretty_assertions::assert_eq;
    use typua_span::{Position, Span};
    use typua_ty::TypeKind;
    use unindent::unindent;
    #[test]
    fn local_assign() {
        let code = unindent(
            r#"
        local x = 12
        "#,
        );
        let (ast, _) = parse(code.as_str(), LuaVersion::Lua51);
        assert_eq!(
            ast.block.stmts,
            vec![Stmt::LocalAssign(LocalAssign {
                vars: vec![Variable {
                    name: "x".to_string(),
                    span: Span {
                        start: Position::new(1, 7),
                        end: Position::new(1, 8),
                    }
                }],
                exprs: vec![Expression::Number {
                    span: Span {
                        start: Position::new(1, 11),
                        end: Position::new(1, 13),
                    },
                    val: "12".to_string(),
                }],
                annotates: Vec::new(),
            })]
        );
        let code = unindent(
            r#"
        ---@type number
        local x = 12
        "#,
        );
        let (ast, _) = parse(code.as_str(), LuaVersion::Lua51);
        assert_eq!(
            ast.block.stmts,
            vec![Stmt::LocalAssign(LocalAssign {
                vars: vec![Variable {
                    name: "x".to_string(),
                    span: Span {
                        start: Position::new(2, 7),
                        end: Position::new(2, 8),
                    }
                }],
                exprs: vec![Expression::Number {
                    span: Span {
                        start: Position::new(2, 11),
                        end: Position::new(2, 13),
                    },
                    val: "12".to_string(),
                }],
                annotates: vec![AnnotationInfo {
                    tag: AnnotationTag::Type(TypeKind::Number),
                    span: Span {
                        start: Position::new(1, 10),
                        end: Position::new(1, 16),
                    }
                }],
            })]
        );
    }
}
