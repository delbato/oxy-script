/// Contains the "core" library
pub mod core;

use crate::{
    vm::{
        core::{
            Core,
            CoreError
        },
        register::{
            RegisterAccess,
            Register as RegisterUnion
        }
    },
    parser::{
        parser::{
            ParseError,
            Parser
        },
        ast::{
            Declaration,
            Statement
        }
    },
    codegen::{
        compiler::{
            Compiler,
            CompilerError
        },
        register::Register
    },
    api::{
        module::Module
    }
};

use std::{
    io::{
        Read
    },
    fs::{
        File
    },
    path::{
        Path,
        PathBuf
    },
    error::Error,
    fmt::{
        Display,
        Debug,
        Formatter,
        Result as FmtResult
    }
};

use serde::{
    de::DeserializeOwned,
    Serialize
};

pub struct Engine {
    core: Core,
    pub compiler: Compiler,
    pub script_root_dir: Option<PathBuf>
}

pub type EngineResult<T> = Result<T, Box<EngineError>>;

#[derive(Debug)]
pub enum EngineError {
    Unknown,
    CoreError(CoreError),
    ParseError(ParseError),
    CompileError(CompilerError),
}

impl Display for EngineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}

impl Error for EngineError {
}

impl Engine {
    pub fn new(stack_size: usize) -> Engine {
        let mut compiler = Compiler::new();
        Engine {
            core: Core::new(stack_size),
            compiler: compiler,
            script_root_dir: None
        }
    }

    pub fn run_code(&mut self, code: &str) -> EngineResult<()> {
        self.load_code(code)?;
        self.run_fn(&String::from("root::main"))
    }

    pub fn load_code(&mut self, code: &str) -> EngineResult<()> {
        let parser = Parser::new(String::from(code));
        if self.script_root_dir.is_some() {
            let script_root_dir = self.script_root_dir.as_ref().unwrap();
            parser.set_root_dir(&script_root_dir);
        }
        let decl_list = parser.parse_root_decl_list()
            .map_err(|p| {
                let mut offset = 0;
                let token_range = p.token_pos.clone();
                let mut line_nr = 0;
                for line in code.lines() {
                    if offset <= token_range.start && offset + line.len() >= token_range.end {
                        //println!("Parse error in line #{} at offset {}", line_nr, token_range.start - offset);
                    }
                    offset += line.len();
                    line_nr += 1;
                }
                Box::new(EngineError::ParseError(p))
            })?;
        self.compiler.compile_root(&decl_list)
            .map_err(|c| Box::new(EngineError::CompileError(c)))?;
        let program = self.compiler.get_program()
            .map_err(|c| Box::new(EngineError::CompileError(c)))?;
        self.core.load_program(program);
        Ok(())
    }

    pub fn run_file(&mut self, path: &Path) -> EngineResult<()> {
        let mut file = File::open(path)
            .map_err(|_| Box::new(EngineError::Unknown))?;
        let script_root_dir = path.parent()
            .ok_or(EngineError::Unknown)?;
        self.script_root_dir = Some(PathBuf::from(script_root_dir));
        let mut file_content = String::new();
        file.read_to_string(&mut file_content)
            .map_err(|_| Box::new(EngineError::Unknown))?;
        self.run_code(&file_content)?;
        self.script_root_dir = None;
        Ok(())
    }

    pub fn load_file(&mut self, path: &Path) -> EngineResult<()> {
        let mut file = File::open(path)
            .map_err(|_| Box::new(EngineError::Unknown))?;
        let script_root_dir = path.parent()
            .ok_or(EngineError::Unknown)?;
        self.script_root_dir = Some(PathBuf::from(script_root_dir));
        let mut file_content = String::new();
        file.read_to_string(&mut file_content)
            .map_err(|_| Box::new(EngineError::Unknown))?;
        self.load_code(&file_content)?;
        self.script_root_dir = None;
        Ok(())
    }

    pub fn run_stream(&mut self, readable: Box<dyn Read>) -> EngineResult<()> {
        Err(Box::new(EngineError::Unknown))
    }

    pub fn push_stack<T: Serialize>(&mut self, item: T) -> EngineResult<()> {
        self.core.push_stack(item)
            .map_err(|c| Box::new(EngineError::CoreError(c)))
    }

    pub fn pop_stack<T: DeserializeOwned>(&mut self) -> EngineResult<T> {
        self.core.pop_stack()
            .map_err(|c| Box::new(EngineError::CoreError(c)))
    }

    pub fn get_register_value<T>(&mut self, reg: Register) -> EngineResult<T>
        where RegisterUnion: RegisterAccess<T> {
        let val = self.core.reg(reg.into())
            .map_err(|ce| EngineError::CoreError(ce))?
            .get::<T>();
        Ok(val)
    }

    pub fn get_stack_size(&self) -> usize {
        self.core.get_stack_size()
    }

    pub fn run_fn<T>(&mut self, name: T) -> EngineResult<()>
        where String: From<T> {
        let name = String::from(name);
        let fn_uid = self.compiler.get_function_uid(&name)  
            .map_err(|ce| EngineError::CompileError(ce))?;
        self.core.run_fn(fn_uid)
            .map_err(|c| Box::new(EngineError::CoreError(c)))
    }

    pub fn register_module(&mut self, module: Module) -> EngineResult<()> {
        self.compiler.register_foreign_root_module(module)
            .map_err(|ce| Box::new(EngineError::CompileError(ce)))
    }
}
