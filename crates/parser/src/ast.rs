#[derive(Debug, Clone, PartialEq)]
pub struct TypeAst {
    pub block: Block,
}

impl From<full_moon::ast::Block> for TypeAst {
    fn from(block: &full_moon::ast::Block) -> Self {
        for stmt in block.stmts() {

        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

/// Statements
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Stmt {
    Assign(Assign),
    LocalAssign(LocalAssign),
    FunctionCall(FunctionCall),
    FunctionDeclaration(FunctionDeclaration),
    LocalFunction(LocalFunction),
    If(If),
    Do(Do),
    While(While),
    Repeat(Repeat),
    Goto(Goto),
    NumericFor(NumericFor),
    GenericFor(GenericFor),
    Label(Label),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assign {}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalAssign {}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalFunction {}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCall {}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDeclaration {}

#[derive(Debug, Clone, PartialEq)]
pub struct If {}

#[derive(Debug, Clone, PartialEq)]
pub struct Do {}

#[derive(Debug, Clone, PartialEq)]
pub struct While {}

#[derive(Debug, Clone, PartialEq)]
pub struct Repeat {}

#[derive(Debug, Clone, PartialEq)]
pub struct Goto {}

#[derive(Debug, Clone, PartialEq)]
pub struct NumericFor {}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericFor {}

#[derive(Debug, Clone, PartialEq)]
pub struct Label {}


/// Expression
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    BinaryOperator {lhs: Box<Expression>, binop: BinOp, rhs: Box<Expression>},
    UnaryOperator {unop: UnOp, expr: Box<Expression>},
    Function(Box<AnonymousFunction>),
    FunctionCall(FunctionCall),
    Number(LuaNumber),
    String(LuaString),
    Symbol(Symbol),
    Var(Var)
}


#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
    GreaterThan,
    GreaterThanEqual,
    LessThan,
    LessThanEqual,
    Equal,
    NotEqual,
    Concat,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnOp {
    Minus,
    Not,
    Hash,
    Tilde,
}

