use crate::{
    codegen::{
        def::{
            ContainerDef,
            FunctionDef,
            InterfaceDef
        },
        register::{
            Register,
            RegisterAllocator
        },
        compiler::{
            CompilerResult,
            CompilerError,
            Compiler
        }
    },
    parser::{
        ast::{
            Type
        }
    }
};

use std::{
    collections::{
        HashMap
    }
};

#[derive(Debug, Clone)]
pub struct ModuleContext {
    pub name: String,
    pub modules: HashMap<String, ModuleContext>,
    pub functions: HashMap<String, FunctionDef>,
    pub containers: HashMap<String, ContainerDef>,
    pub interfaces: HashMap<String, InterfaceDef>,
    pub imports: HashMap<String, String>
}

impl ModuleContext {
    /// Creates a new module context
    pub fn new(name: String) -> ModuleContext {
        ModuleContext {
            name: name,
            modules: HashMap::new(),
            functions: HashMap::new(),
            containers: HashMap::new(),
            interfaces: HashMap::new(),
            imports: HashMap::new()
        }
    }

    /// Adds a function definition to a module context.
    /// Throws a DuplicateFunctionError if a function with the 
    /// same name already exists.
    pub fn add_function(&mut self, def: FunctionDef) -> CompilerResult<()> {
        if self.functions.contains_key(&def.name) {
            return Err(CompilerError::DuplicateFunction(def.name));
        }
        self.functions.insert(def.name.clone(), def);
        Ok(())
    }

    /// Adds a module context to a module context.
    /// Throws a DuplicateModuleError if a module with the
    /// same name already exists.
    pub fn add_module(&mut self, mod_ctx: ModuleContext) -> CompilerResult<()> {
        if self.modules.contains_key(&mod_ctx.name) {
            return Err(CompilerError::DuplicateModule(mod_ctx.name));
        }
        self.modules.insert(mod_ctx.name.clone(), mod_ctx);
        Ok(())
    }

    /// Adds a container definition to a module context.
    /// Throws a DuplicateContainerError if a container with the
    /// same name already exists.
    pub fn add_container(&mut self, cont_def: ContainerDef) -> CompilerResult<()> {
        if self.containers.contains_key(&cont_def.name) {
            return Err(CompilerError::DuplicateContainer(cont_def.name));
        }
        self.containers.insert(cont_def.name.clone(), cont_def);
        Ok(())
    }

    /// Adds an import declaration to a module context
    /// Throws a DuplicateImportError if an import with the same
    /// "import_as" name already exists.
    pub fn add_import(&mut self, import_as: String, import_path: String) -> CompilerResult<()> {
        if self.imports.contains_key(&import_as) {
            return Err(CompilerError::DuplicateImport(import_as));
        }
        self.imports.insert(import_as, import_path);
        Ok(())
    }

    /// Adds an interface
    pub fn add_interface(&mut self, intf_def: InterfaceDef)  {
        self.interfaces.insert(intf_def.name.clone(), intf_def);
    }

    /// Gets an interface
    pub fn get_interface(&self, intf_name: &str) -> CompilerResult<&InterfaceDef> {
        self.interfaces.get(intf_name).ok_or(CompilerError::UnknownContainer(String::from(intf_name)))
    }

    /// Gets a mutable reference to a container definition, given the name
    pub fn get_container_mut(&mut self, name: &String) -> CompilerResult<&mut ContainerDef> {
        self.containers.get_mut(name)
            .ok_or(CompilerError::UnknownContainer(name.clone()))
    }

    /// Gets a reference to a container definition
    pub fn get_container(&self, name: &String) -> CompilerResult<&ContainerDef> {
        self.containers.get(name)
            .ok_or(CompilerError::UnknownContainer(name.clone()))
    }

    /// Gets a reference to the function definition, given the name
    pub fn get_function(&self, name: &String) -> CompilerResult<&FunctionDef> {
        self.functions.get(name)
            .ok_or(CompilerError::UnknownFunction(name.clone()))
    }
}

#[derive(Debug, Clone)]
pub enum VariableLocation {
    Stack(i64),
    Register(Register)
}

#[derive(PartialEq, Debug, Clone)]
pub struct FunctionContext {
    pub def: Option<FunctionDef>,
    pub weak: bool,
    pub is_loop: bool,
    pub stack_size: usize,
    variable_types: HashMap<String, Type>,
    variable_positions: HashMap<String, i64>,
    pub register_allocator: RegisterAllocator
}

impl FunctionContext {
    pub fn new(compiler: &Compiler, def: FunctionDef) -> CompilerResult<FunctionContext> {
        let mut variable_types = HashMap::new();
        let mut variable_positions = HashMap::new();
        let mut pos: i64 = 0;

        for (_, arg_type) in def.arguments.iter().rev() {
            let size_of_type = compiler.get_size_of_type(arg_type)?;
            pos -= size_of_type as i64;
        }

        for (arg_name, arg_type) in def.arguments.iter() {
            let size_of_type = compiler.get_size_of_type(arg_type)?;
            variable_types.insert(arg_name.clone(), arg_type.clone());
            variable_positions.insert(arg_name.clone(), pos);
            pos += size_of_type as i64;
        }

        Ok(
            FunctionContext {
                def: Some(def),
                weak: false,
                is_loop: false,
                stack_size: 0,
                variable_types: variable_types,
                variable_positions: variable_positions,
                register_allocator: RegisterAllocator::new()
            }
        )
    }

    pub fn new_weak(fn_ctx: &FunctionContext) -> CompilerResult<FunctionContext> {
        let stack_size = fn_ctx.stack_size as i64;

        let mut variable_positions = HashMap::new();

        for (var_name, var_pos) in fn_ctx.variable_positions.iter() {
            let var_offset = var_pos - stack_size;
            if var_offset >= 0 {
                return Err(CompilerError::Unknown);
            }
            variable_positions.insert(var_name.clone(), var_offset);
        }

        Ok(
            FunctionContext {
                def: None,
                weak: true,
                is_loop: false,
                stack_size: 0,
                variable_types: fn_ctx.variable_types.clone(),
                variable_positions: variable_positions,
                register_allocator: RegisterAllocator::new()
            }
        )
    }

    pub fn new_loop(fn_ctx: &FunctionContext) -> CompilerResult<FunctionContext> {
        let stack_size = fn_ctx.stack_size as i64;

        let mut variable_positions = HashMap::new();

        for (var_name, var_pos) in fn_ctx.variable_positions.iter() {
            let var_offset = var_pos - stack_size;
            if var_offset >= 0 {
                return Err(CompilerError::Unknown);
            }
            variable_positions.insert(var_name.clone(), var_offset);
        }

        Ok(
            FunctionContext {
                def: None,
                weak: true,
                is_loop: true,
                stack_size: 0,
                variable_types: fn_ctx.variable_types.clone(),
                variable_positions: variable_positions,
                register_allocator: RegisterAllocator::new()
            }
        )
    }

    pub fn set_stack_var(&mut self, (var_name, var_type): (String, Type), stack_pos: i64) -> CompilerResult<()> {
        if self.variable_types.contains_key(&var_name) {
            return Err(CompilerError::DuplicateVariable(var_name));
        } else if self.variable_positions.contains_key(&var_name) {
            return Err(CompilerError::DuplicateVariable(var_name));
        }
        self.variable_types.insert(var_name.clone(), var_type);
        self.variable_positions.insert(var_name, stack_pos);
        Ok(())
    }

    pub fn get_var_type(&self, var_name: &String) -> CompilerResult<Type> {
        self.variable_types.get(var_name)
            .cloned()
            .ok_or(CompilerError::UnknownVariable(var_name.clone()))
    }

    pub fn get_var_loc(&self, var_name: &String) -> CompilerResult<VariableLocation> {
        /*let reg_res = self.register_allocator.get_permanent(var_name);
        if reg_res.is_ok() {
            return Ok(VariableLocation::Register(reg_res.unwrap()));
        }*/
        let position = self.variable_positions.get(var_name)
            .ok_or(CompilerError::UnknownVariable(var_name.clone()))?;
        Ok(
            VariableLocation::Stack(*position)
        )
    }

    pub fn get_var_pos(&self, var_name: &String) -> CompilerResult<i64> {
        self.variable_positions.get(var_name)
            .cloned()
            .ok_or(CompilerError::UnknownVariable(var_name.clone()))
    }

    pub fn get_ret_type(&self) -> CompilerResult<Type> {
        let fn_def = self.def.as_ref()
            .ok_or(CompilerError::Unknown)?;
        Ok(
            fn_def.ret_type.clone()
        )
    }
}

#[derive(Clone)]
pub struct LoopContext {
    pub pos_start: usize,
    pub tag_end: u64
}

impl LoopContext {
    pub fn new(start: usize, tag_end: u64) -> LoopContext {
        LoopContext {
            pos_start: start,
            tag_end: tag_end
        }
    }
}