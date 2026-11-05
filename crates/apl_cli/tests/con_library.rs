use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
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
        let root =
            std::env::temp_dir().join(format!("apl_con_library_{}_{}", std::process::id(), unique));
        fs::create_dir_all(root.join("libraries")).unwrap();
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn build_con_module(&self) {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/console.apl");
        let output = apl()
            .current_dir(&self.root)
            .arg("emit-module")
            .arg(source)
            .arg(self.path("libraries/console.aplmod"))
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

fn run_with_input(path: &Path, input: &[u8]) -> Output {
    let mut child = apl()
        .current_dir(path.parent().unwrap())
        .arg("run")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn con_print_and_wait_in_handle_output_input_and_eof() {
    let project = TestProject::new();
    project.build_con_module();
    let program = project.write_program(
        r#"include >> console
AVInt std::main() {
  con::print([], ",", "!")
  con::print(["A", 2, true], "|", "!")
  VTime answer = con::wait_in("name?")
  con::print([answer], "", "\n")
  return 0
}
"#,
    );

    let checked = apl()
        .current_dir(&project.root)
        .arg("check")
        .arg(&program)
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    let output = run_with_input(&program, b"Ada\n");
    assert!(output.status.success());
    assert_eq!(
        output.stdout,
        b"!A|2|true!name?\nAda\n",
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let blank_line = run_with_input(&program, b"\n");
    assert!(blank_line.status.success());
    assert_eq!(blank_line.stdout, b"!A|2|true!name?\n\n");

    let eof = run_with_input(&program, b"");
    assert_eq!(eof.status.code(), Some(1));
    assert!(eof.stdout.is_empty());
    assert_eq!(eof.stderr, b"input reached EOF\n");
}

#[test]
fn con_output_functions_reject_secret_arguments() {
    let project = TestProject::new();
    project.build_con_module();
    let print_program = project.write_program(
        r#"include >> console
AVInt std::main() {
  ASVStr hidden = "secret"
  con::print([hidden], "", "\n")
  return 0
}
"#,
    );

    let checked = apl()
        .current_dir(&project.root)
        .arg("check")
        .arg(&print_program)
        .output()
        .unwrap();
    assert!(!checked.status.success());
    assert!(
        String::from_utf8_lossy(&checked.stderr).contains("secret expression denied `con::print`"),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    let prompt_program = project.write_program(
        r#"include >> console
AVInt std::main() {
  ASVStr hidden = "secret"
  con::wait_in(hidden)
  return 0
}
"#,
    );
    let checked = apl()
        .current_dir(&project.root)
        .arg("check")
        .arg(&prompt_program)
        .output()
        .unwrap();
    assert!(!checked.status.success());
    assert!(
        String::from_utf8_lossy(&checked.stderr)
            .contains("secret expression denied `con::wait_in`"),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
}

#[test]
fn con_string_case_functions_handle_spaces_and_unicode() {
    let project = TestProject::new();
    project.build_con_module();
    let program = project.write_program(
        r#"include >> console
AVInt std::main() {
  out con::lower("HELLO Мир ЁЖ")
  out "|"
  out con::upper("hello мир ёж")
  out "|"
  out con::title("hELLO  мИР ёЖ")
  out "|"
  out con::title("")
  return 0
}
"#,
    );

    let checked = apl()
        .current_dir(&project.root)
        .arg("check")
        .arg(&program)
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    let output = run_with_input(&program, b"");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        "hello мир ёж|HELLO МИР ЁЖ|Hello  Мир Ёж|".as_bytes()
    );
}

#[test]
fn con_string_helpers_split_join_search_and_replace() {
    let project = TestProject::new();
    project.build_con_module();
    let program = project.write_program(
        r#"include >> console
AVInt std::main() {
  out con::join(con::split("a,,b,", ","), "|")
  out "|"
  out con::join(con::split("abc", ""), "|")
  out "|"
  out con::contains("banana", "nan")
  out "|"
  out con::contains("banana", "xyz")
  out "|"
  out con::replace("banana", "ana", "X")
  out "|"
  out con::replace("aaaa", "aa", "b")
  out "|"
  out con::replace("abc", "", "x")
  return 0
}
"#,
    );

    let checked = apl()
        .current_dir(&project.root)
        .arg("check")
        .arg(&program)
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    let output = run_with_input(&program, b"");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"a||b||NONE|true|false|bXna|bb|abc");
}

#[test]
fn con_case_functions_preserve_secret_protection() {
    let project = TestProject::new();
    project.build_con_module();
    let program = project.write_program(
        r#"include >> console
AVInt std::main() {
  ASVStr hidden = "secret"
  con::print([con::lower(hidden)], "", "\n")
  return 0
}
"#,
    );

    let checked = apl()
        .current_dir(&project.root)
        .arg("check")
        .arg(&program)
        .output()
        .unwrap();
    assert!(!checked.status.success());
    assert!(
        String::from_utf8_lossy(&checked.stderr).contains("secret expression denied `con::print`"),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );

    let program = project.write_program(
        r#"include >> console
AVInt std::main() {
  ASVStr hidden = "secret"
  con::print([con::replace(hidden, "e", "x")], "", "\n")
  return 0
}
"#,
    );
    let checked = apl()
        .current_dir(&project.root)
        .arg("check")
        .arg(&program)
        .output()
        .unwrap();
    assert!(!checked.status.success());
    assert!(
        String::from_utf8_lossy(&checked.stderr).contains("secret expression denied `con::print`"),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
}
