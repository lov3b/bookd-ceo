use chrono::{DateTime, Local};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Booking {
    pub id: Uuid,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    // First room is the most prefered one, second is the second most prefered one, and so on
    pub room_with_backup: Vec<String>,
}
