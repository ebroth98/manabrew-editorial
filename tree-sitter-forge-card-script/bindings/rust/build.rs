fn main() {
    let src_dir = std::path::Path::new("src");
    cc::Build::new()
        .include(src_dir)
        .file(src_dir.join("parser.c"))
        .warnings(false)
        .compile("tree-sitter-forge-card-script");

    println!("cargo:rerun-if-changed=src/parser.c");
    println!("cargo:rerun-if-changed=src/grammar.json");
    println!("cargo:rerun-if-changed=src/node-types.json");
    println!("cargo:rerun-if-changed=queries/highlights.scm");
}
