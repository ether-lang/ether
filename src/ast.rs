// ============================================================================
// ABSTRACT SYNTAX TREE
// ============================================================================

use crate::types::Type;

#[derive(Debug, Clone)]
pub enum Stmt {
  Let {
    name: String,
    value: Box<Expr>,
    type_annotation: Option<Type>,
  },
  Assign {
    name: String,
    value: Box<Expr>,
  },
  IndexAssign {
    target: Box<Expr>,
    index: Box<Expr>,
    value: Box<Expr>,
  },
  Function {
    name: String,
    params: Vec<(String, Option<Type>)>,
    body: Vec<Stmt>,
    return_type: Option<Type>,
  },
  Return {
    value: Option<Box<Expr>>,
  },
  If {
    condition: Box<Expr>,
    then_block: Vec<Stmt>,
    else_block: Option<Vec<Stmt>>,
  },
  While {
    condition: Box<Expr>,
    body: Vec<Stmt>,
  },
  ForIn {
    var_name: String,
    iterable: Box<Expr>,
    body: Vec<Stmt>,
  },
  Try {
    try_block: Vec<Stmt>,
    catch_var: Option<String>,
    catch_block: Option<Vec<Stmt>>,
    finally_block: Option<Vec<Stmt>>,
  },
  Throw {
    value: Box<Expr>,
  },
  Raise {
    exception_type: String,
    message: Box<Expr>,
  },
  Expr(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum Expr {
  Binary {
    left: Box<Expr>,
    op: BinOp,
    right: Box<Expr>,
  },
  Unary {
    op: UnOp,
    operand: Box<Expr>,
  },
  Call {
    name: String,
    args: Vec<Expr>,
  },
  Index {
    target: Box<Expr>,
    index: Box<Expr>,
  },
  Slice {
    target: Box<Expr>,
    start: Option<Box<Expr>>,
    end: Option<Box<Expr>>,
  },
  Ident(String),
  IntLit(i64),
  FloatLit(f64),
  StringLit(String),
  BoolLit(bool),
  ListLit(Vec<Expr>),
  MapLit(Vec<(Expr, Expr)>),
  TensorLit {
    shape: Vec<usize>,
  },
  Range {
    start: Box<Expr>,
    end: Box<Expr>,
    inclusive: bool,
  },
  Match {
    value: Box<Expr>,
    cases: Vec<MatchCase>,
  },
}

#[derive(Debug, Clone)]
pub struct MatchCase {
  pub pattern: Pattern,
  pub guard: Option<Box<Expr>>,
  pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Pattern {
  Literal(Box<Expr>),
  Ident(String),
  List(Vec<Pattern>),
  Wildcard,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
  Add,
  Sub,
  Mul,
  Div,
  Mod,
  Eq,
  Neq,
  Lt,
  Gt,
  Lte,
  Gte,
  And,
  Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
  Neg,
  Not,
}
