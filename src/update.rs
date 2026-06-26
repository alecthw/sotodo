#![allow(unused_imports)]

use crate::config::{
    APP_VERSION, LATEST_RELEASE_API_PATH, UPDATE_CHECK_STARTED, UPDATE_STATUS,
    UPDATE_STATUS_AVAILABLE, UPDATE_STATUS_NO_UPDATE,
};
use crate::platform::winhttp_get;
use std::sync::atomic::Ordering;

pub(crate) fn start_update_check() {
    if UPDATE_CHECK_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(|| {
        let update_available = latest_release_version_with_retries()
            .map(|latest| version_has_update(APP_VERSION, &latest))
            .unwrap_or(false);
        UPDATE_STATUS.store(
            if update_available {
                UPDATE_STATUS_AVAILABLE
            } else {
                UPDATE_STATUS_NO_UPDATE
            },
            Ordering::SeqCst,
        );
    });
}

pub(crate) fn update_available() -> bool {
    UPDATE_STATUS.load(Ordering::SeqCst) == UPDATE_STATUS_AVAILABLE
}

pub(crate) fn latest_release_version_with_retries() -> Option<String> {
    for _ in 0..3 {
        if let Some(version) = fetch_latest_release_version() {
            return Some(version);
        }
    }
    None
}

pub(crate) fn fetch_latest_release_version() -> Option<String> {
    let response = fetch_latest_release_body()?;
    extract_json_string(&response, "tag_name")
}

#[cfg(windows)]
pub(crate) fn fetch_latest_release_body() -> Option<String> {
    unsafe { winhttp_get("api.github.com", LATEST_RELEASE_API_PATH) }
}

#[cfg(not(windows))]
pub(crate) fn fetch_latest_release_body() -> Option<String> {
    None
}

pub(crate) fn version_has_update(current: &str, latest: &str) -> bool {
    normalize_version(current) != normalize_version(latest)
}

pub(crate) fn normalize_version(value: &str) -> String {
    value
        .trim()
        .trim_start_matches(['v', 'V'])
        .to_ascii_lowercase()
}

pub(crate) fn extract_json_string(body: &str, key: &str) -> Option<String> {
    let quoted_key = format!("\"{key}\"");
    let key_index = body.find(&quoted_key)?;
    let after_key = &body[key_index + quoted_key.len()..];
    let colon_index = after_key.find(':')?;
    let after_colon = after_key[colon_index + 1..].trim_start();
    let value = after_colon.strip_prefix('"')?;
    let mut result = String::new();
    let mut escaped = false;
    for chr in value.chars() {
        if escaped {
            result.push(chr);
            escaped = false;
            continue;
        }
        match chr {
            '\\' => escaped = true,
            '"' => return Some(result),
            _ => result.push(chr),
        }
    }
    None
}
