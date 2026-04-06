mod cst;
mod error;
mod span;
use crate::cst::Cst;
use crate::error::SyntaxError;
use crate::span::{Position, Span};

use typua_config::LuaVersion;

/// entry point for parsing lua script
pub fn parse(code: &str, lua_version: LuaVersion) -> (Cst, Vec<SyntaxError>) {
    let result = full_moon::parse_fallible(code, lua_version.into());
    let errors: Vec<SyntaxError> = result
        .errors()
        .iter()
        .map(|e| {
            let (start, end) = e.range();
            SyntaxError::new(
                Span::new(Position::from(start), Position::from(end)),
                format!("{}", e),
            )
        })
        .collect();
    (Cst::from(result.ast().clone()), errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::annotation::{AnnotationInfo, AnnotationTag};
    use crate::cst::{Expression, LocalAssign, Stmt, Variable};
    use crate::span::{Position, Span};
    use pretty_assertions::assert_eq;
    use typua_types::TypeKind;
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
                    span: Span::new(Position::new(1, 7), Position::new(1, 8)),
                }],
                exprs: vec![Expression::Number {
                    span: Span::new(Position::new(1, 11), Position::new(1, 13)),
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
                    span: Span::new(Position::new(2, 7), Position::new(2, 8)),
                }],
                exprs: vec![Expression::Number {
                    span: Span::new(Position::new(2, 11), Position::new(2, 13)),
                    val: "12".to_string(),
                }],
                annotates: vec![AnnotationInfo {
                    tag: AnnotationTag::Type(TypeKind::Number),
                    span: Span::new(Position::new(1, 10), Position::new(1, 16)),
                }],
            })]
        );
    }
}
