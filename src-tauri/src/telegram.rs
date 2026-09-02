//! Telegram Bot API client for forwarding SMS/OTP out of the bank.
//!
//! Push-only: this module never polls for commands. The one read it performs is
//! `getUpdates` behind the Settings "Detect Group ID" button, which is how the
//! operator learns the numeric id of the group they just added the bot to.
//!
//! # Token safety
//!
//! The bot token sits in the request *path* (`/bot<token>/sendMessage`), and
//! `reqwest`'s error `Display` includes the URL it was talking to. This crate
//! logs transport failures verbatim (`core/at.rs` does it for serial), and Info
//! and below reaches both the 1000-entry ring buffer shown on the Logs page and
//! `app.log`, which rotates on size only and is never aged out. A single
//! connection failure would therefore write the token into a file that outlives
//! the inbox retention window.
//!
//! So every error leaving this module is passed through [`redact`] first, and
//! nothing here returns a raw `reqwest::Error`. That is a hard requirement, not
//! a nicety — see AGENTS.md "Logging and privacy".
//!
//! # Why `parse_mode: HTML` and not Markdown
//!
//! The message body is attacker-controlled: anyone who can send an SMS to the
//! bank chooses it. A body containing `*`, `_` or a backtick makes Telegram
//! reject the whole send with `400 Bad Request: can't parse entities`, so the
//! OTP silently never arrives. HTML needs only three characters escaped
//! ([`escape_html`]) and gives `<code>` for the OTP, which is tap-to-copy in
//! every Telegram client.

use serde::Deserialize;
use std::time::Duration;

/// Where forwarded messages go, and how to reach the API.
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub chat_id: String,
    /// `socks5h://host:port` when an ISP blocks `api.telegram.org`. Bot API
    /// traffic works through a SOCKS5 proxy; an MTProto proxy link does not,
    /// it only helps Telegram's own client apps.
    pub proxy_url: Option<String>,
}

/// Why a send did not land.
///
/// The two named variants exist because both are recoverable without telling
/// the operator anything: the caller can save the new id, or wait the stated
/// number of seconds, and retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendError {
    /// The group was upgraded to a supergroup and its id changed. Telegram
    /// hands back the replacement in `parameters.migrate_to_chat_id`, so this
    /// never needs to surface as "press Detect Group ID again".
    Migrated(i64),
    /// `429`, with `parameters.retry_after` in seconds.
    RateLimited(u64),
    /// Anything else. Already passed through [`redact`].
    Other(String),
}

impl std::fmt::Display for SendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Migrated(id) => write!(f, "group migrated to chat id {id}"),
            Self::RateLimited(secs) => write!(f, "rate limited, retry after {secs}s"),
            Self::Other(msg) => f.write_str(msg),
        }
    }
}

/// A group the bot has been added to, as reported by `getUpdates`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedGroup {
    pub chat_id: i64,
    pub title: String,
    /// `"group"` or `"supergroup"`. A plain group's id changes if it is ever
    /// upgraded, so the UI warns when this is not already `"supergroup"`.
    pub kind: String,
}

/// Long enough for a slow mobile uplink, short enough that a blocked network
/// fails while the OTP is still worth delivering.
const TIMEOUT: Duration = Duration::from_secs(15);

/// Stand-in written over the token before any error text is logged or returned.
const REDACTED: &str = "<token redacted>";

/// Remove every occurrence of the bot token from text bound for a log sink, an
/// error toast or the frontend.
///
/// An empty token is left alone deliberately: `str::replace` with an empty
/// needle inserts the replacement between every character, which would turn a
/// diagnostic message into unreadable noise exactly when the operator has not
/// configured a token yet and most needs to read it.
pub fn redact(text: &str, token: &str) -> String {
    if token.is_empty() {
        return text.to_string();
    }
    text.replace(token, REDACTED)
}

/// Escape the three characters Telegram's HTML parse mode treats as markup.
///
/// `&` has to go first — escaping it after `<` would rewrite the `&` in the
/// `&lt;` just produced and emit `&amp;lt;`.
pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn api_url(token: &str, method: &str) -> String {
    format!("https://api.telegram.org/bot{token}/{method}")
}

/// Install the rustls crypto provider this process needs, once.
///
/// `reqwest` is declared with `rustls-no-provider` to reuse the ring-backed
/// rustls `tauri-plugin-updater` already brings in, rather than pulling
/// aws-lc-rs and a cmake/C build step for a TLS stack the binary already has.
/// The cost is that reqwest **panics** while building a client if no provider is
/// registered — and the updater only registers one when *it* runs, which may be
/// never.
///
/// `install_default` fails if another provider is already registered; that is
/// the expected outcome when the updater got there first and is not an error.
fn ensure_crypto_provider() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
    });
}

/// Build the HTTP client, optionally through a SOCKS5 proxy.
///
/// A malformed proxy URL is reported as a configuration error rather than
/// silently falling back to a direct connection: on a network that blocks
/// Telegram, a silent fallback would look like "forwarding is broken" with no
/// hint that the proxy field is the reason.
pub fn build_client(proxy_url: Option<&str>) -> Result<reqwest::blocking::Client, String> {
    ensure_crypto_provider();
    let mut builder = reqwest::blocking::Client::builder().timeout(TIMEOUT);
    if let Some(url) = proxy_url.map(str::trim).filter(|u| !u.is_empty()) {
        let proxy = reqwest::Proxy::all(url).map_err(|e| format!("Invalid proxy URL: {e}"))?;
        builder = builder.proxy(proxy);
    }
    builder
        .build()
        .map_err(|e| format!("Could not create the HTTPS client: {e}"))
}

/// The envelope every Bot API method answers with.
#[derive(Debug, Deserialize)]
struct ApiEnvelope {
    ok: bool,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    parameters: Option<ApiParameters>,
}

#[derive(Debug, Deserialize)]
struct ApiParameters {
    #[serde(default)]
    migrate_to_chat_id: Option<i64>,
    #[serde(default)]
    retry_after: Option<u64>,
}

/// Turn a raw HTTP status + body into either the `result` field or a classified
/// error.
///
/// Split out from the request itself so the whole error taxonomy — migration,
/// rate limiting, a bad token, an HTML error page from a captive portal — is
/// unit-testable with no network and no hardware.
///
/// `parameters` is checked before `ok`, because the two recoverable cases are
/// both reported as `ok: false` with a perfectly ordinary `description` and the
/// machine-readable part is the only reliable signal.
fn interpret(status: u16, body: &str, token: &str) -> Result<serde_json::Value, SendError> {
    let envelope: ApiEnvelope = match serde_json::from_str(body) {
        Ok(e) => e,
        // Not JSON at all: a proxy error page, a captive portal, or an ISP
        // block page. Quote a bounded prefix so the operator can recognise it.
        Err(_) => {
            let preview: String = body.chars().take(120).collect();
            return Err(SendError::Other(redact(
                &format!("Telegram returned HTTP {status} with a non-JSON body: {preview}"),
                token,
            )));
        }
    };

    if let Some(params) = &envelope.parameters {
        if let Some(new_id) = params.migrate_to_chat_id {
            return Err(SendError::Migrated(new_id));
        }
        if let Some(secs) = params.retry_after {
            return Err(SendError::RateLimited(secs));
        }
    }

    if !envelope.ok {
        let why = envelope
            .description
            .unwrap_or_else(|| format!("HTTP {status} with no description"));
        return Err(SendError::Other(redact(
            &format!("Telegram rejected the request: {why}"),
            token,
        )));
    }

    envelope.result.ok_or_else(|| {
        SendError::Other("Telegram reported success but sent no result".to_string())
    })
}

/// POST a JSON body to one Bot API method.
///
/// Every failure path funnels through `redact`, so no caller can leak the token
/// by forwarding the error into a log or a toast.
fn post(
    client: &reqwest::blocking::Client,
    token: &str,
    method: &str,
    payload: &serde_json::Value,
) -> Result<serde_json::Value, SendError> {
    let response = client
        .post(api_url(token, method))
        .json(payload)
        .send()
        .map_err(|e| {
            SendError::Other(redact(&format!("Could not reach api.telegram.org: {e}"), token))
        })?;
    let status = response.status().as_u16();
    let body = response
        .text()
        .map_err(|e| SendError::Other(redact(&format!("Unreadable reply: {e}"), token)))?;
    interpret(status, &body, token)
}

/// Send one HTML-formatted message to the configured chat, returning Telegram's
/// `message_id`.
///
/// The id is what later lets a concatenated SMS be *edited* in place when its
/// remaining fragments arrive, instead of posting a second bubble.
pub fn send_message(
    client: &reqwest::blocking::Client,
    config: &TelegramConfig,
    html: &str,
) -> Result<i64, SendError> {
    let payload = serde_json::json!({
        "chat_id": config.chat_id,
        "text": html,
        "parse_mode": "HTML",
        // Link previews on a forwarded SMS are pure noise and would fetch
        // whatever URL a spam message chose to include.
        "link_preview_options": { "is_disabled": true },
    });
    let result = post(client, &config.bot_token, "sendMessage", &payload)?;
    result
        .get("message_id")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| SendError::Other("Telegram sent no message_id back".to_string()))
}

/// Confirm the token by asking Telegram who the bot is, returning its `@name`.
///
/// Worth its own call: it separates "this token is wrong" from "no group found",
/// which are the two ways setup goes wrong and have completely different fixes.
pub fn get_me(
    client: &reqwest::blocking::Client,
    token: &str,
) -> Result<String, SendError> {
    let result = post(client, token, "getMe", &serde_json::json!({}))?;
    result
        .get("username")
        .and_then(serde_json::Value::as_str)
        .map(|u| format!("@{u}"))
        .ok_or_else(|| SendError::Other("Telegram sent no bot username back".to_string()))
}

/// Pick the group to offer out of a `getUpdates` result array.
///
/// Scans newest-first, because the operator's last action is the one they mean.
/// Three update kinds carry a chat: `my_chat_member` (fires the moment the bot
/// is added, and is the only one that arrives with Group Privacy left at its
/// default), `message` and `channel_post`.
///
/// A `my_chat_member` whose new status is `left` or `kicked` is skipped — that
/// update means the bot was *removed* from that group, and offering it would
/// configure a destination the bot cannot post to.
fn pick_group(updates: &[serde_json::Value]) -> Option<DetectedGroup> {
    for update in updates.iter().rev() {
        for key in ["my_chat_member", "message", "channel_post"] {
            let Some(node) = update.get(key) else { continue };
            if key == "my_chat_member" {
                let status = node
                    .get("new_chat_member")
                    .and_then(|m| m.get("status"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                if status == "left" || status == "kicked" {
                    continue;
                }
            }
            let Some(chat) = node.get("chat") else { continue };
            let kind = chat.get("type").and_then(serde_json::Value::as_str).unwrap_or_default();
            if kind != "group" && kind != "supergroup" {
                continue;
            }
            let Some(chat_id) = chat.get("id").and_then(serde_json::Value::as_i64) else {
                continue;
            };
            return Some(DetectedGroup {
                chat_id,
                title: chat
                    .get("title")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("(untitled group)")
                    .to_string(),
                kind: kind.to_string(),
            });
        }
    }
    None
}

/// Guidance shown when `getUpdates` came back clean but held no group.
///
/// The empty case is the normal outcome of pressing the button too late rather
/// than of anything being broken, so the message has to say what to *do*:
/// Telegram keeps undelivered updates for 24 hours only, and with Group Privacy
/// at its default the bot cannot see ordinary group chatter — a command
/// addressed to it is the reliable way to produce a fresh update.
pub const NO_GROUP_FOUND: &str = "No group found. Add the bot to your private group, \
then send /start@your_bot inside that group and press Detect again. \
Telegram only keeps updates for 24 hours.";

/// Ask Telegram which group the bot was most recently added to.
///
/// `offset` is deliberately not sent: passing one acknowledges and discards
/// every earlier update, so a second press of Detect would find nothing.
pub fn detect_group(
    client: &reqwest::blocking::Client,
    token: &str,
) -> Result<DetectedGroup, SendError> {
    let payload = serde_json::json!({
        "allowed_updates": ["my_chat_member", "message", "channel_post"],
        "limit": 100,
        "timeout": 0,
    });
    let result = post(client, token, "getUpdates", &payload)?;
    let updates = result
        .as_array()
        .ok_or_else(|| SendError::Other("Telegram sent no update list back".to_string()))?;
    pick_group(updates).ok_or_else(|| SendError::Other(NO_GROUP_FOUND.to_string()))
}

/// The message the "Send Test Message" button posts.
///
/// Deliberately says which machine it came from: an operator running two banks
/// needs to know which one they just wired up.
pub fn test_message_html(host: &str) -> String {
    format!(
        "✅ <b>SIM Bank SMS Reader</b>\nForwarding is configured.\n\nHost: <code>{}</code>",
        escape_html(host)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "1234567890:AAF3xK-secret-part";

    /// The whole point of the module's error discipline: a transport failure
    /// quotes the URL, and the URL carries the token.
    #[test]
    fn redact_strips_the_token_from_a_reqwest_style_error() {
        let raw = format!(
            "Could not reach api.telegram.org: error sending request for url \
             (https://api.telegram.org/bot{TOKEN}/sendMessage)"
        );
        let safe = redact(&raw, TOKEN);
        assert!(!safe.contains("AAF3xK-secret-part"));
        assert!(safe.contains(REDACTED));
    }

    /// `"abc".replace("", x)` splices the replacement between every character.
    /// Before a token is configured that would shred the very message telling
    /// the operator to configure one.
    #[test]
    fn redact_leaves_text_alone_when_no_token_is_set() {
        assert_eq!(redact("Set a bot token first", ""), "Set a bot token first");
    }

    #[test]
    fn escape_html_handles_ampersand_before_the_angle_brackets() {
        assert_eq!(escape_html("a & b < c > d"), "a &amp; b &lt; c &gt; d");
        // Not double-escaped: the `&` of `&lt;` must survive as-is.
        assert_eq!(escape_html("<b>"), "&lt;b&gt;");
    }

    /// A body full of Markdown metacharacters is exactly what a promotional SMS
    /// looks like, and under `parse_mode: Markdown` it would 400 the send.
    #[test]
    fn escape_html_passes_markdown_metacharacters_through_untouched() {
        assert_eq!(escape_html("*100%* _off_ `now`"), "*100%* _off_ `now`");
    }

    #[test]
    fn interpret_returns_the_result_on_success() {
        let body = r#"{"ok":true,"result":{"message_id":42}}"#;
        let result = interpret(200, body, TOKEN).expect("should succeed");
        assert_eq!(result.get("message_id").and_then(|v| v.as_i64()), Some(42));
    }

    /// The supergroup upgrade. Recoverable without the operator: the caller
    /// saves the new id and retries, so this must never read as a plain error.
    #[test]
    fn interpret_classifies_a_supergroup_migration() {
        let body = r#"{"ok":false,"error_code":400,
            "description":"Bad Request: group chat was upgraded to a supergroup chat",
            "parameters":{"migrate_to_chat_id":-1001234567890}}"#;
        assert_eq!(
            interpret(400, body, TOKEN),
            Err(SendError::Migrated(-1001234567890))
        );
    }

    #[test]
    fn interpret_classifies_a_rate_limit_with_its_wait() {
        let body = r#"{"ok":false,"error_code":429,
            "description":"Too Many Requests: retry after 7",
            "parameters":{"retry_after":7}}"#;
        assert_eq!(interpret(429, body, TOKEN), Err(SendError::RateLimited(7)));
    }

    /// A wrong token is the most likely setup mistake, and Telegram's own
    /// wording is the most useful thing to show.
    #[test]
    fn interpret_surfaces_a_rejection_description() {
        let body = r#"{"ok":false,"error_code":401,"description":"Unauthorized"}"#;
        match interpret(401, body, TOKEN) {
            Err(SendError::Other(msg)) => assert!(msg.contains("Unauthorized"), "{msg}"),
            other => panic!("expected Other, got {other:?}"),
        }
    }

    /// An ISP block page or a captive portal answers HTML, not JSON. The
    /// operator has to be able to tell that apart from Telegram saying no.
    #[test]
    fn interpret_reports_a_non_json_body_with_a_bounded_preview() {
        let body = "<html><head><title>Blocked</title></head></html>";
        match interpret(403, body, TOKEN) {
            Err(SendError::Other(msg)) => {
                assert!(msg.contains("non-JSON"), "{msg}");
                assert!(msg.contains("Blocked"), "{msg}");
            }
            other => panic!("expected Other, got {other:?}"),
        }
    }

    /// A block page can be megabytes of markup; the log line must not be.
    #[test]
    fn interpret_does_not_quote_an_unbounded_error_page() {
        let body = "x".repeat(50_000);
        match interpret(500, &body, TOKEN) {
            Err(SendError::Other(msg)) => assert!(msg.len() < 300, "len {}", msg.len()),
            other => panic!("expected Other, got {other:?}"),
        }
    }

    fn updates(raw: &str) -> Vec<serde_json::Value> {
        serde_json::from_str(raw).expect("test fixture must parse")
    }

    /// `my_chat_member` is the only update kind that arrives with Group Privacy
    /// left at its default, so it is the one the button really depends on.
    #[test]
    fn pick_group_reads_a_my_chat_member_addition() {
        let list = updates(
            r#"[{"my_chat_member":{
                "new_chat_member":{"status":"member"},
                "chat":{"id":-1001,"type":"supergroup","title":"OTP Vault"}}}]"#,
        );
        assert_eq!(
            pick_group(&list),
            Some(DetectedGroup {
                chat_id: -1001,
                title: "OTP Vault".into(),
                kind: "supergroup".into(),
            })
        );
    }

    /// Being removed from a group also produces a `my_chat_member` update.
    /// Offering that group would configure a destination the bot cannot post to.
    #[test]
    fn pick_group_skips_a_group_the_bot_was_removed_from() {
        let list = updates(
            r#"[{"my_chat_member":{
                "new_chat_member":{"status":"left"},
                "chat":{"id":-1001,"type":"supergroup","title":"Old Group"}}}]"#,
        );
        assert_eq!(pick_group(&list), None);
    }

    /// Private chats are not destinations: a stranger pressing Start on the bot
    /// must never end up configured as the place OTPs go.
    #[test]
    fn pick_group_ignores_private_chats() {
        let list = updates(
            r#"[{"message":{"chat":{"id":12345,"type":"private","title":null}}}]"#,
        );
        assert_eq!(pick_group(&list), None);
    }

    /// Updates arrive oldest-first, and the operator means the group they just
    /// touched.
    #[test]
    fn pick_group_prefers_the_newest_group() {
        let list = updates(
            r#"[{"message":{"chat":{"id":-1001,"type":"group","title":"First"}}},
                {"message":{"chat":{"id":-1002,"type":"supergroup","title":"Second"}}}]"#,
        );
        assert_eq!(pick_group(&list).map(|g| g.chat_id), Some(-1002));
    }

    #[test]
    fn pick_group_returns_none_for_an_empty_update_list() {
        assert_eq!(pick_group(&[]), None);
    }

    #[test]
    fn test_message_escapes_its_host_name() {
        let html = test_message_html("bank-<01>");
        assert!(html.contains("bank-&lt;01&gt;"), "{html}");
    }

    /// Guards the TLS wiring, not the network.
    ///
    /// `reqwest` is declared with `rustls-no-provider`, which leaves the crypto
    /// provider to whatever the process already registered — the ring-backed
    /// rustls `tauri-plugin-updater` brings in. Building a client is where a
    /// missing provider surfaces, and it surfaces as a panic, so this test is the
    /// difference between finding that in CI and finding it when an operator
    /// presses Verify.
    #[test]
    fn build_client_succeeds_with_no_proxy() {
        assert!(build_client(None).is_ok());
    }

    /// Fails if the `socks` feature is ever dropped from Cargo.toml: without it
    /// `Proxy::all` rejects the scheme, and the one setting that makes this
    /// feature usable on a blocked network stops working.
    #[test]
    fn build_client_accepts_a_socks5h_proxy() {
        assert!(build_client(Some("socks5h://127.0.0.1:9050")).is_ok());
    }

    /// An empty or whitespace proxy field means "connect directly" — it is the
    /// default state of the setting and must not read as a configuration error.
    #[test]
    fn build_client_treats_a_blank_proxy_as_direct() {
        assert!(build_client(Some("   ")).is_ok());
    }

    #[test]
    fn build_client_rejects_a_malformed_proxy() {
        assert!(build_client(Some("not a url")).is_err());
    }
}
