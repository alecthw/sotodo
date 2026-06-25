use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday};
#[cfg(windows)]
use dioxus::desktop::tao::platform::windows::WindowExtWindows;
use dioxus::desktop::{
    tao::{dpi::PhysicalPosition, window::Icon as WindowIcon},
    Config, LogicalSize, WindowBuilder,
};
use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{
    LdAlarmClock, LdCalendar, LdCheck, LdChevronDown, LdChevronLeft, LdChevronRight,
    LdChevronsDown, LdChevronsUp, LdCircleCheck, LdClock, LdLanguages, LdListTodo, LdMinus,
    LdPalette, LdPencil, LdPin, LdPinOff, LdPlus, LdRepeat, LdSave, LdSettings, LdTrash2, LdX,
};
use dioxus_free_icons::Icon;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration as StdDuration;
use std::{collections::BTreeMap, fs, path::PathBuf};
use uuid::Uuid;
use winrt_toast_reborn::{
    register as register_toast_app, Scenario, Toast, ToastDuration, ToastManager,
};

const MAIN_CSS: &str = include_str!("../assets/main.css");
const TAILWIND_CSS: &str = include_str!("../assets/tailwind.css");
const APPICON_ICO_BYTES: &[u8] = include_bytes!("../assets/appicon.ico");
const TOAST_APP_ID: &str = "SoTodo.Dioxus.Desktop";
const TOAST_REMINDER_GROUP: &str = "sotodo-reminders";
const REMINDER_CATCH_UP_SECONDS: i64 = 60;
const REMINDER_MIN_DELAY_SECONDS: u64 = 3;
const UNSCHEDULED_DATE: (i32, u32, u32) = (9999, 12, 31);
const APP_WINDOW_WIDTH: f64 = 420.0;
const APP_WINDOW_HEIGHT: f64 = 720.0;
const APP_VERSION: &str = match option_env!("SOTODO_VERSION") {
    Some(version) => version,
    None => "develop",
};
const PROJECT_URL: &str = "https://github.com/alecthw/sotodo";
static REMINDER_THREAD_GENERATION: AtomicU64 = AtomicU64::new(0);

const THEMES: &[&str] = &[
    "light",
    "dark",
    "cupcake",
    "bumblebee",
    "emerald",
    "corporate",
    "synthwave",
    "retro",
    "cyberpunk",
    "valentine",
    "halloween",
    "garden",
    "forest",
    "aqua",
    "lofi",
    "pastel",
    "fantasy",
    "wireframe",
    "black",
    "luxury",
    "dracula",
    "cmyk",
    "autumn",
    "business",
    "acid",
    "lemonade",
    "night",
    "coffee",
    "winter",
    "dim",
    "nord",
    "sunset",
    "caramellatte",
    "abyss",
    "silk",
];

fn main() {
    let window = WindowBuilder::new()
        .with_title("So Todo")
        .with_inner_size(LogicalSize::new(APP_WINDOW_WIDTH, APP_WINDOW_HEIGHT))
        .with_min_inner_size(LogicalSize::new(360.0, 560.0))
        .with_decorations(false);

    let mut config = Config::new().with_window(window);
    if let Some(icon) = app_window_icon() {
        config = config.with_icon(icon);
    }

    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(App);
}

#[cfg(windows)]
fn snap_window_to_top_right() {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA};

    let mut work_area = RECT::default();
    let ok = unsafe {
        SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work_area as *mut _ as *mut _, 0) != 0
    };
    if !ok {
        return;
    }

    let window = &dioxus::desktop::window().window;
    let width = window.outer_size().width as i32;
    let x = (work_area.right - width).max(work_area.left);
    window.set_outer_position(PhysicalPosition::new(x, work_area.top));
}

#[component]
fn App() -> Element {
    rsx! {
        style { "{TAILWIND_CSS}\n{MAIN_CSS}" }
        TodoApp {}
    }
}

fn app_window_icon() -> Option<WindowIcon> {
    WindowIcon::from_rgba(app_icon_rgba(256), 256, 256).ok()
}

#[cfg(windows)]
fn open_project_homepage() {
    use std::ptr::null_mut;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let action = wide_null("open");
    let url = wide_null(PROJECT_URL);
    unsafe {
        ShellExecuteW(
            null_mut(),
            action.as_ptr(),
            url.as_ptr(),
            null_mut(),
            null_mut(),
            SW_SHOWNORMAL,
        );
    }
}

#[cfg(not(windows))]
fn open_project_homepage() {}

fn app_icon_rgba(size: u32) -> Vec<u8> {
    let scale = size as f32 / 456.0;
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);

    for y in 0..size {
        for x in 0..size {
            let point = (x as f32 + 0.5, y as f32 + 0.5);
            let mut color: [f32; 4] = [0.0, 0.0, 0.0, 0.0];
            if rounded_rect_contains(
                point,
                34.0 * scale,
                34.0 * scale,
                388.0 * scale,
                388.0 * scale,
                92.0 * scale,
            ) {
                color = [13.0, 148.0, 136.0, 255.0];
            }
            if rounded_rect_contains(
                point,
                112.0 * scale,
                78.0 * scale,
                232.0 * scale,
                300.0 * scale,
                34.0 * scale,
            ) {
                color = [248.0, 250.0, 252.0, 255.0];
            }
            for (start, end, width, rgb) in [
                ((152.0, 162.0), (276.0, 162.0), 22.0, [15.0, 23.0, 42.0]),
                ((152.0, 220.0), (250.0, 220.0), 22.0, [15.0, 23.0, 42.0]),
                ((152.0, 278.0), (218.0, 278.0), 22.0, [15.0, 23.0, 42.0]),
                ((220.0, 296.0), (264.0, 340.0), 36.0, [250.0, 204.0, 21.0]),
                ((264.0, 340.0), (356.0, 230.0), 36.0, [250.0, 204.0, 21.0]),
            ] {
                if distance_to_segment(
                    point,
                    (start.0 * scale, start.1 * scale),
                    (end.0 * scale, end.1 * scale),
                ) <= width * scale / 2.0
                {
                    color = [rgb[0], rgb[1], rgb[2], 255.0];
                }
            }
            pixels.extend(color.into_iter().map(|value| value.round() as u8));
        }
    }

    pixels
}

fn rounded_rect_contains(
    point: (f32, f32),
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
) -> bool {
    let px = point.0.clamp(x + radius, x + width - radius);
    let py = point.1.clamp(y + radius, y + height - radius);
    distance(point, (px, py)) <= radius
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn distance_to_segment(point: (f32, f32), start: (f32, f32), end: (f32, f32)) -> f32 {
    let segment = (end.0 - start.0, end.1 - start.1);
    let length_squared = segment.0 * segment.0 + segment.1 * segment.1;
    let t = (((point.0 - start.0) * segment.0 + (point.1 - start.1) * segment.1) / length_squared)
        .clamp(0.0, 1.0);
    distance(point, (start.0 + segment.0 * t, start.1 + segment.1 * t))
}

#[cfg(windows)]
#[derive(Clone)]
struct WindowsTray {
    hwnd: std::sync::Arc<std::sync::atomic::AtomicIsize>,
    enabled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    labels: std::sync::Arc<std::sync::Mutex<TrayLabels>>,
}

#[cfg(windows)]
#[derive(Clone)]
struct TrayLabels {
    show: String,
    exit: String,
}

#[cfg(windows)]
impl TrayLabels {
    fn new(language: Language) -> Self {
        match language {
            Language::Zh => Self {
                show: "\u{663e}\u{793a}\u{4e3b}\u{7a97}\u{53e3}".to_string(),
                exit: "\u{9000}\u{51fa}\u{7a0b}\u{5e8f}".to_string(),
            },
            Language::En => Self {
                show: "Show main window".to_string(),
                exit: "Exit app".to_string(),
            },
        }
    }
}

#[cfg(windows)]
impl WindowsTray {
    fn new(language: Language, main_hwnd: isize) -> Self {
        use std::sync::{atomic::AtomicBool, atomic::AtomicIsize, Arc};

        let hwnd = Arc::new(AtomicIsize::new(0));
        let enabled = Arc::new(AtomicBool::new(false));
        let labels = native_tray_labels(language);
        MAIN_WINDOW_HWND.store(main_hwnd, Ordering::SeqCst);
        {
            let mut current = labels
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *current = TrayLabels::new(language);
        }

        let thread_hwnd = Arc::clone(&hwnd);
        let thread_enabled = Arc::clone(&enabled);
        std::thread::spawn(move || unsafe {
            run_windows_tray(thread_hwnd, thread_enabled);
        });

        Self {
            hwnd,
            enabled,
            labels,
        }
    }

    fn set_main_window(&self, hwnd: isize) {
        MAIN_WINDOW_HWND.store(hwnd, Ordering::SeqCst);
    }

    fn set_language(&self, language: Language) {
        let mut labels = self
            .labels
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *labels = TrayLabels::new(language);
    }

    fn set_visible(&self, visible: bool) {
        self.enabled.store(visible, Ordering::SeqCst);
        let hwnd = self.hwnd.load(Ordering::SeqCst) as windows_sys::Win32::Foundation::HWND;
        if !hwnd.is_null() {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(
                    hwnd,
                    WM_TRAY_APPLY,
                    0,
                    0,
                );
            }
        }
    }
}

#[cfg(windows)]
static MAIN_WINDOW_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
#[cfg(windows)]
static TRAY_LABELS: std::sync::OnceLock<std::sync::Arc<std::sync::Mutex<TrayLabels>>> =
    std::sync::OnceLock::new();

#[cfg(windows)]
const WM_TRAY_ICON: u32 = 0x0400 + 10;
#[cfg(windows)]
const WM_TRAY_APPLY: u32 = 0x0400 + 11;
#[cfg(windows)]
const TRAY_UID: u32 = 1;
#[cfg(windows)]
const TRAY_MENU_SHOW: usize = 1001;
#[cfg(windows)]
const TRAY_MENU_EXIT: usize = 1002;

#[cfg(windows)]
fn native_tray_labels(language: Language) -> std::sync::Arc<std::sync::Mutex<TrayLabels>> {
    TRAY_LABELS
        .get_or_init(|| std::sync::Arc::new(std::sync::Mutex::new(TrayLabels::new(language))))
        .clone()
}

#[cfg(windows)]
fn current_main_hwnd() -> isize {
    dioxus::desktop::window().window.hwnd()
}

#[cfg(windows)]
unsafe fn run_windows_tray(
    hwnd_slot: std::sync::Arc<std::sync::atomic::AtomicIsize>,
    enabled: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    use std::ptr::null_mut;
    use windows_sys::Win32::UI::Shell::{Shell_NotifyIconW, NIM_DELETE};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyIcon, DestroyWindow, DispatchMessageW, GetMessageW, RegisterClassW,
        TranslateMessage, UnregisterClassW, HWND_MESSAGE, MSG, WNDCLASSW,
    };

    let class_name = wide_null(&format!("SoTodoTrayWindow{}", std::process::id()));
    let window_name = wide_null("");
    let window_class = WNDCLASSW {
        lpfnWndProc: Some(tray_window_proc),
        lpszClassName: class_name.as_ptr(),
        ..Default::default()
    };
    if RegisterClassW(&window_class) == 0 {
        return;
    }

    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        window_name.as_ptr(),
        0,
        0,
        0,
        0,
        0,
        HWND_MESSAGE,
        null_mut(),
        null_mut(),
        std::ptr::null(),
    );
    if hwnd.is_null() {
        UnregisterClassW(class_name.as_ptr(), null_mut());
        return;
    }
    hwnd_slot.store(hwnd as isize, Ordering::SeqCst);

    let (icon, owns_icon) = load_tray_icon();
    let mut icon_added = false;
    apply_tray_icon(hwnd, icon, enabled.load(Ordering::SeqCst), &mut icon_added);

    let mut message = MSG::default();
    while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
        if message.message == WM_TRAY_APPLY {
            apply_tray_icon(hwnd, icon, enabled.load(Ordering::SeqCst), &mut icon_added);
            continue;
        }
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }

    if icon_added {
        let data = notify_icon_data(hwnd, icon);
        Shell_NotifyIconW(NIM_DELETE, &data);
    }
    hwnd_slot.store(0, Ordering::SeqCst);
    DestroyWindow(hwnd);
    UnregisterClassW(class_name.as_ptr(), null_mut());
    if owns_icon && !icon.is_null() {
        DestroyIcon(icon);
    }
}

#[cfg(windows)]
unsafe extern "system" fn tray_window_proc(
    hwnd: windows_sys::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows_sys::Win32::Foundation::WPARAM,
    lparam: windows_sys::Win32::Foundation::LPARAM,
) -> windows_sys::Win32::Foundation::LRESULT {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DefWindowProcW, PostQuitMessage, WM_DESTROY, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP,
    };

    if msg == WM_TRAY_ICON {
        match lparam as u32 {
            WM_LBUTTONUP | WM_LBUTTONDBLCLK => {
                restore_main_window_native();
                return 0;
            }
            WM_RBUTTONUP => {
                show_tray_menu(hwnd);
                return 0;
            }
            _ => {}
        }
    }

    if msg == WM_DESTROY {
        PostQuitMessage(0);
        return 0;
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

#[cfg(windows)]
unsafe fn show_tray_menu(hwnd: windows_sys::Win32::Foundation::HWND) {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, SetForegroundWindow,
        TrackPopupMenu, MF_STRING, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    };

    let menu = CreatePopupMenu();
    if menu.is_null() {
        return;
    }

    let labels = TRAY_LABELS
        .get()
        .map(|labels| {
            labels
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone()
        })
        .unwrap_or_else(|| TrayLabels::new(Language::En));
    let show = wide_null(&labels.show);
    let exit = wide_null(&labels.exit);
    AppendMenuW(menu, MF_STRING, TRAY_MENU_SHOW, show.as_ptr());
    AppendMenuW(menu, MF_STRING, TRAY_MENU_EXIT, exit.as_ptr());

    let mut point = POINT { x: 0, y: 0 };
    if GetCursorPos(&mut point) == 0 {
        DestroyMenu(menu);
        return;
    }

    SetForegroundWindow(hwnd);
    let command = TrackPopupMenu(
        menu,
        TPM_RIGHTBUTTON | TPM_NONOTIFY | TPM_RETURNCMD,
        point.x,
        point.y,
        0,
        hwnd,
        std::ptr::null(),
    ) as usize;
    DestroyMenu(menu);

    match command {
        TRAY_MENU_SHOW => restore_main_window_native(),
        TRAY_MENU_EXIT => close_main_window_native(),
        _ => {}
    }
}

#[cfg(windows)]
unsafe fn apply_tray_icon(
    hwnd: windows_sys::Win32::Foundation::HWND,
    icon: windows_sys::Win32::UI::WindowsAndMessaging::HICON,
    visible: bool,
    icon_added: &mut bool,
) {
    use windows_sys::Win32::UI::Shell::{Shell_NotifyIconW, NIM_ADD, NIM_DELETE, NIM_MODIFY};

    let data = notify_icon_data(hwnd, icon);
    match (visible, *icon_added) {
        (true, false) => {
            *icon_added = Shell_NotifyIconW(NIM_ADD, &data) != 0;
        }
        (true, true) => {
            Shell_NotifyIconW(NIM_MODIFY, &data);
        }
        (false, true) => {
            Shell_NotifyIconW(NIM_DELETE, &data);
            *icon_added = false;
        }
        (false, false) => {}
    }
}

#[cfg(windows)]
fn notify_icon_data(
    hwnd: windows_sys::Win32::Foundation::HWND,
    icon: windows_sys::Win32::UI::WindowsAndMessaging::HICON,
) -> windows_sys::Win32::UI::Shell::NOTIFYICONDATAW {
    use windows_sys::Win32::UI::Shell::{NIF_ICON, NIF_MESSAGE, NIF_TIP, NOTIFYICONDATAW};

    let mut data = NOTIFYICONDATAW::default();
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = TRAY_UID;
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = WM_TRAY_ICON;
    data.hIcon = icon;
    write_wide_fixed(&mut data.szTip, "So Todo");
    data
}

#[cfg(windows)]
unsafe fn load_tray_icon() -> (windows_sys::Win32::UI::WindowsAndMessaging::HICON, bool) {
    use std::ptr::null_mut;
    use windows_sys::Win32::UI::WindowsAndMessaging::{LoadIconW, IDI_APPLICATION};

    if let Some(icon) = load_embedded_tray_icon() {
        return (icon, true);
    }

    (LoadIconW(null_mut(), IDI_APPLICATION), false)
}

#[cfg(windows)]
unsafe fn load_embedded_tray_icon() -> Option<windows_sys::Win32::UI::WindowsAndMessaging::HICON> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{CreateIconFromResourceEx, LR_DEFAULTCOLOR};

    let image = best_ico_image(APPICON_ICO_BYTES)?;
    let icon = CreateIconFromResourceEx(
        image.as_ptr(),
        image.len() as u32,
        1,
        0x0003_0000,
        32,
        32,
        LR_DEFAULTCOLOR,
    );
    (!icon.is_null()).then_some(icon)
}

#[cfg(windows)]
fn best_ico_image(bytes: &[u8]) -> Option<&[u8]> {
    if read_u16(bytes, 0)? != 0 || read_u16(bytes, 2)? != 1 {
        return None;
    }

    let count = read_u16(bytes, 4)? as usize;
    let mut best: Option<(usize, usize, u32)> = None;
    for index in 0..count {
        let entry = 6 + index * 16;
        let width = bytes.get(entry).copied()? as u32;
        let width = if width == 0 { 256 } else { width };
        let size = read_u32(bytes, entry + 8)? as usize;
        let offset = read_u32(bytes, entry + 12)? as usize;
        if offset.checked_add(size)? > bytes.len() {
            continue;
        }
        if best.map_or(true, |(_, _, best_width)| width >= best_width) {
            best = Some((offset, size, width));
        }
    }

    let (offset, size, _) = best?;
    bytes.get(offset..offset + size)
}

#[cfg(windows)]
fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

#[cfg(windows)]
fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

#[cfg(windows)]
unsafe fn restore_main_window_native() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
    };

    let hwnd = MAIN_WINDOW_HWND.load(Ordering::SeqCst) as windows_sys::Win32::Foundation::HWND;
    if hwnd.is_null() {
        return;
    }
    ShowWindow(hwnd, SW_SHOW);
    ShowWindow(hwnd, SW_RESTORE);
    SetForegroundWindow(hwnd);
}

#[cfg(windows)]
unsafe fn close_main_window_native() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

    let hwnd = MAIN_WINDOW_HWND.load(Ordering::SeqCst) as windows_sys::Win32::Foundation::HWND;
    if !hwnd.is_null() {
        PostMessageW(hwnd, WM_CLOSE, 0, 0);
    }
}

#[cfg(windows)]
fn write_wide_fixed(target: &mut [u16], value: &str) {
    let wide = value.encode_utf16();
    for (slot, chr) in target.iter_mut().zip(wide) {
        *slot = chr;
    }
}

#[component]
fn TodoApp() -> Element {
    let app = use_signal(AppState::load);
    let mut clock = use_signal(|| Local::now().naive_local());
    let initial_language = app().settings.effective_language();
    #[cfg(windows)]
    let tray = use_hook(move || WindowsTray::new(initial_language, current_main_hwnd()));
    use_effect(move || {
        let settings = app().settings;
        #[cfg(windows)]
        {
            tray.set_main_window(current_main_hwnd());
            tray.set_language(settings.effective_language());
            tray.set_visible(settings.tray_enabled);
        }
        apply_startup_setting(settings.startup_enabled);
    });
    use_hook(move || {
        #[cfg(windows)]
        {
            snap_window_to_top_right();
            spawn(async move {
                tokio::time::sleep(StdDuration::from_millis(200)).await;
                snap_window_to_top_right();
            });
        }
        register_system_notifications();
        mutate(app, |state| reschedule_reminders(state, app, clock));
        spawn(async move {
            loop {
                tokio::time::sleep(StdDuration::from_secs(1)).await;
                clock.set(Local::now().naive_local());
            }
        });
    });
    let state = app();
    let now = clock();
    let language = state.settings.effective_language();
    let text = Strings::new(language);
    let theme = state.settings.effective_theme();
    let visible_month = state.visible_month;
    let selected_date = state.selected_date;
    let top_most = state.top_most;

    let groups = grouped_occurrences(&state, state.mode == ViewMode::Completed);
    let calendar_days = calendar_days(visible_month);
    let calendar_occurrences = calendar_occurrences(&state.todos, visible_month);
    let selected_todos = occurrences_between(
        &state.todos,
        selected_date.and_time(NaiveTime::MIN),
        end_of_day(selected_date),
    );

    rsx! {
        div {
            "data-theme": "{theme}",
            class: "min-h-screen bg-base-200 text-base-content",
            div { class: "mx-auto flex h-screen max-w-3xl flex-col overflow-hidden bg-base-100 shadow-xl",
                header { class: "flex shrink-0 items-center justify-between gap-2 border-b border-base-300 px-3 py-2",
                    div {
                        class: "min-w-0 flex-1 cursor-move select-none py-1",
                        onmousedown: move |_| dioxus::desktop::window().drag(),
                        h1 { class: "text-xl font-bold leading-tight", "{text.app_name}" }
                        p { class: "truncate text-xs opacity-65", "{format_title_datetime(now, language)}" }
                    }
                    div { class: "flex items-center gap-2",
                        button {
                            class: "btn btn-primary btn-sm",
                            onclick: move |_| show_editor(app, None),
                            Icon { width: 16, height: 16, icon: LdPlus }
                            "{text.add}"
                        }
                        button {
                            class: "btn btn-ghost btn-square btn-sm",
                            title: "{text.settings}",
                            onclick: move |_| mutate(app, |s| s.dialog = DialogMode::Settings),
                            Icon { width: 16, height: 16, icon: LdSettings }
                        }
                        div { class: "ml-1 flex items-center gap-1 border-l border-base-300 pl-2",
                            button {
                                class: "btn btn-ghost btn-square btn-sm",
                                title: "Minimize",
                                onclick: move |_| dioxus::desktop::window().window.set_minimized(true),
                                Icon { width: 16, height: 16, icon: LdMinus }
                            }
                            button {
                                class: if top_most { "btn btn-primary btn-square btn-sm" } else { "btn btn-ghost btn-square btn-sm" },
                                title: "Always on top",
                                onclick: move |_| toggle_top_most(app),
                                if top_most {
                                    Icon { width: 16, height: 16, icon: LdPinOff }
                                } else {
                                    Icon { width: 16, height: 16, icon: LdPin }
                                }
                            }
                            button {
                                class: "btn btn-ghost btn-square btn-sm text-error",
                                title: "Close",
                                onclick: move |_| request_close(app),
                                Icon { width: 16, height: 16, icon: LdX }
                            }
                        }
                    }
                }

                main { class: "flex min-h-0 flex-1 flex-col p-4",
                    div { role: "tablist", class: "tabs tabs-boxed mb-3 grid grid-cols-3",
                        button {
                            class: tab_class(state.mode == ViewMode::List),
                            onclick: move |_| mutate(app, |s| s.mode = ViewMode::List),
                            Icon { width: 15, height: 15, icon: LdListTodo }
                            "{text.open}"
                        }
                        button {
                            class: tab_class(state.mode == ViewMode::Calendar),
                            onclick: move |_| mutate(app, |s| s.mode = ViewMode::Calendar),
                            Icon { width: 15, height: 15, icon: LdCalendar }
                            "{text.calendar}"
                        }
                        button {
                            class: tab_class(state.mode == ViewMode::Completed),
                            onclick: move |_| mutate(app, |s| s.mode = ViewMode::Completed),
                            Icon { width: 15, height: 15, icon: LdCircleCheck }
                            "{text.completed}"
                        }
                    }

                    if state.mode == ViewMode::Calendar {
                        div { class: "flex min-h-0 flex-1 flex-col gap-3",
                            div { class: "flex items-center justify-between gap-2",
                                button {
                                    class: "btn btn-square btn-sm",
                                    onclick: move |_| mutate(app, |s| s.visible_month = add_months(s.visible_month, -1)),
                                    Icon { width: 16, height: 16, icon: LdChevronLeft }
                                }
                                div { class: "font-semibold", "{month_title(visible_month, language)}" }
                                button {
                                    class: "btn btn-square btn-sm",
                                    onclick: move |_| mutate(app, |s| s.visible_month = add_months(s.visible_month, 1)),
                                    Icon { width: 16, height: 16, icon: LdChevronRight }
                                }
                            }
                            div { class: "grid grid-cols-7 gap-1 text-center text-xs font-semibold opacity-60",
                                for day in weekday_names(language) {
                                    div { "{day}" }
                                }
                            }
                            div { class: "grid grid-cols-7 gap-1",
                                for day in calendar_days {
                                    {
                                        let count = calendar_occurrences.iter().filter(|item| item.due_at.date() == day).count();
                                        let selected = day == selected_date;
                                        let other_month = day.month() != visible_month.month();
                                        let dot_class = if count > 0 { calendar_dot_class(selected) } else { "h-1.5 w-1.5" };
                                        rsx! {
                                            button {
                                                class: calendar_day_class(selected, other_month),
                                                onclick: move |_| mutate(app, |s| {
                                                    s.selected_date = day;
                                                    s.visible_month = first_of_month(day);
                                                }),
                                                span { class: "font-semibold", "{day.day()}" }
                                                span { class: dot_class }
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "min-h-0 flex-1 overflow-y-auto rounded-box border border-base-300",
                                div { class: "flex items-center justify-between bg-base-200 px-3 py-2 font-semibold",
                                    span { "{format_date(selected_date, language)}" }
                                    span { class: "badge badge-ghost", "{selected_todos.len()}" }
                                }
                                if selected_todos.is_empty() {
                                    Empty { label: text.no_todos.clone() }
                                } else {
                                    for occurrence in selected_todos {
                                            TodoRow {
                                                key: "{occurrence_dom_key(&occurrence)}",
                                                occurrence,
                                                now,
                                                text: text.clone(),
                                                on_toggle: move |(occurrence, done)| toggle_done(app, clock, occurrence, done),
                                            on_edit: move |todo: TodoItem| show_editor(app, Some(todo)),
                                            on_delete: move |id| request_delete(app, id),
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        div { class: "flex min-h-0 flex-1 flex-col",
                            div { class: "mb-3 grid grid-cols-[1fr_auto_auto] gap-2",
                                input {
                                    class: "input input-bordered input-sm w-full",
                                    placeholder: "{text.search}",
                                    value: "{state.query}",
                                    oninput: move |e| mutate(app, |s| s.query = e.value()),
                                }
                                button {
                                    class: "btn btn-square btn-sm",
                                    title: "{text.collapse_all}",
                                    onclick: move |_| collapse_all(app),
                                    Icon { width: 16, height: 16, icon: LdChevronsUp }
                                }
                                button {
                                    class: "btn btn-square btn-sm",
                                    title: "{text.expand_all}",
                                    onclick: move |_| mutate(app, |s| s.collapsed_days.clear()),
                                    Icon { width: 16, height: 16, icon: LdChevronsDown }
                                }
                            }
                            div { class: "min-h-0 flex-1 overflow-y-auto rounded-box border border-base-300",
                                if groups.is_empty() {
                                    Empty { label: if state.todos.is_empty() { text.no_todos.clone() } else { text.no_results.clone() } }
                                } else {
                                    for (day, items) in groups {
                                        {
                                            let collapsed = state.collapsed_days.contains(&day);
                                            rsx! {
                                                section { class: "border-b border-base-300 last:border-b-0",
                                                    button {
                                                        class: "flex w-full items-center justify-between bg-base-200 px-3 py-2 text-left",
                                                        onclick: move |_| toggle_day(app, day),
                                                        span { class: "flex items-center gap-2 font-semibold",
                                                            if collapsed {
                                                                Icon { width: 14, height: 14, icon: LdChevronRight }
                                                            } else {
                                                                Icon { width: 14, height: 14, icon: LdChevronDown }
                                                            }
                                                            "{format_group_date(day, language)}"
                                                        }
                                                        span { class: "badge badge-ghost", "{items.len()}" }
                                                    }
                                                    if !collapsed {
                                                        for occurrence in items {
                                                            TodoRow {
                                                                key: "{occurrence_dom_key(&occurrence)}",
                                                                occurrence,
                                                                now,
                                                                text: text.clone(),
                                                                on_toggle: move |(occurrence, done)| toggle_done(app, clock, occurrence, done),
                                                                on_edit: move |todo: TodoItem| show_editor(app, Some(todo)),
                                                                on_delete: move |id| request_delete(app, id),
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            match state.dialog {
                DialogMode::Todo => rsx! { TodoDialog { app, clock, state: state.clone(), text: text.clone() } },
                DialogMode::Settings => rsx! { SettingsDialog { app, state: state.clone(), text: text.clone() } },
                DialogMode::DeleteConfirm => rsx! { DeleteDialog { app, clock, text: text.clone() } },
                DialogMode::CloseConfirm => rsx! { CloseDialog { app, state: state.clone(), text: text.clone() } },
                DialogMode::None => rsx! {},
            }
        }
    }
}

#[component]
fn TodoRow(
    occurrence: TodoOccurrence,
    now: NaiveDateTime,
    text: Strings,
    on_toggle: EventHandler<(TodoOccurrence, bool)>,
    on_edit: EventHandler<TodoItem>,
    on_delete: EventHandler<Uuid>,
) -> Element {
    let done = occurrence.is_done;
    let overdue = is_overdue(&occurrence, now);
    let todo = occurrence.source.clone();
    let edit_todo = todo.clone();
    let delete_id = todo.id;
    let mut notes_expanded = use_signal(|| false);

    rsx! {
        div { key: "{occurrence_dom_key(&occurrence)}", class: "todo-row border-t border-base-300 px-3 py-3 first:border-t-0",
            input {
                r#type: "checkbox",
                class: "checkbox checkbox-primary todo-row-checkbox",
                checked: done,
                onchange: move |e| on_toggle.call((occurrence.clone(), e.checked())),
            }
            div { class: if done { "todo-row-title break-words font-semibold line-through opacity-50" } else { "todo-row-title break-words font-semibold" }, "{todo.title}" }
            button {
                class: "btn btn-ghost btn-square btn-sm",
                title: "{text.edit_todo}",
                onclick: move |_| on_edit.call(edit_todo.clone()),
                Icon { width: 15, height: 15, icon: LdPencil }
            }
            button {
                class: "btn btn-ghost btn-square btn-sm text-error",
                title: "{text.delete}",
                onclick: move |_| on_delete.call(delete_id),
                Icon { width: 15, height: 15, icon: LdTrash2 }
            }
            div { class: "todo-row-meta flex flex-wrap items-center gap-2 text-xs opacity-60",
                span { class: "inline-flex items-center gap-1",
                    Icon { width: 12, height: 12, icon: LdClock }
                    if has_unspecified_time(occurrence.due_at) {
                        "{text.unspecified_time}"
                    } else {
                        "{todo_time(occurrence.due_at)}"
                    }
                }
                if todo.is_recurring() {
                    span { class: "badge badge-outline badge-xs gap-1",
                        Icon { width: 10, height: 10, icon: LdRepeat }
                        "{text.repeat_badge}"
                    }
                }
                if !todo.reminder_minutes.is_empty() {
                    span { class: "badge badge-ghost badge-xs gap-1",
                        Icon { width: 10, height: 10, icon: LdAlarmClock }
                        "{todo.reminder_minutes.len()} {text.reminders}"
                    }
                }
                if overdue {
                    span { class: "badge badge-warning badge-xs", "{text.overdue}" }
                }
            }
            if !todo.notes.trim().is_empty() {
                div { class: "todo-row-notes-line",
                    button {
                        class: "btn btn-ghost btn-square btn-sm todo-row-notes-toggle",
                        title: if notes_expanded() { "{text.collapse_all}" } else { "{text.expand_all}" },
                        onclick: move |_| notes_expanded.set(!notes_expanded()),
                        if notes_expanded() {
                            Icon { width: 15, height: 15, icon: LdChevronDown }
                        } else {
                            Icon { width: 15, height: 15, icon: LdChevronRight }
                        }
                    }
                    p {
                        class: "todo-row-notes-text {todo_notes_class(notes_expanded())}",
                        onclick: move |_| notes_expanded.set(!notes_expanded()),
                        "{todo.notes}"
                    }
                }
            }
        }
    }
}

#[component]
fn TodoDialog(
    app: Signal<AppState>,
    clock: Signal<NaiveDateTime>,
    state: AppState,
    text: Strings,
) -> Element {
    let editor = state.editor.clone();
    let (due_hour, due_minute) = time_select_parts(&editor.due_time);

    rsx! {
        div { class: "modal modal-open",
            div { class: "modal-box max-h-[92vh] overflow-y-auto",
                h2 { class: "mb-3 flex items-center gap-2 text-lg font-bold",
                    if editor.editing_id.is_some() {
                        Icon { width: 18, height: 18, icon: LdPencil }
                    } else {
                        Icon { width: 18, height: 18, icon: LdPlus }
                    }
                    if editor.editing_id.is_some() { "{text.edit_todo}" } else { "{text.new_todo}" }
                }
                if !editor.validation.is_empty() {
                    div { class: "alert alert-error mb-3 py-2", "{editor.validation}" }
                }

                label { class: "form-control",
                    div { class: "label py-1", span { class: "label-text", "{text.title}" } }
                    input {
                        class: "input input-bordered w-full",
                        value: "{editor.title}",
                        oninput: move |e| mutate(app, |s| s.editor.title = e.value()),
                    }
                }

                label { class: "form-control mt-2",
                    div { class: "label py-1", span { class: "label-text", "{text.notes}" } }
                    textarea {
                        class: "textarea textarea-bordered min-h-24 w-full",
                        value: "{editor.notes}",
                        oninput: move |e| mutate(app, |s| s.editor.notes = e.value()),
                    }
                }

                div { class: "mt-3 space-y-2",
                    label { class: "form-control",
                        div { class: "label py-1",
                            span { class: "label-text flex items-center gap-2",
                                "{text.date}"
                                input {
                                    r#type: "checkbox",
                                    class: "toggle toggle-primary toggle-sm",
                                    checked: editor.due_date_enabled,
                                    onchange: move |e| mutate(app, |s| set_editor_date_enabled(&mut s.editor, e.checked())),
                                }
                            }
                        }
                        if editor.due_date_enabled {
                            input {
                                r#type: "date",
                                class: "input input-bordered w-full",
                                value: "{editor.due_date}",
                                oninput: move |e| mutate(app, |s| {
                                    s.editor.due_date = e.value();
                                    sync_editor_repeat_defaults(&mut s.editor);
                                }),
                            }
                        }
                    }

                    if editor.due_date_enabled {
                        if editor.due_time_enabled {
                            label { class: "form-control",
                                div { class: "label py-1",
                                    span { class: "label-text flex items-center gap-2",
                                        "{text.time}"
                                        input {
                                            r#type: "checkbox",
                                            class: "toggle toggle-primary toggle-sm",
                                            checked: editor.due_time_enabled,
                                            onchange: move |e| mutate(app, |s| set_editor_time_enabled(&mut s.editor, e.checked())),
                                        }
                                    }
                                }
                                div { class: "grid grid-cols-2 gap-2",
                                    select {
                                        class: "select select-bordered w-full",
                                        value: "{due_hour}",
                                        onchange: move |e| mutate(app, |s| set_editor_time_hour(&mut s.editor, &e.value())),
                                        for hour in 0..24 {
                                            {
                                                let value = two_digits(hour);
                                                rsx! {
                                                    option { value: "{value}", "{value}" }
                                                }
                                            }
                                        }
                                    }
                                    select {
                                        class: "select select-bordered w-full",
                                        value: "{due_minute}",
                                        onchange: move |e| mutate(app, |s| set_editor_time_minute(&mut s.editor, &e.value())),
                                        for minute in (0..60).step_by(5) {
                                            {
                                                let value = two_digits(minute);
                                                rsx! {
                                                    option { value: "{value}", "{value}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            label { class: "form-control",
                                div { class: "label py-1",
                                    span { class: "label-text flex items-center gap-2",
                                        "{text.time}"
                                        input {
                                            r#type: "checkbox",
                                            class: "toggle toggle-primary toggle-sm",
                                            checked: editor.due_time_enabled,
                                            onchange: move |e| mutate(app, |s| set_editor_time_enabled(&mut s.editor, e.checked())),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if editor.due_date_enabled {
                    div { class: "mt-3 text-sm font-semibold flex items-center gap-2",
                        "{text.repeat}"
                        input {
                            r#type: "checkbox",
                            class: "toggle toggle-primary toggle-sm",
                            checked: editor.recurring,
                            onchange: move |e| mutate(app, |s| {
                                s.editor.recurring = e.checked();
                                sync_editor_repeat_defaults(&mut s.editor);
                            }),
                        }
                    }
                }

                if editor.due_date_enabled && editor.recurring {
                    div { class: "mt-3 rounded-box border border-base-300 bg-base-200 p-3",
                        label { class: "form-control",
                            div { class: "label py-1", span { class: "label-text", "{text.repeat_type}" } }
                            select {
                                class: "select select-bordered w-full",
                                value: editor.recurrence_kind.as_str(),
                                onchange: move |e| mutate(app, |s| {
                                    s.editor.recurrence_kind = RecurrenceKind::from_value(&e.value());
                                    sync_editor_repeat_defaults(&mut s.editor);
                                }),
                                option { value: "weekly", "{text.weekly}" }
                                option { value: "monthly", "{text.monthly}" }
                            }
                        }

                        if editor.recurrence_kind == RecurrenceKind::Weekly {
                            div { class: "mt-2 grid grid-cols-7 gap-1",
                                for (index, name) in weekday_names(state.settings.effective_language()).into_iter().enumerate() {
                                    {
                                        let selected = editor.weekdays.contains(&(index as u32));
                                        rsx! {
                                            button {
                                                class: if selected { "btn btn-primary btn-xs" } else { "btn btn-outline btn-xs" },
                                                onclick: move |_| mutate(app, |s| set_weekday(&mut s.editor, index as u32)),
                                                "{name}"
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            label { class: "form-control mt-2",
                                div { class: "label py-1", span { class: "label-text", "{text.monthly_repeat}" } }
                                select {
                                    class: "select select-bordered w-full",
                                    value: editor.monthly_kind.as_str(),
                                    onchange: move |e| mutate(app, |s| s.editor.monthly_kind = MonthlyKind::from_value(&e.value())),
                                    option { value: "day", "{text.day_of_month}" }
                                    option { value: "last_workday", "{text.last_workday}" }
                                    option { value: "last_day", "{text.last_day}" }
                                }
                            }
                            if editor.monthly_kind == MonthlyKind::DayOfMonth {
                                label { class: "form-control mt-2",
                                    div { class: "label py-1", span { class: "label-text", "{text.day_number}" } }
                                    input {
                                        r#type: "number",
                                        min: "1",
                                        max: "31",
                                        class: "input input-bordered w-full",
                                        value: "{editor.day_of_month}",
                                        oninput: move |e| mutate(app, |s| s.editor.day_of_month = e.value()),
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "mt-3",
                    if editor.due_date_enabled && editor.due_time_enabled {
                        div { class: "mb-2 text-sm font-semibold flex items-center gap-2",
                            "{text.reminders}"
                            input {
                                r#type: "checkbox",
                                class: "toggle toggle-primary toggle-sm",
                                checked: editor.reminders_enabled,
                                onchange: move |e| mutate(app, |s| {
                                    s.editor.reminders_enabled = e.checked();
                                    if !s.editor.reminders_enabled {
                                        s.editor.reminders.clear();
                                    } else if s.editor.reminders.is_empty() {
                                        s.editor.reminders = s.settings.default_reminder_minutes.clone();
                                    }
                                }),
                            }
                        }
                    }
                    if editor.due_date_enabled && editor.due_time_enabled && editor.reminders_enabled {
                        div { class: "space-y-2",
                            for minutes in editor.reminders.clone() {
                                div { class: "flex items-center justify-between rounded-box bg-base-200 px-3 py-2",
                                    span { "{minutes} {text.minutes_before}" }
                                    button {
                                        class: "btn btn-ghost btn-square btn-xs text-error",
                                        onclick: move |_| mutate(app, |s| s.editor.reminders.retain(|value| *value != minutes)),
                                        Icon { width: 13, height: 13, icon: LdX }
                                    }
                                }
                            }
                        }
                        div { class: "mt-2 grid grid-cols-[6rem_1fr] gap-2",
                            input {
                                r#type: "number",
                                min: "0",
                                max: "10080",
                                class: "input input-bordered w-full",
                                placeholder: "15",
                                value: "{editor.new_reminder}",
                                oninput: move |e| mutate(app, |s| s.editor.new_reminder = e.value()),
                            }
                            button {
                                class: "btn",
                                onclick: move |_| mutate(app, |s| add_editor_reminder(&mut s.editor)),
                                Icon { width: 15, height: 15, icon: LdPlus }
                                "{text.add_reminder}"
                            }
                        }
                    }
                }

                div { class: "modal-action",
                    button { class: "btn", onclick: move |_| close_dialog(app),
                        Icon { width: 15, height: 15, icon: LdX }
                        "{text.cancel}"
                    }
                    if let Some(id) = editor.editing_id {
                        button { class: "btn btn-error", onclick: move |_| request_delete(app, id),
                            Icon { width: 15, height: 15, icon: LdTrash2 }
                            "{text.delete}"
                        }
                    }
                    button { class: "btn btn-primary", onclick: move |_| save_todo(app, clock),
                        Icon { width: 15, height: 15, icon: LdSave }
                        "{text.save}"
                    }
                }
            }
            div { class: "modal-backdrop", onclick: move |_| close_dialog(app) }
        }
    }
}

#[component]
fn SettingsDialog(app: Signal<AppState>, state: AppState, text: Strings) -> Element {
    let settings = state.settings.clone();

    rsx! {
        div { class: "modal modal-open",
            div { class: "modal-box max-h-[92vh] overflow-y-auto",
                div { class: "mb-3 flex items-center justify-between gap-3",
                    h2 { class: "flex items-center gap-2 text-lg font-bold",
                        Icon { width: 18, height: 18, icon: LdSettings }
                        "{text.settings}"
                    }
                    button {
                        class: "link link-hover text-xs opacity-70",
                        title: PROJECT_URL,
                        onclick: move |_| open_project_homepage(),
                        "{APP_VERSION}"
                    }
                }

                div { class: "divide-y divide-base-300",
                    div { class: "p-3",
                        label { class: "form-control",
                            div { class: "label py-1", span { class: "label-text inline-flex items-center gap-1",
                                Icon { width: 13, height: 13, icon: LdLanguages }
                                "{text.language}"
                            } }
                            select {
                                class: "select select-bordered w-full",
                                value: "{settings.language}",
                                onchange: move |e| mutate(app, |s| {
                                    s.settings.language = e.value();
                                    s.save();
                                }),
                                option { value: "system", "{text.system}" }
                                option { value: "en", "{text.english}" }
                                option { value: "zh", "{text.chinese}" }
                            }
                        }
                    }

                    div { class: "p-3",
                        label { class: "form-control",
                            div { class: "label py-1", span { class: "label-text inline-flex items-center gap-1",
                                Icon { width: 13, height: 13, icon: LdPalette }
                                "{text.theme}"
                            } }
                            select {
                                class: "select select-bordered w-full",
                                value: "{settings.theme}",
                                onchange: move |e| mutate(app, |s| {
                                    s.settings.theme = e.value();
                                    s.save();
                                }),
                                option { value: "system", "{text.system}" }
                                for theme in THEMES {
                                    option { value: "{theme}", "{theme}" }
                                }
                            }
                        }
                    }

                    div { class: "p-3",
                        label { class: "form-control",
                            div { class: "label py-1", span { class: "label-text", "{text.close_behavior}" } }
                            select {
                                class: "select select-bordered w-full",
                                value: "{settings.close_behavior}",
                                onchange: move |e| mutate(app, |s| {
                                    s.settings.close_behavior = e.value();
                                    s.save();
                                }),
                                option { value: "prompt", "{text.ask_on_close}" }
                                option { value: "tray", "{text.minimize_to_tray}" }
                                option { value: "exit", "{text.exit_app}" }
                            }
                        }
                    }

                    div { class: "p-3",
                        label { class: "flex cursor-pointer items-center justify-between gap-3",
                            span { class: "font-semibold", "{text.tray_enabled}" }
                            input {
                                r#type: "checkbox",
                                class: "toggle toggle-primary",
                                checked: settings.tray_enabled,
                                onchange: move |e| mutate(app, |s| {
                                    s.settings.tray_enabled = e.checked();
                                    s.save();
                                }),
                            }
                        }
                    }

                    div { class: "p-3",
                        label { class: "flex cursor-pointer items-center justify-between gap-3",
                            span { class: "font-semibold", "{text.startup_enabled}" }
                            input {
                                r#type: "checkbox",
                                class: "toggle toggle-primary",
                                checked: settings.startup_enabled,
                                onchange: move |e| mutate(app, |s| {
                                    s.settings.startup_enabled = e.checked();
                                    s.save();
                                }),
                            }
                        }
                    }

                    div { class: "p-3",
                        div { class: "flex items-center gap-2 font-semibold",
                            Icon { width: 15, height: 15, icon: LdAlarmClock }
                            "{text.default_reminders}"
                        }
                        div { class: "mt-2 divide-y divide-base-300 overflow-hidden rounded-box border border-base-300 bg-base-100",
                            for minutes in settings.default_reminder_minutes.clone() {
                                div { class: "flex items-center justify-between px-3 py-2",
                                    span { "{minutes} {text.minutes_before}" }
                                    button {
                                        class: "btn btn-ghost btn-square btn-xs text-error",
                                        onclick: move |_| mutate(app, |s| {
                                            s.settings.default_reminder_minutes.retain(|value| *value != minutes);
                                            if s.settings.default_reminder_minutes.is_empty() {
                                                s.settings.default_reminder_minutes = vec![15, 5];
                                            }
                                            s.save();
                                        }),
                                        Icon { width: 13, height: 13, icon: LdX }
                                    }
                                }
                            }
                        }
                        div { class: "mt-2 grid grid-cols-[6rem_1fr] gap-2",
                            input {
                                r#type: "number",
                                min: "0",
                                max: "10080",
                                class: "input input-bordered w-full",
                                placeholder: "15",
                                value: "{state.new_default_reminder}",
                                oninput: move |e| mutate(app, |s| s.new_default_reminder = e.value()),
                            }
                            button {
                                class: "btn",
                                onclick: move |_| add_default_reminder(app),
                                Icon { width: 15, height: 15, icon: LdPlus }
                                "{text.add_reminder}"
                            }
                        }
                    }
                }

                div { class: "modal-action",
                    button { class: "btn btn-primary", onclick: move |_| close_dialog(app),
                        Icon { width: 15, height: 15, icon: LdCheck }
                        "{text.done}"
                    }
                }
            }
            div { class: "modal-backdrop", onclick: move |_| close_dialog(app) }
        }
    }
}

#[component]
fn CloseDialog(app: Signal<AppState>, state: AppState, text: Strings) -> Element {
    let tray_enabled = state.settings.tray_enabled;

    rsx! {
        div { class: "modal modal-open",
            div { class: "modal-box",
                h2 { class: "text-lg font-bold", "{text.close_app}" }
                p { class: "mt-2 text-sm opacity-70", "{text.close_confirm}" }
                div { class: "modal-action",
                    button { class: "btn", onclick: move |_| close_dialog(app), "{text.cancel}" }
                    if tray_enabled {
                        button { class: "btn btn-secondary", onclick: move |_| {
                            close_dialog(app);
                            hide_main_window();
                        }, "{text.minimize_to_tray}" }
                    }
                    button { class: "btn btn-error", onclick: move |_| dioxus::desktop::window().close(), "{text.exit_app}" }
                }
            }
            div { class: "modal-backdrop", onclick: move |_| close_dialog(app) }
        }
    }
}

#[component]
fn DeleteDialog(app: Signal<AppState>, clock: Signal<NaiveDateTime>, text: Strings) -> Element {
    rsx! {
        div { class: "modal modal-open",
            div { class: "modal-box",
                h2 { class: "flex items-center gap-2 text-lg font-bold",
                    Icon { width: 18, height: 18, icon: LdTrash2 }
                    "{text.delete}"
                }
                p { class: "py-4", "{text.delete_confirm}" }
                div { class: "modal-action",
                    button { class: "btn", onclick: move |_| close_dialog(app),
                        Icon { width: 15, height: 15, icon: LdX }
                        "{text.cancel}"
                    }
                    button { class: "btn btn-error", onclick: move |_| confirm_delete(app, clock),
                        Icon { width: 15, height: 15, icon: LdTrash2 }
                        "{text.delete}"
                    }
                }
            }
            div { class: "modal-backdrop", onclick: move |_| close_dialog(app) }
        }
    }
}

#[component]
fn Empty(label: String) -> Element {
    rsx! {
        div { class: "grid min-h-40 place-items-center gap-2 p-6 text-sm opacity-60",
            Icon { width: 28, height: 28, icon: LdListTodo }
            span { "{label}" }
        }
    }
}

#[derive(Clone, PartialEq)]
struct AppState {
    todos: Vec<TodoItem>,
    settings: Settings,
    mode: ViewMode,
    query: String,
    visible_month: NaiveDate,
    selected_date: NaiveDate,
    collapsed_days: Vec<NaiveDate>,
    pending_completed_occurrences: Vec<TodoOccurrence>,
    top_most: bool,
    dialog: DialogMode,
    editor: TodoEditor,
    pending_delete_id: Option<Uuid>,
    new_default_reminder: String,
    reminder_generation: u64,
    delivered_reminder_ids: Vec<String>,
}

impl AppState {
    fn load() -> Self {
        let today = Local::now().date_naive();
        let store = load_store();

        Self {
            todos: store.todos,
            settings: store.settings,
            mode: ViewMode::List,
            query: String::new(),
            visible_month: first_of_month(today),
            selected_date: today,
            collapsed_days: Vec::new(),
            pending_completed_occurrences: Vec::new(),
            top_most: false,
            dialog: DialogMode::None,
            editor: TodoEditor::new(None, &[15, 5]),
            pending_delete_id: None,
            new_default_reminder: String::new(),
            reminder_generation: 0,
            delivered_reminder_ids: Vec::new(),
        }
    }

    fn save(&self) {
        save_store(&Store {
            todos: self.todos.clone(),
            settings: self.settings.clone(),
        });
        apply_startup_setting(self.settings.startup_enabled);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    List,
    Calendar,
    Completed,
}

#[derive(Clone, Copy, PartialEq)]
enum DialogMode {
    None,
    Todo,
    Settings,
    DeleteConfirm,
    CloseConfirm,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Store {
    #[serde(default)]
    todos: Vec<TodoItem>,
    #[serde(default)]
    settings: Settings,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            todos: Vec::new(),
            settings: Settings::default(),
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Settings {
    #[serde(default = "default_language")]
    language: String,
    #[serde(default = "default_theme")]
    theme: String,
    #[serde(default = "default_close_behavior")]
    close_behavior: String,
    #[serde(default = "default_tray_enabled")]
    tray_enabled: bool,
    #[serde(default = "default_startup_enabled")]
    startup_enabled: bool,
    #[serde(default = "default_reminders")]
    default_reminder_minutes: Vec<i32>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: default_language(),
            theme: default_theme(),
            close_behavior: default_close_behavior(),
            tray_enabled: default_tray_enabled(),
            startup_enabled: default_startup_enabled(),
            default_reminder_minutes: default_reminders(),
        }
    }
}

impl Settings {
    fn effective_language(&self) -> Language {
        match self.language.as_str() {
            "zh" => Language::Zh,
            "en" => Language::En,
            _ => system_language()
                .or_else(|| env_locale().map(|locale| language_from_locale(&locale)))
                .unwrap_or(Language::En),
        }
    }

    fn effective_theme(&self) -> String {
        if self.theme == "system" {
            system_theme().unwrap_or("light").into()
        } else {
            self.theme.clone()
        }
    }
}

fn language_from_locale(locale: &str) -> Language {
    if locale.trim().to_ascii_lowercase().starts_with("zh") {
        Language::Zh
    } else {
        Language::En
    }
}

#[cfg(windows)]
fn system_language() -> Option<Language> {
    use windows_sys::Win32::Globalization::{GetUserDefaultLocaleName, GetUserDefaultUILanguage};

    let ui_language = unsafe { GetUserDefaultUILanguage() };
    if ui_language != 0 {
        return Some(if ui_language & 0x03ff == 0x04 {
            Language::Zh
        } else {
            Language::En
        });
    }

    let mut buffer = [0u16; 85];
    let len = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
    if len <= 1 {
        return None;
    }

    Some(language_from_locale(&String::from_utf16_lossy(
        &buffer[..len as usize - 1],
    )))
}

#[cfg(not(windows))]
fn system_language() -> Option<Language> {
    None
}

fn env_locale() -> Option<String> {
    ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .find_map(|key| {
            std::env::var(key)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct TodoItem {
    id: Uuid,
    title: String,
    due_at: NaiveDateTime,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    is_done: bool,
    completed_at: Option<NaiveDateTime>,
    #[serde(default = "default_reminders")]
    reminder_minutes: Vec<i32>,
    recurrence: Option<RecurrenceRule>,
    #[serde(default)]
    completions: Vec<TodoCompletion>,
}

impl TodoItem {
    fn is_recurring(&self) -> bool {
        matches!(
            self.recurrence.as_ref().map(|rule| rule.kind),
            Some(RecurrenceKind::Weekly | RecurrenceKind::Monthly)
        )
    }

    fn normalized(mut self) -> Self {
        self.title = self.title.trim().to_string();
        self.notes = self.notes.trim().to_string();
        self.reminder_minutes = normalize_reminders(&self.reminder_minutes);
        if has_unspecified_time(self.due_at) {
            self.reminder_minutes.clear();
        }
        if is_unscheduled_due(self.due_at) {
            self.recurrence = None;
        }
        if let Some(rule) = &mut self.recurrence {
            rule.day_of_month = rule.day_of_month.clamp(1, 31);
            rule.weekdays.sort_unstable();
            rule.weekdays.dedup();
            if rule.kind == RecurrenceKind::Weekly && rule.weekdays.is_empty() {
                rule.weekdays.push(weekday_index(self.due_at.weekday()));
            }
        }
        self
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct RecurrenceRule {
    kind: RecurrenceKind,
    #[serde(default)]
    weekdays: Vec<u32>,
    #[serde(default = "default_day_of_month")]
    day_of_month: i32,
    #[serde(default)]
    monthly_kind: MonthlyKind,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RecurrenceKind {
    Weekly,
    Monthly,
}

impl RecurrenceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
        }
    }

    fn from_value(value: &str) -> Self {
        if value == "monthly" {
            Self::Monthly
        } else {
            Self::Weekly
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MonthlyKind {
    DayOfMonth,
    LastWorkday,
    LastDay,
}

impl Default for MonthlyKind {
    fn default() -> Self {
        Self::DayOfMonth
    }
}

impl MonthlyKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::DayOfMonth => "day",
            Self::LastWorkday => "last_workday",
            Self::LastDay => "last_day",
        }
    }

    fn from_value(value: &str) -> Self {
        match value {
            "last_workday" => Self::LastWorkday,
            "last_day" => Self::LastDay,
            _ => Self::DayOfMonth,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct TodoCompletion {
    occurrence_key: String,
    due_at: NaiveDateTime,
    completed_at: NaiveDateTime,
}

#[derive(Clone, PartialEq)]
struct TodoOccurrence {
    source: TodoItem,
    due_at: NaiveDateTime,
    occurrence_key: String,
    is_done: bool,
    completed_at: Option<NaiveDateTime>,
}

#[derive(Clone, PartialEq)]
struct TodoEditor {
    editing_id: Option<Uuid>,
    title: String,
    due_date_enabled: bool,
    due_date: String,
    due_time_enabled: bool,
    due_time: String,
    reminders_enabled: bool,
    notes: String,
    was_done: bool,
    completed_at: Option<NaiveDateTime>,
    reminders: Vec<i32>,
    recurring: bool,
    recurrence_kind: RecurrenceKind,
    weekdays: Vec<u32>,
    monthly_kind: MonthlyKind,
    day_of_month: String,
    new_reminder: String,
    validation: String,
}

impl TodoEditor {
    fn new(todo: Option<&TodoItem>, default_reminders: &[i32]) -> Self {
        let due_at = todo.map(|item| item.due_at).unwrap_or_else(default_due_at);
        let date_enabled = !is_unscheduled_due(due_at);
        let time_enabled = date_enabled && !has_unspecified_time(due_at);
        let recurrence = todo.and_then(|item| item.recurrence.clone());
        let recurrence_kind = recurrence
            .as_ref()
            .map(|rule| rule.kind)
            .unwrap_or(RecurrenceKind::Weekly);
        let weekdays = recurrence
            .as_ref()
            .map(|rule| rule.weekdays.clone())
            .filter(|days| !days.is_empty())
            .unwrap_or_else(|| vec![weekday_index(due_at.weekday())]);
        let day_of_month = recurrence
            .as_ref()
            .map(|rule| rule.day_of_month)
            .unwrap_or(due_at.day() as i32)
            .clamp(1, 31);

        Self {
            editing_id: todo.map(|item| item.id),
            title: todo.map(|item| item.title.clone()).unwrap_or_default(),
            due_date_enabled: todo.map(|_| date_enabled).unwrap_or(true),
            due_date: if date_enabled {
                due_at.date().format("%Y-%m-%d").to_string()
            } else {
                String::new()
            },
            due_time_enabled: todo.map(|_| time_enabled).unwrap_or(true),
            due_time: if time_enabled {
                due_at.time().format("%H:%M").to_string()
            } else {
                String::new()
            },
            reminders_enabled: todo
                .map(|item| time_enabled && !item.reminder_minutes.is_empty())
                .unwrap_or(true),
            notes: todo.map(|item| item.notes.clone()).unwrap_or_default(),
            was_done: todo.map(|item| item.is_done).unwrap_or_default(),
            completed_at: todo.and_then(|item| item.completed_at),
            reminders: if time_enabled || todo.is_none() {
                todo.map(|item| item.reminder_minutes.clone())
                    .unwrap_or_else(|| default_reminders.to_vec())
            } else {
                Vec::new()
            },
            recurring: recurrence.is_some(),
            recurrence_kind,
            weekdays,
            monthly_kind: recurrence
                .as_ref()
                .map(|rule| rule.monthly_kind)
                .unwrap_or_default(),
            day_of_month: day_of_month.to_string(),
            new_reminder: String::new(),
            validation: String::new(),
        }
    }

    fn due_at(&self) -> Option<NaiveDateTime> {
        if !self.due_date_enabled {
            return Some(unscheduled_due_at());
        }
        let date = NaiveDate::parse_from_str(&self.due_date, "%Y-%m-%d").ok()?;
        let time = if self.due_time_enabled {
            snap_time_to_five_minutes(NaiveTime::parse_from_str(&self.due_time, "%H:%M").ok()?)
        } else {
            NaiveTime::MIN
        };
        Some(date.and_time(time))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Language {
    En,
    Zh,
}

#[derive(Clone, PartialEq)]
struct Strings {
    app_name: String,
    add: String,
    save: String,
    cancel: String,
    delete: String,
    done: String,
    settings: String,
    open: String,
    calendar: String,
    completed: String,
    search: String,
    collapse_all: String,
    expand_all: String,
    no_todos: String,
    no_results: String,
    unspecified_time: String,
    new_todo: String,
    edit_todo: String,
    title: String,
    date: String,
    time: String,
    notes: String,
    repeat: String,
    repeat_type: String,
    weekly: String,
    monthly: String,
    monthly_repeat: String,
    day_of_month: String,
    last_workday: String,
    last_day: String,
    day_number: String,
    reminders: String,
    overdue: String,
    minutes_before: String,
    add_reminder: String,
    default_reminders: String,
    language: String,
    system: String,
    english: String,
    chinese: String,
    theme: String,
    close_behavior: String,
    ask_on_close: String,
    minimize_to_tray: String,
    exit_app: String,
    tray_enabled: String,
    startup_enabled: String,
    close_app: String,
    close_confirm: String,
    delete_confirm: String,
    title_required: String,
    invalid_time: String,
    repeat_badge: String,
}

impl Strings {
    fn new(language: Language) -> Self {
        match language {
            Language::Zh => Self {
                app_name: "So Todo".into(),
                add: "\u{6dfb}\u{52a0}".into(),
                save: "\u{4fdd}\u{5b58}".into(),
                cancel: "\u{53d6}\u{6d88}".into(),
                delete: "\u{5220}\u{9664}".into(),
                done: "\u{5b8c}\u{6210}".into(),
                settings: "\u{8bbe}\u{7f6e}".into(),
                open: "\u{672a}\u{5b8c}\u{6210}".into(),
                calendar: "\u{65e5}\u{5386}".into(),
                completed: "\u{5df2}\u{5b8c}\u{6210}".into(),
                search: "\u{641c}\u{7d22}\u{6807}\u{9898}\u{6216}\u{5907}\u{6ce8}".into(),
                collapse_all: "\u{5168}\u{90e8}\u{6298}\u{53e0}".into(),
                expand_all: "\u{5168}\u{90e8}\u{5c55}\u{5f00}".into(),
                no_todos: "\u{6682}\u{65e0}\u{5f85}\u{529e}".into(),
                no_results: "\u{6ca1}\u{6709}\u{5339}\u{914d}\u{7684}\u{5f85}\u{529e}".into(),
                unspecified_time: "\u{672a}\u{6307}\u{5b9a}\u{65f6}\u{95f4}".into(),
                new_todo: "\u{65b0}\u{5efa}\u{5f85}\u{529e}".into(),
                edit_todo: "\u{7f16}\u{8f91}\u{5f85}\u{529e}".into(),
                title: "\u{6807}\u{9898}".into(),
                date: "\u{65e5}\u{671f}".into(),
                time: "\u{65f6}\u{95f4}".into(),
                notes: "\u{5907}\u{6ce8}".into(),
                repeat: "\u{91cd}\u{590d}".into(),
                repeat_type: "\u{91cd}\u{590d}\u{65b9}\u{5f0f}".into(),
                weekly: "\u{6309}\u{5468}".into(),
                monthly: "\u{6309}\u{6708}".into(),
                monthly_repeat: "\u{6309}\u{6708}\u{89c4}\u{5219}".into(),
                day_of_month: "\u{6bcf}\u{6708}\u{51e0}\u{53f7}".into(),
                last_workday: "\u{6700}\u{540e}\u{4e00}\u{4e2a}\u{5de5}\u{4f5c}\u{65e5}".into(),
                last_day: "\u{6700}\u{540e}\u{4e00}\u{5929}".into(),
                day_number: "\u{65e5}\u{671f}".into(),
                reminders: "\u{63d0}\u{9192}".into(),
                overdue: "\u{5df2}\u{8d85}\u{671f}".into(),
                minutes_before: "\u{5206}\u{949f}\u{524d}".into(),
                add_reminder: "\u{6dfb}\u{52a0}\u{63d0}\u{9192}".into(),
                default_reminders: "\u{9ed8}\u{8ba4}\u{63d0}\u{9192}".into(),
                language: "\u{8bed}\u{8a00}".into(),
                system: "\u{8ddf}\u{968f}\u{7cfb}\u{7edf}".into(),
                english: "English".into(),
                chinese: "\u{4e2d}\u{6587}".into(),
                theme: "\u{4e3b}\u{9898}".into(),
                close_behavior: "\u{5173}\u{95ed}\u{6309}\u{94ae}".into(),
                ask_on_close: "\u{6bcf}\u{6b21}\u{8be2}\u{95ee}".into(),
                minimize_to_tray: "\u{6700}\u{5c0f}\u{5316}\u{5230}\u{6258}\u{76d8}".into(),
                exit_app: "\u{9000}\u{51fa}\u{7a0b}\u{5e8f}".into(),
                tray_enabled: "\u{542f}\u{7528}\u{7cfb}\u{7edf}\u{6258}\u{76d8}".into(),
                startup_enabled: "\u{5f00}\u{673a}\u{81ea}\u{542f}".into(),
                close_app: "\u{5173}\u{95ed} So Todo".into(),
                close_confirm: "\u{9009}\u{62e9}\u{6700}\u{5c0f}\u{5316}\u{5230}\u{6258}\u{76d8}\u{6216}\u{9000}\u{51fa}\u{7a0b}\u{5e8f}\u{3002}".into(),
                delete_confirm: "\u{786e}\u{5b9a}\u{5220}\u{9664}\u{8fd9}\u{4e2a}\u{5f85}\u{529e}\u{5417}\u{ff1f}".into(),
                title_required: "\u{8bf7}\u{8f93}\u{5165}\u{6807}\u{9898}\u{3002}".into(),
                invalid_time: "\u{8bf7}\u{9009}\u{62e9}\u{6709}\u{6548}\u{7684}\u{65e5}\u{671f}\u{548c}\u{65f6}\u{95f4}\u{3002}".into(),
                repeat_badge: "\u{91cd}\u{590d}".into(),
            },            Language::En => Self {
                app_name: "So Todo".into(),
                add: "Add".into(),
                save: "Save".into(),
                cancel: "Cancel".into(),
                delete: "Delete".into(),
                done: "Done".into(),
                settings: "Settings".into(),
                open: "Open".into(),
                calendar: "Calendar".into(),
                completed: "Completed".into(),
                search: "Search title or notes".into(),
                collapse_all: "Collapse all".into(),
                expand_all: "Expand all".into(),
                no_todos: "No todos".into(),
                no_results: "No matching todos".into(),
                unspecified_time: "Unspecified time".into(),
                new_todo: "New todo".into(),
                edit_todo: "Edit todo".into(),
                title: "Title".into(),
                date: "Date".into(),
                time: "Time".into(),
                notes: "Notes".into(),
                repeat: "Repeat".into(),
                repeat_type: "Repeat type".into(),
                weekly: "Weekly".into(),
                monthly: "Monthly".into(),
                monthly_repeat: "Monthly repeat".into(),
                day_of_month: "Day of month".into(),
                last_workday: "Last workday".into(),
                last_day: "Last day".into(),
                day_number: "Day".into(),
                reminders: "Reminders".into(),
                overdue: "Overdue".into(),
                minutes_before: "minutes before".into(),
                add_reminder: "Add reminder".into(),
                default_reminders: "Default reminders".into(),
                language: "Language".into(),
                system: "System".into(),
                english: "English".into(),
                chinese: "Chinese".into(),
                theme: "Theme".into(),
                close_behavior: "Close button".into(),
                ask_on_close: "Ask every time".into(),
                minimize_to_tray: "Minimize to tray".into(),
                exit_app: "Exit app".into(),
                tray_enabled: "Enable system tray".into(),
                startup_enabled: "Start with Windows".into(),
                close_app: "Close So Todo".into(),
                close_confirm: "Choose whether to minimize to tray or exit the app.".into(),
                delete_confirm: "Delete this todo?".into(),
                title_required: "Please enter a title.".into(),
                invalid_time: "Choose a valid date and time.".into(),
                repeat_badge: "Repeats".into(),
            },
        }
    }
}

fn mutate(app: Signal<AppState>, action: impl FnOnce(&mut AppState)) {
    let mut app = app;
    app.with_mut(action);
}

fn show_editor(app: Signal<AppState>, todo: Option<TodoItem>) {
    mutate(app, |state| {
        state.editor = TodoEditor::new(todo.as_ref(), &state.settings.default_reminder_minutes);
        state.dialog = DialogMode::Todo;
    });
}

fn close_dialog(app: Signal<AppState>) {
    mutate(app, |state| {
        state.dialog = DialogMode::None;
        state.pending_delete_id = None;
    });
}

fn request_close(app: Signal<AppState>) {
    let state = app();
    match state.settings.close_behavior.as_str() {
        "tray" if state.settings.tray_enabled => hide_main_window(),
        "prompt" => mutate(app, |state| state.dialog = DialogMode::CloseConfirm),
        _ => dioxus::desktop::window().close(),
    }
}

#[cfg(not(windows))]
fn hide_main_window() {
    dioxus::desktop::window().window.set_visible(false);
}

#[cfg(windows)]
fn hide_main_window() {
    unsafe {
        hide_main_window_native();
    }
}

#[cfg(windows)]
unsafe fn hide_main_window_native() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

    let hwnd = MAIN_WINDOW_HWND.load(Ordering::SeqCst) as windows_sys::Win32::Foundation::HWND;
    if !hwnd.is_null() {
        ShowWindow(hwnd, SW_HIDE);
    }
}

fn toggle_top_most(app: Signal<AppState>) {
    mutate(app, |state| {
        state.top_most = !state.top_most;
        dioxus::desktop::window()
            .window
            .set_always_on_top(state.top_most);
    });
}

fn request_delete(app: Signal<AppState>, id: Uuid) {
    mutate(app, |state| {
        state.pending_delete_id = Some(id);
        state.dialog = DialogMode::DeleteConfirm;
    });
}

fn confirm_delete(app: Signal<AppState>, clock: Signal<NaiveDateTime>) {
    mutate(app, |state| {
        if let Some(id) = state.pending_delete_id {
            state.todos.retain(|todo| todo.id != id);
            state.save();
            reschedule_reminders(state, app, clock);
        }
        state.dialog = DialogMode::None;
        state.pending_delete_id = None;
    });
}

fn save_todo(app: Signal<AppState>, clock: Signal<NaiveDateTime>) {
    mutate(app, |state| {
        let due_at = match state.editor.due_at() {
            Some(value) => round_to_minute(value),
            None => {
                state.editor.validation =
                    Strings::new(state.settings.effective_language()).invalid_time;
                return;
            }
        };
        let has_due_time = !has_unspecified_time(due_at);
        if state.editor.title.trim().is_empty() {
            state.editor.validation =
                Strings::new(state.settings.effective_language()).title_required;
            return;
        }

        let recurrence = if state.editor.recurring && state.editor.due_date_enabled {
            Some(RecurrenceRule {
                kind: state.editor.recurrence_kind,
                weekdays: if state.editor.weekdays.is_empty() {
                    vec![weekday_index(due_at.weekday())]
                } else {
                    state.editor.weekdays.clone()
                },
                day_of_month: parse_i32(&state.editor.day_of_month).unwrap_or(due_at.day() as i32),
                monthly_kind: state.editor.monthly_kind,
            })
        } else {
            None
        };
        let existing = state
            .editor
            .editing_id
            .and_then(|id| state.todos.iter().find(|todo| todo.id == id).cloned());
        let todo = TodoItem {
            id: state.editor.editing_id.unwrap_or_else(Uuid::new_v4),
            title: state.editor.title.trim().to_string(),
            due_at,
            notes: state.editor.notes.trim().to_string(),
            is_done: recurrence.is_none() && state.editor.was_done,
            completed_at: if recurrence.is_none() {
                state.editor.completed_at
            } else {
                None
            },
            reminder_minutes: if has_due_time && state.editor.reminders_enabled {
                normalize_reminders(&state.editor.reminders)
            } else {
                Vec::new()
            },
            recurrence,
            completions: existing.map(|todo| todo.completions).unwrap_or_default(),
        }
        .normalized();

        if let Some(index) = state.todos.iter().position(|item| item.id == todo.id) {
            state.todos[index] = todo;
        } else {
            state.todos.push(todo);
        }
        state.save();
        reschedule_reminders(state, app, clock);
        state.dialog = DialogMode::None;
    });
}

fn toggle_done(
    app: Signal<AppState>,
    clock: Signal<NaiveDateTime>,
    occurrence: TodoOccurrence,
    is_done: bool,
) {
    let pending_key = occurrence_dom_key(&occurrence);
    mutate(app, |state| {
        if let Some(todo) = state
            .todos
            .iter_mut()
            .find(|todo| todo.id == occurrence.source.id)
        {
            if todo.is_recurring() {
                todo.completions
                    .retain(|item| item.occurrence_key != occurrence.occurrence_key);
                if is_done {
                    todo.completions.push(TodoCompletion {
                        occurrence_key: occurrence.occurrence_key.clone(),
                        due_at: occurrence.due_at,
                        completed_at: Local::now().naive_local(),
                    });
                }
            } else {
                todo.is_done = is_done;
                todo.completed_at = is_done.then(|| Local::now().naive_local());
            }
            state
                .pending_completed_occurrences
                .retain(|item| occurrence_dom_key(item) != pending_key);
            if state.mode == ViewMode::List && is_done {
                state.pending_completed_occurrences.push(TodoOccurrence {
                    source: todo.clone(),
                    due_at: occurrence.due_at,
                    occurrence_key: occurrence.occurrence_key.clone(),
                    is_done: true,
                    completed_at: Some(Local::now().naive_local()),
                });
            }
            state.save();
            reschedule_reminders(state, app, clock);
        }
    });

    if is_done {
        spawn(async move {
            tokio::time::sleep(StdDuration::from_secs(1)).await;
            mutate(app, |state| {
                state
                    .pending_completed_occurrences
                    .retain(|item| occurrence_dom_key(item) != pending_key);
            });
        });
    }
}

fn collapse_all(app: Signal<AppState>) {
    mutate(app, |state| {
        state.collapsed_days = grouped_occurrences(state, state.mode == ViewMode::Completed)
            .into_iter()
            .map(|(day, _)| day)
            .collect();
    });
}

fn toggle_day(app: Signal<AppState>, day: NaiveDate) {
    mutate(app, |state| {
        if let Some(index) = state.collapsed_days.iter().position(|value| *value == day) {
            state.collapsed_days.remove(index);
        } else {
            state.collapsed_days.push(day);
        }
    });
}

fn add_editor_reminder(editor: &mut TodoEditor) {
    if let Some(value) = parse_i32(&editor.new_reminder) {
        add_reminder_value(&mut editor.reminders, value);
        editor.new_reminder.clear();
    }
}

fn set_editor_date_enabled(editor: &mut TodoEditor, enabled: bool) {
    editor.due_date_enabled = enabled;
    if enabled {
        if editor.due_date.trim().is_empty() {
            editor.due_date = Local::now().date_naive().format("%Y-%m-%d").to_string();
        }
    } else {
        editor.due_date.clear();
        editor.due_time.clear();
        editor.due_time_enabled = false;
        editor.reminders_enabled = false;
        editor.recurring = false;
        editor.reminders.clear();
    }
}

fn set_editor_time_enabled(editor: &mut TodoEditor, enabled: bool) {
    editor.due_time_enabled = enabled;
    if enabled {
        editor.due_time = default_due_at().format("%H:%M").to_string();
    } else {
        editor.due_time.clear();
        editor.reminders_enabled = false;
        editor.reminders.clear();
    }
}

fn default_due_at() -> NaiveDateTime {
    ceil_datetime_to_five_minutes(Local::now().naive_local() + Duration::hours(1))
}

fn set_editor_time_hour(editor: &mut TodoEditor, hour: &str) {
    let (_, minute) = time_select_parts(&editor.due_time);
    editor.due_time = format!("{}:{minute}", hour.clamp("00", "23"));
}

fn set_editor_time_minute(editor: &mut TodoEditor, minute: &str) {
    let (hour, _) = time_select_parts(&editor.due_time);
    editor.due_time = format!("{hour}:{}", minute.clamp("00", "55"));
}

fn time_select_parts(value: &str) -> (String, String) {
    let time = NaiveTime::parse_from_str(value, "%H:%M")
        .map(snap_time_to_five_minutes)
        .unwrap_or(NaiveTime::MIN);
    (two_digits(time.hour()), two_digits(time.minute()))
}

fn snap_time_to_five_minutes(time: NaiveTime) -> NaiveTime {
    time.with_minute(time.minute() / 5 * 5)
        .and_then(|time| time.with_second(0))
        .unwrap_or(time)
}

fn ceil_datetime_to_five_minutes(value: NaiveDateTime) -> NaiveDateTime {
    let rounded = round_to_minute(value);
    let extra = (5 - rounded.minute() % 5) % 5;
    rounded + Duration::minutes(extra as i64)
}

fn two_digits(value: u32) -> String {
    format!("{value:02}")
}

fn add_default_reminder(app: Signal<AppState>) {
    mutate(app, |state| {
        if let Some(value) = parse_i32(&state.new_default_reminder) {
            add_reminder_value(&mut state.settings.default_reminder_minutes, value);
            state.new_default_reminder.clear();
            state.save();
        }
    });
}

fn add_reminder_value(reminders: &mut Vec<i32>, value: i32) {
    if (0..=10080).contains(&value) && !reminders.contains(&value) {
        reminders.push(value);
        reminders.sort_by(|left, right| right.cmp(left));
    }
}

fn set_weekday(editor: &mut TodoEditor, day: u32) {
    editor.weekdays = vec![day];
}

fn sync_editor_repeat_defaults(editor: &mut TodoEditor) {
    if let Some(due_at) = editor.due_at() {
        if editor.weekdays.is_empty() {
            editor.weekdays.push(weekday_index(due_at.weekday()));
        }
        if parse_i32(&editor.day_of_month).is_none() {
            editor.day_of_month = due_at.day().to_string();
        }
    }
}

#[derive(Clone)]
struct ReminderNotification {
    id: String,
    title: String,
    due_at: NaiveDateTime,
    notify_at: NaiveDateTime,
}

fn reschedule_reminders(
    state: &mut AppState,
    _app: Signal<AppState>,
    mut clock: Signal<NaiveDateTime>,
) {
    let generation = REMINDER_THREAD_GENERATION.fetch_add(1, Ordering::Relaxed) + 1;
    state.reminder_generation = generation;
    let now = Local::now().naive_local();
    let language = state.settings.effective_language();
    for reminder in scheduled_reminders(state, now) {
        schedule_system_notification(reminder, language, generation, now);
    }
    clock.set(now);
}

fn scheduled_reminders(state: &AppState, now: NaiveDateTime) -> Vec<ReminderNotification> {
    open_occurrences(state)
        .into_iter()
        .filter(|occurrence| !occurrence.is_done && !has_unspecified_time(occurrence.due_at))
        .flat_map(|occurrence| {
            let mut reminders = occurrence.source.reminder_minutes.clone();
            reminders.push(0);
            reminders.sort_unstable();
            reminders.dedup();
            reminders
                .into_iter()
                .map(move |minutes| (occurrence.clone(), minutes))
        })
        .filter_map(|(occurrence, minutes)| {
            let id = reminder_id(&occurrence, minutes);
            if state.delivered_reminder_ids.contains(&id) {
                return None;
            }
            let notify_at = occurrence.due_at - Duration::minutes(minutes as i64);
            if !reminder_is_due(occurrence.due_at, minutes, now) {
                return None;
            }
            Some(ReminderNotification {
                id,
                title: occurrence.source.title,
                due_at: occurrence.due_at,
                notify_at,
            })
        })
        .collect()
}

fn reminder_id(occurrence: &TodoOccurrence, minutes: i32) -> String {
    format!("{}:{minutes}", occurrence_dom_key(occurrence))
}

fn reminder_is_due(due_at: NaiveDateTime, minutes_before: i32, now: NaiveDateTime) -> bool {
    let reminder_at = due_at - Duration::minutes(minutes_before as i64);
    reminder_can_be_scheduled(due_at, reminder_at, now)
}

fn reminder_can_be_scheduled(
    _due_at: NaiveDateTime,
    notify_at: NaiveDateTime,
    now: NaiveDateTime,
) -> bool {
    notify_at > now
        || (now >= notify_at && now < notify_at + Duration::seconds(REMINDER_CATCH_UP_SECONDS))
}

fn is_overdue(occurrence: &TodoOccurrence, now: NaiveDateTime) -> bool {
    !occurrence.is_done && !has_unspecified_time(occurrence.due_at) && occurrence.due_at <= now
}

fn show_system_notification(reminder: &ReminderNotification, language: Language) {
    let now = Local::now().naive_local();
    let body = format!(
        "{} {} · {}",
        format_date(reminder.due_at.date(), language),
        todo_time(reminder.due_at),
        reminder_remaining_text(reminder.due_at, now, language)
    );
    let mut toast = Toast::new();
    toast
        .text1(reminder.title.clone())
        .text2(body)
        .tag(reminder.id.clone())
        .group(TOAST_REMINDER_GROUP)
        .scenario(Scenario::Reminder)
        .duration(ToastDuration::Short);
    if let Err(error) = ToastManager::new(TOAST_APP_ID).show(&toast) {
        eprintln!("Failed to show reminder notification: {error}");
    }
}

fn schedule_system_notification(
    reminder: ReminderNotification,
    language: Language,
    generation: u64,
    now: NaiveDateTime,
) {
    let delay = reminder_delay(reminder.notify_at, now);
    std::thread::spawn(move || {
        std::thread::sleep(delay);
        if REMINDER_THREAD_GENERATION.load(Ordering::Relaxed) == generation {
            show_system_notification(&reminder, language);
        }
    });
}

fn reminder_delay(notify_at: NaiveDateTime, now: NaiveDateTime) -> StdDuration {
    (notify_at - now)
        .to_std()
        .unwrap_or_else(|_| StdDuration::from_secs(REMINDER_MIN_DELAY_SECONDS))
}

fn reminder_remaining_text(
    due_at: NaiveDateTime,
    now: NaiveDateTime,
    language: Language,
) -> String {
    let seconds = (due_at - now).num_seconds();
    if seconds <= 0 {
        return match language {
            Language::Zh => "\u{5df2}\u{5230}\u{671f}".into(),
            Language::En => "Due now".into(),
        };
    }

    let minutes = (seconds + 59) / 60;
    match language {
        Language::Zh => format!("\u{5269}\u{4f59} {minutes} \u{5206}\u{949f}"),
        Language::En => format!("{minutes} min remaining"),
    }
}

fn register_system_notifications() {
    if let Err(error) = set_current_process_app_id() {
        eprintln!("Failed to set notification app id: {error}");
    }
    if let Err(error) = register_toast_app(TOAST_APP_ID, "So Todo", None) {
        eprintln!("Failed to register notification app: {error}");
    }
}

#[cfg(windows)]
fn set_current_process_app_id() -> Result<(), i32> {
    use windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

    let app_id = wide_null(TOAST_APP_ID);
    let result = unsafe { SetCurrentProcessExplicitAppUserModelID(app_id.as_ptr()) };
    (result == 0).then_some(()).ok_or(result)
}

#[cfg(not(windows))]
fn set_current_process_app_id() -> Result<(), i32> {
    Ok(())
}

fn grouped_occurrences(state: &AppState, completed: bool) -> Vec<(NaiveDate, Vec<TodoOccurrence>)> {
    let mut items = if completed {
        completed_occurrences(&state.todos)
    } else {
        open_occurrences(state)
    };

    let query = state.query.trim().to_lowercase();
    if !query.is_empty() {
        items.retain(|item| {
            item.source.title.to_lowercase().contains(&query)
                || item.source.notes.to_lowercase().contains(&query)
        });
    }

    let mut groups: BTreeMap<NaiveDate, Vec<TodoOccurrence>> = BTreeMap::new();
    for item in items {
        groups
            .entry(occurrence_group_date(&item))
            .or_default()
            .push(item);
    }

    let mut groups: Vec<_> = groups.into_iter().collect();
    if completed {
        for (_, items) in &mut groups {
            items.sort_by(|left, right| {
                has_specific_time(left.due_at)
                    .cmp(&has_specific_time(right.due_at))
                    .then_with(|| right.due_at.cmp(&left.due_at))
            });
        }
        groups.sort_by(|(left, _), (right, _)| {
            is_unscheduled_date(*right)
                .cmp(&is_unscheduled_date(*left))
                .then_with(|| right.cmp(left))
        });
    } else {
        for (_, items) in &mut groups {
            items.sort_by_key(|item| (has_specific_time(item.due_at), item.due_at));
        }
        groups.sort_by_key(|(day, _)| (!is_unscheduled_date(*day), *day));
    }
    groups
}

fn occurrence_group_date(occurrence: &TodoOccurrence) -> NaiveDate {
    occurrence.due_at.date()
}

fn open_occurrences(state: &AppState) -> Vec<TodoOccurrence> {
    let now = Local::now().naive_local();
    let pending_ids: Vec<_> = state
        .pending_completed_occurrences
        .iter()
        .map(|item| item.source.id)
        .collect();
    let mut items: Vec<_> = state
        .todos
        .iter()
        .filter(|todo| !pending_ids.contains(&todo.id))
        .filter_map(|todo| current_open_occurrence(todo, now))
        .collect();
    items.extend(state.pending_completed_occurrences.clone());
    items
}

fn completed_occurrences(todos: &[TodoItem]) -> Vec<TodoOccurrence> {
    let mut items = Vec::new();
    for todo in todos {
        if todo.is_recurring() {
            for completion in &todo.completions {
                items.push(TodoOccurrence {
                    source: todo.clone(),
                    due_at: completion.due_at,
                    occurrence_key: completion.occurrence_key.clone(),
                    is_done: true,
                    completed_at: Some(completion.completed_at),
                });
            }
        } else if todo.is_done {
            items.push(single_occurrence(todo));
        }
    }
    items
}

fn calendar_occurrences(todos: &[TodoItem], month: NaiveDate) -> Vec<TodoOccurrence> {
    let start = month - Duration::days(month.weekday().num_days_from_sunday() as i64);
    let end = start + Duration::days(42) - Duration::nanoseconds(1);
    occurrences_between(todos, start.and_time(NaiveTime::MIN), end_of_day(end))
        .into_iter()
        .filter(|occurrence| !is_unscheduled_due(occurrence.due_at))
        .collect()
}

fn occurrences_between(
    todos: &[TodoItem],
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> Vec<TodoOccurrence> {
    todos
        .iter()
        .flat_map(|todo| todo_occurrences_between(todo, from, to))
        .collect()
}

fn current_open_occurrence(todo: &TodoItem, now: NaiveDateTime) -> Option<TodoOccurrence> {
    if !todo.is_recurring() {
        return (!todo.is_done).then(|| single_occurrence(todo));
    }

    let from_date = todo.due_at.date().max(now.date() - Duration::days(31));
    let to = end_of_day(now.date() + Duration::days(366 * 2));
    let mut occurrences: Vec<_> =
        todo_occurrences_between(todo, from_date.and_time(NaiveTime::MIN), to)
            .into_iter()
            .filter(|item| !item.is_done)
            .collect();

    occurrences.sort_by_key(|item| item.due_at);
    occurrences
        .iter()
        .rev()
        .find(|item| item.due_at <= now)
        .cloned()
        .or_else(|| occurrences.into_iter().find(|item| item.due_at > now))
}

fn todo_occurrences_between(
    todo: &TodoItem,
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> Vec<TodoOccurrence> {
    if !todo.is_recurring() {
        return if todo.due_at >= from && todo.due_at <= to {
            vec![single_occurrence(todo)]
        } else {
            Vec::new()
        };
    }

    due_dates_between(todo, from, to)
        .into_iter()
        .map(|due_at| to_occurrence(todo, due_at))
        .collect()
}

fn due_dates_between(
    todo: &TodoItem,
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> Vec<NaiveDateTime> {
    let Some(rule) = &todo.recurrence else {
        return Vec::new();
    };

    match rule.kind {
        RecurrenceKind::Weekly => weekly_due_dates(todo, rule, from, to),
        RecurrenceKind::Monthly => monthly_due_dates(todo, rule, from, to),
    }
}

fn weekly_due_dates(
    todo: &TodoItem,
    rule: &RecurrenceRule,
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> Vec<NaiveDateTime> {
    let weekdays = if rule.weekdays.is_empty() {
        vec![weekday_index(todo.due_at.weekday())]
    } else {
        rule.weekdays.clone()
    };
    let mut dates = Vec::new();
    let mut date = from.date().max(todo.due_at.date());
    while date <= to.date() {
        if weekdays.contains(&weekday_index(date.weekday())) {
            let due_at = date.and_time(todo.due_at.time());
            if due_at >= todo.due_at && due_at >= from && due_at <= to {
                dates.push(due_at);
            }
        }
        date += Duration::days(1);
    }
    dates
}

fn monthly_due_dates(
    todo: &TodoItem,
    rule: &RecurrenceRule,
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> Vec<NaiveDateTime> {
    let mut dates = Vec::new();
    let mut cursor = first_of_month(from.date().max(todo.due_at.date()));
    let end = first_of_month(to.date());
    while cursor <= end {
        let due_at = monthly_date(cursor.year(), cursor.month(), rule).and_time(todo.due_at.time());
        if due_at >= todo.due_at && due_at >= from && due_at <= to {
            dates.push(due_at);
        }
        cursor = add_months(cursor, 1);
    }
    dates
}

fn monthly_date(year: i32, month: u32, rule: &RecurrenceRule) -> NaiveDate {
    let last_day = days_in_month(year, month);
    match rule.monthly_kind {
        MonthlyKind::LastDay => NaiveDate::from_ymd_opt(year, month, last_day).unwrap(),
        MonthlyKind::LastWorkday => {
            let mut date = NaiveDate::from_ymd_opt(year, month, last_day).unwrap();
            while matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
                date -= Duration::days(1);
            }
            date
        }
        MonthlyKind::DayOfMonth => NaiveDate::from_ymd_opt(
            year,
            month,
            rule.day_of_month.clamp(1, last_day as i32) as u32,
        )
        .unwrap(),
    }
}

fn single_occurrence(todo: &TodoItem) -> TodoOccurrence {
    TodoOccurrence {
        source: todo.clone(),
        due_at: todo.due_at,
        occurrence_key: occurrence_key(todo.due_at),
        is_done: todo.is_done,
        completed_at: todo.completed_at,
    }
}

fn to_occurrence(todo: &TodoItem, due_at: NaiveDateTime) -> TodoOccurrence {
    let key = occurrence_key(due_at);
    let completion = todo
        .completions
        .iter()
        .find(|item| item.occurrence_key == key);
    TodoOccurrence {
        source: todo.clone(),
        due_at,
        occurrence_key: key,
        is_done: completion.is_some(),
        completed_at: completion.map(|item| item.completed_at),
    }
}

fn occurrence_key(due_at: NaiveDateTime) -> String {
    due_at.format("%Y%m%d%H%M").to_string()
}

fn occurrence_dom_key(occurrence: &TodoOccurrence) -> String {
    format!("{}:{}", occurrence.source.id, occurrence.occurrence_key)
}

fn load_store() -> Store {
    open_database()
        .and_then(|connection| load_store_from_db(&connection))
        .unwrap_or_default()
}

fn save_store(store: &Store) {
    let Ok(mut connection) = open_database() else {
        return;
    };
    let Ok(transaction) = connection.transaction() else {
        return;
    };

    if save_store_to_db(&transaction, store).is_ok() {
        let _ = transaction.commit();
    }
}

fn open_database() -> rusqlite::Result<Connection> {
    let path = data_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let connection = Connection::open(path)?;
    ensure_schema(&connection)?;
    Ok(connection)
}

fn ensure_schema(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS settings_default_reminders (
            position INTEGER PRIMARY KEY,
            minutes INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS todos (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            due_at TEXT NOT NULL,
            notes TEXT NOT NULL,
            is_done INTEGER NOT NULL,
            completed_at TEXT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS todo_reminders (
            todo_id TEXT NOT NULL,
            position INTEGER NOT NULL,
            minutes INTEGER NOT NULL,
            PRIMARY KEY (todo_id, position),
            FOREIGN KEY (todo_id) REFERENCES todos(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS todo_recurrences (
            todo_id TEXT PRIMARY KEY,
            kind INTEGER NOT NULL,
            day_of_month INTEGER NOT NULL,
            monthly_kind INTEGER NOT NULL,
            FOREIGN KEY (todo_id) REFERENCES todos(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS todo_recurrence_weekdays (
            todo_id TEXT NOT NULL,
            weekday INTEGER NOT NULL,
            PRIMARY KEY (todo_id, weekday),
            FOREIGN KEY (todo_id) REFERENCES todos(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS todo_completions (
            todo_id TEXT NOT NULL,
            occurrence_key TEXT NOT NULL,
            due_at TEXT NOT NULL,
            completed_at TEXT NOT NULL,
            PRIMARY KEY (todo_id, occurrence_key),
            FOREIGN KEY (todo_id) REFERENCES todos(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS ix_todos_due_at ON todos(due_at);
        CREATE INDEX IF NOT EXISTS ix_todos_is_done ON todos(is_done);
        "#,
    )
}

fn load_store_from_db(connection: &Connection) -> rusqlite::Result<Store> {
    Ok(Store {
        todos: load_todos(connection)?,
        settings: load_settings(connection)?,
    })
}

fn load_settings(connection: &Connection) -> rusqlite::Result<Settings> {
    let mut settings = Settings {
        language: load_setting(connection, "language")?.unwrap_or_else(default_language),
        theme: load_setting(connection, "theme")?.unwrap_or_else(default_theme),
        close_behavior: load_setting(connection, "close_behavior")?
            .unwrap_or_else(default_close_behavior),
        tray_enabled: load_bool_setting(connection, "tray_enabled")?
            .unwrap_or_else(default_tray_enabled),
        startup_enabled: load_bool_setting(connection, "startup_enabled")?
            .unwrap_or_else(startup_enabled_from_registry),
        default_reminder_minutes: load_default_reminders(connection)?,
    };
    settings.default_reminder_minutes = normalize_reminders(&settings.default_reminder_minutes);
    if settings.close_behavior != "exit"
        && settings.close_behavior != "tray"
        && settings.close_behavior != "prompt"
    {
        settings.close_behavior = default_close_behavior();
    }
    if settings.theme != "system" && !THEMES.contains(&settings.theme.as_str()) {
        settings.theme = default_theme();
    }
    Ok(settings)
}

fn load_bool_setting(connection: &Connection, key: &str) -> rusqlite::Result<Option<bool>> {
    Ok(load_setting(connection, key)?.and_then(|value| parse_bool(&value)))
}

fn load_setting(connection: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()
}

fn load_default_reminders(connection: &Connection) -> rusqlite::Result<Vec<i32>> {
    let mut statement =
        connection.prepare("SELECT minutes FROM settings_default_reminders ORDER BY position")?;
    let values = statement
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<i32>>>()?;
    Ok(if values.is_empty() {
        default_reminders()
    } else {
        values
    })
}

fn load_todos(connection: &Connection) -> rusqlite::Result<Vec<TodoItem>> {
    let mut statement = connection.prepare(
        "SELECT id, title, due_at, notes, is_done, completed_at FROM todos ORDER BY due_at",
    )?;
    let mut todos = statement
        .query_map([], |row| {
            let id_text: String = row.get(0)?;
            let due_at_text: String = row.get(2)?;
            let completed_at_text: Option<String> = row.get(5)?;
            Ok(TodoItem {
                id: Uuid::parse_str(&id_text).unwrap_or_else(|_| Uuid::new_v4()),
                title: row.get(1)?,
                due_at: parse_datetime(&due_at_text),
                notes: row.get(3)?,
                is_done: row.get::<_, i32>(4)? == 1,
                completed_at: completed_at_text.as_deref().map(parse_datetime),
                reminder_minutes: Vec::new(),
                recurrence: None,
                completions: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<Vec<TodoItem>>>()?;

    for todo in &mut todos {
        todo.reminder_minutes = load_todo_reminders(connection, todo.id)?;
        todo.recurrence = load_recurrence(connection, todo.id)?;
        todo.completions = load_completions(connection, todo.id)?;
        *todo = todo.clone().normalized();
    }

    Ok(todos)
}

fn load_todo_reminders(connection: &Connection, todo_id: Uuid) -> rusqlite::Result<Vec<i32>> {
    let mut statement = connection
        .prepare("SELECT minutes FROM todo_reminders WHERE todo_id = ?1 ORDER BY position")?;
    let values = statement
        .query_map([todo_id.to_string()], |row| row.get(0))?
        .collect();
    values
}

fn load_recurrence(
    connection: &Connection,
    todo_id: Uuid,
) -> rusqlite::Result<Option<RecurrenceRule>> {
    let mut rule = connection
        .query_row(
            "SELECT kind, day_of_month, monthly_kind FROM todo_recurrences WHERE todo_id = ?1",
            [todo_id.to_string()],
            |row| {
                Ok(RecurrenceRule {
                    kind: recurrence_kind_from_db(row.get(0)?),
                    weekdays: Vec::new(),
                    day_of_month: row.get(1)?,
                    monthly_kind: monthly_kind_from_db(row.get(2)?),
                })
            },
        )
        .optional()?;

    if let Some(rule) = &mut rule {
        let mut statement = connection.prepare(
            "SELECT weekday FROM todo_recurrence_weekdays WHERE todo_id = ?1 ORDER BY weekday",
        )?;
        rule.weekdays = statement
            .query_map([todo_id.to_string()], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<u32>>>()?;
    }

    Ok(rule)
}

fn load_completions(
    connection: &Connection,
    todo_id: Uuid,
) -> rusqlite::Result<Vec<TodoCompletion>> {
    let mut statement = connection.prepare(
        "SELECT occurrence_key, due_at, completed_at FROM todo_completions WHERE todo_id = ?1 ORDER BY completed_at DESC",
    )?;
    let values = statement
        .query_map([todo_id.to_string()], |row| {
            let due_at: String = row.get(1)?;
            let completed_at: String = row.get(2)?;
            Ok(TodoCompletion {
                occurrence_key: row.get(0)?,
                due_at: parse_datetime(&due_at),
                completed_at: parse_datetime(&completed_at),
            })
        })?
        .collect();
    values
}

fn save_store_to_db(transaction: &Transaction<'_>, store: &Store) -> rusqlite::Result<()> {
    save_setting(transaction, "language", &store.settings.language)?;
    save_setting(transaction, "theme", &store.settings.theme)?;
    save_setting(
        transaction,
        "close_behavior",
        &store.settings.close_behavior,
    )?;
    save_setting(
        transaction,
        "tray_enabled",
        bool_setting(store.settings.tray_enabled),
    )?;
    save_setting(
        transaction,
        "startup_enabled",
        bool_setting(store.settings.startup_enabled),
    )?;

    transaction.execute("DELETE FROM settings_default_reminders", [])?;
    for (index, minutes) in normalize_reminders(&store.settings.default_reminder_minutes)
        .into_iter()
        .enumerate()
    {
        transaction.execute(
            "INSERT INTO settings_default_reminders (position, minutes) VALUES (?1, ?2)",
            params![index as i32, minutes],
        )?;
    }

    transaction.execute("DELETE FROM todos", [])?;
    for todo in &store.todos {
        save_todo_to_db(transaction, &todo.clone().normalized())?;
    }

    Ok(())
}

fn save_setting(transaction: &Transaction<'_>, key: &str, value: &str) -> rusqlite::Result<()> {
    transaction.execute(
        r#"
        INSERT INTO app_settings (key, value, updated_at)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            updated_at = excluded.updated_at
        "#,
        params![key, value, datetime_text(Local::now().naive_local())],
    )?;
    Ok(())
}

fn save_todo_to_db(transaction: &Transaction<'_>, todo: &TodoItem) -> rusqlite::Result<()> {
    transaction.execute(
        r#"
        INSERT INTO todos (id, title, due_at, notes, is_done, completed_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            todo.id.to_string(),
            todo.title,
            datetime_text(todo.due_at),
            todo.notes,
            if todo.is_done { 1 } else { 0 },
            todo.completed_at.map(datetime_text),
            datetime_text(Local::now().naive_local()),
        ],
    )?;

    for (index, minutes) in todo.reminder_minutes.iter().enumerate() {
        transaction.execute(
            "INSERT INTO todo_reminders (todo_id, position, minutes) VALUES (?1, ?2, ?3)",
            params![todo.id.to_string(), index as i32, minutes],
        )?;
    }

    if let Some(rule) = &todo.recurrence {
        transaction.execute(
            "INSERT INTO todo_recurrences (todo_id, kind, day_of_month, monthly_kind) VALUES (?1, ?2, ?3, ?4)",
            params![
                todo.id.to_string(),
                recurrence_kind_to_db(rule.kind),
                rule.day_of_month.clamp(1, 31),
                monthly_kind_to_db(rule.monthly_kind),
            ],
        )?;
        for weekday in &rule.weekdays {
            transaction.execute(
                "INSERT INTO todo_recurrence_weekdays (todo_id, weekday) VALUES (?1, ?2)",
                params![todo.id.to_string(), weekday],
            )?;
        }
    }

    for completion in &todo.completions {
        transaction.execute(
            "INSERT INTO todo_completions (todo_id, occurrence_key, due_at, completed_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                todo.id.to_string(),
                completion.occurrence_key,
                datetime_text(completion.due_at),
                datetime_text(completion.completed_at),
            ],
        )?;
    }

    Ok(())
}

fn data_path() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".sotodo")
        .join("sotodo.db")
}

fn datetime_text(value: NaiveDateTime) -> String {
    value.format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn parse_datetime(value: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S"))
        .unwrap_or_else(|_| round_to_minute(Local::now().naive_local()))
}

fn recurrence_kind_to_db(kind: RecurrenceKind) -> i32 {
    match kind {
        RecurrenceKind::Weekly => 1,
        RecurrenceKind::Monthly => 2,
    }
}

fn recurrence_kind_from_db(value: i32) -> RecurrenceKind {
    if value == 2 {
        RecurrenceKind::Monthly
    } else {
        RecurrenceKind::Weekly
    }
}

fn monthly_kind_to_db(kind: MonthlyKind) -> i32 {
    match kind {
        MonthlyKind::DayOfMonth => 0,
        MonthlyKind::LastWorkday => 1,
        MonthlyKind::LastDay => 2,
    }
}

fn monthly_kind_from_db(value: i32) -> MonthlyKind {
    match value {
        1 => MonthlyKind::LastWorkday,
        2 => MonthlyKind::LastDay,
        _ => MonthlyKind::DayOfMonth,
    }
}

fn normalize_reminders(values: &[i32]) -> Vec<i32> {
    let mut values: Vec<_> = values
        .iter()
        .copied()
        .filter(|value| (0..=10080).contains(value))
        .collect();
    values.sort_by(|left, right| right.cmp(left));
    values.dedup();
    if values.is_empty() {
        vec![15, 5]
    } else {
        values
    }
}

fn calendar_days(month: NaiveDate) -> Vec<NaiveDate> {
    let start = month - Duration::days(month.weekday().num_days_from_sunday() as i64);
    (0..42).map(|index| start + Duration::days(index)).collect()
}

fn first_of_month(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap()
}

fn add_months(date: NaiveDate, months: i32) -> NaiveDate {
    let month_index = date.year() * 12 + date.month0() as i32 + months;
    let year = month_index.div_euclid(12);
    let month = month_index.rem_euclid(12) as u32 + 1;
    NaiveDate::from_ymd_opt(year, month, 1).unwrap()
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };
    (next - Duration::days(1)).day()
}

fn weekday_index(day: Weekday) -> u32 {
    day.num_days_from_sunday()
}

fn weekday_names(language: Language) -> Vec<String> {
    match language {
        Language::Zh => [
            "\u{65e5}", "\u{4e00}", "\u{4e8c}", "\u{4e09}", "\u{56db}", "\u{4e94}", "\u{516d}",
        ],
        Language::En => ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
    }
    .into_iter()
    .map(String::from)
    .collect()
}

fn format_date(date: NaiveDate, language: Language) -> String {
    let today = Local::now().date_naive();
    if date == today {
        return match language {
            Language::Zh => "\u{4eca}\u{5929}".into(),
            Language::En => "Today".into(),
        };
    }
    match language {
        Language::Zh => date.format("%Y\u{5e74}%m\u{6708}%d\u{65e5}").to_string(),
        Language::En => date.format("%A, %m/%d/%Y").to_string(),
    }
}

fn format_group_date(date: NaiveDate, language: Language) -> String {
    if is_unscheduled_date(date) {
        return match language {
            Language::Zh => "\u{672a}\u{6307}\u{5b9a}\u{65f6}\u{95f4}".into(),
            Language::En => "Unspecified time".into(),
        };
    }
    format_date(date, language)
}

fn month_title(date: NaiveDate, language: Language) -> String {
    match language {
        Language::Zh => date.format("%Y\u{5e74}%m\u{6708}").to_string(),
        Language::En => date.format("%Y-%m").to_string(),
    }
}

fn format_title_datetime(value: NaiveDateTime, language: Language) -> String {
    match language {
        Language::Zh => value
            .format("%Y\u{5e74}%m\u{6708}%d\u{65e5} %H:%M")
            .to_string(),
        Language::En => value.format("%m/%d/%Y %H:%M").to_string(),
    }
}

fn todo_time(due_at: NaiveDateTime) -> String {
    format!("{:02}:{:02}", due_at.hour(), due_at.minute())
}

fn unscheduled_due_at() -> NaiveDateTime {
    // ponytail: keep SQLite due_at NOT NULL; migrate to nullable if recurrence needs no-date tasks.
    NaiveDate::from_ymd_opt(UNSCHEDULED_DATE.0, UNSCHEDULED_DATE.1, UNSCHEDULED_DATE.2)
        .unwrap()
        .and_hms_opt(23, 59, 0)
        .unwrap()
}

fn is_unscheduled_due(due_at: NaiveDateTime) -> bool {
    is_unscheduled_date(due_at.date())
}

fn has_unspecified_time(due_at: NaiveDateTime) -> bool {
    is_unscheduled_due(due_at) || due_at.time() == NaiveTime::MIN
}

fn has_specific_time(due_at: NaiveDateTime) -> bool {
    !has_unspecified_time(due_at)
}

fn is_unscheduled_date(date: NaiveDate) -> bool {
    date == NaiveDate::from_ymd_opt(UNSCHEDULED_DATE.0, UNSCHEDULED_DATE.1, UNSCHEDULED_DATE.2)
        .unwrap()
}

fn round_to_minute(value: NaiveDateTime) -> NaiveDateTime {
    value
        .date()
        .and_hms_opt(value.hour(), value.minute(), 0)
        .unwrap()
}

fn end_of_day(date: NaiveDate) -> NaiveDateTime {
    date.and_hms_nano_opt(23, 59, 59, 999_999_999).unwrap()
}

fn tab_class(active: bool) -> &'static str {
    if active {
        "tab tab-active gap-2 text-base font-bold"
    } else {
        "tab gap-2 text-base font-bold"
    }
}

fn todo_notes_class(expanded: bool) -> &'static str {
    if expanded {
        "todo-notes mt-1 block cursor-pointer text-sm opacity-70"
    } else {
        "todo-notes todo-notes-collapsed mt-1 block cursor-pointer text-sm opacity-70"
    }
}

fn calendar_day_class(selected: bool, other_month: bool) -> &'static str {
    match (selected, other_month) {
        (true, _) => "btn btn-primary h-9 min-h-0 flex-col gap-0.5 p-1 text-xs",
        (false, true) => "btn btn-ghost h-9 min-h-0 flex-col gap-0.5 p-1 text-xs opacity-40",
        (false, false) => "btn btn-ghost h-9 min-h-0 flex-col gap-0.5 p-1 text-xs",
    }
}

fn calendar_dot_class(selected: bool) -> &'static str {
    if selected {
        "h-1.5 w-1.5 rounded-full bg-primary-content"
    } else {
        "h-1.5 w-1.5 rounded-full bg-primary"
    }
}

fn parse_i32(value: &str) -> Option<i32> {
    value.trim().parse().ok()
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn bool_setting(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn default_language() -> String {
    "system".into()
}

fn default_theme() -> String {
    "system".into()
}

fn default_tray_enabled() -> bool {
    true
}

fn default_startup_enabled() -> bool {
    false
}

#[cfg(windows)]
fn startup_enabled_from_registry() -> bool {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};

    let key = wide_null(r"Software\Microsoft\Windows\CurrentVersion\Run");
    let name = wide_null("SoTodo");
    let mut value = [0u16; 1024];
    let mut size = (value.len() * std::mem::size_of::<u16>()) as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            value.as_mut_ptr() as *mut _,
            &mut size,
        )
    };

    result == 0 && size > std::mem::size_of::<u16>() as u32
}

#[cfg(not(windows))]
fn startup_enabled_from_registry() -> bool {
    false
}

#[cfg(windows)]
fn apply_startup_setting(enabled: bool) {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, HKEY_CURRENT_USER,
        KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    };

    let key_path = wide_null(r"Software\Microsoft\Windows\CurrentVersion\Run");
    let value_name = wide_null("SoTodo");
    let mut key = std::ptr::null_mut();
    let result = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_path.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    if result != 0 {
        return;
    }

    if enabled {
        if let Ok(path) = std::env::current_exe() {
            let command = wide_null(&format!("\"{}\"", path.display()));
            unsafe {
                RegSetValueExW(
                    key,
                    value_name.as_ptr(),
                    0,
                    REG_SZ,
                    command.as_ptr() as *const u8,
                    (command.len() * std::mem::size_of::<u16>()) as u32,
                );
            }
        }
    } else {
        unsafe {
            RegDeleteValueW(key, value_name.as_ptr());
        }
    }

    unsafe {
        RegCloseKey(key);
    }
}

#[cfg(not(windows))]
fn apply_startup_setting(_enabled: bool) {}

#[cfg(windows)]
fn system_theme() -> Option<&'static str> {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

    let key = wide_null(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let name = wide_null("AppsUseLightTheme");
    let mut value = 1u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut value as *mut u32 as *mut _,
            &mut size,
        )
    };

    (result == 0).then(|| theme_from_apps_use_light(value))
}

#[cfg(not(windows))]
fn system_theme() -> Option<&'static str> {
    None
}

fn theme_from_apps_use_light(value: u32) -> &'static str {
    if value == 0 {
        "dark"
    } else {
        "light"
    }
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn default_close_behavior() -> String {
    "prompt".into()
}

fn default_reminders() -> Vec<i32> {
    vec![15, 5]
}

fn default_day_of_month() -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monthly_last_workday_skips_weekend() {
        let todo = TodoItem {
            id: Uuid::nil(),
            title: "pay".into(),
            due_at: NaiveDate::from_ymd_opt(2024, 1, 1)
                .unwrap()
                .and_hms_opt(9, 0, 0)
                .unwrap(),
            notes: String::new(),
            is_done: false,
            completed_at: None,
            reminder_minutes: vec![15],
            recurrence: Some(RecurrenceRule {
                kind: RecurrenceKind::Monthly,
                weekdays: Vec::new(),
                day_of_month: 1,
                monthly_kind: MonthlyKind::LastWorkday,
            }),
            completions: Vec::new(),
        };

        let dates = due_dates_between(
            &todo,
            NaiveDate::from_ymd_opt(2024, 8, 1)
                .unwrap()
                .and_time(NaiveTime::MIN),
            NaiveDate::from_ymd_opt(2024, 8, 31)
                .unwrap()
                .and_hms_nano_opt(23, 59, 59, 999_999_999)
                .unwrap(),
        );

        assert_eq!(
            dates[0].date(),
            NaiveDate::from_ymd_opt(2024, 8, 30).unwrap()
        );
    }

    #[test]
    fn sqlite_round_trip_keeps_todo_children() {
        let connection = Connection::open_in_memory().unwrap();
        ensure_schema(&connection).unwrap();
        let todo = TodoItem {
            id: Uuid::nil(),
            title: "standup".into(),
            due_at: NaiveDate::from_ymd_opt(2024, 9, 2)
                .unwrap()
                .and_hms_opt(9, 30, 0)
                .unwrap(),
            notes: "demo".into(),
            is_done: false,
            completed_at: None,
            reminder_minutes: vec![30, 5],
            recurrence: Some(RecurrenceRule {
                kind: RecurrenceKind::Weekly,
                weekdays: vec![1, 3],
                day_of_month: 2,
                monthly_kind: MonthlyKind::DayOfMonth,
            }),
            completions: vec![TodoCompletion {
                occurrence_key: "202409020930".into(),
                due_at: NaiveDate::from_ymd_opt(2024, 9, 2)
                    .unwrap()
                    .and_hms_opt(9, 30, 0)
                    .unwrap(),
                completed_at: NaiveDate::from_ymd_opt(2024, 9, 2)
                    .unwrap()
                    .and_hms_opt(9, 31, 0)
                    .unwrap(),
            }],
        };

        let transaction = connection.unchecked_transaction().unwrap();
        save_store_to_db(
            &transaction,
            &Store {
                todos: vec![todo],
                settings: Settings::default(),
            },
        )
        .unwrap();
        transaction.commit().unwrap();

        let store = load_store_from_db(&connection).unwrap();
        assert_eq!(store.todos.len(), 1);
        assert_eq!(store.todos[0].reminder_minutes, vec![30, 5]);
        assert_eq!(
            store.todos[0].recurrence.as_ref().unwrap().weekdays,
            vec![1, 3]
        );
        assert_eq!(store.todos[0].completions.len(), 1);
    }

    #[test]
    fn locale_prefix_selects_chinese() {
        assert_eq!(language_from_locale("zh-CN"), Language::Zh);
        assert_eq!(language_from_locale("en-US"), Language::En);
    }

    #[test]
    fn windows_theme_flag_maps_to_daisyui_base_themes() {
        assert_eq!(theme_from_apps_use_light(0), "dark");
        assert_eq!(theme_from_apps_use_light(1), "light");
    }

    #[test]
    fn reminder_window_schedules_future_and_current_minute_notifications() {
        let due_at = NaiveDate::from_ymd_opt(2024, 9, 2)
            .unwrap()
            .and_hms_opt(9, 30, 0)
            .unwrap();
        assert!(reminder_is_due(due_at, 15, due_at - Duration::minutes(16)));
        assert!(reminder_is_due(due_at, 15, due_at - Duration::minutes(15)));
        assert!(reminder_is_due(
            due_at,
            15,
            due_at - Duration::minutes(15) + Duration::seconds(30)
        ));
        assert!(!reminder_is_due(due_at, 15, due_at - Duration::minutes(14)));
        assert!(!reminder_is_due(due_at, 15, due_at + Duration::seconds(1)));
    }

    #[test]
    fn due_time_notification_is_created_before_due() {
        let due_at = NaiveDate::from_ymd_opt(2024, 9, 2)
            .unwrap()
            .and_hms_opt(9, 30, 0)
            .unwrap();
        let todo = TodoItem {
            id: Uuid::nil(),
            title: "due".into(),
            due_at,
            notes: String::new(),
            is_done: false,
            completed_at: None,
            reminder_minutes: Vec::new(),
            recurrence: None,
            completions: Vec::new(),
        };
        let state = AppState {
            todos: vec![todo],
            settings: Settings::default(),
            mode: ViewMode::List,
            query: String::new(),
            visible_month: first_of_month(due_at.date()),
            selected_date: due_at.date(),
            collapsed_days: Vec::new(),
            pending_completed_occurrences: Vec::new(),
            top_most: false,
            dialog: DialogMode::None,
            editor: TodoEditor::new(None, &[15, 5]),
            pending_delete_id: None,
            new_default_reminder: String::new(),
            reminder_generation: 0,
            delivered_reminder_ids: Vec::new(),
        };

        let reminders = scheduled_reminders(&state, due_at - Duration::minutes(1));
        assert_eq!(reminders.len(), 1);
        assert_eq!(
            reminders[0].id,
            format!("{}:{}:0", Uuid::nil(), occurrence_key(due_at))
        );
    }

    #[test]
    fn past_reminder_is_not_created_on_save() {
        let due_at = NaiveDate::from_ymd_opt(2024, 9, 2)
            .unwrap()
            .and_hms_opt(9, 30, 0)
            .unwrap();
        let todo = TodoItem {
            id: Uuid::nil(),
            title: "due".into(),
            due_at,
            notes: String::new(),
            is_done: false,
            completed_at: None,
            reminder_minutes: vec![15],
            recurrence: None,
            completions: Vec::new(),
        };
        let state = AppState {
            todos: vec![todo],
            settings: Settings::default(),
            mode: ViewMode::List,
            query: String::new(),
            visible_month: first_of_month(due_at.date()),
            selected_date: due_at.date(),
            collapsed_days: Vec::new(),
            pending_completed_occurrences: Vec::new(),
            top_most: false,
            dialog: DialogMode::None,
            editor: TodoEditor::new(None, &[15, 5]),
            pending_delete_id: None,
            new_default_reminder: String::new(),
            reminder_generation: 0,
            delivered_reminder_ids: Vec::new(),
        };

        let reminders = scheduled_reminders(&state, due_at - Duration::minutes(14));
        assert_eq!(reminders.len(), 1);
        assert!(reminders[0].id.ends_with(":0"));
    }

    #[test]
    fn reminder_notification_shows_remaining_minutes() {
        let due_at = NaiveDate::from_ymd_opt(2024, 9, 2)
            .unwrap()
            .and_hms_opt(9, 30, 0)
            .unwrap();

        assert_eq!(
            reminder_remaining_text(
                due_at,
                due_at - Duration::minutes(5) + Duration::seconds(1),
                Language::Zh,
            ),
            "\u{5269}\u{4f59} 5 \u{5206}\u{949f}"
        );
        assert_eq!(
            reminder_remaining_text(due_at, due_at, Language::En),
            "Due now"
        );
    }

    #[test]
    fn unfinished_past_occurrence_is_overdue() {
        let due_at = NaiveDate::from_ymd_opt(2024, 9, 2)
            .unwrap()
            .and_hms_opt(9, 30, 0)
            .unwrap();
        let todo = TodoItem {
            id: Uuid::nil(),
            title: "late".into(),
            due_at,
            notes: String::new(),
            is_done: false,
            completed_at: None,
            reminder_minutes: vec![15],
            recurrence: None,
            completions: Vec::new(),
        };

        let mut occurrence = single_occurrence(&todo);
        assert!(is_overdue(&occurrence, due_at));
        assert!(is_overdue(&occurrence, due_at + Duration::minutes(1)));
        occurrence.is_done = true;
        assert!(!is_overdue(&occurrence, due_at + Duration::minutes(1)));
    }

    #[test]
    fn unspecified_time_is_grouped_first_without_reminders_or_overdue() {
        let today = Local::now().date_naive();
        let scheduled = today.and_hms_opt(9, 30, 0).unwrap();
        let date_only_due_at = today.and_time(NaiveTime::MIN);
        let unscheduled = TodoItem {
            id: Uuid::nil(),
            title: "inbox".into(),
            due_at: unscheduled_due_at(),
            notes: String::new(),
            is_done: false,
            completed_at: None,
            reminder_minutes: vec![15],
            recurrence: None,
            completions: Vec::new(),
        }
        .normalized();
        let date_only = TodoItem {
            id: Uuid::new_v4(),
            title: "date only".into(),
            due_at: date_only_due_at,
            notes: String::new(),
            is_done: false,
            completed_at: None,
            reminder_minutes: Vec::new(),
            recurrence: None,
            completions: Vec::new(),
        };
        let timed = TodoItem {
            id: Uuid::new_v4(),
            title: "later".into(),
            due_at: scheduled,
            notes: String::new(),
            is_done: false,
            completed_at: None,
            reminder_minutes: Vec::new(),
            recurrence: None,
            completions: Vec::new(),
        };
        let state = AppState {
            todos: vec![timed, date_only, unscheduled.clone()],
            settings: Settings::default(),
            mode: ViewMode::List,
            query: String::new(),
            visible_month: first_of_month(scheduled.date()),
            selected_date: scheduled.date(),
            collapsed_days: Vec::new(),
            pending_completed_occurrences: Vec::new(),
            top_most: false,
            dialog: DialogMode::None,
            editor: TodoEditor::new(None, &[15]),
            pending_delete_id: None,
            new_default_reminder: String::new(),
            reminder_generation: 0,
            delivered_reminder_ids: Vec::new(),
        };

        let groups = grouped_occurrences(&state, false);
        assert!(is_unscheduled_date(groups[0].0));
        assert!(has_unspecified_time(groups[0].1[0].due_at));
        let today_group = groups.iter().find(|(day, _)| *day == today).unwrap();
        assert_eq!(today_group.1[0].source.title, "date only");
        assert!(has_unspecified_time(today_group.1[0].due_at));
        assert!(!is_unscheduled_due(today_group.1[0].due_at));
        let calendar = calendar_occurrences(&state.todos, first_of_month(today));
        assert!(calendar.iter().any(|item| item.source.title == "date only"));
        assert!(calendar.iter().all(|item| item.source.title != "inbox"));
        let mut only_unscheduled = state.clone();
        only_unscheduled.todos = vec![unscheduled.clone()];
        assert!(
            scheduled_reminders(&only_unscheduled, scheduled - Duration::minutes(1)).is_empty()
        );
        assert!(!is_overdue(
            &single_occurrence(&unscheduled),
            Local::now().naive_local()
        ));
    }

    #[test]
    fn weekly_repeat_day_selection_is_single_choice() {
        let mut editor = TodoEditor::new(None, &[15]);
        set_weekday(&mut editor, 1);
        set_weekday(&mut editor, 3);
        assert_eq!(editor.weekdays, vec![3]);
    }

    #[test]
    fn time_minutes_snap_to_five_minute_steps() {
        let time = NaiveTime::from_hms_opt(9, 37, 42).unwrap();
        assert_eq!(
            snap_time_to_five_minutes(time),
            NaiveTime::from_hms_opt(9, 35, 0).unwrap()
        );
        assert_eq!(time_select_parts("09:37"), ("09".into(), "35".into()));
        assert_eq!(
            ceil_datetime_to_five_minutes(
                NaiveDate::from_ymd_opt(2024, 9, 2)
                    .unwrap()
                    .and_hms_opt(23, 58, 0)
                    .unwrap(),
            ),
            NaiveDate::from_ymd_opt(2024, 9, 3)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        );
    }

    #[test]
    fn todo_notes_are_collapsed_until_expanded() {
        assert!(todo_notes_class(false).contains("todo-notes-collapsed"));
        assert!(!todo_notes_class(true).contains("todo-notes-collapsed"));
    }

    #[test]
    fn chinese_strings_render_from_escapes() {
        let text = Strings::new(Language::Zh);
        assert_eq!(text.add, "\u{6dfb}\u{52a0}");
        assert_eq!(
            format_date(NaiveDate::from_ymd_opt(2024, 9, 2).unwrap(), Language::Zh),
            "2024\u{5e74}09\u{6708}02\u{65e5}"
        );
    }

    #[test]
    fn date_parts_are_zero_padded() {
        let value = NaiveDate::from_ymd_opt(2024, 9, 2)
            .unwrap()
            .and_hms_opt(7, 5, 30)
            .unwrap();

        assert_eq!(
            format_title_datetime(value, Language::Zh),
            "2024\u{5e74}09\u{6708}02\u{65e5} 07:05"
        );
        assert_eq!(
            format_date(value.date(), Language::En),
            "Monday, 09/02/2024"
        );
        assert_eq!(month_title(value.date(), Language::En), "2024-09");
    }

    #[test]
    fn pending_completed_todo_stays_open_temporarily() {
        let due_at = round_to_minute(Local::now().naive_local());
        let todo = TodoItem {
            id: Uuid::nil(),
            title: "done".into(),
            due_at,
            notes: String::new(),
            is_done: true,
            completed_at: Some(due_at),
            reminder_minutes: vec![15],
            recurrence: None,
            completions: Vec::new(),
        };
        let pending = TodoOccurrence {
            source: todo.clone(),
            due_at,
            occurrence_key: occurrence_key(due_at),
            is_done: true,
            completed_at: Some(due_at),
        };
        let today = Local::now().date_naive();
        let mut state = AppState {
            todos: vec![todo],
            settings: Settings::default(),
            mode: ViewMode::List,
            query: String::new(),
            visible_month: first_of_month(today),
            selected_date: today,
            collapsed_days: Vec::new(),
            pending_completed_occurrences: vec![pending],
            top_most: false,
            dialog: DialogMode::None,
            editor: TodoEditor::new(None, &[15]),
            pending_delete_id: None,
            new_default_reminder: String::new(),
            reminder_generation: 0,
            delivered_reminder_ids: Vec::new(),
        };

        assert_eq!(open_occurrences(&state).len(), 1);
        state.pending_completed_occurrences.clear();
        assert!(open_occurrences(&state).is_empty());
    }
}
