use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsMessage {
    pub port: String,
    pub index: i32,
    pub from: String,
    pub received: DateTime<Utc>,
    pub status: String,
    pub text: String,
    /// SIM memory indices of every fragment this message was assembled from.
    /// Empty for single-part messages (only `index` applies).
    #[serde(default)]
    pub part_indices: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsItem {
    pub id: u64,
    pub message: SmsMessage,
    pub otp: Option<String>,
    pub is_new: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortInfo {
    pub name: String,
    pub checked: bool,
    pub sim_number: String,
    pub live_ready: bool,
    pub live_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanStatus {
    pub busy: bool,
    pub done: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LiveStatus {
    pub on: bool,
    pub ready: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UssdStatus {
    pub busy: bool,
    pub done: usize,
    pub total: usize,
    pub found: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum QuickFilter {
    #[default]
    All,
    Otp,
    Today,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ViewMode {
    #[default]
    Table,
    Cards,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Danger,
    Otp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToastData {
    pub id: u64,
    pub kind: ToastKind,
    pub title: String,
    pub body: String,
    pub otp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    pub ports: Vec<PortInfo>,
    pub messages: Vec<SmsItem>,
    pub selected: Vec<u64>,
    pub query: String,
    pub quick_filter: QuickFilter,
    pub port_filter: Option<String>,
    pub view_mode: ViewMode,
    pub scan: ScanStatus,
    pub live: LiveStatus,
    pub ussd: UssdStatus,
    pub delete_busy: bool,
    pub live_ports_ready: Vec<String>,
    pub live_failed: Vec<(String, String)>,
    pub status_text: String,
    pub failed_notes: Vec<String>,
    pub toasts: Vec<ToastData>,
    pub unread_total: usize,
}

impl SmsMessage {
    pub fn empty() -> Self {
        Self {
            port: String::new(),
            index: 0,
            from: String::new(),
            received: DateTime::UNIX_EPOCH,
            status: String::new(),
            text: String::new(),
            part_indices: Vec::new(),
        }
    }
}

pub fn pretty_port(name: &str) -> String {
    if let Ok(num) = name
        .trim_start_matches("COM")
        .trim_start_matches("ttyUSB")
        .trim_start_matches("ttyACM")
        .parse::<u32>()
    {
        format!("Port {num}")
    } else {
        name.to_string()
    }
}
