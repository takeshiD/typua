use typua_config::LuaVersion;
use typua_ty::{ParseError, TypuaError};

use crate::ast::TypeAst;

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
                    .map(|e| TypuaError::Parse(ParseError::SyntaxError(format!("{}", e))))
                    .collect(),
            )
        }
    }
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
                    }
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
                    }
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
