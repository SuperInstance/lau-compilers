// SSA form: basic blocks, phi functions, dominance

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

/// SSA value (virtual register).
pub type ValueId = usize;

/// A phi function in SSA form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhiFunction {
    pub dest: ValueId,
    pub sources: Vec<(usize, ValueId)>, // (block_id, value_id)
}

/// An instruction in SSA form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SsaInstruction {
    /// Constant value: dest = const
    Const {
        dest: ValueId,
        value: SsaValue,
    },
    /// Binary operation: dest = left op right
    BinOp {
        dest: ValueId,
        op: SsaBinOp,
        left: ValueId,
        right: ValueId,
    },
    /// Unary operation: dest = op operand
    UnaryOp {
        dest: ValueId,
        op: SsaUnaryOp,
        operand: ValueId,
    },
    /// Copy: dest = source
    Copy {
        dest: ValueId,
        source: ValueId,
    },
    /// Function call: dest = call func(args...)
    Call {
        dest: Option<ValueId>,
        func: String,
        args: Vec<ValueId>,
    },
    /// Load from memory/address
    Load {
        dest: ValueId,
        addr: ValueId,
    },
    /// Store to memory/address
    Store {
        addr: ValueId,
        value: ValueId,
    },
    /// Phi function
    Phi(PhiFunction),
    /// Branch conditionally
    Branch {
        cond: ValueId,
        true_block: usize,
        false_block: usize,
    },
    /// Jump unconditionally
    Jump(usize),
    /// Return a value
    Return(Option<ValueId>),
}

/// SSA constant values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SsaValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
}

/// SSA binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SsaBinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Lte, Gt, Gte,
    And, Or,
}

/// SSA unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SsaUnaryOp {
    Neg, Not,
}

/// A basic block in SSA form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicBlock {
    pub id: usize,
    pub label: String,
    pub instructions: Vec<SsaInstruction>,
    pub predecessors: Vec<usize>,
    pub successors: Vec<usize>,
}

impl BasicBlock {
    pub fn new(id: usize, label: String) -> Self {
        BasicBlock {
            id,
            label,
            instructions: Vec::new(),
            predecessors: Vec::new(),
            successors: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    pub fn terminator(&self) -> Option<&SsaInstruction> {
        self.instructions.last()
    }
}

/// A function in SSA form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsaFunction {
    pub name: String,
    pub params: Vec<ValueId>,
    pub blocks: Vec<BasicBlock>,
    pub entry: usize,
    pub next_value: ValueId,
}

impl SsaFunction {
    pub fn new(name: String, param_count: usize) -> Self {
        let mut func = SsaFunction {
            name,
            params: (0..param_count).collect(),
            blocks: Vec::new(),
            entry: 0,
            next_value: param_count,
        };
        func.add_block("entry".into());
        func
    }

    pub fn add_block(&mut self, label: String) -> usize {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock::new(id, label));
        id
    }

    pub fn fresh_value(&mut self) -> ValueId {
        let v = self.next_value;
        self.next_value += 1;
        v
    }

    pub fn current_block(&mut self) -> &mut BasicBlock {
        self.blocks.last_mut().unwrap()
    }

    pub fn push_instr(&mut self, instr: SsaInstruction) {
        self.current_block().instructions.push(instr);
    }

    /// Build dominance tree using iterative algorithm.
    pub fn compute_dominators(&self) -> HashMap<usize, HashSet<usize>> {
        let mut dom: HashMap<usize, HashSet<usize>> = HashMap::new();
        if self.blocks.is_empty() {
            return dom;
        }

        let all_blocks: HashSet<usize> = self.blocks.iter().map(|b| b.id).collect();
        let entry = self.entry;

        // Initialize: entry dominates only itself; all others dominated by all
        dom.insert(entry, vec![entry].into_iter().collect());
        for block in &self.blocks {
            if block.id != entry {
                dom.insert(block.id, all_blocks.clone());
            }
        }

        // Iterate until fixed point
        loop {
            let mut changed = false;
            for block in &self.blocks {
                if block.id == entry {
                    continue;
                }
                let preds: Vec<usize> = block.predecessors.iter()
                    .filter(|p| dom.contains_key(p))
                    .copied()
                    .collect();
                if preds.is_empty() {
                    continue;
                }
                let mut new_dom: HashSet<usize> = dom[&preds[0]].clone();
                for p in &preds[1..] {
                    new_dom = new_dom.intersection(&dom[p]).copied().collect();
                }
                new_dom.insert(block.id);
                if new_dom != dom[&block.id] {
                    dom.insert(block.id, new_dom);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        dom
    }

    /// Compute the dominance frontier for each block.
    pub fn compute_dominance_frontier(&self) -> HashMap<usize, HashSet<usize>> {
        let dom = self.compute_dominators();
        let mut df: HashMap<usize, HashSet<usize>> = HashMap::new();

        for block in &self.blocks {
            df.insert(block.id, HashSet::new());
        }

        for block in &self.blocks {
            if block.predecessors.len() >= 2 {
                for pred in &block.predecessors {
                    let mut runner = *pred;
                    loop {
                        if !dom[&block.id].contains(&runner) {
                            df.get_mut(&runner).unwrap().insert(block.id);
                            // Move to immediate dominator
                            let runner_dom = &dom[&runner];
                            let idom_candidates: Vec<usize> = runner_dom.iter()
                                .filter(|d| **d != runner)
                                .copied()
                                .collect();
                            if idom_candidates.is_empty() {
                                break;
                            }
                            // Pick the one with the largest dom set (most specific dominator)
                            runner = *idom_candidates.iter()
                                .max_by_key(|c| dom[c].len())
                                .unwrap();
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        df
    }

    /// Insert phi functions for a variable using dominance frontiers.
    pub fn insert_phi_functions(&mut self, _var_name: &str, def_blocks: &HashSet<usize>) {
        let df = self.compute_dominance_frontier();
        let mut worklist: VecDeque<usize> = def_blocks.iter().copied().collect();
        let mut visited: HashSet<usize> = HashSet::new();
        let mut placed: HashSet<usize> = HashSet::new();

        while let Some(block_id) = worklist.pop_front() {
            if let Some(frontier) = df.get(&block_id) {
                for &fid in frontier {
                    if !placed.contains(&fid) {
                        placed.insert(fid);
                        let dest = self.fresh_value();
                        // Create phi with sources from all predecessors
                        let sources: Vec<(usize, ValueId)> = self.blocks[fid].predecessors.iter()
                            .map(|&pred| (pred, dest)) // placeholder; real register mapping needed
                            .collect();
                        let phi = PhiFunction { dest, sources };
                        // Insert at beginning of block
                        self.blocks[fid].instructions.insert(0, SsaInstruction::Phi(phi));
                    }
                    if !visited.contains(&fid) {
                        visited.insert(fid);
                        worklist.push_back(fid);
                    }
                }
            }
        }
    }

    /// Check if block `potential_dominator` dominates `block`.
    pub fn dominates(&self, dominator: usize, block: usize) -> bool {
        let dom = self.compute_dominators();
        dom.get(&block).map_or(false, |d| d.contains(&dominator))
    }
}

impl fmt::Display for SsaFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "fn {}({}):", self.name, self.params.len())?;
        for block in &self.blocks {
            writeln!(f, "  {}:", block.label)?;
            for instr in &block.instructions {
                writeln!(f, "    {}", instr)?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for SsaInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SsaInstruction::Const { dest, value } => write!(f, "v{} = {}", dest, value),
            SsaInstruction::BinOp { dest, op, left, right } => {
                write!(f, "v{} = v{} {:?} v{}", dest, left, op, right)
            }
            SsaInstruction::UnaryOp { dest, op, operand } => {
                write!(f, "v{} = {:?} v{}", dest, op, operand)
            }
            SsaInstruction::Copy { dest, source } => write!(f, "v{} = v{}", dest, source),
            SsaInstruction::Call { dest, func, args } => {
                let args_str: Vec<String> = args.iter().map(|a| format!("v{}", a)).collect();
                if let Some(d) = dest {
                    write!(f, "v{} = {}({})", d, func, args_str.join(", "))
                } else {
                    write!(f, "call {}({})", func, args_str.join(", "))
                }
            }
            SsaInstruction::Load { dest, addr } => write!(f, "v{} = load v{}", dest, addr),
            SsaInstruction::Store { addr, value } => write!(f, "store v{}, v{}", addr, value),
            SsaInstruction::Phi(phi) => {
                let srcs: Vec<String> = phi.sources.iter()
                    .map(|(b, v)| format!("bb{}:v{}", b, v))
                    .collect();
                write!(f, "v{} = phi({})", phi.dest, srcs.join(", "))
            }
            SsaInstruction::Branch { cond, true_block, false_block } => {
                write!(f, "branch v{}, bb{}, bb{}", cond, true_block, false_block)
            }
            SsaInstruction::Jump(block) => write!(f, "jump bb{}", block),
            SsaInstruction::Return(Some(v)) => write!(f, "return v{}", v),
            SsaInstruction::Return(None) => write!(f, "return"),
        }
    }
}

impl fmt::Display for SsaValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SsaValue::Int(i) => write!(f, "{}", i),
            SsaValue::Float(fl) => write!(f, "{:.6}", fl),
            SsaValue::Bool(b) => write!(f, "{}", b),
            SsaValue::Null => write!(f, "null"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_block_creation() {
        let mut func = SsaFunction::new("test".into(), 0);
        let v = func.fresh_value();
        func.push_instr(SsaInstruction::Const { dest: v, value: SsaValue::Int(42) });
        assert_eq!(func.blocks.len(), 1);
        assert_eq!(func.blocks[0].instructions.len(), 1);
    }

    #[test]
    fn test_dominators_simple() {
        let mut func = SsaFunction::new("test".into(), 0);
        let b1 = func.add_block("bb1".into());
        let b2 = func.add_block("bb2".into());
        func.blocks[b1].successors.push(b2);
        func.blocks[b2].predecessors.push(b1);

        let dom = func.compute_dominators();
        assert!(dom[&0].contains(&0));
        assert!(dom[&1].contains(&0));
        assert!(dom[&1].contains(&1));
    }

    #[test]
    fn test_dominators_diamond() {
        let mut func = SsaFunction::new("test".into(), 0);
        // entry -> bb1, bb2 -> merge
        let entry = 0; // created with new
        let bb1 = func.add_block("bb1".into());
        let bb2 = func.add_block("bb2".into());
        let merge = func.add_block("merge".into());

        func.blocks[entry].successors = vec![bb1, bb2];
        func.blocks[bb1].predecessors.push(entry);
        func.blocks[bb1].successors.push(merge);
        func.blocks[bb2].predecessors.push(entry);
        func.blocks[bb2].successors.push(merge);
        func.blocks[merge].predecessors = vec![bb1, bb2];

        let dom = func.compute_dominators();
        assert!(dom[&merge].contains(&entry));
    }

    #[test]
    fn test_dominance_frontier() {
        let mut func = SsaFunction::new("test".into(), 0);
        let bb1 = func.add_block("bb1".into());
        let bb2 = func.add_block("bb2".into());
        let merge = func.add_block("merge".into());

        func.blocks[0].successors = vec![bb1, bb2];
        func.blocks[bb1].predecessors.push(0);
        func.blocks[bb1].successors.push(merge);
        func.blocks[bb2].predecessors.push(0);
        func.blocks[bb2].successors.push(merge);
        func.blocks[merge].predecessors = vec![bb1, bb2];

        let df = func.compute_dominance_frontier();
        // merge should be in the dominance frontier of both bb1 and bb2
        assert!(df[&bb1].contains(&merge));
        assert!(df[&bb2].contains(&merge));
    }

    #[test]
    fn test_dominates() {
        let mut func = SsaFunction::new("test".into(), 0);
        let bb1 = func.add_block("bb1".into());
        func.blocks[0].successors.push(bb1);
        func.blocks[bb1].predecessors.push(0);

        assert!(func.dominates(0, bb1));
        assert!(!func.dominates(bb1, 0));
    }

    #[test]
    fn test_ssa_display() {
        let mut func = SsaFunction::new("add".into(), 2);
        let v = func.fresh_value();
        func.push_instr(SsaInstruction::BinOp {
            dest: v, op: SsaBinOp::Add, left: 0, right: 1,
        });
        func.push_instr(SsaInstruction::Return(Some(v)));
        let s = format!("{}", func);
        assert!(s.contains("Add"));
        assert!(s.contains("return"));
    }

    #[test]
    fn test_phi_insertion() {
        let mut func = SsaFunction::new("test".into(), 0);
        let bb1 = func.add_block("bb1".into());
        let merge = func.add_block("merge".into());
        func.blocks[0].successors = vec![bb1, merge];
        func.blocks[bb1].predecessors.push(0);
        func.blocks[bb1].successors.push(merge);
        func.blocks[merge].predecessors = vec![0, bb1];

        let def_blocks: HashSet<usize> = vec![0, bb1].into_iter().collect();
        func.insert_phi_functions("x", &def_blocks);
        // merge should have a phi
        let has_phi = func.blocks[merge].instructions.iter().any(|i| matches!(i, SsaInstruction::Phi(_)));
        assert!(has_phi);
    }
}
