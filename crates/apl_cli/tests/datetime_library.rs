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
            "apl_datetime_library_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(root.join("libraries")).unwrap();
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn build_datetime_module(&self) {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/datetime.apl");
        let output = apl()
            .current_dir(&self.root)
            .arg("emit-module")
            .arg(source)
            .arg(self.path("libraries/datetime.aplmod"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn write_program(&self, name: &str, source: &str) -> PathBuf {
        let path = self.path(name);
        fs::write(&path, source).unwrap();
        path
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

#[test]
fn datetime_library_returns_utc_and_fixed_offset_calendar_fields() {
    let project = TestProject::new();
    project.build_datetime_module();
    let program = project.write_program(
        "program.apl",
        r#"include >> datetime
AVInt std::main() {
  VTime utc = datetime::now_utc()
  VTime local = datetime::now_local()
  if std::len(utc) != 7 {
    return 1
  }
  if std::len(local) != 7 {
    return 2
  }
  if datetime.is_valid_date(std::get(utc, 0), std::get(utc, 1), std::get(utc, 2)) == false {
    return 3
  }
  if datetime.is_valid_date(std::get(local, 0), std::get(local, 1), std::get(local, 2)) == false {
    return 4
  }
  if (std::get(utc, 3) < 0) or (std::get(utc, 3) > 23) {
    return 5
  }
  if (std::get(utc, 4) < 0) or (std::get(utc, 4) > 59) {
    return 6
  }
  if (std::get(utc, 5) < 0) or (std::get(utc, 5) > 59) {
    return 7
  }
  if (std::get(utc, 6) < 0) or (std::get(utc, 6) > 999) {
    return 8
  }
  return 0
}
"#,
    );
    let output = apl()
        .current_dir(&project.root)
        .arg("run")
        .arg(&program)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[cfg(all(
    target_arch = "x86_64",
    any(target_os = "windows", target_os = "linux")
))]
fn native_now_utc_builtin_links_and_returns_seven_fields() {
    let project = TestProject::new();
    let source = project.write_program(
        "native_now_utc.apl",
        r#"AVInt std::main() {
  out std::len(std::now_utc()) == 7
  return 0
}
"#,
    );
    let package = project.path("native_now_utc_package");
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

    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| package.join("target"));
    let executable = target_dir.join("debug").join(format!(
        "native_now_utc_compiled_native{}",
        std::env::consts::EXE_SUFFIX
    ));
    assert!(
        executable.is_file(),
        "missing native executable {}",
        executable.display()
    );
    let output = Command::new(executable).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "true");
}
