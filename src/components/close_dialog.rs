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
pub(crate) fn CloseDialog(app: Signal<AppState>, state: AppState, text: Strings) -> Element {
    let tray_enabled = state.settings.tray_enabled;

    rsx! {
        div { class: "modal modal-open",
            div { class: "modal-box",
                h2 { class: "text-lg font-bold", "{text.close_app}" }
                p { class: "mt-2 text-sm opacity-70", "{text.close_confirm}" }
                div { class: "modal-action",
                    button { class: "btn", onclick: move |_| close_dialog(app), "{text.cancel}" }
                    if tray_enabled {
                        button { class: "btn btn-secondary", onclick: move |_| {
                            close_dialog(app);
                            hide_main_window();
                        }, "{text.minimize_to_tray}" }
                    }
                    button { class: "btn btn-error", onclick: move |_| dioxus::desktop::window().close(), "{text.exit_app}" }
                }
            }
            div { class: "modal-backdrop", onclick: move |_| close_dialog(app) }
        }
    }
}
