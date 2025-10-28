// ============================================================================
// COMPILER
// ============================================================================

use core::fmt;
use std::collections::HashMap;

use crate::ast::{BinOp, Expr, Pattern, Stmt, UnOp};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum OpCode {
  LoadConst,
  LoadVar,
  StoreVar,
  Add,
  Sub,
  Mul,
  Div,
  Mod,
  Pow,
  Floor,
  Neg,
  Eq,
  Neq,
  Lt,
  Gt,
  Lte,
  Gte,
  And,
  Or,
  Not,
  Jump,
  JumpIfFalse,
  Call,
  Return,
  TensorCreate,
  MatMul,
  Relu,
  Sigmoid,
  Tanh,
  Softmax,
  BuildList,
  BuildMap,
  Print,
  Pop,
  Halt,
  Raise,
  SetupTry,
  PopTry,
  BeginFinally,
  EndFinally,
  AssertType,
  // New opcodes
  Index,
  IndexSet,
  Slice,
  BuildRange,
  SetupForIn,
  ForInNext,
  PopForIn,
  MatchBegin,
  MatchCase,
  MatchEnd,
}

#[derive(Debug, Clone)]
pub struct Instruction {
  pub opcode: OpCode,
  pub arg: i32,
}

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
  Exception {
    exc_type: String,
    message: String,
  },
  Void,
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
      Value::Tensor { .. } => "Tensor",
      Value::Range { .. } => "Range",
      Value::Exception { .. } => "Exception",
      Value::Void => "void",
    }
  }

  pub fn to_key(&self) -> Option<String> {
    match self {
      Value::String(s) => Some(s.clone()),
      Value::Int(n) => Some(n.to_string()),
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
        write!(f, "Tensor{:?}: [", shape)?;
        for (i, val) in data.iter().take(5).enumerate() {
          if i > 0 {
            write!(f, ", ")?;
          }
          write!(f, "{:.4}", val)?;
        }
        if data.len() > 5 {
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
      Value::Exception { exc_type, message } => {
        write!(f, "{}: {}", exc_type, message)
      }
      Value::Void => write!(f, "void"),
    }
  }
}

pub struct Compiler {
  instructions: Vec<Instruction>,
  constants: Vec<Value>,
  var_indices: HashMap<String, usize>,
  next_var_index: usize,
  function_addresses: HashMap<String, usize>,
  is_repl: bool,
}

impl Compiler {
  pub fn new() -> Self {
    Compiler {
      instructions: Vec::new(),
      constants: Vec::new(),
      var_indices: HashMap::new(),
      next_var_index: 0,
      function_addresses: HashMap::new(),
      is_repl: false,
    }
  }

  pub fn new_repl() -> Self {
    Compiler {
      instructions: Vec::new(),
      constants: Vec::new(),
      var_indices: HashMap::new(),
      next_var_index: 0,
      function_addresses: HashMap::new(),
      is_repl: true,
    }
  }

  fn add_constant(&mut self, value: Value) -> usize {
    for (i, c) in self.constants.iter().enumerate() {
      let matches = match (&value, c) {
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::String(a), Value::String(b)) => a == b,
        _ => false,
      };
      if matches {
        return i;
      }
    }
    self.constants.push(value);
    self.constants.len() - 1
  }

  fn get_var_index(&mut self, name: &str) -> usize {
    if let Some(&idx) = self.var_indices.get(name) {
      idx
    } else {
      let idx = self.next_var_index;
      self.var_indices.insert(name.to_string(), idx);
      self.next_var_index += 1;
      idx
    }
  }

  fn emit(&mut self, opcode: OpCode, arg: i32) {
    self.instructions.push(Instruction { opcode, arg });
  }

  fn current_address(&self) -> usize {
    self.instructions.len()
  }

  pub fn compile(&mut self, statements: &[Stmt]) -> Result<(), String> {
    for stmt in statements {
      self.compile_stmt(stmt)?;
    }
    self.emit(OpCode::Halt, 0);
    Ok(())
  }

  fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
    match stmt {
      Stmt::Let { name, value, .. } => {
        self.compile_expr(value)?;
        let idx = self.get_var_index(name);
        self.emit(OpCode::StoreVar, idx as i32);
      }
      Stmt::Assign { name, value } => {
        self.compile_expr(value)?;
        let idx = self.get_var_index(name);
        self.emit(OpCode::StoreVar, idx as i32);
      }
      Stmt::IndexAssign {
        target,
        index,
        value,
      } => {
        self.compile_expr(target)?;
        self.compile_expr(index)?;
        self.compile_expr(value)?;
        self.emit(OpCode::IndexSet, 0);
      }
      Stmt::Function {
        name, params, body, ..
      } => {
        let jump_addr = self.current_address();
        self.emit(OpCode::Jump, 0);

        let func_addr = self.current_address();
        self.function_addresses.insert(name.clone(), func_addr);

        for (param_name, param_type) in params.iter().rev() {
          if let Some(expected_type) = param_type {
            let type_const = self.add_constant(Value::String(format!("{}", expected_type)));
            self.emit(OpCode::LoadConst, type_const as i32);
            self.emit(OpCode::AssertType, 0);
          }

          let idx = self.get_var_index(param_name);
          self.emit(OpCode::StoreVar, idx as i32);
        }

        for stmt in body {
          self.compile_stmt(stmt)?;
        }

        let const_idx = self.add_constant(Value::Void);
        self.emit(OpCode::LoadConst, const_idx as i32);
        self.emit(OpCode::Return, 0);

        let end_addr = self.current_address();
        self.instructions[jump_addr].arg = end_addr as i32;
      }
      Stmt::Return { value } => {
        if let Some(v) = value {
          self.compile_expr(v)?;
        } else {
          let const_idx = self.add_constant(Value::Void);
          self.emit(OpCode::LoadConst, const_idx as i32);
        }
        self.emit(OpCode::Return, 0);
      }
      Stmt::If {
        condition,
        then_block,
        else_block,
      } => {
        self.compile_expr(condition)?;

        let else_jump = self.current_address();
        self.emit(OpCode::JumpIfFalse, 0);

        for stmt in then_block {
          self.compile_stmt(stmt)?;
        }

        let end_jump = self.current_address();
        self.emit(OpCode::Jump, 0);

        let else_addr = self.current_address();
        self.instructions[else_jump].arg = else_addr as i32;

        if let Some(else_stmts) = else_block {
          for stmt in else_stmts {
            self.compile_stmt(stmt)?;
          }
        }

        let end_addr = self.current_address();
        self.instructions[end_jump].arg = end_addr as i32;
      }
      Stmt::While { condition, body } => {
        let start_addr = self.current_address();

        self.compile_expr(condition)?;

        let end_jump = self.current_address();
        self.emit(OpCode::JumpIfFalse, 0);

        for stmt in body {
          self.compile_stmt(stmt)?;
        }

        self.emit(OpCode::Jump, start_addr as i32);

        let end_addr = self.current_address();
        self.instructions[end_jump].arg = end_addr as i32;
      }
      Stmt::ForIn {
        var_name,
        iterable,
        body,
      } => {
        self.compile_expr(iterable)?;

        let var_idx = self.get_var_index(var_name);
        self.emit(OpCode::SetupForIn, var_idx as i32);

        let loop_start = self.current_address();
        let end_jump = self.current_address();
        self.emit(OpCode::ForInNext, 0);

        for stmt in body {
          self.compile_stmt(stmt)?;
        }

        self.emit(OpCode::Jump, loop_start as i32);

        let end_addr = self.current_address();
        self.instructions[end_jump].arg = end_addr as i32;

        self.emit(OpCode::PopForIn, 0);
      }
      Stmt::Try {
        try_block,
        catch_var,
        catch_block,
        finally_block,
      } => {
        let catch_addr_placeholder = self.current_address();
        self.emit(OpCode::SetupTry, 0);

        for stmt in try_block {
          self.compile_stmt(stmt)?;
        }

        self.emit(OpCode::PopTry, 0);

        let finally_jump = self.current_address();
        self.emit(OpCode::Jump, 0);

        let catch_addr = self.current_address();
        if let Some(block) = catch_block {
          if let Some(var_name) = catch_var {
            let idx = self.get_var_index(var_name);
            self.emit(OpCode::StoreVar, idx as i32);
          } else {
            self.emit(OpCode::Pop, 0);
          }

          for stmt in block {
            self.compile_stmt(stmt)?;
          }
        } else {
          self.emit(OpCode::Pop, 0);
        }

        self.instructions[catch_addr_placeholder].arg = catch_addr as i32;

        let finally_addr = self.current_address();
        self.instructions[finally_jump].arg = finally_addr as i32;

        if let Some(block) = finally_block {
          self.emit(OpCode::BeginFinally, 0);
          for stmt in block {
            self.compile_stmt(stmt)?;
          }
          self.emit(OpCode::EndFinally, 0);
        }
      }
      Stmt::Throw { value } => {
        self.compile_expr(value)?;
        self.emit(OpCode::Raise, 0);
      }
      Stmt::Raise {
        exception_type,
        message,
      } => {
        self.compile_expr(message)?;
        let type_const = self.add_constant(Value::String(exception_type.clone()));
        self.emit(OpCode::LoadConst, type_const as i32);
        self.emit(OpCode::Raise, 1); // arg=1 signals custom exception
      }
      Stmt::Expr(expr) => {
        self.compile_expr(expr)?;
        if !self.is_repl {
          self.emit(OpCode::Pop, 0);
        }
      }
    }
    Ok(())
  }

  fn compile_expr(&mut self, expr: &Expr) -> Result<(), String> {
    match expr {
      Expr::IntLit(n) => {
        let idx = self.add_constant(Value::Int(*n));
        self.emit(OpCode::LoadConst, idx as i32);
      }
      Expr::FloatLit(n) => {
        let idx = self.add_constant(Value::Float(*n));
        self.emit(OpCode::LoadConst, idx as i32);
      }
      Expr::StringLit(s) => {
        let idx = self.add_constant(Value::String(s.clone()));
        self.emit(OpCode::LoadConst, idx as i32);
      }
      Expr::BoolLit(b) => {
        let idx = self.add_constant(Value::Bool(*b));
        self.emit(OpCode::LoadConst, idx as i32);
      }
      Expr::Ident(name) => {
        let idx = self.get_var_index(name);
        self.emit(OpCode::LoadVar, idx as i32);
      }
      Expr::Binary { left, op, right } => {
        self.compile_expr(left)?;
        self.compile_expr(right)?;

        let opcode = match op {
          BinOp::Add => OpCode::Add,
          BinOp::Sub => OpCode::Sub,
          BinOp::Mul => OpCode::Mul,
          BinOp::Div => OpCode::Div,
          BinOp::Mod => OpCode::Mod,
          BinOp::Pow => OpCode::Pow,
          BinOp::Floor => OpCode::Floor,
          BinOp::Eq => OpCode::Eq,
          BinOp::Neq => OpCode::Neq,
          BinOp::Lt => OpCode::Lt,
          BinOp::Gt => OpCode::Gt,
          BinOp::Lte => OpCode::Lte,
          BinOp::Gte => OpCode::Gte,
          BinOp::And => OpCode::And,
          BinOp::Or => OpCode::Or,
        };
        self.emit(opcode, 0);
      }
      Expr::Unary { op, operand } => {
        self.compile_expr(operand)?;
        let opcode = match op {
          UnOp::Neg => OpCode::Neg,
          UnOp::Not => OpCode::Not,
        };
        self.emit(opcode, 0);
      }
      Expr::Call { name, args } => match name.as_str() {
        "print" => {
          for arg in args {
            self.compile_expr(arg)?;
            self.emit(OpCode::Print, 0);
          }
          let idx = self.add_constant(Value::Void);
          self.emit(OpCode::LoadConst, idx as i32);
        }
        "matmul" | "relu" | "sigmoid" | "tanh" | "softmax" => {
          for arg in args {
            self.compile_expr(arg)?;
          }
          let opcode = match name.as_str() {
            "matmul" => OpCode::MatMul,
            "relu" => OpCode::Relu,
            "sigmoid" => OpCode::Sigmoid,
            "tanh" => OpCode::Tanh,
            "softmax" => OpCode::Softmax,
            _ => unreachable!(),
          };
          self.emit(opcode, 0);
        }
        _ => {
          for arg in args {
            self.compile_expr(arg)?;
          }
          if let Some(&addr) = self.function_addresses.get(name) {
            self.emit(OpCode::Call, addr as i32);
          } else {
            return Err(format!("Undefined function: {}", name));
          }
        }
      },
      Expr::Index { target, index } => {
        self.compile_expr(target)?;
        self.compile_expr(index)?;
        self.emit(OpCode::Index, 0);
      }
      Expr::Slice { target, start, end } => {
        self.compile_expr(target)?;

        if let Some(s) = start {
          self.compile_expr(s)?;
        } else {
          let idx = self.add_constant(Value::Int(0));
          self.emit(OpCode::LoadConst, idx as i32);
        }

        if let Some(e) = end {
          self.compile_expr(e)?;
        } else {
          let idx = self.add_constant(Value::Int(-1));
          self.emit(OpCode::LoadConst, idx as i32);
        }

        self.emit(OpCode::Slice, 0);
      }
      Expr::ListLit(elements) => {
        for elem in elements {
          self.compile_expr(elem)?;
        }
        self.emit(OpCode::BuildList, elements.len() as i32);
      }
      Expr::MapLit(pairs) => {
        for (k, v) in pairs {
          self.compile_expr(k)?;
          self.compile_expr(v)?;
        }
        self.emit(OpCode::BuildMap, pairs.len() as i32);
      }
      Expr::TensorLit { shape } => {
        let idx = self.add_constant(Value::List(
          shape.iter().map(|&s| Value::Int(s as i64)).collect(),
        ));
        self.emit(OpCode::LoadConst, idx as i32);
        self.emit(OpCode::TensorCreate, 0);
      }
      Expr::Range {
        start,
        end,
        inclusive,
      } => {
        self.compile_expr(start)?;
        self.compile_expr(end)?;
        self.emit(OpCode::BuildRange, if *inclusive { 1 } else { 0 });
      }
      Expr::Match { value, cases } => {
        self.compile_expr(value)?;
        self.emit(OpCode::MatchBegin, 0);

        let mut end_jumps = Vec::new();

        for case in cases {
          // let case_start = self.current_address();
          self.emit(OpCode::MatchCase, 0); // Placeholder

          // Compile pattern matching logic (simplified)
          match &case.pattern {
            Pattern::Wildcard => {
              // Always matches
              let idx = self.add_constant(Value::Bool(true));
              self.emit(OpCode::LoadConst, idx as i32);
            }
            Pattern::Literal(expr) => {
              self.compile_expr(expr)?;
              self.emit(OpCode::Eq, 0);
            }
            Pattern::Ident(name) => {
              // Bind to variable and match
              let var_idx = self.get_var_index(name);
              self.emit(OpCode::StoreVar, var_idx as i32);
              let idx = self.add_constant(Value::Bool(true));
              self.emit(OpCode::LoadConst, idx as i32);
            }
            _ => {
              let idx = self.add_constant(Value::Bool(false));
              self.emit(OpCode::LoadConst, idx as i32);
            }
          }

          let next_case_jump = self.current_address();
          self.emit(OpCode::JumpIfFalse, 0);

          // Compile case body
          for stmt in &case.body {
            self.compile_stmt(stmt)?;
          }

          let end_jump = self.current_address();
          self.emit(OpCode::Jump, 0);
          end_jumps.push(end_jump);

          let next_case_addr = self.current_address();
          self.instructions[next_case_jump].arg = next_case_addr as i32;
        }

        let end_addr = self.current_address();
        for jump in end_jumps {
          self.instructions[jump].arg = end_addr as i32;
        }

        self.emit(OpCode::MatchEnd, 0);
      }
    }
    Ok(())
  }

  pub fn get_instructions(&self) -> &[Instruction] {
    &self.instructions
  }

  pub fn get_constants(&self) -> &[Value] {
    &self.constants
  }
}

pub fn compile(statements: &[Stmt]) -> Result<Compiler, String> {
  let mut compiler = Compiler::new();
  compiler.compile(statements)?;
  Ok(compiler)
}

pub fn compile_repl(statements: &[Stmt]) -> Result<Compiler, String> {
  let mut compiler = Compiler::new_repl();
  compiler.compile(statements)?;
  Ok(compiler)
}
