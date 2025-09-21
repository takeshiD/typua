use typua::typing::types as T;
use typua::typing::unify as U;

use pretty_assertions::assert_eq;

fn v(id: u32) -> T::Type { T::Type::Var(T::TyVarId(id)) }

#[test]
fn ftv_and_generalize() {
    use T::Prim::*;
    let mut env = std::collections::BTreeMap::new();
    env.insert("x".into(), T::Scheme::new([T::TyVarId(0)], T::Type::Prim(Number)));
    let t = T::Type::Fun { params: T::Params::Fixed(vec![v(1)]), returns: T::Params::Fixed(vec![v(1)]) };
    let s = T::generalize(&env, t.clone());
    // v1 should be generalized
    assert!(s.vars.contains(&T::TyVarId(1)));
    // instantiate produces fresh var
    let mut next = 100u32;
    let mut fresh = || { let id = T::TyVarId(next); next += 1; id };
    let t2 = T::instantiate(&mut fresh, &s);
    let vars = T::ftv(&t2);
    assert!(vars.contains(&T::TyVarId(100)));
}

#[test]
fn unify_simple() {
    let s = T::Subst::default();
    let a = v(0);
    let b = v(1);
    let s = U::unify(s, a, b).unwrap();
    assert!(s.get(&T::TyVarId(0)).is_some() || s.get(&T::TyVarId(1)).is_some());
}
