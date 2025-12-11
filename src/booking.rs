use chrono::{DateTime, Local};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Booking {
    start: DateTime<Local>,
    end: DateTime<Local>,
    // First room is the most prefered one, second is the second most prefered one, and so on
    room_with_backup: Vec<String>,
}
