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
pub(crate) fn SettingsDialog(app: Signal<AppState>, state: AppState, text: Strings) -> Element {
    let settings = state.settings_editor.clone();
    let hotkey_recording = state.settings_hotkey_recording;

    rsx! {
        div { class: "modal modal-open",
            div {
                class: "modal-box settings-dialog-box focus:outline-none",
                tabindex: "0",
                onmounted: move |e| async move {
                    let _ = e.data().set_focus(true).await;
                },
                onkeydown: move |e| handle_settings_dialog_key(e, app),
                div { class: "settings-dialog-title flex items-center justify-between gap-3",
                    h2 { class: "flex items-center gap-2 text-lg font-bold",
                        Icon { width: 18, height: 18, icon: LdSettings }
                        "{text.settings}"
                    }
                    button {
                        class: "link link-hover inline-flex items-center gap-1 text-xs opacity-70",
                        title: PROJECT_URL,
                        onclick: move |_| open_project_homepage(),
                        "{APP_VERSION}"
                        if update_available() {
                            Icon { width: 13, height: 13, icon: LdCircleArrowUp }
                        }
                    }
                }

                div { class: "settings-dialog-content",
                    div { class: "divide-y divide-base-300",
                        div { class: "p-3",
                            label { class: "form-control",
                                div { class: "label py-1", span { class: "label-text inline-flex items-center gap-1",
                                    Icon { width: 13, height: 13, icon: LdLanguages }
                                    "{text.language}"
                                } }
                                select {
                                    class: "select select-bordered w-full",
                                    value: "{settings.language}",
                                    onchange: move |e| mutate(app, |s| {
                                        s.settings_editor.language = e.value();
                                    }),
                                    option {
                                        value: "system",
                                        selected: settings.language == "system",
                                        "{text.system}"
                                    }
                                    option {
                                        value: "en",
                                        selected: settings.language == "en",
                                        "{text.english}"
                                    }
                                    option {
                                        value: "zh",
                                        selected: settings.language == "zh",
                                        "{text.chinese}"
                                    }
                                }
                            }
                        }

                        div { class: "p-3",
                            label { class: "form-control",
                                div { class: "label py-1", span { class: "label-text inline-flex items-center gap-1",
                                    Icon { width: 13, height: 13, icon: LdPalette }
                                    "{text.theme}"
                                } }
                                select {
                                    class: "select select-bordered w-full",
                                    value: "{settings.theme}",
                                    onchange: move |e| mutate(app, |s| {
                                        s.settings_editor.theme = e.value();
                                    }),
                                    option {
                                        value: "system",
                                        selected: settings.theme == "system",
                                        "{text.system}"
                                    }
                                    for theme in THEMES {
                                        {
                                            let selected = settings.theme == *theme;
                                            rsx! {
                                                option { value: "{theme}", selected, "{theme}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "p-3",
                            label { class: "form-control",
                                div { class: "label py-1", span { class: "label-text", "{text.close_behavior}" } }
                                select {
                                    class: "select select-bordered w-full",
                                    value: "{settings.close_behavior}",
                                    onchange: move |e| mutate(app, |s| {
                                        s.settings_editor.close_behavior = e.value();
                                    }),
                                    option {
                                        value: "prompt",
                                        selected: settings.close_behavior == "prompt",
                                        "{text.ask_on_close}"
                                    }
                                    option {
                                        value: "tray",
                                        selected: settings.close_behavior == "tray",
                                        "{text.minimize_to_tray}"
                                    }
                                    option {
                                        value: "exit",
                                        selected: settings.close_behavior == "exit",
                                        "{text.exit_app}"
                                    }
                                }
                            }
                        }

                        div { class: "p-3",
                            label { class: "flex cursor-pointer items-center justify-between gap-3",
                                span { class: "font-semibold", "{text.tray_enabled}" }
                                input {
                                    r#type: "checkbox",
                                    class: "toggle toggle-primary",
                                    checked: settings.tray_enabled,
                                    onchange: move |e| mutate(app, |s| {
                                        s.settings_editor.tray_enabled = e.checked();
                                    }),
                                }
                            }
                        }

                        div { class: "p-3",
                            label { class: "flex cursor-pointer items-center justify-between gap-3",
                                span { class: "font-semibold", "{text.startup_enabled}" }
                                input {
                                    r#type: "checkbox",
                                    class: "toggle toggle-primary",
                                    checked: settings.startup_enabled,
                                    onchange: move |e| mutate(app, |s| {
                                        s.settings_editor.startup_enabled = e.checked();
                                    }),
                                }
                            }
                        }

                        div { class: "p-3",
                            label { class: "flex cursor-pointer items-center justify-between gap-3",
                                span { class: "font-semibold", "{text.hotkey_enabled}" }
                                input {
                                    r#type: "checkbox",
                                    class: "toggle toggle-primary",
                                    checked: settings.hotkey_enabled,
                                    onchange: move |e| mutate(app, |s| {
                                        s.settings_editor.hotkey_enabled = e.checked();
                                    }),
                                }
                            }
                            label { class: "form-control mt-2",
                                div { class: "label py-1", span { class: "label-text", "{text.hotkey}" } }
                                button {
                                    r#type: "button",
                                    class: "input input-bordered flex w-full items-center justify-start gap-2 text-left",
                                    disabled: !settings.hotkey_enabled,
                                    onclick: move |_| mutate(app, |s| s.settings_hotkey_recording = true),
                                    onblur: move |_| mutate(app, |s| s.settings_hotkey_recording = false),
                                    onkeydown: move |e| capture_hotkey_keydown(e, app),
                                    Icon { width: 15, height: 15, icon: LdKeyboard }
                                    if hotkey_recording {
                                        span { class: "opacity-60", "{text.hotkey_recording}" }
                                    } else {
                                        span { "{settings.hotkey}" }
                                    }
                                }
                            }
                        }

                        div { class: "p-3",
                            div { class: "flex items-center gap-2 font-semibold",
                                Icon { width: 15, height: 15, icon: LdAlarmClock }
                                "{text.default_reminders}"
                            }
                            div { class: "mt-2 divide-y divide-base-300 overflow-hidden rounded-box border border-base-300 bg-base-100",
                                for minutes in settings.default_reminder_minutes.clone() {
                                    div { class: "flex items-center justify-between px-3 py-2",
                                        span { "{minutes} {text.minutes_before}" }
                                        button {
                                            class: "btn btn-ghost btn-square btn-xs text-error",
                                            onclick: move |_| mutate(app, |s| {
                                                s.settings_editor.default_reminder_minutes.retain(|value| *value != minutes);
                                                if s.settings_editor.default_reminder_minutes.is_empty() {
                                                    s.settings_editor.default_reminder_minutes = vec![15, 5];
                                                }
                                            }),
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
                                    value: "{state.new_default_reminder}",
                                    oninput: move |e| mutate(app, |s| s.new_default_reminder = e.value()),
                                }
                                button {
                                    class: "btn",
                                    onclick: move |_| add_default_reminder(app),
                                    Icon { width: 15, height: 15, icon: LdPlus }
                                    "{text.add_reminder}"
                                }
                            }
                        }
                    }
                }

                div { class: "modal-action settings-dialog-actions",
                    button { class: "btn", onclick: move |_| close_dialog(app),
                        Icon { width: 15, height: 15, icon: LdX }
                        "{text.cancel}"
                    }
                    button { class: "btn btn-primary", onclick: move |_| apply_settings(app),
                        Icon { width: 15, height: 15, icon: LdCheck }
                        "{text.done}"
                    }
                }
            }
            div { class: "modal-backdrop", onclick: move |_| close_dialog(app) }
        }
    }
}
