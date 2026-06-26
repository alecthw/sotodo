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
pub(crate) fn TodoRow(
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
                    "{todo_time_tag(occurrence.due_at)}"
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
