// ============================================================================
// INSTRUCTION SET
// ============================================================================

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum OpCode {
  Nil,
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
  CallDirect,
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
  Dup,
  Halt,
  Raise,
  SetupTry,
  PopTry,
  BeginFinally,
  EndFinally,
  AssertType,
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
  NewInstance,
  GetField,
  SetField,
  CallMethod,
  LoadSelf,
  CallSuper,
}

impl fmt::Display for OpCode {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      OpCode::Nil => write!(f, "nil"),
      OpCode::LoadConst => write!(f, "load_const"),
      OpCode::LoadVar => write!(f, "load_var"),
      OpCode::StoreVar => write!(f, "store_var"),
      OpCode::Add => write!(f, "add"),
      OpCode::Sub => write!(f, "sub"),
      OpCode::Mul => write!(f, "mul"),
      OpCode::Div => write!(f, "div"),
      OpCode::Mod => write!(f, "mod"),
      OpCode::Pow => write!(f, "pow"),
      OpCode::Floor => write!(f, "floor"),
      OpCode::Neg => write!(f, "neg"),
      OpCode::Eq => write!(f, "eq"),
      OpCode::Neq => write!(f, "neq"),
      OpCode::Lt => write!(f, "lt"),
      OpCode::Gt => write!(f, "gt"),
      OpCode::Lte => write!(f, "lte"),
      OpCode::Gte => write!(f, "gte"),
      OpCode::And => write!(f, "and"),
      OpCode::Or => write!(f, "or"),
      OpCode::Not => write!(f, "not"),
      OpCode::Jump => write!(f, "jump"),
      OpCode::JumpIfFalse => write!(f, "jump_if_false"),
      OpCode::Call => write!(f, "call"),
      OpCode::CallDirect => write!(f, "call_direct"),
      OpCode::Return => write!(f, "return"),
      OpCode::TensorCreate => write!(f, "tensor_create"),
      OpCode::MatMul => write!(f, "matmul"),
      OpCode::Relu => write!(f, "relu"),
      OpCode::Sigmoid => write!(f, "sigmoid"),
      OpCode::Tanh => write!(f, "tanh"),
      OpCode::Softmax => write!(f, "softmax"),
      OpCode::BuildList => write!(f, "build_list"),
      OpCode::BuildMap => write!(f, "build_map"),
      OpCode::Print => write!(f, "print"),
      OpCode::Dup => write!(f, "dup"),
      OpCode::Pop => write!(f, "pop"),
      OpCode::Halt => write!(f, "halt"),
      OpCode::Raise => write!(f, "raise"),
      OpCode::SetupTry => write!(f, "setup_try"),
      OpCode::PopTry => write!(f, "pop_try"),
      OpCode::BeginFinally => write!(f, "begin_finally"),
      OpCode::EndFinally => write!(f, "end_finally"),
      OpCode::AssertType => write!(f, "assert_type"),
      OpCode::Index => write!(f, "index"),
      OpCode::IndexSet => write!(f, "index_set"),
      OpCode::Slice => write!(f, "slice"),
      OpCode::BuildRange => write!(f, "build_range"),
      OpCode::SetupForIn => write!(f, "setup_for_in"),
      OpCode::ForInNext => write!(f, "for_in_next"),
      OpCode::PopForIn => write!(f, "pop_for_in"),
      OpCode::MatchBegin => write!(f, "match_begin"),
      OpCode::MatchCase => write!(f, "match_case"),
      OpCode::MatchEnd => write!(f, "match_end"),
      OpCode::NewInstance => write!(f, "instance"),
      OpCode::GetField => write!(f, "get_field"),
      OpCode::SetField => write!(f, "set_field"),
      OpCode::CallMethod => write!(f, "call_method"),
      OpCode::LoadSelf => write!(f, "load_self"),
      OpCode::CallSuper => write!(f, "call_super"),
    }
  }
}

#[derive(Debug, Clone)]
pub struct Instruction {
  pub opcode: OpCode,
  pub arg: i32,
}

impl fmt::Display for Instruction {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if self.arg == 0 {
      write!(f, "{}", self.opcode)
    } else {
      write!(f, "{} {}", self.opcode, self.arg)
    }
  }
}
