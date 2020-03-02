use crate::{
    api::{
        module::Module,
        function::Function
    },
    codegen::{
        context::{
            ModuleContext,
            FunctionContext,
            VariableLocation,
            LoopContext
        },
        uid_generator::UIDGenerator,
        def::{
            ContainerDef,
            FunctionDef
        },
        builder::{
            Builder
        },
        register::{
            Register
        },
        instruction::{
            Instruction
        },
        data::{
            Data
        },
        program::{
            Program
        }
    },
    parser::{
        ast::{
            Declaration,
            Statement,
            Type,
            Expression,
            IfStatementArgs
        }
    },
    vm::{
        is::{
            Opcode
        }
    }
};

use std::{
    fmt::{
        Display,
        Result as FmtResult,
        Formatter
    },
    error::Error,
    collections::{
        VecDeque,
        HashMap,
        HashSet
    },
    ops::{
        Deref,
        DerefMut
    },
    collections::{
        BTreeMap
    },
    path::{
        PathBuf,
        Path
    }
};

#[derive(Debug, Clone)]
pub enum CompilerError {
    Unknown,
    Unimplemented(String),
    DuplicateVariable(String),
    DuplicateMember(String),
    DuplicateFunction(String),
    DuplicateModule(String),
    DuplicateContainer(String),
    DuplicateImport(String),
    UnknownFunction(String),
    UnknownContainer(String),
    UnknownVariable(String),
    UnknownModule(String),
    UnknownType(Type),
    UnknownMember(String),
    UnsupportedExpression(Expression),
    InvalidModulePath(String),
    AlreadyContainsContainer(String),
    AlreadyContainsModule(String),
    NotAMemberFunction(String),
    ArgumentMismatch(String),
    MemberAccessOnNonContainer,
    TypeMismatch(Type, Type),
    CannotDerefNonPointer,
    CannotDerefSlice,
    RegisterMapping
}

impl Display for CompilerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}

impl Error for CompilerError {}

/// Convenience type for Results returned by a compilation process
pub type CompilerResult<T> = Result<T, CompilerError>;

/// The compiler
pub struct Compiler {
    fn_context_stack: VecDeque<FunctionContext>,
    mod_context_stack: VecDeque<ModuleContext>,
    loop_ctx_stack: VecDeque<LoopContext>,
    fn_uid_map: HashMap<String, u64>,
    foreign_functions: Option<HashMap<u64, Function>>,
    foreign_function_uids: HashSet<u64>,
    uid_generator: UIDGenerator,
    builder: Builder,
    current_cont: Option<String>,
    data: Data
}

impl Compiler {
    /// Creates a new compiler instance and pushes the "root" module on the context stack
    pub fn new() -> Compiler {
        let root_mod_ctx = ModuleContext::new(String::from("root"));
        let mut mod_context_stack = VecDeque::new();
        mod_context_stack.push_front(root_mod_ctx);
        Compiler {
            fn_context_stack: VecDeque::new(),
            mod_context_stack: mod_context_stack,
            loop_ctx_stack: VecDeque::new(),
            fn_uid_map: HashMap::new(),
            foreign_functions: Some(HashMap::new()),
            foreign_function_uids: HashSet::new(),
            uid_generator: UIDGenerator::new(),
            builder: Builder::new(),
            current_cont: None,
            data: Data::new()
        }
    }

    /// Retrieves a reference to the underlying builder
    pub fn get_builder(&self) -> &Builder {
        &self.builder
    }

    /// Retrieves the program instance compiled by this compiler instance.
    pub fn get_program(&mut self) -> CompilerResult<Program> {
        let mut builder = self.builder.clone();
        let data = self.data.clone();
        let data_len = data.bytes.len();

        // Modify target jump addresses of JMP instructions accordingly 
        for offset in builder.jmp_instructions.clone().iter() {
            let instr = builder.get_instr(offset)
                .ok_or(CompilerError::Unknown)?;
            let addr: u64 = match instr.opcode {
                Opcode::JMP => instr.get_operand(0, 8),
                Opcode::JMPF => instr.get_operand(1, 8),
                Opcode::JMPT => instr.get_operand(1, 8),
                _ => return Err(CompilerError::Unknown)
            };
            instr.remove_operand_bytes(8);
            instr.append_operand(addr + data_len as u64);
        }

        let mut functions: HashMap<u64, usize> = HashMap::new();

        // correctly set function offsets
        for (fn_name, fn_uid) in self.fn_uid_map.iter() {
            if self.is_function_foreign(*fn_uid)? {
                continue;
            }
            let fn_offset = builder.get_label_offset(fn_name)
                .ok_or(CompilerError::Unknown)?;
            functions.insert(fn_uid.clone(), fn_offset + data_len);
        }

        let foreign_functions = self.foreign_functions.take()
            .ok_or(CompilerError::Unknown)?;


        let mut code = data.bytes;
        let mut builder_code = builder.build();
        //println!("Data length: {}", code.len());
        code.append(&mut builder_code);

        let program = Program::new()
            .with_code(code)
            .with_functions(functions)
            .with_foreign_functions(foreign_functions);
        
        Ok(program)
    }

    // #region helpers

    /// Gets the module path on the stack, with trailing "::"
    pub fn get_module_path(&self) -> String {
        let mut ret = String::new();
        for mod_ctx in self.mod_context_stack.iter().rev() {
            ret += &mod_ctx.name;
            ret += "::"
        }
        ret
    }

    /// Gets the current module context (the one at the top of the stack)
    pub fn get_current_module(&self) -> CompilerResult<&ModuleContext> {
        self.mod_context_stack.get(0)
            .ok_or(CompilerError::Unknown)
    }

    /// Gets the root module context (mutable)
    pub fn get_root_module_mut(&mut self) -> CompilerResult<&mut ModuleContext> {
        self.mod_context_stack.get_mut(self.mod_context_stack.len() - 1)
            .ok_or(CompilerError::Unknown)
    }

    /// Gets the root module context
    pub fn get_root_module(&self) -> CompilerResult<&ModuleContext> {
        self.mod_context_stack.get(self.mod_context_stack.len() - 1)
            .ok_or(CompilerError::Unknown)
    } 

    /// Gets the current module context (the one at the top of the stack) as a mutable reference
    pub fn get_current_module_mut(&mut self) -> CompilerResult<&mut ModuleContext> {
        self.mod_context_stack.get_mut(0)
            .ok_or(CompilerError::Unknown)
    }

    /// Gets the current function context as a reference
    pub fn get_current_function(&self) -> CompilerResult<&FunctionContext> {
        self.fn_context_stack.get(0)
            .ok_or(CompilerError::Unknown)
    }

    /// Gets the current function context as a mutable reference
    pub fn get_current_function_mut(&mut self) -> CompilerResult<&mut FunctionContext> {
        self.fn_context_stack.get_mut(0)
            .ok_or(CompilerError::Unknown)
    }

    /// Gets the next temporary register from the current context
    pub fn get_next_register(&mut self) -> CompilerResult<Register> {
        let fn_ctx = self.get_current_function_mut()?;
        fn_ctx.register_allocator.get_temp_register()
    }

    /// Gets the last temporary register from the current context
    pub fn get_last_register(&self) -> CompilerResult<Register> {
        let fn_ctx = self.get_current_function()?;
        fn_ctx.register_allocator.get_last_temp_register()
    }

    /// Gets the current loop context
    pub fn get_current_loop(&self) -> CompilerResult<&LoopContext> {
        self.loop_ctx_stack.get(0)
            .ok_or(CompilerError::Unknown)
    }

    /// Gets the function context at stack index
    pub fn get_function(&self, index: usize) -> CompilerResult<&FunctionContext> {
        self.fn_context_stack.get(index)
            .ok_or(CompilerError::Unknown)
    }

    /// Returns true if the given function uid is foreign
    pub fn is_function_foreign(&self, uid: u64) -> CompilerResult<bool> {
        Ok(self.foreign_function_uids.contains(&uid))
    }

    /// Gets the first parent non-weak function context
    pub fn get_parent_function(&self) -> CompilerResult<&FunctionContext> {
        self.fn_context_stack.iter().find(|fn_ctx| !fn_ctx.weak)
            .ok_or(CompilerError::Unknown)
    }

    /// Pushes a module context on the stack
    pub fn push_module_context(&mut self, mod_ctx: ModuleContext) {
        self.mod_context_stack.push_front(mod_ctx);
    }

    /// Pops the front module context off the stack
    pub fn pop_module_context(&mut self) -> CompilerResult<ModuleContext> {
        self.mod_context_stack.pop_front()
            .ok_or(CompilerError::Unknown)
    }

    /// Pushes a function context on the stack
    pub fn push_function_context(&mut self, fn_ctx: FunctionContext) {
        self.fn_context_stack.push_front(fn_ctx);
    }

    /// Pops the front function context off the stack
    pub fn pop_function_context(&mut self) -> CompilerResult<FunctionContext> {
        self.fn_context_stack.pop_front()
            .ok_or(CompilerError::Unknown)
    }

    /// Pushes a loop context on the stack
    pub fn push_loop_context(&mut self, loop_ctx: LoopContext) {
        self.loop_ctx_stack.push_front(loop_ctx);
    }
    
    /// Pops the front loop context off the stack
    pub fn pop_loop_context(&mut self) -> CompilerResult<LoopContext> {
        self.loop_ctx_stack.pop_front()
            .ok_or(CompilerError::Unknown)
    }

    /// Gets a functions uid  by name
    pub fn get_function_uid(&self, name: &String) -> CompilerResult<u64> {
        //println!("Getting function uid: {}", name);
        self.fn_uid_map.get(name)
            .cloned()
            .ok_or(CompilerError::UnknownFunction(name.clone()))
    }

    /// Resolves a function by name to a FunctionDef
    pub fn resolve_function(&self, name: &String) -> CompilerResult<FunctionDef> {
        //println!("Resolving function: {}", name);
        if name.contains("::") {
            let path_fragments: Vec<String> = name.split("::").map(|s| String::from(s)).collect();
            let mut mod_ctx_opt = None;
            let mut cont_def_opt = None;
            let mut start_i = 0;
            if path_fragments[0] == "root" {
                start_i = 1;
                mod_ctx_opt = Some(self.get_root_module()?);
            } else if path_fragments[0] == "super" {
                start_i = 1;
                return Err(CompilerError::Unimplemented(format!("Blub")));
            } else {
                mod_ctx_opt = Some(self.get_current_module()?);
            }

            if let Some(mod_ctx) = mod_ctx_opt {
                //println!("Is in root module");
                if !mod_ctx.modules.contains_key(&path_fragments[0]) {
                    mod_ctx_opt = Some(self.get_root_module()?);
                }
            }

            for i in start_i..path_fragments.len() - 1 {
                let mod_ctx = mod_ctx_opt.unwrap();
                if mod_ctx.containers.contains_key(&path_fragments[i]) {
                    //println!("Function is in container {}", &path_fragments[i]);
                    if i != path_fragments.len() - 2 {
                        //println!("i: {}, len: {}", i, path_fragments.len());
                        //println!("{:?}", path_fragments);
                        return Err(CompilerError::InvalidModulePath(name.clone()));
                    }
                    cont_def_opt = Some(mod_ctx.get_container(&path_fragments[i])?);
                    break;
                }
                //println!("Blub");
                mod_ctx_opt = mod_ctx.modules.get(&path_fragments[i]);
            }

            let last_path = path_fragments.last().unwrap();

            //println!("Resolving function {} for mod_ctx {}", last_path, mod_ctx_opt.as_ref().unwrap().name);
            if cont_def_opt.is_some() {
                let cont_def = cont_def_opt.unwrap();
                return Ok(
                    cont_def.get_member_function(last_path)?
                        .clone()
                )
            } else {
                //println!("Resolved {}. Was in module!", name);
                let mod_ctx = mod_ctx_opt.unwrap();
                //println!("Trying to resolve function {} in module {:?}.", last_path, mod_ctx);
                //println!("Blub");
                return mod_ctx.functions.get(last_path)
                    .cloned()
                    .ok_or(CompilerError::UnknownFunction(name.clone()));
            }
        } else {
            let mod_ctx = self.get_current_module()?;
            //println!("current mod ctx: {:?}", mod_ctx);
            if mod_ctx.functions.contains_key(name) {
                return mod_ctx.functions.get(name)
                    .cloned()
                    .ok_or(CompilerError::UnknownFunction(name.clone()));
            }
            if mod_ctx.imports.contains_key(name) {
                let import_path = mod_ctx.imports.get(name)
                    .ok_or(CompilerError::Unknown)?;
                return self.resolve_function(import_path);
            }
            return Err(CompilerError::UnknownFunction(name.clone()));
        }
    }

    /// Resolves a container by name to a ContainerDef
    pub fn resolve_container(&self, name: &String) -> CompilerResult<ContainerDef> {
        //println!("Resolving container by name {}", name);
        if name.contains("::") {
            let path_fragments: Vec<String> = name.split("::").map(|s| String::from(s)).collect();
            let mut mod_ctx_opt = None;
            let mut start_i = 0;
            if path_fragments[0] == "root" {
                start_i = 1;
                mod_ctx_opt = Some(self.get_root_module()?);
            } else if path_fragments[0] == "super" {
                start_i = 1;
                return Err(CompilerError::Unimplemented(format!("Blub")));
            } else {
                mod_ctx_opt = Some(self.get_current_module()?);
            }

            for i in start_i..path_fragments.len() - 1 {
                let mod_ctx = mod_ctx_opt.unwrap();
                //println!("Blub");
                mod_ctx_opt = mod_ctx.modules.get(&path_fragments[i]);
            }

            let last_path = path_fragments.last().unwrap();

            //println!("Resolving function {} for mod_ctx {}", last_path, mod_ctx_opt.as_ref().unwrap().name);

            let mod_ctx = mod_ctx_opt.unwrap();
            return mod_ctx.containers.get(last_path)
                .cloned()
                .ok_or(CompilerError::UnknownContainer(name.clone()));
        } else {
            let mod_ctx = self.get_current_module()?;
            if mod_ctx.containers.contains_key(name) {
                return mod_ctx.containers.get(name)
                    .cloned()
                    .ok_or(CompilerError::UnknownContainer(name.clone()));
            }
            if mod_ctx.imports.contains_key(name) {
                let import_path = mod_ctx.imports.get(name)
                    .ok_or(CompilerError::Unknown)?;
                return self.resolve_container(import_path);
            }

            return Err(CompilerError::UnknownContainer(name.clone()));
        }
    }

    /// Returns the byte size of a given Type
    pub fn get_size_of_type(&self, var_type: &Type) -> CompilerResult<usize> {
        //println!("Getting size of type");
        let size = match var_type {
            Type::String => 16,
            Type::Void => 0,
            Type::Int => 8,
            Type::Reference(inner) => {
                match inner.deref() {
                    Type::AutoArray(_) => 16,
                    _ => 8
                }
            },
            Type::Float => 4,
            Type::Bool => 4,
            Type::Other(cont_name) => {
                let cont_def = self.resolve_container(&cont_name)?;
                cont_def.get_size(self)?
            },
            Type::Array(inner_type, size) => {
                let inner_type_size = self.get_size_of_type(&inner_type)?;
                inner_type_size * size
            },
            _ => {
                //println!("Error in get_size_of_type()!");
                return Err(CompilerError::UnknownType(var_type.clone()));
            }
        };
        Ok(size)
    }

    /// Returns the type of a given variable
    pub fn get_type_of_var(&self, var_name: &String) -> CompilerResult<Type> {
        let mut type_opt = None;

        for i in 0..self.fn_context_stack.len() {
            let fn_ctx = self.get_function(i)?;
            let var_type_res = fn_ctx.get_var_type(var_name);
            if var_type_res.is_ok() {
                type_opt = Some(var_type_res.unwrap());
                break;
            }
        }

        type_opt.ok_or(CompilerError::UnknownVariable(var_name.clone()))
    }

    /// Returns the offset to SP for a given variable
    pub fn get_sp_offset_of_var(&self, var_name: &String) -> CompilerResult<i64> {
        let fn_ctx = self.get_current_function()?;
        let stack_pos = fn_ctx.get_var_pos(var_name)?;
        let stack_size = fn_ctx.stack_size as i64;
        let mut offset = stack_size - stack_pos;
        if offset > 0 {
            offset = -offset;
        }
        Ok(
            offset
        )
    }

    /// Increments the stack of the current function context
    pub fn inc_stack(&mut self, size: usize) -> CompilerResult<usize> {
        let fn_ctx = self.get_current_function_mut()?;
        fn_ctx.stack_size += size;
        //println!("COMP: Incrementing stack by {}", size);
        //println!("Incrementing stack of {:?} by {}", fn_ctx, size);
        Ok(fn_ctx.stack_size)
    }

    /// Decrements the stack of the current function context
    pub fn dec_stack(&mut self, size: usize) -> CompilerResult<usize> {
        let fn_ctx = self.get_current_function_mut()?;
        fn_ctx.stack_size -= size;
        //println!("COMP: Decrementing stack by {}", size);
        Ok(fn_ctx.stack_size)
    }

    /// Gets the stack size of the current function context
    pub fn get_stack_size(&self) -> CompilerResult<usize> {
        let fn_ctx = self.get_current_function()?;
        Ok(fn_ctx.stack_size)
    }

    // #endregion

    // #region FFI

    /// Registers a foreign module in the root
    pub fn register_foreign_root_module(&mut self, module: Module) -> CompilerResult<()> {
        self.register_foreign_module(module, &String::from("root::"))?;
        Ok(())
    }

    /// Registers a foreign module
    fn register_foreign_module(&mut self, module: Module, path: &String) -> CompilerResult<()> {
        let path = format!("{}{}::", path, module.name.clone());
        let mut mod_ctx = ModuleContext::new(module.name.clone());

        self.push_module_context(mod_ctx);

        for (_, function) in module.functions {
            self.register_foreign_function(function, &path)?;
        }

        for (_, module) in module.modules {
            self.register_foreign_module(module, &path)?;
        }

        mod_ctx = self.pop_module_context()?;

        let front_mod_ctx = self.get_current_module_mut()?;
        front_mod_ctx.add_module(mod_ctx)?;

        Ok(())
    }

    fn register_foreign_function(&mut self, mut function: Function, path: &String) -> CompilerResult<()> {
        if self.foreign_functions.is_none() {
            self.foreign_functions = Some(HashMap::new());
        }

        let full_fn_name = path.clone() + &function.name;
        let fn_uid = self.uid_generator.get_function_uid(&full_fn_name);
        let function_clone = function.clone();

        let mut arg_offset_sum: i64 = 0;
        let mut arg_sizes = Vec::new();
        let mut arg_offsets = Vec::new();
        arg_sizes.resize(function.arg_types.len(), 0);
        arg_offsets.resize(function.arg_types.len(), 0);
        let mut i = arg_sizes.len() - 1;
        for arg_type in function_clone.arg_types.iter().rev() {
            let arg_size = self.get_size_of_type(&arg_type)?;
            arg_sizes[i] = arg_size;
            arg_offset_sum -= arg_size as i64;
            arg_offsets[i] = arg_offset_sum;
            //println!("Registering arg i={}", i);
            if i > 0 {
                i -= 1;
            }
        }

        function.set_arg_offsets(arg_offsets);
        function.set_arg_sizes(arg_sizes);

        self.fn_uid_map.insert(full_fn_name, fn_uid);
        self.foreign_function_uids.insert(fn_uid);
        self.foreign_functions.as_mut()
            .ok_or(CompilerError::Unknown)?
            .insert(fn_uid, function);

        let fn_args: Vec<(String, Type)> = function_clone.arg_types.iter().map(|t| (String::from(""), t.clone())).collect();
        let fn_def = FunctionDef::new(function_clone.name)
            .with_arguments(&fn_args)
            .with_ret_type(function_clone.return_type)
            .with_uid(fn_uid);

        let front_mod_ctx = self.get_current_module_mut()?;
        front_mod_ctx.add_function(fn_def)?;

        Ok(())
    }

    /// Canonizes (adds module path when necessary) a given Type
    pub fn canonize_type(&self, var_type: &mut Type) -> CompilerResult<()> {
        let new_type_opt = match var_type {
            Type::Reference(inner_type) => {
                let inner_type = inner_type.deref_mut();
                self.canonize_type(inner_type)?;
                Some(
                    Type::Reference(Box::new(inner_type.clone()))
                )
            },
            Type::Other(cont_name) => {
                let cont_def = self.resolve_container(cont_name)?;
                Some(
                    Type::Other(cont_def.canonical_name.clone())
                )
            },
            _ => None
        };
        if new_type_opt.is_some() {
            *var_type = new_type_opt.unwrap();
        }
        Ok(())
    }

    // #endregion

    // #region declare functions

    /// (Pre-)declares a given declaration list
    pub fn declare_decl_list(&mut self, decl_list: &[Declaration]) -> CompilerResult<()> {
        for decl in decl_list.iter() {
            self.declare_decl(decl)?;
        }
        Ok(())
    }

    /// (Pre-)declares a given declaration
    pub fn declare_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        match decl {
            Declaration::Module(_, _) => self.declare_mod_decl(decl)?,
            Declaration::Function(_) => self.declare_fn_decl(decl)?,
            Declaration::Container(_) => self.declare_cont_decl(decl)?,
            Declaration::Import(_, _) => self.declare_import_decl(decl)?,
            Declaration::Impl(_, _, _) => self.declare_impl_decl(decl)?,
            Declaration::StaticVar(_) => self.declare_static_var(decl)?
        };
        Ok(())
    }

    /// (Pre-)declares a given static var declaration
    pub fn declare_static_var(&mut self, decl: &Declaration) -> CompilerResult<()> {
        Ok(())
    }

    /// (Pre-)declares a given function declaration
    pub fn declare_fn_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let fn_decl_args = match decl {
            Declaration::Function(fn_decl_args) => fn_decl_args,
            _ => return Err(CompilerError::Unknown)
        };

        let mut full_fn_name = self.get_module_path();
        if let Some(cont_name) = self.current_cont.as_ref().cloned() {
            full_fn_name += &cont_name;
            full_fn_name += "::";
        }
        full_fn_name += &fn_decl_args.name;

        let uid = self.uid_generator.get_function_uid(&full_fn_name);
        self.fn_uid_map.insert(full_fn_name.clone(), uid.clone());

        let mut fn_def = FunctionDef::from(fn_decl_args)
            .with_uid(uid);

        for (arg_name, arg_type) in fn_def.arguments.iter_mut() {
            self.canonize_type(arg_type)?;
        }

        if let Some(cont_name) = self.current_cont.as_ref().cloned() {
            let mod_ctx = self.get_current_module_mut()?;
            let cont_def = mod_ctx.get_container_mut(&cont_name)?;
            cont_def.add_member_function(fn_def)?;
        } else {
            let mod_ctx = self.get_current_module_mut()?;
            mod_ctx.add_function(fn_def)?;
        }

        Ok(())
    }

    /// (Pre-)declares a given module declaration
    pub fn declare_mod_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let (mod_name, decl_list) = match decl {
            Declaration::Module(mod_name, decl_list) => (mod_name, decl_list),
            _ => return Err(CompilerError::Unknown)
        };

        let mut mod_ctx = ModuleContext::new(mod_name.clone());

        self.push_module_context(mod_ctx);

        self.declare_decl_list(decl_list)?;

        mod_ctx = self.pop_module_context()?;

        let front_mod_ctx = self.get_current_module_mut()?;

        if front_mod_ctx.containers.contains_key(mod_name) {
            return Err(CompilerError::AlreadyContainsContainer(mod_name.clone()));
        }

        front_mod_ctx.add_module(mod_ctx)?;

        Ok(())
    }

    /// (Pre-)declares a given container declaration
    pub fn declare_cont_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let cont_decl_args = match decl {
            Declaration::Container(args) => args,
            _ => return Err(CompilerError::Unknown)
        };

        //println!("Declaring cont: {:?}", cont_decl_args);
        let mut canon_name = self.get_module_path();
        canon_name += &cont_decl_args.name;
        let mod_ctx = self.get_current_module_mut()?;
        if mod_ctx.containers.contains_key(&cont_decl_args.name) {
            let cont_def = mod_ctx.containers.get_mut(&cont_decl_args.name)
                .ok_or(CompilerError::UnknownContainer(cont_decl_args.name.clone()))?;
            cont_def.merge_cont_decl(cont_decl_args);
        } else {
            let cont_def = ContainerDef::from_decl(cont_decl_args, canon_name);
            if mod_ctx.modules.contains_key(&cont_decl_args.name) {
                return Err(CompilerError::AlreadyContainsModule(cont_decl_args.name.clone()));
            }
            mod_ctx.add_container(cont_def)?;
        }

        //println!("Containers: {:?}", mod_ctx.containers);

        Ok(())
    }

    /// (Pre-)declares a given import declaration
    pub fn declare_import_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let (import_path, import_as) = match decl {
            Declaration::Import(import_path, import_as) => (import_path, import_as),
            _ => return Err(CompilerError::Unknown)
        };

        let mod_ctx = self.get_current_module_mut()?;
        mod_ctx.add_import(import_as.clone(), import_path.clone())?;
        //println!("Imports: {:?}", mod_ctx.imports);

        Ok(())
    }

    /// (Pre-)declares a given impl declaration
    pub fn declare_impl_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let (impl_type, impl_for, decl_list) = match decl {
            Declaration::Impl(impl_type, impl_for, decl_list) => (impl_type, impl_for, decl_list), 
            _ => return Err(CompilerError::Unknown)
        };

        let mut canonical_name = self.get_module_path();
        canonical_name += &impl_type;

        if impl_type == impl_for {
            let mod_ctx = self.get_current_module_mut()?;
            let cont_res = mod_ctx.get_container(impl_type);
            if cont_res.is_err() {
                let cont_def = ContainerDef::new(impl_type.clone(), canonical_name);
                mod_ctx.add_container(cont_def)?;
            }
            self.current_cont = Some(impl_type.clone());
            self.declare_decl_list(decl_list)?;
            self.current_cont = None;
        } else {
            return Err(CompilerError::Unimplemented(format!("Cannot currently compile non-cont impls!")));
        }

        Ok(())
    }

    // #endregion
    
    // #region compile functions

    /// Compiles the decl list for the root module
    pub fn compile_root(&mut self, decl_list: &[Declaration]) -> CompilerResult<()> {
        self.declare_decl_list(decl_list)?;
        self.compile_decl_list(decl_list)?;
        Ok(())
    }

    /// Compiles a declaration list
    pub fn compile_decl_list(&mut self, decl_list: &[Declaration]) -> CompilerResult<()> {
        for decl in decl_list.iter() {
            self.compile_decl(decl)?;
        }
        Ok(())
    }

    /// Compiles a declaration
    pub fn compile_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        match decl {
            Declaration::Function(_) => self.compile_fn_decl(decl)?,
            Declaration::Impl(_, _, _) => self.compile_impl_decl(decl)?,
            Declaration::Module(_, _) => self.compile_mod_decl(decl)?,
            _ => {}
        };
        Ok(())
    }

    /// Compiles a function declaration
    pub fn compile_fn_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let fn_decl_args = match decl {
            Declaration::Function(fn_decl_args) => fn_decl_args,
            _ => return Err(CompilerError::Unknown)
        };

        //println!("Compiling fn_decl");

        let fn_def = self.resolve_function(&fn_decl_args.name)?;

        //println!("Fn def: {:?}", fn_def);

        let fn_ret_type = fn_def.ret_type.clone();

        let mut fn_ctx = FunctionContext::new(self, fn_def)?;

        let mut full_fn_name = self.get_module_path();
        if self.current_cont.is_some() {
            full_fn_name += self.current_cont.as_ref().unwrap();
            full_fn_name += "::";
        }
        full_fn_name += &fn_decl_args.name;

        //println!("Compiling fn decl with label {}", full_fn_name);

        
        self.builder.push_label(full_fn_name);

        self.push_function_context(fn_ctx);

        if let Some(stmt_list) = &fn_decl_args.code_block {
            self.compile_stmt_list(stmt_list)?;
        }

        // If the type is void, automatically add a return Statement
        if fn_ret_type == Type::Void {
            let ret_stmt = Statement::Return(None);
            self.compile_return_stmt(&ret_stmt)?;
        }

        // Instruction in case the function didnt return a value
        let halt_instr = Instruction::new(Opcode::HALT)
            .with_operand::<u8>(1);
        self.builder.push_instr(halt_instr);

        Ok(())
    }

    /// Compiles the proper SUBU_I instruction for a break statement
    pub fn compile_stack_loop(&mut self) -> CompilerResult<()> {
        let mut pop_size = 0;

        // Pop all values until the first loop context is hit
        for i in 0..self.fn_context_stack.len() {
            let fn_ctx = self.fn_context_stack.get(i)
                .ok_or(CompilerError::Unknown)?;
            pop_size += fn_ctx.stack_size;
            if fn_ctx.is_loop {
                break;
            }
        }

        //println!("Compiling loop stack cleanup with pop size {}", pop_size);

        let stack_instr = Instruction::new_dec_stack(pop_size);
        self.builder.push_instr(stack_instr);

        Ok(())
    }
    

    /// Compiles a stack cleanup for a given function context
    pub fn compile_stack_cleanup_block(&mut self, fn_ctx: &FunctionContext) -> CompilerResult<()> {
        let pop_size = fn_ctx.stack_size;

        //println!("Compiling stack cleanup with stack size {}", pop_size);

        // Instruction for popping values off the stack
        if pop_size > 0 {
            let pop_stack_instr = Instruction::new_dec_stack(pop_size);
            self.builder.push_instr(pop_stack_instr);
        }

        Ok(())
    }

    /// Compiles a full stack unwind until the parent function is hit 
    pub fn compile_stack_cleanup_return(&mut self) -> CompilerResult<()> {
        let mut parent_fn_ctx_opt = None;
        let mut stack_size = 0;

        for ctx in self.fn_context_stack.iter() {
            stack_size += ctx.stack_size;
            if !ctx.weak {
                parent_fn_ctx_opt = Some(ctx);
                break;
            }
        }

        let parent_fn_ctx = parent_fn_ctx_opt.ok_or(CompilerError::Unknown)?;
        let ret_type = parent_fn_ctx.get_ret_type()?;
        let ret_size = self.get_size_of_type(&ret_type)?;
        let mut pop_size = stack_size;
        let stack_begin_offset = -(stack_size as i16);
        
        if !ret_type.is_primitive() {
            //println!("fn return type is non-primitive.");
            pop_size -= ret_size;
            if pop_size > 0 {
                let mov_stack_instr = Instruction::new(Opcode::MOVN_A)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(-(ret_size as i16))
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(stack_begin_offset)
                    .with_operand::<u32>(ret_size as u32);
                self.builder.push_instr(mov_stack_instr);
            }
        }

        if pop_size > 0 {
            //println!("Popping {} off the stack at return.", pop_size);
            let pop_stack_instr = Instruction::new_dec_stack(pop_size);
            self.dec_stack(pop_size)?;
            self.builder.push_instr(pop_stack_instr);
        }

        Ok(())
    }

    /// Compiles a module declaration
    pub fn compile_mod_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let (mod_name, decl_list) = match decl {
            Declaration::Module(mod_name, decl_list) => (mod_name, decl_list),
            _ => return Err(CompilerError::Unknown)
        };

        let module_declared = {
            let front_mod_ctx = self.get_current_module()?;
            front_mod_ctx.modules.contains_key(mod_name)
        };

        if !module_declared {
            return Err(CompilerError::UnknownModule(mod_name.clone()));
        }

        let mod_ctx = {
            let front_mod_ctx = self.get_current_module()?;
            front_mod_ctx.modules.get(mod_name)
                .cloned()
                .ok_or(CompilerError::Unknown)?
        };

        self.push_module_context(mod_ctx);

        self.compile_decl_list(decl_list)?;

        self.pop_module_context()?;

        Ok(())
    }

    /// Compiles an impl declaration
    pub fn compile_impl_decl(&mut self, decl: &Declaration) -> CompilerResult<()> {
        let (impl_type, impl_for, decl_list) = match decl {
            Declaration::Impl(impl_type, impl_for, decl_list) => (impl_type, impl_for, decl_list), 
            _ => return Err(CompilerError::Unknown)
        };

        //println!("Compiling impl: {:?}", decl);

        if impl_type == impl_for {
            self.current_cont = Some(impl_type.clone());
            self.compile_decl_list(decl_list)?;
            self.current_cont = None;
        } else {
            return Err(CompilerError::Unimplemented(format!("impl of interfaces not supported yet!")));
        }

        Ok(())
    }

    /// Compiles a statement list
    pub fn compile_stmt_list(&mut self, stmt_list: &[Statement]) -> CompilerResult<()> {
        for stmt in stmt_list.iter() {
            //println!("Compiling statement... Stack size: {}", self.get_stack_size()?);
            self.compile_stmt(stmt)?;
            //println!("Compiled statement... Stack size: {}", self.get_stack_size()?);
        }
        Ok(())
    }

    /// Compiles a statement
    pub fn compile_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        match stmt {
            Statement::VariableDecl(_) => self.compile_var_decl_stmt(stmt)?,
            Statement::Expression(_) => self.compile_expr_stmt(stmt)?,
            Statement::Return(_) => self.compile_return_stmt(stmt)?,
            Statement::If(_) => self.compile_if_stmt(stmt)?,
            Statement::While(_, _) => self.compile_while_stmt(stmt)?, 
            Statement::Continue => self.compile_continue_stmt(stmt)?,
            Statement::Break => self.compile_break_stmt(stmt)?,
            _ => return Err(CompilerError::Unimplemented(format!("Compilation of {:?} not implemented!", stmt)))
        };
        Ok(())
    }

    /// Compiles a variable declaration statement
    pub fn compile_var_decl_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        let var_decl_args = match stmt {
            Statement::VariableDecl(var_decl_args) => var_decl_args,
            _ => return Err(CompilerError::Unknown)
        };
        //println!("Compiling var decl stmt");
        // The variable name
        let var_name = var_decl_args.name.clone();
        // The variable type
        let mut var_type = var_decl_args.var_type.clone();
        // The assignment expression
        let assignment_expr = &var_decl_args.assignment;
        let assignment_expr_type = self.check_expr_type(&assignment_expr)?;
        //println!("var decl assign expr: {:?}", assignment_expr);
        //println!("var decl assign expr type: {:?}", assignment_expr_type);
        // Special handling for auto typed vars
        if var_type == Type::Auto {
            var_type = assignment_expr_type;
        }

        //println!("Var type: {:?}", var_type);
        // Byte size of this type
        let var_size = self.get_size_of_type(&var_type)?;
        //println!("Size of type: {}", var_size);
        // Compile said expression
        //println!("Compiling assignment expr ({:?}). SP: {}", assignment_expr, self.get_stack_size()?);
        self.compile_expr(assignment_expr)?;
        //println!("Compiled assignment expr ({:?}). SP: {}", assignment_expr, self.get_stack_size()?);

        // If the type can be contained in a register
        if var_type.is_primitive() {
            let last_reg = {
                let fn_ctx = self.get_current_function()?;
                fn_ctx.register_allocator.get_last_temp_register()?
            };
            //println!("Last reg: {:?}", last_reg);
            let var_sp_offset = -(var_size as i16);
            let stack_inc_instr = Instruction::new_inc_stack(var_size);
            self.builder.push_instr(stack_inc_instr);
            self.inc_stack(var_size)?;
            let mov_instr = match var_type {
                Type::Int => {
                    Instruction::new(Opcode::MOVI_RA)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(var_sp_offset)
                },
                Type::Float => {
                    Instruction::new(Opcode::MOVF_RA)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(var_sp_offset)
                },
                Type::Reference(_) => {
                    Instruction::new(Opcode::MOVA_RA)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(var_sp_offset)
                },
                Type::Bool => {
                    Instruction::new(Opcode::MOVB_RA)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(var_sp_offset)
                },
                _ => {
                    //println!("Error in compile_var_decl_stmt()!");
                    return Err(CompilerError::UnknownType(var_type));
                }
            };
            self.builder.push_instr(mov_instr);
        }
        // Otherwise, the value is already on the top of the stack.
        // Set the variable in the context.
        let fn_ctx = self.get_current_function_mut()?;
        fn_ctx.set_stack_var((var_name.clone(), var_type.clone()), (fn_ctx.stack_size - var_size) as i64)?;
        //println!("Setting var {}: {:?} to position {}", var_name, var_type, fn_ctx.stack_size - var_size);
        Ok(())
    }

    /// Compiles a statement expression
    pub fn compile_expr_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        let stmt_expr = match stmt {
            Statement::Expression(expr) => expr,
            _ => return Err(CompilerError::Unknown)
        };

        match stmt_expr {
            Expression::Call(_, _) => self.compile_expr(stmt_expr)?,
            Expression::Assign(_, _) => self.compile_var_assign_stmt_expr(stmt_expr)?,
            Expression::AddAssign(_, _) => self.compile_var_assign_stmt_expr(stmt_expr)?,
            Expression::SubAssign(_, _) => self.compile_var_assign_stmt_expr(stmt_expr)?,
            Expression::MulAssign(_, _) => self.compile_var_assign_stmt_expr(stmt_expr)?,
            Expression::DivAssign(_, _) => self.compile_var_assign_stmt_expr(stmt_expr)?,
            _ => return Err(CompilerError::UnsupportedExpression(stmt_expr.clone()))
        };

        Ok(())
        //Err(CompilerError::Unimplemented(format!("Statement expr compilation not implemented!")))
    }
    

    /// Compiles an if statement
    pub fn compile_if_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        let if_stmt_args: &IfStatementArgs = match stmt {
            Statement::If(if_stmt_args) => if_stmt_args,
            _ => return Err(CompilerError::Unknown)
        };

        // Generate an instruction tag to fill in the end of this if/else chain
        let tag_end = self.uid_generator.generate();
        // Generate an instruction tag for the next branch
        let mut tag_next = self.uid_generator.generate();

        let expr_type = self.check_expr_type(&if_stmt_args.if_expr)?;
        // Only boolean expressions are allowed
        if expr_type != Type::Bool {
            return Err(CompilerError::TypeMismatch(expr_type, Type::Bool));
        }
        // Compile the if expression
        self.compile_expr(&if_stmt_args.if_expr)?;
        // Get the register the result of this boolean expression was saved in
        let last_reg = {
            self.get_current_function()?
                .register_allocator
                .get_last_temp_register()?
        };

        // Instruction for this if expr
        let jmpf_instr = Instruction::new(Opcode::JMPF)
            .with_operand::<u8>(last_reg.into())
            .with_operand(tag_next);
        self.builder.tag(tag_next);
        self.builder.push_instr(jmpf_instr);

        // Create new weak function context
        let mut if_fn_ctx = {
            let fn_ctx = self.get_current_function()?;
            FunctionContext::new_weak(fn_ctx)?
        };
        // And push it on the stack
        self.push_function_context(if_fn_ctx);

        // Compile the if statement list
        self.compile_stmt_list(&if_stmt_args.if_block)?;

        // Pop the function context off the stack again
        if_fn_ctx = self.pop_function_context()?;

        self.compile_stack_cleanup_block(&if_fn_ctx)?;

        // Instruction for jumping to the end
        let jmp_end_instr = Instruction::new(Opcode::JMP)
            .with_operand(tag_end);
        self.builder.tag(tag_end);
        self.builder.push_instr(jmp_end_instr);

        if if_stmt_args.else_if_list.is_some() {
            let else_if_list = if_stmt_args.else_if_list
                .as_ref()
                .ok_or(CompilerError::Unknown)?;
            for (else_if_expr, else_if_stmt_list) in else_if_list.iter() {
                // Current instruction position
                let pos = self.builder.get_current_offset();
                // Set the last JMPF to jump to this instruction
                {
                    // Retrieve the position list
                    let jmp_next_instr_pos_list = self.builder.get_tag(&tag_next)
                        .ok_or(CompilerError::Unknown)?;
                    // Retrieve the position
                    // (Only one instruction should exist with this tag)
                    let jmp_next_instr_pos = jmp_next_instr_pos_list.get(0)
                        .ok_or(CompilerError::Unknown)?;
                    // Get the mutable reference to this instruction
                    let jmp_next_instr = self.builder.get_instr(&jmp_next_instr_pos)
                        .ok_or(CompilerError::Unknown)?;
                    
                    // Update the jump destination
                    jmp_next_instr.remove_operand_bytes(8);
                    jmp_next_instr.append_operand(pos);
                }
                // Only boolean expressions are allowed
                let expr_type = self.check_expr_type(else_if_expr)?;
                if expr_type != Type::Bool {
                    return Err(CompilerError::TypeMismatch(expr_type, Type::Bool));
                }
                // Compile the expression
                self.compile_expr(else_if_expr)?;
                // Get the result register
                let last_reg = {
                    self.get_current_function()?
                        .register_allocator
                        .get_last_temp_register()?
                };
                // Generate new tag for the next jump
                tag_next = self.uid_generator.generate();
                // Instruction for jumping to next or inside statement list
                let jmpf_instr = Instruction::new(Opcode::JMPF)
                    .with_operand::<u8>(last_reg.into())
                    .with_operand(tag_next);
                self.builder.tag(tag_next);
                self.builder.push_instr(jmpf_instr);

                // Create a new weak function context
                let mut else_if_fn_ctx = {
                    let fn_ctx = self.get_current_function()?;
                    FunctionContext::new_weak(fn_ctx)?
                };
                // and push it on the stack
                self.push_function_context(else_if_fn_ctx);

                // Compile this "else if"s statement list
                self.compile_stmt_list(else_if_stmt_list)?;

                // Pop the context off the stack again
                else_if_fn_ctx = self.pop_function_context()?;

                self.compile_stack_cleanup_block(&else_if_fn_ctx)?;

                // Instruction for jumping to the end
                let jmp_end_instr = Instruction::new(Opcode::JMP)
                    .with_operand(tag_end);
                self.builder.tag(tag_end);
                self.builder.push_instr(jmp_end_instr);
            }
        }

        // If an "else" block exists
        if if_stmt_args.else_block.is_some() {
            let else_stmt_list = if_stmt_args.else_block.as_ref()
                .ok_or(CompilerError::Unknown)?;
            // Set the last JMPF to jump to this instruction
            let pos = self.builder.get_current_offset();
            {
                // Retrieve the position list
                let jmp_next_instr_pos_list = self.builder.get_tag(&tag_next)
                    .ok_or(CompilerError::Unknown)?;
                // Retrieve the position
                // (Only one instruction should exist with this tag)
                let jmp_next_instr_pos = jmp_next_instr_pos_list.get(0)
                    .ok_or(CompilerError::Unknown)?;
                // Get the mutable reference to this instruction
                let jmp_next_instr = self.builder.get_instr(&jmp_next_instr_pos)
                    .ok_or(CompilerError::Unknown)?;
                    
                // Update the jump destination
                jmp_next_instr.remove_operand_bytes(8);
                jmp_next_instr.append_operand(pos);
            }

            // Create a new weak function context
            let mut else_fn_ctx = {
                let fn_ctx = self.get_current_function()?;
                FunctionContext::new_weak(fn_ctx)?
            };
            // And push it on the stack
            self.push_function_context(else_fn_ctx);

            // Compile the statement list for this else block
            self.compile_stmt_list(else_stmt_list)?;

            // Pop it off the stack again
            else_fn_ctx = self.pop_function_context()?;

            self.compile_stack_cleanup_block(&else_fn_ctx)?;
        } else {
            // Set the last JMPF to jump to this instruction
            let pos = self.builder.get_current_offset();
            {
                // Retrieve the position list
                let jmp_next_instr_pos_list = self.builder.get_tag(&tag_next)
                    .ok_or(CompilerError::Unknown)?;
                // Retrieve the position
                // (Only one instruction should exist with this tag)
                let jmp_next_instr_pos = jmp_next_instr_pos_list.get(0)
                    .ok_or(CompilerError::Unknown)?;
                // Get the mutable reference to this instruction
                let jmp_next_instr = self.builder.get_instr(&jmp_next_instr_pos)
                    .ok_or(CompilerError::Unknown)?;
                    
                // Update the jump destination
                jmp_next_instr.remove_operand_bytes(8);
                jmp_next_instr.append_operand(pos);
            }
        }

        // Current position is at the end of the entire if/else if/else chain
        let pos_end = self.builder.get_current_offset();

        let jmp_end_pos_list = self.builder.get_tag(&tag_end)
            .ok_or(CompilerError::Unknown)?;

        // Make all the jump instructions jump to the end properly
        for jmp_end_pos in jmp_end_pos_list.iter() {
            let jmp_instr = self.builder.get_instr(jmp_end_pos)
                .ok_or(CompilerError::Unknown)?;
            jmp_instr.remove_operand_bytes(8);
            jmp_instr.append_operand(pos_end);
        }

        Ok(())
    }

    /// Compiles a while statement
    pub fn compile_while_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        let (while_expr, while_stmt_list) = match stmt {
            Statement::While(while_expr, while_stmt_list) => (while_expr, while_stmt_list),
            _ => return Err(CompilerError::Unknown)
        };

        let while_fn_ctx = FunctionContext::new_loop(self.get_current_function()?)?;
        self.push_function_context(while_fn_ctx);
        let while_start_pos = self.builder.get_current_offset();
        let tag_end = self.uid_generator.generate();
        let mut while_loop_ctx = LoopContext::new(while_start_pos, tag_end);
        self.push_loop_context(while_loop_ctx);

        // Check type of while expr
        let expr_type = self.check_expr_type(while_expr)?;
        // Only boolean expr are allowed
        if expr_type != Type::Bool {
            return Err(CompilerError::TypeMismatch(Type::Bool, expr_type.clone()));
        }

        // Compile the expression
        self.compile_expr(while_expr)?;

        let last_reg = {
            self.get_current_function()?
                .register_allocator
                .get_last_temp_register()?
        };

        self.builder.tag(tag_end);
        let jmpf_instr = Instruction::new(Opcode::JMPF)
            .with_operand::<u8>(last_reg.into())
            .with_operand(tag_end);
        self.builder.push_instr(jmpf_instr);

        // Compile the statement list
        self.compile_stmt_list(while_stmt_list)?;

        // Compile a continue statement
        self.compile_continue_stmt(&Statement::Continue)?;

        // This is the end of this while loop
        let while_end_pos = self.builder.get_current_offset();
        
        // Pop the while loop off the stack
        while_loop_ctx = self.pop_loop_context()?;
        let instr_pos_list = self.builder.get_tag(&while_loop_ctx.tag_end)
            .ok_or(CompilerError::Unknown)?;
        
        // Update with correct end position
        for instr_pos in instr_pos_list {
            let jmpf_instr = self.builder.get_instr(&instr_pos)
                .ok_or(CompilerError::Unknown)?;
            jmpf_instr.remove_operand_bytes(8);
            jmpf_instr.append_operand::<u64>(while_end_pos as u64);
        }

        // Pop this while loops fn context off the stack
        self.pop_function_context()?;

        Ok(())
    }

    /// Compiles a break statement
    pub fn compile_break_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        if *stmt != Statement::Break {
            return Err(CompilerError::Unknown);
        }

        // Compile the stack cleanup
        self.compile_stack_loop()?;

        let tag_end = {
            self.get_current_loop()?
                .tag_end
        };

        // Tag this instruction
        self.builder.tag(tag_end);
        // JMP to end instr
        let jmp_end_instr = Instruction::new(Opcode::JMP)
            .with_operand::<u64>(tag_end);
        self.builder.push_instr(jmp_end_instr);

        Ok(())
    }

    /// Compiles a continue statement
    pub fn compile_continue_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        if *stmt != Statement::Continue {
            return Err(CompilerError::Unknown);
        }

        // Compile the stack cleanup
        self.compile_stack_loop()?;

        let loop_start_pos = {
            self.get_current_loop()?
                .pos_start
        };

        // JMP to begin instr
        let jmp_begin_instr = Instruction::new(Opcode::JMP)
            .with_operand::<u64>(loop_start_pos as u64);
        self.builder.push_instr(jmp_begin_instr);
        
        Ok(())
    }

    /// Compiles a return statement
    pub fn compile_return_stmt(&mut self, stmt: &Statement) -> CompilerResult<()> {
        let return_expr_opt = match stmt {
            Statement::Return(ret_expr) => ret_expr,
            _ => return Err(CompilerError::Unknown)
        };

        let mut return_expr_type = Type::Void;

        if return_expr_opt.is_some() {
            let return_expr_ref = return_expr_opt.as_ref().unwrap();
            return_expr_type = self.check_expr_type(return_expr_ref)?;
        }

        let fn_ret_type = {
            let fn_ctx = self.get_parent_function()?;
            fn_ctx.get_ret_type()?
        };

        if fn_ret_type != return_expr_type {
            return Err(CompilerError::TypeMismatch(fn_ret_type, return_expr_type));
        }

        if return_expr_opt.is_some() {
            let return_expr = return_expr_opt.as_ref().unwrap();
            let ret_expr_type = self.check_expr_type(return_expr)?;
            //println!("Ret expr type: {:?}", ret_expr_type);
            //println!("Ret expr: {:?}", return_expr);
            self.compile_expr(return_expr)?;

            // Move to R0 register if type is primitive
            if ret_expr_type.is_primitive() {
                match fn_ret_type {
                    Type::Int => {
                        let last_reg = {
                            let fn_ctx = self.get_current_function()?;
                            fn_ctx.register_allocator.get_last_temp_register()?
                        };
                        // Instruction for doing so
                        let mov_ret_instr = Instruction::new(Opcode::MOVI)
                            .with_operand::<u8>(last_reg.into())
                            .with_operand::<u8>(Register::R0.into());
                        self.builder.push_instr(mov_ret_instr);
                    },
                    Type::Float => {
                        let last_reg = {
                            let fn_ctx = self.get_current_function()?;
                            fn_ctx.register_allocator.get_last_temp_register()?
                        };
                        // Instruction for doing so
                        let mov_ret_instr = Instruction::new(Opcode::MOVF)
                            .with_operand::<u8>(last_reg.into())
                            .with_operand::<u8>(Register::R0.into());
                        self.builder.push_instr(mov_ret_instr);
                    },
                    Type::Bool => {
                        let last_reg = {
                            let fn_ctx = self.get_current_function()?;
                            fn_ctx.register_allocator.get_last_temp_register()?
                        };
                        // Instruction for doing so
                        let mov_ret_instr = Instruction::new(Opcode::MOVB)
                            .with_operand::<u8>(last_reg.into())
                            .with_operand::<u8>(Register::R0.into());
                        self.builder.push_instr(mov_ret_instr);
                    },
                    Type::Reference(_) => {
                        let last_reg = {
                            let fn_ctx = self.get_current_function()?;
                            fn_ctx.register_allocator.get_last_temp_register()?
                        };
                        // Instruction for doing so
                        let mov_ret_instr = Instruction::new(Opcode::MOVA)
                            .with_operand::<u8>(last_reg.into())
                            .with_operand::<u8>(Register::R0.into());
                        self.builder.push_instr(mov_ret_instr);
                    },
                    _ => {}
                };
            }
        }

        // Clean up the stack.
        self.compile_stack_cleanup_return()?;

        // Add the RET function
        let ret_instr = Instruction::new(Opcode::RET);
        self.builder.push_instr(ret_instr);

        Ok(())
    }

    /// Compiles a variable assign statement expression
    pub fn compile_var_assign_stmt_expr(&mut self, assign_expr: &Expression) -> CompilerResult<()> {
        let (lhs_expr, rhs_expr) = match assign_expr {
            Expression::Assign(lhs, rhs) => (lhs.deref().clone(), rhs.deref().clone()),
            Expression::AddAssign(lhs, rhs) => {
                let rhs_expr = Expression::Addition(lhs.clone(), rhs.clone());
                (lhs.deref().clone(), rhs_expr)
            },
            Expression::SubAssign(lhs, rhs) => {
                let rhs_expr = Expression::Subtraction(lhs.clone(), rhs.clone());
                (lhs.deref().clone(), rhs_expr)
            },
            Expression::DivAssign(lhs, rhs) => {
                let rhs_expr = Expression::Division(lhs.clone(), rhs.clone());
                (lhs.deref().clone(), rhs_expr)
            },
            Expression::MulAssign(lhs, rhs) => {
                let rhs_expr = Expression::Multiplication(lhs.clone(), rhs.clone());
                (lhs.deref().clone(), rhs_expr)
            },
            _ => return Err(CompilerError::Unknown)
        };

        // Compile the left hand side of this expression
        let lhs_expr_type = self.compile_lhs_assign_expr(&lhs_expr)?;

        //println!("Type to be assigned to: {:?}", lhs_expr_type);
        // Get the result register
        let mut lhs_reg = {
            let fn_ctx = self.get_current_function_mut()?;
            fn_ctx.register_allocator.get_last_temp_register()?
        };

        // Save the result pointer to the stack;
        let stack_inc_instr = Instruction::new_inc_stack(8);
        self.inc_stack(8)?;

        let save_stack_instr = Instruction::new(Opcode::MOVA_RA)
            .with_operand::<u8>(lhs_reg.into())
            .with_operand::<u8>(Register::SP.into())
            .with_operand::<i16>(-8);
        
        self.builder.push_instr(stack_inc_instr);
        self.builder.push_instr(save_stack_instr);

        let lhs_ptr_pos = {
            self.get_current_function()?
                .stack_size - 8
        };

        // Check the type of the rhs expression
        let rhs_expr_type = self.check_expr_type(&rhs_expr)?;

        // Check for type mismatch
        if lhs_expr_type != rhs_expr_type {
            return Err(CompilerError::TypeMismatch(lhs_expr_type, rhs_expr_type));
        }

        let mut stack_size = self.get_stack_size()?;

        //println!("Stack size before assign expr: {}", stack_size);

        // Compile the right hand of this expression
        self.compile_expr(&rhs_expr)?;
        stack_size = self.get_stack_size()?;
        //println!("Stack size after assign expr: {}", stack_size);

        // Last register used may contain the assignment value
        let rhs_reg = self.get_last_register()?;

        lhs_reg = self.get_next_register()?;

        let stack_offset: i16 = {
            let curr_stack_size = self.get_stack_size()?;
            -((curr_stack_size - lhs_ptr_pos) as i16)
        };

        // Move the pointer from the stack into the lhs register
        let mov_stack_instr = Instruction::new(Opcode::MOVA_AR)
            .with_operand::<u8>(Register::SP.into())
            .with_operand::<i16>(stack_offset)
            .with_operand::<u8>(lhs_reg.clone().into());
        self.builder.push_instr(mov_stack_instr);

        // Move the value to the assignment destination
        let assign_instr = match rhs_expr_type {
            Type::Int => {
                //println!("Moving value from {:?} to the address in {:?}", rhs_reg, lhs_reg);
                Instruction::new(Opcode::MOVI_RA)
                    .with_operand::<u8>(rhs_reg.into())
                    .with_operand::<u8>(lhs_reg.into())
                    .with_operand::<i16>(0)
            },
            Type::Float => {
                Instruction::new(Opcode::MOVF_RA)
                    .with_operand::<u8>(rhs_reg.into())
                    .with_operand::<u8>(lhs_reg.into())
                    .with_operand::<i16>(0)
            },
            Type::Bool => {
                Instruction::new(Opcode::MOVB_RA)
                    .with_operand::<u8>(rhs_reg.into())
                    .with_operand::<u8>(lhs_reg.into())
                    .with_operand::<i16>(0)
            },
            Type::Reference(inner) => {
                match inner.deref() {
                    Type::AutoArray(_) => {
                        Instruction::new(Opcode::MOVN_A)
                            .with_operand::<u8>(Register::SP.into())
                            .with_operand::<i16>(-16)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<i16>(0)
                            .with_operand::<u32>(16)
                    },
                    _ => {
                        Instruction::new(Opcode::MOVA_RA)
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<i16>(0)
                    }
                }
            },
            _ => {
                let size = self.get_size_of_type(&rhs_expr_type)?;
                Instruction::new(Opcode::MOVN_A)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(-(size as i16))
                    .with_operand::<u8>(lhs_reg.into())
                    .with_operand::<i16>(0)
                    .with_operand::<u32>(size as u32)
            }
        };

        self.builder.push_instr(assign_instr);
        Ok(())
        //Err(CompilerError::Unimplemented(format!("Var assign compilation not implemented!")))
    }

    /// Compiles the left hand side of an assignment expression
    pub fn compile_lhs_assign_expr(&mut self, expr: &Expression) -> CompilerResult<Type> {
        let expr_type = match expr {
            Expression::Variable(var_name) => {
                let stack_offset = self.get_sp_offset_of_var(var_name)?.abs() as u64;
                let target_reg = {
                    let fn_ctx = self.get_current_function_mut()?;
                    fn_ctx.register_allocator.get_temp_register()?
                };
                // Instruction for assign
                let stack_offset_instr = Instruction::new(Opcode::SUBU_I)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<u64>(stack_offset)
                    .with_operand::<u8>(target_reg.into());
                self.builder.push_instr(stack_offset_instr);
                self.get_type_of_var(var_name)?
            },
            Expression::MemberAccess(lhs_expr, rhs_expr) => {
                let var_name = match lhs_expr.deref() {
                    Expression::Variable(var_name) => var_name,
                    _ => return Err(CompilerError::UnsupportedExpression(lhs_expr.deref().clone()))
                };
                let var_offset = self.get_sp_offset_of_var(var_name)?;
                let var_type = self.get_type_of_var(var_name)?;
                let lhs_ptr_reg = self.get_next_register()?;

                let cont_def = match var_type {
                    Type::Other(cont_name) => {
                        let subui_instr = Instruction::new(Opcode::SUBU_I)
                            .with_operand::<u8>(Register::SP.into())
                            .with_operand::<u64>(var_offset.abs() as u64)
                            .with_operand::<u8>(lhs_ptr_reg.into());
                        self.builder.push_instr(subui_instr);
                        self.resolve_container(&cont_name)?
                    },
                    Type::Reference(inner_type) => {
                        match inner_type.deref() {
                            Type::Other(cont_name) => {
                                let mova_instr = Instruction::new(Opcode::MOVA_AR)
                                    .with_operand::<u8>(Register::SP.into())
                                    .with_operand::<i16>(var_offset as i16)
                                    .with_operand::<u8>(lhs_ptr_reg.into());
                                self.builder.push_instr(mova_instr);
                                self.resolve_container(cont_name)?
                            },
                            _ => return Err(CompilerError::Unknown)
                        }
                    },
                    _ => return Err(CompilerError::Unknown)
                };

                self.compile_lhs_assign_member_expr(rhs_expr, &cont_def)?
            },
            _ => return Err(CompilerError::UnsupportedExpression(expr.clone()))
        };
        Ok(expr_type)
    }

    pub fn compile_lhs_assign_member_expr(&mut self, rhs_expr: &Expression, cont_def: &ContainerDef) -> CompilerResult<Type> {
        match rhs_expr {
            Expression::Variable(var_name) => {
                let last_reg = self.get_last_register()?;
                let next_reg = self.get_next_register()?;

                let member_offset = cont_def.get_member_offset(self, var_name)?;

                let addui_instr = Instruction::new(Opcode::ADDU_I)
                    .with_operand::<u8>(last_reg.into())
                    .with_operand::<u64>(member_offset as u64)
                    .with_operand::<u8>(next_reg.into());
                
                self.builder.push_instr(addui_instr);
                cont_def.get_member_type(var_name)
            },
            Expression::MemberAccess(lhs_expr, rhs_expr) => {
                let var_name;
                if let Expression::Variable(name) = lhs_expr.deref() {
                    var_name = name;
                } else {
                    return Err(CompilerError::UnsupportedExpression(lhs_expr.deref().clone()));
                }

                let member_offset = cont_def.get_member_offset(self, var_name)?;
                let member_type = cont_def.get_member_type(var_name)?;

                let last_reg = self.get_last_register()?;
                let next_reg = self.get_next_register()?;

                let cont_def = match member_type {
                    Type::Other(cont_name) => {
                        let addui_instr = Instruction::new(Opcode::ADDU_I)
                            .with_operand::<u8>(last_reg.into())
                            .with_operand::<u64>(member_offset as u64)
                            .with_operand::<u8>(next_reg.into());
                        self.builder.push_instr(addui_instr);
                        self.resolve_container(&cont_name)?
                    },
                    Type::Reference(inner_type) => {
                        match inner_type.deref() {
                            Type::Other(cont_name) => {
                                let mova_instr = Instruction::new(Opcode::MOVA_AR)
                                    .with_operand::<u8>(last_reg.into())
                                    .with_operand::<i16>(member_offset as i16)
                                    .with_operand::<u8>(next_reg.into());
                                self.builder.push_instr(mova_instr);
                                self.resolve_container(cont_name)?
                            },
                            _ => return Err(CompilerError::Unknown)
                        }
                    },
                    _ => return Err(CompilerError::Unknown)
                };

                self.compile_lhs_assign_member_expr(rhs_expr, &cont_def)
            },
            _ => return Err(CompilerError::UnsupportedExpression(rhs_expr.deref().clone()))
        }
    }

    /// Compiles an expression
    pub fn compile_expr(&mut self, expr: &Expression) -> CompilerResult<()> {
        let expr_type = self.check_expr_type(expr)?;
        let expr_size = self.get_size_of_type(&expr_type)?;
        //println!("Expr size: {}", expr_size);
        let before_stack_size = self.get_stack_size()?;
        match expr {
            Expression::IntLiteral(int) => {
                let reg = {
                    let fn_ctx = self.get_current_function_mut()?;
                    fn_ctx.register_allocator.get_temp_register()?
                };

                let ldi_instr = Instruction::new(Opcode::LDI)
                    .with_operand::<i64>(*int)
                    .with_operand::<u8>(reg.into());

                self.builder.push_instr(ldi_instr);
            },
            Expression::FloatLiteral(float) => {
                let reg = {
                    let fn_ctx = self.get_current_function_mut()?;
                    fn_ctx.register_allocator.get_temp_register()?
                };

                let ldf_instr = Instruction::new(Opcode::LDF)
                    .with_operand::<f32>(*float)
                    .with_operand::<u8>(reg.into());
                    
                self.builder.push_instr(ldf_instr);
            },
            Expression::BoolLiteral(boolean) => {
                let reg = {
                    let fn_ctx = self.get_current_function_mut()?;
                    fn_ctx.register_allocator.get_temp_register()?
                };

                let ldb_instr = Instruction::new(Opcode::LDB)
                    .with_operand::<bool>(*boolean)
                    .with_operand::<u8>(reg.into());
                    
                self.builder.push_instr(ldb_instr);
            },
            Expression::StringLiteral(string) => {
                let string = String::from(&string[1..string.len() - 1]);
                let (string_size, string_addr) = self.data.get_string_slice(&string);
                let stack_inc_instr = Instruction::new_inc_stack(16);
                self.inc_stack(16)?;

                let size_reg = self.get_next_register()?;
                let addr_reg = self.get_next_register()?;
                
                let size_lda_instr = Instruction::new(Opcode::LDA)
                    .with_operand(string_size)
                    .with_operand::<u8>(size_reg.clone().into());
                let addr_lda_instr = Instruction::new(Opcode::LDA)
                    .with_operand(string_addr)
                    .with_operand::<u8>(addr_reg.clone().into());
                let mov_size_instr = Instruction::new(Opcode::MOVA_RA)
                    .with_operand::<u8>(size_reg.into())
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(-16);
                let mov_addr_instr = Instruction::new(Opcode::MOVA_RA)
                    .with_operand::<u8>(addr_reg.into())
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(-8);

                self.builder.push_instr(stack_inc_instr);
                self.builder.push_instr(size_lda_instr);
                self.builder.push_instr(addr_lda_instr);
                self.builder.push_instr(mov_size_instr);
                self.builder.push_instr(mov_addr_instr);
            },
            Expression::ContainerInstance(_, _) => {
                self.compile_cont_instance_expr(expr)?;
            },
            Expression::Variable(_) => {
                self.compile_var_expr(expr)?;
            },
            Expression::Ref(op_expr) => {
                self.compile_lhs_assign_expr(op_expr)?;
            },
            Expression::Deref(op_expr) => {
                let expr_type = self.check_expr_type(op_expr)?;
                self.compile_expr(op_expr)?;
                let ref_type = expr_type.get_ref_type();
                if ref_type.is_primitive() {
                    let last_reg = self.get_last_register()?;
                    let next_reg = self.get_next_register()?;
                    match ref_type {
                        Type::Int => {
                            let movi_instr = Instruction::new(Opcode::MOVI_AR)
                                .with_operand::<u8>(last_reg.into())
                                .with_operand::<i16>(0)
                                .with_operand::<u8>(next_reg.into());
                            self.builder.push_instr(movi_instr);
                        },
                        Type::Float => {
                            let movf_instr = Instruction::new(Opcode::MOVF_AR)
                                .with_operand::<u8>(last_reg.into())
                                .with_operand::<i16>(0)
                                .with_operand::<u8>(next_reg.into());
                            self.builder.push_instr(movf_instr);
                        },
                        Type::Bool => {
                            let movb_instr = Instruction::new(Opcode::MOVB_AR)
                                .with_operand::<u8>(last_reg.into())
                                .with_operand::<i16>(0)
                                .with_operand::<u8>(next_reg.into());
                            self.builder.push_instr(movb_instr);
                        },
                        Type::Reference(inner_type) => {
                            match inner_type.deref() {
                                Type::AutoArray(_) => {
                                    return Err(CompilerError::CannotDerefSlice)
                                },
                                _ => {}
                            };
                        },
                        _ => {}
                    };
                } else {
                    return Err(CompilerError::Unimplemented(format!("Deref of non-primitive pointer types")));
                }
            },
            Expression::MemberAccess(_, _) => {
                self.compile_member_access_expr(expr)?;
            },
            Expression::Call(fn_name, _) => {
                //println!("Stack size before call expr: {}", self.get_stack_size()?);
                self.compile_call_expr(expr)?;
                let fn_ret_type = {
                    let fn_def = self.resolve_function(fn_name)?;
                    fn_def.ret_type.clone()
                };
                if fn_ret_type.is_primitive() {
                    self.get_current_function_mut()?
                        .register_allocator
                        .force_temp_register(Register::R0);
                }
                //println!("Stack size after call expr: {}", self.get_stack_size()?);
            },
            Expression::Addition(lhs, rhs) => {
                let expr_type = self.check_expr_type(lhs)?;
                self.compile_expr(lhs)?;
                let lhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                self.compile_expr(rhs)?;
                let rhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                //println!("Adding registers {:?} and {:?}", lhs_reg, rhs_reg);
                match expr_type {
                    Type::Int => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        //println!("Saved result into {:?}", res_reg);
                        let addi_instr = Instruction::new(Opcode::ADDI)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(addi_instr);
                    },
                    Type::Float => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let addf_instr = Instruction::new(Opcode::ADDF)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(addf_instr);
                    },
                    _ => return Err(CompilerError::UnsupportedExpression(lhs.deref().clone()))
                };
            },
            Expression::Subtraction(lhs, rhs) => {
                let expr_type = self.check_expr_type(lhs)?;
                self.compile_expr(lhs)?;
                let lhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                self.compile_expr(rhs)?;
                let rhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                match expr_type {
                    Type::Int => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let subi_instr = Instruction::new(Opcode::SUBI)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(subi_instr);
                    },
                    Type::Float => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let subf_instr = Instruction::new(Opcode::SUBF)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(subf_instr);
                    },
                    _ => return Err(CompilerError::UnsupportedExpression(lhs.deref().clone()))
                };
            },
            Expression::Multiplication(lhs, rhs) => {
                let expr_type = self.check_expr_type(lhs)?;
                self.compile_expr(lhs)?;
                let lhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                self.compile_expr(rhs)?;
                let rhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                match expr_type {
                    Type::Int => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let muli_instr = Instruction::new(Opcode::MULI)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(muli_instr);
                    },
                    Type::Float => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let mulf_instr = Instruction::new(Opcode::MULF)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(mulf_instr);
                    },
                    _ => return Err(CompilerError::UnsupportedExpression(lhs.deref().clone()))
                };
            },
            Expression::Division(lhs, rhs) => {
                let expr_type = self.check_expr_type(lhs)?;
                self.compile_expr(lhs)?;
                let lhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                self.compile_expr(rhs)?;
                let rhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                match expr_type {
                    Type::Int => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let divi_instr = Instruction::new(Opcode::DIVI)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(divi_instr);
                    },
                    Type::Float => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let divf_instr = Instruction::new(Opcode::DIVF)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(divf_instr);
                    },
                    _ => return Err(CompilerError::UnsupportedExpression(lhs.deref().clone()))
                };
            },
            Expression::LessThan(lhs, rhs) => {
                let expr_type = self.check_expr_type(lhs)?;
                self.compile_expr(lhs)?;
                let lhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                self.compile_expr(rhs)?;
                let rhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                match expr_type {
                    Type::Int => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let lti_instr = Instruction::new(Opcode::LTI)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(lti_instr);
                    },
                    Type::Float => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let ltf_instr = Instruction::new(Opcode::LTF)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(ltf_instr);
                    },
                    _ => return Err(CompilerError::UnsupportedExpression(lhs.deref().clone()))
                };
            },

            Expression::GreaterThan(lhs, rhs) => {
                let expr_type = self.check_expr_type(lhs)?;
                self.compile_expr(lhs)?;
                let lhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                self.compile_expr(rhs)?;
                let rhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                match expr_type {
                    Type::Int => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let gti_instr = Instruction::new(Opcode::GTI)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(gti_instr);
                    },
                    Type::Float => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let gtf_instr = Instruction::new(Opcode::GTF)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(gtf_instr);
                    },
                    _ => return Err(CompilerError::UnsupportedExpression(lhs.deref().clone()))
                };
            },

            Expression::LessThanEquals(lhs, rhs) => {
                let expr_type = self.check_expr_type(lhs)?;
                self.compile_expr(lhs)?;
                let lhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                self.compile_expr(rhs)?;
                let rhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                match expr_type {
                    Type::Int => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let lteqi_instr = Instruction::new(Opcode::LTEQI)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(lteqi_instr);
                    },
                    Type::Float => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let lteqf_instr = Instruction::new(Opcode::LTEQF)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(lteqf_instr);
                    },
                    _ => return Err(CompilerError::UnsupportedExpression(lhs.deref().clone()))
                };
            },

            Expression::GreaterThanEquals(lhs, rhs) => {
                let expr_type = self.check_expr_type(lhs)?;
                self.compile_expr(lhs)?;
                let lhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                self.compile_expr(rhs)?;
                let rhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                match expr_type {
                    Type::Int => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let gteqi_instr = Instruction::new(Opcode::GTEQI)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(gteqi_instr);
                    },
                    Type::Float => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let gteqf_instr = Instruction::new(Opcode::GTEQF)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(gteqf_instr);
                    },
                    _ => return Err(CompilerError::UnsupportedExpression(lhs.deref().clone()))
                };
            },

            Expression::Equals(lhs, rhs) => {
                let expr_type = self.check_expr_type(lhs)?;
                self.compile_expr(lhs)?;
                let lhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                self.compile_expr(rhs)?;
                let rhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                match expr_type {
                    Type::Int => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let eqi_instr = Instruction::new(Opcode::EQI)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(eqi_instr);
                    },
                    Type::Float => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let eqf_instr = Instruction::new(Opcode::EQF)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(eqf_instr);
                    },
                    _ => return Err(CompilerError::UnsupportedExpression(lhs.deref().clone()))
                };
            },
            Expression::NotEquals(lhs, rhs) => {
                let expr_type = self.check_expr_type(lhs)?;
                self.compile_expr(lhs)?;
                let lhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                self.compile_expr(rhs)?;
                let rhs_reg = {
                    let fn_ctx = self.get_current_function()?;
                    fn_ctx.register_allocator.get_last_temp_register()?
                };
                match expr_type {
                    Type::Int => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let neqi_instr = Instruction::new(Opcode::NEQI)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(neqi_instr);
                    },
                    Type::Float => {
                        let res_reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let neqf_instr = Instruction::new(Opcode::NEQF)
                            .with_operand::<u8>(lhs_reg.into())
                            .with_operand::<u8>(rhs_reg.into())
                            .with_operand::<u8>(res_reg.into());
                        self.builder.push_instr(neqf_instr);
                    },
                    _ => return Err(CompilerError::UnsupportedExpression(lhs.deref().clone()))
                };
            },
            Expression::Not(op) => {
                self.compile_expr(op)?;
                let (op_reg, target_reg) = {
                    let fn_ctx = self.get_current_function_mut()?;
                    let op_reg = fn_ctx.register_allocator.get_last_temp_register()?;
                    let target_reg = fn_ctx.register_allocator.get_temp_register()?;
                    (op_reg, target_reg)
                };
                let not_instr = Instruction::new(Opcode::NOT)
                    .with_operand::<u8>(op_reg.into())
                    .with_operand::<u8>(target_reg.into());
                self.builder.push_instr(not_instr);
            },
            Expression::And(lhs, rhs) => {
                self.compile_expr(lhs)?;
                let lhs_reg = self.get_last_register()?;
                self.compile_expr(rhs)?;
                let rhs_reg = self.get_last_register()?;
                let target_reg = self.get_next_register()?;
                let and_instr = Instruction::new(Opcode::AND)
                    .with_operand::<u8>(lhs_reg.into())
                    .with_operand::<u8>(rhs_reg.into())
                    .with_operand::<u8>(target_reg.into());
                self.builder.push_instr(and_instr);
            },
            Expression::Or(lhs, rhs) => {
                self.compile_expr(lhs)?;
                let lhs_reg = self.get_last_register()?;
                self.compile_expr(rhs)?;
                let rhs_reg = self.get_last_register()?;
                let target_reg = self.get_next_register()?;
                let or_instr = Instruction::new(Opcode::OR)
                    .with_operand::<u8>(lhs_reg.into())
                    .with_operand::<u8>(rhs_reg.into())
                    .with_operand::<u8>(target_reg.into());
                self.builder.push_instr(or_instr);
            },
            _ => return Err(CompilerError::UnsupportedExpression(expr.clone()))
        };

        let after_stack_size = self.get_stack_size()?;
        let stack_diff = after_stack_size - before_stack_size;
        //println!("Stack diff: {}, is expr primitive: {}", stack_diff, expr_type.is_primitive());
        let mut pop_size = stack_diff;
        //println!("{}", pop_size > expr_size);

        if !expr_type.is_primitive() {
            pop_size -= expr_size;
            if pop_size > 0 {
                let mov_stack_instr = Instruction::new(Opcode::MOVN_A)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(-(expr_size as i16))
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(-(stack_diff as i16))
                    .with_operand::<u32>(16);
                self.builder.push_instr(mov_stack_instr);
            }
        }
        if pop_size > 0 {
            //println!("Popping {} bytes off the stack after compile_expr().", pop_size);
            let pop_stack_instr = Instruction::new_dec_stack(pop_size);
            self.dec_stack(pop_size)?;
            self.builder.push_instr(pop_stack_instr);
        }
        Ok(())
        //Err(CompilerError::Unimplemented(format!("Expr compilation not implemented!")))
    }

    /// Compiles a member access expression
    pub fn compile_member_access_expr(&mut self, expr: &Expression) -> CompilerResult<()> {
        //println!("Line 2374");
        let (lhs_expr, rhs_expr) = match expr {
            Expression::MemberAccess(lhs, rhs) => (lhs.deref(), rhs.deref()),
            _ => return Err(CompilerError::Unknown)
        };

        let var_type = self.check_expr_type(lhs_expr)?;
        let is_reference = match var_type {
            Type::Other(_) => false,
            Type::Reference(inner_type) => {
                match inner_type.deref() {
                    Type::Other(_) => true,
                    _ => {
                        return Err(CompilerError::UnsupportedExpression(lhs_expr.deref().clone()));
                    }
                }
            },
            _ => return Err(CompilerError::UnsupportedExpression(lhs_expr.deref().clone()))
        };

        match lhs_expr {
            Expression::Variable(var_name) => {
                let var_offset = self.get_sp_offset_of_var(var_name)?;
                let next_reg = self.get_next_register()?;
                // If its a reference on the stack
                if is_reference {
                    
                }
                // If its a normal stack allocated variable
                else {

                }
            },
            _ => return Err(CompilerError::UnsupportedExpression(lhs_expr.deref().clone()))
        };

        Ok(())
    }

    fn compile_member_access_rhs_expr(&mut self, expr: &Expression) -> CompilerResult<()> {
        Ok(())
    }

    /// Compiles a member call expression
    pub fn compile_member_call_expr(&mut self, expr: &Expression, cont_def: &ContainerDef) -> CompilerResult<()> {
        //println!("Line 2718");
        let (fn_name, fn_arg_exprs) = match expr {
            Expression::Call(fn_name, fn_args) => (fn_name, fn_args),
            _ => return Err(CompilerError::Unknown)
        };

        //println!("Compiling call expr");

        //println!("Compiling member call expr {} for type {}", fn_name, cont_def.canonical_name);

        let fn_def = cont_def.get_member_function(fn_name)?;

        let fn_ret_size = self.get_size_of_type(&fn_def.ret_type)?;

        if fn_arg_exprs.len() + 1 != fn_def.arguments.len() {
            return Err(CompilerError::UnknownFunction(fn_name.clone()));
        }

        let fn_def_first_arg_type = {
            let fn_arg = fn_def.arguments.get(0)
                .ok_or(CompilerError::Unknown)?;
            fn_arg.1.clone()
        };
        let fn_args_first_arg_type = Type::Reference(Box::new(Type::Other(cont_def.canonical_name.clone())));
        if fn_def_first_arg_type != fn_args_first_arg_type {
            return Err(CompilerError::TypeMismatch(fn_def_first_arg_type, fn_args_first_arg_type));
        }

        let before_stack_size = self.get_stack_size()?;

        let last_reg = self.get_last_register()?;
        //println!("Address of container should be in {:?}", last_reg);
        let stack_inc_instr = Instruction::new_inc_stack(8);
        self.inc_stack(8)?;
        let mova_instr = Instruction::new(Opcode::MOVA_RA)
            .with_operand::<u8>(last_reg.into())
            .with_operand::<u8>(Register::SP.into())
            .with_operand::<i16>(-8);
        self.builder.push_instr(stack_inc_instr);
        self.builder.push_instr(mova_instr);

        let mut stack_size = before_stack_size;

        for i in 0..fn_arg_exprs.len() {
            let mut expr_type = self.check_expr_type(&fn_arg_exprs[i])?;
            self.canonize_type(&mut expr_type)?;
            let fn_arg_type = &fn_def.arguments[i + 1].1;

            if *fn_arg_type != expr_type {
                return Err(CompilerError::TypeMismatch(fn_arg_type.clone(), expr_type.clone()));
            }

            // Compile this expr
            self.compile_expr(&fn_arg_exprs[i])?;

            let curr_stack_size = self.get_stack_size()?;

            let stack_diff = curr_stack_size - stack_size;
            let mut pop_size = stack_diff;

            let size = self.get_size_of_type(&expr_type)?;
            
            /*
            if !fn_arg_type.is_primitive() {
                pop_size -= size;
                if pop_size > 0 {
                    let mov_stack_instr = Instruction::new(Opcode::MOVN_A)
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(-(size as i16))
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(-(stack_diff as i16))
                        .with_operand::<u32>(size as u32);
                    self.builder.push_instr(mov_stack_instr);
                }
            }
            if pop_size > 0 {
                let dec_stack_instr = Instruction::new_dec_stack(pop_size);
                self.dec_stack(pop_size)?;
                self.builder.push_instr(dec_stack_instr);
            }*/

            let last_reg = {
                self.get_current_function()?
                    .register_allocator
                    .get_last_temp_register()?
            };

            //println!("CHECKING IF EXPR TYPE IS PRIMITIVE");

            if expr_type.is_primitive() {
                //println!("incrementing stack for primitive type arg");
                let stack_instr = Instruction::new_inc_stack(size);
                self.builder.push_instr(stack_instr);
                self.inc_stack(size)?;
            }

            let mov_instr_opt = match expr_type {
                Type::Int => {
                    Some(Instruction::new(Opcode::MOVI_RA)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(-(size as i16)))
                },
                Type::Float => {
                    Some(Instruction::new(Opcode::MOVF_RA)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(-(size as i16)))
                },
                Type::Bool => {
                    Some(Instruction::new(Opcode::MOVB_RA)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(-(size as i16)))
                },
                Type::String => None,
                Type::Reference(inner_type) => {
                    match inner_type.deref() {
                        Type::AutoArray(_) => None,
                        _ => {
                            Some(
                                Instruction::new(Opcode::MOVA_RA)
                                    .with_operand::<u8>(last_reg.into())
                                    .with_operand::<u8>(Register::SP.into())
                                    .with_operand::<i16>(-(size as i16))
                            )
                        }
                    }
                },
                _ => {
                    //println!("Error in compile_call_expr()!");
                    return Err(CompilerError::UnknownType(expr_type));
                }
            };

            if mov_instr_opt.is_some() {
                self.builder.push_instr(mov_instr_opt.unwrap());
            }

            stack_size = self.get_stack_size()?;
        }

        let call_instr = Instruction::new(Opcode::CALL)
            .with_operand::<u64>(fn_def.uid);
        self.builder.push_instr(call_instr);
        if !fn_def.ret_type.is_primitive() {
            self.inc_stack(fn_ret_size)?;
        }

        let stack_diff = self.get_stack_size()? - before_stack_size;
        //println!("Stack diff after member call expr: {}", stack_diff);
        let mut pop_size = stack_diff;
        if !fn_def.ret_type.is_primitive() {
            let mov_stack_instr = Instruction::new(Opcode::MOVN_A)
                .with_operand::<u8>(Register::SP.into())
                .with_operand::<i16>(-(fn_ret_size as i16))
                .with_operand::<u8>(Register::SP.into())
                .with_operand::<i16>(-(stack_diff as i16))
                .with_operand::<u32>(fn_ret_size as u32);
            pop_size -= fn_ret_size;
            self.builder.push_instr(mov_stack_instr);
        }

        if pop_size > 0 {
            let stack_dec_instr = Instruction::new_dec_stack(pop_size);
            self.dec_stack(pop_size)?;
            self.builder.push_instr(stack_dec_instr);
        }

        //println!("Stack diff of member call: {}", before_stack_size - self.get_stack_size()?);

        Ok(())
    }

    /// Compiles a cont instance expression
    pub fn compile_cont_instance_expr(&mut self, expr: &Expression) -> CompilerResult<()> {
        //println!("Line 2638");
        let (cont_name, cont_memper_map) = match expr {
            Expression::ContainerInstance(a1, a2) => (a1, a2),
            _ => return Err(CompilerError::Unknown)
        };

        // Sorts the member instance expressions in the correct way
        let mut member_map_ordered = BTreeMap::new();

        // Resolve the container definition
        let cont_def = self.resolve_container(cont_name)?;

        // Insert the expressions at the correct position
        for (name, expr) in cont_memper_map.iter() {
            // Retrieve position from container def
            let index = cont_def.get_member_index(name)?;
            member_map_ordered.insert(index, expr);
        }

        // Finally, compile the expressions in the correct order
        for (_, expr) in member_map_ordered.iter() {
            let expr_type = self.check_expr_type(expr)?;
            self.compile_expr(expr)?;
            // Special handling for copying register type values on the stack
            let last_reg = self.get_last_register()?;
            match expr_type {
                Type::Int => {
                    let stack_inc_instr = Instruction::new_inc_stack(8);
                    self.inc_stack(8)?;
                    let movi_instr = Instruction::new(Opcode::MOVI_RA)
                        .with_operand::<u8>(last_reg.clone().into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(-8);
                    self.builder.push_instr(stack_inc_instr);
                    self.builder.push_instr(movi_instr);
                },
                Type::Bool => {
                    let stack_inc_instr = Instruction::new_inc_stack(1);
                    self.inc_stack(1)?;
                    let movb_instr = Instruction::new(Opcode::MOVB_RA)
                        .with_operand::<u8>(last_reg.clone().into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(-1);
                    self.builder.push_instr(stack_inc_instr);
                    self.builder.push_instr(movb_instr);
                },
                Type::Float => {
                    let stack_inc_instr = Instruction::new_inc_stack(4);
                    self.inc_stack(4)?;
                    let movf_instr = Instruction::new(Opcode::MOVF_RA)
                        .with_operand::<u8>(last_reg.clone().into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(-4);
                    self.builder.push_instr(stack_inc_instr);
                    self.builder.push_instr(movf_instr);
                },
                Type::Reference(inner_type) => {
                    match inner_type.deref() {
                        Type::AutoArray(_) => {},
                        _ => {
                            let stack_inc_instr = Instruction::new_inc_stack(8);
                            self.inc_stack(8)?;
                            let mova_instr = Instruction::new(Opcode::MOVA_RA)
                                .with_operand::<u8>(last_reg.clone().into())
                                .with_operand::<u8>(Register::SP.into())
                                .with_operand::<i16>(-8);
                            self.builder.push_instr(stack_inc_instr);
                            self.builder.push_instr(mova_instr);
                        }
                    };
                },
                _ => {}
            };
        }

        Ok(())
    }

    /// Compiles a call expresion
    pub fn compile_call_expr(&mut self, expr: &Expression) -> CompilerResult<()> {
        //println!("Line 2718");
        let (fn_name, fn_arg_exprs) = match expr {
            Expression::Call(fn_name, fn_args) => (fn_name, fn_args),
            _ => return Err(CompilerError::Unknown)
        };

        //println!("Compiling call expr");

        let fn_def = self.resolve_function(fn_name)?;

        let fn_ret_size = self.get_size_of_type(&fn_def.ret_type)?;

        if fn_arg_exprs.len() != fn_def.arguments.len() {
            return Err(CompilerError::UnknownFunction(fn_name.clone()));
        }
        
        let before_call_stack_size = self.get_stack_size()?;
        let mut stack_size = before_call_stack_size;

        for i in 0..fn_def.arguments.len() {
            let mut expr_type = self.check_expr_type(&fn_arg_exprs[i])?;
            self.canonize_type(&mut expr_type)?;
            let fn_arg_type = &fn_def.arguments[i].1;
            if *fn_arg_type != expr_type {
                return Err(CompilerError::TypeMismatch(fn_arg_type.clone(), expr_type.clone()));
            }

            //println!("Compiling call expr arg. Stack size: {}", self.get_stack_size()?);
            //println!("Type of call expr: {:?}, size: {}", expr_type, self.get_size_of_type(&expr_type)?);

            // Compile this expr
            self.compile_expr(&fn_arg_exprs[i])?;


            //println!("Compiled call expr arg. Stack size: {}", self.get_stack_size()?);

            let curr_stack_size = self.get_stack_size()?;

            let stack_diff = curr_stack_size - stack_size;
            let mut pop_size = stack_diff;

            let size = self.get_size_of_type(&expr_type)?;

            if !expr_type.is_primitive() {
                pop_size -= size;
                if pop_size > 0 {
                    let mov_stack_instr = Instruction::new(Opcode::MOVN_A)
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(-(size as i16))
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(-(stack_diff as i16))
                        .with_operand::<u32>(size as u32);
                    self.builder.push_instr(mov_stack_instr);
                }
            }
            if pop_size > 0 {
                let stack_dec_instr = Instruction::new_dec_stack(pop_size);
                self.dec_stack(pop_size)?;
                self.builder.push_instr(stack_dec_instr);
            }

            let last_reg = {
                self.get_current_function()?
                    .register_allocator
                    .get_last_temp_register()?
            };

            //println!("CHECKING IF EXPR TYPE IS PRIMITIVE");

            if expr_type.is_primitive() {
                //println!("incrementing stack for primitive type arg");
                let stack_instr = Instruction::new_inc_stack(size);
                self.builder.push_instr(stack_instr);
                self.inc_stack(size)?;
            }

            let mov_instr_opt = match expr_type {
                Type::Int => {
                    Some(Instruction::new(Opcode::MOVI_RA)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(-(size as i16)))
                },
                Type::Float => {
                    Some(Instruction::new(Opcode::MOVF_RA)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(-(size as i16)))
                },
                Type::Bool => {
                    Some(Instruction::new(Opcode::MOVB_RA)
                        .with_operand::<u8>(last_reg.into())
                        .with_operand::<u8>(Register::SP.into())
                        .with_operand::<i16>(-(size as i16)))
                },
                Type::String => None,
                Type::Reference(inner_type) => {
                    match inner_type.deref() {
                        Type::AutoArray(_) => None,
                        _ => {
                            Some(
                                Instruction::new(Opcode::MOVA_RA)
                                    .with_operand::<u8>(last_reg.into())
                                    .with_operand::<u8>(Register::SP.into())
                                    .with_operand::<i16>(-(size as i16))
                            )
                        }
                    }
                },
                _ => {
                    //println!("Error in compile_call_expr()!");
                    return Err(CompilerError::UnknownType(expr_type));
                }
            };

            if mov_instr_opt.is_some() {
                self.builder.push_instr(mov_instr_opt.unwrap());
            }

            stack_size = self.get_stack_size()?;
        }

        let call_instr = Instruction::new(Opcode::CALL)
            .with_operand::<u64>(fn_def.uid);
        self.builder.push_instr(call_instr);
        if !fn_def.ret_type.is_primitive() {
            self.inc_stack(fn_ret_size)?;
        }

        let stack_diff = self.get_stack_size()? - before_call_stack_size;
        //println!("Stack diff after args + call: {}", stack_diff);
        let mut pop_size = stack_diff;

        if !fn_def.ret_type.is_primitive() {
            let mov_stack_instr = Instruction::new(Opcode::MOVN_A)
                .with_operand::<u8>(Register::SP.into())
                .with_operand::<i16>(-(fn_ret_size as i16))
                .with_operand::<u8>(Register::SP.into())
                .with_operand::<i16>(-(stack_diff as i16))
                .with_operand::<u32>(fn_ret_size as u32);
            pop_size -= fn_ret_size;
            self.builder.push_instr(mov_stack_instr);
        }
        
        let stack_dec_instr = Instruction::new_dec_stack(pop_size);
        self.dec_stack(pop_size)?;
        self.builder.push_instr(stack_dec_instr);

        Ok(())
    }

    /// Compiles a variable expression
    pub fn compile_var_expr(&mut self, expr: &Expression) -> CompilerResult<()> {
        let var_name = match expr {
            Expression::Variable(var_name) => var_name,
            _ => return Err(CompilerError::Unknown)
        };

        //println!("Compiling var expr");

        let var_type = self.get_type_of_var(var_name)?;
        let mut var_offset = self.get_sp_offset_of_var(var_name)?;
        match var_type {
            Type::Int => {
                let reg = {
                    let fn_ctx = self.get_current_function_mut()?;
                    fn_ctx.register_allocator.get_temp_register()?
                };
                let movi_instr = Instruction::new(Opcode::MOVI_AR)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(var_offset as i16)
                    .with_operand::<u8>(reg.into());
                self.builder.push_instr(movi_instr);
            },
            Type::Float => {
                let reg = {
                    let fn_ctx = self.get_current_function_mut()?;
                    fn_ctx.register_allocator.get_temp_register()?
                };
                let movf_instr = Instruction::new(Opcode::MOVF_AR)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(var_offset as i16)
                    .with_operand::<u8>(reg.into());
                self.builder.push_instr(movf_instr);
            },
            Type::Bool => {
                let reg = {
                    let fn_ctx = self.get_current_function_mut()?;
                    fn_ctx.register_allocator.get_temp_register()?
                };
                let movb_instr = Instruction::new(Opcode::MOVB_AR)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(var_offset as i16)
                    .with_operand::<u8>(reg.into());
                self.builder.push_instr(movb_instr);
            },
            Type::Reference(inner_type) => {
                match inner_type.deref() {
                    Type::AutoArray(_) => {
                        let stack_inc_instr = Instruction::new_inc_stack(16);
                        self.inc_stack(16)?;
                        var_offset -= 16;
                        let movn_instr = Instruction::new(Opcode::MOVN_A)
                            .with_operand::<u8>(Register::SP.into())
                            .with_operand::<i16>(var_offset as i16)
                            .with_operand::<u8>(Register::SP.into())
                            .with_operand::<i16>(-16)
                            .with_operand::<u32>(16);
                        self.builder.push_instr(stack_inc_instr);
                        self.builder.push_instr(movn_instr);
                    },
                    _ => {
                        let reg = {
                            let fn_ctx = self.get_current_function_mut()?;
                            fn_ctx.register_allocator.get_temp_register()?
                        };
                        let mova_instr = Instruction::new(Opcode::MOVA_AR)
                            .with_operand::<u8>(Register::SP.into())
                            .with_operand::<i16>(var_offset as i16)
                            .with_operand::<u8>(reg.into());
                        self.builder.push_instr(mova_instr);
                    }
                };
            },
            Type::Other(cont_name) => {
                let cont_def = self.resolve_container(&cont_name)?;
                let size = cont_def.get_size(self)?;

                let stack_inc_instr = Instruction::new_inc_stack(size);
                self.inc_stack(size)?;

                var_offset -= size as i64;

                let movn_instr = Instruction::new(Opcode::MOVN_A)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(var_offset as i16)
                    .with_operand::<u8>(Register::SP.into())
                    .with_operand::<i16>(-(size as i16))
                    .with_operand::<u32>(size as u32);
                
                self.builder.push_instr(stack_inc_instr);
                self.builder.push_instr(movn_instr);
            },
            _ => {
                //println!("Errors in compile_var_expr()");
                return Err(CompilerError::UnknownType(var_type));
            },
        };

        Ok(())
    }

    /// Returns the type of an expression and checks for type mismatches
    pub fn check_expr_type(&self, expr: &Expression) -> CompilerResult<Type> {
        //println!("Checking type of expr: {:?}", expr);
        let expr_type = match expr {
            Expression::IntLiteral(_) => Type::Int,
            Expression::FloatLiteral(_) => Type::Float,
            Expression::BoolLiteral(_) => Type::Bool,
            Expression::StringLiteral(_) => Type::String,
            Expression::Ref(expr) => {
                let expr_type = self.check_expr_type(expr)?;
                Type::Reference(Box::new(expr_type))
            },
            Expression::Deref(expr) => {
                let expr_type = self.check_expr_type(expr)?;
                match expr_type {
                    Type::Reference(inner_type) => {
                        match inner_type.deref() {
                            Type::AutoArray(_) => return Err(CompilerError::CannotDerefSlice),
                            _ => return Ok(inner_type.deref().clone())
                        };
                    },
                    _ => return Err(CompilerError::CannotDerefNonPointer)
                };
            },
            Expression::Call(fn_name, _) => {
                let fn_def = self.resolve_function(fn_name)?;
                fn_def.ret_type
            },
            Expression::Variable(var_name) => {
                self.get_type_of_var(var_name)?
            },
            Expression::MemberAccess(_, _) => {
                self.check_member_access_expr_type(expr, None)?
            },
            Expression::ContainerInstance(cont_name, _) => {
                Type::Other(cont_name.clone())
            },
            Expression::Assign(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                lhs_type
            },
            Expression::Addition(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                lhs_type
            },
            Expression::Subtraction(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                lhs_type
            },
            Expression::Multiplication(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                lhs_type
            },
            Expression::Division(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                lhs_type
            },
            Expression::LessThan(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                Type::Bool
            },
            Expression::GreaterThan(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                Type::Bool
            },
            Expression::LessThanEquals(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                Type::Bool
            },
            Expression::GreaterThanEquals(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                Type::Bool
            },
            Expression::Equals(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                Type::Bool
            },
            Expression::NotEquals(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                Type::Bool
            },
            Expression::Not(op) => {
                let op_type = self.check_expr_type(op)?;
                if Type::Bool != op_type {
                    return Err(CompilerError::TypeMismatch(Type::Bool, op_type));
                }
                Type::Bool
            },
            Expression::And(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                Type::Bool
            },
            Expression::Or(lhs, rhs) => {
                let lhs_type = self.check_expr_type(lhs)?;
                let rhs_type = self.check_expr_type(rhs)?;
                if lhs_type != rhs_type {
                    return Err(CompilerError::TypeMismatch(lhs_type, rhs_type));
                }
                Type::Bool
            },
            _ => return Err(CompilerError::UnsupportedExpression(expr.clone()))
        };
        Ok(expr_type)
        //Err(CompilerError::Unimplemented(format!("Expr type checking not implemented!")))
    }

    pub fn check_member_access_expr_type(&self, expr: &Expression, cont_def: Option<&ContainerDef>) -> CompilerResult<Type> {
        let (lhs_expr, rhs_expr) = match expr {
            Expression::MemberAccess(lhs, rhs) => (lhs.deref(), rhs.deref()),
            _ => return Err(CompilerError::Unknown)
        };

        let lhs_type = match lhs_expr {
            Expression::Variable(var_name) => {
                // If this is a stack variable
                if cont_def.is_none() {
                    self.get_type_of_var(var_name)?
                }
                // If this is a member
                else {
                    let cont_def = cont_def.unwrap();
                    cont_def.get_member_type(var_name)?
                }
            },
            _ => return Err(CompilerError::UnsupportedExpression(lhs_expr.clone()))
        };

        let cont_name = match &lhs_type {
            Type::Other(cont_name) => cont_name,
            Type::Reference(inner_type) => {
                match inner_type.deref() {
                    Type::Other(cont_name) => cont_name,
                    _ => return Err(CompilerError::MemberAccessOnNonContainer)
                }
            },
            _ => return Err(CompilerError::MemberAccessOnNonContainer)
        };

        let cont_def = self.resolve_container(cont_name)?;

        match &rhs_expr {
            Expression::Variable(var_name) => {
                cont_def.get_member_type(var_name)
            },
            Expression::Call(fn_name, _) => {
                let fn_def = cont_def.get_member_function(fn_name)?;
                Ok(fn_def.ret_type.clone())
            },
            Expression::MemberAccess(member_expr, _) => {
                let member_name = match member_expr.deref() {
                    Expression::Variable(var_name) => var_name,
                    _ => return Err(CompilerError::UnsupportedExpression(member_expr.deref().clone()))
                };
                let member_type = cont_def.get_member_type(member_name)?;
                let child_cont_name = match &member_type {
                    Type::Other(cont_name) => cont_name,
                    Type::Reference(inner_type) => {
                        match inner_type.deref() {
                            Type::Other(cont_name) => cont_name,
                            _ => return Err(CompilerError::MemberAccessOnNonContainer)
                        }
                    },
                    _ => return Err(CompilerError::MemberAccessOnNonContainer)
                };
                let child_cont_def = self.resolve_container(child_cont_name)?;
                self.check_member_access_expr_type(rhs_expr, Some(&child_cont_def))
            },
            _ => return Err(CompilerError::MemberAccessOnNonContainer)
        }
    }

    // #endregion
}