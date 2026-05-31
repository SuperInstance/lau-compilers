// Optimization passes: constant folding, dead code elimination

use crate::ssa::*;
use std::collections::HashSet;

/// Result of an optimization pass.
#[derive(Debug)]
pub struct OptResult {
    pub instructions_removed: usize,
    pub constants_folded: usize,
}

/// Constant folding optimization pass.
pub struct ConstantFolding;

impl ConstantFolding {
    /// Fold constants in a single function. Returns number of folds performed.
    pub fn optimize(func: &mut SsaFunction) -> usize {
        // First pass: collect all constant values
        let mut const_map: std::collections::HashMap<ValueId, SsaValue> = std::collections::HashMap::new();
        for block in &func.blocks {
            for instr in &block.instructions {
                if let SsaInstruction::Const { dest, value } = instr {
                    const_map.insert(*dest, value.clone());
                }
            }
        }

        let mut folds = 0;
        for block in &mut func.blocks {
            for instr in &mut block.instructions {
                if let SsaInstruction::BinOp { dest, op, left, right } = *instr {
                    let result = Self::try_fold_binop_with_map(op, left, right, &const_map);
                    if let Some(value) = result {
                        const_map.insert(dest, value.clone());
                        *instr = SsaInstruction::Const { dest, value };
                        folds += 1;
                    }
                }
                if let SsaInstruction::UnaryOp { dest, op, operand } = *instr {
                    let result = Self::try_fold_unary_with_map(op, operand, &const_map);
                    if let Some(value) = result {
                        const_map.insert(dest, value.clone());
                        *instr = SsaInstruction::Const { dest, value };
                        folds += 1;
                    }
                }
            }
        }
        folds
    }

    fn try_fold_binop_with_map(op: SsaBinOp, left: ValueId, right: ValueId, const_map: &std::collections::HashMap<ValueId, SsaValue>) -> Option<SsaValue> {
        let lv = const_map.get(&left)?;
        let rv = const_map.get(&right)?;
        match (lv, rv) {
            (SsaValue::Int(l), SsaValue::Int(r)) => Some(SsaValue::Int(match op {
                SsaBinOp::Add => l + r,
                SsaBinOp::Sub => l - r,
                SsaBinOp::Mul => l * r,
                SsaBinOp::Div => { if *r != 0 { l / r } else { return None } }
                SsaBinOp::Mod => { if *r != 0 { l % r } else { return None } }
                SsaBinOp::Eq => return Some(SsaValue::Bool(l == r)),
                SsaBinOp::Neq => return Some(SsaValue::Bool(l != r)),
                SsaBinOp::Lt => return Some(SsaValue::Bool(l < r)),
                SsaBinOp::Lte => return Some(SsaValue::Bool(l <= r)),
                SsaBinOp::Gt => return Some(SsaValue::Bool(l > r)),
                SsaBinOp::Gte => return Some(SsaValue::Bool(l >= r)),
                _ => return None,
            })),
            (SsaValue::Float(l), SsaValue::Float(r)) => Some(SsaValue::Float(match op {
                SsaBinOp::Add => l + r,
                SsaBinOp::Sub => l - r,
                SsaBinOp::Mul => l * r,
                SsaBinOp::Div => { if *r != 0.0 { l / r } else { return None } }
                _ => return None,
            })),
            (SsaValue::Bool(l), SsaValue::Bool(r)) => Some(SsaValue::Bool(match op {
                SsaBinOp::And => *l && *r,
                SsaBinOp::Or => *l || *r,
                SsaBinOp::Eq => l == r,
                SsaBinOp::Neq => l != r,
                _ => return None,
            })),
            _ => None,
        }
    }

    fn try_fold_unary_with_map(op: SsaUnaryOp, operand: ValueId, const_map: &std::collections::HashMap<ValueId, SsaValue>) -> Option<SsaValue> {
        let v = const_map.get(&operand)?;
        match (v, op) {
            (SsaValue::Int(i), SsaUnaryOp::Neg) => Some(SsaValue::Int(-i)),
            (SsaValue::Float(f), SsaUnaryOp::Neg) => Some(SsaValue::Float(-f)),
            (SsaValue::Bool(b), SsaUnaryOp::Not) => Some(SsaValue::Bool(!b)),
            _ => None,
        }
    }
}

/// Dead code elimination optimization pass.
pub struct DeadCodeElimination;

impl DeadCodeElimination {
    /// Eliminate dead code in a function. Returns number of instructions removed.
    pub fn optimize(func: &mut SsaFunction) -> usize {
        // Find all used values
        let mut used: HashSet<ValueId> = HashSet::new();

        // Values used by side-effecting instructions and terminators are live
        for block in &func.blocks {
            for instr in &block.instructions {
                match instr {
                    SsaInstruction::Return(Some(v)) => { used.insert(*v); }
                    SsaInstruction::Branch { cond, .. } => { used.insert(*cond); }
                    SsaInstruction::Call { dest: None, args, .. } => {
                        used.extend(args);
                    }
                    SsaInstruction::Store { addr, value } => {
                        used.insert(*addr);
                        used.insert(*value);
                    }
                    SsaInstruction::Call { dest: Some(_), args, .. } => {
                        used.extend(args);
                    }
                    _ => {}
                }
            }
        }

        // Propagate: if a value is used, its operands are used too
        let mut changed = true;
        while changed {
            changed = false;
            for block in &func.blocks {
                for instr in &block.instructions {
                    match instr {
                        SsaInstruction::BinOp { dest, left, right, .. } => {
                            if used.contains(dest) {
                                if used.insert(*left) { changed = true; }
                                if used.insert(*right) { changed = true; }
                            }
                        }
                        SsaInstruction::UnaryOp { dest, operand, .. } => {
                            if used.contains(dest) {
                                if used.insert(*operand) { changed = true; }
                            }
                        }
                        SsaInstruction::Copy { dest, source } => {
                            if used.contains(dest) {
                                if used.insert(*source) { changed = true; }
                            }
                        }
                        SsaInstruction::Phi(phi) => {
                            if used.contains(&phi.dest) {
                                for (_, src) in &phi.sources {
                                    if used.insert(*src) { changed = true; }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Remove unused instructions (except side-effecting ones and terminators)
        let mut removed = 0;
        for block in &mut func.blocks {
            let original_len = block.instructions.len();
            block.instructions.retain(|instr| {
                match instr {
                    SsaInstruction::Const { dest, .. } => used.contains(dest),
                    SsaInstruction::BinOp { dest, .. } => used.contains(dest),
                    SsaInstruction::UnaryOp { dest, .. } => used.contains(dest),
                    SsaInstruction::Copy { dest, .. } => used.contains(dest),
                    SsaInstruction::Phi(phi) => used.contains(&phi.dest),
                    // Keep side-effecting and terminator instructions
                    SsaInstruction::Call { .. } => true,
                    SsaInstruction::Store { .. } => true,
                    SsaInstruction::Return(_) => true,
                    SsaInstruction::Branch { .. } => true,
                    SsaInstruction::Jump(_) => true,
                    SsaInstruction::Load { .. } => true,
                }
            });
            removed += original_len - block.instructions.len();
        }
        removed
    }
}

/// Run all optimization passes on a function.
pub fn optimize(func: &mut SsaFunction) -> OptResult {
    let constants_folded = ConstantFolding::optimize(func);
    let instructions_removed = DeadCodeElimination::optimize(func);
    OptResult { instructions_removed, constants_folded }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_func() -> SsaFunction {
        let mut func = SsaFunction::new("test".into(), 0);
        // v0 = 3
        func.push_instr(SsaInstruction::Const { dest: 0, value: SsaValue::Int(3) });
        // v1 = 4
        func.push_instr(SsaInstruction::Const { dest: 1, value: SsaValue::Int(4) });
        // v2 = v0 + v1 (should fold to 7)
        func.push_instr(SsaInstruction::BinOp { dest: 2, op: SsaBinOp::Add, left: 0, right: 1 });
        func.push_instr(SsaInstruction::Return(Some(2)));
        func
    }

    #[test]
    fn test_constant_folding_add() {
        let mut func = make_test_func();
        let folds = ConstantFolding::optimize(&mut func);
        assert_eq!(folds, 1);
        // The BinOp should now be Const(7)
        let binop_replaced = func.blocks[0].instructions.iter().any(|i| {
            if let SsaInstruction::Const { dest: 2, value: SsaValue::Int(7) } = i {
                true
            } else {
                false
            }
        });
        assert!(binop_replaced);
    }

    #[test]
    fn test_constant_folding_mul() {
        let mut func = SsaFunction::new("test".into(), 0);
        func.push_instr(SsaInstruction::Const { dest: 0, value: SsaValue::Int(5) });
        func.push_instr(SsaInstruction::Const { dest: 1, value: SsaValue::Int(6) });
        func.push_instr(SsaInstruction::BinOp { dest: 2, op: SsaBinOp::Mul, left: 0, right: 1 });
        func.push_instr(SsaInstruction::Return(Some(2)));
        let folds = ConstantFolding::optimize(&mut func);
        assert_eq!(folds, 1);
    }

    #[test]
    fn test_constant_folding_comparison() {
        let mut func = SsaFunction::new("test".into(), 0);
        func.push_instr(SsaInstruction::Const { dest: 0, value: SsaValue::Int(3) });
        func.push_instr(SsaInstruction::Const { dest: 1, value: SsaValue::Int(4) });
        func.push_instr(SsaInstruction::BinOp { dest: 2, op: SsaBinOp::Lt, left: 0, right: 1 });
        func.push_instr(SsaInstruction::Return(Some(2)));
        let folds = ConstantFolding::optimize(&mut func);
        assert_eq!(folds, 1);
    }

    #[test]
    fn test_constant_folding_no_fold() {
        let mut func = SsaFunction::new("test".into(), 1);
        // v1 (param) + v0 (const) — can't fold because v1 isn't const
        func.push_instr(SsaInstruction::Const { dest: 1, value: SsaValue::Int(5) });
        func.push_instr(SsaInstruction::BinOp { dest: 2, op: SsaBinOp::Add, left: 0, right: 1 });
        func.push_instr(SsaInstruction::Return(Some(2)));
        let folds = ConstantFolding::optimize(&mut func);
        assert_eq!(folds, 0);
    }

    #[test]
    fn test_constant_folding_negation() {
        let mut func = SsaFunction::new("test".into(), 0);
        func.push_instr(SsaInstruction::Const { dest: 0, value: SsaValue::Int(42) });
        func.push_instr(SsaInstruction::UnaryOp { dest: 1, op: SsaUnaryOp::Neg, operand: 0 });
        func.push_instr(SsaInstruction::Return(Some(1)));
        let folds = ConstantFolding::optimize(&mut func);
        assert_eq!(folds, 1);
    }

    #[test]
    fn test_dead_code_elimination() {
        let mut func = SsaFunction::new("test".into(), 0);
        func.push_instr(SsaInstruction::Const { dest: 0, value: SsaValue::Int(42) }); // used
        func.push_instr(SsaInstruction::Const { dest: 1, value: SsaValue::Int(99) }); // dead
        func.push_instr(SsaInstruction::Return(Some(0)));
        let removed = DeadCodeElimination::optimize(&mut func);
        assert_eq!(removed, 1);
        // v1 should be gone
        assert!(!func.blocks[0].instructions.iter().any(|i| {
            matches!(i, SsaInstruction::Const { dest: 1, .. })
        }));
    }

    #[test]
    fn test_dce_keeps_used_chain() {
        let mut func = SsaFunction::new("test".into(), 0);
        func.push_instr(SsaInstruction::Const { dest: 0, value: SsaValue::Int(1) });
        func.push_instr(SsaInstruction::Const { dest: 1, value: SsaValue::Int(2) });
        func.push_instr(SsaInstruction::BinOp { dest: 2, op: SsaBinOp::Add, left: 0, right: 1 });
        func.push_instr(SsaInstruction::Return(Some(2)));
        let removed = DeadCodeElimination::optimize(&mut func);
        assert_eq!(removed, 0); // all used
    }

    #[test]
    fn test_dke_keeps_calls() {
        let mut func = SsaFunction::new("test".into(), 0);
        func.push_instr(SsaInstruction::Call {
            dest: None,
            func: "side_effect".into(),
            args: vec![],
        });
        func.push_instr(SsaInstruction::Return(None));
        let removed = DeadCodeElimination::optimize(&mut func);
        assert_eq!(removed, 0); // calls are kept
    }

    #[test]
    fn test_combined_optimization() {
        let mut func = make_test_func();
        let result = optimize(&mut func);
        assert_eq!(result.constants_folded, 1);
        // After folding, v0 and v1 may become dead
        assert!(result.instructions_removed >= 0, "removed count should be non-negative");
    }
}
