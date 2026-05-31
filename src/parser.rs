// Parser: recursive descent with precedence climbing

use crate::ast::*;
use crate::lexer::{Span, Token, TokenKind};
use crate::symbol_table::Type;

/// Parser error.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error at {}: {}", self.span, self.message)
    }
}

impl std::error::Error for ParseError {}

pub type ParseResult<T> = Result<T, ParseError>;

/// The parser.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or_else(|| self.tokens.last().unwrap())
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or_else(|| {
            Token::eof(Span::zero())
        });
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &TokenKind) -> ParseResult<Token> {
        let tok = self.advance();
        if std::mem::discriminant(&tok.kind) == std::mem::discriminant(expected) {
            Ok(tok)
        } else {
            Err(ParseError {
                message: format!("expected {}, got {}", expected, tok.kind),
                span: tok.span,
            })
        }
    }

    fn match_kind(&mut self, kind: &TokenKind) -> Option<Token> {
        if std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind) {
            Some(self.advance())
        } else {
            None
        }
    }

    fn is(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    fn span_here(&self) -> Span {
        self.peek().span
    }

    /// Parse a full program.
    pub fn parse_program(&mut self) -> ParseResult<Program> {
        let mut items = Vec::new();
        while !self.is(&TokenKind::Eof) {
            items.push(self.parse_item()?);
        }
        Ok(Program { items })
    }

    fn parse_item(&mut self) -> ParseResult<Item> {
        match &self.peek().kind {
            TokenKind::Fn => {
                let stmt = self.parse_fn_decl()?;
                Ok(Item::Function(stmt))
            }
            TokenKind::Struct => {
                let stmt = self.parse_struct_decl()?;
                Ok(Item::Struct(stmt))
            }
            TokenKind::Agent => {
                let agent = self.parse_agent_decl()?;
                Ok(Item::Agent(agent))
            }
            _ => {
                let expr = self.parse_expression()?;
                self.match_kind(&TokenKind::Semicolon);
                Ok(Item::Expr(expr))
            }
        }
    }

    fn parse_fn_decl(&mut self) -> ParseResult<Stmt> {
        let span = self.expect(&TokenKind::Fn)?.span;
        let name = match self.advance().kind {
            TokenKind::Ident(s) => s,
            other => return Err(ParseError {
                message: format!("expected function name, got {}", other),
                span: self.span_here(),
            }),
        };
        self.expect(&TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect(&TokenKind::RParen)?;
        let return_type = if self.match_kind(&TokenKind::Arrow).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(Stmt::FnDecl {
            name,
            params,
            return_type,
            body: Box::new(body),
            span,
        })
    }

    fn parse_params(&mut self) -> ParseResult<Vec<(String, Option<Type>)>> {
        let mut params = Vec::new();
        if !self.is(&TokenKind::RParen) {
            loop {
                let name = match self.advance().kind {
                    TokenKind::Ident(s) => s,
                    other => return Err(ParseError {
                        message: format!("expected parameter name, got {}", other),
                        span: self.span_here(),
                    }),
                };
                let ty = if self.match_kind(&TokenKind::Colon).is_some() {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                params.push((name, ty));
                if self.match_kind(&TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        Ok(params)
    }

    fn parse_type(&mut self) -> ParseResult<Type> {
        match &self.peek().kind {
            TokenKind::Ident(s) if s == "int" => { self.advance(); Ok(Type::Int) }
            TokenKind::Ident(s) if s == "float" => { self.advance(); Ok(Type::Float) }
            TokenKind::Ident(s) if s == "bool" => { self.advance(); Ok(Type::Bool) }
            TokenKind::Ident(s) if s == "string" => { self.advance(); Ok(Type::String) }
            TokenKind::Ident(s) if s == "void" => { self.advance(); Ok(Type::Void) }
            TokenKind::Ident(s) => {
                let name = s.clone();
                self.advance();
                Ok(Type::Named(name))
            }
            TokenKind::LBracket => {
                self.advance();
                let inner = self.parse_type()?;
                self.expect(&TokenKind::RBracket)?;
                Ok(Type::Array(Box::new(inner)))
            }
            TokenKind::LParen => {
                self.advance();
                let mut params = Vec::new();
                if !self.is(&TokenKind::RParen) {
                    loop {
                        params.push(self.parse_type()?);
                        if self.match_kind(&TokenKind::Comma).is_none() {
                            break;
                        }
                    }
                }
                self.expect(&TokenKind::RParen)?;
                self.expect(&TokenKind::Arrow)?;
                let ret = self.parse_type()?;
                Ok(Type::Function(params, Box::new(ret)))
            }
            other => Err(ParseError {
                message: format!("expected type, got {}", other),
                span: self.span_here(),
            }),
        }
    }

    fn parse_struct_decl(&mut self) -> ParseResult<Stmt> {
        let span = self.expect(&TokenKind::Struct)?.span;
        let name = match self.advance().kind {
            TokenKind::Ident(s) => s,
            other => return Err(ParseError {
                message: format!("expected struct name, got {}", other),
                span: self.span_here(),
            }),
        };
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !self.is(&TokenKind::RBrace) {
            let fname = match self.advance().kind {
                TokenKind::Ident(s) => s,
                other => return Err(ParseError {
                    message: format!("expected field name, got {}", other),
                    span: self.span_here(),
                }),
            };
            self.expect(&TokenKind::Colon)?;
            let ftype = self.parse_type()?;
            fields.push((fname, ftype));
            self.match_kind(&TokenKind::Comma);
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Stmt::StructDecl { name, fields, span })
    }

    fn parse_block(&mut self) -> ParseResult<Stmt> {
        let span = self.expect(&TokenKind::LBrace)?.span;
        let mut stmts = Vec::new();
        while !self.is(&TokenKind::RBrace) && !self.is(&TokenKind::Eof) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Stmt::Block(stmts, span))
    }

    fn parse_stmt(&mut self) -> ParseResult<Stmt> {
        match &self.peek().kind {
            TokenKind::Let => self.parse_let(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Break => {
                let span = self.advance().span;
                self.match_kind(&TokenKind::Semicolon);
                Ok(Stmt::Break(span))
            }
            TokenKind::Continue => {
                let span = self.advance().span;
                self.match_kind(&TokenKind::Semicolon);
                Ok(Stmt::Continue(span))
            }
            TokenKind::LBrace => self.parse_block(),
            _ => self.parse_expr_or_assign_stmt(),
        }
    }

    fn parse_let(&mut self) -> ParseResult<Stmt> {
        let span = self.expect(&TokenKind::Let)?.span;
        let name = match self.advance().kind {
            TokenKind::Ident(s) => s,
            other => return Err(ParseError {
                message: format!("expected variable name, got {}", other),
                span: self.span_here(),
            }),
        };
        let ty = if self.match_kind(&TokenKind::Colon).is_some() {
            Some(self.parse_type()?)
        } else {
            None
        };
        let init = if self.match_kind(&TokenKind::Eq).is_some() {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.match_kind(&TokenKind::Semicolon);
        Ok(Stmt::Let { name, ty, init, span })
    }

    fn parse_if(&mut self) -> ParseResult<Stmt> {
        let span = self.expect(&TokenKind::If)?.span;
        let condition = self.parse_expression()?;
        let then_branch = self.parse_block_or_stmt()?;
        let else_branch = if self.match_kind(&TokenKind::Else).is_some() {
            Some(Box::new(self.parse_block_or_stmt()?))
        } else {
            None
        };
        Ok(Stmt::If {
            condition,
            then_branch: Box::new(then_branch),
            else_branch,
            span,
        })
    }

    fn parse_block_or_stmt(&mut self) -> ParseResult<Stmt> {
        if self.is(&TokenKind::LBrace) {
            self.parse_block()
        } else {
            self.parse_stmt()
        }
    }

    fn parse_while(&mut self) -> ParseResult<Stmt> {
        let span = self.expect(&TokenKind::While)?.span;
        let condition = self.parse_expression()?;
        let body = self.parse_block_or_stmt()?;
        Ok(Stmt::While {
            condition,
            body: Box::new(body),
            span,
        })
    }

    fn parse_for(&mut self) -> ParseResult<Stmt> {
        let span = self.expect(&TokenKind::For)?.span;
        let var = match self.advance().kind {
            TokenKind::Ident(s) => s,
            other => return Err(ParseError {
                message: format!("expected variable name, got {}", other),
                span: self.span_here(),
            }),
        };
        self.expect(&TokenKind::In)?;
        let iterable = self.parse_expression()?;
        let body = self.parse_block_or_stmt()?;
        Ok(Stmt::For {
            var,
            iterable,
            body: Box::new(body),
            span,
        })
    }

    fn parse_return(&mut self) -> ParseResult<Stmt> {
        let span = self.expect(&TokenKind::Return)?.span;
        let value = if !self.is(&TokenKind::Semicolon) && !self.is(&TokenKind::RBrace) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.match_kind(&TokenKind::Semicolon);
        Ok(Stmt::Return(value, span))
    }

    fn parse_expr_or_assign_stmt(&mut self) -> ParseResult<Stmt> {
        let expr = self.parse_expression()?;
        if self.match_kind(&TokenKind::Eq).is_some() {
            let value = self.parse_expression()?;
            let span = expr.span().clone();
            self.match_kind(&TokenKind::Semicolon);
            Ok(Stmt::Assign {
                target: expr,
                value,
                span,
            })
        } else {
            self.match_kind(&TokenKind::Semicolon);
            let span = expr.span().clone();
            Ok(Stmt::ExprStmt(expr, span))
        }
    }

    /// Parse expression with full precedence climbing.
    pub fn parse_expression(&mut self) -> ParseResult<Expr> {
        self.parse_precedence(0)
    }

    fn parse_precedence(&mut self, min_prec: u8) -> ParseResult<Expr> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match Option::<BinOp>::from(&self.peek().kind) {
                Some(op) => op,
                None => break,
            };
            let prec = op.precedence();
            if prec < min_prec {
                break;
            }
            self.advance(); // consume operator
            let right = self.parse_precedence(prec + 1)?;
            let span = Span::new(left.span().start, right.span().end, left.span().line, left.span().col);
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> ParseResult<Expr> {
        let span = self.span_here();
        if self.is(&TokenKind::Minus) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                operand: Box::new(operand),
                span,
            });
        }
        if self.is(&TokenKind::Bang) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                operand: Box::new(operand),
                span,
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            match &self.peek().kind {
                TokenKind::LParen => {
                    let span = expr.span().clone();
                    self.advance();
                    let mut args = Vec::new();
                    if !self.is(&TokenKind::RParen) {
                        args.push(self.parse_expression()?);
                        while self.match_kind(&TokenKind::Comma).is_some() {
                            args.push(self.parse_expression()?);
                        }
                    }
                    let end = self.expect(&TokenKind::RParen)?.span.end;
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args,
                        span: Span::new(span.start, end, span.line, span.col),
                    };
                }
                TokenKind::LBracket => {
                    let span = expr.span().clone();
                    self.advance();
                    let index = self.parse_expression()?;
                    let end = self.expect(&TokenKind::RBracket)?.span.end;
                    expr = Expr::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                        span: Span::new(span.start, end, span.line, span.col),
                    };
                }
                TokenKind::Dot => {
                    let span = expr.span().clone();
                    self.advance();
                    let field = match self.advance().kind {
                        TokenKind::Ident(s) => s,
                        other => return Err(ParseError {
                            message: format!("expected field name, got {}", other),
                            span: self.span_here(),
                        }),
                    };
                    expr = Expr::Member {
                        object: Box::new(expr),
                        field,
                        span,
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> ParseResult<Expr> {
        let tok = self.advance();
        let span = tok.span;
        match tok.kind {
            TokenKind::Int(v) => Ok(Expr::IntLiteral(v, span)),
            TokenKind::Float(s) => Ok(Expr::FloatLiteral(s.parse().unwrap_or(0.0), span)),
            TokenKind::String(s) => Ok(Expr::StringLiteral(s, span)),
            TokenKind::Bool(b) => Ok(Expr::BoolLiteral(b, span)),
            TokenKind::Null => Ok(Expr::NullLiteral(span)),
            TokenKind::Ident(s) => Ok(Expr::Ident(s, span)),
            // Treat agent DSL keywords as identifiers in expression context
            TokenKind::Emit => Ok(Expr::Ident("emit".into(), span)),
            TokenKind::With => Ok(Expr::Ident("with".into(), span)),
            TokenKind::On => Ok(Expr::Ident("on".into(), span)),
            TokenKind::Do => Ok(Expr::Ident("do".into(), span)),
            TokenKind::Timeout => Ok(Expr::Ident("timeout".into(), span)),
            TokenKind::Retry => Ok(Expr::Ident("retry".into(), span)),
            TokenKind::Guard => Ok(Expr::Ident("guard".into(), span)),
            TokenKind::Trigger => Ok(Expr::Ident("trigger".into(), span)),
            TokenKind::LParen => {
                let expr = self.parse_expression()?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::LBracket => {
                let mut elems = Vec::new();
                if !self.is(&TokenKind::RBracket) {
                    elems.push(self.parse_expression()?);
                    while self.match_kind(&TokenKind::Comma).is_some() {
                        elems.push(self.parse_expression()?);
                    }
                }
                self.expect(&TokenKind::RBracket)?;
                Ok(Expr::Array(elems, span))
            }
            other => Err(ParseError {
                message: format!("unexpected token: {}", other),
                span,
            }),
        }
    }

    // ---- Agent DSL parsing ----

    fn parse_agent_decl(&mut self) -> ParseResult<AgentDecl> {
        let span = self.expect(&TokenKind::Agent)?.span;
        let name = match self.advance().kind {
            TokenKind::Ident(s) => s,
            other => return Err(ParseError {
                message: format!("expected agent name, got {}", other),
                span: self.span_here(),
            }),
        };
        self.expect(&TokenKind::LBrace)?;
        let mut skills = Vec::new();
        let mut triggers = Vec::new();
        let mut config = Vec::new();
        while !self.is(&TokenKind::RBrace) && !self.is(&TokenKind::Eof) {
            match &self.peek().kind {
                TokenKind::Skill => skills.push(self.parse_skill_decl()?),
                TokenKind::Trigger | TokenKind::On => triggers.push(self.parse_trigger_decl()?),
                TokenKind::Ident(_)
                | TokenKind::Timeout
                | TokenKind::Retry => {
                    let key = match self.advance().kind {
                        TokenKind::Ident(s) => s,
                        TokenKind::Timeout => "timeout".into(),
                        TokenKind::Retry => "retry".into(),
                        _ => unreachable!(),
                    };
                    self.expect(&TokenKind::Colon)?;
                    let value = self.parse_expression()?;
                    config.push((key, value));
                    self.match_kind(&TokenKind::Comma);
                }
                _ => {
                    self.advance(); // skip unknown
                }
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(AgentDecl { name, skills, triggers, config, span })
    }

    fn parse_skill_decl(&mut self) -> ParseResult<SkillDecl> {
        let span = self.expect(&TokenKind::Skill)?.span;
        let name = match self.advance().kind {
            TokenKind::Ident(s) => s,
            other => return Err(ParseError {
                message: format!("expected skill name, got {}", other),
                span: self.span_here(),
            }),
        };
        self.expect(&TokenKind::LBrace)?;
        let mut handler = Expr::NullLiteral(span);
        let mut config = Vec::new();
        while !self.is(&TokenKind::RBrace) && !self.is(&TokenKind::Eof) {
            match &self.peek().kind {
                TokenKind::Do => {
                    self.advance();
                    handler = self.parse_expression()?;
                    self.match_kind(&TokenKind::Semicolon);
                }
                TokenKind::Ident(_)
                | TokenKind::Timeout
                | TokenKind::Retry
                | TokenKind::Guard => {
                    let key = match self.advance().kind {
                        TokenKind::Ident(s) => s,
                        TokenKind::Timeout => "timeout".into(),
                        TokenKind::Retry => "retry".into(),
                        TokenKind::Guard => "guard".into(),
                        _ => unreachable!(),
                    };
                    self.expect(&TokenKind::Colon)?;
                    let value = self.parse_expression()?;
                    config.push((key, value));
                    self.match_kind(&TokenKind::Comma);
                }
                _ => { self.advance(); }
            }
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(SkillDecl { name, handler, config, span })
    }

    fn parse_trigger_decl(&mut self) -> ParseResult<TriggerDecl> {
        let span = if self.is(&TokenKind::On) {
            self.expect(&TokenKind::On)?.span
        } else {
            self.expect(&TokenKind::Trigger)?.span
        };
        let event = match self.advance().kind {
            TokenKind::Ident(s) => s,
            other => return Err(ParseError {
                message: format!("expected event name, got {}", other),
                span: self.span_here(),
            }),
        };
        let guard = if self.is(&TokenKind::Guard) || self.is(&TokenKind::If) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.expect(&TokenKind::Do)?;
        let action = self.parse_expression()?;
        self.match_kind(&TokenKind::Semicolon);
        Ok(TriggerDecl { event, guard, action, span })
    }
}

/// Parse a source string into a Program.
pub fn parse(source: &str) -> Result<Program, Vec<ParseError>> {
    let tokens = crate::lexer::tokenize(source).map_err(|errs| {
        errs.into_iter().map(|e| ParseError {
            message: e.message,
            span: e.span,
        }).collect::<Vec<_>>()
    })?;
    let mut parser = Parser::new(tokens);
    parser.parse_program().map_err(|e| vec![e])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_expr(src: &str) -> Expr {
        let tokens = crate::lexer::tokenize(src).unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_expression().unwrap()
    }

    #[test]
    fn test_simple_expression() {
        let expr = parse_expr("1 + 2");
        match &expr {
            Expr::Binary { op: BinOp::Add, .. } => {}
            other => panic!("expected binary add, got {:?}", other),
        }
    }

    #[test]
    fn test_precedence_climbing() {
        let expr = parse_expr("1 + 2 * 3");
        // Should be 1 + (2 * 3)
        match &expr {
            Expr::Binary { op: BinOp::Add, right, .. } => {
                match right.as_ref() {
                    Expr::Binary { op: BinOp::Mul, .. } => {}
                    other => panic!("expected mul on right, got {:?}", other),
                }
            }
            other => panic!("expected add, got {:?}", other),
        }
    }

    #[test]
    fn test_left_associativity() {
        let expr = parse_expr("1 - 2 - 3");
        // Should be (1 - 2) - 3
        match &expr {
            Expr::Binary { op: BinOp::Sub, left, .. } => {
                match left.as_ref() {
                    Expr::Binary { op: BinOp::Sub, .. } => {}
                    other => panic!("expected sub on left, got {:?}", other),
                }
            }
            other => panic!("expected sub, got {:?}", other),
        }
    }

    #[test]
    fn test_parentheses() {
        let expr = parse_expr("(1 + 2) * 3");
        match &expr {
            Expr::Binary { op: BinOp::Mul, left, .. } => {
                match left.as_ref() {
                    Expr::Binary { op: BinOp::Add, .. } => {}
                    other => panic!("expected add in parens, got {:?}", other),
                }
            }
            other => panic!("expected mul, got {:?}", other),
        }
    }

    #[test]
    fn test_unary() {
        let expr = parse_expr("-x");
        match &expr {
            Expr::Unary { op: UnaryOp::Neg, .. } => {}
            other => panic!("expected neg, got {:?}", other),
        }
    }

    #[test]
    fn test_function_call() {
        let expr = parse_expr("f(1, 2)");
        match &expr {
            Expr::Call { args, .. } => assert_eq!(args.len(), 2),
            other => panic!("expected call, got {:?}", other),
        }
    }

    #[test]
    fn test_let_stmt() {
        let tokens = crate::lexer::tokenize("let x = 42;").unwrap();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_stmt().unwrap();
        match &stmt {
            Stmt::Let { name, init: Some(Expr::IntLiteral(42, _)), .. } => {
                assert_eq!(name, "x");
            }
            other => panic!("expected let with init, got {:?}", other),
        }
    }

    #[test]
    fn test_fn_decl() {
        let tokens = crate::lexer::tokenize("fn add(a: int, b: int) -> int { return a + b; }").unwrap();
        let mut parser = Parser::new(tokens);
        let item = parser.parse_item().unwrap();
        match &item {
            Item::Function(Stmt::FnDecl { name, params, return_type, .. }) => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(return_type, &Some(Type::Int));
            }
            other => panic!("expected fn decl, got {:?}", other),
        }
    }

    #[test]
    fn test_if_else() {
        let tokens = crate::lexer::tokenize("if x > 0 { x } else { 0 }").unwrap();
        let mut parser = Parser::new(tokens);
        let stmt = parser.parse_stmt().unwrap();
        match &stmt {
            Stmt::If { else_branch: Some(_), .. } => {}
            other => panic!("expected if-else, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_program() {
        let prog = parse("fn main() { return 0; }").unwrap();
        assert_eq!(prog.items.len(), 1);
    }
}
