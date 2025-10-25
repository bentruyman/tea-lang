fn main() {
    cc::Build::new()
        .include("../../src")
        .file("../../src/parser.c")
        .compile("tree-sitter-tea");
}
