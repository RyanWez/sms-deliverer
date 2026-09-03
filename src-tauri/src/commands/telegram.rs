//! Settings → Forwarding commands: verify a bot token, find the destination
//! group, and prove the whole path works.
//!
//! None of these touch a serial port, so none of them consult
//! `AppStateInner::port_busy()`. That is deliberate: the operator configures
//! forwarding *while* the bank is live, and refusing with "Busy" would force
//! them to stop monitoring in order to set up the thing that monitors.
//!
//! They are `async` so the 15-second HTTP timeout cannot occupy Tauri's IPC
//! thread, and the blocking client runs inside `spawn_blocking` so it cannot
//! park an async executor thread either.

use crate::telegram::{self, DetectedGroup, SendError, TelegramConfig};
use serde::{Deserialize, Serialize};

/// Run a blocking Telegram call off the async executor.
async fn offload<T, F>(job: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(job)
        .await
        .map_err(|_| "The Telegram request thread stopped unexpectedly".to_string())?
}

/// Best-effort machine name for the test message, so an operator running two
/// banks can tell which one they just wired up. Env first (set on Windows and
/// on most shells), then the Linux file, then a placeholder — never an error:
/// failing a test send over a missing hostname would be absurd.
fn host_label() -> String {
    for key in ["COMPUTERNAME", "HOSTNAME"] {
        if let Ok(name) = std::env::var(key) {
            let name = name.trim().to_string();
            if !name.is_empty() {
                return name;
            }
        }
    }
    std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "this PC".to_string())
}

/// Forwarding settings as they arrive from the frontend at `start_live`.
///
/// Carried as a command argument rather than persisted on the Rust side, which
/// keeps `core/sim_directory.rs`'s invariant intact — user preferences live in
/// exactly one place, the frontend settings store — and follows the precedent
/// `retention_hours` already set.
///
/// The consequence is that changing a token mid-session takes a Stop → Start,
/// exactly as changing the retention window does.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForwardingConfigDto {
    pub bot_token: String,
    pub chat_id: String,
    #[serde(default)]
    pub proxy_url: Option<String>,
    pub forward_otp: bool,
    pub forward_non_otp: bool,
}

impl ForwardingConfigDto {
    /// Split into what `telegram` needs and what `forwarder` needs.
    ///
    /// Trimming here rather than trusting the page: the settings store is
    /// rehydrated from `localStorage`, so a stray space around a pasted token is
    /// normal input, not an exceptional case.
    pub fn split(self) -> (TelegramConfig, (bool, bool)) {
        (
            TelegramConfig {
                bot_token: self.bot_token.trim().to_string(),
                chat_id: self.chat_id.trim().to_string(),
                proxy_url: self
                    .proxy_url
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty()),
            },
            (self.forward_otp, self.forward_non_otp),
        )
    }
}

/// A trimmed token, or a reason it cannot be used.
///
/// Checked here rather than in the Settings page because the frontend is not a
/// validator: the settings store is rehydrated from `localStorage`, which can
/// hold whatever an older profile or a hand edit left behind.
fn require_token(token: &str) -> Result<String, String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("Enter the bot token from @BotFather first".to_string());
    }
    // `<digits>:<secret>` is the shape BotFather issues. Rejecting anything else
    // here turns "pasted the bot username by mistake" into a clear message
    // instead of Telegram's bare `401 Unauthorized`.
    if !token.split_once(':').is_some_and(|(id, secret)| {
        !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) && !secret.is_empty()
    }) {
        return Err("That does not look like a bot token — expected 123456789:AA...".to_string());
    }
    Ok(token.to_string())
}

/// What `detect_telegram_group` hands the Settings page.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedGroupDto {
    /// A string, not a number: this lands straight in a text input and goes back
    /// to Telegram verbatim, so there is nothing to gain from making the
    /// frontend round-trip it through a float.
    pub chat_id: String,
    pub title: String,
    /// `"group"` or `"supergroup"`. A plain group's id changes the moment it is
    /// upgraded, which is what the UI's warning is for.
    pub kind: String,
}

impl From<DetectedGroup> for DetectedGroupDto {
    fn from(g: DetectedGroup) -> Self {
        Self {
            chat_id: g.chat_id.to_string(),
            title: g.title,
            kind: g.kind,
        }
    }
}

/// Outcome of "Send Test Message".
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramTestResult {
    /// The bot's `@name`, so the operator can confirm they configured the bot
    /// they meant to.
    pub bot: String,
    /// Set when the group had been upgraded to a supergroup and its id changed
    /// underneath the saved value. The frontend stores this and the operator
    /// never learns anything went wrong.
    pub migrated_chat_id: Option<String>,
}

/// Confirm a bot token by asking Telegram who the bot is.
///
/// Separate from the group check on purpose: a wrong token and a missing group
/// are the two ways setup fails, and they have completely different fixes.
#[tauri::command]
pub async fn verify_telegram_token(
    token: String,
    proxy_url: Option<String>,
) -> Result<String, String> {
    let token = require_token(&token)?;
    offload(move || {
        let client = telegram::build_client(proxy_url.as_deref())?;
        telegram::get_me(&client, &token).map_err(|e| e.to_string())
    })
    .await
}

/// Find the group the bot was most recently added to.
#[tauri::command]
pub async fn detect_telegram_group(
    token: String,
    proxy_url: Option<String>,
) -> Result<DetectedGroupDto, String> {
    let token = require_token(&token)?;
    offload(move || {
        let client = telegram::build_client(proxy_url.as_deref())?;
        telegram::detect_group(&client, &token)
            .map(DetectedGroupDto::from)
            .map_err(|e| e.to_string())
    })
    .await
}

/// Post a test message, proving token, group id, membership and the network
/// path all at once.
///
/// Heals a supergroup migration instead of reporting it. Enabling group
/// auto-delete or admin-only posting silently upgrades a basic group and changes
/// its id; Telegram hands back the replacement, so the only correct response is
/// to use it and tell the frontend to save it. Retried exactly once — a second
/// migration inside one round trip is not a real condition, and looping on
/// Telegram's say-so is how a retry becomes a hang.
#[tauri::command]
pub async fn send_telegram_test(
    token: String,
    chat_id: String,
    proxy_url: Option<String>,
) -> Result<TelegramTestResult, String> {
    let token = require_token(&token)?;
    let chat_id = chat_id.trim().to_string();
    if chat_id.is_empty() {
        return Err("No destination group yet — press Detect Group ID first".to_string());
    }

    offload(move || {
        let client = telegram::build_client(proxy_url.as_deref())?;
        let bot = telegram::get_me(&client, &token).map_err(|e| e.to_string())?;
        let mut config = TelegramConfig {
            bot_token: token,
            chat_id,
            proxy_url: None,
        };
        let html = telegram::test_message_html(&host_label());

        match telegram::send_message(&client, &config, &html) {
            Ok(_) => Ok(TelegramTestResult {
                bot,
                migrated_chat_id: None,
            }),
            Err(SendError::Migrated(new_id)) => {
                log::info!("Telegram group migrated to {new_id}; retrying with the new id");
                config.chat_id = new_id.to_string();
                telegram::send_message(&client, &config, &html).map_err(|e| e.to_string())?;
                Ok(TelegramTestResult {
                    bot,
                    migrated_chat_id: Some(config.chat_id),
                })
            }
            Err(e) => Err(e.to_string()),
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_token_accepts_the_botfather_shape() {
        assert_eq!(
            require_token("  1234567890:AAF3xK-secret  ").as_deref(),
            Ok("1234567890:AAF3xK-secret")
        );
    }

    #[test]
    fn require_token_rejects_an_empty_field() {
        assert!(require_token("   ").is_err());
    }

    /// Pasting the bot's @username instead of its token is the easy mistake, and
    /// it should not come back as Telegram's bare `401 Unauthorized`.
    #[test]
    fn require_token_rejects_a_username() {
        assert!(require_token("@my_otp_bot").is_err());
    }

    #[test]
    fn require_token_rejects_a_non_numeric_bot_id() {
        assert!(require_token("abcdef:AAF3xK").is_err());
        assert!(require_token(":AAF3xK").is_err());
        assert!(require_token("1234567890:").is_err());
    }

    /// The label is cosmetic, but it is interpolated into HTML, so it must never
    /// come back empty and leave `<code></code>` in the message.
    #[test]
    fn host_label_always_returns_something() {
        assert!(!host_label().is_empty());
    }
}
