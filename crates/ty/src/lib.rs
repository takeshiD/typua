pub mod kind;
pub mod error;
pub mod diagnostic;
pub mod typeinfo;

pub use kind::{TypeKind, BoolLiteral};
pub use error::{TypuaError, ParseError, AnnotationError, BindError};
