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
            "apl_classes_library_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(root.join("libraries")).unwrap();
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn build_classes_module(&self) {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/classes.apl");
        let output = apl()
            .arg("emit-module")
            .arg(source)
            .arg(self.path("libraries/classes.aplmod"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn build_base_module(&self) {
        let source = self.path("model.apl");
        fs::write(&source, BASE_MODULE).unwrap();
        let output = apl()
            .current_dir(&self.root)
            .arg("emit-module")
            .arg(&source)
            .arg(self.path("libraries/model.aplmod"))
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

const BASE_MODULE: &str = r#"include >> classes
class Base {
  func __init__() {
    VTime ready = true
  }
  func value() {
    return "base"
  }
  func run() {
    return self.value()
  }
}
"#;

const EXTERNAL_DERIVED_SOURCE: &str = r#"include >> classes
include >> model
class Derived {
  class::inheritance "Base"
  func __init__() {
    VTime ready = true
  }
  override func value() {
    return "derived"
  }
}
AVInt std::main() {
  Base item = Derived()
  out item.run()
  return 0
}
"#;

const CLASS_SOURCE: &str = r#"include >> classes
class Message {
  AVStr text = ""
  func __init__(text) {
    self.text = text
  }
  func value() {
    return self.text
  }
  func __out__() {
    out self.value()
  }
}
class LoudMessage {
  class::inheritance "Message"
  func __init__(text) {
    self.text = text
  }
  override func value() {
    return self.text + "!"
  }
}
AVInt std::main() {
  Message message = LoudMessage("APL")
  VTime alias = message
  out message
  out ":"
  out alias.value()
  out ":"
  out classes::id(message) == classes::id(alias)
  return 0
}
"#;

#[test]
fn classes_run_through_portable_and_native_cli_paths() {
    let project = TestProject::new();
    project.build_classes_module();
    let source = project.path("class_program.apl");
    fs::write(&source, CLASS_SOURCE).unwrap();

    let portable = apl().arg("run").arg(&source).output().unwrap();
    assert!(
        portable.status.success(),
        "{}",
        String::from_utf8_lossy(&portable.stderr)
    );
    assert_eq!(
        String::from_utf8(portable.stdout).unwrap(),
        "APL!:APL!:true"
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
        "class_program_compiled_native{}",
        std::env::consts::EXE_SUFFIX
    ));
    let native = Command::new(&executable).output().unwrap();
    assert!(
        native.status.success(),
        "{}",
        String::from_utf8_lossy(&native.stderr)
    );
    assert_eq!(String::from_utf8(native.stdout).unwrap(), "APL!:APL!:true");
}

#[test]
fn imported_base_dispatch_expands_for_external_descendants() {
    let project = TestProject::new();
    project.build_classes_module();
    project.build_base_module();
    let source = project.path("external_derived.apl");
    fs::write(&source, EXTERNAL_DERIVED_SOURCE).unwrap();

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
    assert_eq!(String::from_utf8(portable.stdout).unwrap(), "derived");

    if std::env::consts::ARCH != "x86_64" || (!cfg!(windows) && !cfg!(target_os = "linux")) {
        return;
    }

    let package = project.path("external_derived_native");
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
        "external_derived_compiled_native{}",
        std::env::consts::EXE_SUFFIX
    ));
    let native = Command::new(&executable).output().unwrap();
    assert!(
        native.status.success(),
        "{}",
        String::from_utf8_lossy(&native.stderr)
    );
    assert_eq!(String::from_utf8(native.stdout).unwrap(), "derived");
}
