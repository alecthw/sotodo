use std::sync::atomic::AtomicU64;

pub(crate) const TOAST_APP_ID: &str = "SoTodo.Dioxus.Desktop";
pub(crate) const TOAST_REMINDER_GROUP: &str = "sotodo-reminders";
pub(crate) const REMINDER_CATCH_UP_SECONDS: i64 = 60;
pub(crate) const REMINDER_MIN_DELAY_SECONDS: u64 = 3;
pub(crate) const UNSCHEDULED_DATE: (i32, u32, u32) = (9999, 12, 31);
pub(crate) const APP_WINDOW_WIDTH: f64 = 420.0;
pub(crate) const APP_WINDOW_HEIGHT: f64 = 720.0;
pub(crate) const APP_VERSION: &str = match option_env!("SOTODO_VERSION") {
    Some(version) => version,
    None => "develop",
};
pub(crate) const PROJECT_URL: &str = "https://github.com/alecthw/sotodo";
pub(crate) const LATEST_RELEASE_API_PATH: &str = "/repos/alecthw/sotodo/releases/latest";
pub(crate) const UPDATE_STATUS_NO_UPDATE: u8 = 0;
pub(crate) const UPDATE_STATUS_AVAILABLE: u8 = 1;
pub(crate) static REMINDER_THREAD_GENERATION: AtomicU64 = AtomicU64::new(0);
pub(crate) static UPDATE_CHECK_STARTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
pub(crate) static UPDATE_STATUS: std::sync::atomic::AtomicU8 =
    std::sync::atomic::AtomicU8::new(UPDATE_STATUS_NO_UPDATE);

pub(crate) const THEMES: &[&str] = &[
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
