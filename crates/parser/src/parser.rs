use typua_config::LuaVersion;

use crate::ast::TypeAst;
use crate::error::TypuaError;

/// entry point for parsing lua script
pub fn parse(code: &str, lua_version: LuaVersion) -> (TypeAst, Vec<TypuaError>) {
    match lua_version {
        LuaVersion::Lua51 => {
            let result = full_moon::parse_fallible(code, full_moon::LuaVersion::lua51());
            (
                TypeAst::from(result.ast().clone()),
                result
                    .errors()
                    .iter()
                    .map(|e| TypuaError::SyntaxFalied { source: e.clone() })
                    .collect(),
            )
        }
    }
}

#[cfg(test)]
mod convert_from_fullmoon {
    use super::*;
    use crate::ast::{Expression, LocalAssign, LuaNumber, Stmt, Variable};
    use crate::span::{Position, Span};
    use pretty_assertions::assert_eq;
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
                        end: Position::new(1, 7),
                    }
                }],
                exprs: vec![Expression::Number(LuaNumber {
                    span: Span {
                        start: Position::new(1, 11),
                        end: Position::new(1, 12),
                    }
                })],
                annotates: Vec::new(),
            })]
        );
    }
}
