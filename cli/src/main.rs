use clap::{App, Arg};
use compiler::lower::Lowerer;
use compiler::x86::X86Generator;
use nom::error::Error;
use parser::{parser_entrypoint, print_ast};
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    let matches = App::new("wasm-lang")
        .version("1.0")
        .about("a compiler for a made up language")
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file to compile")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .takes_value(true)
                .help("Path for the generated .asm file (defaults to <input>.asm)"),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("verbose: print the parsed AST"),
        )
        .get_matches();

    let verbose = matches.occurrences_of("v") > 0;
    // `INPUT` is required, so `unwrap` here is safe.
    let input_file = matches.value_of("INPUT").unwrap();
    let output_file = matches
        .value_of("output")
        .map(String::from)
        .unwrap_or_else(|| default_output(input_file));

    let contents = fs::read_to_string(input_file).unwrap_or_else(|e| {
        eprintln!("cannot read {}: {}", input_file, e);
        process::exit(1);
    });

    // 1. Parse into an AST.
    let mut ast = match parser_entrypoint::<Error<&str>>(contents.as_str()) {
        Ok((_, ast)) => ast,
        Err(reason) => {
            eprintln!("parse error: {}", reason);
            process::exit(1);
        }
    };

    if verbose {
        println!("=== AST ===");
        print_ast(input_file.to_string(), ast.as_mut());
    }

    // 2. Lower the AST to IR + a symbol table.
    let mut lowerer = Lowerer::new();
    if let Err(e) = lowerer.lower_program(ast.as_ref()) {
        eprintln!("lowering error: {}", e);
        process::exit(1);
    }

    if verbose {
        println!("\n=== IR ===");
        print!("{}", lowerer.ir);
        println!("\n=== Symbol Table ===");
        print!("{}", lowerer.symbols);
        if !lowerer.strings.is_empty() {
            println!("\n=== String Constants ===");
            for (label, value) in &lowerer.strings {
                println!("    {:<8} {:?}", label, value);
            }
        }
        println!();
    }

    // 3. Generate x86 assembly.
    let mut generator = X86Generator::new(lowerer.symbols, lowerer.strings);
    let asm = generator.generate(&lowerer.ir);

    // 4. Write it out.
    if let Err(e) = fs::write(&output_file, &asm) {
        eprintln!("cannot write {}: {}", output_file, e);
        process::exit(1);
    }
    println!("wrote {}", output_file);
}

fn default_output(input: &str) -> String {
    let stem = Path::new(input)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
    format!("{}.asm", stem)
}
