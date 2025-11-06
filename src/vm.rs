// ============================================================================
// BYTECODE VM
// ============================================================================

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
  instruction::{Instruction, OpCode},
  value::{Closure, Instance, Value},
};

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct StackFrame {
  return_addr: usize,
  base_pointer: usize, // Where this frame's variables start in the variables vector
  var_count: usize,    // Number of variables in this frame
  is_constructor: bool,
  constructor_instance: Option<Value>,
}

#[derive(Debug, Clone)]
struct TryHandler {
  call_site_pc: usize,
  catch_addr: usize,
  stack_size: usize,
}

#[derive(Debug, Clone)]
struct ForInIterator {
  items: Vec<Value>,
  index: usize,
  var_idx: usize,
  base_pointer: usize,
  is_local: bool,
}

pub struct VM {
  instructions: Vec<Instruction>,
  constants: Vec<Value>,
  pub stack: Vec<Value>,
  locals: Vec<Value>,
  globals: HashMap<usize, Value>,
  global_names: HashMap<usize, String>,
  upvalues: Rc<RefCell<Vec<Value>>>,
  pc: usize,
  call_stack: Vec<StackFrame>,
  base_pointer: usize,
  try_stack: Vec<TryHandler>,
  for_in_stack: Vec<ForInIterator>,
  error: Option<Value>,
}

impl VM {
  pub fn new(
    instructions: Vec<Instruction>,
    constants: Vec<Value>,
    global_var_names: HashMap<usize, String>,
  ) -> Self {
    VM {
      instructions,
      constants,
      stack: Vec::with_capacity(256),
      locals: Vec::with_capacity(256),
      globals: HashMap::new(),
      global_names: global_var_names,
      pc: 0,
      call_stack: Vec::new(),
      base_pointer: 0,
      try_stack: Vec::new(),
      for_in_stack: Vec::new(),
      error: None,
      upvalues: Rc::new(RefCell::new(vec![])),
    }
  }

  fn push(&mut self, value: Value) {
    self.stack.push(value);
  }

  // fn peek(&mut self) -> Result<Value, String> {
  //   let last = self.stack.last();

  //   if let Some(last) = last {
  //     Ok(last.clone())
  //   } else {
  //     Err("Stack underflow".to_string())
  //   }
  // }

  fn pop(&mut self) -> Result<Value, String> {
    self
      .stack
      .pop()
      .ok_or_else(|| "Stack underflow".to_string())
  }

  pub fn reset(&mut self, instructions: Vec<Instruction>, constants: &mut Vec<Value>) {
    self.instructions = instructions;
    self.constants.append(constants);
    self.stack.clear();
    self.locals.clear();
    self.globals.clear();
    self.call_stack.clear();
    self.try_stack.clear();
    self.for_in_stack.clear();
    self.error = None;
    self.pc = 0;
    self.upvalues.take().clear();
  }

  pub fn get_global_variables(&self) -> HashMap<String, Value> {
    let mut result = HashMap::new();

    for (idx, value) in &self.globals {
      if let Some(name) = self.global_names.get(idx) {
        result.insert(name.clone(), value.clone());
      }
    }

    result
  }

  fn get_call_site_pc(&mut self) -> usize {
    self
      .call_stack
      .last()
      .or(Some(&StackFrame {
        return_addr: 0,
        base_pointer: 0,
        var_count: 0,
        is_constructor: false,
        constructor_instance: None,
      }))
      .unwrap()
      .base_pointer
  }

  fn pop_call_stack_to_try_site(&mut self) {
    if self.try_stack.len() > 0 {
      let try_call_site_pc = self.try_stack.last().unwrap().call_site_pc;
      loop {
        let call_site_pc = self.get_call_site_pc();
        if call_site_pc != try_call_site_pc {
          self.call_stack.pop();
        } else {
          break;
        }
      }
    }
  }

  fn values_equal(&self, a: &Value, b: &Value) -> bool {
    match (a, b) {
      (Value::Int(x), Value::Int(y)) => x == y,
      (Value::Float(x), Value::Float(y)) => x == y,
      (Value::Bool(x), Value::Bool(y)) => x == y,
      (Value::String(x), Value::String(y)) => x == y,
      (Value::Nil, Value::Nil) => true,
      (Value::List(list_a), Value::List(list_b)) => {
        let a = list_a.borrow();
        let b = list_b.borrow();

        if a.len() != b.len() {
          return false;
        }

        for (val_a, val_b) in a.iter().zip(b.iter()) {
          if !self.values_equal(val_a, val_b) {
            return false;
          }
        }
        true
      }
      (Value::Map(map_a), Value::Map(map_b)) => {
        let a = map_a.borrow();
        let b = map_b.borrow();

        if a.len() != b.len() {
          return false;
        }

        for (key, val_a) in a.iter() {
          if let Some(val_b) = b.get(key) {
            if !self.values_equal(val_a, val_b) {
              return false;
            }
          } else {
            return false;
          }
        }
        true
      }
      _ => false,
    }
  }

  pub fn run(&mut self) -> Result<(), String> {
    while self.pc < self.instructions.len() {
      let instr = self.instructions[self.pc].clone();

      // println!("{:#4} | {}", self.pc, instr);

      if self.error.is_some() {
        self.handle_error()?;
        continue;
      }

      match self.execute(instr) {
        Ok(_) => self.pc += 1,
        Err(e) => {
          self.error = Some(Value::Error {
            exc_type: "RuntimeError".to_string(),
            message: e,
          });
          if !self.handle_error()? {
            if let Some(Value::Error { exc_type, message }) = &self.error {
              return Err(format!("Uncaught {}: {}", exc_type, message));
            }
            return Err("Uncaught error".to_string());
          }
        }
      }
    }
    Ok(())
  }

  fn handle_error(&mut self) -> Result<bool, String> {
    if let Some(handler) = self.try_stack.pop() {
      while self.stack.len() > handler.stack_size {
        self.stack.pop();
      }

      if let Some(exc) = self.error.take() {
        self.stack.push(exc);
      }

      self.pc = handler.catch_addr;
      Ok(true)
    } else {
      Ok(false)
    }
  }

  fn execute(&mut self, instr: Instruction) -> Result<(), String> {
    match instr.opcode {
      OpCode::Nil => {
        self.push(Value::Nil);
      }
      OpCode::LoadConst => {
        let val = self.constants[instr.arg as usize].clone();
        self.push(val);
      }
      OpCode::LoadLocal => {
        let local_idx = instr.arg as usize;
        let actual_idx = self.base_pointer + local_idx;

        if actual_idx < self.locals.len() {
          let val = self.locals[actual_idx].clone();
          self.push(val);
        } else {
          return Err(format!("Local variable at index {} not found", local_idx));
        }
      }
      OpCode::StoreLocal => {
        let val = self.pop()?;
        let local_idx = instr.arg as usize;
        let actual_idx = self.base_pointer + local_idx;

        // Grow locals vector if needed
        while self.locals.len() <= actual_idx {
          self.locals.push(Value::Nil);
        }

        self.locals[actual_idx] = val;
      }
      OpCode::LoadUpvalue => {
        let upvalue_idx = instr.arg as usize;

        if upvalue_idx < self.upvalues.borrow().len() {
          let val = self.upvalues.borrow()[upvalue_idx].clone();
          self.push(val);
        } else {
          return Err(format!("Upvalue at index {} not found", upvalue_idx));
        }
      }
      OpCode::StoreUpvalue => {
        let val = self.pop()?;
        let upvalue_idx = instr.arg as usize;

        if upvalue_idx < self.upvalues.borrow().len() {
          self.upvalues.borrow_mut()[upvalue_idx] = val;
        } else {
          return Err(format!("Upvalue at index {} not found", upvalue_idx));
        }
      }
      OpCode::LoadGlobal => {
        let global_idx = instr.arg as usize;

        if let Some(val) = self.globals.get(&global_idx) {
          self.push(val.clone());
        } else {
          return Err(format!("Global variable at index {} not found", global_idx));
        }
      }
      OpCode::StoreGlobal => {
        let val = self.pop()?;
        let global_idx = instr.arg as usize;
        self.globals.insert(global_idx, val);
      }
      OpCode::Add => {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (a, b) {
          (Value::Int(x), Value::Int(y)) => Value::Int(x + y),
          (Value::Float(x), Value::Float(y)) => Value::Float(x + y),
          (Value::Int(x), Value::Float(y)) => Value::Float(x as f64 + y),
          (Value::Float(x), Value::Int(y)) => Value::Float(x + y as f64),
          (Value::String(x), Value::String(y)) => Value::String(format!("{}{}", x, y)),
          _ => return Err("Type error in addition".to_string()),
        };
        self.push(result);
      }
      OpCode::Sub => {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (a, b) {
          (Value::Int(x), Value::Int(y)) => Value::Int(x - y),
          (Value::Float(x), Value::Float(y)) => Value::Float(x - y),
          (Value::Int(x), Value::Float(y)) => Value::Float(x as f64 - y),
          (Value::Float(x), Value::Int(y)) => Value::Float(x - y as f64),
          _ => return Err("Type error in subtraction".to_string()),
        };
        self.push(result);
      }
      OpCode::Mul => {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (a, b) {
          (Value::Int(x), Value::Int(y)) => Value::Int(x * y),
          (Value::Float(x), Value::Float(y)) => Value::Float(x * y),
          (Value::Int(x), Value::Float(y)) => Value::Float(x as f64 * y),
          (Value::Float(x), Value::Int(y)) => Value::Float(x * y as f64),
          (Value::String(x), Value::Int(y)) => Value::String(x.repeat(y as usize)),
          _ => return Err("Type error in multiplication".to_string()),
        };
        self.push(result);
      }
      OpCode::Div => {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (a, b) {
          (Value::Int(x), Value::Int(y)) => {
            if y == 0 {
              return Err("Division by zero".to_string());
            }

            let res = x as f64 / y as f64;
            if res.floor() == res {
              Value::Int(res as i64)
            } else {
              Value::Float(res)
            }
          }
          (Value::Float(x), Value::Float(y)) => Value::Float(x / y),
          (Value::Int(x), Value::Float(y)) => Value::Float(x as f64 / y),
          (Value::Float(x), Value::Int(y)) => Value::Float(x / y as f64),
          _ => return Err("Type error in division".to_string()),
        };
        self.push(result);
      }
      OpCode::Mod => {
        let b = self.pop()?;
        let a = self.pop()?;
        if let (Value::Int(x), Value::Int(y)) = (a, b) {
          self.push(Value::Int(x % y));
        } else {
          return Err("Type error in modulo".to_string());
        }
      }
      OpCode::Pow => {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (a, b) {
          (Value::Int(x), Value::Int(y)) => Value::Int(x.pow(y as u32)),
          (Value::Float(x), Value::Float(y)) => Value::Float(x.powf(y)),
          (Value::Int(x), Value::Float(y)) => Value::Float((x as f64).powf(y)),
          (Value::Float(x), Value::Int(y)) => Value::Float(x.powf(y as f64)),
          _ => return Err("Type error in power".to_string()),
        };
        self.push(result);
      }
      OpCode::Floor => {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (a, b) {
          (Value::Int(x), Value::Int(y)) => Value::Int(x / y),
          (Value::Float(x), Value::Float(y)) => Value::Int((x / y).floor() as i64),
          (Value::Int(x), Value::Float(y)) => Value::Int(((x as f64) / y).floor() as i64),
          (Value::Float(x), Value::Int(y)) => Value::Int((x / (y as f64)).floor() as i64),
          _ => return Err("Type error in power".to_string()),
        };
        self.push(result);
      }
      OpCode::Neg => {
        let val = self.pop()?;
        let result = match val {
          Value::Int(x) => Value::Int(-x),
          Value::Float(x) => Value::Float(-x),
          _ => return Err("Type error in negation".to_string()),
        };
        self.push(result);
      }
      OpCode::Eq => {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = Value::Bool(self.values_equal(&a, &b));
        self.push(result);
      }
      OpCode::Neq => {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = Value::Bool(!self.values_equal(&a, &b));
        self.push(result);
      }
      OpCode::Lt => {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (a, b) {
          (Value::Int(x), Value::Int(y)) => Value::Bool(x < y),
          (Value::Float(x), Value::Float(y)) => Value::Bool(x < y),
          _ => return Err("Type error in comparison".to_string()),
        };
        self.push(result);
      }
      OpCode::Gt => {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (a, b) {
          (Value::Int(x), Value::Int(y)) => Value::Bool(x > y),
          (Value::Float(x), Value::Float(y)) => Value::Bool(x > y),
          _ => return Err("Type error in comparison".to_string()),
        };
        self.push(result);
      }
      OpCode::Lte => {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (a, b) {
          (Value::Int(x), Value::Int(y)) => Value::Bool(x <= y),
          (Value::Float(x), Value::Float(y)) => Value::Bool(x <= y),
          _ => return Err("Type error in comparison".to_string()),
        };
        self.push(result);
      }
      OpCode::Gte => {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (a, b) {
          (Value::Int(x), Value::Int(y)) => Value::Bool(x >= y),
          (Value::Float(x), Value::Float(y)) => Value::Bool(x >= y),
          _ => return Err("Type error in comparison".to_string()),
        };
        self.push(result);
      }
      OpCode::And => {
        let b = self.pop()?;
        let a = self.pop()?;
        if let (Value::Bool(x), Value::Bool(y)) = (a, b) {
          self.push(Value::Bool(x && y));
        } else {
          return Err("Type error in AND".to_string());
        }
      }
      OpCode::Or => {
        let b = self.pop()?;
        let a = self.pop()?;
        if let (Value::Bool(x), Value::Bool(y)) = (a, b) {
          self.push(Value::Bool(x || y));
        } else {
          return Err("Type error in OR".to_string());
        }
      }
      OpCode::Not => {
        let val = self.pop()?;
        if let Value::Bool(x) = val {
          self.push(Value::Bool(!x));
        } else {
          return Err("Type error in NOT".to_string());
        }
      }
      OpCode::Jump => {
        self.pc = instr.arg as usize - 1;
      }
      OpCode::JumpIfFalse => {
        let cond = self.pop()?;
        if let Value::Bool(false) = cond {
          self.pc = instr.arg as usize - 1;
        }
      }
      OpCode::Call => {
        // Save current frame info
        let frame = StackFrame {
          return_addr: self.pc,
          base_pointer: self.base_pointer,
          var_count: 0, // Will be set by the callee if needed
          is_constructor: false,
          constructor_instance: None,
        };

        self.call_stack.push(frame);

        // Set new base pointer to current variable count
        self.base_pointer = self.locals.len();

        // Jump to function
        self.pc = instr.arg as usize - 1;
      }
      OpCode::Return => {
        let return_val = self.pop().unwrap_or(Value::Nil);

        if let Some(frame) = self.call_stack.pop() {
          // Restore the previous frame's base pointer
          let old_base = self.base_pointer;
          self.base_pointer = frame.base_pointer;

          // Clean up local variables from the returning function
          self.locals.truncate(old_base);

          // Restore program counter
          self.pc = frame.return_addr;

          // If this was a constructor, push the instance instead of return value
          if frame.is_constructor {
            if let Some(instance) = frame.constructor_instance {
              self.push(instance);
            } else {
              self.push(Value::Nil);
            }
          } else {
            // Normal return - push return value
            self.push(return_val);
          }
        } else {
          // Returning from main/top-level
          self.pc = self.instructions.len();
        }
      }
      OpCode::BuildList => {
        let count = instr.arg as usize;
        let mut elements = Vec::new();
        for _ in 0..count {
          elements.push(self.pop()?);
        }
        elements.reverse();
        self.push(Value::List(Rc::new(RefCell::new(elements))));
      }
      OpCode::BuildMap => {
        let count = instr.arg as usize;
        let mut map = HashMap::new();
        for _ in 0..count {
          let value = self.pop()?;
          let key = self.pop()?;
          if let Some(k) = key.to_key() {
            map.insert(k, value);
          } else {
            return Err("Map keys must be strings or integers".to_string());
          }
        }
        self.push(Value::Map(Rc::new(RefCell::new(map))));
      }
      OpCode::Index => {
        let index = self.pop()?;
        let target = self.pop()?;

        match (target, index) {
          (Value::List(list_ref), Value::Int(i)) => {
            let list = list_ref.borrow();
            let idx = if i < 0 {
              (list.len() as i64 + i) as usize
            } else {
              i as usize
            };
            if idx < list.len() {
              self.push(list[idx].clone());
            } else {
              return Err("List index out of bounds".to_string());
            }
          }
          (Value::Map(map_ref), key) => {
            let map = map_ref.borrow();
            if let Some(k) = key.to_key() {
              if let Some(val) = map.get(&k) {
                self.push(val.clone());
              } else {
                return Err(format!("Key '{}' not found in map", k));
              }
            } else {
              return Err("Invalid map key".to_string());
            }
          }
          (Value::Tensor { shape: _, data }, Value::Int(i)) => {
            let data_vec = data.borrow();
            let idx = if i < 0 {
              (data_vec.len() as i64 + i) as usize
            } else {
              i as usize
            };
            if idx < data_vec.len() {
              self.push(Value::Float(data_vec[idx]));
            } else {
              return Err("Tensor index out of bounds".to_string());
            }
          }
          _ => return Err("Invalid indexing operation".to_string()),
        }
      }
      OpCode::IndexSet => {
        let value = self.pop()?;
        let index = self.pop()?;
        let target = self.pop()?;

        match (target, index, value) {
          (Value::List(list_ref), Value::Int(i), val) => {
            let mut list = list_ref.borrow_mut(); // borrow_mut for mutation
            let idx = if i < 0 {
              (list.len() as i64 + i) as usize
            } else {
              i as usize
            };
            if idx < list.len() {
              list[idx] = val;
              // Don't push anything - the list was modified in place
            } else {
              return Err("List index out of bounds".to_string());
            }
          }
          (Value::Map(map_ref), key, val) => {
            let mut map = map_ref.borrow_mut();
            if let Some(k) = key.to_key() {
              map.insert(k, val);
              // Modified in place, no need to push
            } else {
              return Err("Invalid map key".to_string());
            }
          }
          (Value::Tensor { shape: _, data }, Value::Int(i), Value::Float(f)) => {
            let mut data_vec = data.borrow_mut();
            let idx = if i < 0 {
              (data_vec.len() as i64 + i) as usize
            } else {
              i as usize
            };
            if idx < data_vec.len() {
              data_vec[idx] = f;
              // Modified in place
            } else {
              return Err("Tensor index out of bounds".to_string());
            }
          }
          (Value::Tensor { shape: _, data }, Value::Int(i), Value::Int(n)) => {
            let mut data_vec = data.borrow_mut();
            let idx = if i < 0 {
              (data_vec.len() as i64 + i) as usize
            } else {
              i as usize
            };
            if idx < data_vec.len() {
              data_vec[idx] = n as f64;
              // Modified in place
            } else {
              return Err("Tensor index out of bounds".to_string());
            }
          }
          _ => return Err("Invalid index assignment".to_string()),
        }
      }
      OpCode::Slice => {
        let end = self.pop()?;
        let start = self.pop()?;
        let target = self.pop()?;

        match (target, start, end) {
          (Value::List(list_ref), Value::Int(s), Value::Int(e)) => {
            let list = list_ref.borrow();
            let start_idx = if s < 0 {
              (list.len() as i64 + s).max(0) as usize
            } else {
              (s as usize).min(list.len())
            };

            let end_idx = if e < 0 {
              (list.len() as i64 + e + 1).max(0) as usize
            } else {
              (e as usize).min(list.len())
            };

            if start_idx <= end_idx {
              self.push(Value::List(Rc::new(RefCell::new(
                list[start_idx..end_idx].to_vec(),
              ))));
            } else {
              self.push(Value::List(Rc::new(RefCell::new(vec![]))));
            }
          }
          _ => return Err("Invalid slicing operation".to_string()),
        }
      }
      OpCode::BuildRange => {
        let end = self.pop()?;
        let start = self.pop()?;
        let inclusive = instr.arg == 1;

        match (start, end) {
          (Value::Int(s), Value::Int(e)) => {
            self.push(Value::Range {
              start: s,
              end: e,
              inclusive,
            });
          }
          _ => return Err("Range bounds must be integers".to_string()),
        }
      }
      OpCode::SetupForIn => {
        let iterable = self.pop()?;
        let encoded = instr.arg;

        let is_local = (encoded as u32 & 0x8000_0000) != 0;
        let var_idx = (encoded as u32 & 0x7FFF_FFFF) as usize;

        let items = match iterable {
          Value::List(list_ref) => list_ref.borrow().clone(),
          Value::Range {
            start,
            end,
            inclusive,
          } => {
            let mut items = Vec::new();
            if start <= end {
              let limit = if inclusive { end + 1 } else { end };
              for i in start..limit {
                items.push(Value::Int(i));
              }
            } else {
              let limit = if inclusive { end - 1 } else { end };
              let mut i = start;
              while i > limit {
                items.push(Value::Int(i));
                i -= 1;
              }
            }
            items
          }
          Value::Tensor { data, .. } => {
            let data_vec = data.borrow();
            data_vec.iter().map(|&x| Value::Float(x)).collect()
          }
          Value::Map(map_ref) => {
            let map = map_ref.borrow();
            map
              .iter()
              .map(|(k, v)| {
                Value::List(Rc::new(RefCell::new(vec![
                  Value::String(k.clone()),
                  v.clone(),
                ])))
              })
              .collect()
          }
          _ => return Err("Cannot iterate over this type".to_string()),
        };

        self.for_in_stack.push(ForInIterator {
          items,
          index: 0,
          var_idx,
          base_pointer: if is_local { self.base_pointer } else { 0 },
          is_local,
        });
      }
      OpCode::ForInNext => {
        if let Some(iterator) = self.for_in_stack.last_mut() {
          if iterator.index < iterator.items.len() {
            let item = iterator.items[iterator.index].clone();

            if iterator.is_local {
              // Store in locals
              let actual_idx = iterator.base_pointer + iterator.var_idx;
              while self.locals.len() <= actual_idx {
                self.locals.push(Value::Nil);
              }
              self.locals[actual_idx] = item;
            } else {
              // Store in globals
              self.globals.insert(iterator.var_idx, item);
            }

            iterator.index += 1;
          } else {
            self.pc = instr.arg as usize - 1;
          }
        } else {
          return Err("No active for-in loop".to_string());
        }
      }
      OpCode::PopForIn => {
        self.for_in_stack.pop();
      }
      OpCode::TensorCreate => {
        let shape_val = self.pop()?;
        if let Value::List(list_ref) = shape_val {
          let shape_list = list_ref.borrow();
          let shape: Vec<usize> = shape_list
            .iter()
            .filter_map(|v| {
              if let Value::Int(n) = v {
                Some(*n as usize)
              } else {
                None
              }
            })
            .collect();

          let size: usize = shape.iter().product();
          let data: Vec<f64> = (0..size).map(|i| (i as f64) * 0.01).collect();

          self.push(Value::Tensor {
            shape,
            data: Rc::new(RefCell::new(data)),
          });
        } else {
          return Err("Invalid tensor shape".to_string());
        }
      }
      OpCode::MatMul => {
        let _b = self.pop()?;
        let _a = self.pop()?;
        self.push(Value::Tensor {
          shape: vec![1, 1],
          data: Rc::new(RefCell::new(vec![1.0])),
        });
      }
      OpCode::Relu => {
        let tensor = self.pop()?;
        if let Value::Tensor { shape, data } = tensor {
          let mut data_vec = data.borrow_mut();
          for x in data_vec.iter_mut() {
            *x = x.max(0.0);
          }
          drop(data_vec);
          self.push(Value::Tensor { shape, data });
        } else {
          return Err("ReLU requires tensor".to_string());
        }
      }
      OpCode::Sigmoid => {
        let tensor = self.pop()?;
        if let Value::Tensor { shape, data } = tensor {
          let mut data_vec = data.borrow_mut();
          for x in data_vec.iter_mut() {
            *x = 1.0 / (1.0 + (-*x).exp());
          }
          drop(data_vec);
          self.push(Value::Tensor { shape, data });
        } else {
          return Err("Sigmoid requires tensor".to_string());
        }
      }
      OpCode::Tanh => {
        let tensor = self.pop()?;
        if let Value::Tensor { shape, data } = tensor {
          let mut data_vec = data.borrow_mut();
          for x in data_vec.iter_mut() {
            *x = x.tanh();
          }
          drop(data_vec);
          self.push(Value::Tensor { shape, data });
        } else {
          return Err("Tanh requires tensor".to_string());
        }
      }
      OpCode::Softmax => {
        let tensor = self.pop()?;
        if let Value::Tensor { shape, data } = tensor {
          let mut data_vec = data.borrow_mut();
          let max_val = data_vec.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
          let exp_vals: Vec<f64> = data_vec.iter().map(|&x| (x - max_val).exp()).collect();
          let sum: f64 = exp_vals.iter().sum();
          for (i, x) in data_vec.iter_mut().enumerate() {
            *x = exp_vals[i] / sum;
          }
          drop(data_vec);
          self.push(Value::Tensor { shape, data });
        } else {
          return Err("Softmax requires tensor".to_string());
        }
      }
      OpCode::Print => {
        let val = self.pop()?;
        println!("{}", val);
      }
      OpCode::Pop => {
        self.pop()?;
      }
      OpCode::Dup => {
        if let Some(top) = self.stack.last() {
          self.push(top.clone());
        } else {
          return Err("Cannot duplicate: stack is empty".to_string());
        }
      }
      OpCode::Halt => {
        self.pc = self.instructions.len();
      }
      OpCode::Raise => {
        self.pop_call_stack_to_try_site();

        if instr.arg == 1 {
          // Custom error
          let exc_type = self.pop()?;
          let message = self.pop()?;
          if let (Value::String(t), Value::String(m)) = (exc_type, message) {
            return Err(format!("{}: {}", t, m));
          }
        } else {
          let val = self.pop()?;
          let msg = match val {
            Value::String(s) => s,
            Value::Error { message, .. } => message,
            other => format!("{}", other),
          };
          return Err(msg);
        }
      }
      OpCode::SetupTry => {
        let catch_addr = instr.arg as usize;
        let call_site_pc = self.get_call_site_pc();

        self.try_stack.push(TryHandler {
          call_site_pc,
          catch_addr,
          stack_size: self.stack.len(),
        });
      }
      OpCode::PopTry => {
        self.try_stack.pop();
      }
      OpCode::BeginFinally => {}
      OpCode::EndFinally => {}
      OpCode::AssertType => {
        let expected_type_str = self.pop()?;
        let value = self.pop()?;

        if let Value::String(expected) = expected_type_str {
          let actual = value.type_name();

          let matches = match expected.as_str() {
            "int" => matches!(value, Value::Int(_)),
            "float" => matches!(value, Value::Float(_)),
            "bool" => matches!(value, Value::Bool(_)),
            "string" => matches!(value, Value::String(_)),
            "Tensor" => matches!(value, Value::Tensor { .. }),
            "Range" => matches!(value, Value::Range { .. }),
            _ => true,
          };

          if !matches {
            return Err(format!(
              "Type assertion failed: expected '{}', got '{}'",
              expected, actual
            ));
          }
        }

        self.push(value);
      }
      OpCode::NewInstance => {
        let arg_count = instr.arg as usize;

        // Pop arguments
        let mut args = Vec::new();
        for _ in 0..arg_count {
          args.push(self.pop()?);
        }
        args.reverse();

        // Pop the class
        let class_val = self.pop()?;

        if let Value::Class(class_def) = class_val {
          // Create instance
          let instance = Instance::new(Rc::clone(&class_def));
          let instance_val = Value::Instance(Rc::new(RefCell::new(instance)));

          // Check if 'new' constructor exists
          if let Some(init_method) = class_def.find_method("new") {
            let init_addr = init_method.address;

            // Check if this class has module context
            let has_module_context = class_def.source_module.is_some();

            if has_module_context {
              // Execute constructor in module context
              let (instructions, constants) = class_def.source_module.clone().unwrap();
              let mut module_vm = VM::new(instructions, constants, HashMap::new());

              // Push instance as self
              module_vm.stack.push(instance_val.clone());

              // Push arguments in reverse
              for arg in args.iter().rev() {
                module_vm.stack.push(arg.clone());
              }

              module_vm.pc = init_addr;

              // Execute until return
              while module_vm.pc < module_vm.instructions.len() {
                let instr = module_vm.instructions[module_vm.pc].clone();

                if matches!(instr.opcode, OpCode::Return) {
                  match module_vm.execute(instr) {
                    Ok(_) => break,
                    Err(e) => return Err(format!("Constructor error: {}", e)),
                  }
                } else {
                  match module_vm.execute(instr) {
                    Ok(_) => {
                      if module_vm.pc >= module_vm.instructions.len() {
                        break;
                      }
                      module_vm.pc += 1;
                    }
                    Err(e) => return Err(format!("Constructor error: {}", e)),
                  }
                }
              }

              // Push the instance
              self.push(instance_val);
            } else {
              // Regular (non-module) constructor call
              // Push instance as 'self'
              self.push(instance_val.clone());

              // Push arguments
              for arg in args {
                self.push(arg);
              }

              // Call constructor with special flag
              let frame = StackFrame {
                return_addr: self.pc,
                base_pointer: self.base_pointer,
                var_count: 0,
                is_constructor: true, // Mark as constructor
                constructor_instance: Some(instance_val.clone()), // Store instance
              };
              self.call_stack.push(frame);
              self.base_pointer = self.locals.len();
              self.pc = init_addr - 1;
            }
          } else {
            // No constructor, just push the instance
            self.push(instance_val);
          }
        } else {
          return Err(format!(
            "Expected class for instantiation, got {}",
            class_val.type_name()
          ));
        }
      }
      OpCode::GetField => {
        let field_name = self.pop()?;
        let object_val = self.pop()?;

        match (object_val, field_name) {
          (Value::Instance(instance_ref), Value::String(name)) => {
            let instance = instance_ref.borrow();
            if let Some(value) = instance.get_field(&name) {
              self.push(value.clone());
            } else {
              return Err(format!("Instance has no field '{}'", name));
            }
          }
          (Value::Module(module), Value::String(name)) => {
            // Try to get exported value
            if let Some(value) = module.get_export(&name) {
              self.push(value.clone());
            } else {
              return Err(format!("Module '{}' has no export '{}'", module.name, name));
            }
          }
          _ => {
            return Err("Invalid field access".to_string());
          }
        }
      }
      OpCode::SetField => {
        let value = self.pop()?;
        let field_name = self.pop()?;
        let object_val = self.pop()?;

        if let Value::Instance(instance_ref) = object_val {
          if let Value::String(name) = field_name {
            let mut instance = instance_ref.borrow_mut();
            instance.set_field(&name, value);
          } else {
            return Err(format!(
              "Field name must be a name, got {}",
              field_name.type_name()
            ));
          }
        } else {
          return Err(format!(
            "Invalid field assignment - expected instance, got {}",
            object_val.type_name()
          ));
        }
      }
      OpCode::CallMethod => {
        let method_name = self.pop()?;
        let arg_count = instr.arg as usize;

        // Pop arguments
        let mut args = Vec::new();
        for _ in 0..arg_count {
          args.push(self.pop()?);
        }
        args.reverse();

        let target = self.pop()?;

        match target {
          Value::Instance(instance_ref) => {
            // Instance method call
            if let Value::String(method) = method_name {
              let (method_addr, has_module_context) = {
                let instance = instance_ref.borrow();
                let method_def = instance.class.find_method(&method);
                let has_context = instance.class.source_module.is_some();
                (method_def.map(|m| m.address), has_context)
              };

              if let Some(addr) = method_addr {
                // Check if this class came from a module
                if has_module_context {
                  // Execute in module context
                  let (instructions, constants) = {
                    let instance = instance_ref.borrow();
                    instance.class.source_module.clone().unwrap()
                  };

                  let mut module_vm = VM::new(instructions, constants, HashMap::new());

                  // Push self (the instance)
                  module_vm
                    .stack
                    .push(Value::Instance(Rc::clone(&instance_ref)));

                  // Push arguments in reverse
                  for arg in args.iter().rev() {
                    module_vm.stack.push(arg.clone());
                  }

                  module_vm.pc = addr;

                  let mut return_value = Value::Nil;

                  while module_vm.pc < module_vm.instructions.len() {
                    let instr = module_vm.instructions[module_vm.pc].clone();

                    if matches!(instr.opcode, OpCode::Return) {
                      return_value = module_vm.stack.last().cloned().unwrap_or(Value::Nil);
                      match module_vm.execute(instr) {
                        Ok(_) => break,
                        Err(e) => return Err(format!("Module method error: {}", e)),
                      }
                    } else {
                      match module_vm.execute(instr) {
                        Ok(_) => {
                          if module_vm.pc >= module_vm.instructions.len() {
                            break;
                          }
                          module_vm.pc += 1;
                        }
                        Err(e) => return Err(format!("Module method error: {}", e)),
                      }
                    }
                  }

                  self.push(return_value);
                } else {
                  // Regular instance method call (original code)
                  self.push(Value::Instance(instance_ref));
                  for arg in args {
                    self.push(arg);
                  }

                  let frame = StackFrame {
                    return_addr: self.pc,
                    base_pointer: self.base_pointer,
                    var_count: 0,
                    is_constructor: false,
                    constructor_instance: None,
                  };
                  self.call_stack.push(frame);
                  self.base_pointer = self.locals.len();
                  self.pc = addr - 1;
                }
              } else {
                return Err(format!("Method '{}' not found", method));
              }
            } else {
              return Err("Method name must be a string".to_string());
            }
          }
          Value::Module(module) => {
            // Module function call
            if let Value::String(func_name) = method_name {
              if let Some(Value::Function(func_def)) = module.exports.get(&func_name) {
                // Create a new VM with the module's bytecode
                let mut module_vm = VM::new(
                  module.instructions.clone(),
                  module.constants.clone(),
                  HashMap::new(),
                );

                // Push arguments in reverse order onto the stack
                for arg in args.iter().rev() {
                  module_vm.stack.push(arg.clone());
                }

                // Start execution at the function's address
                module_vm.pc = func_def.address;

                let mut return_value = Value::Nil;

                // Execute instructions until we return or halt
                while module_vm.pc < module_vm.instructions.len() {
                  let instr = module_vm.instructions[module_vm.pc].clone();

                  // Check if this is a Return instruction - capture value before executing
                  if matches!(instr.opcode, OpCode::Return) {
                    // The return value should be on top of the stack
                    return_value = module_vm.stack.last().cloned().unwrap_or(Value::Nil);

                    // Now execute the return (which will pop it and set pc)
                    match module_vm.execute(instr) {
                      Ok(_) => break,
                      Err(e) => return Err(format!("Module function error: {}", e)),
                    }
                  } else {
                    match module_vm.execute(instr) {
                      Ok(_) => {
                        // Check if we've finished
                        if module_vm.pc >= module_vm.instructions.len() {
                          break;
                        }
                        module_vm.pc += 1;
                      }
                      Err(e) => {
                        return Err(format!("Module function error: {}", e));
                      }
                    }
                  }
                }

                // Push the captured return value
                self.push(return_value);
              } else {
                return Err(format!(
                  "Module '{}' has no function '{}'",
                  module.name, func_name
                ));
              }
            } else {
              return Err("Function name must be a string".to_string());
            }
          }
          Value::Class(class_def) => {
            // Static method call
            if let Value::String(method) = method_name {
              if let Some(static_method) = class_def.static_methods.get(&method) {
                for arg in args {
                  self.push(arg);
                }

                let frame = StackFrame {
                  return_addr: self.pc,
                  base_pointer: self.base_pointer,
                  var_count: 0,
                  is_constructor: false,
                  constructor_instance: None,
                };
                self.call_stack.push(frame);
                self.base_pointer = self.locals.len();
                self.pc = static_method.address - 1;
              } else {
                return Err(format!("Static method '{}' not found", method));
              }
            } else {
              return Err("Method name must be a string".to_string());
            }
          }
          _ => {
            return Err(format!("Cannot call method on {}", target.type_name()));
          }
        }
      }
      OpCode::CallSuper => {
        let current_class_name = self.pop()?;
        let method_name = self.pop()?;
        let arg_count = instr.arg as usize;

        let mut args = Vec::new();
        for _ in 0..arg_count {
          args.push(self.pop()?);
        }
        args.reverse();

        let instance_val = self.pop()?;

        if let Value::Instance(instance_ref) = &instance_val {
          if let Value::String(method) = &method_name {
            if let Value::String(_current_class) = current_class_name {
              // Find the method in parent classes
              let method_addr = {
                let instance = instance_ref.borrow();

                // Search through parents in order
                let mut addr = None;
                for parent in &instance.class.parents {
                  if let Some(m) = parent.find_method(method) {
                    addr = Some(m.address);
                    break;
                  }
                }

                addr
              };

              if let Some(addr) = method_addr {
                self.push(instance_val.clone());
                for arg in args {
                  self.push(arg);
                }

                // Create stack frame
                // Check if this is a super() call in a constructor (method name is "new")
                let is_super_constructor = method == "new";

                let frame = StackFrame {
                  return_addr: self.pc,
                  base_pointer: self.base_pointer,
                  var_count: 0,
                  is_constructor: is_super_constructor,
                  constructor_instance: if is_super_constructor {
                    Some(instance_val.clone())
                  } else {
                    None
                  },
                };
                self.call_stack.push(frame);
                self.base_pointer = self.locals.len();
                self.pc = addr - 1;
              } else {
                return Err(format!(
                  "Super method '{}' not found in any parent class",
                  method
                ));
              }
            } else {
              return Err("Invalid super call".to_string());
            }
          } else {
            return Err("Invalid super call".to_string());
          }
        } else {
          return Err("Invalid super call".to_string());
        }
      }
      OpCode::LoadSelf => {
        let self_idx = instr.arg as usize;
        let val = self.locals[self_idx].clone();
        self.push(val);
      }

      OpCode::MakeClosure => {
        let upvalue_count = instr.arg as usize;

        // Pop the function from stack
        let func_val = self.pop()?;

        if let Value::Function(func_def) = func_val {
          // Capture upvalues from current scope
          let mut captured_upvalues = Vec::new();

          for i in 0..upvalue_count {
            // For now, capture from current locals
            // In a full implementation, we'd need to track which are local vs upvalue
            let local_idx = self.base_pointer + i;
            if local_idx < self.locals.len() {
              captured_upvalues.push(self.locals[local_idx].clone());
            } else {
              captured_upvalues.push(Value::Nil);
            }
          }

          let closure = Closure {
            function: func_def,
            upvalues: Rc::new(RefCell::new(captured_upvalues)),
          };

          self.push(Value::Closure(Rc::new(closure)));
        } else {
          return Err(format!(
            "Expected function for MakeClosure, got {}",
            func_val.type_name()
          ));
        }
      }
      OpCode::CallClosure => {
        let arg_count = instr.arg as usize;

        // Pop arguments
        let mut args = Vec::new();
        for _ in 0..arg_count {
          args.push(self.pop()?);
        }
        args.reverse();

        // Pop the callable (function or closure)
        let callable = self.pop()?;

        match callable {
          Value::Function(func_def) => {
            // Regular function call (no upvalues)
            for arg in args {
              self.push(arg);
            }

            let frame = StackFrame {
              return_addr: self.pc,
              base_pointer: self.base_pointer,
              var_count: 0,
              is_constructor: false,
              constructor_instance: None,
            };
            self.call_stack.push(frame);
            self.base_pointer = self.locals.len();
            self.pc = func_def.address - 1;
          }
          Value::Closure(closure) => {
            // Closure call - set up upvalues
            // let saved_upvalues = self.upvalues.clone();
            self.upvalues = closure.upvalues.clone();

            for arg in args {
              self.push(arg);
            }

            let frame = StackFrame {
              return_addr: self.pc,
              base_pointer: self.base_pointer,
              var_count: 0,
              is_constructor: false,
              constructor_instance: None,
            };
            self.call_stack.push(frame);
            self.base_pointer = self.locals.len();
            self.pc = closure.function.address - 1;

            // Store saved upvalues so we can restore them on return
            // For now, we'll handle this in Return
          }
          _ => {
            return Err(format!("Cannot call {} as function", callable.type_name()));
          }
        }
      }
      OpCode::MatchBegin | OpCode::MatchCase | OpCode::MatchEnd => {
        // Pattern matching is handled inline during compilation
      }
    }
    Ok(())
  }
}
