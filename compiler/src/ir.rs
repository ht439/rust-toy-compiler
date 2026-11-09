//! A small three-address intermediate representation.
//!
//! Each `IRSeq` is one instruction of the form `addr = op arguments...`.
//! Operands are stored as strings: either an integer literal ("42") or the
//! name of a variable / temporary (which the backend turns into a memory slot).

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Mov,   // addr = arg0
    Add,   // addr = arg0 + arg1
    Sub,   // addr = arg0 - arg1
    Mul,   // addr = arg0 * arg1
    Div,   // addr = arg0 / arg1
    Mod,   // addr = arg0 % arg1
    Print, // print arg0
    Label, // arg0:
    Branch,   // jmp arg0
    CBranch,  // if arg0 goto arg1 else arg2
    Call,  // addr = call arg0(args...)
    Ret,   // ret
    Alloc, // reserve storage for addr
    Store, // *arg0 = arg1
    Load,  // addr = *arg0
}

#[derive(Debug, Clone)]
pub struct IRSeq {
    pub op_code: Op,
    pub addr: Option<String>,
    pub arguments: Vec<String>,
}

impl IRSeq {
    pub fn new(op_code: Op, addr: Option<String>, arguments: Vec<String>) -> Self {
        Self { op_code, addr, arguments }
    }
}

impl fmt::Display for IRSeq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.addr {
            Some(a) => write!(f, "{} = {:?} {}", a, self.op_code, self.arguments.join(", ")),
            None => write!(f, "{:?} {}", self.op_code, self.arguments.join(", ")),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct IRBlock {
    pub label: String,
    pub code: Vec<IRSeq>,
}

impl IRBlock {
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), code: Vec::new() }
    }
}

impl fmt::Display for IRBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}:", self.label)?;
        for instr in &self.code {
            writeln!(f, "    {}", instr)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct IR {
    pub blocks: Vec<IRBlock>,
}

impl IR {
    pub fn new() -> Self {
        Self { blocks: Vec::new() }
    }
}

impl fmt::Display for IR {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for block in &self.blocks {
            write!(f, "{}", block)?;
        }
        Ok(())
    }
}
