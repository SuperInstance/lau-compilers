// Type checking: simple type inference and unification

use crate::ast::*;
use crate::symbol_table::{SymbolKind, SymbolTable, Type};
use crate::lexer::Span;
use std::collections::HashMap;

/// Type checking error.
#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub span: Span,
}

/// Result of type checking.
pub type TypeResult<T> = Result<T, TypeError>;

/// The type checker with inference and unification.
pub struct TypeChecker {
    symbols: SymbolTable,
    errors: Vec<TypeError>,
    /// For unification: maps type variables to concrete types.
    type_vars: HashMap<String, Type>,
    var_counter: usize,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            symbols: SymbolTable::new(),
            errors: Vec::new(),
            type_vars: HashMap::new(),
            var_counter: 0,
        }
    }

    fn fresh_type_var(&mut self) -> Type {
        let name = format!("$t{}", self.var_counter);
        self.var_counter += 1;
        Type::Generic(name)
    }

    /// Check a full program.
    pub fn check_program(&mut self, program: &Program) -> Result<Vec<Type>, Vec<TypeError>> {
        let mut types = Vec::new();
        for item in &program.items {
            match item {
                Item::Function(stmt) => {
                    if let Stmt::FnDecl { name, params, return_type, .. } = stmt {
                        let param_types: Vec<Type> = params.iter()
                            .map(|(_, ty)| ty.clone().unwrap_or_else(|| Type::Named("unknown".into())))
                            .collect();
                        let ret = return_type.clone().unwrap_or(Type::Void);
                        self.symbols.insert(name, SymbolKind::Function, Some(Type::Function(param_types, Box::new(ret))));
                    }
                }
                Item::Struct(stmt) => {
                    if let Stmt::StructDecl { name, fields, .. } = stmt {
                        self.symbols.insert(name, SymbolKind::Struct, Some(Type::Struct(name.clone(), fields.clone())));
                    }
                }
                _ => {}
            }
        }

        for item in &program.items {
            match self.check_item(item) {
                Ok(ty) => types.push(ty),
                Err(e) => self.errors.push(e),
            }
        }

        if self.errors.is_empty() {
            Ok(types)
        } else {
            Err(self.errors.clone())
        }
    }

    fn check_item(&mut self, item: &Item) -> TypeResult<Type> {
        match item {
            Item::Function(stmt) => self.check_stmt(stmt),
            Item::Struct(stmt) => self.check_stmt(stmt),
            Item::Expr(expr) => self.infer_expr(expr),
            Item::Agent(agent) => {
                // Register agent and check its internals
                self.symbols.insert(&agent.name, SymbolKind::Agent, Some(Type::Named("Agent".into())));
                Ok(Type::Named("Agent".into()))
            }
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> TypeResult<Type> {
        match stmt {
            Stmt::Let { name, ty, init, .. } => {
                if let Some(init_expr) = init {
                    let init_type = self.infer_expr(init_expr)?;
                    if let Some(declared) = ty {
                        if !self.types_compatible(declared, &init_type) {
                            return Err(TypeError {
                                message: format!("type mismatch: expected {}, got {}", declared, init_type),
                                span: init_expr.span().clone(),
                            });
                        }
                    }
                    self.symbols.insert(name, SymbolKind::Variable, Some(init_type.clone()));
                    Ok(init_type)
                } else if let Some(declared) = ty {
                    self.symbols.insert(name, SymbolKind::Variable, Some(declared.clone()));
                    Ok(declared.clone())
                } else {
                    let tv = self.fresh_type_var();
                    self.symbols.insert(name, SymbolKind::Variable, Some(tv.clone()));
                    Ok(tv)
                }
            }
            Stmt::Assign { target, value, .. } => {
                let target_type = self.infer_expr(target)?;
                let value_type = self.infer_expr(value)?;
                if !self.types_compatible(&target_type, &value_type) {
                    return Err(TypeError {
                        message: format!("cannot assign {} to {}", value_type, target_type),
                        span: target.span().clone(),
                    });
                }
                Ok(target_type)
            }
            Stmt::ExprStmt(expr, _) => self.infer_expr(expr),
            Stmt::Block(stmts, _) => {
                self.symbols.enter_scope();
                let mut last_type = Type::Void;
                for s in stmts {
                    last_type = self.check_stmt(s)?;
                }
                self.symbols.exit_scope();
                Ok(last_type)
            }
            Stmt::If { condition, then_branch, else_branch, .. } => {
                let cond_type = self.infer_expr(condition)?;
                if cond_type != Type::Bool {
                    return Err(TypeError {
                        message: format!("if condition must be bool, got {}", cond_type),
                        span: condition.span().clone(),
                    });
                }
                let then_type = self.check_stmt(then_branch)?;
                if let Some(else_b) = else_branch {
                    let else_type = self.check_stmt(else_b)?;
                    if then_type != else_type {
                        return Err(TypeError {
                            message: format!("if branches have different types: {} vs {}", then_type, else_type),
                            span: condition.span().clone(),
                        });
                    }
                }
                Ok(then_type)
            }
            Stmt::While { condition, body, .. } => {
                let cond_type = self.infer_expr(condition)?;
                if cond_type != Type::Bool {
                    return Err(TypeError {
                        message: format!("while condition must be bool, got {}", cond_type),
                        span: condition.span().clone(),
                    });
                }
                self.check_stmt(body)
            }
            Stmt::For { var, iterable, body, .. } => {
                let iter_type = self.infer_expr(iterable)?;
                let elem_type = match &iter_type {
                    Type::Array(t) => *t.clone(),
                    _ => Type::Named("unknown".into()),
                };
                self.symbols.enter_scope();
                self.symbols.insert(var, SymbolKind::Variable, Some(elem_type.clone()));
                self.check_stmt(body)?;
                self.symbols.exit_scope();
                Ok(Type::Void)
            }
            Stmt::Return(value, _) => {
                if let Some(v) = value {
                    self.infer_expr(v)
                } else {
                    Ok(Type::Void)
                }
            }
            Stmt::Break(_) | Stmt::Continue(_) => Ok(Type::Void),
            Stmt::FnDecl { name: _, params, return_type, body, .. } => {
                self.symbols.enter_scope();
                for (pname, ptype) in params {
                    let ty = ptype.clone().unwrap_or_else(|| Type::Named("unknown".into()));
                    self.symbols.insert(pname, SymbolKind::Parameter, Some(ty));
                }
                let body_type = self.check_stmt(body)?;
                self.symbols.exit_scope();
                let ret = return_type.clone().unwrap_or(body_type);
                Ok(Type::Function(
                    params.iter().map(|(_, ty)| ty.clone().unwrap_or(Type::Named("unknown".into()))).collect(),
                    Box::new(ret),
                ))
            }
            Stmt::StructDecl { name, fields, .. } => {
                Ok(Type::Struct(name.clone(), fields.clone()))
            }
        }
    }

    /// Infer the type of an expression.
    pub fn infer_expr(&mut self, expr: &Expr) -> TypeResult<Type> {
        match expr {
            Expr::IntLiteral(_, _) => Ok(Type::Int),
            Expr::FloatLiteral(_, _) => Ok(Type::Float),
            Expr::StringLiteral(_, _) => Ok(Type::String),
            Expr::BoolLiteral(_, _) => Ok(Type::Bool),
            Expr::NullLiteral(_) => Ok(Type::Null),
            Expr::Ident(name, span) => {
                self.symbols.lookup(name)
                    .map(|s| s.ty.clone().unwrap_or(Type::Named("unknown".into())))
                    .ok_or_else(|| TypeError {
                        message: format!("undefined variable: {}", name),
                        span: span.clone(),
                    })
            }
            Expr::Binary { op, left, right, span } => {
                let lt = self.infer_expr(left)?;
                let rt = self.infer_expr(right)?;
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                        if !self.is_numeric(&lt) || !self.is_numeric(&rt) {
                            return Err(TypeError {
                                message: format!("arithmetic on non-numeric types: {} and {}", lt, rt),
                                span: span.clone(),
                            });
                        }
                        if lt == Type::Float || rt == Type::Float {
                            Ok(Type::Float)
                        } else {
                            Ok(Type::Int)
                        }
                    }
                    BinOp::Eq | BinOp::Neq => {
                        if !self.types_compatible(&lt, &rt) {
                            return Err(TypeError {
                                message: format!("comparison of incompatible types: {} and {}", lt, rt),
                                span: span.clone(),
                            });
                        }
                        Ok(Type::Bool)
                    }
                    BinOp::Lt | BinOp::Lte | BinOp::Gt | BinOp::Gte => {
                        if !self.is_numeric(&lt) || !self.is_numeric(&rt) {
                            return Err(TypeError {
                                message: format!("comparison on non-numeric types: {} and {}", lt, rt),
                                span: span.clone(),
                            });
                        }
                        Ok(Type::Bool)
                    }
                    BinOp::And | BinOp::Or => {
                        if lt != Type::Bool || rt != Type::Bool {
                            return Err(TypeError {
                                message: format!("logical op requires bool, got {} and {}", lt, rt),
                                span: span.clone(),
                            });
                        }
                        Ok(Type::Bool)
                    }
                    BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                        Ok(Type::Int)
                    }
                }
            }
            Expr::Unary { op, operand, span } => {
                let ot = self.infer_expr(operand)?;
                match op {
                    UnaryOp::Neg => {
                        if !self.is_numeric(&ot) {
                            return Err(TypeError {
                                message: format!("negation of non-numeric type: {}", ot),
                                span: span.clone(),
                            });
                        }
                        Ok(ot)
                    }
                    UnaryOp::Not => {
                        if ot != Type::Bool {
                            return Err(TypeError {
                                message: format!("not on non-bool type: {}", ot),
                                span: span.clone(),
                            });
                        }
                        Ok(Type::Bool)
                    }
                    UnaryOp::BitNot => Ok(Type::Int),
                }
            }
            Expr::Call { callee, args, span } => {
                let callee_type = self.infer_expr(callee)?;
                match callee_type {
                    Type::Function(param_types, ret_type) => {
                        if args.len() != param_types.len() {
                            return Err(TypeError {
                                message: format!("expected {} args, got {}", param_types.len(), args.len()),
                                span: span.clone(),
                            });
                        }
                        for (arg, expected) in args.iter().zip(param_types.iter()) {
                            let arg_type = self.infer_expr(arg)?;
                            if !self.types_compatible(expected, &arg_type) {
                                return Err(TypeError {
                                    message: format!("argument type mismatch: expected {}, got {}", expected, arg_type),
                                    span: arg.span().clone(),
                                });
                            }
                        }
                        Ok(*ret_type)
                    }
                    _ => Err(TypeError {
                        message: format!("calling non-function type: {}", callee_type),
                        span: span.clone(),
                    }),
                }
            }
            Expr::Index { object, index, span } => {
                let obj_type = self.infer_expr(object)?;
                let idx_type = self.infer_expr(index)?;
                match &obj_type {
                    Type::Array(elem) => {
                        if idx_type != Type::Int {
                            return Err(TypeError {
                                message: format!("index must be int, got {}", idx_type),
                                span: span.clone(),
                            });
                        }
                        Ok(*elem.clone())
                    }
                    _ => Err(TypeError {
                        message: format!("indexing non-array type: {}", obj_type),
                        span: span.clone(),
                    }),
                }
            }
            Expr::Member { object, field, span } => {
                let obj_type = self.infer_expr(object)?;
                match &obj_type {
                    Type::Struct(_, fields) => {
                        fields.iter()
                            .find(|(n, _)| n == field)
                            .map(|(_, t)| t.clone())
                            .ok_or_else(|| TypeError {
                                message: format!("struct has no field '{}'", field),
                                span: span.clone(),
                            })
                    }
                    _ => Err(TypeError {
                        message: format!("member access on non-struct type: {}", obj_type),
                        span: span.clone(),
                    }),
                }
            }
            Expr::Array(elems, _) => {
                if elems.is_empty() {
                    Ok(Type::Array(Box::new(Type::Named("unknown".into()))))
                } else {
                    let elem_type = self.infer_expr(&elems[0])?;
                    for e in &elems[1..] {
                        let t = self.infer_expr(e)?;
                        if !self.types_compatible(&elem_type, &t) {
                            return Err(TypeError {
                                message: format!("array element type mismatch: {} vs {}", elem_type, t),
                                span: e.span().clone(),
                            });
                        }
                    }
                    Ok(Type::Array(Box::new(elem_type)))
                }
            }
            Expr::Cast { ty, .. } => Ok(ty.clone()),
            Expr::IfExpr { condition, then_branch, else_branch, span } => {
                let cond_type = self.infer_expr(condition)?;
                if cond_type != Type::Bool {
                    return Err(TypeError {
                        message: format!("if condition must be bool, got {}", cond_type),
                        span: span.clone(),
                    });
                }
                let then_type = self.check_stmt(then_branch)?;
                if let Some(else_b) = else_branch {
                    self.check_stmt(else_b)?;
                }
                Ok(then_type)
            }
            Expr::Lambda { params, body, .. } => {
                let param_types: Vec<Type> = params.iter()
                    .map(|(_, ty)| ty.clone().unwrap_or_else(|| Type::Named("unknown".into())))
                    .collect();
                let ret = self.infer_expr(body)?;
                Ok(Type::Function(param_types, Box::new(ret)))
            }
            Expr::Typed { ty, .. } => Ok(ty.clone()),
        }
    }

    fn is_numeric(&self, ty: &Type) -> bool {
        matches!(ty, Type::Int | Type::Float)
    }

    fn types_compatible(&mut self, expected: &Type, actual: &Type) -> bool {
        match (expected, actual) {
            (Type::Generic(a), Type::Generic(b)) => a == b,
            (Type::Generic(_), _) | (_, Type::Generic(_)) => true,
            (a, b) => a == b,
        }
    }

    /// Unify two types, updating the type variable mapping.
    pub fn unify(&mut self, t1: &Type, t2: &Type) -> TypeResult<Type> {
        match (t1, t2) {
            (Type::Generic(v), other) | (other, Type::Generic(v)) => {
                if let Some(resolved) = self.type_vars.get(v).cloned() {
                    return self.unify(&resolved, other);
                }
                let other_owned = other.clone();
                self.type_vars.insert(v.clone(), other_owned.clone());
                Ok(other_owned)
            }
            (Type::Int, Type::Int) => Ok(Type::Int),
            (Type::Float, Type::Float) => Ok(Type::Float),
            (Type::Bool, Type::Bool) => Ok(Type::Bool),
            (Type::String, Type::String) => Ok(Type::String),
            (Type::Void, Type::Void) => Ok(Type::Void),
            (Type::Array(a), Type::Array(b)) => {
                let elem = self.unify(a, b)?;
                Ok(Type::Array(Box::new(elem)))
            }
            (Type::Function(p1, r1), Type::Function(p2, r2)) => {
                if p1.len() != p2.len() {
                    return Err(TypeError {
                        message: format!("function arity mismatch: {} vs {}", p1.len(), p2.len()),
                        span: Span::zero(),
                    });
                }
                let params: Vec<Type> = p1.iter().zip(p2.iter())
                    .map(|(a, b)| self.unify(a, b))
                    .collect::<Result<_, _>>()?;
                let ret = self.unify(r1, r2)?;
                Ok(Type::Function(params, Box::new(ret)))
            }
            _ => Err(TypeError {
                message: format!("cannot unify {} with {}", t1, t2),
                span: Span::zero(),
            }),
        }
    }

    /// Resolve a type through type variable substitutions.
    pub fn resolve_type(&self, ty: &Type) -> Type {
        match ty {
            Type::Generic(v) => {
                self.type_vars.get(v)
                    .map(|t| self.resolve_type(t))
                    .unwrap_or_else(|| ty.clone())
            }
            Type::Array(t) => Type::Array(Box::new(self.resolve_type(t))),
            Type::Function(params, ret) => {
                Type::Function(
                    params.iter().map(|p| self.resolve_type(p)).collect(),
                    Box::new(self.resolve_type(ret)),
                )
            }
            other => other.clone(),
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn infer(src: &str) -> Type {
        let prog = parser::parse(src).unwrap();
        let mut tc = TypeChecker::new();
        tc.check_program(&prog).unwrap();
        // Re-infer the last expression
        let mut tc2 = TypeChecker::new();
        if let Some(Item::Expr(e)) = prog.items.last() {
            tc2.infer_expr(e).unwrap()
        } else {
            Type::Void
        }
    }

    #[test]
    fn test_infer_int() {
        assert_eq!(infer("42"), Type::Int);
    }

    #[test]
    fn test_infer_bool() {
        assert_eq!(infer("true"), Type::Bool);
    }

    #[test]
    fn test_infer_arithmetic() {
        assert_eq!(infer("1 + 2"), Type::Int);
    }

    #[test]
    fn test_infer_comparison() {
        assert_eq!(infer("1 < 2"), Type::Bool);
    }

    #[test]
    fn test_infer_equality() {
        assert_eq!(infer("1 == 2"), Type::Bool);
    }

    #[test]
    fn test_infer_string() {
        assert_eq!(infer(r#""hello""#), Type::String);
    }

    #[test]
    fn test_unify_same() {
        let mut tc = TypeChecker::new();
        let result = tc.unify(&Type::Int, &Type::Int).unwrap();
        assert_eq!(result, Type::Int);
    }

    #[test]
    fn test_unify_type_var() {
        let mut tc = TypeChecker::new();
        let tv = Type::Generic("$t0".into());
        let result = tc.unify(&tv, &Type::Int).unwrap();
        assert_eq!(result, Type::Int);
        // Now resolving the type var should give Int
        assert_eq!(tc.resolve_type(&tv), Type::Int);
    }

    #[test]
    fn test_unify_array() {
        let mut tc = TypeChecker::new();
        let a1 = Type::Array(Box::new(Type::Generic("$a".into())));
        let a2 = Type::Array(Box::new(Type::Int));
        let result = tc.unify(&a1, &a2).unwrap();
        assert_eq!(result, Type::Array(Box::new(Type::Int)));
    }

    #[test]
    fn test_unify_mismatch() {
        let mut tc = TypeChecker::new();
        assert!(tc.unify(&Type::Int, &Type::Bool).is_err());
    }

    #[test]
    fn test_type_error_undefined() {
        let mut tc = TypeChecker::new();
        let expr = Expr::Ident("undefined_var".into(), Span::zero());
        assert!(tc.infer_expr(&expr).is_err());
    }

    #[test]
    fn test_float_promotion() {
        assert_eq!(infer("1 + 2.5"), Type::Float);
    }
}
