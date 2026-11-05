use std::{
    fs,
    path::PathBuf,
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
            "apl_structures_library_{}_{}",
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
        let source = self.path("records.apl");
        fs::write(&source, RECORDS_MODULE).unwrap();
        let output = apl()
            .current_dir(&self.root)
            .arg("emit-module")
            .arg(&source)
            .arg(self.path("libraries/records.aplmod"))
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
    let mut command = Command::new(env!("CARGO_BIN_EXE_apl"));
    command.env_remove("CARGO_TARGET_DIR");
    command.env(
        "APL_NATIVE_RUNTIME_DIR",
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../apl_native_runtime"),
    );
    command
}

const RECORDS_MODULE: &str = r#"struct Pair {
  AVInt left = 0
  AVInt right = 0
}

func records.make(left, right) {
  Pair result = Pair()
  result.left = left
  result.right = right
  return result
}
"#;

const PROGRAM: &str = r#"include >> records

AVInt std::main() {
  Pair first = records::make(2, 3)
  Pair second = first
  first.left += 5
  out first.left
  out ":"
  out second.left
  out ":"
  out first.right
  return 0
}
"#;

#[test]
fn cyclic_struct_values_are_rejected_before_ir_generation() {
    let project = TestProject::new();
    let source = project.path("cyclic.apl");
    fs::write(
        &source,
        r#"struct Left {
  Right right = Right()
}
struct Right {
  Left left = Left()
}
"#,
    )
    .unwrap();

    let output = apl()
        .current_dir(&project.root)
        .arg("emit-module")
        .arg(&source)
        .arg(project.path("libraries/cyclic.aplmod"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cyclic value composition"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn exported_structs_copy_through_portable_and_native_cli_paths() {
    let project = TestProject::new();
    project.build_module();
    let source = project.path("struct_program.apl");
    fs::write(&source, PROGRAM).unwrap();

    let portable = apl()
        .current_dir(&project.root)
        .arg("run")
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        portable.status.success(),
        "{}",
        String::from_utf8_lossy(&portable.stderr)
    );
    assert_eq!(String::from_utf8(portable.stdout).unwrap(), "7:2:3");

    if std::env::consts::ARCH != "x86_64" || (!cfg!(windows) && !cfg!(target_os = "linux")) {
        return;
    }

    let package = project.path("native_package");
    let compiled = apl()
        .current_dir(&project.root)
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
        "struct_program_compiled_native{}",
        std::env::consts::EXE_SUFFIX
    ));
    let native = Command::new(&executable).output().unwrap();
    assert!(
        native.status.success(),
        "{}",
        String::from_utf8_lossy(&native.stderr)
    );
    assert_eq!(String::from_utf8(native.stdout).unwrap(), "7:2:3");
}
