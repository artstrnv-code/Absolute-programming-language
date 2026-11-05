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
            "apl_timer_library_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(root.join("libraries")).unwrap();
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn build_timer_module(&self) {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/timer.apl");
        let output = apl()
            .current_dir(&self.root)
            .arg("emit-module")
            .arg(source)
            .arg(self.path("libraries/timer.aplmod"))
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
fn timer_library_supports_pause_resume_and_unit_selection() {
    let project = TestProject::new();
    project.build_timer_module();
    let program = project.write_program(
        "program.apl",
        r#"include >> timer
AVStr __timer__ = "ms"
AVInt std::main() {
  timer::start_timer()
  timer::stop_timer()
  timer::start_timer()
  timer::stop_timer()
  out timer::return_timer(__timer__)
  out ":"
  out timer::return_timer("s")
  out ":"
  out timer::return_timer("m")
  out ":"
  out timer::return_timer("invalid")
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
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let fields = stdout.split(':').collect::<Vec<_>>();
    assert_eq!(fields.len(), 4, "{stdout:?}");
    let milliseconds = fields[0].parse::<f64>().unwrap();
    let seconds = fields[1].parse::<f64>().unwrap();
    let minutes = fields[2].parse::<f64>().unwrap();
    assert!(milliseconds >= 0.0);
    assert!(seconds >= 0.0);
    assert!(minutes >= 0.0);
    assert_eq!(fields[3], "NONE");
}

#[test]
fn timer_rejects_starting_while_already_running() {
    let project = TestProject::new();
    project.build_timer_module();
    let program = project.write_program(
        "already_running.apl",
        r#"include >> timer
AVInt std::main() {
  timer::start_timer()
  timer::start_timer()
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
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("timer is already running"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[cfg(all(
    target_arch = "x86_64",
    any(target_os = "windows", target_os = "linux")
))]
fn native_timer_builtins_link_and_run() {
    let project = TestProject::new();
    let source = project.write_program(
        "native_timer.apl",
        r#"AVInt std::main() {
  std::timer_start()
  std::timer_stop()
  out std::timer_elapsed() >= 0.0
  return 0
}
"#,
    );
    let package = project.path("native_timer_package");
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
        "native_timer_compiled_native{}",
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
