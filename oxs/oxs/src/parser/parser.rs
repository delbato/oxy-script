use super::{
    ast::{
        *
    },
    lexer::{
        Token,
        OxyLexer as Lexer
    }
};

use std::{
    collections::{
        HashMap,
        VecDeque,
        HashSet,
        BTreeMap
    },
    fmt::{
        Debug,
        Display,
        Formatter,
        Result as FmtResult
    },
    fs::{
        File
    },
    error::Error,
    ops::{
        Range,
        Deref
    },
    io::Read,
    cell::RefCell,
    path::{
        Path,
        PathBuf
    }
};

use oxlex::prelude::Lexable;

#[derive(Debug)]
pub enum ParseErrorType {
    Unknown,
    Unimplemented,
    EmptyInput,
    FnMissing,
    OpenParanMissing,
    CloseParanMissing,
    BlockMissing,
    ExpectedFunctionName,
    ReturnTypeMissing,
    UnknownType,
    ExpectedArgType,
    ExpectedArgName,
    ExpectedLoop,
    DuplicateArg,
    ExpectedBlockOrSemicolon,
    ExpectedCloseBlock,
    UnknownStatement,
    ExpectedVarName,
    ExpectedWhile,
    ExpectedAssignment,
    ExpectedSemicolon,
    UnsupportedExpression,
    ExpectedColon,
    ExpectedOpenParan,
    ExpectedCloseParan,
    ExpectedStructName,
    ExpectedModName,
    ExpectedOpenBlock,
    ExpectedMemberType,
    ExpectedMemberName,
    ExpectedContainerName,
    ExpectedArraySize,
    ExpectedCloseBracket,
    NotInFileMode,
    AmbiguousModuleFile(String),
    NoModuleFile(String),
    InvalidTypename(String),
    InvalidTokenInTypename(Token),
    DuplicateMember,
    ExpectedImport,
    ExpectedImportString,
    ExpectedMod,
    ExpectedIf,
    ExpectedImpl,
    ExpectedImplType,
    ExpectedThis,
    ThisOnlyAllowedInImpls,
    MalformedImport
}

#[derive(Debug)]
pub struct ParseError {
    pub error_type: ParseErrorType,
    pub token_pos: Range<usize>
}

impl ParseError {
    pub fn new(err_type: ParseErrorType, pos: Range<usize>) -> ParseError {
        ParseError {
            error_type: err_type,
            token_pos: pos
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self)
    }
}

impl Error for ParseError {}

macro_rules! make_parse_error {
    ($lexer:ident, $error:expr) => {
        Err(ParseError::new($error, $lexer.range()))
    };
}

pub type ParseResult<T> = Result<T, ParseError>;

pub struct Parser {
    code: String,
    current_cont: RefCell<String>,
    script_root_dir: RefCell<Option<PathBuf>>
}

fn is_op(token: &Token) -> bool {
    match token {
        Token::Times => true,
        Token::Divide => true,
        Token::Plus => true,
        Token::Minus => true,
        Token::Equals => true,
        Token::NotEquals => true,
        Token::GreaterThan => true,
        Token::GreaterThanEquals => true,
        Token::LessThan => true,
        Token::LessThanEquals => true,
        Token::Not => true,
        Token::Tilde => true,
        Token::And => true,
        Token::Dot => true,
        Token::Assign => true,
        Token::AddAssign => true,
        Token::MulAssign => true,
        Token::SubAssign => true,
        Token::DivAssign => true,
        Token::DoubleDot => true,
        Token::Or => true,
        Token::DoubleAnd => true,
        _ => false
    }
}

fn op_prec(token: &Token) -> i8 {
    match token {
        Token::Times => 3,
        Token::Divide => 3,
        Token::Plus => 2,
        Token::Minus => 2,
        Token::Equals => 1,
        Token::NotEquals => 1,
        Token::GreaterThan => 1,
        Token::GreaterThanEquals => 1,
        Token::LessThan => 1,
        Token::LessThanEquals => 1,
        Token::Not => 4,
        Token::And => 2,
        Token::Tilde => 2,
        Token::Dot => 5,
        Token::Assign => 0,
        Token::AddAssign => 0,
        Token::MulAssign => 0,
        Token::SubAssign => 0,
        Token::DivAssign => 0,
        Token::DoubleDot => 0,
        Token::Or => 0,
        Token::DoubleAnd => 0,
        _ => {
            panic!("ERROR! Not an operator");
        }
    }
}

fn is_op_right_assoc(token: &Token) -> bool {
    match token {
        Token::Times => true,
        Token::Divide => false,
        Token::Plus => false,
        Token::Minus => false,
        Token::Equals => false,
        Token::NotEquals => false,
        Token::GreaterThan => false,
        Token::GreaterThanEquals => false,
        Token::LessThan => false,
        Token::LessThanEquals => false,
        Token::Not => true,
        Token::Tilde => true,
        Token::And => true,
        Token::Dot => true,
        Token::Assign => true,
        Token::AddAssign => true,
        Token::MulAssign => true,
        Token::SubAssign => true,
        Token::DivAssign => true,
        Token::DoubleDot => false,
        Token::Or => false,
        Token::DoubleAnd => false,
        _ => {
            panic!("ERROR! Not an operator");
        }
    }
}

impl Parser {
    pub fn new(code: String) -> Self {
        Parser {
            code: code,
            current_cont: RefCell::new(String::new()),
            script_root_dir: RefCell::new(None)
        }
    }

    /// Sets the scripts root directory
    pub fn set_root_dir(&self, path: &Path) {
        let root_dir = PathBuf::from(path);
        *(self.script_root_dir.borrow_mut()) = Some(root_dir);
    }

    /// Gets the scripts root directory
    pub fn get_root_dir(&self) -> ParseResult<PathBuf> {
        self.script_root_dir.borrow()
            .deref()
            .as_ref()
            .cloned()
            .ok_or(ParseError::new(ParseErrorType::NotInFileMode, 0..0))
    }

    /// Clears the scripts root directory
    pub fn clear_root_dir(&self) {
        *(self.script_root_dir.borrow_mut()) = None;
    }

    pub fn parse_decl_list(&self, lexer: &mut Lexer, delims: &[Token]) -> ParseResult<Vec<Declaration>> {
        let mut ret = Vec::new();
        
        while !delims.contains(&lexer.token) &&
            lexer.token != Token::End &&
            lexer.token != Token::Error {
            match lexer.token {
                Token::Fn => {
                    ret.push(self.parse_fn_decl(lexer)?);
                },
                Token::Container => {
                    ret.push(self.parse_container_decl(lexer)?);
                },
                Token::Import => {
                    let mut import_decls = self.parse_import_decl(lexer)?;
                    ret.append(&mut import_decls);
                },
                Token::Mod => {
                    ret.push(self.parse_mod_decl(lexer)?);
                },
                Token::Impl => {
                    ret.push(self.parse_impl_decl(lexer)?);
                },
                _ => {
                    return Err(ParseError::new(ParseErrorType::ExpectedMod, lexer.range()));
                }
            };
        }

        Ok(ret)
    }

    pub fn parse_impl_decl(&self, lexer: &mut Lexer) -> ParseResult<Declaration> {
        if lexer.token != Token::Impl {
            return make_parse_error!(lexer, ParseErrorType::ExpectedImpl);
        }

        // Swallow "impl"
        lexer.advance();

        if lexer.token != Token::Colon {
            return make_parse_error!(lexer, ParseErrorType::ExpectedColon);
        }

        // Swallow ":"
        lexer.advance();

        if lexer.token != Token::Text {
            return make_parse_error!(lexer, ParseErrorType::ExpectedImplType);
        }

        let impl_type = self.parse_mod_path(lexer)?;
        let mut impl_for = impl_type.clone();

        if lexer.token == Token::For {
            // Swallow "for" if its next
            lexer.advance();

            impl_for = self.parse_mod_path(lexer)?;
        }

        if lexer.token != Token::OpenBlock {
            return make_parse_error!(lexer, ParseErrorType::ExpectedOpenBlock);
        }

        // Swallow "{"
        lexer.advance();

        *(self.current_cont.borrow_mut()) = impl_type.clone();

        let decl_list = self.parse_decl_list(lexer, &[Token::CloseBlock])?;

        *(self.current_cont.borrow_mut()) = String::new();

        // Swallow "}"
        lexer.advance();

        Ok(
            Declaration::Impl(impl_type, impl_for, decl_list)
        )
    }

    pub fn parse_root_decl_list(&self) -> ParseResult<Vec<Declaration>> {
        let mut lexer = Token::lexer(self.code.as_str());
        self.parse_decl_list(&mut lexer, &[])
    }

    pub fn parse_mod_decl(&self, lexer: &mut Lexer) -> ParseResult<Declaration> {
        if lexer.token != Token::Mod {
            return Err(ParseError::new(ParseErrorType::ExpectedMod, lexer.range()));
        }
        // Swallow "mod"
        lexer.advance();

        if lexer.token != Token::Colon {
            return Err(ParseError::new(ParseErrorType::ExpectedColon, lexer.range()));
        }

        // Swallow ":"
        lexer.advance();

        if lexer.token != Token::Text {
            return Err(ParseError::new(ParseErrorType::ExpectedModName, lexer.range()));
        }

        let mod_name = String::from(lexer.slice());

        // Swallow mod name
        lexer.advance();

        let decl_list;

        if lexer.token == Token::Semicolon {
            // Swallow ";"
            lexer.advance();
            decl_list = self.parse_mod_file_decl_list(lexer, &mod_name)?;
        } else if lexer.token == Token::OpenBlock {
            // Swallow "{"
            lexer.advance();
            // Parse contents
            decl_list = self.parse_decl_list(lexer, &[Token::CloseBlock])?;
            // Swallow "}"
            lexer.advance();
        } else {
            return make_parse_error!(lexer, ParseErrorType::Unknown);
        }

        //println!("Decl list of mod {}: {:?}", mod_name, decl_list);

        Ok(
            Declaration::Module(mod_name, decl_list)
        )
    }

    pub fn parse_mod_file_decl_list(&self, old_lexer: &Lexer, mod_name: &String) -> ParseResult<Vec<Declaration>> {
        //println!("Parsing module file with name {}", mod_name);
        let mut script_root_dir = self.get_root_dir()?;
        let mut single_file_name = mod_name.clone();
        single_file_name += ".oxs";
        let single_file_path = script_root_dir.join(single_file_name);
        let multi_file_path = script_root_dir.join(mod_name).join("mod.oxs");

        if single_file_path.exists() && multi_file_path.exists() {
            return make_parse_error!(old_lexer, ParseErrorType::AmbiguousModuleFile(mod_name.clone()));
        } else if single_file_path.exists() {
            //println!("Is single file. path: {}", single_file_path.to_str().unwrap());
            let mut file = File::open(single_file_path)
                .map_err(|_| ParseError::new(ParseErrorType::Unknown, old_lexer.range()))?;
            let mut file_contents = String::new();
            file.read_to_string(&mut file_contents)
                .map_err(|_| ParseError::new(ParseErrorType::Unknown, old_lexer.range()))?;
            let mut lexer = Token::lexer(file_contents.as_str());
            let decl_list = self.parse_decl_list(&mut lexer, &[])?;
            //println!("Decl list: {:?}", decl_list);
            return Ok(decl_list);
        } else if multi_file_path.exists() {
            script_root_dir = PathBuf::from(multi_file_path.parent().unwrap());
            self.set_root_dir(&script_root_dir);
            let mut file = File::open(multi_file_path)
                .map_err(|_| ParseError::new(ParseErrorType::Unknown, old_lexer.range()))?;
            let mut file_contents = String::new();
            file.read_to_string(&mut file_contents)
                .map_err(|_| ParseError::new(ParseErrorType::Unknown, old_lexer.range()))?;
            let mut lexer = Token::lexer(file_contents.as_str());
            let decl_list = self.parse_decl_list(&mut lexer, &[])?;
            script_root_dir = PathBuf::from(script_root_dir.parent().unwrap());
            self.set_root_dir(&script_root_dir);
            return Ok(decl_list);
        } else {
            return make_parse_error!(old_lexer, ParseErrorType::NoModuleFile(mod_name.clone()));
        }
    }

    pub fn parse_import_string(&self, lexer: &mut Lexer, delims: &[Token]) -> ParseResult<(String, String)> {
        let mut import_string = String::new();
        let mut import_as = String::new();

        while !delims.contains(&lexer.token) {
            if lexer.token == Token::Times {
                if import_string.is_empty() {
                    return make_parse_error!(lexer, ParseErrorType::MalformedImport);
                }
                lexer.advance();
                import_as = String::from("*");
                continue;
            }

            if lexer.token != Token::Text {
                return make_parse_error!(lexer, ParseErrorType::ExpectedImportString);
            }

            import_string += lexer.slice();
            import_as = String::from(lexer.slice());
            lexer.advance();

            if lexer.token != Token::DoubleColon {
                break;
            }

            lexer.advance();

            import_string += "::";
        }

        if import_string.is_empty() {
            return make_parse_error!(lexer, ParseErrorType::ExpectedImportString);
        }

        Ok(
            (import_string, import_as)
        )
    }

    pub fn parse_multi_import(&self, lexer: &mut Lexer) -> ParseResult<Vec<Declaration>> {
        let delims = &[
            Token::End,
            Token::Error,
            Token::CloseBlock,
            Token::Semicolon
        ];

        let mut import_decls = Vec::new();

        while !delims.contains(&lexer.token) {
            let (import_name, mut import_as) = self.parse_import_string(lexer, &[Token::Comma, Token::CloseBlock, Token::OpenBlock, Token::Assign])?;
            if lexer.token == Token::Comma {
                lexer.advance();
            }
            match lexer.token {
                Token::Assign => {
                    lexer.advance();
                    if lexer.token != Token::Text {
                        return make_parse_error!(lexer, ParseErrorType::ExpectedImportString);
                    }
                    import_as = String::from(lexer.slice());
                    let decl = Declaration::Import(import_name, import_as);
                    import_decls.push(decl);
                    lexer.advance();
                    if lexer.token == Token::Comma {
                        lexer.advance();
                    }
                },
                Token::OpenBlock => {
                    if !import_name.ends_with("::") {
                        return make_parse_error!(lexer, ParseErrorType::MalformedImport);
                    }
                    lexer.advance();
                    let mut nested_decls = self.parse_multi_import(lexer)?;

                    for decl in nested_decls.iter_mut() {
                        if let Declaration::Import(decl_name, _) = decl {
                            let mut new_name = import_name.clone();
                            new_name += &decl_name;
                            *decl_name = new_name;
                        }
                    }

                    import_decls.append(&mut nested_decls);
                },
                _ => {
                    let decl = Declaration::Import(import_name, import_as);
                    import_decls.push(decl);
                }
            };
        }

        if lexer.token != Token::CloseBlock && lexer.token != Token::Semicolon {
            return make_parse_error!(lexer, ParseErrorType::MalformedImport);
        }

        lexer.advance();

        Ok(import_decls)
    }

    pub fn parse_import_decl(&self, lexer: &mut Lexer) -> ParseResult<Vec<Declaration>> {
        if lexer.token != Token::Import {
            return Err(ParseError::new(ParseErrorType::ExpectedImport, lexer.range()));
        }

        // Swallow "import"
        lexer.advance();

        if lexer.token != Token::Colon {
            return make_parse_error!(lexer, ParseErrorType::ExpectedColon);
        }

        // Swallow ":"
        lexer.advance();

        let delims = &[
            Token::Semicolon,
            Token::OpenBlock,
            Token::Assign,
            Token::End,
            Token::Error
        ];

        
        let import_decls = self.parse_multi_import(lexer)?;

        Ok(
            import_decls
        )
    }

    pub fn parse_fn_decl(&self, lexer: &mut Lexer) -> ParseResult<Declaration> {
        let mut fn_decl_opt = None;

        // Parse "fn" literal
        if lexer.token != Token::Fn {
            return Err(ParseError::new(ParseErrorType::FnMissing, lexer.range()));
        }
        lexer.advance();

        // Parse ":"
        if lexer.token != Token::Colon {
            return Err(ParseError::new(ParseErrorType::ExpectedColon, lexer.range()));
        }
        lexer.advance();

        // Parse function name
        if lexer.token != Token::Text {
            return Err(ParseError::new(ParseErrorType::ExpectedFunctionName, lexer.range()));
        }
        let fn_name = String::from(lexer.slice());
        lexer.advance();

        // Parse "("
        if lexer.token != Token::OpenParan {
            return Err(ParseError::new(ParseErrorType::OpenParanMissing, lexer.range()));
        }
        lexer.advance();

        // Parse function arguments
        let fn_args = self.parse_fn_args(lexer)?;

        if lexer.token != Token::CloseParan {
            return Err(ParseError::new(ParseErrorType::CloseParanMissing, lexer.range()));
        }
        lexer.advance();

        let fn_return_type;

        if lexer.token == Token::Tilde {
            lexer.advance();
            fn_return_type = self.parse_type(lexer)?;
        } else {
            fn_return_type = Type::Void;
        }

        let code_block_opt;

        match lexer.token {
            Token::Semicolon => {
                code_block_opt = None;
            },
            Token::OpenBlock => {
                lexer.advance();
                let statements = self.parse_statement_list(lexer)?;
                code_block_opt = Some(statements);
            },
            _ => {
                return Err(ParseError::new(ParseErrorType::ExpectedBlockOrSemicolon, lexer.range()));
            }
        };

        if lexer.token != Token::CloseBlock && lexer.token != Token::Semicolon {
            return Err(ParseError::new(ParseErrorType::ExpectedBlockOrSemicolon, lexer.range()));
        }

        // Swallow "}"|";"
        lexer.advance();

        let fn_raw = FunctionDeclArgs {
            name: fn_name,
            arguments: fn_args,
            returns: fn_return_type,
            code_block: code_block_opt
        };

        fn_decl_opt = Some(
            Declaration::Function(fn_raw)
        );

        fn_decl_opt.ok_or(ParseError::new(ParseErrorType::Unknown, lexer.range()))
    }

    pub fn parse_fn_args(&self, lexer: &mut Lexer) -> ParseResult<Vec<(String, Type)>> {
        let mut ret = Vec::new();
        let mut fn_arg_set = HashSet::new();

        let mut arg_index = 0;
        
        while lexer.token != Token::CloseParan &&
            lexer.token != Token::End &&
            lexer.token != Token::Error {
            let fn_arg_res = self.parse_fn_arg(lexer);
            if fn_arg_res.is_err() {
                break;
            }
            let fn_arg = fn_arg_res.unwrap();
            if fn_arg_set.contains(&fn_arg.0) {
                return Err(ParseError::new(ParseErrorType::DuplicateArg, lexer.range()));
            }
            fn_arg_set.insert(fn_arg.0.clone());

            ret.push(fn_arg);

            if lexer.token != Token::Comma {
                break;
            }

            arg_index += 1;
            lexer.advance();
        }

        

        Ok(ret)
    }

    pub fn parse_fn_arg(&self, lexer: &mut Lexer) -> ParseResult<(String, Type)> {
        let mut lexer_backup = lexer.clone();
        
        // Special case for argument "this"
        if lexer.token == Token::And {
            // Swallow "&"
            lexer.advance();
            if lexer.token != Token::Text || lexer.slice() != "this" {
                return make_parse_error!(lexer, ParseErrorType::ExpectedThis);
            }
            
            let arg_name = String::from("this");
            let cont_name = self.current_cont.borrow().clone();
            if cont_name.is_empty() {
                return make_parse_error!(lexer, ParseErrorType::ThisOnlyAllowedInImpls);
            }
            let arg_type = Type::Reference(Box::new(Type::Other(cont_name)));

            // Swallow "this"
            lexer.advance();
            return Ok((arg_name, arg_type));
        }

        if lexer.token != Token::Text {
            return Err(ParseError::new(ParseErrorType::ExpectedArgName, lexer.range()));
        }
        let arg_name = String::from(lexer.slice());
        lexer.advance();

        // Parse ":"
        if lexer.token != Token::Colon {
            return Err(ParseError::new(ParseErrorType::ExpectedColon, lexer.range()));
        }
        lexer.advance();


        let arg_type = self.parse_type(lexer)?;

        Ok(
            (arg_name, arg_type)
        )
    }

    pub fn parse_container_decl(&self, lexer: &mut Lexer) -> ParseResult<Declaration> {
        if lexer.token != Token::Container {
            return Err(ParseError::new(ParseErrorType::Unknown, lexer.range()));
        }

        // Swallow "cont"
        lexer.advance();

        if lexer.token != Token::Colon {
            return Err(ParseError::new(ParseErrorType::ExpectedColon, lexer.range()));
        }

        // Swallow ":"
        lexer.advance();

        if lexer.token != Token::Text {
            return Err(ParseError::new(ParseErrorType::ExpectedStructName, lexer.range()));
        }

        let container_name = String::from(lexer.slice());

        // Swallow container name
        lexer.advance();

        if lexer.token != Token::OpenBlock {
            return Err(ParseError::new(ParseErrorType::ExpectedOpenBlock, lexer.range()));
        }

        // Swallow "{"
        lexer.advance();

        let members = self.parse_container_members(lexer)?;

        // Swallow "}"
        lexer.advance();

        let container_args = ContainerDeclArgs {
            name: container_name,
            members: members
        };

        Ok(
            Declaration::Container(container_args)
        )
    }

    pub fn parse_type(&self, lexer: &mut Lexer) -> ParseResult<Type> {
        let ret_type = match lexer.token {
            Token::Int => {
                lexer.advance();
                Type::Int
            },
            Token::Float => {
                lexer.advance();
                Type::Float
            },
            Token::Bool => {
                lexer.advance();
                Type::Bool
            },
            Token::String => {
                lexer.advance();
                Type::String
            },
            Token::And => {
                // Swallow "&"
                lexer.advance();
                let inner_type = self.parse_type(lexer)?;
                Type::Reference(Box::new(inner_type))
            },
            Token::OpenBracket => {
                // Swallow "["
                lexer.advance();
                let arr_type = self.parse_type(lexer)?;
                let mut arr_size = None;
                if lexer.token == Token::Semicolon {
                    // Swallow ";"
                    lexer.advance();
                    if lexer.token != Token::IntLiteral {
                        return make_parse_error!(lexer, ParseErrorType::ExpectedArraySize);
                    }
                    let arr_size_raw = String::from(lexer.slice());
                    arr_size = Some(
                        arr_size_raw.parse::<usize>()
                            .map_err(|_| ParseError::new(ParseErrorType::Unknown, lexer.range()))?
                    );
                    // Swallow arr size
                    lexer.advance();
                }
                if lexer.token != Token::CloseBracket {
                    return make_parse_error!(lexer, ParseErrorType::ExpectedCloseBracket);
                }
                lexer.advance();
                if arr_size.is_none() {
                    Type::AutoArray(Box::new(arr_type))
                } else {
                    Type::Array(Box::new(arr_type), arr_size.unwrap())
                }
            },
            Token::Text => {
                let mut typename = String::new();
                while lexer.token == Token::Text ||
                    lexer.token == Token::DoubleColon {
                    typename += lexer.slice();
                    lexer.advance();
                }
                if typename.ends_with("::") {
                    return make_parse_error!(lexer, ParseErrorType::InvalidTypename(typename));
                }
                Type::Other(typename)
            },
            _ => return make_parse_error!(lexer, ParseErrorType::InvalidTokenInTypename(lexer.token.clone()))
        };
        Ok(ret_type)
    }

    pub fn parse_container_members(&self, lexer: &mut Lexer) -> ParseResult<Vec<(String, Type)>> {
        let mut ret = Vec::new();
        let mut members = HashSet::new();
        while lexer.token != Token::CloseBlock &&
            lexer.token != Token::End &&
            lexer.token != Token::Error {
            let member = self.parse_container_member(lexer)?;
            if members.contains(&member.0) {
                return Err(ParseError::new(ParseErrorType::DuplicateMember, lexer.range()));
            }
            members.insert(member.0.clone());
            ret.push(member);
        }
        Ok(ret)
    }

    pub fn parse_container_member(&self, lexer: &mut Lexer) -> ParseResult<(String, Type)> {
        if lexer.token != Token::Text {
            return Err(ParseError::new(ParseErrorType::ExpectedMemberName, lexer.range()));
        }

        let mut member_name = String::from(lexer.slice());
        // Swallow member name
        lexer.advance();

        // Workaround for logos being broken
        if member_name.len() == 1 && lexer.token == Token::Text {
            member_name += lexer.slice();
            lexer.advance();
        }

        if lexer.token != Token::Colon {
            return Err(ParseError::new(ParseErrorType::ExpectedColon, lexer.range()));
        }

        // Swallow ":"
        lexer.advance();

        let member_type = self.parse_type(lexer)?;

        if lexer.token != Token::Semicolon {
            return Err(ParseError::new(ParseErrorType::ExpectedSemicolon, lexer.range()));
        }

        // Swallow ";"
        lexer.advance();

        Ok(
            (member_name, member_type)
        )
    }

    pub fn parse_loop(&self, lexer: &mut Lexer) -> ParseResult<Statement> {
        if lexer.token != Token::Loop {
            return Err(ParseError::new(ParseErrorType::ExpectedLoop, lexer.range()));
        }

        // Swallow "loop"
        lexer.advance();

        if lexer.token != Token::OpenBlock {
            return Err(ParseError::new(ParseErrorType::ExpectedOpenBlock, lexer.range()));
        }

        // Swallow "{"
        lexer.advance();

        let stmt_list = self.parse_statement_list(lexer)?;

        if lexer.token != Token::CloseBlock {
            return Err(ParseError::new(ParseErrorType::ExpectedCloseBlock, lexer.range()));
        }

        // Swallow "}"
        lexer.advance();

        Ok(
            Statement::Loop(stmt_list)
        )
    }

    pub fn parse_while(&self, lexer: &mut Lexer) -> ParseResult<Statement> {
        if lexer.token != Token::While {
            return Err(ParseError::new(ParseErrorType::ExpectedWhile, lexer.range()));
        }

        // Swallow "while"
        lexer.advance();

        let while_expr = self.parse_expr(lexer, &[
            Token::OpenBlock,
            Token::Semicolon
        ])?;

        //println!("Parsing while with expr: {:?}", while_expr);

        if lexer.token == Token::Semicolon {
            return Ok(
                Statement::While(Box::new(while_expr), Vec::new())
            );
        }

        if lexer.token != Token::OpenBlock {
            return Err(ParseError::new(ParseErrorType::ExpectedOpenBlock, lexer.range()));
        }

        // Swallow "{"
        lexer.advance();

        let stmt_list = self.parse_statement_list(lexer)?;

        // Swallow "}"
        lexer.advance();

        Ok(
            Statement::While(Box::new(while_expr), stmt_list)
        )
    }

    pub fn parse_if(&self, lexer: &mut Lexer) -> ParseResult<Statement> {
        if lexer.token != Token::If {
            return Err(ParseError::new(ParseErrorType::ExpectedIf, lexer.range()));
        }
        // Swallow "if"
        lexer.advance();

        let if_expr = self.parse_expr(lexer, &[
            Token::OpenBlock,
            Token::Semicolon
        ])?;

        if lexer.token != Token::OpenBlock {
            return Err(ParseError::new(ParseErrorType::ExpectedOpenBlock, lexer.range()));
        }

        // Swallow "{"
        lexer.advance();

        let stmt_list = self.parse_statement_list(lexer)?;

        // Swallow "}"
        lexer.advance();

        let mut else_ifs = Vec::new();
        let mut else_stmt_list = Vec::new();

        while lexer.token == Token::Else {
            // Swallow "else"
            lexer.advance();

            if lexer.token == Token::If {
                // Swallow "if" 
                lexer.advance();

                let else_if_expr = self.parse_expr(lexer, &[
                    Token::OpenBlock
                ])?;

                if lexer.token != Token::OpenBlock {
                    return make_parse_error!(lexer, ParseErrorType::ExpectedOpenBlock);
                }
                // Swallow "{"
                lexer.advance();

                let else_if_stmt_list = self.parse_statement_list(lexer)?;

                // Swallow "}"
                lexer.advance();

                else_ifs.push((else_if_expr, else_if_stmt_list));
            } else {
                if lexer.token != Token::OpenBlock {
                    return make_parse_error!(lexer, ParseErrorType::ExpectedOpenBlock);
                }
                // Swallow "{"
                lexer.advance();
                
                else_stmt_list = self.parse_statement_list(lexer)?;
                
                // Swallow "}"
                lexer.advance();

                // Break because else is only allowed at the end
                break;
            }
        }

        let mut else_if_list_opt = None;
        if else_ifs.len() > 0 {
            else_if_list_opt = Some(else_ifs);
        }

        let mut else_opt = None;
        if else_stmt_list.len() > 0 {
            else_opt = Some(else_stmt_list);
        }

        let if_stmt_args = IfStatementArgs {
            if_expr: if_expr,
            if_block: stmt_list,
            else_block: else_opt,
            else_if_list: else_if_list_opt
        };

        Ok(
            Statement::If(if_stmt_args)
        )
    }

    pub fn parse_statement_list(&self, lexer: &mut Lexer) -> ParseResult<Vec<Statement>> {
        let mut ret = Vec::new();

        while lexer.token != Token::CloseBlock &&
            lexer.token != Token::End &&
            lexer.token != Token::Error {
            match lexer.token {
                Token::Var => {
                    ret.push(self.parse_var_decl(lexer)?);
                },
                Token::Return => {
                    ret.push(self.parse_return(lexer)?);
                },
                Token::If => {
                    ret.push(self.parse_if(lexer)?);
                },
                Token::Continue => {
                    ret.push(self.parse_continue(lexer)?);
                },
                Token::Break => {
                    ret.push(self.parse_break(lexer)?);
                },
                Token::While => {
                    ret.push(self.parse_while(lexer)?);
                },
                Token::Loop => {
                    ret.push(self.parse_loop(lexer)?);
                },
                _ => {
                    let expr = self.parse_expr(lexer, &[Token::Semicolon])?;
                    // Swallow ";"
                    lexer.advance();
                    ret.push(Statement::Expression(expr));
                }
            };
            
        }

        Ok(ret)
    }

    pub fn try_parse_call_stmt(&self, lexer: &mut Lexer) -> ParseResult<Statement> {
        let delims = [
            Token::Semicolon,
            Token::End,
            Token::Error
        ];

        let lexer_backup = lexer.clone();

        let mut full_fn_name = String::new();
        let mut last_bit = String::new();

        while !delims.contains(&lexer.token) {
            if lexer.token != Token::Text {
                break;
            }
            full_fn_name += lexer.slice();
            last_bit = String::from(lexer.slice());
            lexer.advance();
            if last_bit.len() == 1 && lexer.token == Token::Text {
                full_fn_name += lexer.slice();
                last_bit += lexer.slice();
                lexer.advance();
            }
            if lexer.token != Token::DoubleColon {
                break;
            }
            full_fn_name += lexer.slice();
            last_bit = String::from(lexer.slice());
            lexer.advance();
        }

        //println!("Trying to parse call stmt to function {}", full_fn_name);

        if &last_bit == "::" {
            *lexer = lexer_backup;
            //println!("ERROR! Trailing \"::\"");
            return Err(ParseError::new(ParseErrorType::UnsupportedExpression, lexer.range()));
        }

        if lexer.token != Token::OpenParan {
            *lexer = lexer_backup;
            //println!("ERROR! No \"(\"");
            return Err(ParseError::new(ParseErrorType::UnsupportedExpression, lexer.range()));
        }

        lexer.advance();

        let mut params = Vec::new();

        while lexer.token != Token::CloseParan &&
            lexer.token != Token::End &&
            lexer.token != Token::Error {
            let arg_res = self.parse_expr(lexer, &[
                Token::Comma,
                Token::CloseParan
            ]);
            if arg_res.is_err() {
                //println!("Error when parsing fn arg");
                *lexer = lexer_backup;
                return Err(ParseError::new(ParseErrorType::UnsupportedExpression, lexer.range()));
            }
            if lexer.token == Token::Comma {
                lexer.advance(); // Swallow "," if its there
            }
            params.push(arg_res.unwrap());
        }

        // Swallow ")"
        lexer.advance();

        if lexer.token != Token::Semicolon {
            *lexer = lexer_backup;
            return Err(ParseError::new(ParseErrorType::ExpectedSemicolon, lexer.range()));
        }

        // Swallow ";"
        lexer.advance();

        Ok(
            Statement::Call(full_fn_name, params)
        )
    }

    pub fn parse_break(&self, lexer: &mut Lexer) -> ParseResult<Statement> {
        if lexer.token != Token::Break {
            return Err(ParseError::new(ParseErrorType::UnknownStatement, lexer.range()));
        }

        // Swallow "break"
        lexer.advance();

        if lexer.token != Token::Semicolon {
            return Err(ParseError::new(ParseErrorType::ExpectedSemicolon, lexer.range()));
        }

        // Swallow ";"
        lexer.advance();

        Ok(
            Statement::Break
        )
    }

    pub fn parse_continue(&self, lexer: &mut Lexer) -> ParseResult<Statement> {
        if lexer.token != Token::Continue {
            return Err(ParseError::new(ParseErrorType::UnknownStatement, lexer.range()));
        }

        // Swallow "continue"
        lexer.advance();

        if lexer.token != Token::Semicolon {
            return Err(ParseError::new(ParseErrorType::ExpectedSemicolon, lexer.range()));
        }

        // Swallow ";"
        lexer.advance();

        Ok(
            Statement::Continue
        )
    }

    pub fn parse_return(&self, lexer: &mut Lexer) -> ParseResult<Statement> {
        // Swallow "return"
        lexer.advance();

        let ret_expr = self.parse_expr(lexer, &[Token::Semicolon])?;

        // Swallow ";"
        lexer.advance();

        Ok(
            Statement::Return(Some(ret_expr))
        )
    }

    pub fn parse_var_decl(&self, lexer: &mut Lexer) -> ParseResult<Statement> {
        let mut lexer_backup = lexer.clone();

        // Swallow "var"
        lexer.advance();
        
        if lexer.token != Token::Text {
            *lexer = lexer_backup;
            return Err(ParseError::new(ParseErrorType::ExpectedVarName, lexer.range()));
        }

        let mut var_name = String::from(lexer.slice());

        // swallow var name
        lexer.advance();
        
        let mut var_type = Type::Auto;

        // if type is specified
        if lexer.token == Token::Colon {
            // Swallow ":"
            lexer.advance();

            var_type = self.parse_type(lexer)?;
        }

        if lexer.token != Token::Assign {
            *lexer = lexer_backup;
            return Err(ParseError::new(ParseErrorType::ExpectedAssignment, lexer.range()));
        }

        lexer.advance();

        let expr = self.parse_expr(lexer, &[Token::Semicolon])?;

        ////println!("Decl assignment expr: {:?}", expr);

        let var_decl_args = VariableDeclArgs {
            var_type: var_type,
            name: var_name,
            assignment: Box::new(expr)
        };

        lexer.advance();

        Ok(
            Statement::VariableDecl(var_decl_args)
        )
    }

    pub fn parse_var_assign(&self, lexer: &mut Lexer) -> ParseResult<Statement> {
        if lexer.token != Token::Text {
            return Err(ParseError::new(ParseErrorType::UnknownStatement, lexer.range()));
        }

        let var_name = String::from(lexer.slice());
        lexer.advance();

        if lexer.token != Token::Assign {
            return Err(ParseError::new(ParseErrorType::ExpectedAssignment, lexer.range()));
        }

        lexer.advance();

        let assign_expr = self.parse_expr(lexer, &[Token::Semicolon])?;

        lexer.advance();

        Ok(
            Statement::Assignment(var_name, Box::new(assign_expr))
        )
    }

    pub fn parse_fn_call_stmt(&self, lexer: &mut Lexer) -> ParseResult<Statement> {
        if lexer.token != Token::Text {
            return Err(ParseError::new(ParseErrorType::ExpectedFunctionName, lexer.range()));
        }

        let fn_name = String::from(lexer.slice());
        // Swallow fn name
        lexer.advance();

        if lexer.token != Token::OpenParan {
            return Err(ParseError::new(ParseErrorType::ExpectedOpenParan, lexer.range()));
        }

        // Swallow "("
        lexer.advance();

        let mut params = Vec::new();

        while lexer.token != Token::CloseParan &&
            lexer.token != Token::End &&
            lexer.token != Token::Error {
            let arg = self.parse_expr(lexer, &[
                Token::Comma,
                Token::CloseParan
            ])?;
            if lexer.token == Token::Comma {
                lexer.advance(); // Swallow "," if its there
            }
            params.push(arg);
        }

        // Swallow ")"
        lexer.advance();

        if lexer.token != Token::Semicolon {
            return Err(ParseError::new(ParseErrorType::ExpectedSemicolon, lexer.range()));
        }
        // Swallow ";"
        lexer.advance();

        Ok(
            Statement::Call(fn_name, params)
        )
    }

    pub fn parse_expr_push(&self, lexer: &mut Lexer, operand_stack: &mut VecDeque<Expression>, operator_stack: &mut VecDeque<Token>) -> ParseResult<Expression> {
        //println!("parse_expr_push(): operator stack len {}", operator_stack.len());
        //println!("parse_expr_push(): operand stack len {}", operand_stack.len());
        let op = operator_stack.pop_front().unwrap();
        //println!("parse_expr_push(): operator {:?}", op);
        //println!("parse_expr_push() start");
        let expr = match op {
            Token::Plus => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::Addition(Box::new(lhs), Box::new(rhs))
            },
            Token::Minus => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::Subtraction(Box::new(lhs), Box::new(rhs))
            },
            Token::Times => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::Multiplication(Box::new(lhs), Box::new(rhs))
            },
            Token::Divide => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::Division(Box::new(lhs), Box::new(rhs))
            },
            Token::Equals => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::Equals(Box::new(lhs), Box::new(rhs))
            },
            Token::NotEquals => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::NotEquals(Box::new(lhs), Box::new(rhs))
            },
            Token::GreaterThan => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::GreaterThan(Box::new(lhs), Box::new(rhs))
            },
            Token::GreaterThanEquals => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::GreaterThanEquals(Box::new(lhs), Box::new(rhs))
            },
            Token::LessThan => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::LessThan(Box::new(lhs), Box::new(rhs))
            },
            Token::LessThanEquals => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::LessThanEquals(Box::new(lhs), Box::new(rhs))
            },
            Token::Not => {
                let op = operand_stack.pop_front().unwrap();
                Expression::Not(Box::new(op))
            },
            Token::Tilde => {
                let op = operand_stack.pop_front().unwrap();
                Expression::Deref(Box::new(op))
            },
            Token::And => {
                let op = operand_stack.pop_front().unwrap();
                Expression::Ref(Box::new(op))
            },
            Token::Dot => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::MemberAccess(Box::new(lhs), Box::new(rhs))
            },
            Token::Assign => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::Assign(Box::new(lhs), Box::new(rhs))
            },
            Token::AddAssign => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::AddAssign(Box::new(lhs), Box::new(rhs))
            },
            Token::SubAssign => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::SubAssign(Box::new(lhs), Box::new(rhs))
            },
            Token::MulAssign => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::MulAssign(Box::new(lhs), Box::new(rhs))
            },
            Token::DivAssign => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::DivAssign(Box::new(lhs), Box::new(rhs))
            },
            Token::DoubleAnd => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::And(Box::new(lhs), Box::new(rhs))
            },
            Token::Or => {
                let rhs = operand_stack.pop_front().unwrap();
                let lhs = operand_stack.pop_front().unwrap();
                Expression::Or(Box::new(lhs), Box::new(rhs))
            },
            _ => {
                return Err(ParseError::new(ParseErrorType::UnsupportedExpression, lexer.range()));
            }
        };

        //println!("parse_expr_push() end");
        Ok(expr)
    }

    pub fn parse_mod_path(&self, lexer: &mut Lexer) -> ParseResult<String> {
        let mut name = String::new();
        while lexer.token == Token::Text ||
            lexer.token == Token::DoubleColon {
            name += lexer.slice();
            lexer.advance();
        }

        if name.ends_with("::") {
            return make_parse_error!(lexer, ParseErrorType::ExpectedContainerName);
        }

        Ok(name)
    }

    pub fn try_parse_cont_instance(&self, lexer: &mut Lexer) -> ParseResult<Expression> {
        let lexer_backup = lexer.clone();
        
        let cont_name = self.parse_mod_path(lexer)?;

        if lexer.token != Token::OpenBlock {
            *lexer = lexer_backup;
            return make_parse_error!(lexer, ParseErrorType::ExpectedOpenBlock);
        }

        // Swallow "{"
        lexer.advance();

        let instance_map_res = self.parse_cont_instance_content(lexer);
        if instance_map_res.is_err() {
            *lexer = lexer_backup;
            return make_parse_error!(lexer, ParseErrorType::ExpectedMemberName);
        }
        let instance_map = instance_map_res.unwrap();

        if lexer.token != Token::CloseBlock {
            *lexer = lexer_backup;
            return make_parse_error!(lexer, ParseErrorType::ExpectedCloseBlock);
        }

        // Swallow "}"
        lexer.advance();

        Ok(
            Expression::ContainerInstance(cont_name, instance_map)
        )
    }

    fn parse_cont_instance_content(&self, lexer: &mut Lexer) -> ParseResult<HashMap<String, Expression>> {
        let delims = &[
            Token::CloseBlock,
            Token::End,
            Token::Error
        ];

        let mut ret = HashMap::new();

        while !delims.contains(&lexer.token) {
            if lexer.token != Token::Text {
                return make_parse_error!(lexer, ParseErrorType::ExpectedMemberName);
            }

            let member_name = String::from(lexer.slice());
            // Swallow member name
            lexer.advance();

            if lexer.token != Token::Colon {
                return make_parse_error!(lexer, ParseErrorType::ExpectedColon);
            }

            // Swallow ":"
            lexer.advance();

            let member_expr = self.parse_expr(lexer, &[Token::Comma, Token::CloseBlock])?;

            if lexer.token == Token::Comma {
                lexer.advance();
            }

            ret.insert(member_name, member_expr);
        }

        Ok(ret)
    }

    pub fn try_parse_call_expr(&self, lexer: &mut Lexer) -> ParseResult<Expression> {
        let lexer_backup = lexer.clone(); // Create lexer backup for backtracking

        let full_fn_name = self.parse_mod_path(lexer)?;

        if full_fn_name.is_empty() {
            return Err(ParseError::new(ParseErrorType::ExpectedFunctionName, lexer.range()));
        }

        if lexer.token != Token::OpenParan {
            *lexer = lexer_backup;
            return Err(ParseError::new(ParseErrorType::ExpectedOpenParan, lexer.range()));
        }

        // Swallow "("
        lexer.advance();

        let mut params = Vec::new();

        while lexer.token != Token::CloseParan &&
            lexer.token != Token::End &&
            lexer.token != Token::Error {
            let arg = self.parse_expr(lexer, &[
                Token::Comma,
                Token::CloseParan
            ])?;
            if lexer.token == Token::Comma {
                lexer.advance(); // Swallow "," if its there
            }
            params.push(arg);
        }

        // Swallow ")"
        lexer.advance();

        Ok(
            Expression::Call(full_fn_name, params)
        )
    }

    pub fn parse_expr(&self, lexer: &mut Lexer, delims: &[Token]) -> ParseResult<Expression> {
        let mut operator_stack = VecDeque::new();
        let mut operand_stack = VecDeque::new();

        // Counter for handling ")" being used as delim
        let mut open_paran_count = 0;
        let mut dec_paran_count = false;

        while lexer.token != Token::End &&
            lexer.token != Token::Error {

            // If Token is delimiter
            if delims.contains(&lexer.token) {
                // Special case if ")" is a delimiter
                if lexer.token == Token::CloseParan && open_paran_count == 0 {
                    break;
                } else if lexer.token != Token::CloseParan {
                    break; // Break if delim is hit
                }
            }

            if lexer.token == Token::True {
                let expr = Expression::BoolLiteral(true);
                operand_stack.push_front(expr);
            }

            if lexer.token == Token::False {
                let expr = Expression::BoolLiteral(false);
                operand_stack.push_front(expr);
            }
            
            if lexer.token == Token::Text {
                let expr;
                let call_expr_res = self.try_parse_call_expr(lexer);
                if call_expr_res.is_ok() {
                    expr = call_expr_res.unwrap();
                } else {
                    let cont_inst_expr_res = self.try_parse_cont_instance(lexer);
                    if cont_inst_expr_res.is_ok() {
                        expr = cont_inst_expr_res.unwrap();
                    } else {
                        let mut var_name = String::from(lexer.slice());
                        expr = Expression::Variable(var_name);
                    }
                }
                operand_stack.push_front(expr);
            }

            if lexer.token == Token::IntLiteral {
                let int = String::from(lexer.slice()).parse::<i64>()
                    .map_err(|_| ParseError::new(ParseErrorType::Unknown, lexer.range()))?;
                let expr = Expression::IntLiteral(int);
                operand_stack.push_front(expr);
            }

            if lexer.token == Token::FloatLiteral {
                let float = String::from(lexer.slice()).parse::<f32>()
                    .map_err(|_| ParseError::new(ParseErrorType::Unknown, lexer.range()))?;
                let expr = Expression::FloatLiteral(float);
                operand_stack.push_front(expr);
            }

            if lexer.token == Token::StringLiteral {
                let string = String::from(lexer.slice());
                //println!("Parsing string literal {}", string);
                let expr = Expression::StringLiteral(string);
                operand_stack.push_front(expr);
            }

            if is_op(&lexer.token) {
                loop {
                    let op_opt = operator_stack.get(0);
                    if op_opt.is_none() {
                        break; // Break if operator stack is empty
                    }
                    let op = op_opt.unwrap();
                    if *op == Token::OpenParan {
                        break; // Break if operator is a "("
                    }

                    if !(op_prec(&lexer.token) - op_prec(op) < 0) &&
                        !(op_prec(&lexer.token) == op_prec(op) && !is_op_right_assoc(op)) {
                        break; // Break if there is no operator of greater precedence on the stack or of equal precedence and right assoc
                    }

                    let expr = self.parse_expr_push(lexer, &mut operand_stack, &mut operator_stack)?;
                    operand_stack.push_front(expr);
                }
                operator_stack.push_front(lexer.token.clone());
            }

            if lexer.token == Token::OpenParan {
                operator_stack.push_front(lexer.token.clone());
                open_paran_count += 1;
            }

            if lexer.token == Token::CloseParan {
                let mut pop = false;               
                while operator_stack.len() > 0 {
                    {
                        let op_ref = operator_stack.get(0).unwrap();
                        if *op_ref == Token::OpenParan {
                            dec_paran_count = true;
                            pop = true;
                            break;
                        }
                    }
                    let expr = self.parse_expr_push(lexer, &mut operand_stack, &mut operator_stack)?;
                    operand_stack.push_front(expr);
                }

                if pop {
                    operator_stack.pop_front();
                }
            }

            // If Token is delimiter
            if delims.contains(&lexer.token) {
                // Special case if ")" is a delimiter
                if lexer.token == Token::CloseParan && open_paran_count == 0 {
                    break;
                } else if lexer.token != Token::CloseParan {
                    break; // Break if delim is hit
                }
            }

            // Workaround for properly decrementing open_paran_count
            if dec_paran_count {
                dec_paran_count = false;
                open_paran_count -= 1;
            }
            
            lexer.advance();
        }

        while operator_stack.len() > 0 {
            let expr = self.parse_expr_push(lexer, &mut operand_stack, &mut operator_stack)?;
            operand_stack.push_front(expr);
        }

        //println!("Operator stack: {:?}", operator_stack);
        //println!("Operand stack: {:?}", operand_stack);

        operand_stack.pop_front()
            .ok_or(ParseError::new(ParseErrorType::UnsupportedExpression, lexer.range()))
    }
}
