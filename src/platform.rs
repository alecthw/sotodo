#![allow(unused_imports)]

use crate::assets::{best_ico_image, APPICON_ICO_BYTES};
use crate::config::PROJECT_URL;
use crate::models::Language;
use crate::todo_logic::{parse_hotkey, HotkeySpec};
use chrono::NaiveDateTime;
#[cfg(windows)]
use dioxus::desktop::tao::dpi::PhysicalPosition;
#[cfg(windows)]
use dioxus::desktop::tao::platform::windows::WindowExtWindows;
use std::sync::atomic::Ordering;

#[cfg(windows)]
pub(crate) fn snap_window_to_top_right() {
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

#[cfg(windows)]
pub(crate) fn open_project_homepage() {
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
pub(crate) fn open_project_homepage() {}

#[cfg(windows)]
pub(crate) unsafe fn winhttp_get(host: &str, path: &str) -> Option<String> {
    use windows_sys::Win32::Networking::WinHttp::{
        WinHttpAddRequestHeaders, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest,
        WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
        WinHttpSetTimeouts, INTERNET_DEFAULT_HTTPS_PORT, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
        WINHTTP_ADDREQ_FLAG_ADD, WINHTTP_FLAG_SECURE, WINHTTP_QUERY_FLAG_NUMBER,
        WINHTTP_QUERY_STATUS_CODE,
    };

    struct HttpHandle(*mut core::ffi::c_void);
    impl Drop for HttpHandle {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    windows_sys::Win32::Networking::WinHttp::WinHttpCloseHandle(self.0);
                }
            }
        }
    }

    let agent = wide_null("SoTodo/1.0");
    let session = WinHttpOpen(
        agent.as_ptr(),
        WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
        std::ptr::null(),
        std::ptr::null(),
        0,
    );
    if session.is_null() {
        return None;
    }
    let session = HttpHandle(session);
    WinHttpSetTimeouts(session.0, 3000, 3000, 3000, 5000);

    let host = wide_null(host);
    let connect = WinHttpConnect(session.0, host.as_ptr(), INTERNET_DEFAULT_HTTPS_PORT, 0);
    if connect.is_null() {
        return None;
    }
    let connect = HttpHandle(connect);

    let method = wide_null("GET");
    let path = wide_null(path);
    let request = WinHttpOpenRequest(
        connect.0,
        method.as_ptr(),
        path.as_ptr(),
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
        WINHTTP_FLAG_SECURE,
    );
    if request.is_null() {
        return None;
    }
    let request = HttpHandle(request);

    let headers = wide_null(
        "User-Agent: SoTodo\r\nAccept: application/vnd.github+json\r\nX-GitHub-Api-Version: 2022-11-28\r\n",
    );
    if WinHttpAddRequestHeaders(
        request.0,
        headers.as_ptr(),
        (headers.len() - 1) as u32,
        WINHTTP_ADDREQ_FLAG_ADD,
    ) == 0
    {
        return None;
    }

    if WinHttpSendRequest(request.0, std::ptr::null(), 0, std::ptr::null(), 0, 0, 0) == 0 {
        return None;
    }
    if WinHttpReceiveResponse(request.0, std::ptr::null_mut()) == 0 {
        return None;
    }

    let mut status_code = 0u32;
    let mut status_size = std::mem::size_of::<u32>() as u32;
    if WinHttpQueryHeaders(
        request.0,
        WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
        std::ptr::null(),
        &mut status_code as *mut u32 as *mut _,
        &mut status_size,
        std::ptr::null_mut(),
    ) == 0
        || status_code != 200
    {
        return None;
    }

    let mut body = Vec::new();
    loop {
        let mut buffer = [0u8; 4096];
        let mut read = 0u32;
        if WinHttpReadData(
            request.0,
            buffer.as_mut_ptr() as *mut _,
            buffer.len() as u32,
            &mut read,
        ) == 0
        {
            return None;
        }
        if read == 0 {
            break;
        }
        body.extend_from_slice(&buffer[..read as usize]);
        if body.len() > 256 * 1024 {
            return None;
        }
    }

    String::from_utf8(body).ok()
}

#[derive(Clone)]
pub(crate) struct WindowsTray {
    pub(crate) hwnd: std::sync::Arc<std::sync::atomic::AtomicIsize>,
    pub(crate) enabled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub(crate) labels: std::sync::Arc<std::sync::Mutex<TrayLabels>>,
    pub(crate) hotkey: std::sync::Arc<std::sync::Mutex<HotkeyConfig>>,
}

#[cfg(windows)]
#[derive(Clone)]
pub(crate) struct TrayLabels {
    pub(crate) show: String,
    pub(crate) exit: String,
}

#[cfg(windows)]
impl TrayLabels {
    pub(crate) fn new(language: Language) -> Self {
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
#[derive(Clone)]
pub(crate) struct HotkeyConfig {
    pub(crate) enabled: bool,
    pub(crate) value: String,
}

#[cfg(windows)]
impl WindowsTray {
    pub(crate) fn new(
        language: Language,
        main_hwnd: isize,
        hotkey_enabled: bool,
        hotkey_value: String,
    ) -> Self {
        use std::sync::{atomic::AtomicBool, atomic::AtomicIsize, Arc};

        let hwnd = Arc::new(AtomicIsize::new(0));
        let enabled = Arc::new(AtomicBool::new(false));
        let labels = native_tray_labels(language);
        let hotkey = Arc::new(std::sync::Mutex::new(HotkeyConfig {
            enabled: hotkey_enabled,
            value: hotkey_value,
        }));
        MAIN_WINDOW_HWND.store(main_hwnd, Ordering::SeqCst);
        {
            let mut current = labels
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *current = TrayLabels::new(language);
        }

        let thread_hwnd = Arc::clone(&hwnd);
        let thread_enabled = Arc::clone(&enabled);
        let thread_hotkey = Arc::clone(&hotkey);
        std::thread::spawn(move || unsafe {
            run_windows_tray(thread_hwnd, thread_enabled, thread_hotkey);
        });

        Self {
            hwnd,
            enabled,
            labels,
            hotkey,
        }
    }

    pub(crate) fn set_main_window(&self, hwnd: isize) {
        MAIN_WINDOW_HWND.store(hwnd, Ordering::SeqCst);
    }

    pub(crate) fn set_language(&self, language: Language) {
        let mut labels = self
            .labels
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *labels = TrayLabels::new(language);
    }

    pub(crate) fn set_visible(&self, visible: bool) {
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

    pub(crate) fn set_hotkey(&self, enabled: bool, value: String) {
        {
            let mut hotkey = self
                .hotkey
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            hotkey.enabled = enabled;
            hotkey.value = value;
        }
        let hwnd = self.hwnd.load(Ordering::SeqCst) as windows_sys::Win32::Foundation::HWND;
        if !hwnd.is_null() {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW(
                    hwnd,
                    WM_HOTKEY_APPLY,
                    0,
                    0,
                );
            }
        }
    }
}

#[cfg(windows)]
pub(crate) static MAIN_WINDOW_HWND: std::sync::atomic::AtomicIsize =
    std::sync::atomic::AtomicIsize::new(0);
#[cfg(windows)]
pub(crate) static TRAY_LABELS: std::sync::OnceLock<std::sync::Arc<std::sync::Mutex<TrayLabels>>> =
    std::sync::OnceLock::new();

#[cfg(windows)]
pub(crate) const WM_TRAY_ICON: u32 = 0x0400 + 10;
#[cfg(windows)]
pub(crate) const WM_TRAY_APPLY: u32 = 0x0400 + 11;
#[cfg(windows)]
pub(crate) const WM_HOTKEY_APPLY: u32 = 0x0400 + 12;
#[cfg(windows)]
pub(crate) const TRAY_UID: u32 = 1;
#[cfg(windows)]
pub(crate) const HOTKEY_ID_SHOW_WINDOW: i32 = 2001;
#[cfg(windows)]
pub(crate) const TRAY_MENU_SHOW: usize = 1001;
#[cfg(windows)]
pub(crate) const TRAY_MENU_EXIT: usize = 1002;

#[cfg(windows)]
pub(crate) fn native_tray_labels(
    language: Language,
) -> std::sync::Arc<std::sync::Mutex<TrayLabels>> {
    TRAY_LABELS
        .get_or_init(|| std::sync::Arc::new(std::sync::Mutex::new(TrayLabels::new(language))))
        .clone()
}

#[cfg(windows)]
pub(crate) fn current_main_hwnd() -> isize {
    dioxus::desktop::window().window.hwnd()
}

#[cfg(windows)]
pub(crate) unsafe fn run_windows_tray(
    hwnd_slot: std::sync::Arc<std::sync::atomic::AtomicIsize>,
    enabled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    hotkey: std::sync::Arc<std::sync::Mutex<HotkeyConfig>>,
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
    let mut hotkey_registered = false;
    apply_tray_icon(hwnd, icon, enabled.load(Ordering::SeqCst), &mut icon_added);
    apply_hotkey(hwnd, &hotkey, &mut hotkey_registered);

    let mut message = MSG::default();
    while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
        if message.message == WM_TRAY_APPLY {
            apply_tray_icon(hwnd, icon, enabled.load(Ordering::SeqCst), &mut icon_added);
            continue;
        }
        if message.message == WM_HOTKEY_APPLY {
            apply_hotkey(hwnd, &hotkey, &mut hotkey_registered);
            continue;
        }
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }

    if hotkey_registered {
        unregister_hotkey(hwnd);
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
        DefWindowProcW, PostQuitMessage, WM_DESTROY, WM_HOTKEY, WM_LBUTTONDBLCLK, WM_LBUTTONUP,
        WM_RBUTTONUP,
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

    if msg == WM_HOTKEY && wparam == HOTKEY_ID_SHOW_WINDOW as usize {
        restore_main_window_native();
        return 0;
    }

    if msg == WM_DESTROY {
        PostQuitMessage(0);
        return 0;
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

#[cfg(windows)]
pub(crate) unsafe fn show_tray_menu(hwnd: windows_sys::Win32::Foundation::HWND) {
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
pub(crate) unsafe fn apply_tray_icon(
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
pub(crate) unsafe fn apply_hotkey(
    hwnd: windows_sys::Win32::Foundation::HWND,
    hotkey: &std::sync::Arc<std::sync::Mutex<HotkeyConfig>>,
    registered: &mut bool,
) {
    let config = hotkey
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();

    if *registered {
        unregister_hotkey(hwnd);
        *registered = false;
    }

    if !config.enabled {
        return;
    }

    let Some(spec) = parse_hotkey(&config.value) else {
        return;
    };

    *registered = register_hotkey(hwnd, spec);
}

#[cfg(windows)]
pub(crate) unsafe fn register_hotkey(
    hwnd: windows_sys::Win32::Foundation::HWND,
    spec: HotkeySpec,
) -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
    };

    let mut modifiers = MOD_NOREPEAT;
    if spec.ctrl {
        modifiers |= MOD_CONTROL;
    }
    if spec.alt {
        modifiers |= MOD_ALT;
    }
    if spec.shift {
        modifiers |= MOD_SHIFT;
    }
    if spec.win {
        modifiers |= MOD_WIN;
    }

    RegisterHotKey(hwnd, HOTKEY_ID_SHOW_WINDOW, modifiers, spec.key) != 0
}

#[cfg(windows)]
pub(crate) unsafe fn unregister_hotkey(hwnd: windows_sys::Win32::Foundation::HWND) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::UnregisterHotKey;

    UnregisterHotKey(hwnd, HOTKEY_ID_SHOW_WINDOW);
}

#[cfg(windows)]
pub(crate) fn notify_icon_data(
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
pub(crate) unsafe fn load_tray_icon() -> (windows_sys::Win32::UI::WindowsAndMessaging::HICON, bool)
{
    use std::ptr::null_mut;
    use windows_sys::Win32::UI::WindowsAndMessaging::{LoadIconW, IDI_APPLICATION};

    if let Some(icon) = load_embedded_tray_icon() {
        return (icon, true);
    }

    (LoadIconW(null_mut(), IDI_APPLICATION), false)
}

#[cfg(windows)]
pub(crate) unsafe fn load_embedded_tray_icon(
) -> Option<windows_sys::Win32::UI::WindowsAndMessaging::HICON> {
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

pub(crate) unsafe fn restore_main_window_native() {
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
pub(crate) unsafe fn close_main_window_native() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

    let hwnd = MAIN_WINDOW_HWND.load(Ordering::SeqCst) as windows_sys::Win32::Foundation::HWND;
    if !hwnd.is_null() {
        PostMessageW(hwnd, WM_CLOSE, 0, 0);
    }
}

#[cfg(windows)]
pub(crate) fn write_wide_fixed(target: &mut [u16], value: &str) {
    let wide = value.encode_utf16();
    for (slot, chr) in target.iter_mut().zip(wide) {
        *slot = chr;
    }
}

#[cfg(not(windows))]
pub(crate) fn hide_main_window() {
    dioxus::desktop::window().window.set_visible(false);
}

#[cfg(windows)]
pub(crate) fn hide_main_window() {
    unsafe {
        hide_main_window_native();
    }
}

#[cfg(windows)]
pub(crate) unsafe fn hide_main_window_native() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};

    let hwnd = MAIN_WINDOW_HWND.load(Ordering::SeqCst) as windows_sys::Win32::Foundation::HWND;
    if !hwnd.is_null() {
        ShowWindow(hwnd, SW_HIDE);
    }
}

pub(crate) fn startup_enabled_from_registry() -> bool {
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
pub(crate) fn startup_enabled_from_registry() -> bool {
    false
}

#[cfg(windows)]
pub(crate) fn apply_startup_setting(enabled: bool) {
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
pub(crate) fn apply_startup_setting(_enabled: bool) {}

#[cfg(windows)]
pub(crate) fn system_theme() -> Option<&'static str> {
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
pub(crate) fn system_theme() -> Option<&'static str> {
    None
}

pub(crate) fn theme_from_apps_use_light(value: u32) -> &'static str {
    if value == 0 {
        "dark"
    } else {
        "light"
    }
}

#[cfg(windows)]
pub(crate) fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
