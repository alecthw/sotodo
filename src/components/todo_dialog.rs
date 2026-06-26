#![allow(unused_imports)]

use crate::actions::*;
use crate::config::{APP_VERSION, PROJECT_URL, THEMES};
use crate::i18n::Strings;
use crate::models::*;
use crate::platform::*;
use crate::reminders::{is_overdue, register_system_notifications, reschedule_reminders};
use crate::todo_logic::*;
use crate::update::{start_update_check, update_available};
use chrono::{Datelike, Local, NaiveDateTime, NaiveTime};
use dioxus::prelude::*;
use dioxus_free_icons::icons::ld_icons::{
    LdAlarmClock, LdCalendar, LdCheck, LdChevronDown, LdChevronLeft, LdChevronRight,
    LdChevronsDown, LdChevronsUp, LdCircleArrowUp, LdCircleCheck, LdClock, LdKeyboard, LdLanguages,
    LdListTodo, LdMinus, LdPalette, LdPencil, LdPin, LdPinOff, LdPlus, LdRepeat, LdSave,
    LdSettings, LdTrash2, LdX,
};
use dioxus_free_icons::Icon;
use std::time::Duration as StdDuration;
use uuid::Uuid;

#[component]
pub(crate) fn TodoDialog(
    app: Signal<AppState>,
    clock: Signal<NaiveDateTime>,
    state: AppState,
    text: Strings,
) -> Element {
    let editor = state.editor.clone();
    let (due_hour, due_minute) = time_select_parts(&editor.due_time);

    rsx! {
        div { class: "modal modal-open",
            div {
                class: "modal-box todo-dialog-box focus:outline-none",
                tabindex: "0",
                onkeydown: move |e| handle_todo_dialog_key(e, app, clock),
                h2 { class: "todo-dialog-title flex items-center gap-2 text-lg font-bold",
                    if editor.editing_id.is_some() {
                        Icon { width: 18, height: 18, icon: LdPencil }
                    } else {
                        Icon { width: 18, height: 18, icon: LdPlus }
                    }
                    if editor.editing_id.is_some() { "{text.edit_todo}" } else { "{text.new_todo}" }
                }
                div { class: "todo-dialog-content",
                    if !editor.validation.is_empty() {
                        div { class: "alert alert-error todo-dialog-alert py-2", "{editor.validation}" }
                    }

                    label { class: "form-control todo-dialog-field",
                        div { class: "label py-1", span { class: "label-text", "{text.title}" } }
                        input {
                            class: "input input-bordered w-full",
                            value: "{editor.title}",
                            onmounted: move |e| async move {
                                let _ = e.data().set_focus(true).await;
                            },
                            oninput: move |e| mutate(app, |s| s.editor.title = e.value()),
                        }
                    }

                    label { class: "form-control todo-dialog-field",
                        div { class: "label py-1", span { class: "label-text", "{text.notes}" } }
                        textarea {
                            class: "textarea textarea-bordered todo-dialog-notes w-full",
                            value: "{editor.notes}",
                            onkeydown: move |e| {
                                if e.key() == Key::Enter {
                                    e.stop_propagation();
                                }
                            },
                            oninput: move |e| mutate(app, |s| s.editor.notes = e.value()),
                        }
                    }

                    div { class: "todo-dialog-stack",
                        label { class: "form-control todo-dialog-field",
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
                                label { class: "form-control todo-dialog-field",
                                    div { class: "label py-1",
                                        span { class: "label-text flex items-center gap-2",
                                            "{text.time}"
                                            input {
                                                r#type: "checkbox",
                                                class: "toggle toggle-primary toggle-sm",
                                                checked: editor.due_time_enabled,
                                                onchange: move |e| mutate(app, |s| {
                                                    let default_reminders = s.settings.default_reminder_minutes.clone();
                                                    set_editor_time_enabled(&mut s.editor, e.checked(), &default_reminders);
                                                }),
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
                                                    let selected = value == due_hour;
                                                    rsx! {
                                                        option { value: "{value}", selected, "{value}" }
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
                                                    let selected = value == due_minute;
                                                    rsx! {
                                                        option { value: "{value}", selected, "{value}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                label { class: "form-control todo-dialog-field",
                                    div { class: "label py-1",
                                        span { class: "label-text flex items-center gap-2",
                                            "{text.time}"
                                            input {
                                                r#type: "checkbox",
                                                class: "toggle toggle-primary toggle-sm",
                                                checked: editor.due_time_enabled,
                                                onchange: move |e| mutate(app, |s| {
                                                    let default_reminders = s.settings.default_reminder_minutes.clone();
                                                    set_editor_time_enabled(&mut s.editor, e.checked(), &default_reminders);
                                                }),
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if editor.due_date_enabled {
                        div { class: "todo-dialog-heading label py-1",
                            span { class: "label-text flex items-center gap-2",
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
                    }

                    if editor.due_date_enabled && editor.recurring {
                        div { class: "todo-dialog-panel rounded-box border border-base-300 bg-base-200 p-3",
                            label { class: "form-control todo-dialog-field",
                                div { class: "label py-1", span { class: "label-text", "{text.repeat_type}" } }
                                select {
                                    class: "select select-bordered w-full",
                                    value: editor.recurrence_kind.as_str(),
                                    onchange: move |e| mutate(app, |s| {
                                        s.editor.recurrence_kind = RecurrenceKind::from_value(&e.value());
                                        sync_editor_repeat_defaults(&mut s.editor);
                                    }),
                                    option {
                                        value: "weekly",
                                        selected: editor.recurrence_kind == RecurrenceKind::Weekly,
                                        "{text.weekly}"
                                    }
                                    option {
                                        value: "monthly",
                                        selected: editor.recurrence_kind == RecurrenceKind::Monthly,
                                        "{text.monthly}"
                                    }
                                    option {
                                        value: "daily",
                                        selected: editor.recurrence_kind == RecurrenceKind::Daily,
                                        "{text.daily}"
                                    }
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
                            } else if editor.recurrence_kind == RecurrenceKind::Monthly {
                                label { class: "form-control todo-dialog-field",
                                    div { class: "label py-1", span { class: "label-text", "{text.monthly_repeat}" } }
                                    select {
                                        class: "select select-bordered w-full",
                                    value: editor.monthly_kind.as_str(),
                                    onchange: move |e| mutate(app, |s| s.editor.monthly_kind = MonthlyKind::from_value(&e.value())),
                                    option {
                                        value: "day",
                                        selected: editor.monthly_kind == MonthlyKind::DayOfMonth,
                                        "{text.day_of_month}"
                                    }
                                    option {
                                        value: "last_workday",
                                        selected: editor.monthly_kind == MonthlyKind::LastWorkday,
                                        "{text.last_workday}"
                                    }
                                    option {
                                        value: "last_day",
                                        selected: editor.monthly_kind == MonthlyKind::LastDay,
                                        "{text.last_day}"
                                    }
                                }
                            }
                                if editor.monthly_kind == MonthlyKind::DayOfMonth {
                                    label { class: "form-control todo-dialog-field",
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

                    if editor.due_date_enabled && editor.due_time_enabled {
                        div { class: "todo-dialog-section",
                            div { class: "todo-dialog-heading label py-1",
                                span { class: "label-text flex items-center gap-2",
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
                            if editor.reminders_enabled {
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
                    }
                }

                div { class: "modal-action todo-dialog-actions",
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
