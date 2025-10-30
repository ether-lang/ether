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
  Let,
  Fn,
  Return,
  If,
  Else,
  While,
  For,
  In,
  Match,
  Case,
  Tensor,
  Import,
  Try,
  Catch,
  Finally,
  Throw,
  Raise,
  Range,
  Map,
  Class,
  Extends,
  New,
  Self_,
  Super,
  Static,
  Public,
  Private,

  // Identifiers
  Ident(String),

  // Operators
  Plus,
  Minus,
  Star,
  StarStar,
  Slash,
  SlashSlash,
  Percent,
  Eq,
  Neq,
  Lt,
  Gt,
  Lte,
  Gte,
  And,
  Or,
  Not,
  Assign,
  Arrow,
  FatArrow,
  DotDot,
  DotDotEq,

  // Delimiters
  LParen,
  RParen,
  LBrace,
  RBrace,
  LBracket,
  RBracket,
  Comma,
  Colon,
  Semicolon,
  Dot,
  Pipe,

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
    if self.current() == Some('#') {
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
      // "as" => TokenType::As,
      "try" => TokenType::Try,
      "catch" => TokenType::Catch,
      "finally" => TokenType::Finally,
      "throw" => TokenType::Throw,
      "raise" => TokenType::Raise,
      "range" => TokenType::Range,
      "map" => TokenType::Map,
      "class" => TokenType::Class,
      "extends" => TokenType::Extends,
      "new" => TokenType::New,
      "self" => TokenType::Self_,
      "super" => TokenType::Super,
      "static" => TokenType::Static,
      "public" => TokenType::Public,
      "private" => TokenType::Private,
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

      let chr = self.current().unwrap();
      if chr == '#' {
        self.skip_comment();
        continue;
      }

      let line = self.line;
      let column = self.column;

      let ttype = match chr {
        ch if ch.is_ascii_digit() => self.read_number(),
        '"' => self.read_string(),
        ch if ch.is_alphabetic() || ch == '_' => self.read_identifier(),
        '+' => {
          self.advance();
          TokenType::Plus
        }
        '-' => {
          self.advance();
          if self.current() == Some('>') {
            self.advance();
            TokenType::Arrow
          } else {
            TokenType::Minus
          }
        }
        '*' => {
          self.advance();
          if self.current() == Some('*') {
            self.advance();
            TokenType::StarStar
          } else {
            TokenType::Star
          }
        }
        '/' => {
          self.advance();
          if self.current() == Some('/') {
            self.advance();
            TokenType::SlashSlash
          } else {
            TokenType::Slash
          }
        }
        '%' => {
          self.advance();
          TokenType::Percent
        }
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
        '(' => {
          self.advance();
          TokenType::LParen
        }
        ')' => {
          self.advance();
          TokenType::RParen
        }
        '{' => {
          self.advance();
          TokenType::LBrace
        }
        '}' => {
          self.advance();
          TokenType::RBrace
        }
        '[' => {
          self.advance();
          TokenType::LBracket
        }
        ']' => {
          self.advance();
          TokenType::RBracket
        }
        ',' => {
          self.advance();
          TokenType::Comma
        }
        ':' => {
          self.advance();
          TokenType::Colon
        }
        ';' => {
          self.advance();
          TokenType::Semicolon
        }
        '|' => {
          self.advance();
          TokenType::Pipe
        }
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
        ch => {
          return Err(format!(
            "Unexpected character '{}' at {}:{}",
            ch, line, column
          ));
        }
      };

      tokens.push(Token {
        ttype,
        line,
        column,
      });
    }

    tokens.push(Token {
      ttype: TokenType::Eof,
      line: self.line,
      column: self.column,
    });
    Ok(tokens)
  }
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, String> {
  Lexer::new(source).tokenize()
}
