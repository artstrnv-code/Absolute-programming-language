use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowEvent {
    pub window: i64,
    pub message: i64,
    pub wparam: i64,
    pub lparam: i64,
    pub source: i64,
    pub code: i64,
    pub control_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub exit_code: i64,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceleratorEntry {
    pub key: i64,
    pub command: i64,
    pub modifiers: i64,
}

#[derive(Debug)]
struct Module {
    handle: usize,
    procedures: Vec<i64>,
}

#[derive(Debug)]
struct Window {
    handle: usize,
    menu: Option<i64>,
    icon: Option<i64>,
    accelerators: Option<i64>,
}

#[derive(Debug)]
struct Menu {
    handle: usize,
    parent: Option<i64>,
    children: Vec<i64>,
    attached_window: Option<i64>,
}

#[derive(Debug)]
struct Icon {
    handle: usize,
    windows: BTreeSet<i64>,
}

#[derive(Debug)]
struct Accelerators {
    handle: usize,
    windows: BTreeSet<i64>,
}

#[derive(Debug)]
pub struct WindowsHost {
    next_id: i64,
    modules: BTreeMap<i64, Module>,
    windows: BTreeMap<i64, Window>,
    menus: BTreeMap<i64, Menu>,
    icons: BTreeMap<i64, Icon>,
    accelerators: BTreeMap<i64, Accelerators>,
    events: VecDeque<WindowEvent>,
    support_modules: Vec<usize>,
}

impl Default for WindowsHost {
    fn default() -> Self {
        Self {
            next_id: 1,
            modules: BTreeMap::new(),
            windows: BTreeMap::new(),
            menus: BTreeMap::new(),
            icons: BTreeMap::new(),
            accelerators: BTreeMap::new(),
            events: VecDeque::new(),
            support_modules: Vec::new(),
        }
    }
}

impl WindowsHost {
    pub fn available(&self) -> bool {
        cfg!(windows)
    }

    pub fn load_library(&mut self, name: &str) -> Result<i64, String> {
        let id = self.allocate_id()?;
        let handle = platform::load_library(name)?;
        self.modules.insert(
            id,
            Module {
                handle,
                procedures: Vec::new(),
            },
        );
        Ok(id)
    }

    pub fn procedure_address(&self, library: i64, name: &str) -> Result<i64, String> {
        let module = self
            .modules
            .get(&library)
            .ok_or_else(|| format!("unknown Windows library identity `{library}`"))?;
        let address = platform::procedure_address(module.handle, name)?;
        i64::try_from(address)
            .map_err(|_| "Windows procedure address does not fit APL Int".to_owned())
    }

    pub fn attach_procedure(&mut self, library: i64, pointer: i64) -> Result<(), String> {
        let module = self
            .modules
            .get_mut(&library)
            .ok_or_else(|| format!("unknown Windows library identity `{library}`"))?;
        module.procedures.push(pointer);
        Ok(())
    }

    pub fn close_library(&mut self, library: i64) -> Result<Vec<i64>, String> {
        let handle = self
            .modules
            .get(&library)
            .map(|module| module.handle)
            .ok_or_else(|| format!("unknown Windows library identity `{library}`"))?;
        platform::free_library(handle)?;
        let module = self.modules.remove(&library).expect("module was checked");
        Ok(module.procedures)
    }

    pub fn create_window(&mut self, title: &str, width: i64, height: i64) -> Result<i64, String> {
        let width = i32::try_from(width)
            .map_err(|_| "window width must fit a signed 32-bit value".to_owned())?;
        let height = i32::try_from(height)
            .map_err(|_| "window height must fit a signed 32-bit value".to_owned())?;
        if width <= 0 || height <= 0 {
            return Err("window width and height must be positive".to_owned());
        }
        let id = self.allocate_id()?;
        let handle = platform::create_window(title, width, height)?;
        self.windows.insert(
            id,
            Window {
                handle,
                menu: None,
                icon: None,
                accelerators: None,
            },
        );
        self.capture_events();
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_control(
        &mut self,
        parent: i64,
        class_name: &str,
        text: &str,
        control_id: i64,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        style: i64,
        extended_style: i64,
    ) -> Result<i64, String> {
        let parent = self.window_handle(parent)?;
        let control_id =
            usize::try_from(control_id).map_err(|_| "control id must be nonnegative".to_owned())?;
        let x = i32::try_from(x).map_err(|_| "control x must fit signed 32-bit".to_owned())?;
        let y = i32::try_from(y).map_err(|_| "control y must fit signed 32-bit".to_owned())?;
        let width =
            i32::try_from(width).map_err(|_| "control width must fit signed 32-bit".to_owned())?;
        let height = i32::try_from(height)
            .map_err(|_| "control height must fit signed 32-bit".to_owned())?;
        if width <= 0 || height <= 0 {
            return Err("control width and height must be positive".to_owned());
        }
        let style = u32::try_from(style)
            .map_err(|_| "control style must fit unsigned 32-bit".to_owned())?;
        let extended_style = u32::try_from(extended_style)
            .map_err(|_| "control extended style must fit unsigned 32-bit".to_owned())?;

        if class_name.eq_ignore_ascii_case("RICHEDIT50W") && self.support_modules.is_empty() {
            self.support_modules
                .push(platform::load_library("Msftedit.dll")?);
        }

        let id = self.allocate_id()?;
        let handle = platform::create_control(
            parent,
            class_name,
            text,
            control_id,
            x,
            y,
            width,
            height,
            style,
            extended_style,
        )?;
        self.windows.insert(
            id,
            Window {
                handle,
                menu: None,
                icon: None,
                accelerators: None,
            },
        );
        self.capture_events();
        Ok(id)
    }

    fn allocate_id(&mut self) -> Result<i64, String> {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| "Windows identity counter overflow".to_owned())?;
        Ok(id)
    }

    pub fn show_window(&mut self, window: i64) -> Result<bool, String> {
        let result = platform::show_window(self.window_handle(window)?)?;
        self.capture_events();
        Ok(result)
    }

    pub fn destroy_window(&mut self, window: i64) -> Result<bool, String> {
        let (handle, menu, icon, accelerators) = self
            .windows
            .get(&window)
            .map(|record| (record.handle, record.menu, record.icon, record.accelerators))
            .ok_or_else(|| format!("unknown Windows window identity `{window}`"))?;
        platform::destroy_window(handle)?;
        self.capture_events();
        self.windows.remove(&window);
        if let Some(menu) = menu {
            for identity in self.menu_subtree(menu) {
                self.menus.remove(&identity);
            }
        }
        if let Some(icon) = icon.and_then(|icon| self.icons.get_mut(&icon)) {
            icon.windows.remove(&window);
        }
        if let Some(accelerators) =
            accelerators.and_then(|accelerators| self.accelerators.get_mut(&accelerators))
        {
            accelerators.windows.remove(&window);
        }
        Ok(true)
    }

    pub fn create_accelerators(&mut self, entries: &[AcceleratorEntry]) -> Result<i64, String> {
        if entries.is_empty() {
            return Err("accelerator table must contain at least one entry".to_owned());
        }
        let mut checked = Vec::with_capacity(entries.len());
        for entry in entries {
            let key = u16::try_from(entry.key)
                .ok()
                .filter(|key| *key != 0)
                .ok_or_else(|| "accelerator key must be between 1 and 65535".to_owned())?;
            let command = u16::try_from(entry.command)
                .ok()
                .filter(|command| *command != 0)
                .ok_or_else(|| "accelerator command must be between 1 and 65535".to_owned())?;
            let modifiers = u8::try_from(entry.modifiers)
                .map_err(|_| "accelerator modifiers must fit unsigned 8-bit".to_owned())?;
            if modifiers & !0x1e != 0 {
                return Err(
                    "accelerator modifiers may contain only ALT, CONTROL, SHIFT, and NOINVERT"
                        .to_owned(),
                );
            }
            checked.push((modifiers, key, command));
        }
        let id = self.allocate_id()?;
        let handle = platform::create_accelerators(&checked)?;
        self.accelerators.insert(
            id,
            Accelerators {
                handle,
                windows: BTreeSet::new(),
            },
        );
        Ok(id)
    }

    pub fn set_accelerators(&mut self, window: i64, accelerators: i64) -> Result<bool, String> {
        let (window_handle, previous) = self
            .windows
            .get(&window)
            .map(|window| (window.handle, window.accelerators))
            .ok_or_else(|| format!("unknown Windows window identity `{window}`"))?;
        let accelerator_handle = if accelerators == 0 {
            0
        } else {
            self.accelerators
                .get(&accelerators)
                .map(|accelerators| accelerators.handle)
                .ok_or_else(|| format!("unknown Windows accelerator identity `{accelerators}`"))?
        };
        platform::set_accelerators(window_handle, accelerator_handle)?;
        if let Some(previous) = previous.filter(|previous| *previous != accelerators) {
            if let Some(record) = self.accelerators.get_mut(&previous) {
                record.windows.remove(&window);
            }
        }
        self.windows
            .get_mut(&window)
            .expect("window was checked")
            .accelerators = (accelerators != 0).then_some(accelerators);
        if accelerators != 0 {
            self.accelerators
                .get_mut(&accelerators)
                .expect("accelerator table was checked")
                .windows
                .insert(window);
        }
        Ok(true)
    }

    pub fn destroy_accelerators(&mut self, accelerators: i64) -> Result<bool, String> {
        let (handle, windows) = self
            .accelerators
            .get(&accelerators)
            .map(|accelerators| {
                (
                    accelerators.handle,
                    accelerators.windows.iter().copied().collect::<Vec<_>>(),
                )
            })
            .ok_or_else(|| format!("unknown Windows accelerator identity `{accelerators}`"))?;
        for window in windows {
            if let Some(record) = self.windows.get_mut(&window) {
                platform::set_accelerators(record.handle, 0)?;
                record.accelerators = None;
            }
        }
        platform::destroy_accelerators(handle)?;
        self.accelerators.remove(&accelerators);
        Ok(true)
    }

    pub fn load_icon(&mut self, path: &str, width: i64, height: i64) -> Result<i64, String> {
        if path.is_empty() {
            return Err("icon path must not be empty".to_owned());
        }
        let width =
            i32::try_from(width).map_err(|_| "icon width must fit signed 32-bit".to_owned())?;
        let height =
            i32::try_from(height).map_err(|_| "icon height must fit signed 32-bit".to_owned())?;
        if width < 0 || height < 0 {
            return Err("icon width and height must be nonnegative".to_owned());
        }
        let id = self.allocate_id()?;
        let handle = platform::load_icon(path, width, height)?;
        self.icons.insert(
            id,
            Icon {
                handle,
                windows: BTreeSet::new(),
            },
        );
        Ok(id)
    }

    pub fn set_window_icon(&mut self, window: i64, icon: i64) -> Result<bool, String> {
        let (window_handle, previous) = self
            .windows
            .get(&window)
            .map(|window| (window.handle, window.icon))
            .ok_or_else(|| format!("unknown Windows window identity `{window}`"))?;
        let icon_handle = if icon == 0 {
            0
        } else {
            self.icons
                .get(&icon)
                .map(|icon| icon.handle)
                .ok_or_else(|| format!("unknown Windows icon identity `{icon}`"))?
        };
        platform::set_window_icon(window_handle, icon_handle)?;
        if let Some(previous) = previous.filter(|previous| *previous != icon) {
            if let Some(record) = self.icons.get_mut(&previous) {
                record.windows.remove(&window);
            }
        }
        self.windows
            .get_mut(&window)
            .expect("window was checked")
            .icon = (icon != 0).then_some(icon);
        if icon != 0 {
            self.icons
                .get_mut(&icon)
                .expect("icon was checked")
                .windows
                .insert(window);
        }
        Ok(true)
    }

    pub fn destroy_icon(&mut self, icon: i64) -> Result<bool, String> {
        let (handle, windows) = self
            .icons
            .get(&icon)
            .map(|icon| {
                (
                    icon.handle,
                    icon.windows.iter().copied().collect::<Vec<_>>(),
                )
            })
            .ok_or_else(|| format!("unknown Windows icon identity `{icon}`"))?;
        for window in windows {
            if let Some(record) = self.windows.get_mut(&window) {
                platform::set_window_icon(record.handle, 0)?;
                record.icon = None;
            }
        }
        platform::destroy_icon(handle)?;
        self.icons.remove(&icon);
        Ok(true)
    }

    pub fn create_menu(&mut self, popup: bool) -> Result<i64, String> {
        let id = self.allocate_id()?;
        let handle = platform::create_menu(popup)?;
        self.menus.insert(
            id,
            Menu {
                handle,
                parent: None,
                children: Vec::new(),
                attached_window: None,
            },
        );
        Ok(id)
    }

    pub fn append_menu_item(&self, menu: i64, command_id: i64, text: &str) -> Result<bool, String> {
        platform::append_menu_item(self.menu_handle(menu)?, menu_command_id(command_id)?, text)
    }

    pub fn append_submenu(&mut self, menu: i64, submenu: i64, text: &str) -> Result<bool, String> {
        if menu == submenu {
            return Err("a menu cannot contain itself".to_owned());
        }
        let parent_handle = self.menu_handle(menu)?;
        let child = self
            .menus
            .get(&submenu)
            .ok_or_else(|| format!("unknown Windows menu identity `{submenu}`"))?;
        if child.parent.is_some() {
            return Err(format!(
                "Windows menu identity `{submenu}` is already a submenu"
            ));
        }
        if child.attached_window.is_some() {
            return Err(format!(
                "Windows menu identity `{submenu}` is attached to a window"
            ));
        }
        if self.menu_subtree(submenu).contains(&menu) {
            return Err("submenu insertion would create a menu cycle".to_owned());
        }
        let child_handle = child.handle;
        platform::append_submenu(parent_handle, child_handle, text)?;
        self.menus
            .get_mut(&menu)
            .expect("parent menu was checked")
            .children
            .push(submenu);
        self.menus
            .get_mut(&submenu)
            .expect("submenu was checked")
            .parent = Some(menu);
        Ok(true)
    }

    pub fn append_menu_separator(&self, menu: i64) -> Result<bool, String> {
        platform::append_menu_separator(self.menu_handle(menu)?)
    }

    pub fn set_menu(&mut self, window: i64, menu: i64) -> Result<bool, String> {
        let (window_handle, previous) = self
            .windows
            .get(&window)
            .map(|record| (record.handle, record.menu))
            .ok_or_else(|| format!("unknown Windows window identity `{window}`"))?;
        let menu_handle = if menu == 0 {
            0
        } else {
            let record = self
                .menus
                .get(&menu)
                .ok_or_else(|| format!("unknown Windows menu identity `{menu}`"))?;
            if record.parent.is_some() {
                return Err("a submenu cannot be attached directly to a window".to_owned());
            }
            if record.attached_window.is_some_and(|owner| owner != window) {
                return Err(format!(
                    "Windows menu identity `{menu}` is attached to another window"
                ));
            }
            record.handle
        };

        platform::set_menu(window_handle, menu_handle)?;
        platform::draw_menu_bar(window_handle)?;
        if let Some(previous) = previous.filter(|previous| *previous != menu) {
            if let Some(record) = self.menus.get_mut(&previous) {
                record.attached_window = None;
            }
        }
        self.windows
            .get_mut(&window)
            .expect("window was checked")
            .menu = (menu != 0).then_some(menu);
        if menu != 0 {
            self.menus
                .get_mut(&menu)
                .expect("menu was checked")
                .attached_window = Some(window);
        }
        Ok(true)
    }

    pub fn set_menu_item_enabled(
        &self,
        menu: i64,
        command_id: i64,
        enabled: bool,
    ) -> Result<bool, String> {
        platform::set_menu_item_enabled(
            self.menu_handle(menu)?,
            menu_command_id(command_id)?,
            enabled,
        )?;
        self.redraw_menu(menu)?;
        Ok(true)
    }

    pub fn set_menu_item_checked(
        &self,
        menu: i64,
        command_id: i64,
        checked: bool,
    ) -> Result<bool, String> {
        platform::set_menu_item_checked(
            self.menu_handle(menu)?,
            menu_command_id(command_id)?,
            checked,
        )?;
        self.redraw_menu(menu)?;
        Ok(true)
    }

    pub fn destroy_menu(&mut self, menu: i64) -> Result<bool, String> {
        let record = self
            .menus
            .get(&menu)
            .ok_or_else(|| format!("unknown Windows menu identity `{menu}`"))?;
        if record.parent.is_some() {
            return Err("an attached submenu must be destroyed through its root menu".to_owned());
        }
        let handle = record.handle;
        let attached_window = record.attached_window;
        let subtree = self.menu_subtree(menu);
        if let Some(window) = attached_window {
            if let Some(record) = self.windows.get_mut(&window) {
                platform::set_menu(record.handle, 0)?;
                platform::draw_menu_bar(record.handle)?;
                record.menu = None;
            }
        }
        platform::destroy_menu(handle)?;
        for identity in subtree {
            self.menus.remove(&identity);
        }
        Ok(true)
    }

    pub fn run_message_loop(&mut self) -> Result<i64, String> {
        let result = platform::run_message_loop()?;
        self.capture_events();
        self.remove_closed_windows();
        Ok(result)
    }

    pub fn pump_messages(&mut self) -> Result<bool, String> {
        let running = platform::pump_messages()?;
        self.capture_events();
        self.remove_closed_windows();
        Ok(running)
    }

    pub fn next_event(&mut self) -> Option<WindowEvent> {
        self.capture_events();
        self.events.pop_front()
    }

    pub fn wait_event(&mut self) -> Result<Option<WindowEvent>, String> {
        self.capture_events();
        if let Some(event) = self.events.pop_front() {
            return Ok(Some(event));
        }
        loop {
            let running = platform::wait_event_step()?;
            self.capture_events();
            self.remove_closed_windows();
            if let Some(event) = self.events.pop_front() {
                return Ok(Some(event));
            }
            if !running {
                return Ok(None);
            }
        }
    }

    pub fn set_deferred_close(&self, window: i64, enabled: bool) -> Result<bool, String> {
        platform::set_deferred_close(self.window_handle(window)?, enabled)
    }

    pub fn set_window_text(&self, window: i64, text: &str) -> Result<bool, String> {
        platform::set_window_text(self.window_handle(window)?, text)
    }

    pub fn window_text(&self, window: i64) -> Result<String, String> {
        platform::window_text(self.window_handle(window)?)
    }

    pub fn move_window(
        &self,
        window: i64,
        x: i64,
        y: i64,
        width: i64,
        height: i64,
        repaint: bool,
    ) -> Result<bool, String> {
        platform::move_window(
            self.window_handle(window)?,
            i32::try_from(x).map_err(|_| "window x must fit signed 32-bit".to_owned())?,
            i32::try_from(y).map_err(|_| "window y must fit signed 32-bit".to_owned())?,
            i32::try_from(width).map_err(|_| "window width must fit signed 32-bit".to_owned())?,
            i32::try_from(height).map_err(|_| "window height must fit signed 32-bit".to_owned())?,
            repaint,
        )
    }

    pub fn selection(&self, control: i64) -> Result<(i64, i64), String> {
        platform::selection(self.window_handle(control)?)
    }

    pub fn set_selection(&self, control: i64, start: i64, end: i64) -> Result<bool, String> {
        if start < -1 || end < -1 {
            return Err("selection positions must be -1 or nonnegative".to_owned());
        }
        if start >= 0 && end >= 0 && start > end {
            return Err("selection start must not exceed its end".to_owned());
        }
        let start = i32::try_from(start)
            .map_err(|_| "selection start must fit signed 32-bit".to_owned())?;
        let end =
            i32::try_from(end).map_err(|_| "selection end must fit signed 32-bit".to_owned())?;
        platform::set_selection(self.window_handle(control)?, start, end)
    }

    pub fn replace_selection(
        &self,
        control: i64,
        text: &str,
        can_undo: bool,
    ) -> Result<bool, String> {
        platform::replace_selection(self.window_handle(control)?, text, can_undo)
    }

    pub fn line_count(&self, control: i64) -> Result<i64, String> {
        platform::line_count(self.window_handle(control)?)
    }

    pub fn set_text_color(
        &self,
        control: i64,
        start: i64,
        end: i64,
        color: i64,
    ) -> Result<bool, String> {
        if start < 0 || end < -1 {
            return Err(
                "format range positions must be nonnegative, except end may be -1".to_owned(),
            );
        }
        if end >= 0 && start > end {
            return Err("format range start must not exceed its end".to_owned());
        }
        let start = i32::try_from(start)
            .map_err(|_| "format range start must fit signed 32-bit".to_owned())?;
        let end =
            i32::try_from(end).map_err(|_| "format range end must fit signed 32-bit".to_owned())?;
        let color = u32::try_from(color)
            .ok()
            .filter(|color| *color <= 0x00ff_ffff)
            .ok_or_else(|| "text color must be between 0 and 0xFFFFFF".to_owned())?;
        platform::set_text_color(self.window_handle(control)?, start, end, color)
    }

    pub fn set_background_color(
        &self,
        control: i64,
        start: i64,
        end: i64,
        color: i64,
    ) -> Result<bool, String> {
        let (start, end) = format_range(start, end)?;
        let color = format_color(color, "background color")?;
        platform::set_background_color(self.window_handle(control)?, start, end, color)
    }

    pub fn set_text_bold(
        &self,
        control: i64,
        start: i64,
        end: i64,
        enabled: bool,
    ) -> Result<bool, String> {
        let (start, end) = format_range(start, end)?;
        platform::set_text_bold(self.window_handle(control)?, start, end, enabled)
    }

    pub fn set_text_font(
        &self,
        control: i64,
        start: i64,
        end: i64,
        face: &str,
        points: i64,
    ) -> Result<bool, String> {
        let (start, end) = format_range(start, end)?;
        let face_units = face.encode_utf16().count();
        if face_units == 0 {
            return Err("font face must not be empty".to_owned());
        }
        if face.contains('\0') {
            return Err("font face must not contain NUL".to_owned());
        }
        if face_units > 31 {
            return Err("font face must fit 31 UTF-16 code units".to_owned());
        }
        let points =
            i32::try_from(points).map_err(|_| "font size must fit signed 32-bit".to_owned())?;
        if points <= 0 {
            return Err("font size must be positive".to_owned());
        }
        let height = points
            .checked_mul(20)
            .ok_or_else(|| "font size in twips exceeds signed 32-bit".to_owned())?;
        platform::set_text_font(self.window_handle(control)?, start, end, face, height)
    }

    pub fn can_undo(&self, control: i64) -> Result<bool, String> {
        platform::can_undo(self.window_handle(control)?)
    }

    pub fn undo(&self, control: i64) -> Result<bool, String> {
        platform::undo(self.window_handle(control)?)
    }

    pub fn can_redo(&self, control: i64) -> Result<bool, String> {
        platform::can_redo(self.window_handle(control)?)
    }

    pub fn redo(&self, control: i64) -> Result<bool, String> {
        platform::redo(self.window_handle(control)?)
    }

    pub fn clear_history(&self, control: i64) -> Result<bool, String> {
        platform::clear_history(self.window_handle(control)?)
    }

    pub fn event_mask(&self, control: i64) -> Result<i64, String> {
        platform::event_mask(self.window_handle(control)?)
    }

    pub fn set_event_mask(&self, control: i64, mask: i64) -> Result<i64, String> {
        let mask = u32::try_from(mask)
            .map_err(|_| "RichEdit event mask must fit unsigned 32-bit".to_owned())?;
        platform::set_event_mask(self.window_handle(control)?, mask)
    }

    pub fn first_visible_line(&self, control: i64) -> Result<i64, String> {
        platform::first_visible_line(self.window_handle(control)?)
    }

    pub fn scroll_lines(&self, control: i64, lines: i64) -> Result<bool, String> {
        let lines = i32::try_from(lines)
            .map_err(|_| "line scroll count must fit signed 32-bit".to_owned())?;
        platform::scroll_lines(self.window_handle(control)?, lines)
    }

    pub fn focus(&self, window: i64) -> Result<bool, String> {
        platform::focus(self.window_handle(window)?)
    }

    pub fn find_text(
        &self,
        control: i64,
        text: &str,
        start: i64,
        end: i64,
        match_case: bool,
        whole_word: bool,
    ) -> Result<Option<(i64, i64)>, String> {
        if text.is_empty() {
            return Err("search text must not be empty".to_owned());
        }
        if start < 0 || end < -1 {
            return Err("search positions must be nonnegative, except end may be -1".to_owned());
        }
        if end >= 0 && start > end {
            return Err("search start must not exceed its end".to_owned());
        }
        let start =
            i32::try_from(start).map_err(|_| "search start must fit signed 32-bit".to_owned())?;
        let end = i32::try_from(end).map_err(|_| "search end must fit signed 32-bit".to_owned())?;
        platform::find_text(
            self.window_handle(control)?,
            text,
            start,
            end,
            match_case,
            whole_word,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn open_file_dialog(
        &self,
        owner: i64,
        title: &str,
        initial_directory: &str,
        file_name: &str,
        filter_name: &str,
        filter_pattern: &str,
        default_extension: &str,
    ) -> Result<Option<String>, String> {
        validate_file_dialog_options(
            title,
            initial_directory,
            file_name,
            filter_name,
            filter_pattern,
            default_extension,
        )?;
        platform::file_dialog(
            false,
            self.optional_window_handle(owner)?,
            title,
            initial_directory,
            file_name,
            filter_name,
            filter_pattern,
            default_extension,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save_file_dialog(
        &self,
        owner: i64,
        title: &str,
        initial_directory: &str,
        file_name: &str,
        filter_name: &str,
        filter_pattern: &str,
        default_extension: &str,
    ) -> Result<Option<String>, String> {
        validate_file_dialog_options(
            title,
            initial_directory,
            file_name,
            filter_name,
            filter_pattern,
            default_extension,
        )?;
        platform::file_dialog(
            true,
            self.optional_window_handle(owner)?,
            title,
            initial_directory,
            file_name,
            filter_name,
            filter_pattern,
            default_extension,
        )
    }

    fn window_handle(&self, window: i64) -> Result<usize, String> {
        self.windows
            .get(&window)
            .map(|window| window.handle)
            .ok_or_else(|| format!("unknown Windows window identity `{window}`"))
    }

    fn menu_handle(&self, menu: i64) -> Result<usize, String> {
        self.menus
            .get(&menu)
            .map(|menu| menu.handle)
            .ok_or_else(|| format!("unknown Windows menu identity `{menu}`"))
    }

    fn menu_subtree(&self, menu: i64) -> Vec<i64> {
        let mut pending = vec![menu];
        let mut result = Vec::new();
        while let Some(identity) = pending.pop() {
            if result.contains(&identity) {
                continue;
            }
            result.push(identity);
            if let Some(menu) = self.menus.get(&identity) {
                pending.extend(menu.children.iter().copied());
            }
        }
        result
    }

    fn redraw_menu(&self, menu: i64) -> Result<(), String> {
        let mut root = menu;
        while let Some(parent) = self.menus.get(&root).and_then(|menu| menu.parent) {
            root = parent;
        }
        if let Some(window) = self
            .menus
            .get(&root)
            .and_then(|menu| menu.attached_window)
            .and_then(|window| self.windows.get(&window))
        {
            platform::draw_menu_bar(window.handle)?;
        }
        Ok(())
    }

    fn optional_window_handle(&self, window: i64) -> Result<usize, String> {
        if window == 0 {
            Ok(0)
        } else {
            self.window_handle(window)
        }
    }

    fn remove_closed_windows(&mut self) {
        let closed = self
            .windows
            .iter()
            .filter_map(|(identity, window)| {
                (!platform::is_window(window.handle)).then_some((
                    *identity,
                    window.menu,
                    window.icon,
                    window.accelerators,
                ))
            })
            .collect::<Vec<_>>();
        for (identity, menu, icon, accelerators) in closed {
            self.windows.remove(&identity);
            if let Some(menu) = menu {
                for identity in self.menu_subtree(menu) {
                    self.menus.remove(&identity);
                }
            }
            if let Some(icon) = icon.and_then(|icon| self.icons.get_mut(&icon)) {
                icon.windows.remove(&identity);
            }
            if let Some(accelerators) =
                accelerators.and_then(|accelerators| self.accelerators.get_mut(&accelerators))
            {
                accelerators.windows.remove(&identity);
            }
        }
    }

    fn capture_events(&mut self) {
        for event in platform::take_events() {
            let Some(window) = self
                .windows
                .iter()
                .find_map(|(id, window)| (window.handle == event.window).then_some(*id))
            else {
                continue;
            };
            self.events.push_back(WindowEvent {
                window,
                message: i64::from(event.message),
                wparam: event.wparam as i64,
                lparam: event.lparam as i64,
                source: self
                    .windows
                    .iter()
                    .find_map(|(id, candidate)| {
                        (event.source != 0 && candidate.handle == event.source).then_some(*id)
                    })
                    .unwrap_or(0),
                code: i64::from(event.code),
                control_id: event.control_id as i64,
            });
        }
    }
}

fn format_range(start: i64, end: i64) -> Result<(i32, i32), String> {
    if start < 0 || end < -1 {
        return Err("format range positions must be nonnegative, except end may be -1".to_owned());
    }
    if end >= 0 && start > end {
        return Err("format range start must not exceed its end".to_owned());
    }
    Ok((
        i32::try_from(start).map_err(|_| "format range start must fit signed 32-bit".to_owned())?,
        i32::try_from(end).map_err(|_| "format range end must fit signed 32-bit".to_owned())?,
    ))
}

fn menu_command_id(command_id: i64) -> Result<u16, String> {
    u16::try_from(command_id)
        .ok()
        .filter(|command_id| *command_id != 0)
        .ok_or_else(|| "menu command id must be between 1 and 65535".to_owned())
}

fn format_color(color: i64, purpose: &str) -> Result<u32, String> {
    u32::try_from(color)
        .ok()
        .filter(|color| *color <= 0x00ff_ffff)
        .ok_or_else(|| format!("{purpose} must be between 0 and 0xFFFFFF"))
}

fn validate_file_dialog_options(
    title: &str,
    initial_directory: &str,
    file_name: &str,
    filter_name: &str,
    filter_pattern: &str,
    default_extension: &str,
) -> Result<(), String> {
    for (label, value) in [
        ("file dialog title", title),
        ("file dialog initial directory", initial_directory),
        ("file dialog file name", file_name),
        ("file dialog filter name", filter_name),
        ("file dialog filter pattern", filter_pattern),
        ("file dialog default extension", default_extension),
    ] {
        if value.contains('\0') {
            return Err(format!("{label} must not contain NUL"));
        }
    }
    if filter_name.is_empty() {
        return Err("file dialog filter name must not be empty".to_owned());
    }
    if filter_pattern.is_empty() {
        return Err("file dialog filter pattern must not be empty".to_owned());
    }
    if file_name.encode_utf16().count() >= 32_768 {
        return Err("file dialog file name is too long".to_owned());
    }
    Ok(())
}

impl Drop for WindowsHost {
    fn drop(&mut self) {
        for window in std::mem::take(&mut self.windows).into_values() {
            let _ = platform::destroy_window(window.handle);
        }
        let menus = std::mem::take(&mut self.menus);
        for menu in menus
            .values()
            .filter(|menu| menu.parent.is_none() && menu.attached_window.is_none())
        {
            let _ = platform::destroy_menu(menu.handle);
        }
        for icon in std::mem::take(&mut self.icons).into_values() {
            let _ = platform::destroy_icon(icon.handle);
        }
        for accelerators in std::mem::take(&mut self.accelerators).into_values() {
            let _ = platform::destroy_accelerators(accelerators.handle);
        }
        for module in std::mem::take(&mut self.modules).into_values() {
            let _ = platform::free_library(module.handle);
        }
        for module in std::mem::take(&mut self.support_modules) {
            let _ = platform::free_library(module);
        }
    }
}

pub fn utf16_bytes(value: &str) -> Vec<u8> {
    value
        .encode_utf16()
        .chain(Some(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

pub fn last_error() -> i64 {
    platform::last_error()
}

pub fn message_box(text: &str, caption: &str, kind: i64) -> Result<i64, String> {
    platform::message_box(text, caption, kind)
}

pub fn run_process(
    program: &str,
    arguments: &[String],
    input: &str,
    working_directory: &str,
) -> Result<ProcessOutput, String> {
    if program.is_empty() {
        return Err("process program must not be empty".to_owned());
    }
    platform::run_process(program, arguments, input, working_directory)
}

#[cfg(windows)]
mod platform {
    use std::{
        cell::{Cell, RefCell},
        collections::{BTreeMap, BTreeSet, VecDeque},
        ffi::CString,
        io::{ErrorKind, Write},
        mem::zeroed,
        os::windows::process::CommandExt,
        process::{Command, Stdio},
        ptr::null_mut,
        thread,
    };

    use super::ProcessOutput;

    type Handle = *mut core::ffi::c_void;
    type WParam = usize;
    type LParam = isize;
    type LResult = isize;

    const CLASS_NAME: &str = "APL.Runtime.Window.1";
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const COLOR_WINDOW: usize = 5;
    const CS_HREDRAW: u32 = 0x0002;
    const CS_VREDRAW: u32 = 0x0001;
    const IDC_ARROW: usize = 32512;
    const EM_GETSEL: u32 = 0x00b0;
    const EM_SETSEL: u32 = 0x00b1;
    const EM_LINESCROLL: u32 = 0x00b6;
    const EM_GETLINECOUNT: u32 = 0x00ba;
    const EM_CANUNDO: u32 = 0x00c6;
    const EM_UNDO: u32 = 0x00c7;
    const EM_REPLACESEL: u32 = 0x00c2;
    const EM_EMPTYUNDOBUFFER: u32 = 0x00cd;
    const EM_GETFIRSTVISIBLELINE: u32 = 0x00ce;
    const EM_GETEVENTMASK: u32 = 0x043b;
    const EM_GETOLEINTERFACE: u32 = 0x043c;
    const EM_SETCHARFORMAT: u32 = 0x0444;
    const EM_SETEVENTMASK: u32 = 0x0445;
    const EM_REDO: u32 = 0x0454;
    const EM_CANREDO: u32 = 0x0455;
    const EM_FINDTEXTEXW: u32 = 0x047c;
    const CFM_BOLD: u32 = 0x0000_0001;
    const CFE_BOLD: u32 = 0x0000_0001;
    const CFM_BACKCOLOR: u32 = 0x0400_0000;
    const CFM_FACE: u32 = 0x2000_0000;
    const CFM_COLOR: u32 = 0x4000_0000;
    const CFM_SIZE: u32 = 0x8000_0000;
    const SCF_SELECTION: usize = 0x0001;
    const FR_DOWN: usize = 0x0001;
    const FR_WHOLEWORD: usize = 0x0002;
    const FR_MATCHCASE: usize = 0x0004;
    const OFN_OVERWRITEPROMPT: u32 = 0x0000_0002;
    const OFN_NOCHANGEDIR: u32 = 0x0000_0008;
    const OFN_PATHMUSTEXIST: u32 = 0x0000_0800;
    const OFN_FILEMUSTEXIST: u32 = 0x0000_1000;
    const OFN_EXPLORER: u32 = 0x0008_0000;
    const MF_BYCOMMAND: u32 = 0x0000;
    const MF_ENABLED: u32 = 0x0000;
    const MF_STRING: u32 = 0x0000;
    const MF_GRAYED: u32 = 0x0001;
    const MF_CHECKED: u32 = 0x0008;
    const MF_POPUP: u32 = 0x0010;
    const MF_SEPARATOR: u32 = 0x0800;
    const MF_UNCHECKED: u32 = 0x0000;
    const IMAGE_ICON: u32 = 1;
    const LR_LOADFROMFILE: u32 = 0x0010;
    const LR_DEFAULTSIZE: u32 = 0x0040;
    const ICON_SMALL: usize = 0;
    const ICON_BIG: usize = 1;
    const TOM_SUSPEND: i32 = -9_999_995;
    const TOM_RESUME: i32 = -9_999_994;
    const PM_REMOVE: u32 = 0x0001;
    const FVIRTKEY: u8 = 0x01;
    const GA_ROOT: u32 = 2;
    const SW_SHOW: i32 = 5;
    const WM_DESTROY: u32 = 0x0002;
    const WM_MOVE: u32 = 0x0003;
    const WM_SIZE: u32 = 0x0005;
    const WM_CLOSE: u32 = 0x0010;
    const WM_QUIT: u32 = 0x0012;
    const WM_NOTIFY: u32 = 0x004e;
    const WM_SETICON: u32 = 0x0080;
    const WM_COMMAND: u32 = 0x0111;
    const WM_KEYDOWN: u32 = 0x0100;
    const WM_KEYUP: u32 = 0x0101;
    const WM_CHAR: u32 = 0x0102;
    const WM_MOUSEMOVE: u32 = 0x0200;
    const WM_LBUTTONDOWN: u32 = 0x0201;
    const WM_LBUTTONUP: u32 = 0x0202;
    const WM_RBUTTONDOWN: u32 = 0x0204;
    const WM_RBUTTONUP: u32 = 0x0205;
    const WS_OVERLAPPEDWINDOW: u32 = 0x00cf_0000;

    thread_local! {
        static WINDOW_COUNT: Cell<usize> = const { Cell::new(0) };
        static EVENTS: RefCell<VecDeque<RawWindowEvent>> = const { RefCell::new(VecDeque::new()) };
        static DEFERRED_CLOSES: RefCell<BTreeSet<usize>> = const { RefCell::new(BTreeSet::new()) };
        static ACCELERATORS: RefCell<BTreeMap<usize, usize>> = const { RefCell::new(BTreeMap::new()) };
    }

    pub struct RawWindowEvent {
        pub window: usize,
        pub message: u32,
        pub wparam: usize,
        pub lparam: isize,
        pub source: usize,
        pub code: u32,
        pub control_id: usize,
    }

    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    struct Message {
        window: Handle,
        message: u32,
        wparam: WParam,
        lparam: LParam,
        time: u32,
        point: Point,
        private: u32,
    }

    #[repr(C)]
    struct Accelerator {
        virtual_flags: u8,
        key: u16,
        command: u16,
    }

    #[repr(C)]
    struct CharFormat2 {
        size: u32,
        mask: u32,
        effects: u32,
        height: i32,
        offset: i32,
        text_color: u32,
        character_set: u8,
        pitch_and_family: u8,
        face_name: [u16; 32],
        weight: u16,
        spacing: i16,
        background_color: u32,
        locale: u32,
        reserved: u32,
        style: i16,
        kerning: u16,
        underline_type: u8,
        animation: u8,
        revision_author: u8,
        underline_color: u8,
    }

    #[repr(C)]
    struct NotificationHeader {
        source: Handle,
        control_id: usize,
        code: u32,
    }

    #[repr(C)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    const IID_ITEXT_DOCUMENT: Guid = Guid {
        data1: 0x8cc4_97c0,
        data2: 0xa1df,
        data3: 0x11ce,
        data4: [0x80, 0x98, 0x00, 0xaa, 0x00, 0x47, 0xbe, 0x5d],
    };

    #[repr(C)]
    struct ComObject {
        vtable: *const *const core::ffi::c_void,
    }

    struct ComPointer(*mut ComObject);

    impl Drop for ComPointer {
        fn drop(&mut self) {
            if self.0.is_null() {
                return;
            }
            type Release = unsafe extern "system" fn(*mut ComObject) -> u32;
            let method = unsafe { *(*self.0).vtable.add(2) };
            let release: Release = unsafe { std::mem::transmute(method) };
            unsafe {
                release(self.0);
            }
        }
    }

    #[derive(Clone, Copy)]
    #[repr(C)]
    struct CharacterRange {
        minimum: i32,
        maximum: i32,
    }

    #[repr(C)]
    struct FindTextEx {
        range: CharacterRange,
        text: *const u16,
        found: CharacterRange,
    }

    type WindowProcedure = unsafe extern "system" fn(Handle, u32, WParam, LParam) -> LResult;

    #[repr(C)]
    struct WindowClass {
        size: u32,
        style: u32,
        procedure: Option<WindowProcedure>,
        class_extra: i32,
        window_extra: i32,
        instance: Handle,
        icon: Handle,
        cursor: Handle,
        background: Handle,
        menu_name: *const u16,
        class_name: *const u16,
        small_icon: Handle,
    }

    #[repr(C)]
    struct OpenFileName {
        size: u32,
        owner: Handle,
        instance: Handle,
        filter: *const u16,
        custom_filter: *mut u16,
        maximum_custom_filter: u32,
        filter_index: u32,
        file: *mut u16,
        maximum_file: u32,
        file_title: *mut u16,
        maximum_file_title: u32,
        initial_directory: *const u16,
        title: *const u16,
        flags: u32,
        file_offset: u16,
        file_extension: u16,
        default_extension: *const u16,
        custom_data: LParam,
        hook: *mut core::ffi::c_void,
        template_name: *const u16,
        reserved: *mut core::ffi::c_void,
        reserved_flags: u32,
        flags_ex: u32,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryW(name: *const u16) -> Handle;
        fn GetProcAddress(module: Handle, name: *const u8) -> *mut core::ffi::c_void;
        fn FreeLibrary(module: Handle) -> i32;
        fn GetLastError() -> u32;
        fn GetModuleHandleW(name: *const u16) -> Handle;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(window: Handle, text: *const u16, caption: *const u16, kind: u32) -> i32;
        fn RegisterClassExW(class: *const WindowClass) -> u16;
        fn LoadCursorW(instance: Handle, cursor_name: *const u16) -> Handle;
        fn LoadImageW(
            instance: Handle,
            name: *const u16,
            image_type: u32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> Handle;
        fn CreateWindowExW(
            extended_style: u32,
            class_name: *const u16,
            window_name: *const u16,
            style: u32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: Handle,
            menu: Handle,
            instance: Handle,
            parameter: *mut core::ffi::c_void,
        ) -> Handle;
        fn DefWindowProcW(window: Handle, message: u32, wparam: WParam, lparam: LParam) -> LResult;
        fn DestroyWindow(window: Handle) -> i32;
        fn CreateMenu() -> Handle;
        fn CreatePopupMenu() -> Handle;
        fn AppendMenuW(menu: Handle, flags: u32, item: usize, text: *const u16) -> i32;
        fn SetMenu(window: Handle, menu: Handle) -> i32;
        fn DrawMenuBar(window: Handle) -> i32;
        fn EnableMenuItem(menu: Handle, item: u32, flags: u32) -> u32;
        fn CheckMenuItem(menu: Handle, item: u32, flags: u32) -> u32;
        fn DestroyMenu(menu: Handle) -> i32;
        fn DestroyIcon(icon: Handle) -> i32;
        fn CreateAcceleratorTableW(accelerators: *const Accelerator, count: i32) -> Handle;
        fn DestroyAcceleratorTable(accelerators: Handle) -> i32;
        fn TranslateAcceleratorW(
            window: Handle,
            accelerators: Handle,
            message: *const Message,
        ) -> i32;
        fn GetAncestor(window: Handle, flags: u32) -> Handle;
        fn DispatchMessageW(message: *const Message) -> LResult;
        fn GetMessageW(message: *mut Message, window: Handle, minimum: u32, maximum: u32) -> i32;
        fn IsWindow(window: Handle) -> i32;
        fn PeekMessageW(
            message: *mut Message,
            window: Handle,
            minimum: u32,
            maximum: u32,
            remove: u32,
        ) -> i32;
        #[cfg(test)]
        fn PostMessageW(window: Handle, message: u32, wparam: WParam, lparam: LParam) -> i32;
        fn PostQuitMessage(exit_code: i32);
        fn ShowWindow(window: Handle, command: i32) -> i32;
        fn TranslateMessage(message: *const Message) -> i32;
        fn UpdateWindow(window: Handle) -> i32;
        fn SetWindowTextW(window: Handle, text: *const u16) -> i32;
        fn GetWindowTextLengthW(window: Handle) -> i32;
        fn GetWindowTextW(window: Handle, text: *mut u16, maximum: i32) -> i32;
        fn MoveWindow(window: Handle, x: i32, y: i32, width: i32, height: i32, repaint: i32)
            -> i32;
        fn SendMessageW(window: Handle, message: u32, wparam: WParam, lparam: LParam) -> LResult;
        fn SetFocus(window: Handle) -> Handle;
        fn GetFocus() -> Handle;
    }

    #[link(name = "comdlg32")]
    unsafe extern "system" {
        fn GetOpenFileNameW(file_name: *mut OpenFileName) -> i32;
        fn GetSaveFileNameW(file_name: *mut OpenFileName) -> i32;
        fn CommDlgExtendedError() -> u32;
    }

    unsafe extern "system" fn window_procedure(
        window: Handle,
        message: u32,
        wparam: WParam,
        lparam: LParam,
    ) -> LResult {
        if captures_event(message) {
            let (source, code, control_id) = if message == WM_COMMAND {
                (
                    lparam as usize,
                    ((wparam >> 16) & 0xffff) as u32,
                    wparam & 0xffff,
                )
            } else if message == WM_NOTIFY && lparam != 0 {
                let header = unsafe { &*(lparam as *const NotificationHeader) };
                (header.source as usize, header.code, header.control_id)
            } else {
                (0, 0, 0)
            };
            EVENTS.with(|events| {
                events.borrow_mut().push_back(RawWindowEvent {
                    window: window as usize,
                    message,
                    wparam,
                    lparam: if matches!(message, WM_COMMAND | WM_NOTIFY) {
                        0
                    } else {
                        lparam
                    },
                    source,
                    code,
                    control_id,
                });
            });
        }
        if message == WM_DESTROY {
            ACCELERATORS.with(|accelerators| {
                accelerators.borrow_mut().remove(&(window as usize));
            });
            DEFERRED_CLOSES.with(|windows| {
                windows.borrow_mut().remove(&(window as usize));
            });
            WINDOW_COUNT.with(|count| {
                let remaining = count.get().saturating_sub(1);
                count.set(remaining);
                if remaining == 0 {
                    unsafe { PostQuitMessage(0) };
                }
            });
            return 0;
        }
        if message == WM_CLOSE
            && DEFERRED_CLOSES.with(|windows| windows.borrow().contains(&(window as usize)))
        {
            return 0;
        }
        unsafe { DefWindowProcW(window, message, wparam, lparam) }
    }

    fn captures_event(message: u32) -> bool {
        matches!(
            message,
            WM_DESTROY
                | WM_MOVE
                | WM_SIZE
                | WM_CLOSE
                | WM_NOTIFY
                | WM_COMMAND
                | WM_KEYDOWN
                | WM_KEYUP
                | WM_CHAR
                | WM_MOUSEMOVE
                | WM_LBUTTONDOWN
                | WM_LBUTTONUP
                | WM_RBUTTONDOWN
                | WM_RBUTTONUP
        )
    }

    pub fn take_events() -> Vec<RawWindowEvent> {
        EVENTS.with(|events| events.borrow_mut().drain(..).collect())
    }

    #[cfg(test)]
    pub fn send_command(window: usize, control: usize, control_id: usize, notification_code: u16) {
        unsafe {
            let wparam = control_id | (usize::from(notification_code) << 16);
            SendMessageW(window as Handle, WM_COMMAND, wparam, control as LParam);
        }
    }

    #[cfg(test)]
    pub fn send_notification(window: usize, control: usize, control_id: usize, code: u32) {
        let header = NotificationHeader {
            source: control as Handle,
            control_id,
            code,
        };
        unsafe {
            SendMessageW(
                window as Handle,
                WM_NOTIFY,
                control_id,
                (&header as *const NotificationHeader) as LParam,
            );
        }
    }

    fn wide(value: &str, purpose: &str) -> Result<Vec<u16>, String> {
        if value.encode_utf16().any(|unit| unit == 0) {
            return Err(format!("{purpose} contains a NUL character"));
        }
        Ok(value.encode_utf16().chain(Some(0)).collect())
    }

    fn register_window_class() -> Result<(Handle, Vec<u16>), String> {
        let class_name = wide(CLASS_NAME, "Windows class name")?;
        let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
        if instance.is_null() {
            return Err(format!(
                "GetModuleHandleW failed with Windows error {}",
                last_error()
            ));
        }
        let class = WindowClass {
            size: std::mem::size_of::<WindowClass>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            procedure: Some(window_procedure),
            class_extra: 0,
            window_extra: 0,
            instance,
            icon: null_mut(),
            cursor: unsafe { LoadCursorW(null_mut(), IDC_ARROW as *const u16) },
            background: (COLOR_WINDOW + 1) as Handle,
            menu_name: std::ptr::null(),
            class_name: class_name.as_ptr(),
            small_icon: null_mut(),
        };
        if unsafe { RegisterClassExW(&class) } == 0 {
            let error = last_error();
            if error != 1410 {
                return Err(format!(
                    "RegisterClassExW failed with Windows error {error}"
                ));
            }
        }
        Ok((instance, class_name))
    }

    pub fn load_library(name: &str) -> Result<usize, String> {
        if name.encode_utf16().any(|unit| unit == 0) {
            return Err("Windows library name contains a NUL character".to_owned());
        }
        let wide = name.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let handle = unsafe { LoadLibraryW(wide.as_ptr()) };
        if handle.is_null() {
            return Err(format!(
                "LoadLibraryW failed for `{name}` with Windows error {}",
                last_error()
            ));
        }
        Ok(handle as usize)
    }

    pub fn procedure_address(module: usize, name: &str) -> Result<usize, String> {
        let name = CString::new(name)
            .map_err(|_| "Windows procedure name contains a NUL character".to_owned())?;
        let address = unsafe { GetProcAddress(module as Handle, name.as_ptr().cast()) };
        if address.is_null() {
            return Err(format!(
                "GetProcAddress failed with Windows error {}",
                last_error()
            ));
        }
        Ok(address as usize)
    }

    pub fn free_library(module: usize) -> Result<(), String> {
        if unsafe { FreeLibrary(module as Handle) } == 0 {
            return Err(format!(
                "FreeLibrary failed with Windows error {}",
                last_error()
            ));
        }
        Ok(())
    }

    pub fn last_error() -> i64 {
        unsafe { GetLastError() as i64 }
    }

    pub fn message_box(text: &str, caption: &str, kind: i64) -> Result<i64, String> {
        let kind = u32::try_from(kind)
            .map_err(|_| "MessageBoxW kind must fit an unsigned 32-bit value".to_owned())?;
        let text = wide(text, "MessageBoxW text")?;
        let caption = wide(caption, "MessageBoxW caption")?;
        Ok(unsafe { MessageBoxW(null_mut(), text.as_ptr(), caption.as_ptr(), kind) as i64 })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn file_dialog(
        save: bool,
        owner: usize,
        title: &str,
        initial_directory: &str,
        file_name: &str,
        filter_name: &str,
        filter_pattern: &str,
        default_extension: &str,
    ) -> Result<Option<String>, String> {
        const MAXIMUM_FILE_UNITS: usize = 32_768;
        let mut file = vec![0; MAXIMUM_FILE_UNITS];
        let encoded_name = file_name.encode_utf16().collect::<Vec<_>>();
        file[..encoded_name.len()].copy_from_slice(&encoded_name);

        let title = wide(title, "file dialog title")?;
        let initial_directory = wide(initial_directory, "file dialog initial directory")?;
        let default_extension = default_extension.trim_start_matches('.');
        let default_extension = wide(default_extension, "file dialog default extension")?;
        let filter = dialog_filter(filter_name, filter_pattern);
        let mut dialog: OpenFileName = unsafe { zeroed() };
        dialog.size = std::mem::size_of::<OpenFileName>() as u32;
        dialog.owner = owner as Handle;
        dialog.filter = filter.as_ptr();
        dialog.filter_index = 1;
        dialog.file = file.as_mut_ptr();
        dialog.maximum_file = file.len() as u32;
        dialog.initial_directory = if initial_directory.len() == 1 {
            std::ptr::null()
        } else {
            initial_directory.as_ptr()
        };
        dialog.title = if title.len() == 1 {
            std::ptr::null()
        } else {
            title.as_ptr()
        };
        dialog.default_extension = if default_extension.len() == 1 {
            std::ptr::null()
        } else {
            default_extension.as_ptr()
        };
        dialog.flags = OFN_EXPLORER | OFN_NOCHANGEDIR | OFN_PATHMUSTEXIST;
        dialog.flags |= if save {
            OFN_OVERWRITEPROMPT
        } else {
            OFN_FILEMUSTEXIST
        };

        let accepted = unsafe {
            if save {
                GetSaveFileNameW(&mut dialog)
            } else {
                GetOpenFileNameW(&mut dialog)
            }
        };
        if accepted == 0 {
            let error = unsafe { CommDlgExtendedError() };
            if error == 0 {
                return Ok(None);
            }
            return Err(format!(
                "{} failed with common-dialog error 0x{error:04X}",
                if save {
                    "GetSaveFileNameW"
                } else {
                    "GetOpenFileNameW"
                }
            ));
        }

        let length = file
            .iter()
            .position(|unit| *unit == 0)
            .ok_or_else(|| "file dialog returned an unterminated path".to_owned())?;
        String::from_utf16(&file[..length])
            .map(Some)
            .map_err(|_| "file dialog returned invalid UTF-16".to_owned())
    }

    fn dialog_filter(name: &str, pattern: &str) -> Vec<u16> {
        let mut filter = Vec::new();
        filter.extend(name.encode_utf16());
        filter.push(0);
        filter.extend(pattern.encode_utf16());
        filter.push(0);
        filter.extend("All files (*.*)".encode_utf16());
        filter.push(0);
        filter.extend("*.*".encode_utf16());
        filter.extend([0, 0]);
        filter
    }

    pub fn run_process(
        program: &str,
        arguments: &[String],
        input: &str,
        working_directory: &str,
    ) -> Result<ProcessOutput, String> {
        let mut command = Command::new(program);
        command
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW);
        if !working_directory.is_empty() {
            command.current_dir(working_directory);
        }

        let mut child = command
            .spawn()
            .map_err(|error| format!("failed to start Windows process `{program}`: {error}"))?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "failed to open child process stdin".to_owned())?;
        let input = input.as_bytes().to_vec();
        let writer = thread::spawn(move || -> Result<(), String> {
            match stdin.write_all(&input) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == ErrorKind::BrokenPipe => Ok(()),
                Err(error) => Err(format!("failed to write child process stdin: {error}")),
            }
        });
        let output = child
            .wait_with_output()
            .map_err(|error| format!("failed to wait for Windows process `{program}`: {error}"))?;
        writer
            .join()
            .map_err(|_| "child process stdin writer panicked".to_owned())??;

        Ok(ProcessOutput {
            exit_code: output.status.code().map(i64::from).unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }

    pub fn create_window(title: &str, width: i32, height: i32) -> Result<usize, String> {
        let (instance, class_name) = register_window_class()?;
        let title = wide(title, "window title")?;
        let use_default = 0x8000_0000_u32 as i32;
        let window = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW,
                use_default,
                use_default,
                width,
                height,
                null_mut(),
                null_mut(),
                instance,
                null_mut(),
            )
        };
        if window.is_null() {
            return Err(format!(
                "CreateWindowExW failed with Windows error {}",
                last_error()
            ));
        }
        WINDOW_COUNT.with(|count| count.set(count.get().saturating_add(1)));
        Ok(window as usize)
    }

    pub fn load_icon(path: &str, width: i32, height: i32) -> Result<usize, String> {
        let path = wide(path, "icon path")?;
        let flags = LR_LOADFROMFILE
            | if width == 0 || height == 0 {
                LR_DEFAULTSIZE
            } else {
                0
            };
        let icon =
            unsafe { LoadImageW(null_mut(), path.as_ptr(), IMAGE_ICON, width, height, flags) };
        if icon.is_null() {
            return Err(format!(
                "LoadImageW failed for icon with Windows error {}",
                last_error()
            ));
        }
        Ok(icon as usize)
    }

    pub fn create_accelerators(entries: &[(u8, u16, u16)]) -> Result<usize, String> {
        let entries = entries
            .iter()
            .map(|(modifiers, key, command)| Accelerator {
                virtual_flags: *modifiers | FVIRTKEY,
                key: *key,
                command: *command,
            })
            .collect::<Vec<_>>();
        let count = i32::try_from(entries.len())
            .map_err(|_| "accelerator table contains too many entries".to_owned())?;
        let handle = unsafe { CreateAcceleratorTableW(entries.as_ptr(), count) };
        if handle.is_null() {
            return Err(format!(
                "CreateAcceleratorTableW failed with Windows error {}",
                last_error()
            ));
        }
        Ok(handle as usize)
    }

    pub fn set_accelerators(window: usize, accelerators: usize) -> Result<(), String> {
        if !is_window(window) {
            return Err("Windows window handle is no longer valid".to_owned());
        }
        ACCELERATORS.with(|tables| {
            let mut tables = tables.borrow_mut();
            if accelerators == 0 {
                tables.remove(&window);
            } else {
                tables.insert(window, accelerators);
            }
        });
        Ok(())
    }

    pub fn destroy_accelerators(accelerators: usize) -> Result<(), String> {
        let destroyed = unsafe { DestroyAcceleratorTable(accelerators as Handle) };
        if destroyed == 0 {
            return Err(format!(
                "DestroyAcceleratorTable failed with Windows error {}",
                last_error()
            ));
        }
        Ok(())
    }

    pub fn set_window_icon(window: usize, icon: usize) -> Result<bool, String> {
        if !is_window(window) {
            return Err("Windows window handle is no longer valid".to_owned());
        }
        unsafe {
            SendMessageW(window as Handle, WM_SETICON, ICON_BIG, icon as LParam);
            SendMessageW(window as Handle, WM_SETICON, ICON_SMALL, icon as LParam);
        }
        Ok(true)
    }

    pub fn set_deferred_close(window: usize, enabled: bool) -> Result<bool, String> {
        if !is_window(window) {
            return Err("Windows window handle is no longer valid".to_owned());
        }
        DEFERRED_CLOSES.with(|windows| {
            if enabled {
                windows.borrow_mut().insert(window);
            } else {
                windows.borrow_mut().remove(&window);
            }
        });
        Ok(true)
    }

    pub fn destroy_icon(icon: usize) -> Result<(), String> {
        if unsafe { DestroyIcon(icon as Handle) } == 0 {
            return Err(format!(
                "DestroyIcon failed with Windows error {}",
                last_error()
            ));
        }
        Ok(())
    }

    pub fn create_menu(popup: bool) -> Result<usize, String> {
        let menu = unsafe {
            if popup {
                CreatePopupMenu()
            } else {
                CreateMenu()
            }
        };
        if menu.is_null() {
            return Err(format!(
                "Create{}Menu failed with Windows error {}",
                if popup { "Popup" } else { "" },
                last_error()
            ));
        }
        Ok(menu as usize)
    }

    pub fn append_menu_item(menu: usize, command_id: u16, text: &str) -> Result<bool, String> {
        let text = wide(text, "menu item text")?;
        if unsafe {
            AppendMenuW(
                menu as Handle,
                MF_STRING,
                usize::from(command_id),
                text.as_ptr(),
            )
        } == 0
        {
            return Err(format!(
                "AppendMenuW failed with Windows error {}",
                last_error()
            ));
        }
        Ok(true)
    }

    pub fn append_submenu(menu: usize, submenu: usize, text: &str) -> Result<bool, String> {
        let text = wide(text, "submenu text")?;
        if unsafe { AppendMenuW(menu as Handle, MF_POPUP | MF_STRING, submenu, text.as_ptr()) } == 0
        {
            return Err(format!(
                "AppendMenuW failed with Windows error {}",
                last_error()
            ));
        }
        Ok(true)
    }

    pub fn append_menu_separator(menu: usize) -> Result<bool, String> {
        if unsafe { AppendMenuW(menu as Handle, MF_SEPARATOR, 0, std::ptr::null()) } == 0 {
            return Err(format!(
                "AppendMenuW failed with Windows error {}",
                last_error()
            ));
        }
        Ok(true)
    }

    pub fn set_menu(window: usize, menu: usize) -> Result<bool, String> {
        if !is_window(window) {
            return Err("Windows window handle is no longer valid".to_owned());
        }
        let menu = if menu == 0 {
            null_mut()
        } else {
            menu as Handle
        };
        if unsafe { SetMenu(window as Handle, menu) } == 0 {
            return Err(format!(
                "SetMenu failed with Windows error {}",
                last_error()
            ));
        }
        Ok(true)
    }

    pub fn draw_menu_bar(window: usize) -> Result<bool, String> {
        if !is_window(window) {
            return Err("Windows window handle is no longer valid".to_owned());
        }
        if unsafe { DrawMenuBar(window as Handle) } == 0 {
            return Err(format!(
                "DrawMenuBar failed with Windows error {}",
                last_error()
            ));
        }
        Ok(true)
    }

    pub fn set_menu_item_enabled(
        menu: usize,
        command_id: u16,
        enabled: bool,
    ) -> Result<bool, String> {
        let flags = MF_BYCOMMAND | if enabled { MF_ENABLED } else { MF_GRAYED };
        if unsafe { EnableMenuItem(menu as Handle, u32::from(command_id), flags) } == u32::MAX {
            return Err(format!("unknown menu command id `{command_id}`"));
        }
        Ok(true)
    }

    pub fn set_menu_item_checked(
        menu: usize,
        command_id: u16,
        checked: bool,
    ) -> Result<bool, String> {
        let flags = MF_BYCOMMAND | if checked { MF_CHECKED } else { MF_UNCHECKED };
        if unsafe { CheckMenuItem(menu as Handle, u32::from(command_id), flags) } == u32::MAX {
            return Err(format!("unknown menu command id `{command_id}`"));
        }
        Ok(true)
    }

    pub fn destroy_menu(menu: usize) -> Result<(), String> {
        if unsafe { DestroyMenu(menu as Handle) } == 0 {
            return Err(format!(
                "DestroyMenu failed with Windows error {}",
                last_error()
            ));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_control(
        parent: usize,
        class_name: &str,
        text: &str,
        control_id: usize,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        style: u32,
        extended_style: u32,
    ) -> Result<usize, String> {
        if !is_window(parent) {
            return Err("parent window handle is no longer valid".to_owned());
        }
        let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
        if instance.is_null() {
            return Err(format!(
                "GetModuleHandleW failed with Windows error {}",
                last_error()
            ));
        }
        let class_name = wide(class_name, "control class name")?;
        let text = wide(text, "control text")?;
        let control = unsafe {
            CreateWindowExW(
                extended_style,
                class_name.as_ptr(),
                text.as_ptr(),
                style,
                x,
                y,
                width,
                height,
                parent as Handle,
                control_id as Handle,
                instance,
                null_mut(),
            )
        };
        if control.is_null() {
            return Err(format!(
                "CreateWindowExW failed for control with Windows error {}",
                last_error()
            ));
        }
        Ok(control as usize)
    }

    pub fn show_window(window: usize) -> Result<bool, String> {
        if !is_window(window) {
            return Err("Windows window handle is no longer valid".to_owned());
        }
        unsafe {
            ShowWindow(window as Handle, SW_SHOW);
        }
        if unsafe { UpdateWindow(window as Handle) } == 0 {
            return Err(format!(
                "UpdateWindow failed with Windows error {}",
                last_error()
            ));
        }
        Ok(true)
    }

    pub fn destroy_window(window: usize) -> Result<(), String> {
        if !is_window(window) {
            return Ok(());
        }
        if unsafe { DestroyWindow(window as Handle) } == 0 {
            return Err(format!(
                "DestroyWindow failed with Windows error {}",
                last_error()
            ));
        }
        Ok(())
    }

    pub fn set_window_text(window: usize, text: &str) -> Result<bool, String> {
        if !is_window(window) {
            return Err("Windows window handle is no longer valid".to_owned());
        }
        let text = wide(text, "window text")?;
        if unsafe { SetWindowTextW(window as Handle, text.as_ptr()) } == 0 {
            return Err(format!(
                "SetWindowTextW failed with Windows error {}",
                last_error()
            ));
        }
        Ok(true)
    }

    pub fn window_text(window: usize) -> Result<String, String> {
        if !is_window(window) {
            return Err("Windows window handle is no longer valid".to_owned());
        }
        let length = unsafe { GetWindowTextLengthW(window as Handle) };
        if length < 0 {
            return Err(format!(
                "GetWindowTextLengthW failed with Windows error {}",
                last_error()
            ));
        }
        let mut text = vec![0u16; length as usize + 1];
        let copied = unsafe { GetWindowTextW(window as Handle, text.as_mut_ptr(), length + 1) };
        if copied < 0 {
            return Err(format!(
                "GetWindowTextW failed with Windows error {}",
                last_error()
            ));
        }
        text.truncate(copied as usize);
        String::from_utf16(&text).map_err(|_| "window text is not valid UTF-16".to_owned())
    }

    pub fn move_window(
        window: usize,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        repaint: bool,
    ) -> Result<bool, String> {
        if !is_window(window) {
            return Err("Windows window handle is no longer valid".to_owned());
        }
        if width <= 0 || height <= 0 {
            return Err("window width and height must be positive".to_owned());
        }
        if unsafe { MoveWindow(window as Handle, x, y, width, height, i32::from(repaint)) } == 0 {
            return Err(format!(
                "MoveWindow failed with Windows error {}",
                last_error()
            ));
        }
        Ok(true)
    }

    pub fn selection(window: usize) -> Result<(i64, i64), String> {
        if !is_window(window) {
            return Err("Windows control handle is no longer valid".to_owned());
        }
        let mut start = 0_u32;
        let mut end = 0_u32;
        unsafe {
            SendMessageW(
                window as Handle,
                EM_GETSEL,
                (&mut start as *mut u32) as WParam,
                (&mut end as *mut u32) as LParam,
            );
        }
        Ok((i64::from(start), i64::from(end)))
    }

    pub fn set_selection(window: usize, start: i32, end: i32) -> Result<bool, String> {
        if !is_window(window) {
            return Err("Windows control handle is no longer valid".to_owned());
        }
        unsafe {
            SendMessageW(
                window as Handle,
                EM_SETSEL,
                start as isize as WParam,
                end as LParam,
            );
        }
        Ok(true)
    }

    pub fn replace_selection(window: usize, text: &str, can_undo: bool) -> Result<bool, String> {
        if !is_window(window) {
            return Err("Windows control handle is no longer valid".to_owned());
        }
        let text = wide(text, "replacement text")?;
        unsafe {
            SendMessageW(
                window as Handle,
                EM_REPLACESEL,
                usize::from(can_undo),
                text.as_ptr() as LParam,
            );
        }
        Ok(true)
    }

    pub fn line_count(window: usize) -> Result<i64, String> {
        if !is_window(window) {
            return Err("Windows control handle is no longer valid".to_owned());
        }
        let count = unsafe { SendMessageW(window as Handle, EM_GETLINECOUNT, 0, 0) };
        i64::try_from(count).map_err(|_| "edit line count does not fit APL Int".to_owned())
    }

    pub fn set_text_color(window: usize, start: i32, end: i32, color: u32) -> Result<bool, String> {
        apply_character_format(window, start, end, CFM_COLOR, 0, color, 0, 0, [0; 32])
    }

    pub fn set_background_color(
        window: usize,
        start: i32,
        end: i32,
        color: u32,
    ) -> Result<bool, String> {
        apply_character_format(window, start, end, CFM_BACKCOLOR, 0, 0, color, 0, [0; 32])
    }

    pub fn set_text_bold(
        window: usize,
        start: i32,
        end: i32,
        enabled: bool,
    ) -> Result<bool, String> {
        apply_character_format(
            window,
            start,
            end,
            CFM_BOLD,
            if enabled { CFE_BOLD } else { 0 },
            0,
            0,
            0,
            [0; 32],
        )
    }

    pub fn set_text_font(
        window: usize,
        start: i32,
        end: i32,
        face: &str,
        height: i32,
    ) -> Result<bool, String> {
        let mut face_name = [0; 32];
        for (target, unit) in face_name.iter_mut().zip(face.encode_utf16()) {
            *target = unit;
        }
        apply_character_format(
            window,
            start,
            end,
            CFM_FACE | CFM_SIZE,
            0,
            0,
            0,
            height,
            face_name,
        )
    }

    fn apply_character_format(
        window: usize,
        start: i32,
        end: i32,
        mask: u32,
        effects: u32,
        text_color: u32,
        background_color: u32,
        height: i32,
        face_name: [u16; 32],
    ) -> Result<bool, String> {
        if !is_window(window) {
            return Err("Windows control handle is no longer valid".to_owned());
        }
        let document = text_document(window)?;
        set_undo_mode(&document, TOM_SUSPEND)?;
        let operation = apply_character_format_suspended(
            window,
            start,
            end,
            mask,
            effects,
            text_color,
            background_color,
            height,
            face_name,
        );
        let resumed = set_undo_mode(&document, TOM_RESUME);
        operation?;
        resumed?;
        Ok(true)
    }

    fn apply_character_format_suspended(
        window: usize,
        start: i32,
        end: i32,
        mask: u32,
        effects: u32,
        text_color: u32,
        background_color: u32,
        height: i32,
        face_name: [u16; 32],
    ) -> Result<(), String> {
        let (saved_start, saved_end) = selection(window)?;
        let saved_start = i32::try_from(saved_start)
            .map_err(|_| "saved selection start does not fit signed 32-bit".to_owned())?;
        let saved_end = i32::try_from(saved_end)
            .map_err(|_| "saved selection end does not fit signed 32-bit".to_owned())?;
        set_selection(window, start, end)?;
        let format = CharFormat2 {
            size: std::mem::size_of::<CharFormat2>() as u32,
            mask,
            effects,
            height,
            offset: 0,
            text_color,
            character_set: 0,
            pitch_and_family: 0,
            face_name,
            weight: 0,
            spacing: 0,
            background_color,
            locale: 0,
            reserved: 0,
            style: 0,
            kerning: 0,
            underline_type: 0,
            animation: 0,
            revision_author: 0,
            underline_color: 0,
        };
        let result = unsafe {
            SendMessageW(
                window as Handle,
                EM_SETCHARFORMAT,
                SCF_SELECTION,
                (&format as *const CharFormat2) as LParam,
            )
        };
        set_selection(window, saved_start, saved_end)?;
        if result == 0 {
            return Err("RichEdit rejected the character format".to_owned());
        }
        Ok(())
    }

    fn text_document(window: usize) -> Result<ComPointer, String> {
        let mut rich_edit_ole: *mut ComObject = null_mut();
        let result = unsafe {
            SendMessageW(
                window as Handle,
                EM_GETOLEINTERFACE,
                0,
                (&mut rich_edit_ole as *mut *mut ComObject) as LParam,
            )
        };
        if result == 0 || rich_edit_ole.is_null() {
            return Err("control does not expose the RichEdit OLE interface".to_owned());
        }
        let rich_edit_ole = ComPointer(rich_edit_ole);
        type QueryInterface = unsafe extern "system" fn(
            *mut ComObject,
            *const Guid,
            *mut *mut core::ffi::c_void,
        ) -> i32;
        let method = unsafe { *(*rich_edit_ole.0).vtable };
        let query_interface: QueryInterface = unsafe { std::mem::transmute(method) };
        let mut document: *mut core::ffi::c_void = null_mut();
        let status = unsafe {
            query_interface(
                rich_edit_ole.0,
                &IID_ITEXT_DOCUMENT,
                &mut document as *mut *mut core::ffi::c_void,
            )
        };
        if status < 0 || document.is_null() {
            return Err(format!(
                "RichEdit ITextDocument query failed with HRESULT 0x{:08X}",
                status as u32
            ));
        }
        Ok(ComPointer(document.cast()))
    }

    fn set_undo_mode(document: &ComPointer, mode: i32) -> Result<(), String> {
        type Undo = unsafe extern "system" fn(*mut ComObject, i32, *mut i32) -> i32;
        const UNDO_VTABLE_INDEX: usize = 22;
        let method = unsafe { *(*document.0).vtable.add(UNDO_VTABLE_INDEX) };
        let undo: Undo = unsafe { std::mem::transmute(method) };
        let mut affected = 0;
        let status = unsafe { undo(document.0, mode, &mut affected) };
        if status < 0 {
            return Err(format!(
                "RichEdit undo mode change failed with HRESULT 0x{:08X}",
                status as u32
            ));
        }
        Ok(())
    }

    pub fn can_undo(window: usize) -> Result<bool, String> {
        send_boolean_edit_message(window, EM_CANUNDO)
    }

    pub fn undo(window: usize) -> Result<bool, String> {
        send_boolean_edit_message(window, EM_UNDO)
    }

    pub fn can_redo(window: usize) -> Result<bool, String> {
        send_boolean_edit_message(window, EM_CANREDO)
    }

    pub fn redo(window: usize) -> Result<bool, String> {
        send_boolean_edit_message(window, EM_REDO)
    }

    fn send_boolean_edit_message(window: usize, message: u32) -> Result<bool, String> {
        if !is_window(window) {
            return Err("Windows control handle is no longer valid".to_owned());
        }
        Ok(unsafe { SendMessageW(window as Handle, message, 0, 0) } != 0)
    }

    pub fn clear_history(window: usize) -> Result<bool, String> {
        if !is_window(window) {
            return Err("Windows control handle is no longer valid".to_owned());
        }
        unsafe {
            SendMessageW(window as Handle, EM_EMPTYUNDOBUFFER, 0, 0);
        }
        Ok(true)
    }

    pub fn event_mask(window: usize) -> Result<i64, String> {
        if !is_window(window) {
            return Err("Windows control handle is no longer valid".to_owned());
        }
        let mask = unsafe { SendMessageW(window as Handle, EM_GETEVENTMASK, 0, 0) };
        Ok(mask as u32 as i64)
    }

    pub fn set_event_mask(window: usize, mask: u32) -> Result<i64, String> {
        if !is_window(window) {
            return Err("Windows control handle is no longer valid".to_owned());
        }
        let previous = unsafe {
            SendMessageW(
                window as Handle,
                EM_SETEVENTMASK,
                0,
                mask as usize as LParam,
            )
        };
        Ok(previous as u32 as i64)
    }

    pub fn first_visible_line(window: usize) -> Result<i64, String> {
        if !is_window(window) {
            return Err("Windows control handle is no longer valid".to_owned());
        }
        let line = unsafe { SendMessageW(window as Handle, EM_GETFIRSTVISIBLELINE, 0, 0) };
        i64::try_from(line).map_err(|_| "first visible line does not fit APL Int".to_owned())
    }

    pub fn scroll_lines(window: usize, lines: i32) -> Result<bool, String> {
        if !is_window(window) {
            return Err("Windows control handle is no longer valid".to_owned());
        }
        Ok(unsafe { SendMessageW(window as Handle, EM_LINESCROLL, 0, lines as LParam) } != 0)
    }

    pub fn focus(window: usize) -> Result<bool, String> {
        if !is_window(window) {
            return Err("Windows control handle is no longer valid".to_owned());
        }
        unsafe {
            SetFocus(window as Handle);
            Ok(GetFocus() == window as Handle)
        }
    }

    pub fn find_text(
        window: usize,
        text: &str,
        start: i32,
        end: i32,
        match_case: bool,
        whole_word: bool,
    ) -> Result<Option<(i64, i64)>, String> {
        if !is_window(window) {
            return Err("Windows control handle is no longer valid".to_owned());
        }
        let text = wide(text, "search text")?;
        let mut request = FindTextEx {
            range: CharacterRange {
                minimum: start,
                maximum: end,
            },
            text: text.as_ptr(),
            found: CharacterRange {
                minimum: -1,
                maximum: -1,
            },
        };
        let mut flags = FR_DOWN;
        if match_case {
            flags |= FR_MATCHCASE;
        }
        if whole_word {
            flags |= FR_WHOLEWORD;
        }
        let result = unsafe {
            SendMessageW(
                window as Handle,
                EM_FINDTEXTEXW,
                flags,
                (&mut request as *mut FindTextEx) as LParam,
            )
        };
        if result < 0 {
            return Ok(None);
        }
        if request.found.minimum < 0 || request.found.maximum < request.found.minimum {
            return Err("RichEdit returned an invalid search range".to_owned());
        }
        Ok(Some((
            i64::from(request.found.minimum),
            i64::from(request.found.maximum),
        )))
    }

    fn translate_and_dispatch(message: &Message) {
        let root = if message.window.is_null() {
            message.window
        } else {
            unsafe { GetAncestor(message.window, GA_ROOT) }
        };
        let translated = if root.is_null() {
            false
        } else {
            ACCELERATORS.with(|tables| {
                tables
                    .borrow()
                    .get(&(root as usize))
                    .copied()
                    .map(|accelerators| unsafe {
                        TranslateAcceleratorW(root, accelerators as Handle, message) != 0
                    })
                    .unwrap_or(false)
            })
        };
        if !translated {
            unsafe {
                TranslateMessage(message);
                DispatchMessageW(message);
            }
        }
    }

    pub fn run_message_loop() -> Result<i64, String> {
        let mut message: Message = unsafe { zeroed() };
        loop {
            let status = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
            if status == -1 {
                return Err(format!(
                    "GetMessageW failed with Windows error {}",
                    last_error()
                ));
            }
            if status == 0 {
                return Ok(message.wparam as i64);
            }
            translate_and_dispatch(&message);
        }
    }

    pub fn pump_messages() -> Result<bool, String> {
        let mut message: Message = unsafe { zeroed() };
        while unsafe { PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) } != 0 {
            if message.message == WM_QUIT {
                return Ok(false);
            }
            translate_and_dispatch(&message);
        }
        Ok(true)
    }

    pub fn wait_event_step() -> Result<bool, String> {
        let mut message: Message = unsafe { zeroed() };
        let status = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
        if status == -1 {
            return Err(format!(
                "GetMessageW failed with Windows error {}",
                last_error()
            ));
        }
        if status == 0 {
            return Ok(false);
        }
        translate_and_dispatch(&message);
        Ok(true)
    }

    #[cfg(test)]
    pub fn post_command(window: usize, command_id: usize) {
        let posted = unsafe { PostMessageW(window as Handle, WM_COMMAND, command_id, 0) };
        assert_ne!(posted, 0, "PostMessageW failed with error {}", last_error());
    }

    #[cfg(test)]
    pub fn post_close(window: usize) {
        let posted = unsafe { PostMessageW(window as Handle, WM_CLOSE, 0, 0) };
        assert_ne!(posted, 0, "PostMessageW failed with error {}", last_error());
    }

    #[cfg(test)]
    pub fn post_key(window: usize, key: usize) {
        let posted = unsafe { PostMessageW(window as Handle, WM_KEYDOWN, key, 0) };
        assert_ne!(posted, 0, "PostMessageW failed with error {}", last_error());
    }

    pub fn is_window(window: usize) -> bool {
        (unsafe { IsWindow(window as Handle) }) != 0
    }
}

#[cfg(not(windows))]
mod platform {
    use super::ProcessOutput;

    pub struct RawWindowEvent {
        pub window: usize,
        pub message: u32,
        pub wparam: usize,
        pub lparam: isize,
        pub source: usize,
        pub code: u32,
        pub control_id: usize,
    }

    fn unavailable() -> String {
        "Windows API is unavailable on this target".to_owned()
    }

    pub fn load_library(_name: &str) -> Result<usize, String> {
        Err(unavailable())
    }

    pub fn procedure_address(_module: usize, _name: &str) -> Result<usize, String> {
        Err(unavailable())
    }

    pub fn free_library(_module: usize) -> Result<(), String> {
        Err(unavailable())
    }

    pub fn last_error() -> i64 {
        0
    }

    pub fn message_box(_text: &str, _caption: &str, _kind: i64) -> Result<i64, String> {
        Err(unavailable())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn file_dialog(
        _save: bool,
        _owner: usize,
        _title: &str,
        _initial_directory: &str,
        _file_name: &str,
        _filter_name: &str,
        _filter_pattern: &str,
        _default_extension: &str,
    ) -> Result<Option<String>, String> {
        Err(unavailable())
    }

    pub fn run_process(
        _program: &str,
        _arguments: &[String],
        _input: &str,
        _working_directory: &str,
    ) -> Result<ProcessOutput, String> {
        Err(unavailable())
    }

    pub fn create_window(_title: &str, _width: i32, _height: i32) -> Result<usize, String> {
        Err(unavailable())
    }

    pub fn load_icon(_path: &str, _width: i32, _height: i32) -> Result<usize, String> {
        Err(unavailable())
    }

    pub fn set_window_icon(_window: usize, _icon: usize) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn set_deferred_close(_window: usize, _enabled: bool) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn destroy_icon(_icon: usize) -> Result<(), String> {
        Err(unavailable())
    }

    pub fn create_accelerators(_entries: &[(u8, u16, u16)]) -> Result<usize, String> {
        Err(unavailable())
    }

    pub fn set_accelerators(_window: usize, _accelerators: usize) -> Result<(), String> {
        Err(unavailable())
    }

    pub fn destroy_accelerators(_accelerators: usize) -> Result<(), String> {
        Err(unavailable())
    }

    pub fn create_menu(_popup: bool) -> Result<usize, String> {
        Err(unavailable())
    }

    pub fn append_menu_item(_menu: usize, _command_id: u16, _text: &str) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn append_submenu(_menu: usize, _submenu: usize, _text: &str) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn append_menu_separator(_menu: usize) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn set_menu(_window: usize, _menu: usize) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn draw_menu_bar(_window: usize) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn set_menu_item_enabled(
        _menu: usize,
        _command_id: u16,
        _enabled: bool,
    ) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn set_menu_item_checked(
        _menu: usize,
        _command_id: u16,
        _checked: bool,
    ) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn destroy_menu(_menu: usize) -> Result<(), String> {
        Err(unavailable())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_control(
        _parent: usize,
        _class_name: &str,
        _text: &str,
        _control_id: usize,
        _x: i32,
        _y: i32,
        _width: i32,
        _height: i32,
        _style: u32,
        _extended_style: u32,
    ) -> Result<usize, String> {
        Err(unavailable())
    }

    pub fn show_window(_window: usize) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn destroy_window(_window: usize) -> Result<(), String> {
        Err(unavailable())
    }

    pub fn set_window_text(_window: usize, _text: &str) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn window_text(_window: usize) -> Result<String, String> {
        Err(unavailable())
    }

    pub fn move_window(
        _window: usize,
        _x: i32,
        _y: i32,
        _width: i32,
        _height: i32,
        _repaint: bool,
    ) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn selection(_window: usize) -> Result<(i64, i64), String> {
        Err(unavailable())
    }

    pub fn set_selection(_window: usize, _start: i32, _end: i32) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn replace_selection(_window: usize, _text: &str, _can_undo: bool) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn line_count(_window: usize) -> Result<i64, String> {
        Err(unavailable())
    }

    pub fn set_text_color(
        _window: usize,
        _start: i32,
        _end: i32,
        _color: u32,
    ) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn set_background_color(
        _window: usize,
        _start: i32,
        _end: i32,
        _color: u32,
    ) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn set_text_bold(
        _window: usize,
        _start: i32,
        _end: i32,
        _enabled: bool,
    ) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn set_text_font(
        _window: usize,
        _start: i32,
        _end: i32,
        _face: &str,
        _height: i32,
    ) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn can_undo(_window: usize) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn undo(_window: usize) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn can_redo(_window: usize) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn redo(_window: usize) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn clear_history(_window: usize) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn event_mask(_window: usize) -> Result<i64, String> {
        Err(unavailable())
    }

    pub fn set_event_mask(_window: usize, _mask: u32) -> Result<i64, String> {
        Err(unavailable())
    }

    pub fn first_visible_line(_window: usize) -> Result<i64, String> {
        Err(unavailable())
    }

    pub fn scroll_lines(_window: usize, _lines: i32) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn focus(_window: usize) -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn find_text(
        _window: usize,
        _text: &str,
        _start: i32,
        _end: i32,
        _match_case: bool,
        _whole_word: bool,
    ) -> Result<Option<(i64, i64)>, String> {
        Err(unavailable())
    }

    pub fn run_message_loop() -> Result<i64, String> {
        Err(unavailable())
    }

    pub fn pump_messages() -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn wait_event_step() -> Result<bool, String> {
        Err(unavailable())
    }

    pub fn is_window(_window: usize) -> bool {
        false
    }

    pub fn take_events() -> Vec<RawWindowEvent> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_nul_terminated_utf16le() {
        assert_eq!(
            utf16_bytes("A😀"),
            vec![0x41, 0x00, 0x3d, 0xd8, 0x00, 0xde, 0x00, 0x00]
        );
    }

    #[test]
    fn validates_file_dialog_text_before_crossing_the_platform_boundary() {
        assert_eq!(
            validate_file_dialog_options("", "", "", "", "*.apl", "apl").unwrap_err(),
            "file dialog filter name must not be empty"
        );
        assert_eq!(
            validate_file_dialog_options("", "", "", "APL", "", "apl").unwrap_err(),
            "file dialog filter pattern must not be empty"
        );
        assert_eq!(
            validate_file_dialog_options("bad\0title", "", "", "APL", "*.apl", "apl").unwrap_err(),
            "file dialog title must not contain NUL"
        );
    }

    #[test]
    fn validates_menu_command_identity_without_truncation() {
        assert_eq!(
            menu_command_id(0).unwrap_err(),
            "menu command id must be between 1 and 65535"
        );
        assert_eq!(menu_command_id(65_535).unwrap(), 65_535);
        assert_eq!(
            menu_command_id(65_536).unwrap_err(),
            "menu command id must be between 1 and 65535"
        );
    }

    #[test]
    #[cfg(windows)]
    fn loads_and_resolves_a_system_library() {
        let mut host = WindowsHost::default();
        let library = host.load_library("kernel32.dll").unwrap();
        assert!(
            host.procedure_address(library, "GetCurrentProcessId")
                .unwrap()
                > 0
        );
        assert!(host.close_library(library).unwrap().is_empty());
    }

    #[test]
    #[cfg(windows)]
    fn runs_a_hidden_process_with_input_and_captured_output() {
        let arguments = [
            "/D".to_owned(),
            "/V:ON".to_owned(),
            "/S".to_owned(),
            "/C".to_owned(),
            "set /p value=& <nul set /p =OUT:!value!& echo ERR>&2& exit /b 7".to_owned(),
        ];
        let output = run_process("cmd.exe", &arguments, "hello\n", "").unwrap();
        assert_eq!(output.exit_code, 7);
        assert_eq!(output.stdout, "OUT:hello");
        assert_eq!(output.stderr, "ERR\r\n");
    }

    #[test]
    #[cfg(windows)]
    fn queues_destroy_events_with_the_stable_window_identity() {
        let mut host = WindowsHost::default();
        let window = host.create_window("APL event test", 320, 200).unwrap();
        while host.next_event().is_some() {}
        host.destroy_window(window).unwrap();
        assert!(host
            .next_event()
            .is_some_and(|event| event.window == window && event.message == 0x0002));
    }

    #[test]
    #[cfg(windows)]
    fn command_events_resolve_the_stable_control_identity() {
        let mut host = WindowsHost::default();
        let window = host.create_window("APL command test", 320, 200).unwrap();
        let control = host
            .create_control(
                window,
                "STATIC",
                "label",
                101,
                0,
                0,
                120,
                28,
                0x5000_0000,
                0,
            )
            .unwrap();
        while host.next_event().is_some() {}

        platform::send_command(
            host.window_handle(window).unwrap(),
            host.window_handle(control).unwrap(),
            101,
            0x0300,
        );

        assert!(host.next_event().is_some_and(|event| {
            event.window == window
                && event.message == 0x0111
                && event.source == control
                && event.code == 0x0300
                && event.control_id == 101
                && event.lparam == 0
        }));

        platform::send_notification(
            host.window_handle(window).unwrap(),
            host.window_handle(control).unwrap(),
            101,
            0x0702,
        );
        assert!(host.next_event().is_some_and(|event| {
            event.window == window
                && event.message == 0x004e
                && event.source == control
                && event.code == 0x0702
                && event.control_id == 101
                && event.lparam == 0
        }));
    }

    #[test]
    #[cfg(windows)]
    fn wait_event_blocks_through_the_native_message_queue() {
        let mut host = WindowsHost::default();
        let window = host.create_window("APL wait event test", 320, 200).unwrap();
        while host.next_event().is_some() {}
        platform::post_command(host.window_handle(window).unwrap(), 401);
        let event = host.wait_event().unwrap().expect("posted command event");
        assert_eq!(event.window, window);
        assert_eq!(event.message, 0x0111);
        assert_eq!(event.control_id, 401);
        assert_eq!(event.source, 0);
        assert_eq!(event.code, 0);
        host.destroy_window(window).unwrap();
    }

    #[test]
    #[cfg(windows)]
    fn deferred_close_waits_for_an_explicit_apl_decision() {
        let mut host = WindowsHost::default();
        let window = host
            .create_window("APL deferred close test", 320, 200)
            .unwrap();
        assert!(host.set_deferred_close(window, true).unwrap());
        while host.next_event().is_some() {}
        platform::post_close(host.window_handle(window).unwrap());
        let event = host.wait_event().unwrap().expect("deferred close event");
        assert_eq!(event.window, window);
        assert_eq!(event.message, 0x0010);
        assert!(platform::is_window(host.window_handle(window).unwrap()));
        assert!(host.destroy_window(window).unwrap());
    }

    #[test]
    #[cfg(windows)]
    fn accelerator_tables_translate_keys_and_detach_before_destruction() {
        let mut host = WindowsHost::default();
        let window = host
            .create_window("APL accelerator test", 320, 200)
            .unwrap();
        let accelerators = host
            .create_accelerators(&[AcceleratorEntry {
                key: i64::from(b'N'),
                command: 501,
                modifiers: 0,
            }])
            .unwrap();
        assert!(host.set_accelerators(window, accelerators).unwrap());
        while host.next_event().is_some() {}
        platform::post_key(host.window_handle(window).unwrap(), b'N' as usize);
        let event = host
            .wait_event()
            .unwrap()
            .expect("accelerator command event");
        assert_eq!(event.window, window);
        assert_eq!(event.message, 0x0111);
        assert_eq!(event.control_id, 501);
        assert_eq!(event.source, 0);
        assert_eq!(event.code, 1);
        assert!(host.destroy_accelerators(accelerators).unwrap());
        assert_eq!(host.windows.get(&window).unwrap().accelerators, None);
        assert!(!host.accelerators.contains_key(&accelerators));
        assert!(host.destroy_window(window).unwrap());
    }

    #[test]
    #[cfg(windows)]
    fn owns_loaded_icons_and_detaches_them_before_destruction() {
        let path = std::env::temp_dir().join(format!(
            "apl_windows_icon_{}_{}.ico",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
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
        std::fs::write(&path, bytes).unwrap();

        let mut host = WindowsHost::default();
        let window = host.create_window("APL icon test", 320, 200).unwrap();
        let icon = host.load_icon(path.to_str().unwrap(), 16, 16).unwrap();
        assert!(host.set_window_icon(window, icon).unwrap());
        assert_eq!(host.windows.get(&window).unwrap().icon, Some(icon));
        assert!(host.icons.get(&icon).unwrap().windows.contains(&window));
        assert!(host.destroy_icon(icon).unwrap());
        assert_eq!(host.windows.get(&window).unwrap().icon, None);
        assert!(!host.icons.contains_key(&icon));

        let icon = host.load_icon(path.to_str().unwrap(), 0, 0).unwrap();
        assert!(host.set_window_icon(window, icon).unwrap());
        assert!(host.destroy_window(window).unwrap());
        assert!(host.icons.get(&icon).unwrap().windows.is_empty());
        assert!(host.destroy_icon(icon).unwrap());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    #[cfg(windows)]
    fn menus_keep_stable_ownership_and_emit_command_ids() {
        let mut host = WindowsHost::default();
        let window = host.create_window("APL menu test", 320, 200).unwrap();
        let menu = host.create_menu(false).unwrap();
        let file = host.create_menu(true).unwrap();
        assert!(host.append_menu_item(file, 101, "Open").unwrap());
        assert!(host.append_menu_separator(file).unwrap());
        assert!(host.append_menu_item(file, 102, "Exit").unwrap());
        assert!(host.append_submenu(menu, file, "File").unwrap());
        assert!(host.set_menu(window, menu).unwrap());
        assert!(host.set_menu_item_enabled(file, 101, false).unwrap());
        assert!(host.set_menu_item_enabled(file, 101, true).unwrap());
        assert!(host.set_menu_item_checked(file, 101, true).unwrap());
        assert!(host.set_menu_item_checked(file, 101, false).unwrap());
        assert!(host.destroy_menu(file).is_err());

        while host.next_event().is_some() {}
        platform::send_command(host.window_handle(window).unwrap(), 0, 101, 0);
        assert!(host.next_event().is_some_and(|event| {
            event.window == window
                && event.message == 0x0111
                && event.source == 0
                && event.code == 0
                && event.control_id == 101
        }));

        assert!(host.destroy_menu(menu).unwrap());
        assert!(host.menu_handle(menu).is_err());
        assert!(host.menu_handle(file).is_err());

        let replacement = host.create_menu(false).unwrap();
        let replacement_popup = host.create_menu(true).unwrap();
        assert!(host
            .append_submenu(replacement, replacement_popup, "Replacement")
            .unwrap());
        assert!(host.set_menu(window, replacement).unwrap());
        assert!(host.destroy_window(window).unwrap());
        assert!(host.menu_handle(replacement).is_err());
        assert!(host.menu_handle(replacement_popup).is_err());
    }

    #[test]
    #[cfg(windows)]
    fn edits_rich_text_without_exposing_native_pointers() {
        let mut host = WindowsHost::default();
        let window = host.create_window("APL RichEdit test", 320, 200).unwrap();
        let editor = host
            .create_control(
                window,
                "RICHEDIT50W",
                "one\r\ntwo",
                102,
                0,
                0,
                240,
                120,
                0x5000_0004,
                0,
            )
            .unwrap();

        assert_eq!(host.line_count(editor).unwrap(), 2);
        assert!(host.set_selection(editor, 0, 3).unwrap());
        assert_eq!(host.selection(editor).unwrap(), (0, 3));
        assert!(host.replace_selection(editor, "three", true).unwrap());
        assert_eq!(host.window_text(editor).unwrap(), "three\r\ntwo");
        assert_eq!(host.selection(editor).unwrap(), (5, 5));
        assert!(host.set_text_color(editor, 0, 5, 0x00_80_ff).unwrap());
        assert!(host.set_background_color(editor, 0, 5, 0x00_ff_e0).unwrap());
        assert!(host.set_text_bold(editor, 0, 5, true).unwrap());
        assert!(host.set_text_font(editor, 0, -1, "Consolas", 11).unwrap());
        assert_eq!(host.selection(editor).unwrap(), (5, 5));
        assert!(host.can_undo(editor).unwrap());
        assert!(host.undo(editor).unwrap());
        assert_eq!(host.window_text(editor).unwrap(), "one\r\ntwo");
        assert!(host.can_redo(editor).unwrap());
        assert!(host.redo(editor).unwrap());
        assert_eq!(host.window_text(editor).unwrap(), "three\r\ntwo");
        assert!(host.clear_history(editor).unwrap());
        assert!(!host.can_undo(editor).unwrap());
        assert!(!host.can_redo(editor).unwrap());
        let initial_mask = host.event_mask(editor).unwrap();
        assert_eq!(
            host.set_event_mask(editor, initial_mask | 0x0008_0000)
                .unwrap(),
            initial_mask
        );
        assert_eq!(host.event_mask(editor).unwrap(), initial_mask | 0x0008_0000);
        assert_eq!(
            host.set_event_mask(editor, initial_mask).unwrap(),
            initial_mask | 0x0008_0000
        );
        assert_eq!(host.first_visible_line(editor).unwrap(), 0);
        assert!(host.scroll_lines(editor, 1).unwrap());
        assert!(host.focus(editor).unwrap());
        assert_eq!(
            host.find_text(editor, "THREE", 0, -1, false, true).unwrap(),
            Some((0, 5))
        );
        assert_eq!(
            host.find_text(editor, "THREE", 0, -1, true, true).unwrap(),
            None
        );
    }
}
