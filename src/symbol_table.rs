// Symbol table management: scoping, symbol resolution

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Symbol kinds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    Variable,
    Function,
    Parameter,
    Struct,
    Enum,
    Field,
    Module,
    Agent,
    Skill,
    Trigger,
}

/// Type representation for symbols.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Void,
    Null,
    Array(Box<Type>),
    Function(Vec<Type>, Box<Type>), // params, return
    Struct(String, Vec<(String, Type)>), // name, fields
    Named(String),
    Generic(String),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "string"),
            Type::Void => write!(f, "void"),
            Type::Null => write!(f, "null"),
            Type::Array(t) => write!(f, "[{}]", t),
            Type::Function(params, ret) => {
                let ps: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "({}) -> {}", ps.join(", "), ret)
            }
            Type::Struct(name, fields) => {
                let fs: Vec<String> = fields.iter().map(|(n, t)| format!("{}: {}", n, t)).collect();
                write!(f, "struct {} {{ {} }}", name, fs.join(", "))
            }
            Type::Named(n) => write!(f, "{}", n),
            Type::Generic(n) => write!(f, "{}", n),
        }
    }
}

/// A symbol entry in the symbol table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub ty: Option<Type>,
    pub scope_depth: usize,
    pub index: usize, // unique index within scope
}

/// A scope level in the symbol table.
#[derive(Debug, Clone)]
struct Scope {
    symbols: HashMap<String, Symbol>,
    depth: usize,
    parent: Option<usize>, // index in scopes vec
}

/// The symbol table managing nested scopes.
pub struct SymbolTable {
    scopes: Vec<Scope>,
    current: usize,
    counter: usize,
}

impl SymbolTable {
    pub fn new() -> Self {
        let global = Scope {
            symbols: HashMap::new(),
            depth: 0,
            parent: None,
        };
        SymbolTable {
            scopes: vec![global],
            current: 0,
            counter: 0,
        }
    }

    /// Enter a new scope.
    pub fn enter_scope(&mut self) {
        let depth = self.scopes[self.current].depth + 1;
        let parent = Some(self.current);
        self.scopes.push(Scope {
            symbols: HashMap::new(),
            depth,
            parent,
        });
        self.current = self.scopes.len() - 1;
    }

    /// Exit the current scope, returning to parent.
    pub fn exit_scope(&mut self) -> bool {
        if let Some(parent) = self.scopes[self.current].parent {
            self.current = parent;
            true
        } else {
            false // can't exit global scope
        }
    }

    /// Insert a symbol into the current scope.
    pub fn insert(&mut self, name: &str, kind: SymbolKind, ty: Option<Type>) -> Option<Symbol> {
        let depth = self.scopes[self.current].depth;
        let index = self.counter;
        self.counter += 1;
        let symbol = Symbol {
            name: name.to_string(),
            kind,
            ty,
            scope_depth: depth,
            index,
        };
        self.scopes[self.current].symbols.insert(name.to_string(), symbol)
    }

    /// Look up a symbol, searching from current scope outward.
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        let mut scope_idx = Some(self.current);
        while let Some(idx) = scope_idx {
            if let Some(sym) = self.scopes[idx].symbols.get(name) {
                return Some(sym);
            }
            scope_idx = self.scopes[idx].parent;
        }
        None
    }

    /// Look up a symbol mutably.
    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        let mut scope_idx = Some(self.current);
        while let Some(idx) = scope_idx {
            if self.scopes[idx].symbols.contains_key(name) {
                return self.scopes[idx].symbols.get_mut(name);
            }
            scope_idx = self.scopes[idx].parent;
        }
        None
    }

    /// Check if a symbol exists in the current scope only.
    pub fn lookup_current(&self, name: &str) -> Option<&Symbol> {
        self.scopes[self.current].symbols.get(name)
    }

    /// Get all symbols in the current scope.
    pub fn current_symbols(&self) -> Vec<&Symbol> {
        self.scopes[self.current].symbols.values().collect()
    }

    /// Current scope depth (0 = global).
    pub fn depth(&self) -> usize {
        self.scopes[self.current].depth
    }

    /// Resolve a type name to a Type if it's a known struct/enum.
    pub fn resolve_type(&self, name: &str) -> Option<Type> {
        self.lookup(name).and_then(|sym| sym.ty.clone())
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_scope() {
        let mut st = SymbolTable::new();
        st.insert("x", SymbolKind::Variable, Some(Type::Int));
        assert!(st.lookup("x").is_some());
        assert_eq!(st.lookup("x").unwrap().name, "x");
    }

    #[test]
    fn test_nested_scopes() {
        let mut st = SymbolTable::new();
        st.insert("x", SymbolKind::Variable, Some(Type::Int));
        st.enter_scope();
        st.insert("y", SymbolKind::Variable, Some(Type::Bool));
        // Can see both
        assert!(st.lookup("x").is_some());
        assert!(st.lookup("y").is_some());
        st.exit_scope();
        // y is gone
        assert!(st.lookup("x").is_some());
        assert!(st.lookup("y").is_none());
    }

    #[test]
    fn test_shadowing() {
        let mut st = SymbolTable::new();
        st.insert("x", SymbolKind::Variable, Some(Type::Int));
        st.enter_scope();
        st.insert("x", SymbolKind::Variable, Some(Type::Float));
        // Should see the inner x
        let sym = st.lookup("x").unwrap();
        assert_eq!(sym.ty, Some(Type::Float));
        st.exit_scope();
        // Now should see outer x
        let sym = st.lookup("x").unwrap();
        assert_eq!(sym.ty, Some(Type::Int));
    }

    #[test]
    fn test_scope_depth() {
        let mut st = SymbolTable::new();
        assert_eq!(st.depth(), 0);
        st.enter_scope();
        assert_eq!(st.depth(), 1);
        st.enter_scope();
        assert_eq!(st.depth(), 2);
        st.exit_scope();
        assert_eq!(st.depth(), 1);
    }

    #[test]
    fn test_lookup_current() {
        let mut st = SymbolTable::new();
        st.insert("x", SymbolKind::Variable, Some(Type::Int));
        st.enter_scope();
        assert!(st.lookup_current("x").is_none()); // not in current scope
        assert!(st.lookup("x").is_some()); // but visible via parent
    }

    #[test]
    fn test_function_symbol() {
        let mut st = SymbolTable::new();
        st.insert("add", SymbolKind::Function, Some(Type::Function(
            vec![Type::Int, Type::Int],
            Box::new(Type::Int),
        )));
        let sym = st.lookup("add").unwrap();
        assert_eq!(sym.kind, SymbolKind::Function);
    }

    #[test]
    fn test_cannot_exit_global() {
        let mut st = SymbolTable::new();
        assert!(!st.exit_scope());
    }
}
