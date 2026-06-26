#![allow(unused_imports)]

use crate::platform::{apply_startup_setting, system_theme};
use crate::storage::{load_store, save_store};
use crate::todo_logic::*;
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, PartialEq)]
pub(crate) struct AppState {
    pub(crate) todos: Vec<TodoItem>,
    pub(crate) settings: Settings,
    pub(crate) mode: ViewMode,
    pub(crate) query: String,
    pub(crate) visible_month: NaiveDate,
    pub(crate) selected_date: NaiveDate,
    pub(crate) collapsed_days: Vec<NaiveDate>,
    pub(crate) pending_completed_occurrences: Vec<TodoOccurrence>,
    pub(crate) top_most: bool,
    pub(crate) dialog: DialogMode,
    pub(crate) editor: TodoEditor,
    pub(crate) settings_editor: Settings,
    pub(crate) settings_hotkey_recording: bool,
    pub(crate) pending_delete_id: Option<Uuid>,
    pub(crate) new_default_reminder: String,
    pub(crate) reminder_generation: u64,
    pub(crate) delivered_reminder_ids: Vec<String>,
}

impl AppState {
    pub(crate) fn load() -> Self {
        let today = Local::now().date_naive();
        let store = load_store();
        let settings = store.settings.clone();

        Self {
            todos: store.todos,
            settings,
            mode: ViewMode::List,
            query: String::new(),
            visible_month: first_of_month(today),
            selected_date: today,
            collapsed_days: Vec::new(),
            pending_completed_occurrences: Vec::new(),
            top_most: false,
            dialog: DialogMode::None,
            editor: TodoEditor::new(None, &[15, 5]),
            settings_editor: store.settings,
            settings_hotkey_recording: false,
            pending_delete_id: None,
            new_default_reminder: String::new(),
            reminder_generation: 0,
            delivered_reminder_ids: Vec::new(),
        }
    }

    pub(crate) fn save(&self) {
        save_store(&Store {
            todos: self.todos.clone(),
            settings: self.settings.clone(),
        });
        apply_startup_setting(self.settings.startup_enabled);
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ViewMode {
    List,
    Calendar,
    Completed,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum DialogMode {
    None,
    Todo,
    Settings,
    DeleteConfirm,
    CloseConfirm,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Store {
    #[serde(default)]
    pub(crate) todos: Vec<TodoItem>,
    #[serde(default)]
    pub(crate) settings: Settings,
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
pub(crate) struct Settings {
    #[serde(default = "default_language")]
    pub(crate) language: String,
    #[serde(default = "default_theme")]
    pub(crate) theme: String,
    #[serde(default = "default_close_behavior")]
    pub(crate) close_behavior: String,
    #[serde(default = "default_tray_enabled")]
    pub(crate) tray_enabled: bool,
    #[serde(default = "default_startup_enabled")]
    pub(crate) startup_enabled: bool,
    #[serde(default = "default_hotkey_enabled")]
    pub(crate) hotkey_enabled: bool,
    #[serde(default = "default_hotkey")]
    pub(crate) hotkey: String,
    #[serde(default = "default_reminders")]
    pub(crate) default_reminder_minutes: Vec<i32>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: default_language(),
            theme: default_theme(),
            close_behavior: default_close_behavior(),
            tray_enabled: default_tray_enabled(),
            startup_enabled: default_startup_enabled(),
            hotkey_enabled: default_hotkey_enabled(),
            hotkey: default_hotkey(),
            default_reminder_minutes: default_reminders(),
        }
    }
}

impl Settings {
    pub(crate) fn effective_language(&self) -> Language {
        match self.language.as_str() {
            "zh" => Language::Zh,
            "en" => Language::En,
            _ => system_language()
                .or_else(|| env_locale().map(|locale| language_from_locale(&locale)))
                .unwrap_or(Language::En),
        }
    }

    pub(crate) fn effective_theme(&self) -> String {
        if self.theme == "system" {
            system_theme().unwrap_or("light").into()
        } else {
            self.theme.clone()
        }
    }
}

pub(crate) fn language_from_locale(locale: &str) -> Language {
    if locale.trim().to_ascii_lowercase().starts_with("zh") {
        Language::Zh
    } else {
        Language::En
    }
}

#[cfg(windows)]
pub(crate) fn system_language() -> Option<Language> {
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
pub(crate) fn system_language() -> Option<Language> {
    None
}

pub(crate) fn env_locale() -> Option<String> {
    ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .find_map(|key| {
            std::env::var(key)
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct TodoItem {
    pub(crate) id: Uuid,
    pub(crate) title: String,
    pub(crate) due_at: NaiveDateTime,
    #[serde(default)]
    pub(crate) notes: String,
    #[serde(default)]
    pub(crate) is_done: bool,
    pub(crate) completed_at: Option<NaiveDateTime>,
    #[serde(default = "default_reminders")]
    pub(crate) reminder_minutes: Vec<i32>,
    pub(crate) recurrence: Option<RecurrenceRule>,
    #[serde(default)]
    pub(crate) completions: Vec<TodoCompletion>,
}

impl TodoItem {
    pub(crate) fn is_recurring(&self) -> bool {
        matches!(
            self.recurrence.as_ref().map(|rule| rule.kind),
            Some(RecurrenceKind::Weekly | RecurrenceKind::Monthly | RecurrenceKind::Daily)
        )
    }

    pub(crate) fn normalized(mut self) -> Self {
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
pub(crate) struct RecurrenceRule {
    pub(crate) kind: RecurrenceKind,
    #[serde(default)]
    pub(crate) weekdays: Vec<u32>,
    #[serde(default = "default_day_of_month")]
    pub(crate) day_of_month: i32,
    #[serde(default)]
    pub(crate) monthly_kind: MonthlyKind,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecurrenceKind {
    Weekly,
    Monthly,
    Daily,
}

impl RecurrenceKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Daily => "daily",
        }
    }

    pub(crate) fn from_value(value: &str) -> Self {
        match value {
            "monthly" => Self::Monthly,
            "daily" => Self::Daily,
            _ => Self::Weekly,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MonthlyKind {
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
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::DayOfMonth => "day",
            Self::LastWorkday => "last_workday",
            Self::LastDay => "last_day",
        }
    }

    pub(crate) fn from_value(value: &str) -> Self {
        match value {
            "last_workday" => Self::LastWorkday,
            "last_day" => Self::LastDay,
            _ => Self::DayOfMonth,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct TodoCompletion {
    pub(crate) occurrence_key: String,
    pub(crate) due_at: NaiveDateTime,
    pub(crate) completed_at: NaiveDateTime,
}

#[derive(Clone, PartialEq)]
pub(crate) struct TodoOccurrence {
    pub(crate) source: TodoItem,
    pub(crate) due_at: NaiveDateTime,
    pub(crate) occurrence_key: String,
    pub(crate) is_done: bool,
    pub(crate) completed_at: Option<NaiveDateTime>,
}

#[derive(Clone, PartialEq)]
pub(crate) struct TodoEditor {
    pub(crate) editing_id: Option<Uuid>,
    pub(crate) title: String,
    pub(crate) due_date_enabled: bool,
    pub(crate) due_date: String,
    pub(crate) due_time_enabled: bool,
    pub(crate) due_time: String,
    pub(crate) reminders_enabled: bool,
    pub(crate) notes: String,
    pub(crate) was_done: bool,
    pub(crate) completed_at: Option<NaiveDateTime>,
    pub(crate) reminders: Vec<i32>,
    pub(crate) recurring: bool,
    pub(crate) recurrence_kind: RecurrenceKind,
    pub(crate) weekdays: Vec<u32>,
    pub(crate) monthly_kind: MonthlyKind,
    pub(crate) day_of_month: String,
    pub(crate) new_reminder: String,
    pub(crate) validation: String,
}

impl TodoEditor {
    pub(crate) fn new(todo: Option<&TodoItem>, default_reminders: &[i32]) -> Self {
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

        let editor_time_enabled = todo.map(|_| time_enabled).unwrap_or(false);

        Self {
            editing_id: todo.map(|item| item.id),
            title: todo.map(|item| item.title.clone()).unwrap_or_default(),
            due_date_enabled: todo.map(|_| date_enabled).unwrap_or(true),
            due_date: if date_enabled {
                due_at.date().format("%Y-%m-%d").to_string()
            } else {
                String::new()
            },
            due_time_enabled: editor_time_enabled,
            due_time: if editor_time_enabled {
                due_at.time().format("%H:%M").to_string()
            } else {
                String::new()
            },
            reminders_enabled: todo
                .map(|item| time_enabled && !item.reminder_minutes.is_empty())
                .unwrap_or(false),
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

    pub(crate) fn due_at(&self) -> Option<NaiveDateTime> {
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
pub(crate) enum Language {
    En,
    Zh,
}
