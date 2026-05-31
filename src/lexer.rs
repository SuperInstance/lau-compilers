// Lexer: regex-based tokenizer with position tracking

use serde::{Deserialize, Serialize};
use std::fmt;

/// Source position (line, column are 1-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub col: u32,
}

impl Span {
    pub fn new(start: usize, end: usize, line: u32, col: u32) -> Self {
        Self { start, end, line, col }
    }

    pub fn zero() -> Self {
        Self::new(0, 0, 1, 1)
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// Token types for a general-purpose language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenKind {
    // Literals
    Int(i64),
    Float(String),
    String(String),
    Bool(bool),
    Ident(String),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    Bang,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    Ampersand,
    Pipe,
    Caret,
    Shl,
    Shr,
    Arrow,       // ->
    FatArrow,    // =>

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Colon,
    Dot,

    // Keywords
    Let,
    If,
    Else,
    While,
    For,
    Fn,
    Return,
    Break,
    Continue,
    Struct,
    Enum,
    Match,
    True,
    False,
    Null,
    Import,
    Export,
    As,
    In,

    // Agent DSL keywords
    Agent,
    Skill,
    Trigger,
    On,
    Do,
    Timeout,
    Retry,
    Guard,
    Emit,
    With,

    // Special
    Eof,
    Error(String),
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Int(v) => write!(f, "{}", v),
            TokenKind::Float(v) => write!(f, "{}f", v),
            TokenKind::String(v) => write!(f, "\"{}\"", v),
            TokenKind::Bool(b) => write!(f, "{}", b),
            TokenKind::Ident(s) => write!(f, "{}", s),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::Eq => write!(f, "="),
            TokenKind::EqEq => write!(f, "=="),
            TokenKind::Bang => write!(f, "!"),
            TokenKind::BangEq => write!(f, "!="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::LtEq => write!(f, "<="),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::GtEq => write!(f, ">="),
            TokenKind::And => write!(f, "&&"),
            TokenKind::Or => write!(f, "||"),
            TokenKind::Ampersand => write!(f, "&"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Caret => write!(f, "^"),
            TokenKind::Shl => write!(f, "<<"),
            TokenKind::Shr => write!(f, ">>"),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Let => write!(f, "let"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::While => write!(f, "while"),
            TokenKind::For => write!(f, "for"),
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Struct => write!(f, "struct"),
            TokenKind::Enum => write!(f, "enum"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Null => write!(f, "null"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::Export => write!(f, "export"),
            TokenKind::As => write!(f, "as"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Agent => write!(f, "agent"),
            TokenKind::Skill => write!(f, "skill"),
            TokenKind::Trigger => write!(f, "trigger"),
            TokenKind::On => write!(f, "on"),
            TokenKind::Do => write!(f, "do"),
            TokenKind::Timeout => write!(f, "timeout"),
            TokenKind::Retry => write!(f, "retry"),
            TokenKind::Guard => write!(f, "guard"),
            TokenKind::Emit => write!(f, "emit"),
            TokenKind::With => write!(f, "with"),
            TokenKind::Eof => write!(f, "<eof>"),
            TokenKind::Error(e) => write!(f, "<error: {}>", e),
        }
    }
}

/// A token with its kind and source span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn eof(span: Span) -> Self {
        Self::new(TokenKind::Eof, span)
    }
}

/// Lexer error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexerError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lexer error at {}: {}", self.span, self.message)
    }
}

impl std::error::Error for LexerError {}

/// A rule for the regex-based lexer.
struct LexerRule {
    pattern: regex::Regex,
    action: fn(&regex::Match) -> TokenKind,
}

/// The lexer / tokenizer.
pub struct Lexer {
    source: String,
    rules: Vec<LexerRule>,
    pos: usize,
    line: u32,
    col: u32,
}

fn make_keyword_or_ident(s: &str) -> TokenKind {
    match s {
        "let" => TokenKind::Let,
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        "while" => TokenKind::While,
        "for" => TokenKind::For,
        "fn" => TokenKind::Fn,
        "return" => TokenKind::Return,
        "break" => TokenKind::Break,
        "continue" => TokenKind::Continue,
        "struct" => TokenKind::Struct,
        "enum" => TokenKind::Enum,
        "match" => TokenKind::Match,
        "true" => TokenKind::Bool(true),
        "false" => TokenKind::Bool(false),
        "null" => TokenKind::Null,
        "import" => TokenKind::Import,
        "export" => TokenKind::Export,
        "as" => TokenKind::As,
        "in" => TokenKind::In,
        "agent" => TokenKind::Agent,
        "skill" => TokenKind::Skill,
        "trigger" => TokenKind::Trigger,
        "on" => TokenKind::On,
        "do" => TokenKind::Do,
        "timeout" => TokenKind::Timeout,
        "retry" => TokenKind::Retry,
        "guard" => TokenKind::Guard,
        "emit" => TokenKind::Emit,
        "with" => TokenKind::With,
        _ => TokenKind::Ident(s.to_string()),
    }
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        let rules = vec![
            // Whitespace and comments (skip)
            LexerRule {
                pattern: regex::Regex::new(r"^[ \t\r\n]+").unwrap(),
                action: |_| TokenKind::Error("whitespace".into()),
            },
            LexerRule {
                pattern: regex::Regex::new(r"^//[^\n]*").unwrap(),
                action: |_| TokenKind::Error("comment".into()),
            },
            LexerRule {
                pattern: regex::Regex::new(r"^/\*[\s\S]*?\*/").unwrap(),
                action: |_| TokenKind::Error("comment".into()),
            },
            // Float (before int to avoid partial match)
            LexerRule {
                pattern: regex::Regex::new(r"^[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?").unwrap(),
                action: |m| TokenKind::Float(m.as_str().to_string()),
            },
            // Integer
            LexerRule {
                pattern: regex::Regex::new(r"^[0-9]+").unwrap(),
                action: |m| TokenKind::Int(m.as_str().parse().unwrap_or(0)),
            },
            // String literals
            LexerRule {
                pattern: regex::Regex::new(r#"^"([^"\\]|\\.)*""#).unwrap(),
                action: |m| {
                    let s = &m.as_str()[1..m.as_str().len()-1];
                    TokenKind::String(s.to_string())
                },
            },
            // Multi-char operators (must come before single-char)
            LexerRule {
                pattern: regex::Regex::new(r"^==").unwrap(),
                action: |_| TokenKind::EqEq,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^!=").unwrap(),
                action: |_| TokenKind::BangEq,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^<=").unwrap(),
                action: |_| TokenKind::LtEq,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^>=").unwrap(),
                action: |_| TokenKind::GtEq,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^&&").unwrap(),
                action: |_| TokenKind::And,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^\|\|").unwrap(),
                action: |_| TokenKind::Or,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^<<").unwrap(),
                action: |_| TokenKind::Shl,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^>>").unwrap(),
                action: |_| TokenKind::Shr,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^->").unwrap(),
                action: |_| TokenKind::Arrow,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^=>").unwrap(),
                action: |_| TokenKind::FatArrow,
            },
            // Identifiers and keywords
            LexerRule {
                pattern: regex::Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*").unwrap(),
                action: |m| make_keyword_or_ident(m.as_str()),
            },
            // Single-char operators/delimiters
            LexerRule {
                pattern: regex::Regex::new(r"^\+").unwrap(),
                action: |_| TokenKind::Plus,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^-").unwrap(),
                action: |_| TokenKind::Minus,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^\*").unwrap(),
                action: |_| TokenKind::Star,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^/").unwrap(),
                action: |_| TokenKind::Slash,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^%").unwrap(),
                action: |_| TokenKind::Percent,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^=").unwrap(),
                action: |_| TokenKind::Eq,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^!").unwrap(),
                action: |_| TokenKind::Bang,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^<").unwrap(),
                action: |_| TokenKind::Lt,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^>").unwrap(),
                action: |_| TokenKind::Gt,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^&").unwrap(),
                action: |_| TokenKind::Ampersand,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^\|").unwrap(),
                action: |_| TokenKind::Pipe,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^\^").unwrap(),
                action: |_| TokenKind::Caret,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^\(").unwrap(),
                action: |_| TokenKind::LParen,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^\)").unwrap(),
                action: |_| TokenKind::RParen,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^\{").unwrap(),
                action: |_| TokenKind::LBrace,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^\}").unwrap(),
                action: |_| TokenKind::RBrace,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^\[").unwrap(),
                action: |_| TokenKind::LBracket,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^\]").unwrap(),
                action: |_| TokenKind::RBracket,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^,").unwrap(),
                action: |_| TokenKind::Comma,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^;").unwrap(),
                action: |_| TokenKind::Semicolon,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^:").unwrap(),
                action: |_| TokenKind::Colon,
            },
            LexerRule {
                pattern: regex::Regex::new(r"^\.").unwrap(),
                action: |_| TokenKind::Dot,
            },
        ];

        Lexer {
            source: source.to_string(),
            rules,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// Advance position tracking through matched text.
    fn advance(&mut self, text: &str) {
        for ch in text.chars() {
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        self.pos += text.len();
    }

    /// Tokenize the entire source, returning tokens or errors.
    pub fn tokenize(&mut self) -> Result<Vec<Token>, Vec<LexerError>> {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        while self.pos < self.source.len() {
            let remainder = &self.source[self.pos..];
            let mut matched = false;

            for rule in &self.rules {
                if let Some(m) = rule.pattern.find(remainder) {
                    if m.start() != 0 {
                        continue;
                    }
                    let text = m.as_str().to_string(); // clone to own the string
                    let start = self.pos;
                    let line = self.line;
                    let col = self.col;
                    let kind = (rule.action)(&m);

                    self.advance(&text);

                    let end = self.pos;

                    match kind {
                        TokenKind::Error(ref label)
                            if label == "whitespace" || label == "comment" =>
                        {
                            // Skip whitespace and comments
                        }
                        TokenKind::Error(ref msg) => {
                            errors.push(LexerError {
                                message: msg.clone(),
                                span: Span::new(start, end, line, col),
                            });
                        }
                        _ => {
                            tokens.push(Token::new(kind, Span::new(start, end, line, col)));
                        }
                    }
                    matched = true;
                    break;
                }
            }

            if !matched {
                let ch = self.source[self.pos..].chars().next().unwrap();
                let start = self.pos;
                let line = self.line;
                let col = self.col;
                let ch_str = ch.to_string();
                errors.push(LexerError {
                    message: format!("unexpected character '{}'", ch),
                    span: Span::new(start, start + ch.len_utf8(), line, col),
                });
                self.advance(&ch_str);
            }
        }

        tokens.push(Token::eof(Span::new(self.pos, self.pos, self.line, self.col)));

        if errors.is_empty() {
            Ok(tokens)
        } else {
            Err(errors)
        }
    }
}

/// Convenience: tokenize a string source in one shot.
pub fn tokenize(source: &str) -> Result<Vec<Token>, Vec<LexerError>> {
    let mut lexer = Lexer::new(source);
    lexer.tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokenization() {
        let tokens = tokenize("let x = 42;").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Let));
        assert!(matches!(&tokens[1].kind, TokenKind::Ident(ref s) if s == "x"));
        assert!(matches!(tokens[2].kind, TokenKind::Eq));
        assert!(matches!(tokens[3].kind, TokenKind::Int(42)));
        assert!(matches!(tokens[4].kind, TokenKind::Semicolon));
        assert!(matches!(tokens[5].kind, TokenKind::Eof));
    }

    #[test]
    fn test_position_tracking() {
        let tokens = tokenize("a\nb").unwrap();
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[0].span.col, 1);
        assert_eq!(tokens[1].span.line, 2);
        assert_eq!(tokens[1].span.col, 1);
    }

    #[test]
    fn test_float_literal() {
        let tokens = tokenize("3.14").unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Float(ref s) if s == "3.14"));
    }

    #[test]
    fn test_string_literal() {
        let tokens = tokenize(r#""hello world""#).unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::String(ref s) if s == "hello world"));
    }

    #[test]
    fn test_operators() {
        let tokens = tokenize("== != <= >= && || -> =>").unwrap();
        let kinds: Vec<String> = tokens.iter().take(8).map(|t| t.kind.to_string()).collect();
        assert_eq!(kinds, vec!["==", "!=", "<=", ">=", "&&", "||", "->", "=>"]);
    }

    #[test]
    fn test_comments_skipped() {
        let tokens = tokenize("x // comment\ny").unwrap();
        let idents: Vec<&str> = tokens.iter()
            .filter_map(|t| match &t.kind {
                TokenKind::Ident(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(idents, vec!["x", "y"]);
    }

    #[test]
    fn test_keywords() {
        let tokens = tokenize("if else while for fn return").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::If));
        assert!(matches!(tokens[1].kind, TokenKind::Else));
        assert!(matches!(tokens[2].kind, TokenKind::While));
        assert!(matches!(tokens[3].kind, TokenKind::For));
        assert!(matches!(tokens[4].kind, TokenKind::Fn));
        assert!(matches!(tokens[5].kind, TokenKind::Return));
    }

    #[test]
    fn test_agent_keywords() {
        let tokens = tokenize("agent skill trigger on do").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Agent));
        assert!(matches!(tokens[1].kind, TokenKind::Skill));
        assert!(matches!(tokens[2].kind, TokenKind::Trigger));
        assert!(matches!(tokens[3].kind, TokenKind::On));
        assert!(matches!(tokens[4].kind, TokenKind::Do));
    }

    #[test]
    fn test_error_on_bad_char() {
        let result = tokenize("@");
        assert!(result.is_err());
    }

    #[test]
    fn test_bool_literals() {
        let tokens = tokenize("true false").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Bool(true)));
        assert!(matches!(tokens[1].kind, TokenKind::Bool(false)));
    }
}
