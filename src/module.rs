// module.rs
use std::collections::HashMap;
use crate::Value;

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub members: HashMap<String, Value>,
}

impl Module {
    pub fn new(name: String) -> Self {
        Module {
            name,
            members: HashMap::new()
        }
    }

    pub fn get_public_member(&self, name: &str) -> Option<&Value> {
        if name.starts_with('_') {
            None
        } else {
            self.members.get(name)
        }
    }

    pub fn get_member(&self, name: &str) -> Option<&Value> {
        self.members.get(name)
    }

    pub fn set_member(&mut self, name: String, value: Value) {
        self.members.insert(name, value);
    }
}

#[derive(Default)]
pub struct ModuleRegistry {
    modules: HashMap<String, Module>
}

impl ModuleRegistry {
    pub fn new() -> Self {
        ModuleRegistry {
            modules: HashMap::new()
        }
    }

    pub fn register_module(&mut self, module: Module) {
        self.modules.insert(module.name.clone(), module);
    }

    pub fn get_module(&self, name: &str) -> Option<&Module> {
        self.modules.get(name)
    }

    pub fn get_module_member(&self, module_name: &str, member_name: &str) -> Option<&Value> {
        self.modules.get(module_name)
            .and_then(|m| m.get_public_member(member_name))
    }
}