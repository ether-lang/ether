// ============================================================================
// MAIN API
// ============================================================================

pub mod ast;
pub mod compiler;
pub mod lexer;
pub mod parser;
pub mod types;
pub mod vm;

pub struct Ether {
    vm: vm::VM,
}

pub type Result<T> = std::result::Result<T, String>;

impl Ether {
    pub fn new() -> Self {
        Self {
            vm: vm::VM::new(vec![], vec![]),
        }
    }

    pub fn execute(&mut self, source: &str) -> Result<()> {
        let tokens = lexer::tokenize(source)?;
        let ast = parser::parse(tokens)?;
        let bytecode = compiler::compile(&ast)?;
        self.vm.reset(bytecode.get_instructions().to_vec(), bytecode.get_constants().to_vec().as_mut());
        self.vm.run()
    }

    pub fn execute_repl(&mut self, source: &str) -> Result<()> {
        let tokens = lexer::tokenize(source)?;
        let ast = parser::parse(tokens)?;
        let bytecode = compiler::compile_repl(&ast)?;
        self.vm.reset(bytecode.get_instructions().to_vec(), bytecode.get_constants().to_vec().as_mut());
        let result =self.vm.run();

        if result.is_ok() && self.vm.stack.len() > 0 {
          println!("{:?}", self.vm.stack.pop().unwrap());
        }

        result
    }

    pub fn execute_file(&mut self, path: &str) -> Result<()> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        self.execute(&source)
    }
}

impl Default for Ether {
    fn default() -> Self {
        Self::new()
    }
}


pub fn compile_and_run(source: &str) -> Result<()> {
  let mut lexer = lexer::Lexer::new(source);
  let tokens = lexer.tokenize()?;

  let mut parser = parser::Parser::new(tokens);
  let ast = parser.parse()?;

  let mut compiler = compiler::Compiler::new();
  compiler.compile(&ast)?;

  let instructions = compiler.get_instructions().to_vec();
  let constants = compiler.get_constants().to_vec();
  let mut vm = vm::VM::new(instructions, constants);
  vm.run()?;

  Ok(())
}
