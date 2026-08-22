use chrono::Local;
use std::time::Duration;

#[derive(Debug)]
pub struct DateTimeChanged;
impl app::Event for DateTimeChanged {}

#[derive(Debug)]
pub struct DateTimeTick;
impl app::Event for DateTimeTick {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateTimeUpdate {
    pub time: String,
    pub date: String,
}
impl app::Event for DateTimeUpdate {}

#[derive(Debug, Clone, Default)]
pub struct DateTime;

impl DateTime {
    pub fn new() -> Self {
        Self
    }

    pub fn format(fmt: &str) -> String {
        Local::now().format(fmt).to_string().to_uppercase()
    }

    pub fn next_deadline(&self) -> Duration {
        let now = std::time::SystemTime::now();
        let duration_since_epoch = now
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO);
        let current_secs = duration_since_epoch.as_secs();

        let next_minute_secs = (current_secs / 60 + 1) * 60;
        Duration::new(next_minute_secs, 100_000_000)
    }

    pub fn module<AppState>() -> impl app::RegisteredModule<Self, AppState> {
        app::Module::new().on(|_w: &mut Self, _ev: &DateTimeTick| Some(DateTimeChanged))
    }
}
