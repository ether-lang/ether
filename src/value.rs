// ============================================================================
// VALUES
// ============================================================================

use core::fmt;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Value {
  Int(i64),
  Float(f64),
  Bool(bool),
  String(String),
  List(Vec<Value>),
  Map(HashMap<String, Value>),
  Tensor {
    shape: Vec<usize>,
    data: Vec<f64>,
  },
  Range {
    start: i64,
    end: i64,
    inclusive: bool,
  },
  Error {
    exc_type: String,
    message: String,
  },
  Nil,
}

impl Value {
  pub fn type_name(&self) -> &str {
    match self {
      Value::Int(_) => "int",
      Value::Float(_) => "float",
      Value::Bool(_) => "bool",
      Value::String(_) => "string",
      Value::List(_) => "list",
      Value::Map(_) => "map",
      Value::Tensor { .. } => "tensor",
      Value::Range { .. } => "range",
      Value::Error { .. } => "Error",
      Value::Nil => "nil",
    }
  }

  pub fn to_key(&self) -> Option<String> {
    match self {
      Value::String(s) => Some(s.clone()),
      Value::Int(n) => Some(n.to_string()),
      Value::Float(n) => Some(n.to_string()),
      Value::Bool(n) => Some(n.to_string()),
      _ => None,
    }
  }
}

impl fmt::Display for Value {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Value::Int(n) => write!(f, "{}", n),
      Value::Float(n) => write!(f, "{}", n),
      Value::Bool(b) => write!(f, "{}", b),
      Value::String(s) => write!(f, "{}", s),
      Value::List(v) => {
        write!(f, "[")?;
        for (i, val) in v.iter().enumerate() {
          if i > 0 {
            write!(f, ", ")?;
          }
          write!(f, "{}", val)?;
        }
        write!(f, "]")
      }
      Value::Map(m) => {
        write!(f, "{{")?;
        for (i, (k, v)) in m.iter().enumerate() {
          if i > 0 {
            write!(f, ", ")?;
          }
          write!(f, "{}: {}", k, v)?;
        }
        write!(f, "}}")
      }
      Value::Tensor { shape, data } => {
        write!(f, "Tensor[{:?}]->{{", shape)?;
        for (i, val) in data.iter().take(5).enumerate() {
          if i > 0 {
            write!(f, ", ")?;
          }
          write!(f, "{:.4}", val)?;
        }
        if data.len() > 5 {
          write!(f, ", ...")?;
        }
        write!(f, "}}")
      }
      Value::Range {
        start,
        end,
        inclusive,
      } => {
        if *inclusive {
          write!(f, "{}..={}", start, end)
        } else {
          write!(f, "{}..{}", start, end)
        }
      }
      Value::Error { exc_type, message } => {
        write!(f, "{}: {}", exc_type, message)
      }
      Value::Nil => write!(f, "void"),
    }
  }
}
