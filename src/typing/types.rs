use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TyVarId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Prim {
    Nil,
    Any,
    Boolean,
    Number,
    Integer,
    String,
    Thread,
    UserData,
    LightUserData,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Prim(Prim),
    Var(TyVarId),
    Fun { params: Params, returns: Params },
    Tuple(Vec<Type>),
    Union(Vec<Type>),
    Optional(Box<Type>),
    Table(Table),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Params {
    Fixed(Vec<Type>),
    VarArg(Vec<Type>, Box<Type>), // fixed, vararg element type
}

#[derive(Debug, Clone, PartialEq)]
pub enum Table {
    Record {
        fields: BTreeMap<String, Type>,
        exact: bool,
    },
    Map {
        key: Box<Type>,
        value: Box<Type>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scheme {
    pub vars: BTreeSet<TyVarId>,
    pub body: Type,
}

impl Scheme {
    pub fn new(vars: impl IntoIterator<Item = TyVarId>, body: Type) -> Self {
        Self {
            vars: vars.into_iter().collect(),
            body,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Subst(pub HashMap<TyVarId, Type>);

impl Subst {
    pub fn get(&self, v: &TyVarId) -> Option<&Type> {
        self.0.get(v)
    }
    pub fn insert(&mut self, v: TyVarId, t: Type) {
        self.0.insert(v, t);
    }
    pub fn compose(&self, other: &Subst) -> Subst {
        // self after other: S2 ∘ S1  ≡  apply S2 to S1, then merge S2
        let mut m = other.0.clone();
        for (k, v) in m.iter_mut() {
            *v = apply(self, v.clone());
        }
        for (k, v) in self.0.iter() {
            m.insert(*k, v.clone());
        }
        Subst(m)
    }
}

pub fn ftv(t: &Type) -> BTreeSet<TyVarId> {
    match t {
        Type::Prim(_) => BTreeSet::new(),
        Type::Var(v) => [*v].into_iter().collect(),
        Type::Fun { params, returns } => ftv_params(params)
            .into_iter()
            .chain(ftv_params(returns))
            .collect(),
        Type::Tuple(vs) => vs.iter().flat_map(ftv).collect(),
        Type::Union(us) => us.iter().flat_map(ftv).collect(),
        Type::Optional(t) => ftv(t),
        Type::Table(Table::Record { fields, .. }) => fields.values().flat_map(ftv).collect(),
        Type::Table(Table::Map { key, value }) => ftv(key).into_iter().chain(ftv(value)).collect(),
    }
}

fn ftv_params(p: &Params) -> BTreeSet<TyVarId> {
    match p {
        Params::Fixed(vs) => vs.iter().flat_map(ftv).collect(),
        Params::VarArg(vs, v) => vs.iter().flat_map(ftv).chain(ftv(v)).collect(),
    }
}

pub fn ftv_scheme(s: &Scheme) -> BTreeSet<TyVarId> {
    let mut set = ftv(&s.body);
    for v in &s.vars {
        set.remove(v);
    }
    set
}

pub fn ftv_env(env: &BTreeMap<String, Scheme>) -> BTreeSet<TyVarId> {
    env.values().flat_map(ftv_scheme).collect()
}

pub fn generalize(env: &BTreeMap<String, Scheme>, t: Type) -> Scheme {
    let env_ftv = ftv_env(env);
    let t_ftv = ftv(&t);
    let vars: BTreeSet<_> = t_ftv.difference(&env_ftv).cloned().collect();
    Scheme { vars, body: t }
}

pub fn instantiate(next_id: &mut impl FnMut() -> TyVarId, s: &Scheme) -> Type {
    let mut m: HashMap<TyVarId, Type> = HashMap::new();
    for v in &s.vars {
        m.insert(*v, Type::Var(next_id()));
    }
    apply(&Subst(m), s.body.clone())
}

pub fn apply(s: &Subst, t: Type) -> Type {
    match t {
        Type::Prim(_) => t,
        Type::Var(v) => s.0.get(&v).cloned().unwrap_or(Type::Var(v)),
        Type::Fun { params, returns } => Type::Fun {
            params: apply_params(s, params),
            returns: apply_params(s, returns),
        },
        Type::Tuple(vs) => Type::Tuple(vs.into_iter().map(|t| apply(s, t)).collect()),
        Type::Union(us) => Type::Union(us.into_iter().map(|t| apply(s, t)).collect()),
        Type::Optional(t1) => Type::Optional(Box::new(apply(s, *t1))),
        Type::Table(Table::Record { fields, exact }) => {
            let mut nf = BTreeMap::new();
            for (k, v) in fields.into_iter() {
                nf.insert(k, apply(s, v));
            }
            Type::Table(Table::Record { fields: nf, exact })
        }
        Type::Table(Table::Map { key, value }) => Type::Table(Table::Map {
            key: Box::new(apply(s, *key)),
            value: Box::new(apply(s, *value)),
        }),
    }
}

fn apply_params(s: &Subst, p: Params) -> Params {
    match p {
        Params::Fixed(vs) => Params::Fixed(vs.into_iter().map(|t| apply(s, t)).collect()),
        Params::VarArg(vs, v) => Params::VarArg(
            vs.into_iter().map(|t| apply(s, t)).collect(),
            Box::new(apply(s, *v)),
        ),
    }
}

// Pretty printing
pub struct TypeDisplay<'a> {
    ty: &'a Type,
}

impl<'a> TypeDisplay<'a> {
    pub fn new(ty: &'a Type) -> Self {
        Self { ty }
    }
}

impl fmt::Display for TypeDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut names: HashMap<TyVarId, String> = HashMap::new();
        let mut next = 0usize;
        fn name_for(names: &mut HashMap<TyVarId, String>, next: &mut usize, v: TyVarId) -> String {
            if let Some(n) = names.get(&v) {
                return n.clone();
            }
            let s =
                ["T", "U", "V", "W", "X", "Y", "Z"][*next % 7].to_string() + &"'".repeat(*next / 7);
            *next += 1;
            names.insert(v, s.clone());
            s
        }
        fn go(
            f: &mut fmt::Formatter<'_>,
            t: &Type,
            names: &mut HashMap<TyVarId, String>,
            next: &mut usize,
        ) -> fmt::Result {
            match t {
                Type::Prim(p) => write!(
                    f,
                    "{}",
                    match p {
                        Prim::Nil => "nil",
                        Prim::Any => "any",
                        Prim::Boolean => "boolean",
                        Prim::Number => "number",
                        Prim::Integer => "integer",
                        Prim::String => "string",
                        Prim::Thread => "thread",
                        Prim::UserData => "userdata",
                        Prim::LightUserData => "lightuserdata",
                    }
                ),
                Type::Var(v) => write!(f, "{}", name_for(names, next, *v)),
                Type::Optional(t1) => {
                    go(f, t1, names, next)?;
                    write!(f, "?")
                }
                Type::Tuple(vs) => {
                    write!(f, "[")?;
                    for (i, t) in vs.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        go(f, t, names, next)?;
                    }
                    write!(f, "]")
                }
                Type::Union(us) => {
                    for (i, t) in us.iter().enumerate() {
                        if i > 0 {
                            write!(f, " | ")?;
                        }
                        go(f, t, names, next)?;
                    }
                    Ok(())
                }
                Type::Fun { params, returns } => {
                    write!(f, "fun(")?;
                    match params {
                        Params::Fixed(vs) => {
                            for (i, t) in vs.iter().enumerate() {
                                if i > 0 {
                                    write!(f, ", ")?;
                                }
                                go(f, t, names, next)?;
                            }
                        }
                        Params::VarArg(vs, v) => {
                            for (i, t) in vs.iter().enumerate() {
                                if i > 0 {
                                    write!(f, ", ")?;
                                }
                                go(f, t, names, next)?;
                            }
                            if !vs.is_empty() {
                                write!(f, ", ")?;
                            }
                            go(f, v, names, next)?;
                            write!(f, "...")?;
                        }
                    }
                    write!(f, ")")?;
                    match returns {
                        Params::Fixed(vs) if !vs.is_empty() => {
                            write!(f, ": ")?;
                            if vs.len() == 1 {
                                go(f, &vs[0], names, next)
                            } else {
                                write!(f, "(")?;
                                for (i, t) in vs.iter().enumerate() {
                                    if i > 0 {
                                        write!(f, ", ")?;
                                    }
                                    go(f, t, names, next)?;
                                }
                                write!(f, ")")
                            }
                        }
                        Params::VarArg(vs, v) => {
                            write!(f, ": (")?;
                            for (i, t) in vs.iter().enumerate() {
                                if i > 0 {
                                    write!(f, ", ")?;
                                }
                                go(f, t, names, next)?;
                            }
                            if !vs.is_empty() {
                                write!(f, ", ")?;
                            }
                            go(f, v, names, next)?;
                            write!(f, "...)")
                        }
                        _ => Ok(()),
                    }
                }
                Type::Table(Table::Record { fields, exact }) => {
                    write!(f, "{{ ")?;
                    let mut first = true;
                    for (k, v) in fields.iter() {
                        if !first {
                            write!(f, ", ")?;
                        }
                        first = false;
                        write!(f, "{}: ", k)?;
                        go(f, v, names, next)?;
                    }
                    write!(f, " }}")?;
                    if *exact {
                        write!(f, " (exact)")?;
                    }
                    Ok(())
                }
                Type::Table(Table::Map { key, value }) => {
                    write!(f, "{{ [")?;
                    go(f, key, names, next)?;
                    write!(f, "]: ")?;
                    go(f, value, names, next)?;
                    write!(f, " }}")
                }
            }
        }
        go(f, self.ty, &mut names, &mut next)
    }
}

// Helpers to build types succinctly in tests
impl Prim {
    pub fn as_type(self) -> Type {
        Type::Prim(self)
    }
}

pub fn union(mut items: Vec<Type>) -> Type {
    // normalize: flatten, dedup by string form, sort
    let mut flat: Vec<Type> = Vec::new();
    for t in items.drain(..) {
        match t {
            Type::Optional(inner) => flat.push(Type::Union(vec![*inner, Type::Prim(Prim::Nil)])),
            Type::Union(us) => flat.extend(us),
            other => flat.push(other),
        }
    }
    // string key for dedup; in future consider hashing canonical form
    let mut seen: BTreeMap<String, Type> = BTreeMap::new();
    for t in flat.into_iter() {
        let key = format!("{}", TypeDisplay::new(&t));
        seen.entry(key).or_insert(t);
    }
    Type::Union(seen.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn pretty_union_optional() {
        let t = union(vec![
            Prim::String.as_type(),
            Type::Optional(Box::new(Prim::Number.as_type())),
        ]);
        let s = format!("{}", TypeDisplay::new(&t));
        assert!(s.contains("string"));
        assert!(s.contains("number | nil"));
    }
}
