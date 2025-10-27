fn main() {
    // Check if ast.rs exists, if not print helpful error
    let ast_path = std::path::Path::new("src/ast.rs");
    if !ast_path.exists() {
        eprintln!("ERROR: tea-compiler/src/ast.rs is missing!");
        eprintln!("This file is generated from docs/ast.yaml.");
        eprintln!("Please run: make codegen");
        std::process::exit(1);
    }
}
