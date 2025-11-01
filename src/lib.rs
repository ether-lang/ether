// ============================================================================
// MAIN API
// ============================================================================

use std::{collections::HashMap, path::Path};

pub mod ast;
pub mod compiler;
pub mod instruction;
pub mod lexer;
pub mod module;
pub mod parser;
pub mod types;
pub mod value;
pub mod vm;

pub struct Ether {
  vm: vm::VM,
}

pub type Result<T> = std::result::Result<T, String>;

impl Ether {
  pub fn new() -> Self {
    Self {
      vm: vm::VM::new(vec![], vec![], HashMap::new()),
    }
  }

  pub fn execute(&mut self, source: &str) -> Result<()> {
    let tokens = lexer::tokenize(source)?;
    let ast = parser::parse(tokens)?;
    let bytecode = compiler::compile(&ast)?;
    self.vm.reset(
      bytecode.get_instructions().to_vec(),
      bytecode.get_constants().to_vec().as_mut(),
    );
    self.vm.run()
  }

  pub fn execute_repl(&mut self, source: &str) -> Result<()> {
    let tokens = lexer::tokenize(source)?;
    let ast = parser::parse(tokens)?;

    let filepath = std::path::Path::new("<repl>.eth");

    let mut loader = module::ModuleLoader::new();
    loader.set_startup_script_dir(filepath);

    let mut compiler = compiler::Compiler::new();
    compiler.set_current_file(filepath.to_path_buf());
    compiler.set_module_loader(loader);
    compiler.compile(&ast)?;

    self.vm.reset(
      compiler.get_instructions().to_vec(),
      compiler.get_constants().to_vec().as_mut(),
    );
    let result = self.vm.run();

    if result.is_ok() && self.vm.stack.len() > 0 {
      println!("{:?}", self.vm.stack.pop().unwrap());
    }

    result
  }

  pub fn execute_file(&mut self, path: &str) -> Result<()> {
    compile_and_run_file(Path::new(path))
  }
}

impl Default for Ether {
  fn default() -> Self {
    Self::new()
  }
}

pub fn compile_and_run_file(filepath: &Path) -> Result<()> {
  let source = std::fs::read_to_string(filepath)
    .map_err(|e| format!("Failed to read file '{}': {}", filepath.display(), e))?;

  compile_and_run(&source, filepath)
}

pub fn compile_and_run(source: &str, filepath: &Path) -> Result<()> {
  let mut lexer = lexer::Lexer::new(&source);
  let tokens = lexer.tokenize()?;

  let mut parser = parser::Parser::new(tokens);
  let ast = parser.parse()?;

  let mut loader = module::ModuleLoader::new();
  loader.set_startup_script_dir(filepath);

  let mut compiler = compiler::Compiler::new();
  compiler.set_current_file(filepath.to_path_buf());
  compiler.set_module_loader(loader);
  compiler.compile(&ast)?;

  let mut vm = vm::VM::new(
    compiler.get_instructions().to_vec(),
    compiler.get_constants().to_vec(),
    compiler.get_global_var_names().clone(),
  );
  vm.run()?;

  Ok(())
}
