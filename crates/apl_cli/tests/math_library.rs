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
            "apl_math_library_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(root.join("libraries")).unwrap();
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn build_math_module(&self) {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/math.apl");
        let output = apl()
            .current_dir(&self.root)
            .arg("emit-module")
            .arg(source)
            .arg(self.path("libraries/math.aplmod"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn write_program(&self, source: &str) -> PathBuf {
        let path = self.path("program.apl");
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
fn math_library_compiles_and_handles_domains_and_numeric_boundaries() {
    let project = TestProject::new();
    project.build_math_module();
    let program = project.write_program(
        r#"include >> math
AVInt std::main() {
  out math::fcl(0) == 1
  out math::fcl(5) == 120
  out math::fcl(2.5) == NONE
  out math::fcl(21) == NONE
  out math::log(0.5, 0.25) > 1.999999999999
  out math::log(0.5, 0.25) < 2.000000000001
  out math::log(1.0, 10.0) == NONE
  out math::log(10.0, 0.0) == NONE
  out math::sqrt(-1.0) == NONE
  out math::abs(math::sqrt(0.000000000000000000000001) - 0.000000000001) < 0.0000000000000000000000001
  out math::abs(math::cbrt(-27.0) + 3.0) < 0.000000000000001
  out math::abs(math::atan(1.0) - math::pi() / 4.0) < 0.000000000000001
  out math::sin(math::pi() / 2.0) > 0.999999999999
  out math::sin(math::pi() / 2.0) < 1.000000000001
  out math::cos(0.0) > 0.999999999999
  out math::cos(0.0) < 1.000000000001
  out math::abs(math::rad(180.0) - math::pi()) < 0.000000000000001
  out math::asin(1.0) == math::pi() / 2.0
  out math::asin(1.1) == NONE
  out math::acos(-1.1) == NONE
  VTime huge = std::float("1e100")
  VTime sine = math::sin(huge)
  VTime cosine = math::cos(huge)
  out (sine > -1.000001) and (sine < 1.000001)
  out (cosine > -1.000001) and (cosine < 1.000001)
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
    assert_eq!(String::from_utf8_lossy(&output.stdout), "true".repeat(22));
}
