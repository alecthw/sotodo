mod actions;
mod app;
mod assets;
mod components;
mod config;
mod i18n;
mod models;
mod platform;
mod reminders;
mod storage;
mod todo_logic;
mod update;

#[cfg(test)]
mod tests;

fn main() {
    app::run();
}
