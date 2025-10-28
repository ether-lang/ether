// ============================================================================
// TYPE SYSTEM
// ============================================================================

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
  Int,
  Float,
  Bool,
  String,
  Void,
  Tensor(Option<Vec<usize>>),
  List(Box<Type>),
  Map(Box<Type>, Box<Type>),
  Range,
  Function(Vec<Type>, Box<Type>),
  TypeVar(String),
}

impl fmt::Display for Type {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Type::Int => write!(f, "int"),
      Type::Float => write!(f, "float"),
      Type::Bool => write!(f, "bool"),
      Type::String => write!(f, "string"),
      Type::Void => write!(f, "void"),
      Type::Tensor(None) => write!(f, "Tensor[]"),
      Type::Tensor(Some(shape)) => write!(f, "Tensor[{:?}]", shape),
      Type::List(t) => write!(f, "List<{}>", t),
      Type::Map(k, v) => write!(f, "Map<{}, {}>", k, v),
      Type::Range => write!(f, "Range"),
      Type::Function(params, ret) => {
        write!(f, "(")?;
        for (i, p) in params.iter().enumerate() {
          if i > 0 {
            write!(f, ", ")?;
          }
          write!(f, "{}", p)?;
        }
        write!(f, ") -> {}", ret)
      }
      Type::TypeVar(name) => write!(f, "'{}", name),
    }
  }
}
