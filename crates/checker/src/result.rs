use typua_binder::TypeEnv;
use typua_ty::diagnostic::Diagnostic;

#[derive(Debug, Clone, Default)]
pub struct CheckResult {
    pub diagnostics: Vec<Diagnostic>,
    pub type_env: TypeEnv,
}
