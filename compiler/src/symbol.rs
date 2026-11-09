//! Symbol table + type information produced during lowering and consumed by
//! the code generator (e.g. to pick the right `printf` format string).

use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Int32,
    Float64,
    Str,
    Bool,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub ty: Type,
}

#[derive(Debug, Default)]
pub struct SymbolTable {
    pub table: HashMap<String, Symbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self { table: HashMap::new() }
    }

    pub fn insert(&mut self, name: &str, ty: Type) {
        self.table.insert(name.to_string(), Symbol { ty });
    }

    pub fn get(&self, name: &str) -> Option<&Symbol> {
        self.table.get(name)
    }
}

impl fmt::Display for SymbolTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut entries: Vec<(&String, &Symbol)> = self.table.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0)); // deterministic order (HashMap is not)
        for (name, sym) in entries {
            writeln!(f, "    {:<8} {:?}", name, sym.ty)?;
        }
        Ok(())
    }
}
