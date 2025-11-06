// ============================================================================
// COMPILER
// ============================================================================

use std::{cell::RefCell, collections::HashMap, path::PathBuf, rc::Rc};

use crate::{
  ast::{BinOp, Expr, Pattern, Stmt, UnOp},
  instruction::{Instruction, OpCode},
  module::ModuleLoader,
  value::{ClassDef, FunctionDef, MethodDef, Value},
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum VarLocation {
  Local(usize),   // Local variable in current function
  Upvalue(usize), // Upvalue (captured from parent)
  Global(usize),  // Global variable
}

#[derive(Debug, Clone)]
struct Upvalue {
  index: usize,   // Index in parent's locals or upvalues
  is_local: bool, // True if captures from immediate parent's local
}

#[derive(Debug, Clone)]
struct FunctionCompiler {
  enclosing: Option<Box<FunctionCompiler>>,
  _function_name: String,
  locals: Vec<String>,    // Local variable names in order
  upvalues: Vec<Upvalue>, // Upvalues this function captures
  _scope_depth: usize,    // Current scope depth (for blocks)
}

pub struct Compiler {
  instructions: Vec<Instruction>,
  constants: Vec<Value>,
  pub function_addresses: HashMap<String, usize>,
  classes: HashMap<String, Rc<ClassDef>>,
  current_class: Option<String>,
  module_loader: Option<ModuleLoader>,
  current_file: PathBuf,
  current_function: Option<Box<FunctionCompiler>>,
  globals: HashMap<String, usize>, // Global variables
  next_global_index: usize,
  is_repl: bool,
}

impl Compiler {
  pub fn new() -> Self {
    Compiler {
      instructions: Vec::new(),
      constants: Vec::new(),
      function_addresses: HashMap::new(),
      classes: HashMap::new(),
      current_class: None,
      module_loader: None,
      current_file: PathBuf::from("."),
      current_function: None,
      globals: HashMap::new(),
      next_global_index: 0,
      is_repl: false,
    }
  }

  pub fn new_repl() -> Self {
    Compiler {
      instructions: Vec::new(),
      constants: Vec::new(),
      function_addresses: HashMap::new(),
      classes: HashMap::new(),
      current_class: None,
      module_loader: None,
      current_file: PathBuf::from("."),
      current_function: None,
      globals: HashMap::new(),
      next_global_index: 0,
      is_repl: true,
    }
  }

  pub fn set_module_loader(&mut self, loader: ModuleLoader) {
    self.module_loader = Some(loader);
  }

  pub fn set_current_file(&mut self, path: PathBuf) {
    self.current_file = path;
  }

  pub fn get_global_var_names(&self) -> HashMap<usize, String> {
    self.globals.iter().map(|(k, v)| (*v, k.clone())).collect()
  }

  fn get_var_index(&mut self, name: &str) -> usize {
    // For cases where we just need an index (like classes, etc.)
    // Check if local first
    if let Some(ref func) = self.current_function {
      for (i, local) in func.locals.iter().enumerate() {
        if local == name {
          return i;
        }
      }
    }

    // Otherwise treat as global
    if let Some(&idx) = self.globals.get(name) {
      return idx;
    }

    let idx = self.next_global_index;
    self.globals.insert(name.to_string(), idx);
    self.next_global_index += 1;
    idx
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

  fn resolve_local(&self, name: &str) -> Option<usize> {
    if let Some(ref func) = self.current_function {
      for (i, local_name) in func.locals.iter().enumerate().rev() {
        if local_name == name {
          return Some(i);
        }
      }
    }
    None
  }

  fn add_upvalue(&mut self, index: usize, is_local: bool) -> usize {
    if let Some(ref mut func) = self.current_function {
      // Check if this upvalue already exists
      for (i, upvalue) in func.upvalues.iter().enumerate() {
        if upvalue.index == index && upvalue.is_local == is_local {
          return i;
        }
      }

      // Add new upvalue
      let upvalue_index = func.upvalues.len();
      func.upvalues.push(Upvalue { index, is_local });
      upvalue_index
    } else {
      0
    }
  }

  fn resolve_upvalue(&mut self, name: &str) -> Option<usize> {
    // Recursive function to resolve upvalues through enclosing scopes
    if let Some(ref mut func) = self.current_function {
      if let Some(ref mut enclosing) = func.enclosing {
        // Try to find in enclosing function's locals
        for (i, local_name) in enclosing.locals.iter().enumerate().rev() {
          if local_name == name {
            return Some(self.add_upvalue(i, true));
          }
        }

        // Try to find in enclosing function's upvalues (recursive)
        // This is complex - we'll simplify for now
        // Just check one level up
      }
    }
    None
  }

  fn resolve_variable(&mut self, name: &str) -> VarLocation {
    // 1. Try local variables
    if let Some(idx) = self.resolve_local(name) {
      return VarLocation::Local(idx);
    }

    // 2. Try upvalues (from enclosing scopes)
    if let Some(idx) = self.resolve_upvalue(name) {
      return VarLocation::Upvalue(idx);
    }

    // 3. Must be global
    if let Some(&idx) = self.globals.get(name) {
      return VarLocation::Global(idx);
    }

    // Not found - create as global
    let idx = self.next_global_index;
    self.globals.insert(name.to_string(), idx);
    self.next_global_index += 1;
    VarLocation::Global(idx)
  }

  fn add_local(&mut self, name: String) {
    if let Some(ref mut func) = self.current_function {
      func.locals.push(name);
    }
  }

  fn emit(&mut self, opcode: OpCode, arg: i32) {
    self.instructions.push(Instruction { opcode, arg });
  }

  fn current_address(&self) -> usize {
    self.instructions.len()
  }

  fn compile_match_case_body(&mut self, body: &[Stmt]) -> Result<(), String> {
    if body.is_empty() {
      let const_idx = self.add_constant(Value::Nil);
      self.emit(OpCode::LoadConst, const_idx as i32);
      return Ok(());
    }

    for (i, stmt) in body.iter().enumerate() {
      let is_last = i == body.len() - 1;

      match stmt {
        Stmt::Return { value } => {
          // Return from function
          if let Some(v) = value {
            self.compile_expr(v)?;
          } else {
            let const_idx = self.add_constant(Value::Nil);
            self.emit(OpCode::LoadConst, const_idx as i32);
          }
          self.emit(OpCode::Return, 0);
          return Ok(());
        }
        Stmt::Expr(expr) if is_last => {
          // Last expression is the result
          self.compile_expr(expr)?;
        }
        _ => {
          // Regular statement
          self.compile_stmt(stmt)?;

          // If this was the last statement and not an expression, push Nil
          if is_last {
            let const_idx = self.add_constant(Value::Nil);
            self.emit(OpCode::LoadConst, const_idx as i32);
          }
        }
      }
    }

    Ok(())
  }

  pub fn compile(&mut self, statements: &[Stmt]) -> Result<(), String> {
    // Global scope - don't use enter_scope/exit_scope
    for stmt in statements {
      self.compile_stmt(stmt)?;
    }
    self.emit(OpCode::Halt, 0);
    Ok(())
  }

  fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
    match stmt {
      Stmt::Let {
        name,
        value,
        type_annotation: _,
      } => {
        // Compile the value
        self.compile_expr(value)?;

        // Add as local if in function, otherwise global
        if self.current_function.is_some() {
          self.add_local(name.clone());
          let local_count = if let Some(ref func) = self.current_function {
            func.locals.len() - 1
          } else {
            0
          };
          self.emit(OpCode::StoreLocal, local_count as i32);
        } else {
          // Global variable
          let location = self.resolve_variable(name);
          if let VarLocation::Global(idx) = location {
            self.emit(OpCode::StoreGlobal, idx as i32);
          }
        }
      }
      Stmt::Assign { name, value } => {
        // Compile the value
        self.compile_expr(value)?;

        // Resolve where to store it
        let location = self.resolve_variable(name);
        match location {
          VarLocation::Local(idx) => {
            self.emit(OpCode::StoreLocal, idx as i32);
          }
          VarLocation::Upvalue(idx) => {
            self.emit(OpCode::StoreUpvalue, idx as i32);
          }
          VarLocation::Global(idx) => {
            self.emit(OpCode::StoreGlobal, idx as i32);
          }
        }
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
        name,
        params,
        body,
        return_type: _,
      } => {
        // Jump over the function body
        let jump_addr = self.current_address();
        self.emit(OpCode::Jump, 0);

        let func_addr = self.current_address();
        self.function_addresses.insert(name.clone(), func_addr);

        // Create a new function compiler context
        let enclosing = self.current_function.take();
        self.current_function = Some(Box::new(FunctionCompiler {
          enclosing: enclosing.clone(),
          _function_name: name.clone(),
          locals: Vec::new(),
          upvalues: Vec::new(),
          _scope_depth: 0,
        }));

        // Add parameters as local variables
        for (param_name, _param_type) in params.iter() {
          self.add_local(param_name.clone());
        }

        // Store parameters from stack (in reverse order)
        for i in (0..params.len()).rev() {
          if let Some((_param_name, Some(expected_type))) = params.get(i) {
            let type_const = self.add_constant(Value::String(format!("{}", expected_type)));
            self.emit(OpCode::LoadConst, type_const as i32);
            self.emit(OpCode::AssertType, 0);
          }
          self.emit(OpCode::StoreLocal, i as i32);
        }

        // Compile function body
        for stmt in body {
          self.compile_stmt(stmt)?;
        }

        // Default return
        let const_idx = self.add_constant(Value::Nil);
        self.emit(OpCode::LoadConst, const_idx as i32);
        self.emit(OpCode::Return, 0);

        // Get upvalue count
        let upvalue_count = if let Some(ref func) = self.current_function {
          func.upvalues.len()
        } else {
          0
        };

        // Restore enclosing function context
        if let Some(mut current) = self.current_function.take() {
          self.current_function = current.enclosing.take().map(|b| b);
        }

        let end_addr = self.current_address();
        self.instructions[jump_addr].arg = end_addr as i32;

        // Create function value
        let func_def = FunctionDef {
          name: name.clone(),
          address: func_addr,
          module_id: None,
          upvalue_count,
        };

        if upvalue_count > 0 {
          // This function needs closures - emit MakeClosure
          let func_const = self.add_constant(Value::Function(Rc::new(func_def)));
          self.emit(OpCode::LoadConst, func_const as i32);
          self.emit(OpCode::MakeClosure, upvalue_count as i32);
        } else {
          // Regular function - just store it
          let func_const = self.add_constant(Value::Function(Rc::new(func_def)));
          self.emit(OpCode::LoadConst, func_const as i32);
        }

        // Store function in variable
        let location = self.resolve_variable(name);
        match location {
          VarLocation::Local(idx) => {
            self.emit(OpCode::StoreLocal, idx as i32);
          }
          VarLocation::Upvalue(idx) => {
            self.emit(OpCode::StoreUpvalue, idx as i32);
          }
          VarLocation::Global(idx) => {
            self.emit(OpCode::StoreGlobal, idx as i32);
          }
        }
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

        let (var_idx, is_local) = if let Some(_) = self.current_function {
          // Local variable
          self.add_local(var_name.clone());
          let idx = if let Some(ref func) = self.current_function {
            func.locals.len() - 1
          } else {
            0
          };
          (idx, true)
        } else {
          // Global variable
          let idx = self.get_var_index(var_name);
          (idx, false)
        };

        // Encode: high bit for is_local, low bits for index
        let encoded = if is_local {
          var_idx as i32 | 0x8000_0000u32 as i32
        } else {
          var_idx as i32
        };

        self.emit(OpCode::SetupForIn, encoded);

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
            // Add catch variable as local
            if let Some(_) = self.current_function {
              self.add_local(var_name.clone());
              let var_idx = if let Some(ref func) = self.current_function {
                func.locals.len() - 1
              } else {
                0
              };
              self.emit(OpCode::StoreLocal, var_idx as i32);
            } else {
              let idx = self.get_var_index(var_name);
              self.emit(OpCode::StoreGlobal, idx as i32);
            }
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

          // Create method function context
          let enclosing = self.current_function.take();
          self.current_function = Some(Box::new(FunctionCompiler {
            enclosing: enclosing.clone(),
            _function_name: method_name.clone(),
            locals: Vec::new(),
            upvalues: Vec::new(),
            _scope_depth: 0,
          }));

          // Add 'self' as first local (if not static)
          if !is_static {
            self.add_local("self".to_string());
          }

          // Add parameters as locals
          for (param_name, _) in params.iter() {
            self.add_local(param_name.clone());
          }

          // Store parameters from stack (in REVERSE order)
          for i in (0..params.len()).rev() {
            self.emit(
              OpCode::StoreLocal,
              (i + if *is_static { 0 } else { 1 }) as i32,
            );
          }

          // Store self LAST (if not static)
          if !is_static {
            self.emit(OpCode::StoreLocal, 0);
          }

          // Compile method body
          for stmt in body {
            self.compile_stmt(stmt)?;
          }

          // Default return
          let const_idx = self.add_constant(Value::Nil);
          self.emit(OpCode::LoadConst, const_idx as i32);
          self.emit(OpCode::Return, 0);

          // Restore enclosing context
          if let Some(mut current) = self.current_function.take() {
            self.current_function = current.enclosing.take();
          }

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

        let location = self.resolve_variable(name);
        match location {
          VarLocation::Local(idx) => {
            self.emit(OpCode::StoreLocal, idx as i32);
          }
          VarLocation::Upvalue(idx) => {
            self.emit(OpCode::StoreUpvalue, idx as i32);
          }
          VarLocation::Global(idx) => {
            self.emit(OpCode::StoreGlobal, idx as i32);
          }
        }
      }
      Stmt::Import { path, alias } => {
        let loader = self
          .module_loader
          .as_mut()
          .ok_or_else(|| "Module loader not initialized".to_string())?;

        let module = loader.load_module(path, &self.current_file)?;

        // Store module as a value
        let const_idx = self.add_constant(Value::Module(module));
        self.emit(OpCode::LoadConst, const_idx as i32);

        // Determine the binding name
        let binding_name = if let Some(alias_name) = alias {
          alias_name.clone()
        } else {
          path
            .split(&[':', '/'][..])
            .last()
            .unwrap_or(path)
            .to_string()
        };

        let location = self.resolve_variable(&binding_name);
        match location {
          VarLocation::Local(idx) => {
            self.emit(OpCode::StoreLocal, idx as i32);
          }
          VarLocation::Upvalue(idx) => {
            self.emit(OpCode::StoreUpvalue, idx as i32);
          }
          VarLocation::Global(idx) => {
            self.emit(OpCode::StoreGlobal, idx as i32);
          }
        }
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
        let location = self.resolve_variable(name);
        match location {
          VarLocation::Local(idx) => {
            self.emit(OpCode::LoadLocal, idx as i32);
          }
          VarLocation::Upvalue(idx) => {
            self.emit(OpCode::LoadUpvalue, idx as i32);
          }
          VarLocation::Global(idx) => {
            self.emit(OpCode::LoadGlobal, idx as i32);
          }
        }
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
          // Load the function/closure
          let location = self.resolve_variable(name);
          match location {
            VarLocation::Local(idx) => {
              self.emit(OpCode::LoadLocal, idx as i32);
            }
            VarLocation::Upvalue(idx) => {
              self.emit(OpCode::LoadUpvalue, idx as i32);
            }
            VarLocation::Global(idx) => {
              self.emit(OpCode::LoadGlobal, idx as i32);
            }
          }

          // Push arguments
          for arg in args {
            self.compile_expr(arg)?;
          }

          // Call the function/closure
          self.emit(OpCode::CallClosure, args.len() as i32);
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

        let mut end_jumps = Vec::new();
        let mut has_wildcard = false;

        for (case_idx, case) in cases.iter().enumerate() {
          // Check if this is a wildcard pattern
          if matches!(case.pattern, Pattern::Wildcard) {
            has_wildcard = true;
          }

          // Duplicate the match value for comparison (except for last wildcard)
          if !(case_idx == cases.len() - 1 && has_wildcard) {
            self.emit(OpCode::Dup, 0);
          }

          // Compile pattern matching
          let pattern_matches = match &case.pattern {
            Pattern::Wildcard => {
              // Wildcard always matches, no comparison needed
              if case_idx == cases.len() - 1 {
                // Last case and it's wildcard - no dup, no pop
                self.emit(OpCode::Pop, 0); // Pop the original match value
              } else {
                self.emit(OpCode::Pop, 0); // Pop the dup
              }
              None // No conditional jump needed
            }
            Pattern::Literal(expr) => {
              self.compile_expr(expr)?;
              self.emit(OpCode::Eq, 0);
              Some(true) // Need conditional jump
            }
            Pattern::Ident(name) => {
              // Bind the matched value to this name
              if let Some(_) = self.current_function {
                self.add_local(name.clone());
                let var_idx = if let Some(ref func) = self.current_function {
                  func.locals.len() - 1
                } else {
                  0
                };
                self.emit(OpCode::StoreLocal, var_idx as i32);
              } else {
                let idx = self.get_var_index(name);
                self.emit(OpCode::StoreGlobal, idx as i32);
              }
              None // Always matches after binding
            }
            Pattern::List(patterns) => {
              // The user wrote a list literal pattern like [1, 2, 3] or []
              // Just compile it as a list and compare for equality

              // Compile each pattern element (they should be literals or wildcards)
              for pattern in patterns {
                match pattern {
                  Pattern::Literal(expr) => {
                    self.compile_expr(expr)?;
                  }
                  _ => {
                    return Err("Complex expression in lists pattern".to_string());
                  }
                }
              }

              // Build the list from the patterns
              self.emit(OpCode::BuildList, patterns.len() as i32);

              // Compare with the match value
              self.emit(OpCode::Eq, 0);
              Some(true)
            } // _ => {
              //   self.emit(OpCode::Pop, 0);
              //   let idx = self.add_constant(Value::Bool(false));
              //   self.emit(OpCode::LoadConst, idx as i32);
              //   Some(true)
              // }
          };

          let next_case_jump = if pattern_matches.is_some() {
            let addr = self.current_address();
            self.emit(OpCode::JumpIfFalse, 0);
            Some(addr)
          } else {
            None
          };

          // Pop the original match value if we haven't already
          if !matches!(case.pattern, Pattern::Wildcard) {
            self.emit(OpCode::Pop, 0);
          }

          // Compile case body - ensure it leaves a value on the stack
          self.compile_match_case_body(&case.body)?;

          // Jump to end
          let end_jump = self.current_address();
          self.emit(OpCode::Jump, 0);
          end_jumps.push(end_jump);

          // Patch the "next case" jump if it exists
          if let Some(jump_addr) = next_case_jump {
            let next_case_addr = self.current_address();
            self.instructions[jump_addr].arg = next_case_addr as i32;
          }
        }

        // If no case matched (and no wildcard), pop value and push Nil
        if !has_wildcard {
          self.emit(OpCode::Pop, 0);
          let const_idx = self.add_constant(Value::Nil);
          self.emit(OpCode::LoadConst, const_idx as i32);
        }

        // Patch all end jumps
        let end_addr = self.current_address();
        for jump in end_jumps {
          self.instructions[jump].arg = end_addr as i32;
        }
      }
      Expr::MemberAccess { object, member } => {
        self.compile_expr(object)?;
        let member_const = self.add_constant(Value::String(member.clone()));
        self.emit(OpCode::LoadConst, member_const as i32);
        self.emit(OpCode::GetField, 0);
      }
      Expr::New { class_expr, args } => {
        // Evaluate the class expression (could be Ident, MemberAccess, etc.)
        self.compile_expr(class_expr)?;

        // Now the class is on the stack
        // Push arguments
        for arg in args {
          self.compile_expr(arg)?;
        }

        // Call NewInstance with the class on stack
        self.emit(OpCode::NewInstance, args.len() as i32);
      }
      Expr::SelfExpr => {
        // 'self' is always a local variable (first local in methods)
        let location = self.resolve_variable("self");
        match location {
          VarLocation::Local(idx) => {
            self.emit(OpCode::LoadLocal, idx as i32);
          }
          _ => {
            return Err("'self' can only be used inside methods".to_string());
          }
        }
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
        let location = self.resolve_variable("self");
        match location {
          VarLocation::Local(idx) => {
            self.emit(OpCode::LoadLocal, idx as i32);
          }
          _ => {
            return Err("'super' can only be used inside methods".to_string());
          }
        }

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
      Expr::CallExpr { callee, args } => {
        // Compile the callee expression (could be anything that returns a function)
        self.compile_expr(callee)?;

        // Push arguments
        for arg in args {
          self.compile_expr(arg)?;
        }

        // Call whatever's on the stack
        self.emit(OpCode::CallClosure, args.len() as i32);
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
