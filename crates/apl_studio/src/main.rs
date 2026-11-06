#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("APL Studio is available on Windows only");
}

#[cfg(windows)]
mod windows_app {
    use std::{
        collections::HashSet,
        ffi::{c_int, c_void, OsStr},
        fs,
        mem::{size_of, zeroed},
        os::windows::ffi::OsStrExt,
        os::windows::process::CommandExt,
        path::{Path, PathBuf},
        process::Command,
        ptr::{null, null_mut},
        sync::mpsc::{self, Receiver, Sender, TryRecvError},
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    type Bool = i32;
    type Dword = u32;
    type Hbrush = *mut c_void;
    type Hcursor = *mut c_void;
    type Hfont = *mut c_void;
    type Hinstance = *mut c_void;
    type Hkey = *mut c_void;
    type Hmenu = *mut c_void;
    type Hwnd = *mut c_void;
    type Lparam = isize;
    type Lresult = isize;
    type Uint = u32;
    type Wparam = usize;

    type WndProc = Option<unsafe extern "system" fn(Hwnd, Uint, Wparam, Lparam) -> Lresult>;

    #[repr(C)]
    struct Accel {
        virtual_key: u8,
        key: u16,
        command: u16,
    }

    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    struct Msg {
        hwnd: Hwnd,
        message: Uint,
        w_param: Wparam,
        l_param: Lparam,
        time: Dword,
        point: Point,
        private: Dword,
    }

    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CharRange {
        min: i32,
        max: i32,
    }

    #[repr(C)]
    struct CharFormat2W {
        size: Dword,
        mask: Dword,
        effects: Dword,
        height: i32,
        offset: i32,
        text_color: Dword,
        charset: u8,
        pitch_and_family: u8,
        face_name: [u16; 32],
        weight: u16,
        spacing: i16,
        back_color: Dword,
        locale: Dword,
        reserved: Dword,
        style: i16,
        kerning: u16,
        underline_type: u8,
        animation: u8,
        revision_author: u8,
        reserved_byte: u8,
    }

    #[repr(C)]
    struct NmHdr {
        hwnd_from: Hwnd,
        id_from: usize,
        code: Uint,
    }

    #[repr(C)]
    struct WndClassW {
        style: Uint,
        wnd_proc: WndProc,
        cls_extra: c_int,
        wnd_extra: c_int,
        instance: Hinstance,
        icon: *mut c_void,
        cursor: Hcursor,
        background: Hbrush,
        menu_name: *const u16,
        class_name: *const u16,
    }

    #[repr(C)]
    struct CreateStructW {
        create_params: *mut c_void,
        instance: Hinstance,
        menu: Hmenu,
        parent: Hwnd,
        cy: c_int,
        cx: c_int,
        y: c_int,
        x: c_int,
        style: i32,
        name: *const u16,
        class: *const u16,
        ex_style: Dword,
    }

    #[repr(C)]
    struct MinMaxInfo {
        reserved: Point,
        max_size: Point,
        max_position: Point,
        min_track_size: Point,
        max_track_size: Point,
    }

    #[repr(C)]
    struct OpenFileNameW {
        struct_size: Dword,
        owner: Hwnd,
        instance: Hinstance,
        filter: *const u16,
        custom_filter: *mut u16,
        max_custom_filter: Dword,
        filter_index: Dword,
        file: *mut u16,
        max_file: Dword,
        file_title: *mut u16,
        max_file_title: Dword,
        initial_dir: *const u16,
        title: *const u16,
        flags: Dword,
        file_offset: u16,
        file_extension: u16,
        default_extension: *const u16,
        custom_data: Lparam,
        hook: *mut c_void,
        template_name: *const u16,
        reserved: *mut c_void,
        reserved_flags: Dword,
        flags_ex: Dword,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(module_name: *const u16) -> Hinstance;
        fn LoadLibraryW(file_name: *const u16) -> Hinstance;
    }

    #[link(name = "user32")]
    extern "system" {
        fn AppendMenuW(menu: Hmenu, flags: Uint, id: usize, text: *const u16) -> Bool;
        fn CreateAcceleratorTableW(accelerators: *const Accel, count: c_int) -> *mut c_void;
        fn CreateMenu() -> Hmenu;
        fn CreatePopupMenu() -> Hmenu;
        fn CreateWindowExW(
            ex_style: Dword,
            class_name: *const u16,
            window_name: *const u16,
            style: Dword,
            x: c_int,
            y: c_int,
            width: c_int,
            height: c_int,
            parent: Hwnd,
            menu: Hmenu,
            instance: Hinstance,
            param: *mut c_void,
        ) -> Hwnd;
        fn DefWindowProcW(hwnd: Hwnd, message: Uint, w_param: Wparam, l_param: Lparam) -> Lresult;
        fn DestroyAcceleratorTable(accelerator: *mut c_void) -> Bool;
        fn DestroyWindow(hwnd: Hwnd) -> Bool;
        fn DispatchMessageW(message: *const Msg) -> Lresult;
        fn EnableWindow(hwnd: Hwnd, enabled: Bool) -> Bool;
        fn GetClientRect(hwnd: Hwnd, rect: *mut Rect) -> Bool;
        fn GetMessageW(message: *mut Msg, hwnd: Hwnd, min: Uint, max: Uint) -> Bool;
        fn GetWindowLongPtrW(hwnd: Hwnd, index: c_int) -> isize;
        fn GetWindowTextLengthW(hwnd: Hwnd) -> c_int;
        fn GetWindowTextW(hwnd: Hwnd, text: *mut u16, max_count: c_int) -> c_int;
        fn InvalidateRect(hwnd: Hwnd, rect: *const Rect, erase: Bool) -> Bool;
        fn KillTimer(hwnd: Hwnd, id: usize) -> Bool;
        fn LoadCursorW(instance: Hinstance, cursor_name: *const u16) -> Hcursor;
        fn MessageBoxW(hwnd: Hwnd, text: *const u16, caption: *const u16, kind: Uint) -> c_int;
        fn MoveWindow(
            hwnd: Hwnd,
            x: c_int,
            y: c_int,
            width: c_int,
            height: c_int,
            repaint: Bool,
        ) -> Bool;
        fn PostMessageW(hwnd: Hwnd, message: Uint, w_param: Wparam, l_param: Lparam) -> Bool;
        fn PostQuitMessage(exit_code: c_int);
        fn RegisterClassW(class: *const WndClassW) -> u16;
        fn SendMessageW(hwnd: Hwnd, message: Uint, w_param: Wparam, l_param: Lparam) -> Lresult;
        fn SetFocus(hwnd: Hwnd) -> Hwnd;
        fn SetMenu(hwnd: Hwnd, menu: Hmenu) -> Bool;
        fn SetTimer(hwnd: Hwnd, id: usize, timeout_ms: Uint, callback: *mut c_void) -> usize;
        fn SetWindowLongPtrW(hwnd: Hwnd, index: c_int, value: isize) -> isize;
        fn SetWindowTextW(hwnd: Hwnd, text: *const u16) -> Bool;
        fn ShowWindow(hwnd: Hwnd, command: c_int) -> Bool;
        fn TranslateMessage(message: *const Msg) -> Bool;
        fn TranslateAcceleratorW(
            hwnd: Hwnd,
            accelerator: *mut c_void,
            message: *const Msg,
        ) -> c_int;
        fn UpdateWindow(hwnd: Hwnd) -> Bool;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateFontW(
            height: c_int,
            width: c_int,
            escapement: c_int,
            orientation: c_int,
            weight: c_int,
            italic: Dword,
            underline: Dword,
            strike_out: Dword,
            charset: Dword,
            output_precision: Dword,
            clip_precision: Dword,
            quality: Dword,
            pitch_and_family: Dword,
            face: *const u16,
        ) -> Hfont;
        fn DeleteObject(object: *mut c_void) -> Bool;
    }

    #[link(name = "comdlg32")]
    extern "system" {
        fn GetOpenFileNameW(file_name: *mut OpenFileNameW) -> Bool;
        fn GetSaveFileNameW(file_name: *mut OpenFileNameW) -> Bool;
    }

    #[link(name = "advapi32")]
    extern "system" {
        fn RegCloseKey(key: Hkey) -> i32;
        fn RegCreateKeyExW(
            key: Hkey,
            sub_key: *const u16,
            reserved: Dword,
            class: *mut u16,
            options: Dword,
            desired_access: Dword,
            security_attributes: *mut c_void,
            result: *mut Hkey,
            disposition: *mut Dword,
        ) -> i32;
        fn RegSetValueExW(
            key: Hkey,
            value_name: *const u16,
            reserved: Dword,
            value_type: Dword,
            data: *const u8,
            data_size: Dword,
        ) -> i32;
    }

    #[link(name = "shell32")]
    extern "system" {
        fn SHChangeNotify(event_id: i32, flags: Uint, item1: *const c_void, item2: *const c_void);
    }

    const CLASS_NAME: &str = "APLStudioWindow";
    const APP_TITLE: &str = "APL Studio";
    const STARTER_SOURCE: &str = "AVInt std::main() {\r\n  AVInt x = 4\r\n  AVInt y = (x + 2) * 3\r\n\r\n  out y\r\n  return 0\r\n}\r\n";

    const ID_OPEN: usize = 1001;
    const ID_SAVE: usize = 1002;
    const ID_SAVE_AS: usize = 1003;
    const ID_RUN: usize = 1004;
    const ID_EXIT: usize = 1005;
    const ID_BUILD: usize = 1006;
    const ID_UNDO: usize = 1007;
    const ID_REDO: usize = 1008;
    const ID_FIND: usize = 1009;
    const ID_FIND_NEXT: usize = 1010;
    const ID_REPLACE: usize = 1011;
    const ID_REPLACE_ALL: usize = 1012;
    const ID_DEBUG: usize = 1013;
    const ID_DEBUG_CONTINUE: usize = 1014;
    const ID_DEBUG_PAUSE: usize = 1015;
    const ID_DEBUG_STOP: usize = 1016;
    const ID_STEP_INTO: usize = 1017;
    const ID_STEP_OVER: usize = 1018;
    const ID_STEP_OUT: usize = 1019;
    const ID_BREAKPOINT: usize = 1020;
    const ID_EDITOR: usize = 1101;
    const ID_OUTPUT: usize = 1102;
    const ID_INPUT: usize = 1103;
    const ID_GUTTER: usize = 1104;
    const ID_FIND_TEXT: usize = 1105;
    const ID_REPLACE_TEXT: usize = 1106;
    const ID_DEBUG_PANEL: usize = 1107;

    const WM_CREATE: Uint = 0x0001;
    const WM_DESTROY: Uint = 0x0002;
    const WM_SIZE: Uint = 0x0005;
    const WM_SETFOCUS: Uint = 0x0007;
    const WM_CLOSE: Uint = 0x0010;
    const WM_COMMAND: Uint = 0x0111;
    const WM_TIMER: Uint = 0x0113;
    const WM_NOTIFY: Uint = 0x004e;
    const WM_GETMINMAXINFO: Uint = 0x0024;
    const WM_SETFONT: Uint = 0x0030;
    const WM_NCCREATE: Uint = 0x0081;
    const WM_NCDESTROY: Uint = 0x0082;
    const WM_APP_RUN_COMPLETE: Uint = 0x8001;
    const WM_APP_BUILD_COMPLETE: Uint = 0x8002;
    const WM_APP_DEBUG_PAUSED: Uint = 0x8003;
    const WM_APP_DEBUG_COMPLETE: Uint = 0x8004;

    const EM_SETSEL: Uint = 0x00b1;
    const EM_REPLACESEL: Uint = 0x00c2;
    const EM_GETFIRSTVISIBLELINE: Uint = 0x00ce;
    const EM_LINESCROLL: Uint = 0x00b6;
    const EM_EXGETSEL: Uint = 0x0434;
    const EM_EXSETSEL: Uint = 0x0437;
    const EM_SETCHARFORMAT: Uint = 0x0444;
    const EM_GETEVENTMASK: Uint = 0x043b;
    const EM_SETEVENTMASK: Uint = 0x0445;
    const EM_REDO: Uint = 0x0454;
    const SCF_SELECTION: Wparam = 0x0001;
    const CFM_BOLD: Dword = 0x0000_0001;
    const CFM_COLOR: Dword = 0x4000_0000;
    const CFM_BACKCOLOR: Dword = 0x0400_0000;
    const CFE_BOLD: Dword = 0x0000_0001;

    const GWLP_USERDATA: c_int = -21;
    const EN_CHANGE: usize = 0x0300;
    const EN_VSCROLL: usize = 0x0602;
    const EN_SELCHANGE: Uint = 0x0702;
    const ENM_SELCHANGE: Lparam = 0x0008_0000;
    const HIGHLIGHT_TIMER: usize = 1;
    const HIGHLIGHT_DELAY_MS: Uint = 180;
    const BN_CLICKED: usize = 0;

    const WS_OVERLAPPEDWINDOW: Dword = 0x00cf_0000;
    const WS_CHILD: Dword = 0x4000_0000;
    const WS_VISIBLE: Dword = 0x1000_0000;
    const WS_BORDER: Dword = 0x0080_0000;
    const WS_VSCROLL: Dword = 0x0020_0000;
    const WS_HSCROLL: Dword = 0x0010_0000;
    const WS_TABSTOP: Dword = 0x0001_0000;
    const ES_MULTILINE: Dword = 0x0004;
    const ES_AUTOVSCROLL: Dword = 0x0040;
    const ES_AUTOHSCROLL: Dword = 0x0080;
    const ES_READONLY: Dword = 0x0800;
    const ES_WANTRETURN: Dword = 0x1000;
    const BS_PUSHBUTTON: Dword = 0;
    const SS_LEFT: Dword = 0;
    const CW_USEDEFAULT: c_int = i32::MIN;
    const SW_SHOWDEFAULT: c_int = 10;
    const SW_HIDE: c_int = 0;
    const SW_SHOW: c_int = 5;
    const COLOR_WINDOW: usize = 5;
    const IDC_ARROW: *const u16 = 32512usize as *const u16;
    const FVIRTKEY: u8 = 0x01;
    const FCONTROL: u8 = 0x08;
    const VK_F5: u16 = 0x74;
    const VK_F7: u16 = 0x76;
    const VK_F6: u16 = 0x75;
    const VK_F8: u16 = 0x77;
    const VK_F9: u16 = 0x78;

    const MF_STRING: Uint = 0;
    const MF_POPUP: Uint = 0x0010;
    const MF_SEPARATOR: Uint = 0x0800;

    const MB_ICONERROR: Uint = 0x0010;
    const MB_ICONWARNING: Uint = 0x0030;
    const MB_YESNOCANCEL: Uint = 0x0003;
    const IDYES: c_int = 6;
    const IDNO: c_int = 7;
    const IDCANCEL: c_int = 2;

    const OFN_OVERWRITEPROMPT: Dword = 0x0002;
    const OFN_FILEMUSTEXIST: Dword = 0x1000;
    const OFN_PATHMUSTEXIST: Dword = 0x0800;
    const OFN_EXPLORER: Dword = 0x0008_0000;
    const KEY_SET_VALUE: Dword = 0x0002;
    const REG_NONE: Dword = 0;
    const REG_SZ: Dword = 1;
    const SHCNE_ASSOCCHANGED: i32 = 0x0800_0000;
    const HKEY_CURRENT_USER: Hkey = 0x8000_0001usize as Hkey;

    struct StudioResult {
        text: String,
        diagnostic: Option<(usize, usize)>,
    }

    #[derive(Clone, Copy)]
    enum DebugCommand {
        Continue,
        Pause,
        Stop,
        StepInto,
        StepOver,
        StepOut,
    }

    enum DebugRunMode {
        Continue,
        StepInto,
        StepOver(usize),
        StepOut(usize),
    }

    struct DebugEvent {
        snapshot: apl_runtime::DebugSnapshot,
    }

    struct AppState {
        editor: Hwnd,
        gutter: Hwnd,
        input: Hwnd,
        input_label: Hwnd,
        output: Hwnd,
        output_label: Hwnd,
        path_label: Hwnd,
        open_button: Hwnd,
        save_button: Hwnd,
        run_button: Hwnd,
        build_button: Hwnd,
        find_text: Hwnd,
        replace_text: Hwnd,
        find_next_button: Hwnd,
        replace_button: Hwnd,
        replace_all_button: Hwnd,
        debug_button: Hwnd,
        continue_button: Hwnd,
        pause_button: Hwnd,
        stop_button: Hwnd,
        step_into_button: Hwnd,
        step_over_button: Hwnd,
        step_out_button: Hwnd,
        debug_panel: Hwnd,
        font: Hfont,
        current_path: Option<PathBuf>,
        dirty: bool,
        suppress_change: bool,
        running: bool,
        building: bool,
        find_visible: bool,
        last_source: String,
        gutter_first_line: i32,
        debug_sender: Option<Sender<DebugCommand>>,
        debug_active: bool,
        breakpoints: HashSet<usize>,
        matched_brackets: Option<(i32, i32)>,
        formatting: bool,
        startup_path: Option<PathBuf>,
    }

    impl AppState {
        fn new(startup_path: Option<PathBuf>) -> Self {
            Self {
                editor: null_mut(),
                gutter: null_mut(),
                input: null_mut(),
                input_label: null_mut(),
                output: null_mut(),
                output_label: null_mut(),
                path_label: null_mut(),
                open_button: null_mut(),
                save_button: null_mut(),
                run_button: null_mut(),
                build_button: null_mut(),
                find_text: null_mut(),
                replace_text: null_mut(),
                find_next_button: null_mut(),
                replace_button: null_mut(),
                replace_all_button: null_mut(),
                debug_button: null_mut(),
                continue_button: null_mut(),
                pause_button: null_mut(),
                stop_button: null_mut(),
                step_into_button: null_mut(),
                step_over_button: null_mut(),
                step_out_button: null_mut(),
                debug_panel: null_mut(),
                font: null_mut(),
                current_path: None,
                dirty: false,
                suppress_change: true,
                running: false,
                building: false,
                find_visible: false,
                last_source: STARTER_SOURCE.replace("\r\n", "\n"),
                gutter_first_line: 0,
                debug_sender: None,
                debug_active: false,
                breakpoints: HashSet::new(),
                matched_brackets: None,
                formatting: false,
                startup_path,
            }
        }
    }

    impl Drop for AppState {
        fn drop(&mut self) {
            if !self.font.is_null() {
                unsafe { DeleteObject(self.font) };
            }
        }
    }

    pub fn run() {
        unsafe {
            if LoadLibraryW(wide("Msftedit.dll").as_ptr()).is_null() {
                show_error(null_mut(), "Windows RichEdit could not be loaded.");
                return;
            }
            let instance = GetModuleHandleW(null());
            let class_name = wide(CLASS_NAME);
            let class = WndClassW {
                style: 0x0001 | 0x0002,
                wnd_proc: Some(window_proc),
                cls_extra: 0,
                wnd_extra: 0,
                instance,
                icon: null_mut(),
                cursor: LoadCursorW(null_mut(), IDC_ARROW),
                background: (COLOR_WINDOW + 1) as Hbrush,
                menu_name: null(),
                class_name: class_name.as_ptr(),
            };

            if RegisterClassW(&class) == 0 {
                show_error(
                    null_mut(),
                    "Could not register the APL Studio window class.",
                );
                return;
            }

            let startup_path = std::env::args_os().nth(1).map(PathBuf::from);
            let state = Box::new(AppState::new(startup_path));
            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                wide(APP_TITLE).as_ptr(),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                1050,
                760,
                null_mut(),
                null_mut(),
                instance,
                Box::into_raw(state).cast(),
            );

            if hwnd.is_null() {
                show_error(null_mut(), "Could not create the APL Studio window.");
                return;
            }

            ShowWindow(hwnd, SW_SHOWDEFAULT);
            UpdateWindow(hwnd);

            // Registry and shell notification calls can block while Explorer is busy.
            // File association maintenance must never delay creation of the editor window.
            thread::spawn(|| {
                let _ = register_file_association();
            });

            let accelerators = [
                Accel {
                    virtual_key: FVIRTKEY | FCONTROL,
                    key: b'O' as u16,
                    command: ID_OPEN as u16,
                },
                Accel {
                    virtual_key: FVIRTKEY | FCONTROL,
                    key: b'S' as u16,
                    command: ID_SAVE as u16,
                },
                Accel {
                    virtual_key: FVIRTKEY,
                    key: VK_F5,
                    command: ID_RUN as u16,
                },
                Accel {
                    virtual_key: FVIRTKEY,
                    key: VK_F7,
                    command: ID_BUILD as u16,
                },
                Accel {
                    virtual_key: FVIRTKEY | FCONTROL,
                    key: b'Z' as u16,
                    command: ID_UNDO as u16,
                },
                Accel {
                    virtual_key: FVIRTKEY | FCONTROL,
                    key: b'Y' as u16,
                    command: ID_REDO as u16,
                },
                Accel {
                    virtual_key: FVIRTKEY | FCONTROL,
                    key: b'F' as u16,
                    command: ID_FIND as u16,
                },
                Accel {
                    virtual_key: FVIRTKEY,
                    key: VK_F6,
                    command: ID_DEBUG as u16,
                },
                Accel {
                    virtual_key: FVIRTKEY,
                    key: VK_F8,
                    command: ID_DEBUG_CONTINUE as u16,
                },
                Accel {
                    virtual_key: FVIRTKEY,
                    key: VK_F9,
                    command: ID_BREAKPOINT as u16,
                },
            ];
            let accelerator =
                CreateAcceleratorTableW(accelerators.as_ptr(), accelerators.len() as c_int);

            let mut message: Msg = zeroed();
            while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
                if accelerator.is_null() || TranslateAcceleratorW(hwnd, accelerator, &message) == 0
                {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            if !accelerator.is_null() {
                DestroyAcceleratorTable(accelerator);
            }
        }
    }

    unsafe extern "system" fn window_proc(
        hwnd: Hwnd,
        message: Uint,
        w_param: Wparam,
        l_param: Lparam,
    ) -> Lresult {
        if message == WM_NCCREATE {
            let create = &*(l_param as *const CreateStructW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.create_params as isize);
        }

        let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;

        match message {
            WM_CREATE => {
                if state_ptr.is_null() {
                    return -1;
                }
                create_ui(hwnd, &mut *state_ptr);
                0
            }
            WM_SIZE => {
                if !state_ptr.is_null() {
                    layout(hwnd, &*state_ptr);
                }
                0
            }
            WM_GETMINMAXINFO => {
                let info = &mut *(l_param as *mut MinMaxInfo);
                info.min_track_size.x = 640;
                info.min_track_size.y = 480;
                0
            }
            WM_SETFOCUS => {
                if !state_ptr.is_null() {
                    SetFocus((*state_ptr).editor);
                }
                0
            }
            WM_COMMAND => {
                if !state_ptr.is_null() {
                    let id = w_param & 0xffff;
                    let notification = (w_param >> 16) & 0xffff;
                    handle_command(hwnd, &mut *state_ptr, id, notification);
                }
                0
            }
            WM_NOTIFY => {
                if !state_ptr.is_null() && l_param != 0 {
                    let header = &*(l_param as *const NmHdr);
                    if header.id_from == ID_EDITOR
                        && header.code == EN_SELCHANGE
                        && !(*state_ptr).formatting
                    {
                        schedule_highlight(hwnd);
                    }
                }
                0
            }
            WM_TIMER => {
                if w_param == HIGHLIGHT_TIMER && !state_ptr.is_null() {
                    KillTimer(hwnd, HIGHLIGHT_TIMER);
                    let state = &mut *state_ptr;
                    highlight_source(state);
                    highlight_matching_brackets(state);
                    InvalidateRect(state.editor, null(), 0);
                    UpdateWindow(state.editor);
                }
                0
            }
            WM_APP_RUN_COMPLETE => {
                if !state_ptr.is_null() && l_param != 0 {
                    let result = Box::from_raw(l_param as *mut StudioResult);
                    let state = &mut *state_ptr;
                    set_text(state.output, &display_output(&result.text));
                    if let Some((line, column)) = result.diagnostic {
                        navigate_to(state.editor, &get_text(state.editor), line, column);
                    }
                    state.running = false;
                    EnableWindow(state.run_button, 1);
                    set_text(state.output_label, "Output");
                }
                0
            }
            WM_APP_BUILD_COMPLETE => {
                if !state_ptr.is_null() && l_param != 0 {
                    let result = Box::from_raw(l_param as *mut StudioResult);
                    let state = &mut *state_ptr;
                    set_text(state.output, &display_output(&result.text));
                    if let Some((line, column)) = result.diagnostic {
                        navigate_to(state.editor, &get_text(state.editor), line, column);
                    }
                    state.building = false;
                    EnableWindow(state.build_button, 1);
                    set_text(state.output_label, "Output");
                }
                0
            }
            WM_APP_DEBUG_PAUSED => {
                if !state_ptr.is_null() && l_param != 0 {
                    let event = Box::from_raw(l_param as *mut DebugEvent);
                    let state = &mut *state_ptr;
                    show_debug_snapshot(state, &event.snapshot);
                    navigate_to(
                        state.editor,
                        &get_text(state.editor),
                        event.snapshot.location.line,
                        event.snapshot.location.column,
                    );
                    set_debug_controls(state, true);
                    set_text(state.output_label, "Debugger paused");
                }
                0
            }
            WM_APP_DEBUG_COMPLETE => {
                if !state_ptr.is_null() && l_param != 0 {
                    let result = Box::from_raw(l_param as *mut StudioResult);
                    let state = &mut *state_ptr;
                    state.debug_active = false;
                    state.debug_sender = None;
                    set_debug_controls(state, false);
                    show_debug_ui(state, false);
                    set_text(state.output, &display_output(&result.text));
                    if let Some((line, column)) = result.diagnostic {
                        navigate_to(state.editor, &get_text(state.editor), line, column);
                    }
                    set_text(state.output_label, "Output");
                    layout(hwnd, state);
                }
                0
            }
            WM_CLOSE => {
                if state_ptr.is_null() || confirm_discard(hwnd, &mut *state_ptr) {
                    DestroyWindow(hwnd);
                }
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            WM_NCDESTROY => {
                if !state_ptr.is_null() {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    drop(Box::from_raw(state_ptr));
                }
                DefWindowProcW(hwnd, message, w_param, l_param)
            }
            _ => DefWindowProcW(hwnd, message, w_param, l_param),
        }
    }

    unsafe fn create_ui(hwnd: Hwnd, state: &mut AppState) {
        let instance = GetModuleHandleW(null());
        let menu = CreateMenu();
        let file_menu = CreatePopupMenu();
        AppendMenuW(
            file_menu,
            MF_STRING,
            ID_OPEN,
            wide("&Open...\tCtrl+O").as_ptr(),
        );
        AppendMenuW(
            file_menu,
            MF_STRING,
            ID_SAVE,
            wide("&Save\tCtrl+S").as_ptr(),
        );
        AppendMenuW(
            file_menu,
            MF_STRING,
            ID_SAVE_AS,
            wide("Save &As...").as_ptr(),
        );
        AppendMenuW(
            file_menu,
            MF_STRING,
            ID_BUILD,
            wide("&Build EXE...\tF7").as_ptr(),
        );
        AppendMenuW(file_menu, MF_SEPARATOR, 0, null());
        AppendMenuW(file_menu, MF_STRING, ID_EXIT, wide("E&xit").as_ptr());
        AppendMenuW(menu, MF_POPUP, file_menu as usize, wide("&File").as_ptr());
        let edit_menu = CreatePopupMenu();
        AppendMenuW(
            edit_menu,
            MF_STRING,
            ID_UNDO,
            wide("&Undo\tCtrl+Z").as_ptr(),
        );
        AppendMenuW(
            edit_menu,
            MF_STRING,
            ID_REDO,
            wide("&Redo\tCtrl+Y").as_ptr(),
        );
        AppendMenuW(edit_menu, MF_SEPARATOR, 0, null());
        AppendMenuW(
            edit_menu,
            MF_STRING,
            ID_FIND,
            wide("&Find / Replace\tCtrl+F").as_ptr(),
        );
        AppendMenuW(menu, MF_POPUP, edit_menu as usize, wide("&Edit").as_ptr());
        let run_menu = CreatePopupMenu();
        AppendMenuW(run_menu, MF_STRING, ID_RUN, wide("&Run\tF5").as_ptr());
        AppendMenuW(run_menu, MF_STRING, ID_DEBUG, wide("&Debug\tF6").as_ptr());
        AppendMenuW(
            run_menu,
            MF_STRING,
            ID_BUILD,
            wide("&Build EXE...\tF7").as_ptr(),
        );
        AppendMenuW(run_menu, MF_SEPARATOR, 0, null());
        AppendMenuW(
            run_menu,
            MF_STRING,
            ID_DEBUG_CONTINUE,
            wide("&Continue\tF8").as_ptr(),
        );
        AppendMenuW(
            run_menu,
            MF_STRING,
            ID_BREAKPOINT,
            wide("Toggle &Breakpoint\tF9").as_ptr(),
        );
        AppendMenuW(menu, MF_POPUP, run_menu as usize, wide("&Run").as_ptr());
        SetMenu(hwnd, menu);

        state.open_button = create_control(
            instance,
            hwnd,
            "BUTTON",
            "Open",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON,
            ID_OPEN,
        );
        state.save_button = create_control(
            instance,
            hwnd,
            "BUTTON",
            "Save",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON,
            ID_SAVE,
        );
        state.run_button = create_control(
            instance,
            hwnd,
            "BUTTON",
            "Run  F5",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON,
            ID_RUN,
        );
        state.build_button = create_control(
            instance,
            hwnd,
            "BUTTON",
            "Build  F7",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON,
            ID_BUILD,
        );
        state.debug_button = create_control(
            instance,
            hwnd,
            "BUTTON",
            "Debug  F6",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON,
            ID_DEBUG,
        );
        state.path_label = create_control(
            instance,
            hwnd,
            "STATIC",
            "Untitled.apl",
            WS_CHILD | WS_VISIBLE | SS_LEFT,
            0,
        );
        state.gutter = create_control(
            instance,
            hwnd,
            "RICHEDIT50W",
            "1",
            WS_CHILD | WS_VISIBLE | ES_MULTILINE | ES_READONLY,
            ID_GUTTER,
        );
        state.editor = create_control(
            instance,
            hwnd,
            "RICHEDIT50W",
            STARTER_SOURCE,
            WS_CHILD
                | WS_VISIBLE
                | WS_BORDER
                | WS_TABSTOP
                | WS_VSCROLL
                | WS_HSCROLL
                | ES_MULTILINE
                | ES_AUTOVSCROLL
                | ES_AUTOHSCROLL
                | ES_WANTRETURN,
            ID_EDITOR,
        );
        state.find_text = create_control(
            instance,
            hwnd,
            "EDIT",
            "",
            WS_CHILD | WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL,
            ID_FIND_TEXT,
        );
        state.replace_text = create_control(
            instance,
            hwnd,
            "EDIT",
            "",
            WS_CHILD | WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL,
            ID_REPLACE_TEXT,
        );
        state.find_next_button = create_control(
            instance,
            hwnd,
            "BUTTON",
            "Find next",
            WS_CHILD | WS_TABSTOP | BS_PUSHBUTTON,
            ID_FIND_NEXT,
        );
        state.replace_button = create_control(
            instance,
            hwnd,
            "BUTTON",
            "Replace",
            WS_CHILD | WS_TABSTOP | BS_PUSHBUTTON,
            ID_REPLACE,
        );
        state.replace_all_button = create_control(
            instance,
            hwnd,
            "BUTTON",
            "Replace all",
            WS_CHILD | WS_TABSTOP | BS_PUSHBUTTON,
            ID_REPLACE_ALL,
        );
        state.continue_button = create_control(
            instance,
            hwnd,
            "BUTTON",
            "Continue",
            WS_CHILD | WS_TABSTOP | BS_PUSHBUTTON,
            ID_DEBUG_CONTINUE,
        );
        state.pause_button = create_control(
            instance,
            hwnd,
            "BUTTON",
            "Pause",
            WS_CHILD | WS_TABSTOP | BS_PUSHBUTTON,
            ID_DEBUG_PAUSE,
        );
        state.stop_button = create_control(
            instance,
            hwnd,
            "BUTTON",
            "Stop",
            WS_CHILD | WS_TABSTOP | BS_PUSHBUTTON,
            ID_DEBUG_STOP,
        );
        state.step_into_button = create_control(
            instance,
            hwnd,
            "BUTTON",
            "Into",
            WS_CHILD | WS_TABSTOP | BS_PUSHBUTTON,
            ID_STEP_INTO,
        );
        state.step_over_button = create_control(
            instance,
            hwnd,
            "BUTTON",
            "Over",
            WS_CHILD | WS_TABSTOP | BS_PUSHBUTTON,
            ID_STEP_OVER,
        );
        state.step_out_button = create_control(
            instance,
            hwnd,
            "BUTTON",
            "Out",
            WS_CHILD | WS_TABSTOP | BS_PUSHBUTTON,
            ID_STEP_OUT,
        );
        state.debug_panel = create_control(
            instance,
            hwnd,
            "RICHEDIT50W",
            "",
            WS_CHILD | WS_BORDER | WS_VSCROLL | ES_MULTILINE | ES_AUTOVSCROLL | ES_READONLY,
            ID_DEBUG_PANEL,
        );
        state.input_label = create_control(
            instance,
            hwnd,
            "STATIC",
            "Input",
            WS_CHILD | WS_VISIBLE | SS_LEFT,
            0,
        );
        state.input = create_control(
            instance,
            hwnd,
            "EDIT",
            "",
            WS_CHILD
                | WS_VISIBLE
                | WS_BORDER
                | WS_TABSTOP
                | WS_VSCROLL
                | ES_MULTILINE
                | ES_AUTOVSCROLL
                | ES_WANTRETURN,
            ID_INPUT,
        );
        state.output_label = create_control(
            instance,
            hwnd,
            "STATIC",
            "Output",
            WS_CHILD | WS_VISIBLE | SS_LEFT,
            0,
        );
        state.output = create_control(
            instance,
            hwnd,
            "EDIT",
            "Ready.",
            WS_CHILD
                | WS_VISIBLE
                | WS_BORDER
                | WS_TABSTOP
                | WS_VSCROLL
                | ES_MULTILINE
                | ES_AUTOVSCROLL
                | ES_READONLY,
            ID_OUTPUT,
        );

        let event_mask = SendMessageW(state.editor, EM_GETEVENTMASK, 0, 0);
        SendMessageW(state.editor, EM_SETEVENTMASK, 0, event_mask | ENM_SELCHANGE);

        let face = wide("Consolas");
        state.font = CreateFontW(-17, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0x31, face.as_ptr());

        for control in [
            state.editor,
            state.gutter,
            state.input,
            state.input_label,
            state.output,
            state.path_label,
            state.output_label,
            state.open_button,
            state.save_button,
            state.run_button,
            state.build_button,
            state.debug_button,
            state.find_text,
            state.replace_text,
            state.find_next_button,
            state.replace_button,
            state.replace_all_button,
            state.continue_button,
            state.pause_button,
            state.stop_button,
            state.step_into_button,
            state.step_over_button,
            state.step_out_button,
            state.debug_panel,
        ] {
            SendMessageW(control, WM_SETFONT, state.font as Wparam, 1);
        }

        if let Some(path) = state.startup_path.take() {
            state.suppress_change = false;
            load_source_path(hwnd, state, path);
        } else {
            update_line_numbers(state);
            highlight_source(state);
            state.suppress_change = false;
        }
        update_title(hwnd, state);
        layout(hwnd, state);
        SetFocus(state.editor);
    }

    unsafe fn create_control(
        instance: Hinstance,
        parent: Hwnd,
        class: &str,
        text: &str,
        style: Dword,
        id: usize,
    ) -> Hwnd {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            style,
            0,
            0,
            0,
            0,
            parent,
            id as Hmenu,
            instance,
            null_mut(),
        )
    }

    unsafe fn layout(hwnd: Hwnd, state: &AppState) {
        let mut rect: Rect = zeroed();
        GetClientRect(hwnd, &mut rect);
        let width = (rect.right - rect.left).max(1);
        let height = (rect.bottom - rect.top).max(1);
        let margin = 10;
        let toolbar_height = 40;
        let button_width = 82;
        let button_height = 30;
        let gutter_width = 52;
        let find_height = if state.find_visible { 34 } else { 0 };
        let debug_toolbar_height = if state.debug_active { 34 } else { 0 };
        let debug_panel_width = if state.debug_active {
            (width / 3).clamp(240, 340)
        } else {
            0
        };
        let output_height = (height / 4).clamp(110, 210);
        let input_height = (height / 7).clamp(65, 120);
        let section_label_height = 25;
        let editor_top = margin + toolbar_height + find_height + debug_toolbar_height;
        let output_top = height - margin - output_height;
        let input_top = output_top - section_label_height - 6 - input_height;
        let editor_height = (input_top - section_label_height - editor_top - 6).max(80);

        MoveWindow(
            state.open_button,
            margin,
            margin,
            button_width,
            button_height,
            1,
        );
        MoveWindow(
            state.save_button,
            margin + button_width + 8,
            margin,
            button_width,
            button_height,
            1,
        );
        MoveWindow(
            state.run_button,
            margin + (button_width + 8) * 2,
            margin,
            button_width + 12,
            button_height,
            1,
        );
        MoveWindow(
            state.path_label,
            margin + (button_width + 8) * 5 + 12,
            margin + 6,
            (width - 540).max(100),
            24,
            1,
        );
        MoveWindow(
            state.build_button,
            margin + (button_width + 8) * 3,
            margin,
            button_width + 4,
            button_height,
            1,
        );
        MoveWindow(
            state.debug_button,
            margin + (button_width + 8) * 4,
            margin,
            button_width + 4,
            button_height,
            1,
        );
        let find_top = margin + toolbar_height;
        MoveWindow(state.find_text, margin, find_top, 210, 26, 1);
        MoveWindow(state.replace_text, margin + 218, find_top, 210, 26, 1);
        MoveWindow(state.find_next_button, margin + 436, find_top, 92, 26, 1);
        MoveWindow(state.replace_button, margin + 536, find_top, 82, 26, 1);
        MoveWindow(state.replace_all_button, margin + 626, find_top, 104, 26, 1);
        let debug_top = margin + toolbar_height + find_height;
        for (index, control) in [
            state.continue_button,
            state.pause_button,
            state.stop_button,
            state.step_into_button,
            state.step_over_button,
            state.step_out_button,
        ]
        .into_iter()
        .enumerate()
        {
            MoveWindow(control, margin + index as i32 * 88, debug_top, 80, 26, 1);
        }
        MoveWindow(
            state.gutter,
            margin,
            editor_top,
            gutter_width,
            editor_height,
            1,
        );
        MoveWindow(
            state.editor,
            margin + gutter_width,
            editor_top,
            width - margin * 2 - gutter_width - debug_panel_width,
            editor_height,
            1,
        );
        MoveWindow(
            state.debug_panel,
            width - margin - debug_panel_width,
            editor_top,
            debug_panel_width,
            editor_height,
            1,
        );
        MoveWindow(
            state.input_label,
            margin,
            input_top - section_label_height,
            width - margin * 2,
            section_label_height,
            1,
        );
        MoveWindow(
            state.input,
            margin,
            input_top,
            width - margin * 2,
            input_height,
            1,
        );
        MoveWindow(
            state.output_label,
            margin,
            output_top - section_label_height,
            width - margin * 2,
            section_label_height,
            1,
        );
        MoveWindow(
            state.output,
            margin,
            output_top,
            width - margin * 2,
            output_height,
            1,
        );
    }

    unsafe fn handle_command(hwnd: Hwnd, state: &mut AppState, id: usize, notification: usize) {
        if id == ID_EDITOR && notification == EN_CHANGE && !state.suppress_change {
            state.dirty = true;
            auto_close_delimiter(state);
            update_line_numbers(state);
            schedule_highlight(hwnd);
            update_title(hwnd, state);
            return;
        }

        if id == ID_EDITOR && notification == EN_VSCROLL {
            sync_gutter_scroll(state);
            return;
        }

        if notification != BN_CLICKED && notification != 0 {
            return;
        }

        match id {
            ID_OPEN => open_file(hwnd, state),
            ID_SAVE => {
                save_file(hwnd, state, false);
            }
            ID_SAVE_AS => {
                save_file(hwnd, state, true);
            }
            ID_RUN => run_source(hwnd, state),
            ID_BUILD => build_source(hwnd, state),
            ID_DEBUG => start_debugger(hwnd, state),
            ID_DEBUG_CONTINUE => send_debug_command(state, DebugCommand::Continue),
            ID_DEBUG_PAUSE => send_debug_command(state, DebugCommand::Pause),
            ID_DEBUG_STOP => send_debug_command(state, DebugCommand::Stop),
            ID_STEP_INTO => send_debug_command(state, DebugCommand::StepInto),
            ID_STEP_OVER => send_debug_command(state, DebugCommand::StepOver),
            ID_STEP_OUT => send_debug_command(state, DebugCommand::StepOut),
            ID_BREAKPOINT => toggle_breakpoint(state),
            ID_UNDO => {
                SendMessageW(state.editor, 0x0304, 0, 0);
            }
            ID_REDO => {
                SendMessageW(state.editor, EM_REDO, 0, 0);
            }
            ID_FIND => toggle_find(hwnd, state),
            ID_FIND_NEXT => {
                find_next(state);
            }
            ID_REPLACE => replace_current(state),
            ID_REPLACE_ALL => replace_all(state),
            ID_EXIT if confirm_discard(hwnd, state) => {
                DestroyWindow(hwnd);
            }
            ID_EXIT => {}
            _ => {}
        }
    }

    unsafe fn schedule_highlight(hwnd: Hwnd) {
        KillTimer(hwnd, HIGHLIGHT_TIMER);
        SetTimer(hwnd, HIGHLIGHT_TIMER, HIGHLIGHT_DELAY_MS, null_mut());
    }

    unsafe fn update_line_numbers(state: &mut AppState) {
        let source = normalize_newlines(&get_text(state.editor));
        let count = source.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let numbers = (1..=count)
            .map(|line| {
                if state.breakpoints.contains(&line) {
                    format!("* {line}")
                } else {
                    format!("  {line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        set_text(state.gutter, &numbers);
        state.last_source = source;
        state.gutter_first_line = 0;
        sync_gutter_scroll(state);
    }

    unsafe fn sync_gutter_scroll(state: &mut AppState) {
        let first = SendMessageW(state.editor, EM_GETFIRSTVISIBLELINE, 0, 0) as i32;
        let delta = first - state.gutter_first_line;
        if delta != 0 {
            SendMessageW(state.gutter, EM_LINESCROLL, 0, delta as Lparam);
            state.gutter_first_line = first;
        }
    }

    unsafe fn auto_close_delimiter(state: &mut AppState) {
        let current = normalize_newlines(&get_text(state.editor));
        let old = state.last_source.encode_utf16().collect::<Vec<_>>();
        let new = current.encode_utf16().collect::<Vec<_>>();
        if new.len() != old.len() + 1 {
            state.last_source = current;
            return;
        }

        let prefix = old
            .iter()
            .zip(&new)
            .take_while(|(left, right)| left == right)
            .count();
        if old[prefix..] != new[prefix + 1..] {
            state.last_source = current;
            return;
        }
        if matches!(new[prefix] as u8, b')' | b']' | b'}' | b'"')
            && old.get(prefix) == Some(&new[prefix])
        {
            state.suppress_change = true;
            let range = CharRange {
                min: prefix as i32,
                max: (prefix + 1) as i32,
            };
            SendMessageW(
                state.editor,
                EM_EXSETSEL,
                0,
                (&range as *const CharRange) as Lparam,
            );
            SendMessageW(state.editor, EM_REPLACESEL, 1, wide("").as_ptr() as Lparam);
            SendMessageW(
                state.editor,
                EM_SETSEL,
                (prefix + 1) as Wparam,
                (prefix + 1) as Lparam,
            );
            state.suppress_change = false;
            state.last_source = normalize_newlines(&get_text(state.editor));
            return;
        }
        let closer = match char::from_u32(new[prefix] as u32) {
            Some('(') => ')',
            Some('[') => ']',
            Some('{') => '}',
            Some('"') => '"',
            _ => {
                state.last_source = current;
                return;
            }
        };

        let mut selection = CharRange { min: 0, max: 0 };
        SendMessageW(
            state.editor,
            EM_EXGETSEL,
            0,
            (&mut selection as *mut CharRange) as Lparam,
        );
        if selection.min != (prefix + 1) as i32 || selection.max != selection.min {
            state.last_source = current;
            return;
        }

        state.suppress_change = true;
        let replacement = wide(&closer.to_string());
        SendMessageW(
            state.editor,
            EM_REPLACESEL,
            1,
            replacement.as_ptr() as Lparam,
        );
        SendMessageW(
            state.editor,
            EM_SETSEL,
            (prefix + 1) as Wparam,
            (prefix + 1) as Lparam,
        );
        state.suppress_change = false;
        state.last_source = normalize_newlines(&get_text(state.editor));
    }

    unsafe fn highlight_source(state: &mut AppState) {
        state.formatting = true;
        let source = get_text(state.editor);
        let normalized = normalize_newlines(&source);
        let mut selection = CharRange { min: 0, max: 0 };
        SendMessageW(
            state.editor,
            EM_EXGETSEL,
            0,
            (&mut selection as *mut CharRange) as Lparam,
        );
        let event_mask = SendMessageW(state.editor, EM_GETEVENTMASK, 0, 0);
        SendMessageW(
            state.editor,
            EM_SETEVENTMASK,
            0,
            event_mask & !ENM_SELCHANGE,
        );
        apply_format(
            state.editor,
            CharRange {
                min: 0,
                max: normalized.encode_utf16().count() as i32,
            },
            rgb(32, 34, 37),
            false,
        );
        apply_background_range(
            state.editor,
            CharRange {
                min: 0,
                max: normalized.encode_utf16().count() as i32,
            },
            rgb(255, 255, 255),
        );
        state.matched_brackets = None;

        for span in apl_parser::syntax_spans(&normalized) {
            let color = match span.kind {
                apl_parser::SyntaxKind::Keyword => rgb(0, 76, 153),
                apl_parser::SyntaxKind::Identifier => continue,
                apl_parser::SyntaxKind::Number => rgb(0, 110, 110),
                apl_parser::SyntaxKind::String => rgb(163, 55, 45),
                apl_parser::SyntaxKind::Comment => rgb(49, 116, 60),
                apl_parser::SyntaxKind::Symbol => rgb(82, 82, 82),
            };
            apply_format(
                state.editor,
                CharRange {
                    min: normalized[..span.start].encode_utf16().count() as i32,
                    max: normalized[..span.end].encode_utf16().count() as i32,
                },
                color,
                span.kind == apl_parser::SyntaxKind::Keyword,
            );
        }
        SendMessageW(
            state.editor,
            EM_EXSETSEL,
            0,
            (&selection as *const CharRange) as Lparam,
        );
        SendMessageW(state.editor, EM_SETEVENTMASK, 0, event_mask);
        state.formatting = false;
    }

    unsafe fn apply_format(editor: Hwnd, range: CharRange, color: Dword, bold: bool) {
        SendMessageW(
            editor,
            EM_EXSETSEL,
            0,
            (&range as *const CharRange) as Lparam,
        );
        let mut format: CharFormat2W = zeroed();
        format.size = size_of::<CharFormat2W>() as Dword;
        format.mask = CFM_COLOR | CFM_BOLD;
        format.text_color = color;
        format.effects = if bold { CFE_BOLD } else { 0 };
        SendMessageW(
            editor,
            EM_SETCHARFORMAT,
            SCF_SELECTION,
            (&format as *const CharFormat2W) as Lparam,
        );
    }

    unsafe fn highlight_matching_brackets(state: &mut AppState) {
        let mut selection = CharRange { min: 0, max: 0 };
        SendMessageW(
            state.editor,
            EM_EXGETSEL,
            0,
            (&mut selection as *mut CharRange) as Lparam,
        );
        if selection.min != selection.max {
            return;
        }

        state.formatting = true;
        let event_mask = SendMessageW(state.editor, EM_GETEVENTMASK, 0, 0);
        SendMessageW(
            state.editor,
            EM_SETEVENTMASK,
            0,
            event_mask & !ENM_SELCHANGE,
        );
        if let Some((left, right)) = state.matched_brackets.take() {
            apply_background(state.editor, left, rgb(255, 255, 255));
            apply_background(state.editor, right, rgb(255, 255, 255));
        }

        let source = get_text(state.editor);
        let encoded = source.encode_utf16().collect::<Vec<_>>();
        let ignored = apl_parser::syntax_spans(&source)
            .into_iter()
            .filter(|span| {
                matches!(
                    span.kind,
                    apl_parser::SyntaxKind::String | apl_parser::SyntaxKind::Comment
                )
            })
            .map(|span| {
                (
                    source[..span.start].encode_utf16().count(),
                    source[..span.end].encode_utf16().count(),
                )
            })
            .collect::<Vec<_>>();
        let caret = selection.min.max(0) as usize;
        let candidate = caret
            .checked_sub(1)
            .filter(|index| is_bracket(encoded.get(*index).copied()))
            .or_else(|| {
                (caret < encoded.len() && is_bracket(Some(encoded[caret]))).then_some(caret)
            });

        if let Some(index) = candidate {
            if !in_ranges(index, &ignored) {
                if let Some(other) = matching_bracket(&encoded, index, &ignored) {
                    apply_background(state.editor, index as i32, rgb(255, 239, 153));
                    apply_background(state.editor, other as i32, rgb(255, 239, 153));
                    state.matched_brackets = Some((index as i32, other as i32));
                }
            }
        }
        SendMessageW(
            state.editor,
            EM_EXSETSEL,
            0,
            (&selection as *const CharRange) as Lparam,
        );
        SendMessageW(state.editor, EM_SETEVENTMASK, 0, event_mask);
        state.formatting = false;
    }

    unsafe fn apply_background(editor: Hwnd, position: i32, color: Dword) {
        apply_background_range(
            editor,
            CharRange {
                min: position,
                max: position + 1,
            },
            color,
        );
    }

    unsafe fn apply_background_range(editor: Hwnd, range: CharRange, color: Dword) {
        SendMessageW(
            editor,
            EM_EXSETSEL,
            0,
            (&range as *const CharRange) as Lparam,
        );
        let mut format: CharFormat2W = zeroed();
        format.size = size_of::<CharFormat2W>() as Dword;
        format.mask = CFM_BACKCOLOR;
        format.back_color = color;
        SendMessageW(
            editor,
            EM_SETCHARFORMAT,
            SCF_SELECTION,
            (&format as *const CharFormat2W) as Lparam,
        );
    }

    fn is_bracket(value: Option<u16>) -> bool {
        value.is_some_and(|value| matches!(value as u8, b'(' | b')' | b'[' | b']' | b'{' | b'}'))
    }

    fn in_ranges(index: usize, ranges: &[(usize, usize)]) -> bool {
        ranges
            .iter()
            .any(|(start, end)| *start <= index && index < *end)
    }

    fn matching_bracket(source: &[u16], index: usize, ignored: &[(usize, usize)]) -> Option<usize> {
        let current = source[index] as u8;
        let (open, close, direction) = match current {
            b'(' => (b'(', b')', 1),
            b'[' => (b'[', b']', 1),
            b'{' => (b'{', b'}', 1),
            b')' => (b'(', b')', -1),
            b']' => (b'[', b']', -1),
            b'}' => (b'{', b'}', -1),
            _ => return None,
        };
        let mut depth = 0i32;
        let mut cursor = index as i32;
        loop {
            cursor += direction;
            if cursor < 0 || cursor as usize >= source.len() {
                return None;
            }
            let cursor = cursor as usize;
            if in_ranges(cursor, ignored) {
                continue;
            }
            let value = source[cursor] as u8;
            if (direction > 0 && value == open) || (direction < 0 && value == close) {
                depth += 1;
            } else if (direction > 0 && value == close) || (direction < 0 && value == open) {
                if depth == 0 {
                    return Some(cursor);
                }
                depth -= 1;
            }
        }
    }

    unsafe fn toggle_find(hwnd: Hwnd, state: &mut AppState) {
        state.find_visible = !state.find_visible;
        let command = if state.find_visible { SW_SHOW } else { SW_HIDE };
        for control in [
            state.find_text,
            state.replace_text,
            state.find_next_button,
            state.replace_button,
            state.replace_all_button,
        ] {
            ShowWindow(control, command);
        }
        layout(hwnd, state);
        SetFocus(if state.find_visible {
            state.find_text
        } else {
            state.editor
        });
    }

    unsafe fn find_next(state: &AppState) -> bool {
        let needle = get_text(state.find_text).encode_utf16().collect::<Vec<_>>();
        if needle.is_empty() {
            return false;
        }
        let haystack = get_text(state.editor).encode_utf16().collect::<Vec<_>>();
        let mut selection = CharRange { min: 0, max: 0 };
        SendMessageW(
            state.editor,
            EM_EXGETSEL,
            0,
            (&mut selection as *mut CharRange) as Lparam,
        );
        let start = selection.max.max(0) as usize;
        let found =
            find_utf16(&haystack, &needle, start).or_else(|| find_utf16(&haystack, &needle, 0));
        if let Some(index) = found {
            let range = CharRange {
                min: index as i32,
                max: (index + needle.len()) as i32,
            };
            SendMessageW(
                state.editor,
                EM_EXSETSEL,
                0,
                (&range as *const CharRange) as Lparam,
            );
            SetFocus(state.editor);
            true
        } else {
            false
        }
    }

    unsafe fn replace_current(state: &mut AppState) {
        let query = get_text(state.find_text).encode_utf16().collect::<Vec<_>>();
        let source = get_text(state.editor).encode_utf16().collect::<Vec<_>>();
        let mut selection = CharRange { min: 0, max: 0 };
        SendMessageW(
            state.editor,
            EM_EXGETSEL,
            0,
            (&mut selection as *mut CharRange) as Lparam,
        );
        let range = selection.min.max(0) as usize..selection.max.max(0) as usize;
        if !query.is_empty() && source.get(range.clone()) == Some(query.as_slice()) {
            let replacement = wide(&get_text(state.replace_text));
            SendMessageW(
                state.editor,
                EM_REPLACESEL,
                1,
                replacement.as_ptr() as Lparam,
            );
        }
        find_next(state);
    }

    unsafe fn replace_all(state: &mut AppState) {
        let query = get_text(state.find_text);
        if query.is_empty() {
            return;
        }
        let replacement = get_text(state.replace_text);
        let source = get_text(state.editor);
        let replaced = source.replace(&query, &replacement);
        if replaced != source {
            set_text(state.editor, &replaced);
        }
    }

    fn find_utf16(haystack: &[u16], needle: &[u16], start: usize) -> Option<usize> {
        if needle.len() > haystack.len() || start > haystack.len() {
            return None;
        }
        haystack[start..]
            .windows(needle.len())
            .position(|window| window == needle)
            .map(|offset| start + offset)
    }

    fn normalize_newlines(text: &str) -> String {
        text.replace("\r\n", "\n").replace('\r', "\n")
    }

    const fn rgb(red: u8, green: u8, blue: u8) -> Dword {
        red as Dword | ((green as Dword) << 8) | ((blue as Dword) << 16)
    }

    unsafe fn start_debugger(hwnd: Hwnd, state: &mut AppState) {
        if state.debug_active {
            return;
        }
        if state.current_path.is_some() && state.dirty && !save_file(hwnd, state, false) {
            return;
        }
        let source = get_text(state.editor);
        let input = parse_input(&get_text(state.input));
        let breakpoints = state.breakpoints.clone();
        let (sender, receiver) = mpsc::channel();
        state.debug_sender = Some(sender);
        state.debug_active = true;
        show_debug_ui(state, true);
        set_debug_controls(state, false);
        EnableWindow(state.editor, 0);
        EnableWindow(state.debug_button, 0);
        set_text(state.debug_panel, "Starting debugger...");
        set_text(state.output_label, "Debugger running");
        layout(hwnd, state);
        let window = hwnd as isize;

        thread::spawn(move || {
            let result = execute_debug_source(&source, input, &breakpoints, receiver, window);
            let result_ptr = Box::into_raw(Box::new(result));
            let posted = unsafe {
                PostMessageW(
                    window as Hwnd,
                    WM_APP_DEBUG_COMPLETE,
                    0,
                    result_ptr as Lparam,
                )
            };
            if posted == 0 {
                unsafe { drop(Box::from_raw(result_ptr)) };
            }
        });
    }

    fn execute_debug_source(
        source: &str,
        input: Vec<String>,
        breakpoints: &HashSet<usize>,
        receiver: Receiver<DebugCommand>,
        window: isize,
    ) -> StudioResult {
        let parsed = match apl_parser::parse_program_with_locations(source) {
            Ok(parsed) => parsed,
            Err(error) => {
                return StudioResult {
                    text: format!(
                        "Debug compile error:\r\nparse error at {}:{}: {}",
                        error.line, error.column, error.message
                    ),
                    diagnostic: Some((error.line, error.column)),
                }
            }
        };
        if let Err(error) = apl_core::validate_executable_program(&parsed.program) {
            return StudioResult {
                text: format!("Debug compile error:\r\ncheck error: {error:?}"),
                diagnostic: None,
            };
        }
        let locations = parsed
            .locations
            .into_iter()
            .map(|location| apl_runtime::DebugSourceLocation {
                path: location.path,
                line: location.line,
                column: location.column,
            })
            .collect::<Vec<_>>();

        let mut mode = DebugRunMode::StepInto;
        let mut first = true;
        let mut pause_requested = false;
        let mut stopped = false;
        let result =
            apl_runtime::run_program_debug(&parsed.program, &locations, input, |snapshot| {
                loop {
                    match receiver.try_recv() {
                        Ok(DebugCommand::Pause) => pause_requested = true,
                        Ok(DebugCommand::Stop) => {
                            stopped = true;
                            return apl_runtime::DebugAction::Stop;
                        }
                        Ok(command) => {
                            set_debug_mode(&mut mode, command, snapshot.call_stack.len())
                        }
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => {
                            stopped = true;
                            return apl_runtime::DebugAction::Stop;
                        }
                    }
                }

                let depth = snapshot.call_stack.len();
                let should_pause = first
                    || pause_requested
                    || breakpoints.contains(&snapshot.location.line)
                    || match mode {
                        DebugRunMode::Continue => false,
                        DebugRunMode::StepInto => true,
                        DebugRunMode::StepOver(base) => depth <= base,
                        DebugRunMode::StepOut(base) => depth < base,
                    };
                first = false;
                if !should_pause {
                    return apl_runtime::DebugAction::Continue;
                }
                pause_requested = false;
                let event_ptr = Box::into_raw(Box::new(DebugEvent {
                    snapshot: snapshot.clone(),
                }));
                let posted = unsafe {
                    PostMessageW(window as Hwnd, WM_APP_DEBUG_PAUSED, 0, event_ptr as Lparam)
                };
                if posted == 0 {
                    unsafe { drop(Box::from_raw(event_ptr)) };
                    stopped = true;
                    return apl_runtime::DebugAction::Stop;
                }

                loop {
                    match receiver.recv() {
                        Ok(DebugCommand::Stop) | Err(_) => {
                            stopped = true;
                            return apl_runtime::DebugAction::Stop;
                        }
                        Ok(DebugCommand::Pause) => {}
                        Ok(command) => {
                            set_debug_mode(&mut mode, command, depth);
                            return apl_runtime::DebugAction::Continue;
                        }
                    }
                }
            });

        match result {
            Ok(output) if stopped => StudioResult {
                text: if output.stdout.is_empty() {
                    "Debug session stopped.".to_owned()
                } else {
                    format!("{}\r\n\r\nDebug session stopped.", output.stdout)
                },
                diagnostic: None,
            },
            Ok(output) => StudioResult {
                text: if output.stdout.is_empty() {
                    "Debug session finished with no output.".to_owned()
                } else {
                    output.stdout
                },
                diagnostic: None,
            },
            Err(error) => StudioResult {
                text: format!("Debug runtime error:\r\n{error:?}"),
                diagnostic: None,
            },
        }
    }

    fn set_debug_mode(mode: &mut DebugRunMode, command: DebugCommand, depth: usize) {
        *mode = match command {
            DebugCommand::Continue | DebugCommand::Pause => DebugRunMode::Continue,
            DebugCommand::Stop => DebugRunMode::Continue,
            DebugCommand::StepInto => DebugRunMode::StepInto,
            DebugCommand::StepOver => DebugRunMode::StepOver(depth),
            DebugCommand::StepOut => DebugRunMode::StepOut(depth),
        };
    }

    unsafe fn send_debug_command(state: &mut AppState, command: DebugCommand) {
        if let Some(sender) = &state.debug_sender {
            if sender.send(command).is_ok() {
                let paused = matches!(command, DebugCommand::Pause);
                set_debug_controls(state, paused);
                if !paused {
                    set_text(state.output_label, "Debugger running");
                }
            }
        }
    }

    unsafe fn set_debug_controls(state: &AppState, paused: bool) {
        for control in [
            state.continue_button,
            state.step_into_button,
            state.step_over_button,
            state.step_out_button,
        ] {
            EnableWindow(control, i32::from(paused));
        }
        EnableWindow(state.pause_button, i32::from(!paused && state.debug_active));
        EnableWindow(state.stop_button, i32::from(state.debug_active));
    }

    unsafe fn show_debug_ui(state: &AppState, visible: bool) {
        let command = if visible { SW_SHOW } else { SW_HIDE };
        for control in [
            state.continue_button,
            state.pause_button,
            state.stop_button,
            state.step_into_button,
            state.step_over_button,
            state.step_out_button,
            state.debug_panel,
        ] {
            ShowWindow(control, command);
        }
        EnableWindow(state.editor, i32::from(!visible));
        EnableWindow(state.debug_button, i32::from(!visible));
    }

    unsafe fn show_debug_snapshot(state: &AppState, snapshot: &apl_runtime::DebugSnapshot) {
        let stack = if snapshot.call_stack.is_empty() {
            "<entry>".to_owned()
        } else {
            snapshot.call_stack.join("\r\n")
        };
        let variables = if snapshot.variables.is_empty() {
            "<none>".to_owned()
        } else {
            snapshot
                .variables
                .iter()
                .map(|variable| format!("{} = {}", variable.name, variable.value))
                .collect::<Vec<_>>()
                .join("\r\n")
        };
        set_text(
            state.debug_panel,
            &format!(
                "Line {}:{}\r\n\r\nCall stack\r\n{}\r\n\r\nVariables\r\n{}",
                snapshot.location.line, snapshot.location.column, stack, variables
            ),
        );
        set_text(state.output, &snapshot.stdout);
    }

    unsafe fn toggle_breakpoint(state: &mut AppState) {
        let line = current_editor_line(state.editor);
        if !state.breakpoints.remove(&line) {
            state.breakpoints.insert(line);
        }
        update_line_numbers(state);
    }

    unsafe fn current_editor_line(editor: Hwnd) -> usize {
        let mut selection = CharRange { min: 0, max: 0 };
        SendMessageW(
            editor,
            EM_EXGETSEL,
            0,
            (&mut selection as *mut CharRange) as Lparam,
        );
        let encoded = get_text(editor).encode_utf16().collect::<Vec<_>>();
        let end = (selection.min.max(0) as usize).min(encoded.len());
        String::from_utf16_lossy(&encoded[..end])
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1
    }

    unsafe fn open_file(hwnd: Hwnd, state: &mut AppState) {
        if !confirm_discard(hwnd, state) {
            return;
        }
        let Some(path) = choose_file(hwnd, false) else {
            return;
        };

        load_source_path(hwnd, state, path);
    }

    unsafe fn load_source_path(hwnd: Hwnd, state: &mut AppState, path: PathBuf) -> bool {
        match fs::read_to_string(&path) {
            Ok(source) => {
                state.suppress_change = true;
                set_text(state.editor, &source);
                state.last_source = source.replace("\r\n", "\n").replace('\r', "\n");
                update_line_numbers(state);
                highlight_source(state);
                state.suppress_change = false;
                state.current_path = Some(path);
                state.dirty = false;
                set_text(state.output, "Ready.");
                update_title(hwnd, state);
                true
            }
            Err(error) => {
                show_error(hwnd, &format!("Could not open the file:\r\n{error}"));
                false
            }
        }
    }

    unsafe fn save_file(hwnd: Hwnd, state: &mut AppState, save_as: bool) -> bool {
        let path = if save_as || state.current_path.is_none() {
            let Some(path) = choose_file(hwnd, true) else {
                return false;
            };
            path
        } else {
            state.current_path.clone().unwrap()
        };

        let source = get_text(state.editor);
        match fs::write(&path, source.as_bytes()) {
            Ok(()) => {
                state.current_path = Some(path);
                state.dirty = false;
                update_title(hwnd, state);
                true
            }
            Err(error) => {
                show_error(hwnd, &format!("Could not save the file:\r\n{error}"));
                false
            }
        }
    }

    unsafe fn confirm_discard(hwnd: Hwnd, state: &mut AppState) -> bool {
        if !state.dirty {
            return true;
        }

        let answer = MessageBoxW(
            hwnd,
            wide("Save changes before continuing?").as_ptr(),
            wide(APP_TITLE).as_ptr(),
            MB_YESNOCANCEL | MB_ICONWARNING,
        );
        match answer {
            IDYES => save_file(hwnd, state, false),
            IDNO => true,
            IDCANCEL => false,
            _ => false,
        }
    }

    unsafe fn run_source(hwnd: Hwnd, state: &mut AppState) {
        if state.running {
            return;
        }

        if state.current_path.is_some() && state.dirty && !save_file(hwnd, state, false) {
            return;
        }

        let source = get_text(state.editor);
        let input = parse_input(&get_text(state.input));
        let source_path = state.current_path.clone();
        state.running = true;
        EnableWindow(state.run_button, 0);
        set_text(state.output_label, "Running...");
        set_text(state.output, "Checking and running APL source...");
        let window = hwnd as isize;

        thread::spawn(move || {
            let parse_diagnostic = source_diagnostic(&source);
            let text = execute_source_at_path(&source, input, source_path.as_deref());
            let result = StudioResult {
                diagnostic: parse_diagnostic.or_else(|| diagnostic_from_message(&text)),
                text,
            };
            let result_ptr = Box::into_raw(Box::new(result));
            let posted = unsafe {
                PostMessageW(window as Hwnd, WM_APP_RUN_COMPLETE, 0, result_ptr as Lparam)
            };
            if posted == 0 {
                unsafe { drop(Box::from_raw(result_ptr)) };
            }
        });
    }

    unsafe fn build_source(hwnd: Hwnd, state: &mut AppState) {
        if state.building {
            return;
        }
        if (state.current_path.is_none() || state.dirty) && !save_file(hwnd, state, false) {
            return;
        }
        let Some(destination) = choose_executable(hwnd, state.current_path.as_deref()) else {
            return;
        };
        let source_path = state.current_path.clone().unwrap();
        state.building = true;
        EnableWindow(state.build_button, 0);
        set_text(state.output_label, "Building native EXE...");
        set_text(
            state.output,
            "Compiling APL to native x86-64 code. This can take a moment...",
        );
        let window = hwnd as isize;

        thread::spawn(move || {
            let result = build_native_binary(&source_path, &destination);
            let result_ptr = Box::into_raw(Box::new(result));
            let posted = unsafe {
                PostMessageW(
                    window as Hwnd,
                    WM_APP_BUILD_COMPLETE,
                    0,
                    result_ptr as Lparam,
                )
            };
            if posted == 0 {
                unsafe { drop(Box::from_raw(result_ptr)) };
            }
        });
    }

    fn build_native_binary(source_path: &Path, destination: &Path) -> StudioResult {
        let source = match fs::read_to_string(source_path) {
            Ok(source) => source,
            Err(error) => {
                return StudioResult {
                    text: format!("Build error:\r\nCould not read source: {error}"),
                    diagnostic: None,
                }
            }
        };
        if let Some(diagnostic) = source_diagnostic(&source) {
            let error = apl_parser::parse_program(&source).unwrap_err();
            return StudioResult {
                text: format!(
                    "Build error:\r\nparse error at {}:{}: {}",
                    error.line, error.column, error.message
                ),
                diagnostic: Some(diagnostic),
            };
        }

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis());
        let package_dir =
            std::env::temp_dir().join(format!("apl-studio-native-{}-{stamp}", std::process::id()));
        let result = (|| -> Result<String, String> {
            let target =
                apl_compiler::host_native_target().map_err(|error| format!("{error:?}"))?;
            let output =
                apl_compiler::build_native_executable_package(source_path, &package_dir, target)
                    .map_err(|error| format!("{error:?}"))?;
            let cargo = Command::new("cargo")
                .arg("build")
                .arg("--release")
                .arg("--offline")
                .arg("--manifest-path")
                .arg(&output.manifest_path)
                .current_dir(&output.package_dir)
                .creation_flags(0x0800_0000)
                .output()
                .map_err(|error| format!("Could not start Cargo: {error}"))?;
            if !cargo.status.success() {
                let details = String::from_utf8_lossy(&cargo.stderr);
                return Err(format!("Native linker failed:\n{details}"));
            }
            let built = output
                .package_dir
                .join("target")
                .join("release")
                .join(format!("{}.exe", output.package_name));
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::copy(&built, destination).map_err(|error| {
                format!(
                    "Could not copy `{}` to `{}`: {error}",
                    built.display(),
                    destination.display()
                )
            })?;
            Ok(format!(
                "Native executable built successfully.\r\n{}\r\n\r\nMade with APL",
                destination.display()
            ))
        })();
        let _ = fs::remove_dir_all(&package_dir);

        match result {
            Ok(text) => StudioResult {
                text,
                diagnostic: None,
            },
            Err(message) => StudioResult {
                text: format!("Build error:\r\n{message}"),
                diagnostic: None,
            },
        }
    }

    fn source_diagnostic(source: &str) -> Option<(usize, usize)> {
        apl_parser::parse_program(source)
            .err()
            .map(|error| (error.line, error.column))
    }

    fn diagnostic_from_message(message: &str) -> Option<(usize, usize)> {
        for marker in [" at ", "error at "] {
            let Some((_, location)) = message.rsplit_once(marker) else {
                continue;
            };
            let mut parts = location.split(':');
            let line = parts.next().unwrap_or_default();
            let column = parts.next().unwrap_or_default();
            if let (Ok(line), Ok(column)) = (line.parse(), column.parse()) {
                return Some((line, column));
            }
        }
        None
    }

    unsafe fn navigate_to(editor: Hwnd, source: &str, line: usize, column: usize) {
        let normalized = normalize_newlines(source);
        let byte = normalized
            .split_inclusive('\n')
            .take(line.saturating_sub(1))
            .map(str::len)
            .sum::<usize>();
        let line_end = normalized[byte..]
            .find('\n')
            .map_or(normalized.len(), |offset| byte + offset);
        let within = normalized[byte..line_end]
            .char_indices()
            .nth(column.saturating_sub(1))
            .map_or(line_end - byte, |(offset, _)| offset);
        let position = normalized[..byte + within].encode_utf16().count();
        SendMessageW(editor, EM_SETSEL, position, position as Lparam);
        SetFocus(editor);
    }

    #[cfg(test)]
    fn execute_source(source: &str, input: Vec<String>) -> String {
        execute_source_at_path(source, input, None)
    }

    fn execute_source_at_path(
        source: &str,
        input: Vec<String>,
        source_path: Option<&Path>,
    ) -> String {
        let program = match apl_parser::parse_program(source) {
            Ok(program) => program,
            Err(error) => return portable_compile_error(error.into()),
        };
        if let Err(error) = apl_core::validate_executable_structure(&program) {
            return portable_compile_error(error.into());
        }

        let external_libraries = match apl_compiler::requested_external_libraries(source) {
            Ok(libraries) => libraries,
            Err(error) => return portable_compile_error(error),
        };

        let libraries = if external_libraries.is_empty() {
            Vec::new()
        } else {
            let Some(source_path) = source_path else {
                return "Library error:\r\nSave the APL source before using external libraries."
                    .to_owned();
            };
            match apl_compiler::resolve_named_libraries(source_path, source) {
                Ok(libraries) => libraries,
                Err(apl_compiler::CompileError::LibraryNotFound { name, searched }) => {
                    let searched = searched
                        .iter()
                        .map(|path| format!("`{}`", path.display()))
                        .collect::<Vec<_>>()
                        .join("\r\n");
                    return format!(
                        "Library error:\r\n`{name}` was not found. Searched:\r\n{searched}"
                    );
                }
                Err(error) => return portable_compile_error(error),
            }
        };

        let artifact = if libraries.is_empty() {
            apl_compiler::compile_linked_artifact(source)
        } else {
            apl_compiler::compile_linked_artifact_with_libraries(source, &libraries)
        };
        let artifact = match artifact {
            Ok(artifact) => artifact,
            Err(error) => return portable_compile_error(error),
        };

        match apl_compiler::run_linked_artifact(&artifact, input) {
            Ok(output) if output.stdout.is_empty() => "Program finished with no output.".to_owned(),
            Ok(output) => output.stdout,
            Err(apl_compiler::CompileError::Runtime(apl_runtime::RuntimeError::Failed(
                message,
            ))) => {
                if message == "linked artifact failed" && libraries.is_empty() {
                    if let Ok(prelude) = apl_parser::parse_program(apl_compiler::STANDARD_PRELUDE) {
                        if let Err(error) =
                            apl_core::validate_executable_program_with_prelude(&prelude, &program)
                        {
                            return portable_compile_error(error.into());
                        }
                    }
                }
                format!("APL fail:\r\n{message}")
            }
            Err(apl_compiler::CompileError::Runtime(
                apl_runtime::RuntimeError::FailedWithCode { message, code },
            )) => format!("APL error ({code}):\r\n{message}"),
            Err(error) => format!("Runtime error:\r\n{error:?}"),
        }
    }

    fn portable_compile_error(error: apl_compiler::CompileError) -> String {
        match error {
            apl_compiler::CompileError::Parse(error) => format!(
                "Compile error:\r\nparse error at {}:{}: {}",
                error.line, error.column, error.message
            ),
            apl_compiler::CompileError::Check(error) => {
                format!("Compile error:\r\ncheck error: {error:?}")
            }
            apl_compiler::CompileError::Runtime(apl_runtime::RuntimeError::Failed(message))
            | apl_compiler::CompileError::Bootstrap(message) => {
                format!("Compile error:\r\n{message}")
            }
            apl_compiler::CompileError::Runtime(apl_runtime::RuntimeError::FailedWithCode {
                message,
                code,
            }) => format!("Compile error ({code}):\r\n{message}"),
            error => format!("Compile error:\r\n{error:?}"),
        }
    }

    fn display_output(output: &str) -> String {
        output.replace('\0', "\\0")
    }

    fn parse_input(text: &str) -> Vec<String> {
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        if normalized.is_empty() {
            Vec::new()
        } else {
            normalized.split('\n').map(str::to_owned).collect()
        }
    }

    unsafe fn choose_file(hwnd: Hwnd, save: bool) -> Option<PathBuf> {
        let mut buffer = vec![0u16; 32_768];
        let filter = wide("APL source (*.apl)\0*.apl\0All files (*.*)\0*.*\0");
        let extension = wide("apl");
        let mut dialog: OpenFileNameW = zeroed();
        dialog.struct_size = size_of::<OpenFileNameW>() as Dword;
        dialog.owner = hwnd;
        dialog.filter = filter.as_ptr();
        dialog.filter_index = 1;
        dialog.file = buffer.as_mut_ptr();
        dialog.max_file = buffer.len() as Dword;
        dialog.default_extension = extension.as_ptr();
        dialog.flags = OFN_EXPLORER | OFN_PATHMUSTEXIST;
        if save {
            dialog.flags |= OFN_OVERWRITEPROMPT;
        } else {
            dialog.flags |= OFN_FILEMUSTEXIST;
        }

        let accepted = if save {
            GetSaveFileNameW(&mut dialog)
        } else {
            GetOpenFileNameW(&mut dialog)
        };
        if accepted == 0 {
            return None;
        }

        let length = buffer.iter().position(|value| *value == 0).unwrap_or(0);
        Some(PathBuf::from(String::from_utf16_lossy(&buffer[..length])))
    }

    unsafe fn choose_executable(hwnd: Hwnd, source_path: Option<&Path>) -> Option<PathBuf> {
        let suggested = source_path
            .and_then(Path::file_stem)
            .and_then(|name| name.to_str())
            .map(|name| format!("{name}.exe"))
            .unwrap_or_else(|| "program.exe".to_owned());
        let mut buffer = vec![0u16; 32_768];
        let encoded = wide(&suggested);
        buffer[..encoded.len()].copy_from_slice(&encoded);
        let filter = wide("Windows executable (*.exe)\0*.exe\0All files (*.*)\0*.*\0");
        let extension = wide("exe");
        let mut dialog: OpenFileNameW = zeroed();
        dialog.struct_size = size_of::<OpenFileNameW>() as Dword;
        dialog.owner = hwnd;
        dialog.filter = filter.as_ptr();
        dialog.filter_index = 1;
        dialog.file = buffer.as_mut_ptr();
        dialog.max_file = buffer.len() as Dword;
        dialog.default_extension = extension.as_ptr();
        dialog.flags = OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_OVERWRITEPROMPT;
        if GetSaveFileNameW(&mut dialog) == 0 {
            return None;
        }
        let length = buffer.iter().position(|value| *value == 0).unwrap_or(0);
        Some(PathBuf::from(String::from_utf16_lossy(&buffer[..length])))
    }

    unsafe fn update_title(hwnd: Hwnd, state: &AppState) {
        let name = state
            .current_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled.apl");
        let marker = if state.dirty { "*" } else { "" };
        set_text(hwnd, &format!("{marker}{name} - {APP_TITLE}"));
        set_text(state.path_label, &format!("{marker}{name}"));
    }

    unsafe fn get_text(hwnd: Hwnd) -> String {
        let length = GetWindowTextLengthW(hwnd).max(0) as usize;
        let mut buffer = vec![0u16; length + 1];
        let copied =
            GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as c_int).max(0) as usize;
        String::from_utf16_lossy(&buffer[..copied])
    }

    unsafe fn set_text(hwnd: Hwnd, text: &str) {
        let normalized = text.replace("\r\n", "\n").replace('\n', "\r\n");
        SetWindowTextW(hwnd, wide(&normalized).as_ptr());
    }

    unsafe fn show_error(hwnd: Hwnd, message: &str) {
        MessageBoxW(
            hwnd,
            wide(message).as_ptr(),
            wide(APP_TITLE).as_ptr(),
            MB_ICONERROR,
        );
    }

    fn register_file_association() -> Result<(), String> {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let command = association_command(&executable);
        let icon = format!("\"{}\",0", executable.display());

        set_registry_string(r"Software\Classes\APL.Source", None, "APL Source File")?;
        set_registry_string(r"Software\Classes\APL.Source\DefaultIcon", None, &icon)?;
        set_registry_string(
            r"Software\Classes\APL.Source\shell\open\command",
            None,
            &command,
        )?;
        set_registry_empty(r"Software\Classes\.apl\OpenWithProgids", "APL.Source")?;
        set_registry_string(
            r"Software\Classes\Applications\apl_studio.exe\shell\open\command",
            None,
            &command,
        )?;
        set_registry_empty(
            r"Software\Classes\Applications\apl_studio.exe\SupportedTypes",
            ".apl",
        )?;
        set_registry_string(
            r"Software\APL Studio\Capabilities",
            Some("ApplicationName"),
            "APL Studio",
        )?;
        set_registry_string(
            r"Software\APL Studio\Capabilities",
            Some("ApplicationDescription"),
            "Editor, runner, builder, and debugger for APL source files",
        )?;
        set_registry_string(
            r"Software\APL Studio\Capabilities\FileAssociations",
            Some(".apl"),
            "APL.Source",
        )?;
        set_registry_string(
            r"Software\RegisteredApplications",
            Some("APL Studio"),
            r"Software\APL Studio\Capabilities",
        )?;
        unsafe {
            SHChangeNotify(SHCNE_ASSOCCHANGED, 0, null(), null());
        }
        Ok(())
    }

    fn association_command(executable: &Path) -> String {
        format!("\"{}\" \"%1\"", executable.display())
    }

    fn set_registry_string(key_path: &str, name: Option<&str>, value: &str) -> Result<(), String> {
        let data = wide(value);
        set_registry_value(
            key_path,
            name,
            REG_SZ,
            data.as_ptr().cast(),
            (data.len() * size_of::<u16>()) as Dword,
        )
    }

    fn set_registry_empty(key_path: &str, name: &str) -> Result<(), String> {
        set_registry_value(key_path, Some(name), REG_NONE, null(), 0)
    }

    fn set_registry_value(
        key_path: &str,
        name: Option<&str>,
        value_type: Dword,
        data: *const u8,
        data_size: Dword,
    ) -> Result<(), String> {
        let key_path_wide = wide(key_path);
        let name_wide = name.map(wide);
        let mut key = null_mut();
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                key_path_wide.as_ptr(),
                0,
                null_mut(),
                0,
                KEY_SET_VALUE,
                null_mut(),
                &mut key,
                null_mut(),
            )
        };
        if status != 0 {
            return Err(format!(
                "Could not create registry key `{key_path}` ({status})"
            ));
        }
        let status = unsafe {
            RegSetValueExW(
                key,
                name_wide.as_ref().map_or(null(), |name| name.as_ptr()),
                0,
                value_type,
                data,
                data_size,
            )
        };
        unsafe { RegCloseKey(key) };
        if status == 0 {
            Ok(())
        } else {
            Err(format!(
                "Could not write registry value in `{key_path}` ({status})"
            ))
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value).encode_wide().chain(Some(0)).collect()
    }

    #[cfg(test)]
    mod tests {
        use super::{
            build_native_binary, diagnostic_from_message, display_output, execute_source,
            execute_source_at_path, parse_input,
        };
        use std::{fs, process::Command, time::SystemTime};

        fn temp_root(label: &str) -> std::path::PathBuf {
            let unique = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            std::env::temp_dir().join(format!(
                "apl_studio_{label}_{}_{}",
                std::process::id(),
                unique
            ))
        }

        #[test]
        fn executes_source_from_editor_buffer() {
            assert_eq!(
                execute_source(
                    "AVInt std::main() {\n  AVInt value = 6 * 7\n  out value\n  return 0\n}",
                    Vec::new()
                ),
                "42"
            );
            assert_eq!(
                execute_source(
                    "AVInt total = 0\nfunc add_one() {\n  total += 1\n  return total\n}\nAVInt std::main() {\n  add_one()\n  out total\n  return 0\n}",
                    Vec::new()
                ),
                "1"
            );
        }

        #[test]
        fn executes_new_control_flow_and_displays_explicit_errors() {
            assert_eq!(
                execute_source(
                    r#"AVInt std::main() {
  AVInt count = 0
  match 4 {
    case 1, 2 {
      out "small"
    }
    case 3..5 {
      out "range"
    }
    case default {
      out "default"
    }
  }
  while (false) (2) {
    out "bad"
  } limit {
    out "bad"
  } else {
    out ":else"
  }
  while (true) (1) {
    count++
  } limit {
    out ":limit"
  }
  return 0
}"#,
                    Vec::new()
                ),
                "range:else:limit"
            );
            assert_eq!(
                execute_source(
                    r#"func abort() {
  return(ERR=["boom"][7])
}
AVInt std::main() {
  abort()
  return 0
}"#,
                    Vec::new()
                ),
                "APL error (7):\r\nboom"
            );
        }

        #[test]
        fn applies_composition_only_to_editor_source() {
            assert_eq!(
                execute_source(
                    "exclude \"len\" << std\nAVInt std::main() {\n  out 42\n  return 0\n}",
                    Vec::new()
                ),
                "42"
            );
            assert_eq!(
                execute_source(
                    "exclude \"len\" << std\nAVInt std::main() {\n  out std::len([1])\n  return 0\n}",
                    Vec::new()
                ),
                "Compile error:\r\nsemantic error at 3:3: excluded symbol `len` from library `std`"
            );
            assert_eq!(
                execute_source(
                    "include >> math\nAVInt std::main() {\n  out math::answer()\n  return 0\n}",
                    Vec::new()
                ),
                "Library error:\r\nSave the APL source before using external libraries."
            );
        }

        #[test]
        fn resolves_named_library_for_saved_source() {
            let root = temp_root("library");
            let libraries = root.join("libraries");
            fs::create_dir_all(&libraries).unwrap();
            let module =
                apl_compiler::compile_module_artifact("func math.answer() {\n  return 42\n}")
                    .unwrap();
            fs::write(libraries.join("math.aplmod"), module).unwrap();

            let source =
                "include >> math\nAVInt std::main() {\n  out math::answer()\n  return 0\n}";
            let output = execute_source_at_path(source, Vec::new(), Some(&root.join("main.apl")));
            assert_eq!(output, "42");

            fs::remove_dir_all(root).unwrap();
        }

        #[test]
        fn reports_invalid_source_in_output_panel() {
            assert_eq!(
                execute_source("AVInt = 4", Vec::new()),
                "Compile error:\r\nparse error at 1:7: expected identifier"
            );
        }

        #[test]
        fn rejects_legacy_editor_buffer_without_main() {
            assert_eq!(
                execute_source("AVInt value = 6 * 7\nout value", Vec::new()),
                "Compile error:\r\ncheck error: MissingMain"
            );
        }

        #[test]
        fn explains_top_level_return_instead_of_reporting_generic_link_failure() {
            assert_eq!(
                execute_source("return 1\nAVInt std::main() {\n  return 0\n}", Vec::new()),
                "Compile error:\r\ncheck error: TopLevelExecutableStatement"
            );
        }

        #[test]
        fn blocks_compound_vtime_secret_laundering() {
            let source = r#"
List m = [4:SASV]
AVInt std::main() {
  pick (m): curr {
    curr += 1
    VTime x = curr
    x = std::str(curr)
    AVStr z = x
  }
  AVInt steal = std::int(z)
  steal -= 1
  out steal
  return 0
}
"#;

            assert_eq!(
                execute_source(source, Vec::new()),
                "Compile error:\r\nsemantic error at 8:5: protection downgrade `z`: source SASV, target AV"
            );
        }

        #[test]
        fn passes_editor_input_lines_to_runtime() {
            let input = parse_input("41\r\nhello\r\n");
            assert_eq!(input, vec!["41", "hello", ""]);
            assert_eq!(
                execute_source(
                    "AVInt std::main() {\n  AVInt number = input\n  AVStr word = input\n  out number\n  out word\n  return 0\n}",
                    input,
                ),
                "41hello"
            );
        }

        #[test]
        fn keeps_output_after_nul_visible() {
            let output = execute_source(
                "AVInt std::main() {\n  out \"left\\0right\"\n  return 0\n}",
                Vec::new(),
            );

            assert_eq!(output, "left\0right");
            assert_eq!(display_output(&output), "left\\0right");
        }

        #[test]
        fn locates_semantic_diagnostics() {
            assert_eq!(
                diagnostic_from_message("Compile error:\r\nsemantic error at 8:5: denied"),
                Some((8, 5))
            );
            assert_eq!(diagnostic_from_message("Runtime error"), None);
        }

        #[test]
        fn builds_and_brands_native_executable() {
            let root = temp_root("native");
            let libraries = root.join("libraries");
            fs::create_dir_all(&libraries).unwrap();
            let source = root.join("answer.apl");
            let executable = root.join("answer.exe");
            let module =
                apl_compiler::compile_module_artifact("func math.answer() {\n  return 6 * 7\n}")
                    .unwrap();
            fs::write(libraries.join("math.aplmod"), module).unwrap();
            fs::write(
                &source,
                "include >> math\nAVInt std::main() {\n  out math::answer()\n  return 0\n}",
            )
            .unwrap();

            let result = build_native_binary(&source, &executable);
            assert!(result
                .text
                .starts_with("Native executable built successfully"));
            assert!(result.diagnostic.is_none());
            fs::remove_dir_all(&libraries).unwrap();
            let output = Command::new(&executable).output().unwrap();
            assert!(output.status.success());
            assert_eq!(String::from_utf8_lossy(&output.stdout), "42");
            assert!(fs::read(&executable)
                .unwrap()
                .windows(b"APLBUILD1:Made with APL".len())
                .any(|window| window == b"APLBUILD1:Made with APL"));

            fs::remove_dir_all(root).unwrap();
        }
    }
}

#[cfg(windows)]
fn main() {
    windows_app::run();
}
