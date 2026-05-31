// Agent DSL: parsing and compiling domain-specific agent configuration languages

use crate::ast::*;
use crate::lexer;
use crate::parser::Parser;
use crate::ssa::*;
use crate::ir::IrGenerator;
use crate::optimization;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Agent configuration compiled from DSL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub skills: Vec<SkillConfig>,
    pub triggers: Vec<TriggerConfig>,
    pub config: HashMap<String, ConfigValue>,
}

/// A compiled skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    pub name: String,
    pub handler_type: HandlerType,
    pub timeout_ms: Option<u64>,
    pub retry_count: Option<u32>,
    pub config: HashMap<String, ConfigValue>,
}

/// A compiled trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    pub event: String,
    pub guard: Option<String>, // simplified guard expression as string
    pub action_type: ActionType,
}

/// Handler types for skills.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandlerType {
    Inline(String),
    Function(String),
    Pipeline(Vec<String>),
}

/// Action types for triggers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    Emit(String),
    Call(String),
    Sequence(Vec<ActionType>),
}

/// Configuration values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConfigValue {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Null,
    Array(Vec<ConfigValue>),
    Object(HashMap<String, ConfigValue>),
}

impl fmt::Display for ConfigValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigValue::Int(i) => write!(f, "{}", i),
            ConfigValue::Float(fl) => write!(f, "{:.6}", fl),
            ConfigValue::String(s) => write!(f, "\"{}\"", s),
            ConfigValue::Bool(b) => write!(f, "{}", b),
            ConfigValue::Null => write!(f, "null"),
            ConfigValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", items.join(", "))
            }
            ConfigValue::Object(map) => {
                let items: Vec<String> = map.iter().map(|(k, v)| format!("{}: {}", k, v)).collect();
                write!(f, "{{ {} }}", items.join(", "))
            }
        }
    }
}

/// Compile an agent DSL source into AgentConfig.
pub fn compile_agent(source: &str) -> Result<AgentConfig, Vec<String>> {
    let tokens = lexer::tokenize(source).map_err(|errs| {
        errs.iter().map(|e| format!("lexer error: {}", e)).collect::<Vec<_>>()
    })?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().map_err(|e| vec![format!("{}", e)])?;

    let mut agent_config = None;
    for item in &program.items {
        if let Item::Agent(agent) = item {
            let config = AgentConfig {
                name: agent.name.clone(),
                skills: agent.skills.iter().map(|s| SkillConfig {
                    name: s.name.clone(),
                    handler_type: expr_to_handler(&s.handler),
                    timeout_ms: s.config.iter()
                        .find(|(k, _)| k == "timeout")
                        .and_then(|(_, v)| expr_to_int(v).map(|i| i as u64)),
                    retry_count: s.config.iter()
                        .find(|(k, _)| k == "retry")
                        .and_then(|(_, v)| expr_to_int(v).map(|i| i as u32)),
                    config: s.config.iter()
                        .map(|(k, v)| (k.clone(), expr_to_config(v)))
                        .collect(),
                }).collect(),
                triggers: agent.triggers.iter().map(|t| TriggerConfig {
                    event: t.event.clone(),
                    guard: t.guard.as_ref().map(|g| format!("{:?}", g)),
                    action_type: expr_to_action(&t.action),
                }).collect(),
                config: agent.config.iter()
                    .map(|(k, v)| (k.clone(), expr_to_config(v)))
                    .collect(),
            };
            agent_config = Some(config);
            break;
        }
    }

    agent_config.ok_or_else(|| vec!["no agent declaration found".into()])
}

fn expr_to_handler(expr: &Expr) -> HandlerType {
    match expr {
        Expr::StringLiteral(s, _) => HandlerType::Inline(s.clone()),
        Expr::Ident(name, _) => HandlerType::Function(name.clone()),
        Expr::Array(elems, _) => {
            let names: Vec<String> = elems.iter()
                .filter_map(|e| match e {
                    Expr::Ident(s, _) => Some(s.clone()),
                    Expr::StringLiteral(s, _) => Some(s.clone()),
                    _ => None,
                })
                .collect();
            HandlerType::Pipeline(names)
        }
        _ => HandlerType::Inline(format!("{:?}", expr)),
    }
}

fn expr_to_action(expr: &Expr) -> ActionType {
    match expr {
        Expr::Call { callee, args, .. } => {
            if let Expr::Ident(name, _) = callee.as_ref() {
                if name == "emit" {
                    if let Some(Expr::StringLiteral(s, _)) = args.first() {
                        return ActionType::Emit(s.clone());
                    }
                }
                return ActionType::Call(name.clone());
            }
            ActionType::Call(format!("{:?}", expr))
        }
        Expr::Ident(name, _) => ActionType::Call(name.clone()),
        _ => ActionType::Call(format!("{:?}", expr)),
    }
}

fn expr_to_int(expr: &Expr) -> Option<i64> {
    match expr {
        Expr::IntLiteral(i, _) => Some(*i),
        _ => None,
    }
}

fn expr_to_config(expr: &Expr) -> ConfigValue {
    match expr {
        Expr::IntLiteral(i, _) => ConfigValue::Int(*i),
        Expr::FloatLiteral(f, _) => ConfigValue::Float(*f),
        Expr::StringLiteral(s, _) => ConfigValue::String(s.clone()),
        Expr::BoolLiteral(b, _) => ConfigValue::Bool(*b),
        Expr::NullLiteral(_) => ConfigValue::Null,
        Expr::Array(elems, _) => ConfigValue::Array(elems.iter().map(expr_to_config).collect()),
        _ => ConfigValue::Null,
    }
}

/// Compile an agent DSL source into SSA IR.
pub fn compile_agent_ir(source: &str) -> Result<Vec<SsaFunction>, Vec<String>> {
    let tokens = lexer::tokenize(source).map_err(|errs| {
        errs.iter().map(|e| format!("{}", e)).collect::<Vec<_>>()
    })?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program().map_err(|e| vec![format!("{}", e)])?;

    let mut gen = IrGenerator::new();
    gen.generate(&program).map_err(|errs| {
        errs.iter().map(|e| format!("IR error: {}", e.message)).collect()
    })
}

/// Compile and optimize agent DSL.
pub fn compile_and_optimize(source: &str) -> Result<(AgentConfig, Vec<SsaFunction>), Vec<String>> {
    let config = compile_agent(source)?;
    let mut funcs = compile_agent_ir(source)?;

    for func in &mut funcs {
        optimization::optimize(func);
    }

    Ok((config, funcs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_agent() {
        let source = r#"
            agent Assistant {
                skill greet {
                    do "Hello, how can I help?"
                    timeout: 5000
                }
                on message do emit("response")
            }
        "#;
        let config = compile_agent(source).unwrap();
        assert_eq!(config.name, "Assistant");
        assert_eq!(config.skills.len(), 1);
        assert_eq!(config.skills[0].name, "greet");
        assert_eq!(config.skills[0].timeout_ms, Some(5000));
        assert_eq!(config.triggers.len(), 1);
    }

    #[test]
    fn test_agent_config_values() {
        let source = r#"
            agent Bot {
                retries: 3
                skill respond {
                    do "OK"
                    retry: 2
                }
            }
        "#;
        let config = compile_agent(source).unwrap();
        assert_eq!(config.config.get("retries"), Some(&ConfigValue::Int(3)));
        assert_eq!(config.skills[0].retry_count, Some(2));
    }

    #[test]
    fn test_agent_pipeline_handler() {
        let source = r#"
            agent Worker {
                skill process {
                    do [parse, transform, store]
                }
            }
        "#;
        let config = compile_agent(source).unwrap();
        match &config.skills[0].handler_type {
            HandlerType::Pipeline(steps) => {
                assert_eq!(steps, &vec!["parse", "transform", "store"]);
            }
            other => panic!("expected pipeline, got {:?}", other),
        }
    }

    #[test]
    fn test_agent_trigger_action() {
        let source = r#"
            agent Bot {
                on message do emit("reply")
            }
        "#;
        let config = compile_agent(source).unwrap();
        match &config.triggers[0].action_type {
            ActionType::Emit(s) => assert_eq!(s, "reply"),
            other => panic!("expected emit, got {:?}", other),
        }
    }

    #[test]
    fn test_config_display() {
        let cv = ConfigValue::Array(vec![
            ConfigValue::Int(1),
            ConfigValue::String("hello".into()),
        ]);
        let s = cv.to_string();
        assert!(s.contains("1"));
        assert!(s.contains("hello"));
    }

    #[test]
    fn test_no_agent_error() {
        let result = compile_agent("42");
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].contains("no agent"));
    }

    #[test]
    fn test_agent_with_guard() {
        let source = r#"
            agent Guarded {
                on message guard true do emit("filtered")
            }
        "#;
        let config = compile_agent(source).unwrap();
        assert!(config.triggers[0].guard.is_some());
    }

    #[test]
    fn test_compile_and_optimize() {
        let source = r#"
            agent OptBot {
                skill compute {
                    do "calculate"
                    timeout: 1000
                }
                on request do emit("result")
            }
        "#;
        let result = compile_and_optimize(source);
        assert!(result.is_ok());
    }
}
