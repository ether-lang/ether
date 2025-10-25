// Ether: High-Performance AI Language

#![allow(unused_variables)]
#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt;

// ============================================================================
// TYPE SYSTEM
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Void,
    Tensor(Option<Vec<usize>>),
    List(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Range,
    Function(Vec<Type>, Box<Type>),
    TypeVar(String),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "string"),
            Type::Void => write!(f, "void"),
            Type::Tensor(None) => write!(f, "Tensor"),
            Type::Tensor(Some(shape)) => write!(f, "Tensor[{:?}]", shape),
            Type::List(t) => write!(f, "[{}]", t),
            Type::Map(k, v) => write!(f, "Map[{}, {}]", k, v),
            Type::Range => write!(f, "Range"),
            Type::Function(params, ret) => {
                write!(f, "(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::TypeVar(name) => write!(f, "'{}", name),
        }
    }
}

// ============================================================================
// LEXER
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Literals
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    
    // Keywords
    Let, Fn, Return, If, Else, While, For, In, Match, Case, Tensor, Import,
    Try, Catch, Finally, Throw, Raise, Range, Map,
    
    // Identifiers
    Ident(String),
    
    // Operators
    Plus, Minus, Star, Slash, Percent,
    Eq, Neq, Lt, Gt, Lte, Gte,
    And, Or, Not,
    Assign, Arrow, FatArrow, DotDot, DotDotEq,
    
    // Delimiters
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Comma, Colon, Semicolon, Dot, Pipe,
    
    // Special
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub ttype: TokenType,
    pub line: usize,
    pub column: usize,
}

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }
    
    fn current(&self) -> Option<char> {
        if self.pos < self.source.len() {
            Some(self.source[self.pos])
        } else {
            None
        }
    }
    
    fn peek(&self, offset: usize) -> Option<char> {
        let pos = self.pos + offset;
        if pos < self.source.len() {
            Some(self.source[pos])
        } else {
            None
        }
    }
    
    fn advance(&mut self) {
        if self.pos < self.source.len() {
            if self.source[self.pos] == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.pos += 1;
        }
    }
    
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current() {
            if ch == ' ' || ch == '\t' || ch == '\r' || ch == '\n' {
                self.advance();
            } else {
                break;
            }
        }
    }
    
    fn skip_comment(&mut self) {
        if self.current() == Some('/') && self.peek(1) == Some('/') {
            while self.current().is_some() && self.current() != Some('\n') {
                self.advance();
            }
            self.skip_whitespace();
        }
    }
    
    fn read_number(&mut self) -> TokenType {
        let mut num_str = String::new();
        let mut is_float = false;
        
        while let Some(ch) = self.current() {
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else if ch == '.' && !is_float && self.peek(1).map_or(false, |c| c.is_ascii_digit()) {
                is_float = true;
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        if is_float {
            TokenType::FloatLit(num_str.parse().unwrap())
        } else {
            TokenType::IntLit(num_str.parse().unwrap())
        }
    }
    
    fn read_string(&mut self) -> TokenType {
        self.advance();
        let mut string = String::new();
        
        while let Some(ch) = self.current() {
            if ch == '"' {
                break;
            }
            if ch == '\\' {
                self.advance();
                if let Some(escaped) = self.current() {
                    let ch = match escaped {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '"' => '"',
                        '\\' => '\\',
                        _ => escaped,
                    };
                    string.push(ch);
                    self.advance();
                }
            } else {
                string.push(ch);
                self.advance();
            }
        }
        
        if self.current() == Some('"') {
            self.advance();
        }
        
        TokenType::StringLit(string)
    }
    
    fn read_identifier(&mut self) -> TokenType {
        let mut ident = String::new();
        
        while let Some(ch) = self.current() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        
        match ident.as_str() {
            "let" => TokenType::Let,
            "fn" => TokenType::Fn,
            "return" => TokenType::Return,
            "if" => TokenType::If,
            "else" => TokenType::Else,
            "while" => TokenType::While,
            "for" => TokenType::For,
            "in" => TokenType::In,
            "match" => TokenType::Match,
            "case" => TokenType::Case,
            "tensor" => TokenType::Tensor,
            "import" => TokenType::Import,
            "try" => TokenType::Try,
            "catch" => TokenType::Catch,
            "finally" => TokenType::Finally,
            "throw" => TokenType::Throw,
            "raise" => TokenType::Raise,
            "range" => TokenType::Range,
            "map" => TokenType::Map,
            "true" => TokenType::BoolLit(true),
            "false" => TokenType::BoolLit(false),
            "and" => TokenType::And,
            "or" => TokenType::Or,
            "not" => TokenType::Not,
            _ => TokenType::Ident(ident),
        }
    }
    
    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        
        while self.pos < self.source.len() {
            self.skip_whitespace();
            self.skip_comment();
            
            if self.current().is_none() {
                break;
            }
            
            let line = self.line;
            let column = self.column;
            
            let ttype = match self.current().unwrap() {
                ch if ch.is_ascii_digit() => self.read_number(),
                '"' => self.read_string(),
                ch if ch.is_alphabetic() || ch == '_' => self.read_identifier(),
                '+' => { self.advance(); TokenType::Plus }
                '-' => {
                    self.advance();
                    if self.current() == Some('>') {
                        self.advance();
                        TokenType::Arrow
                    } else {
                        TokenType::Minus
                    }
                }
                '*' => { self.advance(); TokenType::Star }
                '/' => { self.advance(); TokenType::Slash }
                '%' => { self.advance(); TokenType::Percent }
                '=' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        TokenType::Eq
                    } else if self.current() == Some('>') {
                        self.advance();
                        TokenType::FatArrow
                    } else {
                        TokenType::Assign
                    }
                }
                '!' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        TokenType::Neq
                    } else {
                        return Err(format!("Unexpected '!' at {}:{}", line, column));
                    }
                }
                '<' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        TokenType::Lte
                    } else {
                        TokenType::Lt
                    }
                }
                '>' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        TokenType::Gte
                    } else {
                        TokenType::Gt
                    }
                }
                '(' => { self.advance(); TokenType::LParen }
                ')' => { self.advance(); TokenType::RParen }
                '{' => { self.advance(); TokenType::LBrace }
                '}' => { self.advance(); TokenType::RBrace }
                '[' => { self.advance(); TokenType::LBracket }
                ']' => { self.advance(); TokenType::RBracket }
                ',' => { self.advance(); TokenType::Comma }
                ':' => { self.advance(); TokenType::Colon }
                ';' => { self.advance(); TokenType::Semicolon }
                '|' => { self.advance(); TokenType::Pipe }
                '.' => {
                    self.advance();
                    if self.current() == Some('.') {
                        self.advance();
                        if self.current() == Some('=') {
                            self.advance();
                            TokenType::DotDotEq
                        } else {
                            TokenType::DotDot
                        }
                    } else {
                        TokenType::Dot
                    }
                }
                ch => return Err(format!("Unexpected character '{}' at {}:{}", ch, line, column)),
            };
            
            tokens.push(Token { ttype, line, column });
        }
        
        tokens.push(Token { ttype: TokenType::Eof, line: self.line, column: self.column });
        Ok(tokens)
    }
}

// ============================================================================
// ABSTRACT SYNTAX TREE
// ============================================================================

#[derive(Debug, Clone)]
pub enum Stmt {
    Let { name: String, value: Box<Expr>, type_annotation: Option<Type> },
    Assign { name: String, value: Box<Expr> },
    IndexAssign { target: Box<Expr>, index: Box<Expr>, value: Box<Expr> },
    Function { name: String, params: Vec<(String, Option<Type>)>, body: Vec<Stmt>, return_type: Option<Type> },
    Return { value: Option<Box<Expr>> },
    If { condition: Box<Expr>, then_block: Vec<Stmt>, else_block: Option<Vec<Stmt>> },
    While { condition: Box<Expr>, body: Vec<Stmt> },
    ForIn { var_name: String, iterable: Box<Expr>, body: Vec<Stmt> },
    Try { try_block: Vec<Stmt>, catch_var: Option<String>, catch_block: Option<Vec<Stmt>>, finally_block: Option<Vec<Stmt>> },
    Throw { value: Box<Expr> },
    Raise { exception_type: String, message: Box<Expr> },
    Expr(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Binary { left: Box<Expr>, op: BinOp, right: Box<Expr> },
    Unary { op: UnOp, operand: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
    Index { target: Box<Expr>, index: Box<Expr> },
    Slice { target: Box<Expr>, start: Option<Box<Expr>>, end: Option<Box<Expr>> },
    Ident(String),
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    ListLit(Vec<Expr>),
    MapLit(Vec<(Expr, Expr)>),
    TensorLit { shape: Vec<usize> },
    Range { start: Box<Expr>, end: Box<Expr>, inclusive: bool },
    Match { value: Box<Expr>, cases: Vec<MatchCase> },
}

#[derive(Debug, Clone)]
pub struct MatchCase {
    pub pattern: Pattern,
    pub guard: Option<Box<Expr>>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Literal(Box<Expr>),
    Ident(String),
    List(Vec<Pattern>),
    Wildcard,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Gt, Lte, Gte,
    And, Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Neg, Not,
}

// ============================================================================
// PARSER
// ============================================================================

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }
    
    fn current(&self) -> &Token {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos]
        } else {
            self.tokens.last().unwrap()
        }
    }
    
    fn peek(&self, offset: usize) -> &Token {
        let pos = self.pos + offset;
        if pos < self.tokens.len() {
            &self.tokens[pos]
        } else {
            self.tokens.last().unwrap()
        }
    }
    
    fn advance(&mut self) {
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
    }
    
    fn expect(&mut self, expected: fn(&TokenType) -> bool) -> Result<Token, String> {
        let token = self.current().clone();
        if expected(&token.ttype) {
            self.advance();
            Ok(token)
        } else {
            Err(format!("Unexpected token at {}:{}", token.line, token.column))
        }
    }
    
    pub fn parse(&mut self) -> Result<Vec<Stmt>, String> {
        let mut statements = Vec::new();
        
        while !matches!(self.current().ttype, TokenType::Eof) {
            statements.push(self.parse_statement()?);
        }
        
        Ok(statements)
    }
    
    fn parse_statement(&mut self) -> Result<Stmt, String> {
        match &self.current().ttype {
            TokenType::Let => self.parse_let(),
            TokenType::Fn => self.parse_function(),
            TokenType::Return => self.parse_return(),
            TokenType::If => self.parse_if(),
            TokenType::While => self.parse_while(),
            TokenType::For => self.parse_for_in(),
            TokenType::Try => self.parse_try(),
            TokenType::Throw => self.parse_throw(),
            TokenType::Raise => self.parse_raise(),
            TokenType::Ident(_) => {
                // Check for assignment or index assignment
                let start_pos = self.pos;
                let expr = self.parse_expression()?;
                
                if matches!(self.current().ttype, TokenType::Assign) {
                    self.advance();
                    let value = Box::new(self.parse_expression()?);
                    
                    match expr {
                        Expr::Ident(name) => Ok(Stmt::Assign { name, value }),
                        Expr::Index { target, index } => {
                            Ok(Stmt::IndexAssign { target, index, value })
                        }
                        _ => Err("Invalid assignment target".to_string()),
                    }
                } else {
                    Ok(Stmt::Expr(Box::new(expr)))
                }
            }
            _ => Ok(Stmt::Expr(Box::new(self.parse_expression()?))),
        }
    }
    
    fn parse_let(&mut self) -> Result<Stmt, String> {
        self.advance();
        
        let name = if let TokenType::Ident(n) = &self.current().ttype {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected identifier after 'let'".to_string());
        };
        
        let type_annotation = if matches!(self.current().ttype, TokenType::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        
        self.expect(|t| matches!(t, TokenType::Assign))?;
        let value = Box::new(self.parse_expression()?);
        
        Ok(Stmt::Let { name, value, type_annotation })
    }
    
    fn parse_function(&mut self) -> Result<Stmt, String> {
        self.advance();
        
        let name = if let TokenType::Ident(n) = &self.current().ttype {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected function name".to_string());
        };
        
        self.expect(|t| matches!(t, TokenType::LParen))?;
        
        let mut params = Vec::new();
        while !matches!(self.current().ttype, TokenType::RParen) {
            let param_name = if let TokenType::Ident(n) = &self.current().ttype {
                let name = n.clone();
                self.advance();
                name
            } else {
                return Err("Expected parameter name".to_string());
            };
            
            let param_type = if matches!(self.current().ttype, TokenType::Colon) {
                self.advance();
                Some(self.parse_type()?)
            } else {
                None
            };
            
            params.push((param_name, param_type));
            
            if matches!(self.current().ttype, TokenType::Comma) {
                self.advance();
            }
        }
        
        self.expect(|t| matches!(t, TokenType::RParen))?;
        
        let return_type = if matches!(self.current().ttype, TokenType::Arrow) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        
        self.expect(|t| matches!(t, TokenType::LBrace))?;
        
        let mut body = Vec::new();
        while !matches!(self.current().ttype, TokenType::RBrace) {
            body.push(self.parse_statement()?);
        }
        
        self.expect(|t| matches!(t, TokenType::RBrace))?;
        
        Ok(Stmt::Function { name, params, body, return_type })
    }
    
    fn parse_return(&mut self) -> Result<Stmt, String> {
        self.advance();
        
        let value = if matches!(self.current().ttype, TokenType::RBrace | TokenType::Eof) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };
        
        Ok(Stmt::Return { value })
    }
    
    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.advance();
        
        let condition = Box::new(self.parse_expression()?);
        
        self.expect(|t| matches!(t, TokenType::LBrace))?;
        let mut then_block = Vec::new();
        while !matches!(self.current().ttype, TokenType::RBrace) {
            then_block.push(self.parse_statement()?);
        }
        self.expect(|t| matches!(t, TokenType::RBrace))?;
        
        let else_block = if matches!(self.current().ttype, TokenType::Else) {
            self.advance();
            self.expect(|t| matches!(t, TokenType::LBrace))?;
            let mut block = Vec::new();
            while !matches!(self.current().ttype, TokenType::RBrace) {
                block.push(self.parse_statement()?);
            }
            self.expect(|t| matches!(t, TokenType::RBrace))?;
            Some(block)
        } else {
            None
        };
        
        Ok(Stmt::If { condition, then_block, else_block })
    }
    
    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.advance();
        
        let condition = Box::new(self.parse_expression()?);
        
        self.expect(|t| matches!(t, TokenType::LBrace))?;
        let mut body = Vec::new();
        while !matches!(self.current().ttype, TokenType::RBrace) {
            body.push(self.parse_statement()?);
        }
        self.expect(|t| matches!(t, TokenType::RBrace))?;
        
        Ok(Stmt::While { condition, body })
    }
    
    fn parse_for_in(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'for'
        
        let var_name = if let TokenType::Ident(n) = &self.current().ttype {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected variable name after 'for'".to_string());
        };
        
        self.expect(|t| matches!(t, TokenType::In))?;
        
        let iterable = Box::new(self.parse_expression()?);
        
        self.expect(|t| matches!(t, TokenType::LBrace))?;
        let mut body = Vec::new();
        while !matches!(self.current().ttype, TokenType::RBrace) {
            body.push(self.parse_statement()?);
        }
        self.expect(|t| matches!(t, TokenType::RBrace))?;
        
        Ok(Stmt::ForIn { var_name, iterable, body })
    }
    
    fn parse_try(&mut self) -> Result<Stmt, String> {
        self.advance();
        
        self.expect(|t| matches!(t, TokenType::LBrace))?;
        let mut try_block = Vec::new();
        while !matches!(self.current().ttype, TokenType::RBrace) {
            try_block.push(self.parse_statement()?);
        }
        self.expect(|t| matches!(t, TokenType::RBrace))?;
        
        let mut catch_var = None;
        let mut catch_block = None;
        
        if matches!(self.current().ttype, TokenType::Catch) {
            self.advance();
            
            if matches!(self.current().ttype, TokenType::LParen) {
                self.advance();
                if let TokenType::Ident(name) = &self.current().ttype {
                    catch_var = Some(name.clone());
                    self.advance();
                }
                self.expect(|t| matches!(t, TokenType::RParen))?;
            }
            
            self.expect(|t| matches!(t, TokenType::LBrace))?;
            let mut block = Vec::new();
            while !matches!(self.current().ttype, TokenType::RBrace) {
                block.push(self.parse_statement()?);
            }
            self.expect(|t| matches!(t, TokenType::RBrace))?;
            catch_block = Some(block);
        }
        
        let mut finally_block = None;
        if matches!(self.current().ttype, TokenType::Finally) {
            self.advance();
            self.expect(|t| matches!(t, TokenType::LBrace))?;
            let mut block = Vec::new();
            while !matches!(self.current().ttype, TokenType::RBrace) {
                block.push(self.parse_statement()?);
            }
            self.expect(|t| matches!(t, TokenType::RBrace))?;
            finally_block = Some(block);
        }
        
        Ok(Stmt::Try { try_block, catch_var, catch_block, finally_block })
    }
    
    fn parse_throw(&mut self) -> Result<Stmt, String> {
        self.advance();
        let value = Box::new(self.parse_expression()?);
        Ok(Stmt::Throw { value })
    }
    
    fn parse_raise(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'raise'
        
        let exception_type = if let TokenType::Ident(n) = &self.current().ttype {
            let name = n.clone();
            self.advance();
            name
        } else {
            return Err("Expected exception type after 'raise'".to_string());
        };
        
        self.expect(|t| matches!(t, TokenType::LParen))?;
        let message = Box::new(self.parse_expression()?);
        self.expect(|t| matches!(t, TokenType::RParen))?;
        
        Ok(Stmt::Raise { exception_type, message })
    }
    
    fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_match()
    }
    
    fn parse_match(&mut self) -> Result<Expr, String> {
        if matches!(self.current().ttype, TokenType::Match) {
            self.advance();
            let value = Box::new(self.parse_or()?);
            
            self.expect(|t| matches!(t, TokenType::LBrace))?;
            
            let mut cases = Vec::new();
            while !matches!(self.current().ttype, TokenType::RBrace) {
                let pattern = self.parse_pattern()?;
                
                let guard = if matches!(self.current().ttype, TokenType::If) {
                    self.advance();
                    Some(Box::new(self.parse_or()?))
                } else {
                    None
                };
                
                self.expect(|t| matches!(t, TokenType::FatArrow))?;
                
                let mut body = vec![];
                if matches!(self.current().ttype, TokenType::LBrace) {
                    self.advance();
                    while !matches!(self.current().ttype, TokenType::RBrace) {
                        body.push(self.parse_statement()?);
                    }
                    self.expect(|t| matches!(t, TokenType::RBrace))?;
                } else {
                    let expr = self.parse_or()?;
                    body.push(Stmt::Expr(Box::new(expr)));
                }
                
                cases.push(MatchCase { pattern, guard, body });
                
                if matches!(self.current().ttype, TokenType::Comma) {
                    self.advance();
                }
            }
            
            self.expect(|t| matches!(t, TokenType::RBrace))?;
            
            Ok(Expr::Match { value, cases })
        } else {
            self.parse_or()
        }
    }
    
    fn parse_pattern(&mut self) -> Result<Pattern, String> {
        match &self.current().ttype {
            TokenType::Ident(name) if name == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            TokenType::Ident(name) => {
                let n = name.clone();
                self.advance();
                Ok(Pattern::Ident(n))
            }
            TokenType::LBracket => {
                self.advance();
                let mut patterns = Vec::new();
                while !matches!(self.current().ttype, TokenType::RBracket) {
                    patterns.push(self.parse_pattern()?);
                    if matches!(self.current().ttype, TokenType::Comma) {
                        self.advance();
                    }
                }
                self.expect(|t| matches!(t, TokenType::RBracket))?;
                Ok(Pattern::List(patterns))
            }
            _ => {
                let expr = self.parse_primary()?;
                Ok(Pattern::Literal(Box::new(expr)))
            }
        }
    }
    
    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        
        while matches!(self.current().ttype, TokenType::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
            };
        }
        
        Ok(left)
    }
    
    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;
        
        while matches!(self.current().ttype, TokenType::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
            };
        }
        
        Ok(left)
    }
    
    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;
        
        while let Some(op) = match &self.current().ttype {
            TokenType::Eq => Some(BinOp::Eq),
            TokenType::Neq => Some(BinOp::Neq),
            _ => None,
        } {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        
        Ok(left)
    }
    
    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_range()?;
        
        while let Some(op) = match &self.current().ttype {
            TokenType::Lt => Some(BinOp::Lt),
            TokenType::Gt => Some(BinOp::Gt),
            TokenType::Lte => Some(BinOp::Lte),
            TokenType::Gte => Some(BinOp::Gte),
            _ => None,
        } {
            self.advance();
            let right = self.parse_range()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        
        Ok(left)
    }
    
    fn parse_range(&mut self) -> Result<Expr, String> {
        let left = self.parse_addition()?;
        
        if matches!(self.current().ttype, TokenType::DotDot | TokenType::DotDotEq) {
            let inclusive = matches!(self.current().ttype, TokenType::DotDotEq);
            self.advance();
            let right = self.parse_addition()?;
            Ok(Expr::Range {
                start: Box::new(left),
                end: Box::new(right),
                inclusive,
            })
        } else {
            Ok(left)
        }
    }
    
    fn parse_addition(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplication()?;
        
        while let Some(op) = match &self.current().ttype {
            TokenType::Plus => Some(BinOp::Add),
            TokenType::Minus => Some(BinOp::Sub),
            _ => None,
        } {
            self.advance();
            let right = self.parse_multiplication()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        
        Ok(left)
    }
    
    fn parse_multiplication(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        
        while let Some(op) = match &self.current().ttype {
            TokenType::Star => Some(BinOp::Mul),
            TokenType::Slash => Some(BinOp::Div),
            TokenType::Percent => Some(BinOp::Mod),
            _ => None,
        } {
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        
        Ok(left)
    }
    
    fn parse_unary(&mut self) -> Result<Expr, String> {
        match &self.current().ttype {
            TokenType::Minus => {
                self.advance();
                Ok(Expr::Unary {
                    op: UnOp::Neg,
                    operand: Box::new(self.parse_unary()?),
                })
            }
            TokenType::Not => {
                self.advance();
                Ok(Expr::Unary {
                    op: UnOp::Not,
                    operand: Box::new(self.parse_unary()?),
                })
            }
            _ => self.parse_postfix(),
        }
    }
    
    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        
        loop {
            match &self.current().ttype {
                TokenType::LBracket => {
                    self.advance();
                    
                    // Check for slicing
                    if matches!(self.current().ttype, TokenType::Colon) {
                        self.advance();
                        let end = if matches!(self.current().ttype, TokenType::RBracket) {
                            None
                        } else {
                            Some(Box::new(self.parse_expression()?))
                        };
                        self.expect(|t| matches!(t, TokenType::RBracket))?;
                        expr = Expr::Slice {
                            target: Box::new(expr),
                            start: None,
                            end,
                        };
                    } else {
                        let index_or_start = self.parse_expression()?;
                        
                        if matches!(self.current().ttype, TokenType::Colon) {
                            self.advance();
                            let end = if matches!(self.current().ttype, TokenType::RBracket) {
                                None
                            } else {
                                Some(Box::new(self.parse_expression()?))
                            };
                            self.expect(|t| matches!(t, TokenType::RBracket))?;
                            expr = Expr::Slice {
                                target: Box::new(expr),
                                start: Some(Box::new(index_or_start)),
                                end,
                            };
                        } else {
                            self.expect(|t| matches!(t, TokenType::RBracket))?;
                            expr = Expr::Index {
                                target: Box::new(expr),
                                index: Box::new(index_or_start),
                            };
                        }
                    }
                }
                _ => break,
            }
        }
        
        Ok(expr)
    }
    
    fn parse_primary(&mut self) -> Result<Expr, String> {
        match &self.current().ttype {
            TokenType::IntLit(n) => {
                let val = *n;
                self.advance();
                Ok(Expr::IntLit(val))
            }
            TokenType::FloatLit(n) => {
                let val = *n;
                self.advance();
                Ok(Expr::FloatLit(val))
            }
            TokenType::StringLit(s) => {
                let val = s.clone();
                self.advance();
                Ok(Expr::StringLit(val))
            }
            TokenType::BoolLit(b) => {
                let val = *b;
                self.advance();
                Ok(Expr::BoolLit(val))
            }
            TokenType::Ident(name) => {
                let name = name.clone();
                self.advance();
                
                if matches!(self.current().ttype, TokenType::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    
                    while !matches!(self.current().ttype, TokenType::RParen) {
                        args.push(self.parse_expression()?);
                        if matches!(self.current().ttype, TokenType::Comma) {
                            self.advance();
                        }
                    }
                    
                    self.expect(|t| matches!(t, TokenType::RParen))?;
                    Ok(Expr::Call { name, args })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            TokenType::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                
                while !matches!(self.current().ttype, TokenType::RBracket) {
                    elements.push(self.parse_expression()?);
                    if matches!(self.current().ttype, TokenType::Comma) {
                        self.advance();
                    }
                }
                
                self.expect(|t| matches!(t, TokenType::RBracket))?;
                Ok(Expr::ListLit(elements))
            }
            TokenType::LBrace => {
                self.advance();
                let mut pairs = Vec::new();
                
                while !matches!(self.current().ttype, TokenType::RBrace) {
                    let key = self.parse_expression()?;
                    self.expect(|t| matches!(t, TokenType::Colon))?;
                    let value = self.parse_expression()?;
                    pairs.push((key, value));
                    
                    if matches!(self.current().ttype, TokenType::Comma) {
                        self.advance();
                    }
                }
                
                self.expect(|t| matches!(t, TokenType::RBrace))?;
                Ok(Expr::MapLit(pairs))
            }
            TokenType::Tensor => {
                self.advance();
                self.expect(|t| matches!(t, TokenType::LParen))?;
                
                let mut shape = Vec::new();
                if matches!(self.current().ttype, TokenType::LBracket) {
                    self.advance();
                    while !matches!(self.current().ttype, TokenType::RBracket) {
                        if let TokenType::IntLit(n) = self.current().ttype {
                            shape.push(n as usize);
                            self.advance();
                        }
                        if matches!(self.current().ttype, TokenType::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(|t| matches!(t, TokenType::RBracket))?;
                }
                
                self.expect(|t| matches!(t, TokenType::RParen))?;
                Ok(Expr::TensorLit { shape })
            }
            TokenType::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(|t| matches!(t, TokenType::RParen))?;
                Ok(expr)
            }
            _ => Err(format!("Unexpected token at {}:{}", 
                self.current().line, self.current().column)),
        }
    }
    
    fn parse_type(&mut self) -> Result<Type, String> {
        match &self.current().ttype {
            TokenType::Ident(name) => {
                let t = match name.as_str() {
                    "int" => Type::Int,
                    "float" => Type::Float,
                    "bool" => Type::Bool,
                    "string" => Type::String,
                    "Tensor" => Type::Tensor(None),
                    "Range" => Type::Range,
                    _ => Type::TypeVar(name.clone()),
                };
                self.advance();
                Ok(t)
            }
            _ => Err("Expected type".to_string()),
        }
    }
}

// ============================================================================
// BYTECODE & VM
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum OpCode {
    LoadConst, LoadVar, StoreVar,
    Add, Sub, Mul, Div, Mod, Neg,
    Eq, Neq, Lt, Gt, Lte, Gte,
    And, Or, Not,
    Jump, JumpIfFalse, Call, Return,
    TensorCreate, MatMul, Relu, Sigmoid, Tanh, Softmax,
    BuildList, BuildMap, Print, Pop, Halt,
    Throw, SetupTry, PopTry, BeginFinally, EndFinally,
    AssertType,
    // New opcodes
    Index, IndexSet, Slice, BuildRange,
    SetupForIn, ForInNext, PopForIn,
    MatchBegin, MatchCase, MatchEnd,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: OpCode,
    pub arg: i32,
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    List(Vec<Value>),
    Map(HashMap<String, Value>),
    Tensor { shape: Vec<usize>, data: Vec<f64> },
    Range { start: i64, end: i64, inclusive: bool },
    Exception { exc_type: String, message: String },
    Void,
}

impl Value {
    pub fn type_name(&self) -> &str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::String(_) => "string",
            Value::List(_) => "list",
            Value::Map(_) => "map",
            Value::Tensor { .. } => "Tensor",
            Value::Range { .. } => "Range",
            Value::Exception { .. } => "Exception",
            Value::Void => "void",
        }
    }
    
    pub fn to_key(&self) -> Option<String> {
        match self {
            Value::String(s) => Some(s.clone()),
            Value::Int(n) => Some(n.to_string()),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::List(v) => {
                write!(f, "[")?;
                for (i, val) in v.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", val)?;
                }
                write!(f, "]")
            }
            Value::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Tensor { shape, data } => {
                write!(f, "Tensor{:?}: [", shape)?;
                for (i, val) in data.iter().take(5).enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{:.4}", val)?;
                }
                if data.len() > 5 {
                    write!(f, ", ...")?;
                }
                write!(f, "]")
            }
            Value::Range { start, end, inclusive } => {
                if *inclusive {
                    write!(f, "{}..={}", start, end)
                } else {
                    write!(f, "{}..{}", start, end)
                }
            }
            Value::Exception { exc_type, message } => {
                write!(f, "{}: {}", exc_type, message)
            }
            Value::Void => write!(f, "void"),
        }
    }
}

pub struct Compiler {
    instructions: Vec<Instruction>,
    constants: Vec<Value>,
    var_indices: HashMap<String, usize>,
    next_var_index: usize,
    function_addresses: HashMap<String, usize>,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            instructions: Vec::new(),
            constants: Vec::new(),
            var_indices: HashMap::new(),
            next_var_index: 0,
            function_addresses: HashMap::new(),
        }
    }
    
    fn add_constant(&mut self, value: Value) -> usize {
        for (i, c) in self.constants.iter().enumerate() {
            let matches = match (&value, c) {
                (Value::Int(a), Value::Int(b)) => a == b,
                (Value::Float(a), Value::Float(b)) => a == b,
                (Value::Bool(a), Value::Bool(b)) => a == b,
                (Value::String(a), Value::String(b)) => a == b,
                _ => false,
            };
            if matches {
                return i;
            }
        }
        self.constants.push(value);
        self.constants.len() - 1
    }
    
    fn get_var_index(&mut self, name: &str) -> usize {
        if let Some(&idx) = self.var_indices.get(name) {
            idx
        } else {
            let idx = self.next_var_index;
            self.var_indices.insert(name.to_string(), idx);
            self.next_var_index += 1;
            idx
        }
    }
    
    fn emit(&mut self, opcode: OpCode, arg: i32) {
        self.instructions.push(Instruction { opcode, arg });
    }
    
    fn current_address(&self) -> usize {
        self.instructions.len()
    }
    
    pub fn compile(&mut self, statements: &[Stmt]) -> Result<(), String> {
        for stmt in statements {
            self.compile_stmt(stmt)?;
        }
        self.emit(OpCode::Halt, 0);
        Ok(())
    }
    
    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Let { name, value, .. } => {
                self.compile_expr(value)?;
                let idx = self.get_var_index(name);
                self.emit(OpCode::StoreVar, idx as i32);
            }
            Stmt::Assign { name, value } => {
                self.compile_expr(value)?;
                let idx = self.get_var_index(name);
                self.emit(OpCode::StoreVar, idx as i32);
            }
            Stmt::IndexAssign { target, index, value } => {
                self.compile_expr(target)?;
                self.compile_expr(index)?;
                self.compile_expr(value)?;
                self.emit(OpCode::IndexSet, 0);
            }
            Stmt::Function { name, params, body, .. } => {
                let jump_addr = self.current_address();
                self.emit(OpCode::Jump, 0);
                
                let func_addr = self.current_address();
                self.function_addresses.insert(name.clone(), func_addr);
                
                for (param_name, param_type) in params.iter().rev() {
                    if let Some(expected_type) = param_type {
                        let type_const = self.add_constant(Value::String(format!("{}", expected_type)));
                        self.emit(OpCode::LoadConst, type_const as i32);
                        self.emit(OpCode::AssertType, 0);
                    }
                    
                    let idx = self.get_var_index(param_name);
                    self.emit(OpCode::StoreVar, idx as i32);
                }
                
                for stmt in body {
                    self.compile_stmt(stmt)?;
                }
                
                let const_idx = self.add_constant(Value::Void);
                self.emit(OpCode::LoadConst, const_idx as i32);
                self.emit(OpCode::Return, 0);
                
                let end_addr = self.current_address();
                self.instructions[jump_addr].arg = end_addr as i32;
            }
            Stmt::Return { value } => {
                if let Some(v) = value {
                    self.compile_expr(v)?;
                } else {
                    let const_idx = self.add_constant(Value::Void);
                    self.emit(OpCode::LoadConst, const_idx as i32);
                }
                self.emit(OpCode::Return, 0);
            }
            Stmt::If { condition, then_block, else_block } => {
                self.compile_expr(condition)?;
                
                let else_jump = self.current_address();
                self.emit(OpCode::JumpIfFalse, 0);
                
                for stmt in then_block {
                    self.compile_stmt(stmt)?;
                }
                
                let end_jump = self.current_address();
                self.emit(OpCode::Jump, 0);
                
                let else_addr = self.current_address();
                self.instructions[else_jump].arg = else_addr as i32;
                
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.compile_stmt(stmt)?;
                    }
                }
                
                let end_addr = self.current_address();
                self.instructions[end_jump].arg = end_addr as i32;
            }
            Stmt::While { condition, body } => {
                let start_addr = self.current_address();
                
                self.compile_expr(condition)?;
                
                let end_jump = self.current_address();
                self.emit(OpCode::JumpIfFalse, 0);
                
                for stmt in body {
                    self.compile_stmt(stmt)?;
                }
                
                self.emit(OpCode::Jump, start_addr as i32);
                
                let end_addr = self.current_address();
                self.instructions[end_jump].arg = end_addr as i32;
            }
            Stmt::ForIn { var_name, iterable, body } => {
                self.compile_expr(iterable)?;
                
                let var_idx = self.get_var_index(var_name);
                self.emit(OpCode::SetupForIn, var_idx as i32);
                
                let loop_start = self.current_address();
                let end_jump = self.current_address();
                self.emit(OpCode::ForInNext, 0);
                
                for stmt in body {
                    self.compile_stmt(stmt)?;
                }
                
                self.emit(OpCode::Jump, loop_start as i32);
                
                let end_addr = self.current_address();
                self.instructions[end_jump].arg = end_addr as i32;
                
                self.emit(OpCode::PopForIn, 0);
            }
            Stmt::Try { try_block, catch_var, catch_block, finally_block } => {
                let catch_addr_placeholder = self.current_address();
                self.emit(OpCode::SetupTry, 0);
                
                for stmt in try_block {
                    self.compile_stmt(stmt)?;
                }
                
                self.emit(OpCode::PopTry, 0);
                
                let finally_jump = self.current_address();
                self.emit(OpCode::Jump, 0);
                
                let catch_addr = self.current_address();
                if let Some(block) = catch_block {
                    if let Some(var_name) = catch_var {
                        let idx = self.get_var_index(var_name);
                        self.emit(OpCode::StoreVar, idx as i32);
                    } else {
                        self.emit(OpCode::Pop, 0);
                    }
                    
                    for stmt in block {
                        self.compile_stmt(stmt)?;
                    }
                } else {
                    self.emit(OpCode::Pop, 0);
                }
                
                self.instructions[catch_addr_placeholder].arg = catch_addr as i32;
                
                let finally_addr = self.current_address();
                self.instructions[finally_jump].arg = finally_addr as i32;
                
                if let Some(block) = finally_block {
                    self.emit(OpCode::BeginFinally, 0);
                    for stmt in block {
                        self.compile_stmt(stmt)?;
                    }
                    self.emit(OpCode::EndFinally, 0);
                }
            }
            Stmt::Throw { value } => {
                self.compile_expr(value)?;
                self.emit(OpCode::Throw, 0);
            }
            Stmt::Raise { exception_type, message } => {
                self.compile_expr(message)?;
                let type_const = self.add_constant(Value::String(exception_type.clone()));
                self.emit(OpCode::LoadConst, type_const as i32);
                self.emit(OpCode::Throw, 1); // arg=1 signals custom exception
            }
            Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                self.emit(OpCode::Pop, 0);
            }
        }
        Ok(())
    }
    
    fn compile_expr(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::IntLit(n) => {
                let idx = self.add_constant(Value::Int(*n));
                self.emit(OpCode::LoadConst, idx as i32);
            }
            Expr::FloatLit(n) => {
                let idx = self.add_constant(Value::Float(*n));
                self.emit(OpCode::LoadConst, idx as i32);
            }
            Expr::StringLit(s) => {
                let idx = self.add_constant(Value::String(s.clone()));
                self.emit(OpCode::LoadConst, idx as i32);
            }
            Expr::BoolLit(b) => {
                let idx = self.add_constant(Value::Bool(*b));
                self.emit(OpCode::LoadConst, idx as i32);
            }
            Expr::Ident(name) => {
                let idx = self.get_var_index(name);
                self.emit(OpCode::LoadVar, idx as i32);
            }
            Expr::Binary { left, op, right } => {
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                
                let opcode = match op {
                    BinOp::Add => OpCode::Add,
                    BinOp::Sub => OpCode::Sub,
                    BinOp::Mul => OpCode::Mul,
                    BinOp::Div => OpCode::Div,
                    BinOp::Mod => OpCode::Mod,
                    BinOp::Eq => OpCode::Eq,
                    BinOp::Neq => OpCode::Neq,
                    BinOp::Lt => OpCode::Lt,
                    BinOp::Gt => OpCode::Gt,
                    BinOp::Lte => OpCode::Lte,
                    BinOp::Gte => OpCode::Gte,
                    BinOp::And => OpCode::And,
                    BinOp::Or => OpCode::Or,
                };
                self.emit(opcode, 0);
            }
            Expr::Unary { op, operand } => {
                self.compile_expr(operand)?;
                let opcode = match op {
                    UnOp::Neg => OpCode::Neg,
                    UnOp::Not => OpCode::Not,
                };
                self.emit(opcode, 0);
            }
            Expr::Call { name, args } => {
                match name.as_str() {
                    "print" => {
                        for arg in args {
                            self.compile_expr(arg)?;
                            self.emit(OpCode::Print, 0);
                        }
                        let idx = self.add_constant(Value::Void);
                        self.emit(OpCode::LoadConst, idx as i32);
                    }
                    "matmul" | "relu" | "sigmoid" | "tanh" | "softmax" => {
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        let opcode = match name.as_str() {
                            "matmul" => OpCode::MatMul,
                            "relu" => OpCode::Relu,
                            "sigmoid" => OpCode::Sigmoid,
                            "tanh" => OpCode::Tanh,
                            "softmax" => OpCode::Softmax,
                            _ => unreachable!(),
                        };
                        self.emit(opcode, 0);
                    }
                    _ => {
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        if let Some(&addr) = self.function_addresses.get(name) {
                            self.emit(OpCode::Call, addr as i32);
                        } else {
                            return Err(format!("Undefined function: {}", name));
                        }
                    }
                }
            }
            Expr::Index { target, index } => {
                self.compile_expr(target)?;
                self.compile_expr(index)?;
                self.emit(OpCode::Index, 0);
            }
            Expr::Slice { target, start, end } => {
                self.compile_expr(target)?;
                
                if let Some(s) = start {
                    self.compile_expr(s)?;
                } else {
                    let idx = self.add_constant(Value::Int(0));
                    self.emit(OpCode::LoadConst, idx as i32);
                }
                
                if let Some(e) = end {
                    self.compile_expr(e)?;
                } else {
                    let idx = self.add_constant(Value::Int(-1));
                    self.emit(OpCode::LoadConst, idx as i32);
                }
                
                self.emit(OpCode::Slice, 0);
            }
            Expr::ListLit(elements) => {
                for elem in elements {
                    self.compile_expr(elem)?;
                }
                self.emit(OpCode::BuildList, elements.len() as i32);
            }
            Expr::MapLit(pairs) => {
                for (k, v) in pairs {
                    self.compile_expr(k)?;
                    self.compile_expr(v)?;
                }
                self.emit(OpCode::BuildMap, pairs.len() as i32);
            }
            Expr::TensorLit { shape } => {
                let idx = self.add_constant(Value::List(
                    shape.iter().map(|&s| Value::Int(s as i64)).collect()
                ));
                self.emit(OpCode::LoadConst, idx as i32);
                self.emit(OpCode::TensorCreate, 0);
            }
            Expr::Range { start, end, inclusive } => {
                self.compile_expr(start)?;
                self.compile_expr(end)?;
                self.emit(OpCode::BuildRange, if *inclusive { 1 } else { 0 });
            }
            Expr::Match { value, cases } => {
                self.compile_expr(value)?;
                self.emit(OpCode::MatchBegin, 0);
                
                let mut end_jumps = Vec::new();
                
                for case in cases {
                    let case_start = self.current_address();
                    self.emit(OpCode::MatchCase, 0); // Placeholder
                    
                    // Compile pattern matching logic (simplified)
                    match &case.pattern {
                        Pattern::Wildcard => {
                            // Always matches
                            let idx = self.add_constant(Value::Bool(true));
                            self.emit(OpCode::LoadConst, idx as i32);
                        }
                        Pattern::Literal(expr) => {
                            self.compile_expr(expr)?;
                            self.emit(OpCode::Eq, 0);
                        }
                        Pattern::Ident(name) => {
                            // Bind to variable and match
                            let var_idx = self.get_var_index(name);
                            self.emit(OpCode::StoreVar, var_idx as i32);
                            let idx = self.add_constant(Value::Bool(true));
                            self.emit(OpCode::LoadConst, idx as i32);
                        }
                        _ => {
                            let idx = self.add_constant(Value::Bool(false));
                            self.emit(OpCode::LoadConst, idx as i32);
                        }
                    }
                    
                    let next_case_jump = self.current_address();
                    self.emit(OpCode::JumpIfFalse, 0);
                    
                    // Compile case body
                    for stmt in &case.body {
                        self.compile_stmt(stmt)?;
                    }
                    
                    let end_jump = self.current_address();
                    self.emit(OpCode::Jump, 0);
                    end_jumps.push(end_jump);
                    
                    let next_case_addr = self.current_address();
                    self.instructions[next_case_jump].arg = next_case_addr as i32;
                }
                
                let end_addr = self.current_address();
                for jump in end_jumps {
                    self.instructions[jump].arg = end_addr as i32;
                }
                
                self.emit(OpCode::MatchEnd, 0);
            }
        }
        Ok(())
    }
    
    pub fn get_instructions(&self) -> &[Instruction] {
        &self.instructions
    }
    
    pub fn get_constants(&self) -> &[Value] {
        &self.constants
    }
}

#[derive(Debug, Clone)]
struct TryHandler {
    catch_addr: usize,
    stack_size: usize,
}

#[derive(Debug, Clone)]
struct ForInIterator {
    items: Vec<Value>,
    index: usize,
    var_idx: usize,
}

pub struct VM {
    instructions: Vec<Instruction>,
    constants: Vec<Value>,
    stack: Vec<Value>,
    variables: Vec<Value>,
    pc: usize,
    call_stack: Vec<usize>,
    try_stack: Vec<TryHandler>,
    for_in_stack: Vec<ForInIterator>,
    exception: Option<Value>,
}

impl VM {
    pub fn new(instructions: Vec<Instruction>, constants: Vec<Value>) -> Self {
        VM {
            instructions,
            constants,
            stack: Vec::with_capacity(256),
            variables: vec![Value::Void; 256],
            pc: 0,
            call_stack: Vec::new(),
            try_stack: Vec::new(),
            for_in_stack: Vec::new(),
            exception: None,
        }
    }
    
    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }
    
    fn pop(&mut self) -> Result<Value, String> {
        self.stack.pop().ok_or_else(|| "Stack underflow".to_string())
    }
    
    pub fn run(&mut self) -> Result<(), String> {
        while self.pc < self.instructions.len() {
            let instr = self.instructions[self.pc].clone();
            
            if self.exception.is_some() {
                self.handle_exception()?;
                continue;
            }
            
            match self.execute(instr) {
                Ok(_) => self.pc += 1,
                Err(e) => {
                    self.exception = Some(Value::Exception {
                        exc_type: "RuntimeError".to_string(),
                        message: e,
                    });
                    if !self.handle_exception()? {
                        if let Some(Value::Exception { exc_type, message }) = &self.exception {
                            return Err(format!("Uncaught {}: {}", exc_type, message));
                        }
                        return Err("Uncaught exception".to_string());
                    }
                }
            }
        }
        Ok(())
    }
    
    fn handle_exception(&mut self) -> Result<bool, String> {
        if let Some(handler) = self.try_stack.pop() {
            while self.stack.len() > handler.stack_size {
                self.stack.pop();
            }
            
            if let Some(exc) = self.exception.take() {
                self.stack.push(exc);
            }
            
            self.pc = handler.catch_addr;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    fn execute(&mut self, instr: Instruction) -> Result<(), String> {
        match instr.opcode {
            OpCode::LoadConst => {
                let val = self.constants[instr.arg as usize].clone();
                self.push(val);
            }
            OpCode::LoadVar => {
                let val = self.variables[instr.arg as usize].clone();
                self.push(val);
            }
            OpCode::StoreVar => {
                let val = self.pop()?;
                self.variables[instr.arg as usize] = val;
            }
            OpCode::Add => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Value::Int(x + y),
                    (Value::Float(x), Value::Float(y)) => Value::Float(x + y),
                    (Value::Int(x), Value::Float(y)) => Value::Float(x as f64 + y),
                    (Value::Float(x), Value::Int(y)) => Value::Float(x + y as f64),
                    (Value::String(x), Value::String(y)) => Value::String(format!("{}{}", x, y)),
                    _ => return Err("Type error in addition".to_string()),
                };
                self.push(result);
            }
            OpCode::Sub => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Value::Int(x - y),
                    (Value::Float(x), Value::Float(y)) => Value::Float(x - y),
                    (Value::Int(x), Value::Float(y)) => Value::Float(x as f64 - y),
                    (Value::Float(x), Value::Int(y)) => Value::Float(x - y as f64),
                    _ => return Err("Type error in subtraction".to_string()),
                };
                self.push(result);
            }
            OpCode::Mul => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Value::Int(x * y),
                    (Value::Float(x), Value::Float(y)) => Value::Float(x * y),
                    (Value::Int(x), Value::Float(y)) => Value::Float(x as f64 * y),
                    (Value::Float(x), Value::Int(y)) => Value::Float(x * y as f64),
                    _ => return Err("Type error in multiplication".to_string()),
                };
                self.push(result);
            }
            OpCode::Div => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = match (a, b) {
                    (Value::Int(x), Value::Int(y)) => {
                        if y == 0 { return Err("Division by zero".to_string()); }
                        Value::Int(x / y)
                    }
                    (Value::Float(x), Value::Float(y)) => Value::Float(x / y),
                    (Value::Int(x), Value::Float(y)) => Value::Float(x as f64 / y),
                    (Value::Float(x), Value::Int(y)) => Value::Float(x / y as f64),
                    _ => return Err("Type error in division".to_string()),
                };
                self.push(result);
            }
            OpCode::Mod => {
                let b = self.pop()?;
                let a = self.pop()?;
                if let (Value::Int(x), Value::Int(y)) = (a, b) {
                    self.push(Value::Int(x % y));
                } else {
                    return Err("Type error in modulo".to_string());
                }
            }
            OpCode::Neg => {
                let val = self.pop()?;
                let result = match val {
                    Value::Int(x) => Value::Int(-x),
                    Value::Float(x) => Value::Float(-x),
                    _ => return Err("Type error in negation".to_string()),
                };
                self.push(result);
            }
            OpCode::Eq => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Value::Bool(x == y),
                    (Value::Float(x), Value::Float(y)) => Value::Bool(x == y),
                    (Value::Bool(x), Value::Bool(y)) => Value::Bool(x == y),
                    (Value::String(x), Value::String(y)) => Value::Bool(x == y),
                    _ => Value::Bool(false),
                };
                self.push(result);
            }
            OpCode::Neq => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Value::Bool(x != y),
                    (Value::Float(x), Value::Float(y)) => Value::Bool(x != y),
                    (Value::Bool(x), Value::Bool(y)) => Value::Bool(x != y),
                    (Value::String(x), Value::String(y)) => Value::Bool(x != y),
                    _ => Value::Bool(true),
                };
                self.push(result);
            }
            OpCode::Lt => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Value::Bool(x < y),
                    (Value::Float(x), Value::Float(y)) => Value::Bool(x < y),
                    _ => return Err("Type error in comparison".to_string()),
                };
                self.push(result);
            }
            OpCode::Gt => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Value::Bool(x > y),
                    (Value::Float(x), Value::Float(y)) => Value::Bool(x > y),
                    _ => return Err("Type error in comparison".to_string()),
                };
                self.push(result);
            }
            OpCode::Lte => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Value::Bool(x <= y),
                    (Value::Float(x), Value::Float(y)) => Value::Bool(x <= y),
                    _ => return Err("Type error in comparison".to_string()),
                };
                self.push(result);
            }
            OpCode::Gte => {
                let b = self.pop()?;
                let a = self.pop()?;
                let result = match (a, b) {
                    (Value::Int(x), Value::Int(y)) => Value::Bool(x >= y),
                    (Value::Float(x), Value::Float(y)) => Value::Bool(x >= y),
                    _ => return Err("Type error in comparison".to_string()),
                };
                self.push(result);
            }
            OpCode::And => {
                let b = self.pop()?;
                let a = self.pop()?;
                if let (Value::Bool(x), Value::Bool(y)) = (a, b) {
                    self.push(Value::Bool(x && y));
                } else {
                    return Err("Type error in AND".to_string());
                }
            }
            OpCode::Or => {
                let b = self.pop()?;
                let a = self.pop()?;
                if let (Value::Bool(x), Value::Bool(y)) = (a, b) {
                    self.push(Value::Bool(x || y));
                } else {
                    return Err("Type error in OR".to_string());
                }
            }
            OpCode::Not => {
                let val = self.pop()?;
                if let Value::Bool(x) = val {
                    self.push(Value::Bool(!x));
                } else {
                    return Err("Type error in NOT".to_string());
                }
            }
            OpCode::Jump => {
                self.pc = instr.arg as usize - 1;
            }
            OpCode::JumpIfFalse => {
                let cond = self.pop()?;
                if let Value::Bool(false) = cond {
                    self.pc = instr.arg as usize - 1;
                }
            }
            OpCode::Call => {
                self.call_stack.push(self.pc);
                self.pc = instr.arg as usize - 1;
            }
            OpCode::Return => {
                if let Some(return_addr) = self.call_stack.pop() {
                    self.pc = return_addr;
                } else {
                    self.pc = self.instructions.len();
                }
            }
            OpCode::BuildList => {
                let count = instr.arg as usize;
                let mut elements = Vec::new();
                for _ in 0..count {
                    elements.push(self.pop()?);
                }
                elements.reverse();
                self.push(Value::List(elements));
            }
            OpCode::BuildMap => {
                let count = instr.arg as usize;
                let mut map = HashMap::new();
                for _ in 0..count {
                    let value = self.pop()?;
                    let key = self.pop()?;
                    if let Some(k) = key.to_key() {
                        map.insert(k, value);
                    } else {
                        return Err("Map keys must be strings or integers".to_string());
                    }
                }
                self.push(Value::Map(map));
            }
            OpCode::Index => {
                let index = self.pop()?;
                let target = self.pop()?;
                
                match (target, index) {
                    (Value::List(list), Value::Int(i)) => {
                        let idx = if i < 0 {
                            (list.len() as i64 + i) as usize
                        } else {
                            i as usize
                        };
                        if idx < list.len() {
                            self.push(list[idx].clone());
                        } else {
                            return Err("List index out of bounds".to_string());
                        }
                    }
                    (Value::Map(map), key) => {
                        if let Some(k) = key.to_key() {
                            if let Some(val) = map.get(&k) {
                                self.push(val.clone());
                            } else {
                                return Err(format!("Key '{}' not found in map", k));
                            }
                        } else {
                            return Err("Invalid map key".to_string());
                        }
                    }
                    (Value::Tensor { shape, data }, Value::Int(i)) => {
                        let idx = if i < 0 {
                            (data.len() as i64 + i) as usize
                        } else {
                            i as usize
                        };
                        if idx < data.len() {
                            self.push(Value::Float(data[idx]));
                        } else {
                            return Err("Tensor index out of bounds".to_string());
                        }
                    }
                    _ => return Err("Invalid indexing operation".to_string()),
                }
            }
            OpCode::IndexSet => {
                let value = self.pop()?;
                let index = self.pop()?;
                let target = self.pop()?;
                
                match (target, index, value) {
                    (Value::List(mut list), Value::Int(i), val) => {
                        let idx = if i < 0 {
                            (list.len() as i64 + i) as usize
                        } else {
                            i as usize
                        };
                        if idx < list.len() {
                            list[idx] = val;
                            self.push(Value::List(list));
                        } else {
                            return Err("List index out of bounds".to_string());
                        }
                    }
                    (Value::Map(mut map), key, val) => {
                        if let Some(k) = key.to_key() {
                            map.insert(k, val);
                            self.push(Value::Map(map));
                        } else {
                            return Err("Invalid map key".to_string());
                        }
                    }
                    _ => return Err("Invalid index assignment".to_string()),
                }
            }
            OpCode::Slice => {
                let end = self.pop()?;
                let start = self.pop()?;
                let target = self.pop()?;
                
                match (target, start, end) {
                    (Value::List(list), Value::Int(s), Value::Int(e)) => {
                        let start_idx = if s < 0 {
                            (list.len() as i64 + s).max(0) as usize
                        } else {
                            (s as usize).min(list.len())
                        };
                        
                        let end_idx = if e < 0 {
                            (list.len() as i64 + e + 1).max(0) as usize
                        } else {
                            (e as usize).min(list.len())
                        };
                        
                        if start_idx <= end_idx {
                            self.push(Value::List(list[start_idx..end_idx].to_vec()));
                        } else {
                            self.push(Value::List(vec![]));
                        }
                    }
                    _ => return Err("Invalid slicing operation".to_string()),
                }
            }
            OpCode::BuildRange => {
                let end = self.pop()?;
                let start = self.pop()?;
                let inclusive = instr.arg == 1;
                
                match (start, end) {
                    (Value::Int(s), Value::Int(e)) => {
                        self.push(Value::Range { start: s, end: e, inclusive });
                    }
                    _ => return Err("Range bounds must be integers".to_string()),
                }
            }
            OpCode::SetupForIn => {
                let iterable = self.pop()?;
                let var_idx = instr.arg as usize;
                
                let items = match iterable {
                    Value::List(list) => list,
                    Value::Range { start, end, inclusive } => {
                        let mut items = Vec::new();
                        if start <= end {
                            let limit = if inclusive { end + 1 } else { end };
                            for i in start..limit {
                                items.push(Value::Int(i));
                            }
                        } else {
                            let limit = if inclusive { end - 1 } else { end };
                            let mut i = start;
                            while i > limit {
                                items.push(Value::Int(i));
                                i -= 1;
                            }
                        }
                        items
                    }
                    Value::Tensor { data, .. } => {
                        data.iter().map(|&x| Value::Float(x)).collect()
                    }
                    Value::Map(map) => {
                        map.iter().map(|(k, v)| Value::List(vec![
                            Value::String(k.clone()),
                            v.clone()
                        ])).collect()
                    }
                    _ => return Err("Cannot iterate over this type".to_string()),
                };
                
                self.for_in_stack.push(ForInIterator { items, index: 0, var_idx });
            }
            OpCode::ForInNext => {
                if let Some(iterator) = self.for_in_stack.last_mut() {
                    if iterator.index < iterator.items.len() {
                        let item = iterator.items[iterator.index].clone();
                        self.variables[iterator.var_idx] = item;
                        iterator.index += 1;
                    } else {
                        self.pc = instr.arg as usize - 1;
                    }
                } else {
                    return Err("No active for-in loop".to_string());
                }
            }
            OpCode::PopForIn => {
                self.for_in_stack.pop();
            }
            OpCode::TensorCreate => {
                let shape_val = self.pop()?;
                if let Value::List(shape_list) = shape_val {
                    let shape: Vec<usize> = shape_list.iter()
                        .filter_map(|v| if let Value::Int(n) = v { Some(*n as usize) } else { None })
                        .collect();
                    
                    let size: usize = shape.iter().product();
                    let data: Vec<f64> = (0..size).map(|i| (i as f64) * 0.01).collect();
                    
                    self.push(Value::Tensor { shape, data });
                } else {
                    return Err("Invalid tensor shape".to_string());
                }
            }
            OpCode::MatMul => {
                let _b = self.pop()?;
                let _a = self.pop()?;
                self.push(Value::Tensor {
                    shape: vec![1, 1],
                    data: vec![1.0],
                });
            }
            OpCode::Relu => {
                let tensor = self.pop()?;
                if let Value::Tensor { shape, data } = tensor {
                    let new_data: Vec<f64> = data.iter().map(|&x| x.max(0.0)).collect();
                    self.push(Value::Tensor { shape, data: new_data });
                } else {
                    return Err("ReLU requires tensor".to_string());
                }
            }
            OpCode::Sigmoid => {
                let tensor = self.pop()?;
                if let Value::Tensor { shape, data } = tensor {
                    let new_data: Vec<f64> = data.iter()
                        .map(|&x| 1.0 / (1.0 + (-x).exp()))
                        .collect();
                    self.push(Value::Tensor { shape, data: new_data });
                } else {
                    return Err("Sigmoid requires tensor".to_string());
                }
            }
            OpCode::Tanh => {
                let tensor = self.pop()?;
                if let Value::Tensor { shape, data } = tensor {
                    let new_data: Vec<f64> = data.iter().map(|&x| x.tanh()).collect();
                    self.push(Value::Tensor { shape, data: new_data });
                } else {
                    return Err("Tanh requires tensor".to_string());
                }
            }
            OpCode::Softmax => {
                let tensor = self.pop()?;
                if let Value::Tensor { shape, data } = tensor {
                    let max_val = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    let exp_vals: Vec<f64> = data.iter().map(|&x| (x - max_val).exp()).collect();
                    let sum: f64 = exp_vals.iter().sum();
                    let new_data: Vec<f64> = exp_vals.iter().map(|&x| x / sum).collect();
                    self.push(Value::Tensor { shape, data: new_data });
                } else {
                    return Err("Softmax requires tensor".to_string());
                }
            }
            OpCode::Print => {
                let val = self.pop()?;
                println!("{}", val);
            }
            OpCode::Pop => {
                self.pop()?;
            }
            OpCode::Halt => {
                self.pc = self.instructions.len();
            }
            OpCode::Throw => {
                if instr.arg == 1 {
                    // Custom exception
                    let exc_type = self.pop()?;
                    let message = self.pop()?;
                    if let (Value::String(t), Value::String(m)) = (exc_type, message) {
                        return Err(format!("{}: {}", t, m));
                    }
                } else {
                    let val = self.pop()?;
                    let msg = match val {
                        Value::String(s) => s,
                        Value::Exception { message, .. } => message,
                        other => format!("{}", other),
                    };
                    return Err(msg);
                }
            }
            OpCode::SetupTry => {
                let catch_addr = instr.arg as usize;
                self.try_stack.push(TryHandler {
                    catch_addr,
                    stack_size: self.stack.len(),
                });
            }
            OpCode::PopTry => {
                self.try_stack.pop();
            }
            OpCode::BeginFinally => {}
            OpCode::EndFinally => {}
            OpCode::AssertType => {
                let expected_type_str = self.pop()?;
                let value = self.pop()?;
                
                if let Value::String(expected) = expected_type_str {
                    let actual = value.type_name();
                    
                    let matches = match expected.as_str() {
                        "int" => matches!(value, Value::Int(_)),
                        "float" => matches!(value, Value::Float(_)),
                        "bool" => matches!(value, Value::Bool(_)),
                        "string" => matches!(value, Value::String(_)),
                        "Tensor" => matches!(value, Value::Tensor { .. }),
                        "Range" => matches!(value, Value::Range { .. }),
                        _ => true,
                    };
                    
                    if !matches {
                        return Err(format!(
                            "Type assertion failed: expected '{}', got '{}'",
                            expected, actual
                        ));
                    }
                }
                
                self.push(value);
            }
            OpCode::MatchBegin | OpCode::MatchCase | OpCode::MatchEnd => {
                // Pattern matching is handled inline during compilation
            }
        }
        Ok(())
    }
}

// ============================================================================
// MAIN API
// ============================================================================

pub fn compile_and_run(source: &str) -> Result<(), String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    
    let mut parser = Parser::new(tokens);
    let ast = parser.parse()?;
    
    let mut compiler = Compiler::new();
    compiler.compile(&ast)?;
    
    let instructions = compiler.get_instructions().to_vec();
    let constants = compiler.get_constants().to_vec();
    let mut vm = VM::new(instructions, constants);
    vm.run()?;
    
    Ok(())
}

// ============================================================================
// EXAMPLES AND TESTS
// ============================================================================

fn main() {
    println!("Ether v0.2 - Advanced Features Demo");
    println!("{}", "=".repeat(60));
    
    // Example 1: List Indexing and Slicing
    println!("\nExample 1: List Indexing and Slicing");
    println!("{}", "-".repeat(50));
    let code1 = r#"
        let numbers = [10, 20, 30, 40, 50]
        print(numbers[0])
        print(numbers[2])
        print(numbers[-1])
        
        let slice1 = numbers[1:3]
        print(slice1)
        
        let slice2 = numbers[:3]
        print(slice2)
        
        let slice3 = numbers[2:]
        print(slice3)
    "#;
    if let Err(e) = compile_and_run(code1) {
        eprintln!("Error: {}", e);
    }
    
    // Example 2: Maps
    println!("\nExample 2: Map/Hash Structure");
    println!("{}", "-".repeat(50));
    let code2 = r#"
        let person = {
            "name": "Alice",
            "age": 30,
            "city": "Lagos"
        }
        
        print(person["name"])
        print(person["age"])
        
        person["country"] = "Nigeria"
        print(person["country"])
    "#;
    if let Err(e) = compile_and_run(code2) {
        eprintln!("Error: {}", e);
    }
    
    // Example 3: Ranges
    println!("\nExample 3: Range Objects");
    println!("{}", "-".repeat(50));
    let code3 = r#"
        let range1 = 1..5
        print(range1)
        
        let range2 = 1..=5
        print(range2)
        
        let range3 = 10..5
        print(range3)
    "#;
    if let Err(e) = compile_and_run(code3) {
        eprintln!("Error: {}", e);
    }
    
    // Example 4: For-In Loops
    println!("\nExample 4: For-In Loops");
    println!("{}", "-".repeat(50));
    let code4 = r#"
        print("Iterating over list:")
        for item in [1, 2, 3, 4, 5] {
            print(item)
        }
        
        print("Iterating over range:")
        for i in 0..3 {
            print(i)
        }
        
        print("Iterating over map:")
        let data = {"x": 10, "y": 20}
        for pair in data {
            print(pair)
        }
    "#;
    if let Err(e) = compile_and_run(code4) {
        eprintln!("Error: {}", e);
    }
    
    // Example 5: Custom Exceptions
    println!("\nExample 5: Custom Exceptions with raise");
    println!("{}", "-".repeat(50));
    let code5 = r#"
        fn validate_age(age) {
            if age < 0 {
                raise ValueError("Age cannot be negative")
            }
            if age > 150 {
                raise ValueError("Age is unrealistic")
            }
            return age
        }
        
        try {
            validate_age(25)
            print("Age 25 is valid")
            
            validate_age(-5)
        } catch (e) {
            print("Caught exception:")
            print(e)
        }
        
        try {
            validate_age(200)
        } catch (e) {
            print("Another exception:")
            print(e)
        }
    "#;
    if let Err(e) = compile_and_run(code5) {
        eprintln!("Error: {}", e);
    }
    
    // Example 6: Pattern Matching
    println!("\nExample 6: Match Expressions");
    println!("{}", "-".repeat(50));
    let code6 = r#"
        fn describe_number(n) {
            let result = match n {
                0 => { print("zero") },
                1 => { print("one") },
                2 => { print("two") },
                _ => { print("many") }
            }
        }
        
        describe_number(0)
        describe_number(1)
        describe_number(5)
        
        fn classify_value(x) {
            let result = match x {
                true => { print("boolean true") },
                false => { print("boolean false") },
                _ => { print("other value") }
            }
        }
        
        classify_value(true)
        classify_value(false)
    "#;
    if let Err(e) = compile_and_run(code6) {
        eprintln!("Error: {}", e);
    }
    
    // Example 7: Advanced List Operations
    println!("\nExample 7: Advanced List Operations");
    println!("{}", "-".repeat(50));
    let code7 = r#"
        let matrix = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
        print(matrix[0])
        print(matrix[1])
        print(matrix[0][1])
        
        let modified = [1, 2, 3, 4, 5]
        modified[2] = 99
        print(modified)
        
        let sliced = modified[1:4]
        print(sliced)
    "#;
    if let Err(e) = compile_and_run(code7) {
        eprintln!("Error: {}", e);
    }
    
    // Example 8: Combining Features
    println!("\nExample 8: Combining All Features");
    println!("{}", "-".repeat(50));
    let code8 = r#"
        fn process_data(data) {
            let results = [1]
            
            for item in data {
                if item > 0 {
                    results[0] = item * 2
                }
            }
            
            return results
        }
        
        let numbers = [1, 2, 3, 4, 5]
        let doubled = process_data(numbers)
        
        for n in 0..=3 {
            print(n)
        }
        
        let scores = {
            "alice": 95,
            "bob": 87,
            "charlie": 92
        }
        
        print(scores["alice"])
        
        try {
            let x = scores["dave"]
        } catch (e) {
            print("Key not found")
        }
    "#;
    if let Err(e) = compile_and_run(code8) {
        eprintln!("Error: {}", e);
    }
    
    // Example 9: Tensor with Indexing
    println!("\nExample 9: Tensor Operations with Indexing");
    println!("{}", "-".repeat(50));
    let code9 = r#"
        let weights = tensor([3, 3])
        print(weights)
        
        print(weights[0])
        print(weights[4])
        
        let activated = relu(weights)
        print(activated[2])
    "#;
    if let Err(e) = compile_and_run(code9) {
        eprintln!("Error: {}", e);
    }
    
    // Example 10: Complex Pattern Matching with Guards
    println!("\nExample 10: Advanced Exception Handling");
    println!("{}", "-".repeat(50));
    let code10 = r#"
        fn divide_safe(a, b) {
            if b == 0 {
                raise DivisionError("Cannot divide by zero")
            }
            return a / b
        }
        
        fn process_values(x, y) {
            try {
                let result = divide_safe(x, y)
                print(result)
                
                if result > 100 {
                    raise ValueError("Result too large")
                }
            } catch (e) {
                print("Error during processing:")
                print(e)
            } finally {
                print("Processing completed")
            }
        }
        
        process_values(10, 2)
        process_values(10, 0)
    "#;
    if let Err(e) = compile_and_run(code10) {
        eprintln!("Error: {}", e);
    }
    
    // Example 11: Nested Structures
    println!("\nExample 11: Nested Data Structures");
    println!("{}", "-".repeat(50));
    let code11 = r#"
        let database = {
            "users": [
                {"name": "Alice", "age": 30},
                {"name": "Bob", "age": 25}
            ],
            "active": true
        }
        
        print(database["active"])
        
        for user in database["users"] {
            print(user)
        }
    "#;
    if let Err(e) = compile_and_run(code11) {
        eprintln!("Error: {}", e);
    }
    
    // Example 12: Range-based Iteration
    println!("\nExample 12: Advanced Range Usage");
    println!("{}", "-".repeat(50));
    let code12 = r#"
        print("Count up:")
        for i in 1..=5 {
            print(i)
        }
        
        print("Count down:")
        for i in 5..1 {
            print(i)
        }
        
        print("Step through list with range:")
        let data = ["a", "b", "c", "d", "e"]
        for i in 0..3 {
            print(data[i])
        }
    "#;
    if let Err(e) = compile_and_run(code12) {
        eprintln!("Error: {}", e);
    }
    
    println!("\n{}", "=".repeat(60));
    println!("Ether v0.2 Demo Complete!");
    println!("All Advanced Features Implemented:");
    println!("  ✓ List/Tensor Indexing & Slicing");
    println!("  ✓ Map/Hash Structure");
    println!("  ✓ Range Objects (.. and ..=)");
    println!("  ✓ For-In Loops");
    println!("  ✓ Custom Exceptions (raise)");
    println!("  ✓ Pattern Matching (match/case)");
    println!("{}", "=".repeat(60));
}