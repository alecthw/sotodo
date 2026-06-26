#![allow(unused_imports)]

use crate::config::THEMES;
use crate::models::*;
use crate::platform::startup_enabled_from_registry;
use crate::todo_logic::*;
use chrono::{Local, NaiveDateTime};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::{fs, path::PathBuf};
use uuid::Uuid;

pub(crate) fn load_store() -> Store {
    open_database()
        .and_then(|connection| load_store_from_db(&connection))
        .unwrap_or_default()
}

pub(crate) fn save_store(store: &Store) {
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

pub(crate) fn open_database() -> rusqlite::Result<Connection> {
    let path = data_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let connection = Connection::open(path)?;
    ensure_schema(&connection)?;
    Ok(connection)
}

pub(crate) fn ensure_schema(connection: &Connection) -> rusqlite::Result<()> {
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

pub(crate) fn load_store_from_db(connection: &Connection) -> rusqlite::Result<Store> {
    Ok(Store {
        todos: load_todos(connection)?,
        settings: load_settings(connection)?,
    })
}

pub(crate) fn load_settings(connection: &Connection) -> rusqlite::Result<Settings> {
    let mut settings = Settings {
        language: load_setting(connection, "language")?.unwrap_or_else(default_language),
        theme: load_setting(connection, "theme")?.unwrap_or_else(default_theme),
        close_behavior: load_setting(connection, "close_behavior")?
            .unwrap_or_else(default_close_behavior),
        tray_enabled: load_bool_setting(connection, "tray_enabled")?
            .unwrap_or_else(default_tray_enabled),
        startup_enabled: load_bool_setting(connection, "startup_enabled")?
            .unwrap_or_else(startup_enabled_from_registry),
        hotkey_enabled: load_bool_setting(connection, "hotkey_enabled")?
            .unwrap_or_else(default_hotkey_enabled),
        hotkey: load_setting(connection, "hotkey")?.unwrap_or_else(default_hotkey),
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
    if parse_hotkey(&settings.hotkey).is_none() {
        settings.hotkey = default_hotkey();
    }
    Ok(settings)
}

pub(crate) fn load_bool_setting(
    connection: &Connection,
    key: &str,
) -> rusqlite::Result<Option<bool>> {
    Ok(load_setting(connection, key)?.and_then(|value| parse_bool(&value)))
}

pub(crate) fn load_setting(connection: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    connection
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()
}

pub(crate) fn load_default_reminders(connection: &Connection) -> rusqlite::Result<Vec<i32>> {
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

pub(crate) fn load_todos(connection: &Connection) -> rusqlite::Result<Vec<TodoItem>> {
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

pub(crate) fn load_todo_reminders(
    connection: &Connection,
    todo_id: Uuid,
) -> rusqlite::Result<Vec<i32>> {
    let mut statement = connection
        .prepare("SELECT minutes FROM todo_reminders WHERE todo_id = ?1 ORDER BY position")?;
    let values = statement
        .query_map([todo_id.to_string()], |row| row.get(0))?
        .collect();
    values
}

pub(crate) fn load_recurrence(
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

pub(crate) fn load_completions(
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

pub(crate) fn save_store_to_db(
    transaction: &Transaction<'_>,
    store: &Store,
) -> rusqlite::Result<()> {
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
    save_setting(
        transaction,
        "hotkey_enabled",
        bool_setting(store.settings.hotkey_enabled),
    )?;
    save_setting(transaction, "hotkey", &store.settings.hotkey)?;

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

pub(crate) fn save_setting(
    transaction: &Transaction<'_>,
    key: &str,
    value: &str,
) -> rusqlite::Result<()> {
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

pub(crate) fn save_todo_to_db(
    transaction: &Transaction<'_>,
    todo: &TodoItem,
) -> rusqlite::Result<()> {
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

pub(crate) fn data_path() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".sotodo")
        .join("sotodo.db")
}

pub(crate) fn datetime_text(value: NaiveDateTime) -> String {
    value.format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub(crate) fn parse_datetime(value: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S"))
        .unwrap_or_else(|_| round_to_minute(Local::now().naive_local()))
}

pub(crate) fn recurrence_kind_to_db(kind: RecurrenceKind) -> i32 {
    match kind {
        RecurrenceKind::Weekly => 1,
        RecurrenceKind::Monthly => 2,
        RecurrenceKind::Daily => 3,
    }
}

pub(crate) fn recurrence_kind_from_db(value: i32) -> RecurrenceKind {
    match value {
        2 => RecurrenceKind::Monthly,
        3 => RecurrenceKind::Daily,
        _ => RecurrenceKind::Weekly,
    }
}

pub(crate) fn monthly_kind_to_db(kind: MonthlyKind) -> i32 {
    match kind {
        MonthlyKind::DayOfMonth => 0,
        MonthlyKind::LastWorkday => 1,
        MonthlyKind::LastDay => 2,
    }
}

pub(crate) fn monthly_kind_from_db(value: i32) -> MonthlyKind {
    match value {
        1 => MonthlyKind::LastWorkday,
        2 => MonthlyKind::LastDay,
        _ => MonthlyKind::DayOfMonth,
    }
}
