//! # lau-compilers
//!
//! Compiler construction fundamentals: lexing, parsing, type checking,
//! SSA form, IR generation, optimization passes, and agent DSL support.

pub mod lexer;
pub mod symbol_table;
pub mod ast;
pub mod parser;
pub mod type_checker;
pub mod ssa;
pub mod ir;
pub mod optimization;
pub mod agent_dsl;

pub use lexer::{Lexer, Token, TokenKind, Span, tokenize};
pub use symbol_table::{SymbolTable, Symbol, SymbolKind, Type};
pub use ast::*;
pub use parser::{Parser, ParseError, parse};
pub use type_checker::TypeChecker;
pub use ssa::{SsaFunction, BasicBlock, SsaInstruction, SsaValue};
pub use ir::IrGenerator;
pub use optimization::{ConstantFolding, DeadCodeElimination, optimize};
pub use agent_dsl::{AgentConfig, compile_agent, compile_and_optimize};
