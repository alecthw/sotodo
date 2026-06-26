#![allow(unused_imports)]

use crate::config::UNSCHEDULED_DATE;
use crate::models::*;
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday};
use dioxus::prelude::Key;
use std::collections::BTreeMap;

pub(crate) fn grouped_occurrences(
    state: &AppState,
    completed: bool,
) -> Vec<(NaiveDate, Vec<TodoOccurrence>)> {
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

pub(crate) fn occurrence_group_date(occurrence: &TodoOccurrence) -> NaiveDate {
    occurrence.due_at.date()
}

pub(crate) fn open_occurrences(state: &AppState) -> Vec<TodoOccurrence> {
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

pub(crate) fn completed_occurrences(todos: &[TodoItem]) -> Vec<TodoOccurrence> {
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

pub(crate) fn calendar_occurrences(todos: &[TodoItem], month: NaiveDate) -> Vec<TodoOccurrence> {
    let start = month - Duration::days(month.weekday().num_days_from_sunday() as i64);
    let end = start + Duration::days(42) - Duration::nanoseconds(1);
    occurrences_between(todos, start.and_time(NaiveTime::MIN), end_of_day(end))
        .into_iter()
        .filter(|occurrence| !is_unscheduled_due(occurrence.due_at))
        .collect()
}

pub(crate) fn occurrences_between(
    todos: &[TodoItem],
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> Vec<TodoOccurrence> {
    todos
        .iter()
        .flat_map(|todo| todo_occurrences_between(todo, from, to))
        .collect()
}

pub(crate) fn current_open_occurrence(
    todo: &TodoItem,
    now: NaiveDateTime,
) -> Option<TodoOccurrence> {
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

pub(crate) fn todo_occurrences_between(
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

pub(crate) fn due_dates_between(
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
        RecurrenceKind::Daily => daily_due_dates(todo, from, to),
    }
}

pub(crate) fn daily_due_dates(
    todo: &TodoItem,
    from: NaiveDateTime,
    to: NaiveDateTime,
) -> Vec<NaiveDateTime> {
    let mut dates = Vec::new();
    let mut date = from.date().max(todo.due_at.date());
    while date <= to.date() {
        let due_at = date.and_time(todo.due_at.time());
        if due_at >= todo.due_at && due_at >= from && due_at <= to {
            dates.push(due_at);
        }
        date += Duration::days(1);
    }
    dates
}

pub(crate) fn weekly_due_dates(
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

pub(crate) fn monthly_due_dates(
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

pub(crate) fn monthly_date(year: i32, month: u32, rule: &RecurrenceRule) -> NaiveDate {
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

pub(crate) fn single_occurrence(todo: &TodoItem) -> TodoOccurrence {
    TodoOccurrence {
        source: todo.clone(),
        due_at: todo.due_at,
        occurrence_key: occurrence_key(todo.due_at),
        is_done: todo.is_done,
        completed_at: todo.completed_at,
    }
}

pub(crate) fn to_occurrence(todo: &TodoItem, due_at: NaiveDateTime) -> TodoOccurrence {
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

pub(crate) fn occurrence_key(due_at: NaiveDateTime) -> String {
    due_at.format("%Y%m%d%H%M").to_string()
}

pub(crate) fn occurrence_dom_key(occurrence: &TodoOccurrence) -> String {
    format!("{}:{}", occurrence.source.id, occurrence.occurrence_key)
}

pub(crate) fn normalize_reminders(values: &[i32]) -> Vec<i32> {
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

pub(crate) fn calendar_days(month: NaiveDate) -> Vec<NaiveDate> {
    let start = month - Duration::days(month.weekday().num_days_from_sunday() as i64);
    (0..42).map(|index| start + Duration::days(index)).collect()
}

pub(crate) fn first_of_month(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap()
}

pub(crate) fn add_months(date: NaiveDate, months: i32) -> NaiveDate {
    let month_index = date.year() * 12 + date.month0() as i32 + months;
    let year = month_index.div_euclid(12);
    let month = month_index.rem_euclid(12) as u32 + 1;
    NaiveDate::from_ymd_opt(year, month, 1).unwrap()
}

pub(crate) fn days_in_month(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };
    (next - Duration::days(1)).day()
}

pub(crate) fn weekday_index(day: Weekday) -> u32 {
    day.num_days_from_sunday()
}

pub(crate) fn weekday_names(language: Language) -> Vec<String> {
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

pub(crate) fn format_date(date: NaiveDate, language: Language) -> String {
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

pub(crate) fn format_group_date(date: NaiveDate, language: Language) -> String {
    if is_unscheduled_date(date) {
        return match language {
            Language::Zh => "\u{672a}\u{6307}\u{5b9a}\u{65e5}\u{671f}".into(),
            Language::En => "Unspecified date".into(),
        };
    }
    format_date(date, language)
}

pub(crate) fn month_title(date: NaiveDate, language: Language) -> String {
    match language {
        Language::Zh => date.format("%Y\u{5e74}%m\u{6708}").to_string(),
        Language::En => date.format("%Y-%m").to_string(),
    }
}

pub(crate) fn format_title_datetime(value: NaiveDateTime, language: Language) -> String {
    match language {
        Language::Zh => value
            .format("%Y\u{5e74}%m\u{6708}%d\u{65e5} %H:%M")
            .to_string(),
        Language::En => value.format("%m/%d/%Y %H:%M").to_string(),
    }
}

pub(crate) fn todo_time(due_at: NaiveDateTime) -> String {
    format!("{:02}:{:02}", due_at.hour(), due_at.minute())
}

pub(crate) fn todo_time_tag(due_at: NaiveDateTime) -> String {
    if has_unspecified_time(due_at) {
        "--:--".into()
    } else {
        todo_time(due_at)
    }
}

pub(crate) fn default_due_at() -> NaiveDateTime {
    ceil_datetime_to_five_minutes(Local::now().naive_local() + Duration::hours(1))
}

pub(crate) fn time_select_parts(value: &str) -> (String, String) {
    let time = NaiveTime::parse_from_str(value, "%H:%M")
        .map(snap_time_to_five_minutes)
        .unwrap_or(NaiveTime::MIN);
    (two_digits(time.hour()), two_digits(time.minute()))
}

pub(crate) fn snap_time_to_five_minutes(time: NaiveTime) -> NaiveTime {
    time.with_minute(time.minute() / 5 * 5)
        .and_then(|time| time.with_second(0))
        .unwrap_or(time)
}

pub(crate) fn ceil_datetime_to_five_minutes(value: NaiveDateTime) -> NaiveDateTime {
    let rounded = round_to_minute(value);
    let extra = (5 - rounded.minute() % 5) % 5;
    rounded + Duration::minutes(extra as i64)
}

pub(crate) fn two_digits(value: u32) -> String {
    format!("{value:02}")
}

pub(crate) fn unscheduled_due_at() -> NaiveDateTime {
    // ponytail: keep SQLite due_at NOT NULL; migrate to nullable if recurrence needs no-date tasks.
    NaiveDate::from_ymd_opt(UNSCHEDULED_DATE.0, UNSCHEDULED_DATE.1, UNSCHEDULED_DATE.2)
        .unwrap()
        .and_hms_opt(23, 59, 0)
        .unwrap()
}

pub(crate) fn is_unscheduled_due(due_at: NaiveDateTime) -> bool {
    is_unscheduled_date(due_at.date())
}

pub(crate) fn has_unspecified_time(due_at: NaiveDateTime) -> bool {
    is_unscheduled_due(due_at) || due_at.time() == NaiveTime::MIN
}

pub(crate) fn has_specific_time(due_at: NaiveDateTime) -> bool {
    !has_unspecified_time(due_at)
}

pub(crate) fn is_unscheduled_date(date: NaiveDate) -> bool {
    date == NaiveDate::from_ymd_opt(UNSCHEDULED_DATE.0, UNSCHEDULED_DATE.1, UNSCHEDULED_DATE.2)
        .unwrap()
}

pub(crate) fn round_to_minute(value: NaiveDateTime) -> NaiveDateTime {
    value
        .date()
        .and_hms_opt(value.hour(), value.minute(), 0)
        .unwrap()
}

pub(crate) fn end_of_day(date: NaiveDate) -> NaiveDateTime {
    date.and_hms_nano_opt(23, 59, 59, 999_999_999).unwrap()
}

pub(crate) fn tab_class(active: bool) -> &'static str {
    if active {
        "tab tab-active gap-2 text-base font-bold"
    } else {
        "tab gap-2 text-base font-bold"
    }
}

pub(crate) fn todo_notes_class(expanded: bool) -> &'static str {
    if expanded {
        "todo-notes mt-1 block cursor-pointer text-sm opacity-70"
    } else {
        "todo-notes todo-notes-collapsed mt-1 block cursor-pointer text-sm opacity-70"
    }
}

pub(crate) fn calendar_day_class(selected: bool, other_month: bool) -> &'static str {
    match (selected, other_month) {
        (true, _) => "btn btn-primary h-9 min-h-0 flex-col gap-0.5 p-1 text-xs",
        (false, true) => "btn btn-ghost h-9 min-h-0 flex-col gap-0.5 p-1 text-xs opacity-40",
        (false, false) => "btn btn-ghost h-9 min-h-0 flex-col gap-0.5 p-1 text-xs",
    }
}

pub(crate) fn calendar_dot_class(selected: bool) -> &'static str {
    if selected {
        "h-1.5 w-1.5 rounded-full bg-primary-content"
    } else {
        "h-1.5 w-1.5 rounded-full bg-primary"
    }
}

pub(crate) fn parse_i32(value: &str) -> Option<i32> {
    value.trim().parse().ok()
}

pub(crate) fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

pub(crate) fn bool_setting(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

pub(crate) fn default_language() -> String {
    "system".into()
}

pub(crate) fn default_theme() -> String {
    "system".into()
}

pub(crate) fn default_tray_enabled() -> bool {
    true
}

pub(crate) fn default_startup_enabled() -> bool {
    false
}

pub(crate) fn default_hotkey_enabled() -> bool {
    true
}

pub(crate) fn default_close_behavior() -> String {
    "prompt".into()
}

pub(crate) fn default_hotkey() -> String {
    "Ctrl+Alt+X".into()
}

pub(crate) fn hotkey_from_key(
    key: &Key,
    ctrl: bool,
    alt: bool,
    shift: bool,
    win: bool,
) -> Option<String> {
    if !ctrl && !alt && !shift && !win {
        return None;
    }

    let key = hotkey_key_label(key)?;
    let mut parts = Vec::new();
    if ctrl {
        parts.push("Ctrl".to_string());
    }
    if alt {
        parts.push("Alt".to_string());
    }
    if shift {
        parts.push("Shift".to_string());
    }
    if win {
        parts.push("Win".to_string());
    }
    parts.push(key);
    Some(parts.join("+"))
}

pub(crate) fn hotkey_key_label(key: &Key) -> Option<String> {
    match key {
        Key::Character(value) => {
            let mut chars = value.chars();
            let chr = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            if chr == ' ' {
                return Some("Space".into());
            }
            chr.is_ascii_alphanumeric()
                .then(|| chr.to_ascii_uppercase().to_string())
        }
        Key::Control | Key::Alt | Key::Shift | Key::Meta | Key::Super => None,
        _ => {
            let value = key.to_string();
            parse_hotkey_key(&value).is_some().then_some(value)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HotkeySpec {
    pub(crate) ctrl: bool,
    pub(crate) alt: bool,
    pub(crate) shift: bool,
    pub(crate) win: bool,
    pub(crate) key: u32,
}

pub(crate) fn parse_hotkey(value: &str) -> Option<HotkeySpec> {
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut win = false;
    let mut key = None;

    for part in value
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => ctrl = true,
            "alt" => alt = true,
            "shift" => shift = true,
            "win" | "windows" | "meta" | "super" => win = true,
            _ if key.is_none() => key = parse_hotkey_key(part),
            _ => return None,
        }
    }

    let key = key?;
    (ctrl || alt || shift || win).then_some(HotkeySpec {
        ctrl,
        alt,
        shift,
        win,
        key,
    })
}

pub(crate) fn parse_hotkey_key(value: &str) -> Option<u32> {
    let upper = value.trim().to_ascii_uppercase();
    if upper.len() == 1 {
        let byte = upper.as_bytes()[0];
        if byte.is_ascii_alphanumeric() {
            return Some(byte as u32);
        }
    }

    if let Some(number) = upper
        .strip_prefix('F')
        .and_then(|value| value.parse::<u32>().ok())
    {
        if (1..=24).contains(&number) {
            return Some(0x70 + number - 1);
        }
    }

    match upper.as_str() {
        "SPACE" => Some(0x20),
        "ENTER" => Some(0x0D),
        "ESC" | "ESCAPE" => Some(0x1B),
        "TAB" => Some(0x09),
        "BACKSPACE" => Some(0x08),
        "DELETE" => Some(0x2E),
        "INSERT" => Some(0x2D),
        "HOME" => Some(0x24),
        "END" => Some(0x23),
        "PAGEUP" => Some(0x21),
        "PAGEDOWN" => Some(0x22),
        "ARROWUP" | "UP" => Some(0x26),
        "ARROWDOWN" | "DOWN" => Some(0x28),
        "ARROWLEFT" | "LEFT" => Some(0x25),
        "ARROWRIGHT" | "RIGHT" => Some(0x27),
        _ => None,
    }
}

pub(crate) fn default_reminders() -> Vec<i32> {
    vec![15, 5]
}

pub(crate) fn default_day_of_month() -> i32 {
    1
}
