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
            "apl_windows_library_{}_{}",
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
        let library_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../libraries");
        for name in ["classes", "windows"] {
            let local_source = self.path(&format!("{name}.apl"));
            fs::copy(library_root.join(format!("{name}.apl")), &local_source).unwrap();
            fs::copy(
                library_root.join(format!("{name}.metadata.toml")),
                self.path(&format!("{name}.metadata.toml")),
            )
            .unwrap();
            fs::copy(
                library_root.join(format!("{name}.metadata.toml")),
                self.path(&format!("libraries/{name}.metadata.toml")),
            )
            .unwrap();
            let output = apl()
                .current_dir(&self.root)
                .arg("emit-module")
                .arg(local_source)
                .arg(self.path(&format!("libraries/{name}.aplmod")))
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "failed to build {name}: status={} stdout={} stderr={}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
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
    let mut command = Command::new(env!("CARGO_BIN_EXE_apl"));
    command.env_remove("CARGO_TARGET_DIR");
    command.env(
        "APL_NATIVE_RUNTIME_DIR",
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../apl_native_runtime"),
    );
    command
}

fn minimal_icon() -> Vec<u8> {
    let mut bytes = vec![
        0, 0, 1, 0, 1, 0, 1, 1, 0, 0, 1, 0, 32, 0, 48, 0, 0, 0, 22, 0, 0, 0,
    ];
    bytes.extend_from_slice(&40_u32.to_le_bytes());
    bytes.extend_from_slice(&1_i32.to_le_bytes());
    bytes.extend_from_slice(&2_i32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&32_u16.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&4_u32.to_le_bytes());
    bytes.extend_from_slice(&0_i32.to_le_bytes());
    bytes.extend_from_slice(&0_i32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&[0x00, 0x80, 0xff, 0xff, 0, 0, 0, 0]);
    bytes
}

const WINDOWS_PROGRAM: &str = r#"include >> classes
include >> windows

class TestWindowApplication {
  class::inheritance "WindowApplication"
  AVBool destroy_handled = false

  func __init__(window) {
    self.window = window
    windows.set_deferred_close(window, true)
  }

  override func on_destroy(raw) {
    self.destroy_handled = true
    return true
  }
}

AVInt std::main() {
  FileDialogConfig dialog = FileDialogConfig()
  dialog.title = "Open APL source"
  if false {
    VTime unused_open = dialog.open(0)
    VTime unused_save = dialog.save(0)
    Icon unused_icon = Icon("unused.ico", 16, 16)
    unused_icon.apply(0)
    unused_icon.destroy()
    out unused_open
    out unused_save
  }
  out windows::available()
  out ":"
  out std::len(windows::utf16("A"))
  if windows::available() {
    VTime library = windows::load_library("kernel32.dll")
    VTime procedure = windows::get_procedure(library, "GetCurrentProcessId", [], "U32")
    out ":"
    out windows::call(procedure, []) > 0
    out ":"
    out windows::close_library(library)
    VTime callback = windows::create_callback(windows::CALLBACK_WNDPROC, 73)
    out ":"
    out windows::call(callback, [0, 12, 34, -5]) == 73
    VTime callback_event = windows::next_callback(callback)
    out ":"
    out (std::len(callback_event) == 4) and (std::get(callback_event, 1) == 12)
    out ":"
    out windows::next_callback(callback) == NONE
    out ":"
    out windows::destroy_callback(callback)
    ProcessResult child = ProcessResult(windows::run_process("cmd.exe", ["/D", "/V:ON", "/S", "/C", "set /p value=& <nul set /p =OUT:!value!& echo ERR>&2& exit /b 7"], "hello\n", ""))
    out ":"
    out child.exit_code == 7
    out ":"
    out child.stdout == "OUT:hello"
    out ":"
    out std::contains(child.stderr, "ERR")
    VTime async_child = windows::start_process("cmd.exe", ["/D", "/S", "/C", "echo READY& ping -n 2 127.0.0.1 >nul & echo DONE"], "", "")
    ProcessSnapshot live_child = ProcessSnapshot(windows::poll_process(async_child))
    out ":"
    out live_child.running
    ProcessResult async_result = ProcessResult(windows::wait_process(async_child))
    out ":"
    out async_result.exit_code == 0
    out ":"
    out std::contains(async_result.stdout, "READY")
    out ":"
    out std::contains(async_result.stdout, "DONE")
    VTime terminated_child = windows::start_process("cmd.exe", ["/D", "/S", "/C", "ping -n 30 127.0.0.1 >nul"], "", "")
    ProcessResult terminated_result = ProcessResult(windows::terminate_process(terminated_child))
    out ":"
    out terminated_result.exit_code != 0
    Accelerators shortcuts = Accelerators([[78, 203, 0]])
    WindowConfig config = WindowConfig()
    config.title = "APL test"
    config.width = 320
    config.height = 200
    config.accelerators = shortcuts.identity
    VTime window = config.create()
    out ":"
    out window > 0
    while (windows.next_event() != NONE) (-1) {
    }
    out ":"
    out windows::enable_window(window, false)
    out ":"
    out windows::enable_window(window, true)
    out ":"
    out windows::invalidate_window(window, false)
    out ":"
    out windows::set_timer(window, 601, 10)
    WindowEvent timer_event = WindowEvent(windows::wait_event())
    out ":"
    out timer_event.is_timer(601)
    out ":"
    out windows::kill_timer(window, 601)
    Menu menu_bar = Menu(false)
    Menu file_menu = Menu(true)
    file_menu.item(201, "Open")
    file_menu.separator()
    file_menu.item(202, "Exit")
    menu_bar.submenu(file_menu.identity, "File")
    menu_bar.attach(window)
    file_menu.enable(201, false)
    file_menu.enable(201, true)
    file_menu.check(201, true)
    file_menu.check(201, false)
    ControlConfig control = ControlConfig()
    control.class_name = windows::CLASS_RICHEDIT
    control.text = "one\r\ntwo"
    control.id = 101
    control.style += windows::ES_MULTILINE
    control.style += windows::ES_AUTOVSCROLL
    control.style += windows::WS_VSCROLL
    VTime label = control.create(window)
    out ":"
    out label > 0
    out ":"
    out windows::line_count(label) == 2
    out ":"
    out windows::set_selection(label, 0, 3)
    VTime selected = windows::selection(label)
    out ":"
    out (std::get(selected, 0) == 0) and (std::get(selected, 1) == 3)
    out ":"
    out windows::replace_selection(label, "three", true)
    out ":"
    out windows::window_text(label) == "three\r\ntwo"
    out ":"
    out windows::set_text_color(label, 0, 5, windows::rgb(255, 128, 0))
    out ":"
    out windows::set_background_color(label, 0, 5, windows::rgb(255, 255, 224))
    out ":"
    out windows::set_text_bold(label, 0, 5, true)
    VTime caret = windows::selection(label)
    out ":"
    out (std::get(caret, 0) == 5) and (std::get(caret, 1) == 5)
    out ":"
    out windows::set_text_font(label, 0, -1, "Consolas", 11)
    out ":"
    out windows::can_undo(label)
    out ":"
    out windows::undo(label)
    out ":"
    out windows::window_text(label) == "one\r\ntwo"
    out ":"
    out windows::can_redo(label)
    out ":"
    out windows::redo(label)
    out ":"
    out windows::window_text(label) == "three\r\ntwo"
    out ":"
    out windows::clear_history(label)
    out ":"
    out windows::can_undo(label) == false
    out ":"
    out windows::can_redo(label) == false
    out ":"
    out windows::enable_event(label, windows::ENM_SELCHANGE)
    out ":"
    out windows::_has_flag(windows::event_mask(label), windows::ENM_SELCHANGE)
    out ":"
    out windows::disable_event(label, windows::ENM_SELCHANGE)
    out ":"
    out windows::_has_flag(windows::event_mask(label), windows::ENM_SELCHANGE) == false
    out ":"
    out windows::first_visible_line(label) == 0
    out ":"
    out windows::scroll_lines(label, 1)
    out ":"
    out windows::focus(label)
    VTime found = windows::find_text(label, "THREE", 0, -1, false, true)
    out ":"
    out (std::get(found, 0) == 0) and (std::get(found, 1) == 5)
    out ":"
    out windows::find_text(label, "THREE", 0, -1, true, true) == NONE
    out ":"
    out windows::move_window(label, 8, 8, 160, 28, true)
    menu_bar.destroy()
    shortcuts.destroy()
    TestWindowApplication concrete_application = TestWindowApplication(window)
    WindowApplication application = concrete_application
    out ":"
    out windows::destroy_window(window)
    application.run()
    out ":"
    out application.main_destroyed
    out ":"
    out concrete_application.destroy_handled
    out ":"
    out windows::run_message_loop() == 0
    VTime second_window = windows::create_window("APL pump test", 320, 200)
    VTime third_window = windows::create_window("APL second pump test", 320, 200)
    windows::destroy_window(second_window)
    out ":"
    out windows::pump_messages() == true
    windows::destroy_window(third_window)
    out ":"
    out windows::pump_messages() == false
  }
  return 0
}
"#;

#[test]
fn windows_library_runs_through_portable_and_native_paths() {
    let project = TestProject::new();
    project.build_module();
    let source = project.write_program("windows_program.apl", WINDOWS_PROGRAM);

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
    let expected = if cfg!(windows) {
        std::iter::once("true")
            .chain(std::iter::once("4"))
            .chain(std::iter::repeat("true").take(57))
            .collect::<Vec<_>>()
            .join(":")
    } else {
        "false:4".to_owned()
    };
    assert_eq!(String::from_utf8(portable.stdout).unwrap(), expected);

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
        "windows_program_compiled_native{}",
        std::env::consts::EXE_SUFFIX
    ));
    let native = Command::new(&executable).output().unwrap();
    assert!(
        native.status.success(),
        "{}",
        String::from_utf8_lossy(&native.stderr)
    );
    assert_eq!(String::from_utf8(native.stdout).unwrap(), expected);
}

#[test]
#[cfg(windows)]
fn native_executable_embeds_and_loads_its_configured_icon() {
    let project = TestProject::new();
    project.build_module();
    let source = project.write_program(
        "resource_icon.apl",
        r#"include >> windows
AVInt std::main() {
  VTime icon = windows::load_resource_icon(1, 16, 16)
  WindowConfig config = WindowConfig()
  config.title = "APL resource icon"
  config.width = 320
  config.height = 200
  config.icon = icon
  VTime window = config.create()
  AVBool created = window > 0
  windows::destroy_window(window)
  AVBool released = windows::destroy_icon(icon)
  out created and released
  return 0
}
"#,
    );
    fs::write(project.path("resource_icon.ico"), minimal_icon()).unwrap();
    fs::write(
        project.path("resource_icon.build.toml"),
        "format = 1\n\n[windows]\nicon = \"resource_icon.ico\"\n",
    )
    .unwrap();

    let package = project.path("resource_icon_package");
    let generated = apl()
        .current_dir(&project.root)
        .arg("build-native")
        .arg(&source)
        .arg(&package)
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let built = Command::new("cargo")
        .env_remove("CARGO_TARGET_DIR")
        .arg("build")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(package.join("Cargo.toml"))
        .current_dir(&package)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );

    let executable = package.join("target").join("debug").join(format!(
        "resource_icon_compiled_native{}",
        std::env::consts::EXE_SUFFIX
    ));
    let output = Command::new(executable).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "true");
}

#[test]
#[cfg(windows)]
fn destroyed_windows_capabilities_cannot_be_called_again() {
    let project = TestProject::new();
    project.build_module();
    let source = project.write_program(
        "revoked_procedure.apl",
        r#"include >> windows
AVInt std::main() {
  VTime library = windows::load_library("kernel32.dll")
  VTime procedure = windows::get_procedure(library, "GetCurrentProcessId", [], "U32")
  windows::close_library(library)
  windows::call(procedure, [])
  return 0
}
"#,
    );

    let output = apl()
        .current_dir(&project.root)
        .arg("run")
        .arg(&source)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("is unloaded"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let source = project.write_program(
        "revoked_callback.apl",
        r#"include >> windows
AVInt std::main() {
  VTime callback = windows::create_callback(windows::CALLBACK_WNDPROC, 0)
  windows::destroy_callback(callback)
  windows::call(callback, [0, 0, 0, 0])
  return 0
}
"#,
    );
    let output = apl()
        .current_dir(&project.root)
        .arg("run")
        .arg(&source)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("is unloaded"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
