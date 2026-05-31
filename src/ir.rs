// IR generation: three-address code

use crate::ast::*;
use crate::lexer::Span;
use crate::ssa::*;
use std::collections::HashMap;

/// IR generation error.
#[derive(Debug, Clone)]
pub struct IrError {
    pub message: String,
    pub span: Span,
}

/// The IR generator: walks AST and produces SSA IR.
pub struct IrGenerator {
    /// Generated functions.
    pub functions: Vec<SsaFunction>,
    /// Current function being compiled.
    current: Option<SsaFunction>,
    /// Symbol table for variable -> ValueId mapping.
    locals: HashMap<String, ValueId>,
    /// Label counter.
    label_counter: usize,
}

impl IrGenerator {
    pub fn new() -> Self {
        IrGenerator {
            functions: Vec::new(),
            current: None,
            locals: HashMap::new(),
            label_counter: 0,
        }
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let label = format!("{}{}", prefix, self.label_counter);
        self.label_counter += 1;
        label
    }

    fn emit(&mut self, instr: SsaInstruction) {
        if let Some(ref mut func) = self.current {
            func.push_instr(instr);
        }
    }

    fn fresh(&mut self) -> ValueId {
        self.current.as_mut().unwrap().fresh_value()
    }

    /// Generate IR for a full program.
    pub fn generate(&mut self, program: &Program) -> Result<Vec<SsaFunction>, Vec<IrError>> {
        let mut errors = Vec::new();
        for item in &program.items {
            if let Err(e) = self.gen_item(item) {
                errors.push(e);
            }
        }
        if errors.is_empty() {
            Ok(self.functions.clone())
        } else {
            Err(errors)
        }
    }

    fn gen_item(&mut self, item: &Item) -> Result<(), IrError> {
        match item {
            Item::Function(stmt) => {
                if let Stmt::FnDecl { name, params, body, .. } = stmt {
                    self.gen_function(name, params, body)
                } else {
                    Ok(())
                }
            }
            Item::Expr(expr) => {
                if self.current.is_none() {
                    let func = SsaFunction::new("__toplevel".into(), 0);
                    // Add a return block
                    self.current = Some(func);
                }
                self.gen_expr(expr)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn gen_function(&mut self, name: &str, params: &[(String, Option<crate::symbol_table::Type>)], body: &Stmt) -> Result<(), IrError> {
        let param_count = params.len();
        let mut func = SsaFunction::new(name.to_string(), param_count);
        func.add_block("body".into());
        self.current = Some(func);
        self.locals.clear();

        // Register params by name
        for (i, (pname, _)) in params.iter().enumerate() {
            self.locals.insert(pname.clone(), i);
        }

        self.gen_stmt(body)?;

        // Add implicit return if not terminated
        if let Some(ref func) = self.current {
            if let Some(block) = func.blocks.last() {
                if block.terminator().is_none() {
                    self.emit(SsaInstruction::Return(None));
                }
            }
        }

        if let Some(func) = self.current.take() {
            self.functions.push(func);
        }
        Ok(())
    }

    fn gen_stmt(&mut self, stmt: &Stmt) -> Result<(), IrError> {
        match stmt {
            Stmt::Let { name, init, .. } => {
                if let Some(expr) = init {
                    let val = self.gen_expr(expr)?;
                    self.locals.insert(name.clone(), val);
                } else {
                    let v = self.fresh();
                    self.locals.insert(name.clone(), v);
                }
                Ok(())
            }
            Stmt::Assign { target, value, span } => {
                let val = self.gen_expr(value)?;
                match target {
                    Expr::Ident(name, _) => {
                        self.locals.insert(name.clone(), val);
                        Ok(())
                    }
                    Expr::Index { object, index, .. } => {
                        let _obj = self.gen_expr(object)?;
                        let idx = self.gen_expr(index)?;
                        self.emit(SsaInstruction::Store { addr: idx, value: val });
                        Ok(())
                    }
                    _ => Err(IrError {
                        message: "invalid assignment target".into(),
                        span: span.clone(),
                    }),
                }
            }
            Stmt::ExprStmt(expr, _) => {
                self.gen_expr(expr)?;
                Ok(())
            }
            Stmt::Block(stmts, _) => {
                for s in stmts {
                    self.gen_stmt(s)?;
                }
                Ok(())
            }
            Stmt::If { condition, then_branch, else_branch, .. } => {
                let cond = self.gen_expr(condition)?;

                let then_label = self.fresh_label("then");
                let else_label = self.fresh_label("else");
                let merge_label = self.fresh_label("merge");

                let func = self.current.as_mut().unwrap();
                let current_block = func.blocks.len() - 1;

                let then_block = func.add_block(then_label);
                let else_block = func.add_block(else_label.clone());
                let merge_block = func.add_block(merge_label);

                // Fix up: add branch to current block
                func.blocks[current_block].instructions.push(SsaInstruction::Branch {
                    cond,
                    true_block: then_block,
                    false_block: else_block,
                });
                func.blocks[current_block].successors.push(then_block);
                func.blocks[current_block].successors.push(else_block);
                func.blocks[then_block].predecessors.push(current_block);
                func.blocks[else_block].predecessors.push(current_block);

                // Generate then
                self.gen_stmt(then_branch)?;
                let func = self.current.as_mut().unwrap();
                let last_then = func.blocks.len() - 1;
                if func.blocks[last_then].terminator().is_none() {
                    func.blocks[last_then].instructions.push(SsaInstruction::Jump(merge_block));
                    func.blocks[last_then].successors.push(merge_block);
                    func.blocks[merge_block].predecessors.push(last_then);
                }

                // Generate else
                if let Some(else_b) = else_branch {
                    self.gen_stmt(else_b)?;
                    let func = self.current.as_mut().unwrap();
                    let last_else = func.blocks.len() - 1;
                    if func.blocks[last_else].terminator().is_none() {
                        func.blocks[last_else].instructions.push(SsaInstruction::Jump(merge_block));
                        func.blocks[last_else].successors.push(merge_block);
                        func.blocks[merge_block].predecessors.push(last_else);
                    }
                } else {
                    let func = self.current.as_mut().unwrap();
                    func.blocks[else_block].instructions.push(SsaInstruction::Jump(merge_block));
                    func.blocks[else_block].successors.push(merge_block);
                    func.blocks[merge_block].predecessors.push(else_block);
                }

                Ok(())
            }
            Stmt::Return(value, _) => {
                let val = value.as_ref().map(|v| self.gen_expr(v)).transpose()?;
                self.emit(SsaInstruction::Return(val));
                Ok(())
            }
            Stmt::While { condition, body, .. } => {
                let header_label = self.fresh_label("while_header");
                let body_label = self.fresh_label("while_body");
                let exit_label = self.fresh_label("while_exit");

                let func = self.current.as_mut().unwrap();
                let current = func.blocks.len() - 1;
                let header = func.add_block(header_label);
                let body_block = func.add_block(body_label);
                let exit = func.add_block(exit_label);

                func.blocks[current].instructions.push(SsaInstruction::Jump(header));
                func.blocks[current].successors.push(header);
                func.blocks[header].predecessors.push(current);

                // Evaluate condition in header
                let cond = self.gen_expr(condition)?;
                let func = self.current.as_mut().unwrap();
                let header_idx = header;
                func.blocks[header_idx].instructions.push(SsaInstruction::Branch {
                    cond,
                    true_block: body_block,
                    false_block: exit,
                });
                func.blocks[header_idx].successors.push(body_block);
                func.blocks[header_idx].successors.push(exit);
                func.blocks[body_block].predecessors.push(header_idx);
                func.blocks[exit].predecessors.push(header_idx);

                self.gen_stmt(body)?;
                let func = self.current.as_mut().unwrap();
                let last_body = func.blocks.len() - 1;
                func.blocks[last_body].instructions.push(SsaInstruction::Jump(header));
                func.blocks[last_body].successors.push(header);
                func.blocks[header].predecessors.push(last_body);

                Ok(())
            }
            Stmt::FnDecl { .. } => Ok(()), // handled at item level
            _ => Ok(()),
        }
    }

    fn gen_expr(&mut self, expr: &Expr) -> Result<ValueId, IrError> {
        match expr {
            Expr::IntLiteral(v, _) => {
                let dest = self.fresh();
                self.emit(SsaInstruction::Const { dest, value: SsaValue::Int(*v) });
                Ok(dest)
            }
            Expr::FloatLiteral(v, _) => {
                let dest = self.fresh();
                self.emit(SsaInstruction::Const { dest, value: SsaValue::Float(*v) });
                Ok(dest)
            }
            Expr::BoolLiteral(b, _) => {
                let dest = self.fresh();
                self.emit(SsaInstruction::Const { dest, value: SsaValue::Bool(*b) });
                Ok(dest)
            }
            Expr::NullLiteral(_) => {
                let dest = self.fresh();
                self.emit(SsaInstruction::Const { dest, value: SsaValue::Null });
                Ok(dest)
            }
            Expr::StringLiteral(_, span) => {
                Err(IrError {
                    message: "string literals not yet supported in IR".into(),
                    span: span.clone(),
                })
            }
            Expr::Ident(name, span) => {
                self.locals.get(name).copied().ok_or_else(|| IrError {
                    message: format!("undefined variable: {}", name),
                    span: span.clone(),
                })
            }
            Expr::Binary { op, left, right, .. } => {
                let lv = self.gen_expr(left)?;
                let rv = self.gen_expr(right)?;
                let dest = self.fresh();
                let ssa_op = match op {
                    BinOp::Add => SsaBinOp::Add,
                    BinOp::Sub => SsaBinOp::Sub,
                    BinOp::Mul => SsaBinOp::Mul,
                    BinOp::Div => SsaBinOp::Div,
                    BinOp::Mod => SsaBinOp::Mod,
                    BinOp::Eq => SsaBinOp::Eq,
                    BinOp::Neq => SsaBinOp::Neq,
                    BinOp::Lt => SsaBinOp::Lt,
                    BinOp::Lte => SsaBinOp::Lte,
                    BinOp::Gt => SsaBinOp::Gt,
                    BinOp::Gte => SsaBinOp::Gte,
                    BinOp::And => SsaBinOp::And,
                    BinOp::Or => SsaBinOp::Or,
                    _ => SsaBinOp::Add, // bitops default
                };
                self.emit(SsaInstruction::BinOp { dest, op: ssa_op, left: lv, right: rv });
                Ok(dest)
            }
            Expr::Unary { op, operand, .. } => {
                let ov = self.gen_expr(operand)?;
                let dest = self.fresh();
                let ssa_op = match op {
                    UnaryOp::Neg => SsaUnaryOp::Neg,
                    UnaryOp::Not => SsaUnaryOp::Not,
                    UnaryOp::BitNot => SsaUnaryOp::Not,
                };
                self.emit(SsaInstruction::UnaryOp { dest, op: ssa_op, operand: ov });
                Ok(dest)
            }
            Expr::Call { callee, args, .. } => {
                let arg_vals: Result<Vec<ValueId>, IrError> = args.iter()
                    .map(|a| self.gen_expr(a))
                    .collect();
                let arg_vals = arg_vals?;
                let func_name = match callee.as_ref() {
                    Expr::Ident(name, _) => name.clone(),
                    _ => "<lambda>".into(),
                };
                let dest = self.fresh();
                self.emit(SsaInstruction::Call {
                    dest: Some(dest),
                    func: func_name,
                    args: arg_vals,
                });
                Ok(dest)
            }
            Expr::Array(elems, _) => {
                // Allocate array by generating each element
                let mut last = self.fresh();
                self.emit(SsaInstruction::Const { dest: last, value: SsaValue::Int(elems.len() as i64) });
                for e in elems {
                    let v = self.gen_expr(e)?;
                    self.emit(SsaInstruction::Store { addr: last, value: v });
                    last = v;
                }
                Ok(last)
            }
            _ => {
                let dest = self.fresh();
                self.emit(SsaInstruction::Const { dest, value: SsaValue::Null });
                Ok(dest)
            }
        }
    }
}

impl Default for IrGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn test_simple_ir_generation() {
        let prog = parser::parse("fn add(a, b) { return a + b; }").unwrap();
        let mut gen = IrGenerator::new();
        let funcs = gen.generate(&prog).unwrap();
        assert_eq!(funcs.len(), 1);
        assert_eq!(funcs[0].name, "add");
        // Should have: param0, param1, fresh(2) for a+b, return
        assert!(funcs[0].blocks.iter().any(|b| b.instructions.iter().any(|i| matches!(i, SsaInstruction::BinOp { .. }))));
    }

    #[test]
    fn test_const_fold_candidate() {
        let prog = parser::parse("fn f() { let x = 3 + 4; return x; }").unwrap();
        let mut gen = IrGenerator::new();
        let funcs = gen.generate(&prog).unwrap();
        // Should have a const 3, const 4, and an add
        let instrs: Vec<&SsaInstruction> = funcs[0].blocks.iter()
            .flat_map(|b| &b.instructions)
            .collect();
        let consts: Vec<&SsaInstruction> = instrs.iter()
            .filter(|i| matches!(i, SsaInstruction::Const { .. }))
            .copied()
            .collect();
        assert!(consts.len() >= 2);
    }

    #[test]
    fn test_if_generates_branch() {
        let prog = parser::parse("fn f(x) { if x { return 1; } else { return 0; } }").unwrap();
        let mut gen = IrGenerator::new();
        let funcs = gen.generate(&prog).unwrap();
        let has_branch = funcs[0].blocks.iter().any(|b| b.instructions.iter().any(|i| matches!(i, SsaInstruction::Branch { .. })));
        assert!(has_branch);
    }

    #[test]
    fn test_while_generates_loop() {
        let prog = parser::parse("fn f(x) { while x { x = x - 1; } return x; }").unwrap();
        let mut gen = IrGenerator::new();
        let funcs = gen.generate(&prog).unwrap();
        // Should have multiple blocks for header/body/exit
        assert!(funcs[0].blocks.len() > 1);
    }

    #[test]
    fn test_call_generation() {
        let prog = parser::parse("fn main() { return add(1, 2); }").unwrap();
        let mut gen = IrGenerator::new();
        let funcs = gen.generate(&prog).unwrap();
        let has_call = funcs[0].blocks.iter().any(|b| b.instructions.iter().any(|i| matches!(i, SsaInstruction::Call { .. })));
        assert!(has_call);
    }
}
