// ============================================================================
// COMPILER
// ============================================================================

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
  ast::{BinOp, Expr, Pattern, Stmt, UnOp},
  instruction::{Instruction, OpCode},
  value::{ClassDef, MethodDef, Value},
};

pub struct Compiler {
  instructions: Vec<Instruction>,
  constants: Vec<Value>,
  var_indices: HashMap<String, usize>,
  next_var_index: usize,
  function_addresses: HashMap<String, usize>,
  classes: HashMap<String, Rc<ClassDef>>,
  current_class: Option<String>,
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
      classes: HashMap::new(),
      current_class: None,
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
      classes: HashMap::new(),
      current_class: None,
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
        // Check if it's a field assignment (self.field = value)
        // This is handled through normal assignment for now
        self.compile_expr(value)?;
        let idx = self.get_var_index(name);
        self.emit(OpCode::StoreVar, idx as i32);
      }
      Stmt::FieldAssign {
        object,
        field,
        value,
      } => {
        self.compile_expr(object)?; // Push object (e.g., self)
        let field_const = self.add_constant(Value::String(field.clone()));
        self.emit(OpCode::LoadConst, field_const as i32); // Push field name
        self.compile_expr(value)?; // Push value
        self.emit(OpCode::SetField, 0);
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

        let const_idx = self.add_constant(Value::Nil);
        self.emit(OpCode::LoadConst, const_idx as i32);
        self.emit(OpCode::Return, 0);

        let end_addr = self.current_address();
        self.instructions[jump_addr].arg = end_addr as i32;
      }
      Stmt::Return { value } => {
        if let Some(v) = value {
          self.compile_expr(v)?;
        } else {
          let const_idx = self.add_constant(Value::Nil);
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
        error_type,
        message,
      } => {
        self.compile_expr(message)?;
        let type_const = self.add_constant(Value::String(error_type.clone()));
        self.emit(OpCode::LoadConst, type_const as i32);
        self.emit(OpCode::Raise, 1); // arg=1 signals custom error
      }
      Stmt::Class {
        name,
        parents,
        methods,
        fields,
      } => {
        let mut class_def = ClassDef::new(name.clone());

        // Resolve parent classes
        for parent_name in parents {
          if let Some(parent_class) = self.classes.get(parent_name) {
            class_def.parents.push(Rc::clone(parent_class));
          } else {
            return Err(format!("Parent class '{}' not found", parent_name));
          }
        }

        // Add fields
        for (field_name, default_expr, is_private) in fields {
          let default_value = if let Some(_expr) = default_expr {
            // Compile and evaluate the default expression
            // For simplicity, we'll just use Nil for now
            // You could extend this to evaluate constant expressions
            None
          } else {
            None
          };
          class_def
            .fields
            .push((field_name.clone(), default_value, *is_private));
        }

        let old_class = self.current_class.clone();
        self.current_class = Some(name.clone());

        // Compile methods
        for (method_name, params, body, _return_type, is_static, is_private) in methods {
          let jump_addr = self.current_address();
          self.emit(OpCode::Jump, 0);

          let method_addr = self.current_address();

          // Store parameters in REVERSE order (top to bottom)
          for (param_name, _) in params.iter().rev() {
            let idx = self.get_var_index(param_name);
            self.emit(OpCode::StoreVar, idx as i32);
          }

          // Store self LAST (it's at the bottom of the stack)
          if !is_static {
            let self_idx = self.get_var_index("self");
            self.emit(OpCode::StoreVar, self_idx as i32);
          }

          // Compile method body
          for stmt in body {
            self.compile_stmt(stmt)?;
          }

          // Default return
          let const_idx = self.add_constant(Value::Nil);
          self.emit(OpCode::LoadConst, const_idx as i32);
          self.emit(OpCode::Return, 0);

          let end_addr = self.current_address();
          self.instructions[jump_addr].arg = end_addr as i32;

          let method_def = MethodDef {
            name: method_name.clone(),
            params: params.iter().map(|(n, _)| n.clone()).collect(),
            address: method_addr,
            is_private: *is_private,
          };

          if *is_static {
            class_def
              .static_methods
              .insert(method_name.clone(), method_def);
          } else {
            class_def.methods.insert(method_name.clone(), method_def);
          }
        }

        self.current_class = old_class;

        let class_rc = Rc::new(class_def);
        self.classes.insert(name.clone(), Rc::clone(&class_rc));

        // Store class as a constant and bind to variable
        let const_idx = self.add_constant(Value::Class(class_rc));
        self.emit(OpCode::LoadConst, const_idx as i32);
        let var_idx = self.get_var_index(name);
        self.emit(OpCode::StoreVar, var_idx as i32);
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
          self.emit(OpCode::Nil, 0);
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
        let idx = self.add_constant(Value::List(Rc::new(RefCell::new(
          shape.iter().map(|&s| Value::Int(s as i64)).collect(),
        ))));
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
        // Duplicate the match value for comparison (we need it for each case)
        self.emit(OpCode::Dup, 0);
        
        // Compile pattern matching
        match &case.pattern {
            Pattern::Wildcard => {
                // Pop the duplicated value (wildcard always matches)
                self.emit(OpCode::Pop, 0);
                let idx = self.add_constant(Value::Bool(true));
                self.emit(OpCode::LoadConst, idx as i32);
            }
            Pattern::Literal(expr) => {
                self.compile_expr(expr)?;
                self.emit(OpCode::Eq, 0);
            }
            Pattern::Ident(name) => {
                // Bind to variable
                let var_idx = self.get_var_index(name);
                self.emit(OpCode::StoreVar, var_idx as i32);
                let idx = self.add_constant(Value::Bool(true));
                self.emit(OpCode::LoadConst, idx as i32);
            }
            _ => {
                self.emit(OpCode::Pop, 0);
                let idx = self.add_constant(Value::Bool(false));
                self.emit(OpCode::LoadConst, idx as i32);
            }
        }
        
        let next_case_jump = self.current_address();
        self.emit(OpCode::JumpIfFalse, 0);
        
        // Pop the match value since we've matched
        self.emit(OpCode::Pop, 0);
        
        // Compile case body - it must leave exactly one value on stack
        let mut case_leaves_value = false;
        
        for (i, stmt) in case.body.iter().enumerate() {
            let is_last = i == case.body.len() - 1;
            
            match stmt {
                Stmt::Return { value } => {
                    // Return from the function, not just the match
                    if let Some(v) = value {
                        self.compile_expr(v)?;
                    } else {
                        let const_idx = self.add_constant(Value::Nil);
                        self.emit(OpCode::LoadConst, const_idx as i32);
                    }
                    self.emit(OpCode::Return, 0);
                    case_leaves_value = true;
                }
                Stmt::Expr(expr) if is_last => {
                    // Last expression in case body becomes the result
                    self.compile_expr(expr)?;
                    case_leaves_value = true;
                }
                _ => {
                    self.compile_stmt(stmt)?;
                }
            }
        }
        
        // If case body didn't leave a value, push Void
        if !case_leaves_value {
            let const_idx = self.add_constant(Value::Nil);
            self.emit(OpCode::LoadConst, const_idx as i32);
        }
        
        // Jump to end (skip other cases)
        let end_jump = self.current_address();
        self.emit(OpCode::Jump, 0);
        end_jumps.push(end_jump);
        
        // Patch the "next case" jump
        let next_case_addr = self.current_address();
        self.instructions[next_case_jump].arg = next_case_addr as i32;
    }
    
    // If no case matched, pop the value and push Void
    self.emit(OpCode::Pop, 0);
    let const_idx = self.add_constant(Value::Nil);
    self.emit(OpCode::LoadConst, const_idx as i32);
    
    // Patch all end jumps
    let end_addr = self.current_address();
    for jump in end_jumps {
        self.instructions[jump].arg = end_addr as i32;
    }
    
    self.emit(OpCode::MatchEnd, 0);
}
      Expr::MemberAccess { object, member } => {
        self.compile_expr(object)?;
        let member_const = self.add_constant(Value::String(member.clone()));
        self.emit(OpCode::LoadConst, member_const as i32);
        self.emit(OpCode::GetField, 0);
      }
      Expr::New { class_name, args } => {
        let class_info = if let Some(class_def) = self.classes.get(class_name) {
          let class_rc = Rc::clone(class_def);
          let init_addr = class_def.find_method("new").map(|m| m.address);
          Some((class_rc, init_addr))
        } else {
          None
        };

        if let Some((class_def, init_addr)) = class_info {
          // Push class
          let const_idx = self.add_constant(Value::Class(class_def));
          self.emit(OpCode::LoadConst, const_idx as i32);

          // Create instance (leaves instance on stack)
          self.emit(OpCode::NewInstance, 0);

          // If constructor exists
          if let Some(addr) = init_addr {
            // Duplicate the instance (we need it twice: once for init, once for result)
            // Add a Dup opcode
            self.emit(OpCode::Dup, 0);

            // Push arguments
            for arg in args {
              self.compile_expr(arg)?;
            }

            // Call constructor (this consumes the duplicated instance)
            self.emit(OpCode::Call, addr as i32);

            // Pop constructor's return value
            self.emit(OpCode::Pop, 0);

            // Original instance is still on stack
          }
        } else {
          return Err(format!("Class '{}' not found", class_name));
        }
      }
      Expr::SelfExpr => {
        let idx = self.get_var_index("self");
        self.emit(OpCode::LoadVar, idx as i32);
      }
      Expr::MethodCall {
        object,
        method,
        args,
      } => {
        // Compile object (this will be 'self' for the method)
        self.compile_expr(object)?;

        // Push arguments
        for arg in args {
          self.compile_expr(arg)?;
        }

        // Push method name
        let method_const = self.add_constant(Value::String(method.clone()));
        self.emit(OpCode::LoadConst, method_const as i32);

        self.emit(OpCode::CallMethod, args.len() as i32);
      }
      Expr::SuperCall { method, args } => {
        // Load self
        let self_idx = self.get_var_index("self");
        self.emit(OpCode::LoadVar, self_idx as i32);

        // Push arguments
        for arg in args {
          self.compile_expr(arg)?;
        }

        // Push method name and current class name
        let method_const = self.add_constant(Value::String(method.clone()));
        self.emit(OpCode::LoadConst, method_const as i32);

        if let Some(current_class) = &self.current_class {
          let class_const = self.add_constant(Value::String(current_class.clone()));
          self.emit(OpCode::LoadConst, class_const as i32);
        }

        self.emit(OpCode::CallSuper, args.len() as i32);
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
