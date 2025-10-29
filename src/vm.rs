// ============================================================================
// BYTECODE VM
// ============================================================================

use std::collections::HashMap;

use crate::{
  instruction::{Instruction, OpCode},
  value::Value,
};

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
}

pub struct VM {
  instructions: Vec<Instruction>,
  constants: Vec<Value>,
  pub stack: Vec<Value>,
  variables: Vec<Value>,
  pc: usize,
  call_stack: Vec<usize>,
  try_stack: Vec<TryHandler>,
  for_in_stack: Vec<ForInIterator>,
  error: Option<Value>,
}

impl VM {
  pub fn new(instructions: Vec<Instruction>, constants: Vec<Value>) -> Self {
    VM {
      instructions,
      constants,
      stack: Vec::with_capacity(256),
      variables: vec![Value::Nil; 256],
      pc: 0,
      call_stack: Vec::new(),
      try_stack: Vec::new(),
      for_in_stack: Vec::new(),
      error: None,
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
    self.call_stack.clear();
    self.try_stack.clear();
    self.for_in_stack.clear();
    self.error = None;
    self.pc = 0;
  }

  fn get_call_site_pc(&mut self) -> usize {
    *self.call_stack.last().or(Some(&0)).unwrap()
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
      OpCode::LoadVar => {
        let val = self.variables[instr.arg as usize].clone();
        self.push(val);
      }
      OpCode::StoreVar => {
        let val = self.pop()?;
        self.variables[instr.arg as usize] = val;
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
        let result = match (a, b) {
          (Value::Int(x), Value::Int(y)) => Value::Bool(x == y),
          (Value::Float(x), Value::Float(y)) => Value::Bool(x == y),
          (Value::Bool(x), Value::Bool(y)) => Value::Bool(x == y),
          (Value::String(x), Value::String(y)) => Value::Bool(x == y),
          _ => Value::Bool(false),
        };
        self.push(result);
      }
      OpCode::Neq => {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (a, b) {
          (Value::Int(x), Value::Int(y)) => Value::Bool(x != y),
          (Value::Float(x), Value::Float(y)) => Value::Bool(x != y),
          (Value::Bool(x), Value::Bool(y)) => Value::Bool(x != y),
          (Value::String(x), Value::String(y)) => Value::Bool(x != y),
          _ => Value::Bool(true),
        };
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
        self.call_stack.push(self.pc);
        self.pc = instr.arg as usize - 1;
      }
      OpCode::Return => {
        if let Some(return_addr) = self.call_stack.pop() {
          self.pc = return_addr;
        } else {
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
        self.push(Value::List(elements));
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
        self.push(Value::Map(map));
      }
      OpCode::Index => {
        let index = self.pop()?;
        let target = self.pop()?;

        match (target, index) {
          (Value::List(list), Value::Int(i)) => {
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
          (Value::Map(map), key) => {
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
            let idx = if i < 0 {
              (data.len() as i64 + i) as usize
            } else {
              i as usize
            };
            if idx < data.len() {
              self.push(Value::Float(data[idx]));
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
          (Value::List(mut list), Value::Int(i), val) => {
            let idx = if i < 0 {
              (list.len() as i64 + i) as usize
            } else {
              i as usize
            };
            if idx < list.len() {
              list[idx] = val;
              self.push(Value::List(list));
            } else {
              return Err("List index out of bounds".to_string());
            }
          }
          (Value::Map(mut map), key, val) => {
            if let Some(k) = key.to_key() {
              map.insert(k, val);
              self.push(Value::Map(map));
            } else {
              return Err("Invalid map key".to_string());
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
          (Value::List(list), Value::Int(s), Value::Int(e)) => {
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
              self.push(Value::List(list[start_idx..end_idx].to_vec()));
            } else {
              self.push(Value::List(vec![]));
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
        let var_idx = instr.arg as usize;

        let items = match iterable {
          Value::List(list) => list,
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
          Value::Tensor { data, .. } => data.iter().map(|&x| Value::Float(x)).collect(),
          Value::Map(map) => map
            .iter()
            .map(|(k, v)| Value::List(vec![Value::String(k.clone()), v.clone()]))
            .collect(),
          _ => return Err("Cannot iterate over this type".to_string()),
        };

        self.for_in_stack.push(ForInIterator {
          items,
          index: 0,
          var_idx,
        });
      }
      OpCode::ForInNext => {
        if let Some(iterator) = self.for_in_stack.last_mut() {
          if iterator.index < iterator.items.len() {
            let item = iterator.items[iterator.index].clone();
            self.variables[iterator.var_idx] = item;
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
        if let Value::List(shape_list) = shape_val {
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

          self.push(Value::Tensor { shape, data });
        } else {
          return Err("Invalid tensor shape".to_string());
        }
      }
      OpCode::MatMul => {
        let _b = self.pop()?;
        let _a = self.pop()?;
        self.push(Value::Tensor {
          shape: vec![1, 1],
          data: vec![1.0],
        });
      }
      OpCode::Relu => {
        let tensor = self.pop()?;
        if let Value::Tensor { shape, data } = tensor {
          let new_data: Vec<f64> = data.iter().map(|&x| x.max(0.0)).collect();
          self.push(Value::Tensor {
            shape,
            data: new_data,
          });
        } else {
          return Err("ReLU requires tensor".to_string());
        }
      }
      OpCode::Sigmoid => {
        let tensor = self.pop()?;
        if let Value::Tensor { shape, data } = tensor {
          let new_data: Vec<f64> = data.iter().map(|&x| 1.0 / (1.0 + (-x).exp())).collect();
          self.push(Value::Tensor {
            shape,
            data: new_data,
          });
        } else {
          return Err("Sigmoid requires tensor".to_string());
        }
      }
      OpCode::Tanh => {
        let tensor = self.pop()?;
        if let Value::Tensor { shape, data } = tensor {
          let new_data: Vec<f64> = data.iter().map(|&x| x.tanh()).collect();
          self.push(Value::Tensor {
            shape,
            data: new_data,
          });
        } else {
          return Err("Tanh requires tensor".to_string());
        }
      }
      OpCode::Softmax => {
        let tensor = self.pop()?;
        if let Value::Tensor { shape, data } = tensor {
          let max_val = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
          let exp_vals: Vec<f64> = data.iter().map(|&x| (x - max_val).exp()).collect();
          let sum: f64 = exp_vals.iter().sum();
          let new_data: Vec<f64> = exp_vals.iter().map(|&x| x / sum).collect();
          self.push(Value::Tensor {
            shape,
            data: new_data,
          });
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
      OpCode::MatchBegin | OpCode::MatchCase | OpCode::MatchEnd => {
        // Pattern matching is handled inline during compilation
      }
    }
    Ok(())
  }
}
