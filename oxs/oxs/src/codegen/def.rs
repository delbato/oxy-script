use crate::{
    parser::{
        ast::{
            Type,
            FunctionDeclArgs,
            ContainerDeclArgs
        }
    },
    codegen::{
        compiler::{
            CompilerResult,
            CompilerError,
            Compiler
        }
    }
};

use std::{
    collections::{
        HashMap,
        HashSet,
        BTreeMap
    },
    convert::{
        From
    }
};

/// A function definition
#[derive(Clone, PartialEq, Debug)]
pub struct FunctionDef {
    pub name: String,
    pub uid: u64,
    pub ret_type: Type,
    pub arguments: Vec<(String, Type)>
}

impl FunctionDef {
    /// Creates a new function definition with no return type or arguments
    pub fn new(name: String) -> FunctionDef {
        FunctionDef {
            name: name,
            uid: 0,
            ret_type: Type::Void,
            arguments: Vec::new()
        }
    }

    /// With a specific return type
    pub fn with_ret_type(mut self, ret_type: Type) -> FunctionDef {
        self.ret_type = ret_type;
        self
    }

    /// With decl args arguments
    pub fn with_arguments(mut self, arguments: &[(String, Type)]) -> FunctionDef {
        for argument in arguments.iter() {
            self.arguments.push(argument.clone());
        }
        self
    }

    /// With a uid
    pub fn with_uid(mut self, uid: u64) -> FunctionDef {
        self.uid = uid;
        self
    }
}

impl From<&FunctionDeclArgs> for FunctionDef {
    fn from(item: &FunctionDeclArgs) -> FunctionDef {
        FunctionDef::new(item.name.clone())
            .with_ret_type(item.returns.clone())
            .with_arguments(&item.arguments)
    }
}

/// A container definition
#[derive(Clone, Debug)]
pub struct ContainerDef {
    /// Name of the container
    pub name: String,
    /// Name of the container, including full module path
    pub canonical_name: String,
    /// Map of member variable types
    pub member_variables: HashMap<String, Type>,
    /// Map of member variable indices
    pub member_indices: BTreeMap<String, usize>,
    /// Map of member functions
    pub member_functions: HashMap<String, FunctionDef>,
    /// Map of interface implements
    pub interfaces: HashSet<String>
}

impl ContainerDef {
    /// Creates a new container definition
    pub fn new(name: String, canon_name: String) -> ContainerDef {
        ContainerDef {
            name: name,
            canonical_name: canon_name,
            member_indices: BTreeMap::new(),
            member_functions: HashMap::new(),
            member_variables: HashMap::new(),
            interfaces: HashSet::new()
        }
    }

    /// Marks this container as implementing an interface
    pub fn implements(&mut self, intf_canon_name: String) {
        self.interfaces.insert(intf_canon_name);
    }

    /// Returns true if this container implements an interface
    pub fn does_implement(&self, intf_canon_name: &String) -> bool {
        self.interfaces.contains(intf_canon_name)
    }

    /// Adds a member variable
    pub fn add_member_variable(&mut self, var: (String, Type)) -> CompilerResult<()> {
        if self.member_variables.contains_key(&var.0) {
            return Err(CompilerError::DuplicateMember(var.0));
        }
        self.member_variables.insert(var.0.clone(), var.1);
        let index = self.member_indices.len();
        self.member_indices.insert(var.0, index);
        Ok(())
    }

    /// Adds a member function
    pub fn add_member_function(&mut self, fn_def: FunctionDef) -> CompilerResult<()> {
        if self.member_functions.contains_key(&fn_def.name) {
            return Err(CompilerError::DuplicateFunction(fn_def.name));
        }
        self.member_functions.insert(fn_def.name.clone(), fn_def);
        Ok(())
    }

    /// Gets the byte offset of a member
    pub fn get_member_offset(&self, compiler: &Compiler, var_name: &String) -> CompilerResult<usize> {
        let target_index = self.get_member_index(var_name)?;
        let mut offset = 0;
        for (member_name, member_index) in self.member_indices.iter() {
            let member_type = self.get_member_type(member_name)?;
            let member_size = compiler.get_size_of_type(&member_type)?;
            if *member_index == target_index {
                break;
            }
            offset += member_size;
        }
        Ok(offset)
    }

    /// Returns the type of a member
    pub fn get_member_type(&self, var_name: &String) -> CompilerResult<Type> {
        self.member_variables.get(var_name)
            .cloned()
            .ok_or(CompilerError::UnknownMember(var_name.clone()))
    }

    /// Returns the byte size of this container
    pub fn get_size(&self, compiler: &Compiler) -> CompilerResult<usize> {
        let mut size = 0;
        for (_, var_type) in self.member_variables.iter() {
            size += compiler.get_size_of_type(var_type)?;
        }
        Ok(size)
    }

    /// Returns the index of a member
    pub fn get_member_index(&self, name: &String) -> CompilerResult<usize> {
        self.member_indices.get(name)
            .cloned()
            .ok_or(CompilerError::UnknownMember(name.clone()))
    }

    /// Returns a function definition 
    pub fn get_member_function(&self, name: &String) -> CompilerResult<&FunctionDef> {
        self.member_functions.get(name)
            .ok_or(CompilerError::UnknownMember(name.clone()))
    }

    /// Merges a container declaration into an existing containerdef
    pub fn merge_cont_decl(&mut self, item: &ContainerDeclArgs) {
        for member in item.members.iter() {
            self.add_member_variable(member.clone()).unwrap();
        }
    }

    /// Creates a new ContainerDef from a declaration
    pub fn from_decl(item: &ContainerDeclArgs, canon_name: String) -> ContainerDef {
        let mut def = ContainerDef::new(item.name.clone(), canon_name);
        def.merge_cont_decl(item);
        def
    }
}

#[derive(Clone, Debug)]
pub struct InterfaceDef {
    pub name: String,
    pub functions: HashMap<String, FunctionDef>
}

impl InterfaceDef {
    pub fn new(name: String) -> Self {
        InterfaceDef {
            name: name,
            functions: HashMap::new()
        }
    }

    pub fn add_function(&mut self, fn_def: FunctionDef) {
        let fn_name = fn_def.name.clone();
        self.functions.insert(fn_name, fn_def);
    }

    pub fn get_function(&self, fn_name: &str) -> Option<&FunctionDef> {
        self.functions.get(fn_name)
    }
}