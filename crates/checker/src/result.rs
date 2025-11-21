use typua_binder::TypeEnv;
use typua_ty::diagnostic::Diagnostic;
use typua_ty::typeinfo::TypeInfo;

#[derive(Debug, Clone, Default)]
pub struct CheckResult {
    pub diagnostics: Vec<Diagnostic>,
    pub type_env: TypeEnv,
    pub type_infos: Vec<TypeInfo>,
}
