#![allow(unused_imports)]

use crate::config::{
    REMINDER_CATCH_UP_SECONDS, REMINDER_MIN_DELAY_SECONDS, REMINDER_THREAD_GENERATION,
    TOAST_APP_ID, TOAST_REMINDER_GROUP,
};
use crate::models::*;
use crate::platform::wide_null;
use crate::todo_logic::*;
use chrono::{Duration, Local, NaiveDateTime};
use dioxus::prelude::*;
use std::sync::atomic::Ordering;
use std::time::Duration as StdDuration;
use winrt_toast_reborn::{
    register as register_toast_app, Scenario, Toast, ToastDuration, ToastManager,
};

pub(crate) struct ReminderNotification {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) due_at: NaiveDateTime,
    pub(crate) notify_at: NaiveDateTime,
}

pub(crate) fn reschedule_reminders(
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

pub(crate) fn scheduled_reminders(
    state: &AppState,
    now: NaiveDateTime,
) -> Vec<ReminderNotification> {
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

pub(crate) fn reminder_id(occurrence: &TodoOccurrence, minutes: i32) -> String {
    format!("{}:{minutes}", occurrence_dom_key(occurrence))
}

pub(crate) fn reminder_is_due(
    due_at: NaiveDateTime,
    minutes_before: i32,
    now: NaiveDateTime,
) -> bool {
    let reminder_at = due_at - Duration::minutes(minutes_before as i64);
    reminder_can_be_scheduled(due_at, reminder_at, now)
}

pub(crate) fn reminder_can_be_scheduled(
    _due_at: NaiveDateTime,
    notify_at: NaiveDateTime,
    now: NaiveDateTime,
) -> bool {
    notify_at > now
        || (now >= notify_at && now < notify_at + Duration::seconds(REMINDER_CATCH_UP_SECONDS))
}

pub(crate) fn is_overdue(occurrence: &TodoOccurrence, now: NaiveDateTime) -> bool {
    !occurrence.is_done && !has_unspecified_time(occurrence.due_at) && occurrence.due_at <= now
}

pub(crate) fn show_system_notification(reminder: &ReminderNotification, language: Language) {
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

pub(crate) fn schedule_system_notification(
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

pub(crate) fn reminder_delay(notify_at: NaiveDateTime, now: NaiveDateTime) -> StdDuration {
    (notify_at - now)
        .to_std()
        .unwrap_or_else(|_| StdDuration::from_secs(REMINDER_MIN_DELAY_SECONDS))
}

pub(crate) fn reminder_remaining_text(
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

pub(crate) fn register_system_notifications() {
    if let Err(error) = set_current_process_app_id() {
        eprintln!("Failed to set notification app id: {error}");
    }
    if let Err(error) = register_toast_app(TOAST_APP_ID, "So Todo", None) {
        eprintln!("Failed to register notification app: {error}");
    }
}

#[cfg(windows)]
pub(crate) fn set_current_process_app_id() -> Result<(), i32> {
    use windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

    let app_id = wide_null(TOAST_APP_ID);
    let result = unsafe { SetCurrentProcessExplicitAppUserModelID(app_id.as_ptr()) };
    (result == 0).then_some(()).ok_or(result)
}

#[cfg(not(windows))]
pub(crate) fn set_current_process_app_id() -> Result<(), i32> {
    Ok(())
}
