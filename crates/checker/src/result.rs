use typua_ty::diagnostic::Diagnostic;

#[derive(Debug, Clone, Default)]
pub struct CheckResult {
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckResult {
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }
}
