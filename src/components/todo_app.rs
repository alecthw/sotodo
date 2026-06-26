#![allow(unused_imports)]

use super::{CloseDialog, DeleteDialog, Empty, SettingsDialog, TodoDialog, TodoRow};
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
pub(crate) fn TodoApp() -> Element {
    let app = use_signal(AppState::load);
    let mut clock = use_signal(|| Local::now().naive_local());
    let initial_settings = app().settings;
    let initial_language = initial_settings.effective_language();
    #[cfg(windows)]
    let tray = use_hook(move || {
        WindowsTray::new(
            initial_language,
            current_main_hwnd(),
            initial_settings.hotkey_enabled,
            initial_settings.hotkey.clone(),
        )
    });
    use_effect(move || {
        let settings = app().settings;
        #[cfg(windows)]
        {
            tray.set_main_window(current_main_hwnd());
            tray.set_language(settings.effective_language());
            tray.set_visible(settings.tray_enabled);
            tray.set_hotkey(settings.hotkey_enabled, settings.hotkey.clone());
        }
        apply_startup_setting(settings.startup_enabled);
    });
    use_hook(move || {
        #[cfg(windows)]
        {
            snap_window_to_top_right();
            spawn(async move {
                tokio::time::sleep(StdDuration::from_millis(200)).await;
                snap_window_to_top_right();
            });
        }
        register_system_notifications();
        start_update_check();
        mutate(app, |state| reschedule_reminders(state, app, clock));
        spawn(async move {
            loop {
                tokio::time::sleep(StdDuration::from_secs(1)).await;
                clock.set(Local::now().naive_local());
            }
        });
    });
    let state = app();
    let now = clock();
    let language = state.settings.effective_language();
    let text = Strings::new(language);
    let theme = active_app_theme(&state);
    let visible_month = state.visible_month;
    let selected_date = state.selected_date;
    let top_most = state.top_most;

    let groups = grouped_occurrences(&state, state.mode == ViewMode::Completed);
    let calendar_days = calendar_days(visible_month);
    let calendar_occurrences = calendar_occurrences(&state.todos, visible_month);
    let selected_todos = occurrences_between(
        &state.todos,
        selected_date.and_time(NaiveTime::MIN),
        end_of_day(selected_date),
    );

    rsx! {
        div {
            "data-theme": "{theme}",
            class: "min-h-screen bg-base-200 text-base-content",
            div { class: "mx-auto flex h-screen max-w-3xl flex-col overflow-hidden bg-base-100 shadow-xl",
                header { class: "flex shrink-0 items-center justify-between gap-2 border-b border-base-300 px-3 py-2",
                    div {
                        class: "min-w-0 flex-1 cursor-move select-none py-1",
                        onmousedown: move |_| dioxus::desktop::window().drag(),
                        h1 { class: "text-xl font-bold leading-tight", "{text.app_name}" }
                        p { class: "truncate text-xs opacity-65", "{format_title_datetime(now, language)}" }
                    }
                    div { class: "flex items-center gap-2",
                        button {
                            class: "btn btn-primary btn-sm",
                            onclick: move |_| show_editor(app, None),
                            Icon { width: 16, height: 16, icon: LdPlus }
                            "{text.add}"
                        }
                        button {
                            class: "btn btn-ghost btn-square btn-sm",
                            title: "{text.settings}",
                            onclick: move |_| show_settings(app),
                            Icon { width: 16, height: 16, icon: LdSettings }
                        }
                        div { class: "ml-1 flex items-center gap-1 border-l border-base-300 pl-2",
                            button {
                                class: "btn btn-ghost btn-square btn-sm",
                                title: "Minimize",
                                onclick: move |_| dioxus::desktop::window().window.set_minimized(true),
                                Icon { width: 16, height: 16, icon: LdMinus }
                            }
                            button {
                                class: if top_most { "btn btn-primary btn-square btn-sm" } else { "btn btn-ghost btn-square btn-sm" },
                                title: "Always on top",
                                onclick: move |_| toggle_top_most(app),
                                if top_most {
                                    Icon { width: 16, height: 16, icon: LdPinOff }
                                } else {
                                    Icon { width: 16, height: 16, icon: LdPin }
                                }
                            }
                            button {
                                class: "btn btn-ghost btn-square btn-sm text-error",
                                title: "Close",
                                onclick: move |_| request_close(app),
                                Icon { width: 16, height: 16, icon: LdX }
                            }
                        }
                    }
                }

                main { class: "flex min-h-0 flex-1 flex-col p-4",
                    div { role: "tablist", class: "tabs tabs-boxed mb-3 grid grid-cols-3",
                        button {
                            class: tab_class(state.mode == ViewMode::List),
                            onclick: move |_| mutate(app, |s| s.mode = ViewMode::List),
                            Icon { width: 15, height: 15, icon: LdListTodo }
                            "{text.open}"
                        }
                        button {
                            class: tab_class(state.mode == ViewMode::Calendar),
                            onclick: move |_| mutate(app, |s| s.mode = ViewMode::Calendar),
                            Icon { width: 15, height: 15, icon: LdCalendar }
                            "{text.calendar}"
                        }
                        button {
                            class: tab_class(state.mode == ViewMode::Completed),
                            onclick: move |_| mutate(app, |s| s.mode = ViewMode::Completed),
                            Icon { width: 15, height: 15, icon: LdCircleCheck }
                            "{text.completed}"
                        }
                    }

                    if state.mode == ViewMode::Calendar {
                        div { class: "flex min-h-0 flex-1 flex-col gap-3",
                            div { class: "flex items-center justify-between gap-2",
                                button {
                                    class: "btn btn-square btn-sm",
                                    onclick: move |_| mutate(app, |s| s.visible_month = add_months(s.visible_month, -1)),
                                    Icon { width: 16, height: 16, icon: LdChevronLeft }
                                }
                                div { class: "font-semibold", "{month_title(visible_month, language)}" }
                                button {
                                    class: "btn btn-square btn-sm",
                                    onclick: move |_| mutate(app, |s| s.visible_month = add_months(s.visible_month, 1)),
                                    Icon { width: 16, height: 16, icon: LdChevronRight }
                                }
                            }
                            div { class: "grid grid-cols-7 gap-1 text-center text-xs font-semibold opacity-60",
                                for day in weekday_names(language) {
                                    div { "{day}" }
                                }
                            }
                            div { class: "grid grid-cols-7 gap-1",
                                for day in calendar_days {
                                    {
                                        let count = calendar_occurrences.iter().filter(|item| item.due_at.date() == day).count();
                                        let selected = day == selected_date;
                                        let other_month = day.month() != visible_month.month();
                                        let dot_class = if count > 0 { calendar_dot_class(selected) } else { "h-1.5 w-1.5" };
                                        rsx! {
                                            button {
                                                class: calendar_day_class(selected, other_month),
                                                onclick: move |_| mutate(app, |s| {
                                                    s.selected_date = day;
                                                    s.visible_month = first_of_month(day);
                                                }),
                                                span { class: "font-semibold", "{day.day()}" }
                                                span { class: dot_class }
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "min-h-0 flex-1 overflow-y-auto rounded-box border border-base-300",
                                div { class: "flex items-center justify-between bg-base-200 px-3 py-2 font-semibold",
                                    span { "{format_date(selected_date, language)}" }
                                    span { class: "badge badge-ghost", "{selected_todos.len()}" }
                                }
                                if selected_todos.is_empty() {
                                    Empty { label: text.no_todos.clone() }
                                } else {
                                    for occurrence in selected_todos {
                                            TodoRow {
                                                key: "{occurrence_dom_key(&occurrence)}",
                                                occurrence,
                                                now,
                                                text: text.clone(),
                                                on_toggle: move |(occurrence, done)| toggle_done(app, clock, occurrence, done),
                                            on_edit: move |todo: TodoItem| show_editor(app, Some(todo)),
                                            on_delete: move |id| request_delete(app, id),
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        div { class: "flex min-h-0 flex-1 flex-col",
                            div { class: "mb-3 grid grid-cols-[1fr_auto_auto] gap-2",
                                input {
                                    class: "input input-bordered input-sm w-full",
                                    placeholder: "{text.search}",
                                    value: "{state.query}",
                                    oninput: move |e| mutate(app, |s| s.query = e.value()),
                                }
                                button {
                                    class: "btn btn-square btn-sm",
                                    title: "{text.collapse_all}",
                                    onclick: move |_| collapse_all(app),
                                    Icon { width: 16, height: 16, icon: LdChevronsUp }
                                }
                                button {
                                    class: "btn btn-square btn-sm",
                                    title: "{text.expand_all}",
                                    onclick: move |_| mutate(app, |s| s.collapsed_days.clear()),
                                    Icon { width: 16, height: 16, icon: LdChevronsDown }
                                }
                            }
                            div { class: "min-h-0 flex-1 overflow-y-auto rounded-box border border-base-300",
                                if groups.is_empty() {
                                    Empty { label: if state.todos.is_empty() { text.no_todos.clone() } else { text.no_results.clone() } }
                                } else {
                                    for (day, items) in groups {
                                        {
                                            let collapsed = state.collapsed_days.contains(&day);
                                            rsx! {
                                                section { class: "border-b border-base-300 last:border-b-0",
                                                    button {
                                                        class: "flex w-full items-center justify-between bg-base-200 px-3 py-2 text-left",
                                                        onclick: move |_| toggle_day(app, day),
                                                        span { class: "flex items-center gap-2 font-semibold",
                                                            if collapsed {
                                                                Icon { width: 14, height: 14, icon: LdChevronRight }
                                                            } else {
                                                                Icon { width: 14, height: 14, icon: LdChevronDown }
                                                            }
                                                            "{format_group_date(day, language)}"
                                                        }
                                                        span { class: "badge badge-ghost", "{items.len()}" }
                                                    }
                                                    if !collapsed {
                                                        for occurrence in items {
                                                            TodoRow {
                                                                key: "{occurrence_dom_key(&occurrence)}",
                                                                occurrence,
                                                                now,
                                                                text: text.clone(),
                                                                on_toggle: move |(occurrence, done)| toggle_done(app, clock, occurrence, done),
                                                                on_edit: move |todo: TodoItem| show_editor(app, Some(todo)),
                                                                on_delete: move |id| request_delete(app, id),
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            match state.dialog {
                DialogMode::Todo => rsx! { TodoDialog { app, clock, state: state.clone(), text: text.clone() } },
                DialogMode::Settings => rsx! { SettingsDialog { app, state: state.clone(), text: text.clone() } },
                DialogMode::DeleteConfirm => rsx! { DeleteDialog { app, clock, text: text.clone() } },
                DialogMode::CloseConfirm => rsx! { CloseDialog { app, state: state.clone(), text: text.clone() } },
                DialogMode::None => rsx! {},
            }
        }
    }
}
