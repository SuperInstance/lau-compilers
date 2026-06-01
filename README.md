# lau-compilers

> Compiler construction fundamentals: lexing, parsing, type checking, SSA, IR, optimization, and agent DSL

## What This Does

Compiler construction fundamentals: lexing, parsing, type checking, SSA, IR, optimization, and agent DSL. Part of the PLATO/LAU ecosystem — a mathematically rigorous framework for building educational agents that learn, teach, and evolve.

## The Key Idea

This crate implements the core abstractions needed for its domain, with a focus on correctness, composability, and conservation guarantees. Every public type is serializable (serde), every algorithm is tested, and every invariant is verified.

## Install

```bash
cargo add lau-compilers
```

## Quick Start

See the API Reference below for complete usage. Key entry points:

```rust
use lau_compilers::*;
// See types and methods below for complete usage
```

## API Reference

```rust
pub struct PhiFunction 
pub enum SsaInstruction 
pub enum SsaValue 
pub enum SsaBinOp 
pub enum SsaUnaryOp 
pub struct BasicBlock 
    pub fn new(id: usize, label: String) -> Self 
    pub fn is_empty(&self) -> bool 
    pub fn terminator(&self) -> Option<&SsaInstruction> 
pub struct SsaFunction 
    pub fn new(name: String, param_count: usize) -> Self 
    pub fn add_block(&mut self, label: String) -> usize 
    pub fn fresh_value(&mut self) -> ValueId 
    pub fn current_block(&mut self) -> &mut BasicBlock 
    pub fn push_instr(&mut self, instr: SsaInstruction) 
    pub fn compute_dominators(&self) -> HashMap<usize, HashSet<usize>> 
    pub fn compute_dominance_frontier(&self) -> HashMap<usize, HashSet<usize>> 
    pub fn insert_phi_functions(&mut self, _var_name: &str, def_blocks: &HashSet<usize>) 
    pub fn dominates(&self, dominator: usize, block: usize) -> bool 
pub struct ParseError 
pub struct Parser 
    pub fn new(tokens: Vec<Token>) -> Self 
    pub fn parse_program(&mut self) -> ParseResult<Program> 
    pub fn parse_expression(&mut self) -> ParseResult<Expr> 
pub fn parse(source: &str) -> Result<Program, Vec<ParseError>> 
pub struct IrError 
pub struct IrGenerator 
    pub fn new() -> Self 
    pub fn generate(&mut self, program: &Program) -> Result<Vec<SsaFunction>, Vec<IrError>> 
pub struct AgentConfig 
pub struct SkillConfig 
pub struct TriggerConfig 
pub enum HandlerType 
pub enum ActionType 
pub enum ConfigValue 
pub fn compile_agent(source: &str) -> Result<AgentConfig, Vec<String>> 
pub fn compile_agent_ir(source: &str) -> Result<Vec<SsaFunction>, Vec<String>> 
pub fn compile_and_optimize(source: &str) -> Result<(AgentConfig, Vec<SsaFunction>), Vec<String>> 
pub struct OptResult 
pub struct ConstantFolding;
    pub fn optimize(func: &mut SsaFunction) -> usize 
pub struct DeadCodeElimination;
    pub fn optimize(func: &mut SsaFunction) -> usize 
pub fn optimize(func: &mut SsaFunction) -> OptResult 
pub enum BinOp 
    pub fn precedence(&self) -> u8 
    pub fn is_right_associative(&self) -> bool 
pub enum UnaryOp 
pub enum Expr 
    pub fn span(&self) -> &Span 
pub enum Stmt 
    pub fn span(&self) -> &Span 
pub enum Item 
pub struct AgentDecl 
pub struct SkillDecl 
pub struct TriggerDecl 
pub struct Program 
pub trait Visitor 
pub struct RecursiveVisitor<F1, F2, F3>
pub fn collect_idents(expr: &Expr) -> Vec<String> 
```

## How It Works

Read the source in `src/` for full implementation details. All algorithms are documented with inline comments explaining the mathematical foundations.

## The Math

This crate implements formal mathematical constructs. See the source documentation for theorem statements and proofs of correctness.

## Testing

**72 tests** covering construction, serialization, correctness properties, edge cases, and composability with other lau-* crates.

## License

MIT
