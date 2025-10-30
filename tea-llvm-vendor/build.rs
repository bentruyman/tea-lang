use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let target = env::var("TARGET").unwrap();

    // Only support macOS arm64 for now
    if target != "aarch64-apple-darwin" {
        println!("cargo:warning=tea-llvm-vendor only supports aarch64-apple-darwin currently");
        println!(
            "cargo:warning=Skipping LLVM static linking for target: {}",
            target
        );
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let install_dir = manifest_dir.join("install-macos-arm64");
    let lib_dir = install_dir.join("lib");
    let link_args_file = install_dir.join("link-args.txt");

    // Check if LLVM has been built
    if !lib_dir.exists() {
        eprintln!("\n========================================");
        eprintln!("tea-llvm-vendor: LLVM libraries not found");
        eprintln!("========================================");
        eprintln!("Expected location: {}", lib_dir.display());
        eprintln!("");
        eprintln!("To build vendored LLVM + LLD:");
        eprintln!("  ./scripts/llvm/build-all-macos-arm64.sh");
        eprintln!("");
        eprintln!("Or to skip AOT compilation:");
        eprintln!("  cargo build --workspace --exclude tea-llvm-vendor");
        eprintln!("========================================\n");

        // Don't panic - just skip linking and let compilation continue
        // This allows building tea-cli without llvm-aot feature
        return;
    }

    // Tell cargo where to find the libraries
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // Read and parse link-args.txt to get library order
    if link_args_file.exists() {
        let content = fs::read_to_string(&link_args_file).expect("failed to read link-args.txt");

        for line in content.lines() {
            let line = line.trim();
            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Emit link directive for each library
            println!("cargo:rustc-link-lib=static={}", line);
        }
    } else {
        println!("cargo:warning=link-args.txt not found, using fallback library list");

        // Fallback: minimal library list in dependency order
        let libs = vec![
            // LLD
            "lldMachO",
            "lldCommon",
            // LLVM core
            "LLVMAArch64CodeGen",
            "LLVMAArch64AsmParser",
            "LLVMAArch64Desc",
            "LLVMAArch64Info",
            "LLVMAArch64Utils",
            "LLVMX86CodeGen",
            "LLVMX86AsmParser",
            "LLVMX86Desc",
            "LLVMX86Info",
            "LLVMAsmPrinter",
            "LLVMGlobalISel",
            "LLVMSelectionDAG",
            "LLVMCodeGen",
            "LLVMScalarOpts",
            "LLVMInstCombine",
            "LLVMAggressiveInstCombine",
            "LLVMTransformUtils",
            "LLVMBitWriter",
            "LLVMAnalysis",
            "LLVMProfileData",
            "LLVMObject",
            "LLVMMCParser",
            "LLVMMC",
            "LLVMBitReader",
            "LLVMCore",
            "LLVMBinaryFormat",
            "LLVMSupport",
            "LLVMDemangle",
        ];

        for lib in libs {
            println!("cargo:rustc-link-lib=static={}", lib);
        }
    }

    // Link system libraries that LLVM depends on
    println!("cargo:rustc-link-lib=dylib=c++");
    println!("cargo:rustc-link-lib=dylib=z");

    // Rerun if the install directory changes
    println!("cargo:rerun-if-changed={}", install_dir.display());
    println!("cargo:rerun-if-changed={}", link_args_file.display());
}
