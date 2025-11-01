// ============================================================================
// VALUES
// ============================================================================

use core::fmt;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::module::Module;

#[derive(Debug, Clone)]
pub struct ClassDef {
  pub name: String,
  pub parents: Vec<Rc<ClassDef>>,
  pub methods: HashMap<String, MethodDef>,
  pub static_methods: HashMap<String, MethodDef>,
  pub fields: Vec<(String, Option<Value>, bool)>, // (name, default, is_private)
}

#[derive(Debug, Clone)]
pub struct MethodDef {
  pub name: String,
  pub params: Vec<String>,
  pub address: usize,
  pub is_private: bool,
}

#[derive(Debug, Clone)]
pub struct FunctionDef {
  pub name: String,
  pub address: usize,
  pub module_id: Option<String>, // To identify which module it belongs to
}

#[derive(Debug, Clone)]
pub struct Instance {
  pub class: Rc<ClassDef>,
  pub fields: HashMap<String, Value>,
}

impl ClassDef {
  pub fn new(name: String) -> Self {
    ClassDef {
      name,
      parents: Vec::new(),
      methods: HashMap::new(),
      static_methods: HashMap::new(),
      fields: Vec::new(),
    }
  }

  pub fn find_method(&self, name: &str) -> Option<&MethodDef> {
    // Check own methods first
    if let Some(method) = self.methods.get(name) {
      return Some(method);
    }

    // Check parent classes (C3 linearization for multiple inheritance)
    for parent in &self.parents {
      if let Some(method) = parent.find_method(name) {
        return Some(method);
      }
    }

    None
  }

  pub fn is_private_accessible(&self, method_name: &str) -> bool {
    if let Some(method) = self.methods.get(method_name) {
      !method.is_private
    } else {
      true
    }
  }
}

impl Instance {
  pub fn new(class: Rc<ClassDef>) -> Self {
    let mut fields = HashMap::new();

    // Initialize fields with defaults
    for (field_name, default_value, _) in &class.fields {
      fields.insert(
        field_name.clone(),
        default_value.clone().unwrap_or(Value::Nil),
      );
    }

    Instance { class, fields }
  }

  pub fn get_field(&self, name: &str) -> Option<&Value> {
    self.fields.get(name)
  }

  pub fn set_field(&mut self, name: &str, value: Value) {
    self.fields.insert(name.to_string(), value);
  }
}

#[derive(Debug, Clone)]
pub enum Value {
  Int(i64),
  Float(f64),
  Bool(bool),
  String(String),
  List(Rc<RefCell<Vec<Value>>>),
  Map(Rc<RefCell<HashMap<String, Value>>>),
  Tensor {
    shape: Vec<usize>,
    data: Rc<RefCell<Vec<f64>>>,
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
  Class(Rc<ClassDef>),
  Instance(Rc<RefCell<Instance>>),
  Module(Rc<Module>),
  Function(Rc<FunctionDef>),
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
      Value::Instance(_) => "object",
      Value::Class(_) => "class",
      Value::Module(_) => "module",
      Value::Function(_) => "function",
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
        let list = v.borrow();
        write!(f, "[")?;
        for (i, val) in list.iter().enumerate() {
          if i > 0 {
            write!(f, ", ")?;
          }
          write!(f, "{}", val)?;
        }
        write!(f, "]")
      }
      Value::Map(map_ref) => {
        let map = map_ref.borrow();
        write!(f, "{{")?;
        for (i, (k, v)) in map.iter().enumerate() {
          if i > 0 {
            write!(f, ", ")?;
          }
          write!(f, "{}: {}", k, v)?;
        }
        write!(f, "}}")
      }
      Value::Tensor { shape, data } => {
        let data_vec = data.borrow();
        write!(f, "Tensor{:?}: [", shape)?;
        for (i, val) in data_vec.iter().take(5).enumerate() {
          if i > 0 {
            write!(f, ", ")?;
          }
          write!(f, "{:.4}", val)?;
        }
        if data_vec.len() > 5 {
          write!(f, ", ...")?;
        }
        write!(f, "]")
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
      Value::Class(class_def) => write!(f, "[class {}]", class_def.name),
      Value::Instance(instance_ref) => write!(f, "[object {}]", instance_ref.borrow().class.name),
      Value::Module(module) => write!(f, "[module '{}']", module.name),
      Value::Function(func_def) => write!(f, "[function {}]", func_def.name),
      Value::Nil => write!(f, "nil"),
    }
  }
}
