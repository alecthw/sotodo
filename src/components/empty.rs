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
pub(crate) fn Empty(label: String) -> Element {
    rsx! {
        div { class: "grid min-h-40 place-items-center gap-2 p-6 text-sm opacity-60",
            Icon { width: 28, height: 28, icon: LdListTodo }
            span { "{label}" }
        }
    }
}
