use std::{
    collections::{
        HashMap,
        BTreeMap
    },
    ops::Deref
};

#[derive(PartialEq, Debug, Clone)]
pub enum Expression {
    IntLiteral(i64),
    FloatLiteral(f32),
    StringLiteral(String),
    BoolLiteral(bool),
    Variable(String),
    ContainerInstance(String, HashMap<String, Expression>),
    MemberAccess(Box<Expression>, Box<Expression>),
    Deref(Box<Expression>),
    Ref(Box<Expression>),
    Call(String, Vec<Expression>),
    Addition(Box<Expression>, Box<Expression>),
    Subtraction(Box<Expression>, Box<Expression>),
    Multiplication(Box<Expression>, Box<Expression>),
    Division(Box<Expression>, Box<Expression>),
    Not(Box<Expression>),
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Equals(Box<Expression>, Box<Expression>),
    NotEquals(Box<Expression>, Box<Expression>),
    GreaterThan(Box<Expression>, Box<Expression>),
    LessThan(Box<Expression>, Box<Expression>),
    GreaterThanEquals(Box<Expression>, Box<Expression>),
    LessThanEquals(Box<Expression>, Box<Expression>),
    Assign(Box<Expression>, Box<Expression>),
    AddAssign(Box<Expression>, Box<Expression>),
    SubAssign(Box<Expression>, Box<Expression>),
    MulAssign(Box<Expression>, Box<Expression>),
    DivAssign(Box<Expression>, Box<Expression>),
}

impl Expression {
    pub fn print(&self, n: u8) {
        let mut baseline = String::new();
        for i in 0..n {
            baseline += "----";
        }
        match self {
            Expression::IntLiteral(int) => {
                //println!("{} Int:{}", baseline, int);
            },
            Expression::FloatLiteral(float) => {
                //println!("{} Float:{}", baseline, float);
            },
            Expression::StringLiteral(string) => {
                //println!("{} String:{}", baseline, string);
            },
            Expression::Variable(variable) => {
                //println!("{} Variable:{}", baseline, variable);
            },
            Expression::Addition(lhs, rhs) => {
                //println!("{} Addition:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            Expression::Subtraction(lhs, rhs) => {
                //println!("{} Subtraction:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            Expression::Multiplication(lhs, rhs) => {
                //println!("{} Multiplication:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            Expression::Division(lhs, rhs) => {
                //println!("{} Division:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            Expression::MemberAccess(lhs, rhs) => {
                //println!("{} Member access:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1);
            },
            Expression::Call(fn_name, args) => {
                //println!("{} Call \"{}\":", baseline, fn_name);
                //println!("{} Arguments:", baseline);
                for arg in args.iter() {
                    arg.print(n + 1);
                }
            },
            Expression::Assign(lhs, rhs) => {
                //println!("{} Assign:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            Expression::AddAssign(lhs, rhs) => {
                //println!("{} AddAssign:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            Expression::SubAssign(lhs, rhs) => {
                //println!("{} SubAssign:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            Expression::MulAssign(lhs, rhs) => {
                //println!("{} MulAssign:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            Expression::DivAssign(lhs, rhs) => {
                //println!("{} DivAssign:", baseline);
                lhs.print(n + 1);
                rhs.print(n + 1)
            },
            _ => {
                //println!("{} Other:", baseline);
            }
        }
    }

    /// Checks if an expression is a member access expr
    pub fn is_member_access(&self) -> bool {
        match self {
            Expression::MemberAccess(_, _) => true,
            _ => false
        }
    }
    /// Checks if an expression ends in a call expr
    pub fn ends_in_call(&self) -> bool {
        match self {
            Expression::MemberAccess(_, rhs) => {
                rhs.ends_in_call()
            },
            Expression::Call(_, _) => true,
            _ => false
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum Operator {
    OpenParan,
    CloseParan,
    Plus,
    Minus,
    Times,
    Divide,
    Equals,
    NotEquals,
    GreaterThan,
    GreaterThanEquals,
    LessThan,
    LessThanEquals,
    Not
}

#[derive(PartialEq, Debug, Clone)]
pub struct FunctionDeclArgs {
    pub name: String,
    pub arguments: Vec<(String, Type)>,
    pub returns: Type,
    pub code_block: Option<Vec<Statement>>
}

#[derive(PartialEq, Debug, Clone)]
pub struct ContainerDeclArgs {
    pub name: String,
    pub members: Vec<(String, Type)>
}

#[derive(PartialEq, Debug)]
pub enum Declaration {
    Function(FunctionDeclArgs),
    Module(String, Vec<Declaration>),
    Container(ContainerDeclArgs),
    Import(String, String),
    Impl(String, String, Vec<Declaration>),
    Interface(String, Vec<Declaration>),
    StaticVar(VariableDeclArgs)
}

#[derive(PartialEq, Debug, Clone)]
pub struct VariableDeclArgs {
    pub var_type: Type,
    pub name: String,
    pub assignment: Box<Expression>
}

#[derive(PartialEq, Debug, Clone)]
pub struct IfStatementArgs {
    pub if_expr: Expression,
    pub if_block: Vec<Statement>,
    pub else_block: Option<Vec<Statement>>,
    pub else_if_list: Option<Vec<(Expression, Vec<Statement>)>>
}

#[derive(PartialEq, Debug, Clone)]
pub enum Statement {
    VariableDecl(VariableDeclArgs),
    Assignment(String, Box<Expression>),
    Call(String, Vec<Expression>),
    Return(Option<Expression>),
    CodeBlock(Vec<Statement>),
    Loop(Vec<Statement>),
    While(Box<Expression>, Vec<Statement>),
    Break,
    Continue,
    Expression(Expression),
    If(IfStatementArgs)
}

#[derive(PartialEq, Debug, Clone)]
pub enum Type {
    Void,
    Int,
    String,
    Float,
    Bool,
    Auto,
    Array(Box<Type>, usize),
    AutoArray(Box<Type>),
    Other(String),
    Tuple(Vec<Type>),
    Reference(Box<Type>)
}

impl Type {
    pub fn is_primitive(&self) -> bool {
        match self {
            Type::Bool => true,
            Type::Int => true,
            Type::Float => true,
            Type::Reference(inner_type) => {
                match inner_type.deref() {
                    Type::AutoArray(_) => false,
                    _ => true
                }
            },
            _ => false
        }
    }

    pub fn get_ref_type(&self) -> Type {
        match self {
            Type::Reference(inner_type) => {
                inner_type.deref().clone()
            },
            _ => panic!("Not a reference!")
        }
    }

    pub fn is_cont_reference(&self) -> bool {
        match self {
            Type::Reference(inner_type) => {
                match **inner_type {
                    Type::Other(_) => true,
                    _ => false
                }
            },
            _ => false
        }
    }

    pub fn get_cont_name(&self) -> Option<&String> {
        match self {
            Type::Other(cont_name) => Some(cont_name),
            Type::Reference(inner_type) => {
                match inner_type.deref() {
                    Type::Other(cont_name) => Some(&cont_name),
                    _ => None
                }
            },
            _ => None,
        }
    }
}
