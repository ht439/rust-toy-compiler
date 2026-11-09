//! 32-bit x86 (NASM syntax) code generator.
//!
//! Simple memory-backed model: every variable and temporary gets a 4-byte slot
//! in `.bss`, and each instruction loads its operands into scratch registers,
//! computes, and stores the result. This is not optimized, but it assembles and
//! runs. Link against libc for `printf`:
//!
//!     nasm -f elf32 out.asm -o out.o
//!     gcc  -m32 out.o -o prog

use crate::ir::IRBlock;
use crate::Op;
use crate::{Type, SymbolTable};
use crate::{IRSeq, IR};

pub struct X86Generator {
    asm: Vec<String>,
    symbol_table: SymbolTable,
    strings: Vec<(String, String)>,
}

impl X86Generator {
    pub fn new(symbol_table: SymbolTable, strings: Vec<(String, String)>) -> Self {
        Self { asm: Vec::new(), symbol_table, strings }
    }

    pub fn generate(&mut self, ir: &IR) -> String {
        self.asm.push("section .text".to_string());
        self.asm.push("global main".to_string());
        self.asm.push("extern printf".to_string());
        self.asm.push(String::new());

        for block in &ir.blocks {
            self.process_block(block);
        }

        // Return 0 from `main` so libc reports success.
        self.asm.push("    mov eax, 0".to_string());
        self.asm.push("    ret".to_string());

        self.generate_bss();
        self.generate_rodata();
        self.asm.join("\n") + "\n"
    }

    fn process_block(&mut self, block: &IRBlock) {
        self.asm.push(format!("{}:", block.label));
        for instr in &block.code {
            match instr.op_code {
                Op::Mov => self.handle_mov(instr),
                Op::Add => self.handle_arithmetic(instr, "add"),
                Op::Sub => self.handle_arithmetic(instr, "sub"),
                Op::Mul => self.handle_arithmetic(instr, "imul"),
                Op::Div => self.handle_divmod(instr, false),
                Op::Mod => self.handle_divmod(instr, true),
                Op::Print => self.handle_print(instr),
                other => panic!("x86 backend: unsupported operation {:?}", other),
            }
        }
    }

    fn is_string_const(&self, name: &str) -> bool {
        self.strings.iter().any(|(label, _)| label == name)
    }

    /// Turn an IR operand into an x86 operand: integer literals pass through, a
    /// string constant becomes its (immediate) address, and any other name is a
    /// memory reference.
    fn operand(&self, src: &str) -> String {
        if src.parse::<i32>().is_ok() || self.is_string_const(src) {
            src.to_string()
        } else {
            format!("[{}]", src)
        }
    }

    fn handle_mov(&mut self, instr: &IRSeq) {
        let dest = instr.addr.as_ref().expect("mov requires a destination");
        let src = self.operand(&instr.arguments[0]);
        self.asm.push(format!("    mov eax, {}", src));
        self.asm.push(format!("    mov [{}], eax", dest));
    }

    fn handle_arithmetic(&mut self, instr: &IRSeq, op: &str) {
        let dest = instr.addr.as_ref().expect("arithmetic requires a destination");
        let a = self.operand(&instr.arguments[0]);
        let b = self.operand(&instr.arguments[1]);
        self.asm.push(format!("    mov eax, {}", a));
        self.asm.push(format!("    mov ecx, {}", b));
        self.asm.push(format!("    {} eax, ecx", op));
        self.asm.push(format!("    mov [{}], eax", dest));
    }

    fn handle_divmod(&mut self, instr: &IRSeq, is_mod: bool) {
        let dest = instr.addr.as_ref().expect("div requires a destination");
        let a = self.operand(&instr.arguments[0]);
        let b = self.operand(&instr.arguments[1]);
        self.asm.push(format!("    mov eax, {}", a));
        self.asm.push("    cdq".to_string()); // sign-extend eax into edx:eax
        self.asm.push(format!("    mov ecx, {}", b));
        self.asm.push("    idiv ecx".to_string()); // quotient -> eax, remainder -> edx
        let result = if is_mod { "edx" } else { "eax" };
        self.asm.push(format!("    mov [{}], {}", dest, result));
    }

    fn handle_print(&mut self, instr: &IRSeq) {
        let var = &instr.arguments[0];
        let ty = self.symbol_table.get(var).map(|s| s.ty).unwrap_or(Type::Int32);
        // Every printable operand is a slot holding either an integer value or a
        // pointer; only the printf format string differs by type.
        let format = if ty == Type::Str { "format_str" } else { "format_int" };
        self.asm.push(format!("    push dword [{}]", var));
        self.asm.push(format!("    push {}", format));
        self.asm.push("    call printf".to_string());
        self.asm.push("    add esp, 8".to_string());
    }

    fn generate_bss(&mut self) {
        self.asm.push(String::new());
        self.asm.push("section .bss".to_string());
        // Every symbol gets a 4-byte slot (int value or pointer) except string
        // constants, which live in `.rodata`.
        let mut names: Vec<String> = self
            .symbol_table
            .table
            .keys()
            .filter(|name| !self.is_string_const(name))
            .cloned()
            .collect();
        names.sort(); // deterministic output (HashMap iteration order is not)
        for name in names {
            self.asm.push(format!("{}: resd 1", name));
        }
    }

    fn generate_rodata(&mut self) {
        self.asm.push(String::new());
        self.asm.push("section .rodata".to_string());
        self.asm.push("format_int: db \"%d\", 10, 0".to_string());
        self.asm.push("format_str: db \"%s\", 10, 0".to_string());
        for (label, value) in &self.strings {
            // NOTE: assumes the literal contains no embedded double-quotes.
            self.asm.push(format!("{}: db \"{}\", 0", label, value));
        }
    }
}
