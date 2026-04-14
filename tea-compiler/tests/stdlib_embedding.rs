use std::env;
use std::sync::Mutex;

use tea_compiler::{CompileOptions, Compiler, SourceFile, SourceId};
use tempfile::tempdir;

static CURRENT_DIR_LOCK: Mutex<()> = Mutex::new(());

struct CurrentDirGuard {
    previous: std::path::PathBuf,
}

impl CurrentDirGuard {
    fn change_to(path: &std::path::Path) -> anyhow::Result<Self> {
        let previous = env::current_dir()?;
        env::set_current_dir(path)?;
        Ok(Self { previous })
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        let _ = env::set_current_dir(&self.previous);
    }
}

#[test]
fn source_stdlib_is_available_outside_workspace() -> anyhow::Result<()> {
    let _lock = CURRENT_DIR_LOCK.lock().expect("current dir lock");
    let tmp = tempdir()?;
    let _cwd = CurrentDirGuard::change_to(tmp.path())?;

    let source = r#"
use string = "std.string"

var repeated = string.repeat("ha", 2)
@println(string.to_upper(repeated))
"#;

    let source_file = SourceFile::new(
        SourceId(0),
        tmp.path().join("stdlib.tea"),
        source.to_string(),
    );

    let mut compiler = Compiler::new(CompileOptions::default());
    compiler.compile(&source_file)?;

    assert!(
        compiler.diagnostics().is_empty(),
        "expected no diagnostics, found {:?}",
        compiler.diagnostics().entries()
    );

    Ok(())
}
