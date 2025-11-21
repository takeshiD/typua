use crate::TypeKind;
use tower_lsp::lsp_types::InlayHint;
use tower_lsp::lsp_types::{InlayHintLabel, Position as LspPosition};
use typua_span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct TypeInfo {
    pub ty: TypeKind,
    pub span: Span,
}

impl From<&TypeInfo> for InlayHint {
    fn from(type_info: &TypeInfo) -> InlayHint {
        InlayHint {
            position: LspPosition::from(type_info.span.end.clone()),
            label: InlayHintLabel::String(format!(": {}", type_info.ty)),
            kind: None,
            text_edits: None,
            tooltip: None,
            padding_left: None,
            padding_right: None,
            data: None,
        }
    }
}
