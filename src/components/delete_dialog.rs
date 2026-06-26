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
pub(crate) fn DeleteDialog(
    app: Signal<AppState>,
    clock: Signal<NaiveDateTime>,
    text: Strings,
) -> Element {
    rsx! {
        div { class: "modal modal-open",
            div { class: "modal-box",
                h2 { class: "flex items-center gap-2 text-lg font-bold",
                    Icon { width: 18, height: 18, icon: LdTrash2 }
                    "{text.delete}"
                }
                p { class: "py-4", "{text.delete_confirm}" }
                div { class: "modal-action",
                    button { class: "btn", onclick: move |_| close_dialog(app),
                        Icon { width: 15, height: 15, icon: LdX }
                        "{text.cancel}"
                    }
                    button { class: "btn btn-error", onclick: move |_| confirm_delete(app, clock),
                        Icon { width: 15, height: 15, icon: LdTrash2 }
                        "{text.delete}"
                    }
                }
            }
            div { class: "modal-backdrop", onclick: move |_| close_dialog(app) }
        }
    }
}
