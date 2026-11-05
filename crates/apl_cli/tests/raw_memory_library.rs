use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

struct TestProject {
    root: PathBuf,
}

impl TestProject {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "apl_raw_memory_library_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(root.join("libraries")).unwrap();
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn build_module(&self) {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/raw_memory.apl");
        let output = apl()
            .arg("emit-module")
            .arg(source)
            .arg(self.path("libraries/raw_memory.aplmod"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn apl() -> Command {
    Command::new(env!("CARGO_BIN_EXE_apl"))
}

const SOURCE: &str = r#"include >> raw_memory
@marked
@flee
func scoped() {
  @collector
  while (true) (1) {
    VTime temporary = memory::malloc(1)
    return 5
  }
  return 0
}
@collector
func retained() {
  VTime pointer = memory::malloc(4)
  return pointer
}
AVInt std::main() {
  out scoped()
  out ":"
  VTime pointer = retained()
  memory::write(pointer, 1, std::bytes("AB"))
  out memory::where(pointer, 1) > 0
  out ":"
  out memory::where(pointer, 2)
  out ":"
  out std::str(memory::read(pointer, 0, 4))
  out ":"
  memory::mark(pointer)
  out memory::free()
  return 0
}
"#;

#[test]
fn raw_memory_runs_through_portable_and_native_cli_paths() {
    let project = TestProject::new();
    project.build_module();
    let source = project.path("raw_memory_program.apl");
    fs::write(&source, SOURCE).unwrap();

    let portable = apl().arg("run").arg(&source).output().unwrap();
    assert!(
        portable.status.success(),
        "{}",
        String::from_utf8_lossy(&portable.stderr)
    );
    assert_eq!(
        String::from_utf8(portable.stdout).unwrap(),
        "5:true:pointer:[0, 65, 66, 0]:1"
    );

    if std::env::consts::ARCH != "x86_64" || (!cfg!(windows) && !cfg!(target_os = "linux")) {
        return;
    }

    let package = project.path("native_package");
    let compiled = apl()
        .arg("compile-native")
        .arg(&source)
        .arg(&package)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let executable = package.join("target").join("debug").join(format!(
        "raw_memory_program_compiled_native{}",
        std::env::consts::EXE_SUFFIX
    ));
    let native = Command::new(&executable).output().unwrap();
    assert!(
        native.status.success(),
        "{}",
        String::from_utf8_lossy(&native.stderr)
    );
    assert_eq!(
        String::from_utf8(native.stdout).unwrap(),
        "5:true:pointer:[0, 65, 66, 0]:1"
    );
}
