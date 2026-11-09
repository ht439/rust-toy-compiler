//! AST -> IR lowering.
//!
//! Walks the parser's AST and produces three-address `IRSeq` instructions while
//! populating a `SymbolTable`. This skeleton supports the integer subset of the
//! language: `let` bindings, arithmetic, assignment, and `print`. Unsupported
//! constructs (functions, `while`, comparisons) return an `Err` for now so the
//! gaps are explicit rather than silently miscompiled.

use crate::ir::{IRBlock, IRSeq, Op, IR};
use crate::symbol::{SymbolTable, Type};
use parser::ast::{Declaration, Expr, Literal, OpType, Stmt, UnaryType};

pub struct Lowerer {
    pub ir: IR,
    pub symbols: SymbolTable,
    /// (label, value) pairs for string literals, emitted into `.rodata`.
    pub strings: Vec<(String, String)>,
    code: Vec<IRSeq>,
    temp_counter: usize,
    str_counter: usize,
}

impl Lowerer {
    pub fn new() -> Self {
        Self {
            ir: IR::new(),
            symbols: SymbolTable::new(),
            strings: Vec::new(),
            code: Vec::new(),
            temp_counter: 0,
            str_counter: 0,
        }
    }

    /// Lower a whole program (the top-level `Stmt::Block`) into `self.ir`.
    pub fn lower_program(&mut self, program: &Stmt) -> Result<(), String> {
        let decls = match program {
            Stmt::Block(decls) => decls,
            _ => return Err("expected a top-level block".to_string()),
        };
        for d in decls {
            self.lower_decl(d)?;
        }
        let code = std::mem::take(&mut self.code);
        self.ir.blocks.push(IRBlock { label: "main".to_string(), code });
        Ok(())
    }

    fn emit(&mut self, op: Op, addr: Option<String>, args: Vec<String>) {
        self.code.push(IRSeq::new(op, addr, args));
    }

    fn new_temp(&mut self) -> String {
        self.new_temp_typed(Type::Int32)
    }

    fn new_temp_typed(&mut self, ty: Type) -> String {
        let t = format!("t{}", self.temp_counter);
        self.temp_counter += 1;
        self.symbols.insert(&t, ty);
        t
    }

    fn is_string_const(&self, name: &str) -> bool {
        self.strings.iter().any(|(label, _)| label == name)
    }

    fn new_string(&mut self, value: &str) -> String {
        let label = format!("str{}", self.str_counter);
        self.str_counter += 1;
        self.strings.push((label.clone(), value.to_string()));
        self.symbols.insert(&label, Type::Str);
        label
    }

    fn lower_decl(&mut self, d: &Declaration) -> Result<(), String> {
        match d {
            Declaration::VarDecl(id, init) => {
                let name = match &**id {
                    Expr::Id(s) => s.to_string(),
                    _ => return Err("variable declaration target must be an identifier".to_string()),
                };
                match init {
                    Some(expr) => {
                        let (operand, ty) = self.lower_expr(expr)?;
                        self.symbols.insert(&name, ty);
                        self.emit(Op::Mov, Some(name), vec![operand]);
                    }
                    None => {
                        // Uninitialized binding: default to Int32 with no store.
                        self.symbols.insert(&name, Type::Int32);
                    }
                }
                Ok(())
            }
            Declaration::Statement(stmt) => self.lower_stmt(stmt),
            Declaration::FuncDecl(..) => {
                Err("function declarations are not supported in the skeleton yet".to_string())
            }
        }
    }

    fn lower_stmt(&mut self, s: &Stmt) -> Result<(), String> {
        match s {
            Stmt::Print(expr) => {
                // `print` needs a named location whose type we know at codegen.
                let (operand, ty) = self.lower_expr(expr)?;
                let loc = self.ensure_location(operand, ty);
                self.emit(Op::Print, None, vec![loc]);
                Ok(())
            }
            Stmt::ExprStmt(expr) => {
                self.lower_expr(expr)?;
                Ok(())
            }
            Stmt::Block(decls) => {
                for d in decls {
                    self.lower_decl(d)?;
                }
                Ok(())
            }
            Stmt::While(..) => Err("`while` is not supported in the skeleton yet".to_string()),
            Stmt::Call(..) => Err("function calls are not supported in the skeleton yet".to_string()),
            Stmt::Empty => Ok(()),
        }
    }

    /// Lower an expression, returning `(operand, type)` where `operand` is either
    /// an integer literal or the name of a variable / temporary.
    fn lower_expr(&mut self, e: &Expr) -> Result<(String, Type), String> {
        match e {
            Expr::ExprLiteral(lit) => match lit {
                // Numbers are parsed as f32; the integer skeleton truncates.
                Literal::NumLiteral(n) => Ok((format!("{}", *n as i32), Type::Int32)),
                Literal::BoolLiteral(b) => Ok(((*b as i32).to_string(), Type::Bool)),
                Literal::StringLiteral(s) => Ok((self.new_string(s), Type::Str)),
            },
            Expr::Id(name) => {
                let ty = self.symbols.get(name).map(|s| s.ty).unwrap_or(Type::Int32);
                Ok((name.to_string(), ty))
            }
            Expr::OpExpr(op, lhs, rhs) => {
                let (l, _) = self.lower_expr(lhs)?;
                let (r, _) = self.lower_expr(rhs)?;
                let temp = self.new_temp();
                let ir_op = match op {
                    OpType::Add => Op::Add,
                    OpType::Sub => Op::Sub,
                    OpType::Mul => Op::Mul,
                    OpType::Div => Op::Div,
                    OpType::Mod => Op::Mod,
                };
                self.emit(ir_op, Some(temp.clone()), vec![l, r]);
                Ok((temp, Type::Int32))
            }
            Expr::UnaryExpr(op, inner) => match op {
                UnaryType::Negative => {
                    let (v, _) = self.lower_expr(inner)?;
                    let temp = self.new_temp();
                    self.emit(Op::Sub, Some(temp.clone()), vec!["0".to_string(), v]);
                    Ok((temp, Type::Int32))
                }
                UnaryType::Not => {
                    Err("unary `!` is not supported in the skeleton yet".to_string())
                }
            },
            Expr::Assign(target, value) => {
                let name = match &**target {
                    Expr::Id(s) => s.to_string(),
                    _ => return Err("assignment target must be an identifier".to_string()),
                };
                let (v, ty) = self.lower_expr(value)?;
                if self.symbols.get(&name).is_none() {
                    self.symbols.insert(&name, ty);
                }
                self.emit(Op::Mov, Some(name.clone()), vec![v]);
                Ok((name, ty))
            }
            Expr::OrderExpr(..) => {
                Err("comparison / logical operators are not supported in the skeleton yet".to_string())
            }
        }
    }

    /// Ensure an operand refers to a named memory slot. Integer literals and raw
    /// string constants are materialized into a fresh temporary so `print` always
    /// receives a variable/temp (holding a value or a pointer), never a literal.
    fn ensure_location(&mut self, operand: String, ty: Type) -> String {
        if operand.parse::<i32>().is_ok() || self.is_string_const(&operand) {
            let temp = self.new_temp_typed(ty);
            self.emit(Op::Mov, Some(temp.clone()), vec![operand]);
            temp
        } else {
            operand
        }
    }
}

impl Default for Lowerer {
    fn default() -> Self {
        Self::new()
    }
}
