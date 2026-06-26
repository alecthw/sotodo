#![allow(unused_imports)]

use crate::i18n::Strings;
use crate::models::*;
use crate::platform::hide_main_window;
use crate::reminders::reschedule_reminders;
use crate::storage::save_store;
use crate::todo_logic::*;
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use dioxus::prelude::*;
use std::time::Duration as StdDuration;
use uuid::Uuid;

pub(crate) fn mutate(app: Signal<AppState>, action: impl FnOnce(&mut AppState)) {
    let mut app = app;
    app.with_mut(action);
}

pub(crate) fn active_app_theme(state: &AppState) -> String {
    if state.dialog == DialogMode::Settings {
        state.settings_editor.effective_theme()
    } else {
        state.settings.effective_theme()
    }
}

pub(crate) fn handle_todo_dialog_key(
    event: KeyboardEvent,
    app: Signal<AppState>,
    clock: Signal<NaiveDateTime>,
) {
    match event.key() {
        Key::Escape => {
            event.prevent_default();
            close_dialog(app);
        }
        Key::Enter => {
            event.prevent_default();
            save_todo(app, clock);
        }
        _ => {}
    }
}

pub(crate) fn handle_settings_dialog_key(event: KeyboardEvent, app: Signal<AppState>) {
    match event.key() {
        Key::Escape => {
            event.prevent_default();
            close_dialog(app);
        }
        Key::Enter => {
            event.prevent_default();
            apply_settings(app);
        }
        _ => {}
    }
}

pub(crate) fn capture_hotkey_keydown(event: KeyboardEvent, app: Signal<AppState>) {
    if !app().settings_hotkey_recording {
        if is_keyboard_activation_key(&event.key()) {
            event.prevent_default();
            event.stop_propagation();
            mutate(app, |state| state.settings_hotkey_recording = true);
        }
        return;
    }

    event.prevent_default();
    event.stop_propagation();

    if event.key() == Key::Escape {
        mutate(app, |state| state.settings_hotkey_recording = false);
        return;
    }

    let modifiers = event.modifiers();
    if let Some(value) = hotkey_from_key(
        &event.key(),
        modifiers.ctrl(),
        modifiers.alt(),
        modifiers.shift(),
        modifiers.meta(),
    ) {
        mutate(app, |state| {
            state.settings_editor.hotkey = value;
            state.settings_hotkey_recording = false;
        });
    }
}

pub(crate) fn is_keyboard_activation_key(key: &Key) -> bool {
    matches!(key, Key::Enter) || matches!(key, Key::Character(value) if value == " ")
}

pub(crate) fn show_editor(app: Signal<AppState>, todo: Option<TodoItem>) {
    mutate(app, |state| {
        state.editor = TodoEditor::new(todo.as_ref(), &state.settings.default_reminder_minutes);
        state.dialog = DialogMode::Todo;
    });
}

pub(crate) fn show_settings(app: Signal<AppState>) {
    mutate(app, |state| {
        state.settings_editor = state.settings.clone();
        state.settings_hotkey_recording = false;
        state.new_default_reminder.clear();
        state.dialog = DialogMode::Settings;
    });
}

pub(crate) fn close_dialog(app: Signal<AppState>) {
    mutate(app, |state| {
        state.dialog = DialogMode::None;
        state.pending_delete_id = None;
        state.settings_hotkey_recording = false;
        state.new_default_reminder.clear();
    });
}

pub(crate) fn apply_settings(app: Signal<AppState>) {
    mutate(app, |state| {
        let mut settings = state.settings_editor.clone();
        settings.default_reminder_minutes = normalize_reminders(&settings.default_reminder_minutes);
        if parse_hotkey(&settings.hotkey).is_none() {
            settings.hotkey = default_hotkey();
        }
        state.settings = settings;
        state.settings_editor = state.settings.clone();
        state.settings_hotkey_recording = false;
        state.dialog = DialogMode::None;
        state.new_default_reminder.clear();
        state.save();
    });
}

pub(crate) fn request_close(app: Signal<AppState>) {
    let state = app();
    match state.settings.close_behavior.as_str() {
        "tray" if state.settings.tray_enabled => hide_main_window(),
        "prompt" => mutate(app, |state| state.dialog = DialogMode::CloseConfirm),
        _ => dioxus::desktop::window().close(),
    }
}

pub(crate) fn toggle_top_most(app: Signal<AppState>) {
    mutate(app, |state| {
        state.top_most = !state.top_most;
        dioxus::desktop::window()
            .window
            .set_always_on_top(state.top_most);
    });
}

pub(crate) fn request_delete(app: Signal<AppState>, id: Uuid) {
    mutate(app, |state| {
        state.pending_delete_id = Some(id);
        state.dialog = DialogMode::DeleteConfirm;
    });
}

pub(crate) fn confirm_delete(app: Signal<AppState>, clock: Signal<NaiveDateTime>) {
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

pub(crate) fn save_todo(app: Signal<AppState>, clock: Signal<NaiveDateTime>) {
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

pub(crate) fn toggle_done(
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

pub(crate) fn collapse_all(app: Signal<AppState>) {
    mutate(app, |state| {
        state.collapsed_days = grouped_occurrences(state, state.mode == ViewMode::Completed)
            .into_iter()
            .map(|(day, _)| day)
            .collect();
    });
}

pub(crate) fn toggle_day(app: Signal<AppState>, day: NaiveDate) {
    mutate(app, |state| {
        if let Some(index) = state.collapsed_days.iter().position(|value| *value == day) {
            state.collapsed_days.remove(index);
        } else {
            state.collapsed_days.push(day);
        }
    });
}

pub(crate) fn add_editor_reminder(editor: &mut TodoEditor) {
    if let Some(value) = parse_i32(&editor.new_reminder) {
        add_reminder_value(&mut editor.reminders, value);
        editor.new_reminder.clear();
    }
}

pub(crate) fn set_editor_date_enabled(editor: &mut TodoEditor, enabled: bool) {
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

pub(crate) fn set_editor_time_enabled(
    editor: &mut TodoEditor,
    enabled: bool,
    default_reminders: &[i32],
) {
    editor.due_time_enabled = enabled;
    if enabled {
        editor.due_time = default_due_at().format("%H:%M").to_string();
        editor.reminders_enabled = true;
        if editor.reminders.is_empty() {
            editor.reminders = default_reminders.to_vec();
        }
    } else {
        editor.due_time.clear();
        editor.reminders_enabled = false;
        editor.reminders.clear();
    }
}

pub(crate) fn set_editor_time_hour(editor: &mut TodoEditor, hour: &str) {
    let (_, minute) = time_select_parts(&editor.due_time);
    editor.due_time = format!("{}:{minute}", hour.clamp("00", "23"));
}

pub(crate) fn set_editor_time_minute(editor: &mut TodoEditor, minute: &str) {
    let (hour, _) = time_select_parts(&editor.due_time);
    editor.due_time = format!("{hour}:{}", minute.clamp("00", "55"));
}

pub(crate) fn add_default_reminder(app: Signal<AppState>) {
    mutate(app, |state| {
        if let Some(value) = parse_i32(&state.new_default_reminder) {
            add_reminder_value(&mut state.settings_editor.default_reminder_minutes, value);
            state.new_default_reminder.clear();
        }
    });
}

pub(crate) fn add_reminder_value(reminders: &mut Vec<i32>, value: i32) {
    if (0..=10080).contains(&value) && !reminders.contains(&value) {
        reminders.push(value);
        reminders.sort_by(|left, right| right.cmp(left));
    }
}

pub(crate) fn set_weekday(editor: &mut TodoEditor, day: u32) {
    editor.weekdays = vec![day];
}

pub(crate) fn sync_editor_repeat_defaults(editor: &mut TodoEditor) {
    if let Some(due_at) = editor.due_at() {
        if editor.weekdays.is_empty() {
            editor.weekdays.push(weekday_index(due_at.weekday()));
        }
        if parse_i32(&editor.day_of_month).is_none() {
            editor.day_of_month = due_at.day().to_string();
        }
    }
}
