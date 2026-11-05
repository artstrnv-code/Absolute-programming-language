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
            "apl_crypto_library_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(root.join("libraries")).unwrap();
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn build_crypto_module(&self) {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries/crypto.apl");
        let output = apl()
            .current_dir(&self.root)
            .arg("emit-module")
            .arg(source)
            .arg(self.path("libraries/crypto.aplmod"))
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
fn temporary_random_integer_stays_in_half_open_range_and_can_be_secret_typed() {
    let project = TestProject::new();
    project.build_crypto_module();
    let program = project.write_program(
        "range.apl",
        r#"include >> crypto
AVInt std::main() {
  AVInt first = crypto::safe_rand_int(-50, 50)
  AVInt second = crypto::safe_rand_int(10, 11)
  ASVInt secret_value = crypto::safe_rand_int(10, 11)
  out first >= -50
  out first < 50
  out second == 10
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

    let output = apl()
        .current_dir(&project.root)
        .arg("run")
        .arg(&program)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"truetruetrue");
}

#[test]
fn temporary_random_integer_fails_for_empty_range() {
    let project = TestProject::new();
    project.build_crypto_module();
    let program = project.write_program(
        "invalid_range.apl",
        r#"include >> crypto
AVInt std::main() {
  VTime value = crypto::safe_rand_int(4, 4)
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
        String::from_utf8_lossy(&output.stderr)
            .contains("crypto::safe_rand_int requires minimum < maximum"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn crypto_library_hashes_bytes_with_sha1_sha256_sha512_and_sha3_384() {
    let project = TestProject::new();
    project.build_crypto_module();
    let program = project.write_program(
        "hashes.apl",
        r#"include >> crypto
AVInt std::main() {
  out crypto::sha1_hex("abc")
  out ":"
  out crypto::sha256_hex("abc")
  out ":"
  out crypto::sha512_hex("abc")
  out ":"
  out crypto::sha3_384_hex("abc")
  out ":"
  out std::len(crypto::sha3_384("abc"))
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
    assert_eq!(
        output.stdout,
        b"a9993e364706816aba3e25717850c26c9cd0d89d:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad:ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f:ec01498288516fc926459f58e2c6ad8df9b473cb0fc08c2596da7cf0e49be4b298d88cea927ac7f539f1edf228376d25:48"
    );
}

#[test]
fn crypto_hash_preserves_secret_protection() {
    let project = TestProject::new();
    project.build_crypto_module();
    let program = project.write_program(
        "secret_hash.apl",
        r#"include >> crypto
AVInt std::main() {
  ASVBytes secret_data = std::bytes("secret")
  ASVBytes digest = crypto::sha256(secret_data)
  out digest
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
        String::from_utf8_lossy(&checked.stderr).contains("secret expression denied `out`"),
        "stderr: {}",
        String::from_utf8_lossy(&checked.stderr)
    );
}

#[test]
fn crypto_hash_rejects_non_bytes_input() {
    let project = TestProject::new();
    project.build_crypto_module();
    let program = project.write_program(
        "invalid_hash_input.apl",
        r#"include >> crypto
AVInt std::main() {
  VTime digest = std::sha256(42)
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
        String::from_utf8_lossy(&checked.stderr).contains("sha256(value) requires Bytes"),
        "stderr: {}",
        String::from_utf8_lossy(&checked.stderr)
    );
}
