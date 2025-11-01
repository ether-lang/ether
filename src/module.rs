use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::compiler::Compiler;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::value::{FunctionDef, Value};
use crate::vm::VM;

#[derive(Debug, Clone)]
pub struct Module {
  pub name: String,
  pub path: PathBuf,
  pub exports: HashMap<String, Value>,
}

impl Module {
  pub fn new(name: String, path: PathBuf) -> Self {
    Module {
      name,
      path,
      exports: HashMap::new(),
    }
  }

  pub fn is_public(name: &str) -> bool {
    !name.starts_with('_')
  }

  pub fn add_export(&mut self, name: String, value: Value) {
    if Self::is_public(&name) {
      self.exports.insert(name, value);
    }
  }

  pub fn add_function(&mut self, name: String, addr: usize) {
    if Self::is_public(&name) {
      let func_def = FunctionDef {
        name: name.clone(),
        address: addr,
        module_id: Some(self.name.clone()),
      };
      // Store as an exportable value
      self
        .exports
        .insert(name, Value::Function(Rc::new(func_def)));
    }
  }

  pub fn get_export(&self, name: &str) -> Option<&Value> {
    if Self::is_public(name) {
      self.exports.get(name)
    } else {
      None
    }
  }
}

pub struct ModuleLoader {
  loaded_modules: HashMap<String, Rc<Module>>,
  runtime_dir: PathBuf,
  startup_script_dir: PathBuf,
}

impl ModuleLoader {
  pub fn new() -> Self {
    let runtime_dir = env::current_exe()
      .ok()
      .and_then(|p| p.parent().map(|p| p.join("libs")))
      .unwrap_or_else(|| PathBuf::from("libs"));

    ModuleLoader {
      loaded_modules: HashMap::new(),
      runtime_dir,
      startup_script_dir: PathBuf::from("."),
    }
  }

  pub fn set_startup_script_dir(&mut self, path: &Path) {
    if let Some(parent) = path.parent() {
      self.startup_script_dir = parent.to_path_buf();
    }
  }

  fn resolve_module_path(
    &self,
    module_path: &str,
    requesting_file: &Path,
  ) -> Result<PathBuf, String> {
    // Remove .eth extension if provided
    let clean_path = if module_path.ends_with(".eth") {
      &module_path[..module_path.len() - 4]
    } else {
      module_path
    };

    if let Some(std_name) = clean_path.strip_prefix("std:") {
      // Standard library module
      self.resolve_std_module(std_name)
    } else if let Some(mod_name) = clean_path.strip_prefix("mod:") {
      // User module in .ether directory
      self.resolve_mod_module(mod_name)
    } else {
      // Relative module
      self.resolve_relative_module(clean_path, requesting_file)
    }
  }

  fn resolve_std_module(&self, name: &str) -> Result<PathBuf, String> {
    // Try <runtime_dir>/libs/<name>.eth
    let direct_path = self.runtime_dir.join(format!("{}.eth", name));
    if direct_path.exists() {
      return Ok(direct_path);
    }

    // Try <runtime_dir>/libs/<name>/index.eth
    let index_path = self.runtime_dir.join(name).join("index.eth");
    if index_path.exists() {
      return Ok(index_path);
    }

    Err(format!("Standard library module 'std:{}' not found", name))
  }

  fn resolve_mod_module(&self, name: &str) -> Result<PathBuf, String> {
    let ether_dir = self.startup_script_dir.join(".ether");

    // Try <startup_dir>/.ether/<name>.eth
    let direct_path = ether_dir.join(format!("{}.eth", name));
    if direct_path.exists() {
      return Ok(direct_path);
    }

    // Try <startup_dir>/.ether/<name>/index.eth
    let index_path = ether_dir.join(name).join("index.eth");
    if index_path.exists() {
      return Ok(index_path);
    }

    Err(format!(
      "Module 'mod:{}' not found in .ether directory",
      name
    ))
  }

  fn resolve_relative_module(&self, name: &str, requesting_file: &Path) -> Result<PathBuf, String> {
    let base_dir = requesting_file.parent().unwrap_or_else(|| Path::new("."));

    // Try <base_dir>/<name>.eth
    let direct_path = base_dir.join(format!("{}.eth", name));
    if direct_path.exists() {
      return Ok(direct_path);
    }

    // Try <base_dir>/<name>/index.eth
    let index_path = base_dir.join(name).join("index.eth");
    if index_path.exists() {
      return Ok(index_path);
    }

    Err(format!(
      "Module '{}' not found relative to {:?}",
      name, requesting_file
    ))
  }

  pub fn load_module(
    &mut self,
    module_path: &str,
    requesting_file: &Path,
  ) -> Result<Rc<Module>, String> {
    // Resolve the actual file path
    let resolved_path = self.resolve_module_path(module_path, requesting_file)?;

    // Check if already loaded (use canonical path as key)
    let canonical_path = resolved_path
      .canonicalize()
      .unwrap_or_else(|_| resolved_path.clone());
    let cache_key = canonical_path.to_string_lossy().to_string();

    if let Some(module) = self.loaded_modules.get(&cache_key) {
      return Ok(Rc::clone(module));
    }

    // Load and parse the module
    let source = std::fs::read_to_string(&resolved_path)
      .map_err(|e| format!("Failed to read module at {:?}: {}", resolved_path, e))?;

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize()?;

    let mut parser = Parser::new(tokens);
    let ast = parser.parse()?;

    // Create module
    let module_name = resolved_path
      .file_stem()
      .and_then(|s| s.to_str())
      .unwrap_or("unknown")
      .to_string();

    // Compile the module
    let mut module_compiler = Compiler::new();
    module_compiler.set_current_file(resolved_path.clone());
    module_compiler.set_module_loader(self.clone_for_nested());

    module_compiler.compile(&ast)?;

    // Create module
    let mut module = Module::new(module_name, resolved_path.clone());

    // Extract public functions
    for (name, addr) in &module_compiler.function_addresses {
      if Module::is_public(name) {
        module.add_function(name.clone(), *addr);
      }
    }

    // Execute module to get variable values
    let instructions = module_compiler.get_instructions().to_vec();
    let constants = module_compiler.get_constants().to_vec();
    let global_var_names = module_compiler.get_global_var_names().clone();

    let mut vm = VM::new(instructions, constants, global_var_names);
    vm.run()?;

    // Get all public variables from VM
    for (name, value) in vm.get_global_variables() {
      if Module::is_public(&name) {
        module.exports.insert(name, value);
      }
    }

    // Store module with its bytecode (needed for function calls)
    // We need to keep the instructions and constants accessible
    // So we'll need to store them in the module

    // Cache the module
    let module_rc = Rc::new(module);
    self.loaded_modules.insert(cache_key, Rc::clone(&module_rc));

    Ok(module_rc)
  }

  fn clone_for_nested(&self) -> ModuleLoader {
    ModuleLoader {
      loaded_modules: self.loaded_modules.clone(),
      runtime_dir: self.runtime_dir.clone(),
      startup_script_dir: self.startup_script_dir.clone(),
    }
  }
}
