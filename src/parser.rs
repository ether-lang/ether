// ============================================================================
// PARSER
// ============================================================================

use crate::{
  ast::{BinOp, Expr, MatchCase, Pattern, Stmt, UnOp},
  lexer::{Token, TokenType},
  types::Type,
};

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

  // fn peek(&self, offset: usize) -> &Token {
  //   let pos = self.pos + offset;
  //   if pos < self.tokens.len() {
  //     &self.tokens[pos]
  //   } else {
  //     self.tokens.last().unwrap()
  //   }
  // }

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
      Err(format!(
        "Unexpected token at {}:{}",
        token.line, token.column
      ))
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
      TokenType::Def => self.parse_function(),
      TokenType::Class => self.parse_class(),
      TokenType::Return => self.parse_return(),
      TokenType::If => self.parse_if(),
      TokenType::While => self.parse_while(),
      TokenType::For => self.parse_for_in(),
      TokenType::Try => self.parse_try(),
      TokenType::Throw => self.parse_throw(),
      TokenType::Raise => self.parse_raise(),
      TokenType::Ident(_) | TokenType::Self_ => {
        // Save position in case we need to backtrack
        let _start_pos = self.pos;

        // Parse the full expression (including member access)
        let expr = self.parse_expression()?;

        // Now check if this is an assignment
        if matches!(self.current().ttype, TokenType::Assign) {
          self.advance(); // consume '='
          let value = Box::new(self.parse_expression()?);

          match expr {
            Expr::Ident(name) => Ok(Stmt::Assign { name, value }),
            Expr::Index { target, index } => Ok(Stmt::IndexAssign {
              target,
              index,
              value,
            }),
            Expr::MemberAccess { object, member } => Ok(Stmt::FieldAssign {
              object,
              field: member,
              value,
            }),
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

    Ok(Stmt::Let {
      name,
      value,
      type_annotation,
    })
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

    Ok(Stmt::Function {
      name,
      params,
      body,
      return_type,
    })
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

    Ok(Stmt::If {
      condition,
      then_block,
      else_block,
    })
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

    Ok(Stmt::ForIn {
      var_name,
      iterable,
      body,
    })
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

    if catch_block.is_none() && finally_block.is_none() {
      return Err("At least one of catch or finally block is expected for Try.".to_string());
    }

    Ok(Stmt::Try {
      try_block,
      catch_var,
      catch_block,
      finally_block,
    })
  }

  fn parse_throw(&mut self) -> Result<Stmt, String> {
    self.advance();
    let value = Box::new(self.parse_expression()?);
    Ok(Stmt::Throw { value })
  }

  fn parse_raise(&mut self) -> Result<Stmt, String> {
    self.advance(); // consume 'raise'

    let error_type = if let TokenType::Ident(n) = &self.current().ttype {
      let name = n.clone();
      self.advance();
      name
    } else {
      return Err("Expected error type after 'raise'".to_string());
    };

    self.expect(|t| matches!(t, TokenType::LParen))?;
    let message = Box::new(self.parse_expression()?);
    self.expect(|t| matches!(t, TokenType::RParen))?;

    Ok(Stmt::Raise {
      error_type,
      message,
    })
  }

  fn parse_class(&mut self) -> Result<Stmt, String> {
    self.advance(); // consume 'class'

    let name = if let TokenType::Ident(n) = &self.current().ttype {
      let name = n.clone();
      self.advance();
      name
    } else {
      return Err("Expected class name".to_string());
    };

    let mut parents = Vec::new();
    if matches!(self.current().ttype, TokenType::Extends) {
      self.advance();

      loop {
        if let TokenType::Ident(parent) = &self.current().ttype {
          parents.push(parent.clone());
          self.advance();

          if matches!(self.current().ttype, TokenType::Comma) {
            self.advance();
          } else {
            break;
          }
        } else {
          return Err("Expected parent class name".to_string());
        }
      }
    }

    self.expect(|t| matches!(t, TokenType::LBrace))?;

    let mut methods = Vec::new();
    let mut fields = Vec::new();

    while !matches!(self.current().ttype, TokenType::RBrace) {
      let is_static = if matches!(self.current().ttype, TokenType::Static) {
        self.advance();
        true
      } else {
        false
      };

      let is_private = if matches!(self.current().ttype, TokenType::Private) {
        self.advance();
        true
      } else if matches!(self.current().ttype, TokenType::Public) {
        self.advance();
        false
      } else {
        false
      };

      if matches!(self.current().ttype, TokenType::Def) {
        self.advance();

        let method_name = match &self.current().ttype {
          TokenType::Ident(n) => {
            let name = n.clone();
            self.advance();
            name
          }
          TokenType::New => {
            self.advance();
            "new".to_string() // Treat 'new' keyword as the identifier "new"
          }
          _ => return Err("Expected method name or constructor".to_string()),
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

        methods.push((
          method_name,
          params,
          body,
          return_type,
          is_static,
          is_private,
        ));
      } else if matches!(self.current().ttype, TokenType::Let) {
        self.advance();

        let field_name = if let TokenType::Ident(n) = &self.current().ttype {
          let name = n.clone();
          self.advance();
          name
        } else {
          return Err("Expected field name".to_string());
        };

        let default_value = if matches!(self.current().ttype, TokenType::Assign) {
          self.advance();
          Some(self.parse_expression()?)
        } else {
          None
        };

        fields.push((field_name, default_value, is_private));
      } else {
        return Err("Expected method or field declaration".to_string());
      }
    }

    self.expect(|t| matches!(t, TokenType::RBrace))?;

    Ok(Stmt::Class {
      name,
      parents,
      methods,
      fields,
    })
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

        cases.push(MatchCase {
          pattern,
          guard,
          body,
        });

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

    if matches!(
      self.current().ttype,
      TokenType::DotDot | TokenType::DotDotEq
    ) {
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
      TokenType::StarStar => Some(BinOp::Pow),
      TokenType::SlashSlash => Some(BinOp::Floor),
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
        TokenType::Dot => {
          self.advance();
          if let TokenType::Ident(member) = &self.current().ttype {
            let member_name = member.clone();
            self.advance();

            // Check if it's a method call
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
              expr = Expr::MethodCall {
                object: Box::new(expr),
                method: member_name,
                args,
              };
            } else {
              // It's a field/member access
              expr = Expr::MemberAccess {
                object: Box::new(expr),
                member: member_name,
              };
            }
          } else {
            return Err("Expected member name after '.'".to_string());
          }
        }
        TokenType::LBracket => {
          self.advance();

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
      TokenType::New => {
        self.advance();

        let class_name = if let TokenType::Ident(n) = &self.current().ttype {
          let name = n.clone();
          self.advance();
          name
        } else {
          return Err("Expected class name after 'new'".to_string());
        };

        self.expect(|t| matches!(t, TokenType::LParen))?;

        let mut args = Vec::new();
        while !matches!(self.current().ttype, TokenType::RParen) {
          args.push(self.parse_expression()?);
          if matches!(self.current().ttype, TokenType::Comma) {
            self.advance();
          }
        }

        self.expect(|t| matches!(t, TokenType::RParen))?;
        Ok(Expr::New { class_name, args })
      }
      TokenType::Self_ => {
        self.advance();
        Ok(Expr::SelfExpr)
      }
      TokenType::Super => {
        self.advance();

        // Check if it's super(args) or super.method(args)
        if matches!(self.current().ttype, TokenType::LParen) {
          // super(args) - automatically calls super.new(args)
          self.advance();

          let mut args = Vec::new();
          while !matches!(self.current().ttype, TokenType::RParen) {
            args.push(self.parse_expression()?);
            if matches!(self.current().ttype, TokenType::Comma) {
              self.advance();
            }
          }

          self.expect(|t| matches!(t, TokenType::RParen))?;
          Ok(Expr::SuperCall {
            method: "new".to_string(),
            args,
          })
        } else if matches!(self.current().ttype, TokenType::Dot) {
          // super.method(args)
          self.advance();

          let method = match &self.current().ttype {
            TokenType::Ident(n) => {
              let name = n.clone();
              self.advance();
              name
            }
            TokenType::New => {
              // Allow super.new explicitly
              self.advance();
              "new".to_string()
            }
            _ => return Err("Expected method name after 'super.'".to_string()),
          };

          self.expect(|t| matches!(t, TokenType::LParen))?;

          let mut args = Vec::new();
          while !matches!(self.current().ttype, TokenType::RParen) {
            args.push(self.parse_expression()?);
            if matches!(self.current().ttype, TokenType::Comma) {
              self.advance();
            }
          }

          self.expect(|t| matches!(t, TokenType::RParen))?;
          Ok(Expr::SuperCall { method, args })
        } else {
          return Err("Expected '(' or '.' after 'super'".to_string());
        }
      }
      _ => Err(format!(
        "Unexpected token at {}:{}",
        self.current().line,
        self.current().column
      )),
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

pub fn parse(tokens: Vec<Token>) -> Result<Vec<Stmt>, String> {
  Parser::new(tokens).parse()
}
