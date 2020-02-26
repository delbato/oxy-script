use crate::{
    api::{
        function::Function
    }
};

use std::{
    collections::HashMap
};

/// A Container definition
pub struct Container {
    pub name: String,
    pub members: HashMap<String, ContainerMember>
}

impl Container {
    /// Creates a new container
    pub fn new(name: String) -> Container {
        Container {
            name: name,
            members: HashMap::new()
        }
    }

    /// ...with a member function
    pub fn with_function(mut self, function: Function) -> Container {
        self.members.insert(function.name.clone(), ContainerMember::Function(function));
        self
    }

    /// ...with a member variable
    pub fn with_variable(mut self, (name, acc_fn): (String, Function)) -> Container {
        self.members.insert(name.clone(), ContainerMember::Variable {
            name: name,
            accessor_fn: acc_fn
        });
        self
    }
}

pub enum ContainerMember {
    Function(Function),
    Variable {
        name: String,
        accessor_fn: Function
    }
}

