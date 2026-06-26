use crate::actions::*;
use crate::i18n::*;
use crate::models::*;
use crate::platform::*;
use crate::reminders::*;
use crate::storage::*;
use crate::todo_logic::*;
use crate::update::*;
use chrono::{Duration, Local, NaiveDate, NaiveTime, Timelike};
use dioxus::prelude::Key;
use rusqlite::Connection;
use uuid::Uuid;

#[test]
fn hotkey_parser_accepts_default_and_function_keys() {
    let default = parse_hotkey(&default_hotkey()).unwrap();
    assert!(default.ctrl);
    assert!(default.alt);
    assert!(!default.shift);
    assert!(!default.win);
    assert_eq!(default.key, 'X' as u32);

    let f12 = parse_hotkey("Ctrl+Shift+F12").unwrap();
    assert!(f12.ctrl);
    assert!(!f12.alt);
    assert!(f12.shift);
    assert_eq!(f12.key, 0x7B);

    assert_eq!(
        hotkey_from_key(&Key::Character("x".into()), true, true, false, false),
        Some("Ctrl+Alt+X".into())
    );
    assert_eq!(
        hotkey_from_key(&Key::F12, true, false, true, false),
        Some("Ctrl+Shift+F12".into())
    );
    assert_eq!(parse_hotkey("Ctrl+Alt+Enter").unwrap().key, 0x0D);

    assert!(parse_hotkey("X").is_none());
    assert!(parse_hotkey("Ctrl+Alt+Mouse").is_none());
}

#[test]
fn update_check_compares_normalized_versions_and_extracts_tag() {
    assert!(!version_has_update("v0.0.3", "0.0.3"));
    assert!(version_has_update("develop", "v0.0.3"));
    assert!(version_has_update("v0.0.2", "v0.0.3"));
    assert_eq!(
        extract_json_string(r#"{"name":"SoTodo","tag_name":"v0.0.3"}"#, "tag_name"),
        Some("v0.0.3".into())
    );
}

#[test]
fn settings_theme_previews_from_editor_without_saving() {
    let today = Local::now().date_naive();
    let mut state = AppState {
        todos: Vec::new(),
        settings: Settings {
            theme: "light".into(),
            ..Settings::default()
        },
        settings_editor: Settings {
            theme: "dark".into(),
            ..Settings::default()
        },
        mode: ViewMode::List,
        query: String::new(),
        visible_month: first_of_month(today),
        selected_date: today,
        collapsed_days: Vec::new(),
        pending_completed_occurrences: Vec::new(),
        top_most: false,
        dialog: DialogMode::Settings,
        editor: TodoEditor::new(None, &[15]),
        settings_hotkey_recording: false,
        pending_delete_id: None,
        new_default_reminder: String::new(),
        reminder_generation: 0,
        delivered_reminder_ids: Vec::new(),
    };

    assert_eq!(active_app_theme(&state), "dark");
    assert_eq!(state.settings.effective_theme(), "light");

    state.dialog = DialogMode::None;
    assert_eq!(active_app_theme(&state), "light");
}

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
fn daily_repeat_generates_each_day() {
    let todo = TodoItem {
        id: Uuid::nil(),
        title: "journal".into(),
        due_at: NaiveDate::from_ymd_opt(2024, 9, 2)
            .unwrap()
            .and_hms_opt(9, 30, 0)
            .unwrap(),
        notes: String::new(),
        is_done: false,
        completed_at: None,
        reminder_minutes: vec![15],
        recurrence: Some(RecurrenceRule {
            kind: RecurrenceKind::Daily,
            weekdays: Vec::new(),
            day_of_month: 1,
            monthly_kind: MonthlyKind::DayOfMonth,
        }),
        completions: Vec::new(),
    };

    let dates = due_dates_between(
        &todo,
        NaiveDate::from_ymd_opt(2024, 9, 2)
            .unwrap()
            .and_time(NaiveTime::MIN),
        NaiveDate::from_ymd_opt(2024, 9, 4)
            .unwrap()
            .and_hms_nano_opt(23, 59, 59, 999_999_999)
            .unwrap(),
    );

    assert_eq!(
        dates
            .into_iter()
            .map(|value| value.date())
            .collect::<Vec<_>>(),
        vec![
            NaiveDate::from_ymd_opt(2024, 9, 2).unwrap(),
            NaiveDate::from_ymd_opt(2024, 9, 3).unwrap(),
            NaiveDate::from_ymd_opt(2024, 9, 4).unwrap(),
        ]
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
        settings_editor: Settings::default(),
        settings_hotkey_recording: false,
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
        settings_editor: Settings::default(),
        settings_hotkey_recording: false,
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
        settings_editor: Settings::default(),
        settings_hotkey_recording: false,
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
    assert!(scheduled_reminders(&only_unscheduled, scheduled - Duration::minutes(1)).is_empty());
    assert!(!is_overdue(
        &single_occurrence(&unscheduled),
        Local::now().naive_local()
    ));
}

#[test]
fn unspecified_date_and_time_labels_are_distinct() {
    let text = Strings::new(Language::Zh);
    assert_eq!(
        text.unspecified_date,
        "\u{672a}\u{6307}\u{5b9a}\u{65e5}\u{671f}"
    );
    assert_eq!(
        text.unspecified_time,
        "\u{672a}\u{6307}\u{5b9a}\u{65f6}\u{95f4}"
    );
    assert_eq!(
        format_group_date(unscheduled_due_at().date(), Language::Zh),
        text.unspecified_date
    );
}

#[test]
fn unspecified_due_time_tag_uses_placeholder() {
    let today = Local::now().date_naive();
    assert_eq!(todo_time_tag(unscheduled_due_at()), "--:--");
    assert_eq!(todo_time_tag(today.and_time(NaiveTime::MIN)), "--:--");
    assert_eq!(todo_time_tag(today.and_hms_opt(9, 30, 0).unwrap()), "09:30");
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
fn new_todo_starts_without_time_and_enables_default_reminders_with_time() {
    let mut editor = TodoEditor::new(None, &[30, 5]);
    assert!(editor.due_date_enabled);
    assert!(!editor.due_time_enabled);
    assert!(editor.due_time.is_empty());
    assert!(!editor.reminders_enabled);

    set_editor_time_enabled(&mut editor, true, &[30, 5]);

    assert!(editor.due_time_enabled);
    assert!(editor.reminders_enabled);
    assert_eq!(editor.reminders, vec![30, 5]);
    let time = NaiveTime::parse_from_str(&editor.due_time, "%H:%M").unwrap();
    assert_eq!(time.minute() % 5, 0);
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
        settings_editor: Settings::default(),
        settings_hotkey_recording: false,
        pending_delete_id: None,
        new_default_reminder: String::new(),
        reminder_generation: 0,
        delivered_reminder_ids: Vec::new(),
    };

    assert_eq!(open_occurrences(&state).len(), 1);
    state.pending_completed_occurrences.clear();
    assert!(open_occurrences(&state).is_empty());
}
