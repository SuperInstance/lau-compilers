// AST: typed nodes with visitor pattern

use crate::lexer::{Span, TokenKind};
use crate::symbol_table::Type;
use serde::{Deserialize, Serialize};

/// AST node identifier for referencing.
pub type NodeId = usize;

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Lte, Gt, Gte,
    And, Or,
    BitAnd, BitOr, BitXor, Shl, Shr,
}

impl From<&TokenKind> for Option<BinOp> {
    fn from(kind: &TokenKind) -> Option<BinOp> {
        match kind {
            TokenKind::Plus => Some(BinOp::Add),
            TokenKind::Minus => Some(BinOp::Sub),
            TokenKind::Star => Some(BinOp::Mul),
            TokenKind::Slash => Some(BinOp::Div),
            TokenKind::Percent => Some(BinOp::Mod),
            TokenKind::EqEq => Some(BinOp::Eq),
            TokenKind::BangEq => Some(BinOp::Neq),
            TokenKind::Lt => Some(BinOp::Lt),
            TokenKind::LtEq => Some(BinOp::Lte),
            TokenKind::Gt => Some(BinOp::Gt),
            TokenKind::GtEq => Some(BinOp::Gte),
            TokenKind::And => Some(BinOp::And),
            TokenKind::Or => Some(BinOp::Or),
            TokenKind::Ampersand => Some(BinOp::BitAnd),
            TokenKind::Pipe => Some(BinOp::BitOr),
            TokenKind::Caret => Some(BinOp::BitXor),
            TokenKind::Shl => Some(BinOp::Shl),
            TokenKind::Shr => Some(BinOp::Shr),
            _ => None,
        }
    }
}

impl BinOp {
    pub fn precedence(&self) -> u8 {
        match self {
            BinOp::Or => 1,
            BinOp::And => 2,
            BinOp::BitOr => 3,
            BinOp::BitXor => 4,
            BinOp::BitAnd => 5,
            BinOp::Eq | BinOp::Neq => 6,
            BinOp::Lt | BinOp::Lte | BinOp::Gt | BinOp::Gte => 7,
            BinOp::Shl | BinOp::Shr => 8,
            BinOp::Add | BinOp::Sub => 9,
            BinOp::Mul | BinOp::Div | BinOp::Mod => 10,
        }
    }

    pub fn is_right_associative(&self) -> bool {
        false
    }
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
}

/// Expression AST nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    IntLiteral(i64, Span),
    FloatLiteral(f64, Span),
    StringLiteral(String, Span),
    BoolLiteral(bool, Span),
    NullLiteral(Span),
    Ident(String, Span),
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    Member {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    Array(Vec<Expr>, Span),
    Cast {
        expr: Box<Expr>,
        ty: Type,
        span: Span,
    },
    IfExpr {
        condition: Box<Expr>,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
        span: Span,
    },
    Lambda {
        params: Vec<(String, Option<Type>)>,
        body: Box<Expr>,
        span: Span,
    },
    Typed {
        expr: Box<Expr>,
        ty: Type,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> &Span {
        match self {
            Expr::IntLiteral(_, s) => s,
            Expr::FloatLiteral(_, s) => s,
            Expr::StringLiteral(_, s) => s,
            Expr::BoolLiteral(_, s) => s,
            Expr::NullLiteral(s) => s,
            Expr::Ident(_, s) => s,
            Expr::Binary { span, .. } => span,
            Expr::Unary { span, .. } => span,
            Expr::Call { span, .. } => span,
            Expr::Index { span, .. } => span,
            Expr::Member { span, .. } => span,
            Expr::Array(_, s) => s,
            Expr::Cast { span, .. } => span,
            Expr::IfExpr { span, .. } => span,
            Expr::Lambda { span, .. } => span,
            Expr::Typed { span, .. } => span,
        }
    }
}

/// Statement AST nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Stmt {
    Let {
        name: String,
        ty: Option<Type>,
        init: Option<Expr>,
        span: Span,
    },
    Assign {
        target: Expr,
        value: Expr,
        span: Span,
    },
    ExprStmt(Expr, Span),
    Block(Vec<Stmt>, Span),
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    For {
        var: String,
        iterable: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    Return(Option<Expr>, Span),
    Break(Span),
    Continue(Span),
    FnDecl {
        name: String,
        params: Vec<(String, Option<Type>)>,
        return_type: Option<Type>,
        body: Box<Stmt>,
        span: Span,
    },
    StructDecl {
        name: String,
        fields: Vec<(String, Type)>,
        span: Span,
    },
}

impl Stmt {
    pub fn span(&self) -> &Span {
        match self {
            Stmt::Let { span, .. } => span,
            Stmt::Assign { span, .. } => span,
            Stmt::ExprStmt(_, s) => s,
            Stmt::Block(_, s) => s,
            Stmt::If { span, .. } => span,
            Stmt::While { span, .. } => span,
            Stmt::For { span, .. } => span,
            Stmt::Return(_, s) => s,
            Stmt::Break(s) => s,
            Stmt::Continue(s) => s,
            Stmt::FnDecl { span, .. } => span,
            Stmt::StructDecl { span, .. } => span,
        }
    }
}

/// Top-level items in a module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Item {
    Function(Stmt),
    Struct(Stmt),
    Agent(AgentDecl),
    Expr(Expr),
}

/// Agent DSL declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDecl {
    pub name: String,
    pub skills: Vec<SkillDecl>,
    pub triggers: Vec<TriggerDecl>,
    pub config: Vec<(String, Expr)>,
    pub span: Span,
}

/// Agent skill declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDecl {
    pub name: String,
    pub handler: Expr,
    pub config: Vec<(String, Expr)>,
    pub span: Span,
}

/// Agent trigger declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDecl {
    pub event: String,
    pub guard: Option<Expr>,
    pub action: Expr,
    pub span: Span,
}

/// A full program / module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub items: Vec<Item>,
}

// ---- Visitor pattern ----

/// Visitor trait for walking the AST.
pub trait Visitor {
    type Result;
    fn visit_expr(&mut self, expr: &Expr) -> Self::Result;
    fn visit_stmt(&mut self, stmt: &Stmt) -> Self::Result;
    fn visit_item(&mut self, item: &Item) -> Self::Result;
}

/// A default recursive visitor that walks all nodes.
pub struct RecursiveVisitor<F1, F2, F3>
where
    F1: FnMut(&Expr),
    F2: FnMut(&Stmt),
    F3: FnMut(&Item),
{
    pub on_expr: F1,
    pub on_stmt: F2,
    pub on_item: F3,
}

impl<F1, F2, F3> Visitor for RecursiveVisitor<F1, F2, F3>
where
    F1: FnMut(&Expr),
    F2: FnMut(&Stmt),
    F3: FnMut(&Item),
{
    type Result = ();

    fn visit_expr(&mut self, expr: &Expr) {
        (self.on_expr)(expr);
        match expr {
            Expr::Binary { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            Expr::Unary { operand, .. } => {
                self.visit_expr(operand);
            }
            Expr::Call { callee, args, .. } => {
                self.visit_expr(callee);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            Expr::Index { object, index, .. } => {
                self.visit_expr(object);
                self.visit_expr(index);
            }
            Expr::Member { object, .. } => {
                self.visit_expr(object);
            }
            Expr::Array(exprs, _) => {
                for e in exprs {
                    self.visit_expr(e);
                }
            }
            Expr::Cast { expr, .. } => self.visit_expr(expr),
            Expr::IfExpr { condition, then_branch, else_branch, .. } => {
                self.visit_expr(condition);
                self.visit_stmt(then_branch);
                if let Some(eb) = else_branch {
                    self.visit_stmt(eb);
                }
            }
            Expr::Lambda { body, .. } => self.visit_expr(body),
            Expr::Typed { expr, .. } => self.visit_expr(expr),
            _ => {}
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        (self.on_stmt)(stmt);
        match stmt {
            Stmt::Let { init, .. } => {
                if let Some(e) = init {
                    self.visit_expr(e);
                }
            }
            Stmt::Assign { target, value, .. } => {
                self.visit_expr(target);
                self.visit_expr(value);
            }
            Stmt::ExprStmt(e, _) => self.visit_expr(e),
            Stmt::Block(stmts, _) => {
                for s in stmts {
                    self.visit_stmt(s);
                }
            }
            Stmt::If { condition, then_branch, else_branch, .. } => {
                self.visit_expr(condition);
                self.visit_stmt(then_branch);
                if let Some(eb) = else_branch {
                    self.visit_stmt(eb);
                }
            }
            Stmt::While { condition, body, .. } => {
                self.visit_expr(condition);
                self.visit_stmt(body);
            }
            Stmt::For { iterable, body, .. } => {
                self.visit_expr(iterable);
                self.visit_stmt(body);
            }
            Stmt::Return(Some(e), _) => self.visit_expr(e),
            Stmt::FnDecl { body, .. } => self.visit_stmt(body),
            _ => {}
        }
    }

    fn visit_item(&mut self, item: &Item) {
        (self.on_item)(item);
        match item {
            Item::Function(s) | Item::Struct(s) => self.visit_stmt(s),
            Item::Expr(e) => self.visit_expr(e),
            _ => {}
        }
    }
}

/// Collect all identifier names used in expressions.
pub fn collect_idents(expr: &Expr) -> Vec<String> {
    let mut idents = Vec::new();
    let mut visitor = RecursiveVisitor {
        on_expr: |e| {
            if let Expr::Ident(name, _) = e {
                idents.push(name.clone());
            }
        },
        on_stmt: |_| {},
        on_item: |_| {},
    };
    visitor.visit_expr(expr);
    idents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binop_precedence() {
        assert!(BinOp::Mul.precedence() > BinOp::Add.precedence());
        assert!(BinOp::And.precedence() < BinOp::Eq.precedence());
    }

    #[test]
    fn test_expr_span() {
        let span = Span::new(0, 5, 1, 1);
        let expr = Expr::IntLiteral(42, span);
        assert_eq!(expr.span(), &span);
    }

    #[test]
    fn test_collect_idents() {
        let expr = Expr::Binary {
            op: BinOp::Add,
            left: Box::new(Expr::Ident("x".into(), Span::zero())),
            right: Box::new(Expr::Ident("y".into(), Span::zero())),
            span: Span::zero(),
        };
        let idents = collect_idents(&expr);
        assert_eq!(idents, vec!["x", "y"]);
    }

    #[test]
    fn test_visitor_walks_binary() {
        let expr = Expr::Binary {
            op: BinOp::Mul,
            left: Box::new(Expr::IntLiteral(2, Span::zero())),
            right: Box::new(Expr::Binary {
                op: BinOp::Add,
                left: Box::new(Expr::Ident("x".into(), Span::zero())),
                right: Box::new(Expr::IntLiteral(3, Span::zero())),
                span: Span::zero(),
            }),
            span: Span::zero(),
        };
        let mut count = 0;
        let mut visitor = RecursiveVisitor {
            on_expr: |_| count += 1,
            on_stmt: |_| (),
            on_item: |_| (),
        };
        visitor.visit_expr(&expr);
        assert_eq!(count, 5); // outer binary, 2, inner binary, x, 3
    }
}
