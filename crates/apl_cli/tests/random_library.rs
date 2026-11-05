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
            "apl_random_library_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(root.join("libraries")).unwrap();
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn build_random_module(&self) {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/random.apl");
        let output = apl()
            .current_dir(&self.root)
            .arg("emit-module")
            .arg(source)
            .arg(self.path("libraries/random.aplmod"))
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

#[test]
fn random_library_returns_reproducible_scalar_values() {
    let project = TestProject::new();
    project.build_random_module();
    let program = project.path("program.apl");
    fs::write(
        &program,
        r#"include >> random
exclude "_normalize_seed" << random
exclude "_step" << random
exclude "_draw_integer" << random
AVInt std::main() {
  VTime first = random::integer(99, 1, 200)
  VTime repeat = random::integer(99, 1, 200)
  out first
  out ":"
  out first == repeat
  out ":"
  VTime float_first = random::rfloat(99, 1.0, 200.0)
  VTime float_repeat = random::rfloat(99, 1.0, 200.0)
  out float_first >= 1.0
  out ":"
  out float_first < 200.0
  out ":"
  out float_first == float_repeat
  out ":"
  out random::choose(99, ["red", "green", "blue"])
  out ":"
  out random::choose(99, [])
  out ":"
  out random::boolean(99)
  out ":"
  out random::boolean(99) == random::boolean(99)
  out ":"
  List variants = [1, 2, 3, 4]
  VTime shuffled = random::shuffle(99, variants)
  out shuffled
  out ":"
  out variants
  out ":"
  out random::sample(99, variants, 2)
  out ":"
  out random::sample(99, variants, 0)
  out ":"
  out random::sample(99, variants, 5)
  out ":"
  out random::integer(99, 5, 5)
  out ":"
  out random::integer(99, 10, 1)
  out ":"
  out random::integer(99, -9223372036854775807 - 1, 0)
  return 0
}
"#,
    )
    .unwrap();

    let output = apl()
        .current_dir(&project.root)
        .arg("run")
        .arg(program)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let fields = stdout.split(':').collect::<Vec<_>>();
    assert_eq!(fields.len(), 17, "{stdout:?}");
    let value = fields[0].parse::<i64>().unwrap();
    assert!((1..=200).contains(&value), "{value}");
    assert_eq!(value, 29);
    assert_eq!(fields[1], "true");
    assert_eq!(fields[2], "true");
    assert_eq!(fields[3], "true");
    assert_eq!(fields[4], "true");
    assert_eq!(fields[5], "blue");
    assert_eq!(fields[6], "NONE");
    assert_eq!(fields[7], "false");
    assert_eq!(fields[8], "true");
    let shuffled = fields[9]
        .trim_matches(['[', ']'])
        .split(", ")
        .collect::<Vec<_>>();
    let mut sorted = shuffled.clone();
    sorted.sort();
    assert_eq!(sorted, ["1", "2", "3", "4"]);
    assert_eq!(fields[10], "[1, 2, 3, 4]");
    let sampled = fields[11]
        .trim_matches(['[', ']'])
        .split(", ")
        .collect::<Vec<_>>();
    assert_eq!(sampled.len(), 2);
    assert_ne!(sampled[0], sampled[1]);
    assert!(sampled
        .iter()
        .all(|item| ["1", "2", "3", "4"].contains(item)));
    assert_eq!(fields[12], "[]");
    assert_eq!(fields[13], "NONE");
    assert_eq!(fields[14], "5");
    assert_eq!(fields[15], "NONE");
    assert_eq!(fields[16], "NONE");

    let sample = apl()
        .current_dir(&project.root)
        .arg("run")
        .arg(project.path("program.apl"))
        .output()
        .unwrap();
    assert!(sample.status.success());
    assert_eq!(sample.stdout, output.stdout);
}
