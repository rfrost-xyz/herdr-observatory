//! Claude Code account attribution (design D4, D5): the environment name
//! rule, the configuration location checks, the `apiKeyHelper` presence
//! read, the three typed `~/.claude.json` extractions, the account and
//! session hashes and the ISO 8601 parser. Every reader takes the home
//! directory as a parameter, and every parse or validation error is a fixed
//! message, so no value read from a file can reach an error or output.
use crate::common;
use serde::Deserialize;
use serde::de::{Deserializer, IgnoredAny};
use std::ffi::{OsStr, OsString};
use std::path::Path;

/// Environment name patterns that refuse attribution (D4 step 1). `*`
/// matches zero or more characters; matching is case-sensitive.
const REFUSING: [&str; 10] = [
    "ANTHROPIC_*KEY*",
    "ANTHROPIC_*TOKEN*",
    "ANTHROPIC_CUSTOM_HEADERS",
    "ANTHROPIC_BASE_URL",
    "CLAUDE_CODE_*TOKEN*",
    "CLAUDE_CODE_*_FILE_DESCRIPTOR",
    "CLAUDE_CODE_HOST_*",
    "CCR_OAUTH_TOKEN_FILE",
    "CLAUDE_CODE_CUSTOM_OAUTH_URL",
    "CLAUDE_CODE_USE_*",
];
/// The closed exemption list (G3). No other name may be added without a new
/// decision.
pub const EXEMPT: [&str; 1] = ["CLAUDE_CODE_MESSAGING_TOKEN"];
/// The variable that moves Claude Code's configuration (D4 step 2).
const CONFIG_DIR: &str = "CLAUDE_CONFIG_DIR";
/// Bounds of the files read here (D4 step 4, D5).
const SETTINGS_LIMIT: usize = 1_048_576;
const PROVIDER_LIMIT: usize = 4 * 1_048_576;

/// The first refusing attribution step (design D4, D8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Environment,
    ConfigDir,
    LegacyConfig,
    ApiKeyHelper,
    ProviderState,
}
impl Refusal {
    /// The step name `--claude-attribution-check` prints.
    pub fn name(self) -> &'static str {
        match self {
            Self::Environment => "environment",
            Self::ConfigDir => "config-dir",
            Self::LegacyConfig => "legacy-config",
            Self::ApiKeyHelper => "api-key-helper",
            Self::ProviderState => "provider-state",
        }
    }
}

/// An ASCII glob where `*` matches zero or more bytes.
fn glob(pattern: &[u8], name: &[u8]) -> bool {
    match pattern.split_first() {
        None => name.is_empty(),
        Some((b'*', rest)) => (0..=name.len()).any(|skip| glob(rest, &name[skip..])),
        Some((byte, rest)) => name
            .split_first()
            .is_some_and(|(head, tail)| head == byte && glob(rest, tail)),
    }
}

/// The names of the process environment, read without their values.
pub fn environment_names() -> Vec<OsString> {
    use std::os::unix::ffi::OsStrExt;
    unsafe extern "C" {
        static environ: *const *const libc::c_char;
    }
    let mut names = Vec::new();
    // SAFETY: `environ` is a null-terminated array of C strings; the
    // reporter and the commands calling this never modify the environment.
    unsafe {
        let mut entry = environ;
        while !entry.is_null() && !(*entry).is_null() {
            let bytes = std::ffi::CStr::from_ptr(*entry).to_bytes();
            // As the standard library does, a leading `=` belongs to the name.
            let end = bytes
                .iter()
                .skip(1)
                .position(|&c| c == b'=')
                .map_or(bytes.len(), |index| index + 1);
            names.push(OsStr::from_bytes(&bytes[..end]).to_owned());
            entry = entry.add(1);
        }
    }
    names
}

/// The environment names matching a refusing pattern (D4 step 1), each with
/// whether it is exempt, and whether any name is not valid UTF-8.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Environment {
    pub matches: Vec<(String, bool)>,
    pub invalid: bool,
}
impl Environment {
    pub fn refuses(&self) -> bool {
        self.invalid || self.matches.iter().any(|(_, exempt)| !exempt)
    }
}
/// D4 step 1 over `names`, which are names only, never values.
pub fn environment<S: AsRef<OsStr>>(names: &[S]) -> Environment {
    let mut result = Environment::default();
    for name in names {
        let Some(name) = name.as_ref().to_str() else {
            result.invalid = true;
            continue;
        };
        if REFUSING
            .iter()
            .any(|pattern| glob(pattern.as_bytes(), name.as_bytes()))
        {
            result
                .matches
                .push((name.to_owned(), EXEMPT.contains(&name)));
        }
    }
    result
}

/// D4 steps 2 and 3, the only steps the collector, identity refresh and
/// `--claude-account-key` apply: `CLAUDE_CONFIG_DIR` set in `names` (any
/// value), or a legacy `<home>/.claude/.config.json` in any form.
pub fn location_refusal<S: AsRef<OsStr>>(names: &[S], home: &Path) -> Option<Refusal> {
    if names.iter().any(|name| name.as_ref() == CONFIG_DIR) {
        return Some(Refusal::ConfigDir);
    }
    // No read and no link following: a dangling link still refuses.
    match std::fs::symlink_metadata(home.join(".claude/.config.json")) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        _ => Some(Refusal::LegacyConfig),
    }
}

/// Key presence with any value, including `null` (D5). `Option<IgnoredAny>`
/// would map `null` to absent.
#[derive(Default)]
struct Present(bool);
impl<'de> Deserialize<'de> for Present {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        IgnoredAny::deserialize(deserializer)?;
        Ok(Self(true))
    }
}

/// A top-level JSON object parsed as `T`. A derived struct alone would also
/// accept an array.
struct Object<T>(T);
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Fields<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Fields<T> {
            type Value = T;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an object")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
                T::deserialize(serde::de::value::MapAccessDeserializer::new(map))
            }
        }
        deserializer
            .deserialize_map(Fields(std::marker::PhantomData))
            .map(Object)
    }
}

/// Settings: only whether `apiKeyHelper` is present.
#[derive(Deserialize)]
struct Settings {
    #[serde(rename = "apiKeyHelper", default)]
    api_key_helper: Present,
}
/// D4 step 4: `<home>/.claude/settings.json` absent passes; present, it
/// refuses when `apiKeyHelper` is set (any value) or the file is unsafe,
/// oversized or malformed. Nothing else is extracted.
pub fn api_key_helper_refuses(home: &Path) -> bool {
    let path = home.join(".claude/settings.json");
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return false,
        Err(_) => return true,
        Ok(_) => {}
    }
    let Ok(bytes) = common::read_owned(&path, SETTINGS_LIMIT, false) else {
        return true;
    };
    serde_json::from_slice::<Object<Settings>>(&bytes)
        .map(|settings| settings.0.api_key_helper.0)
        .unwrap_or(true)
}

/// The account id and `primaryApiKey` presence: the reporter, key command
/// and attribution check extraction.
#[derive(Deserialize)]
struct AccountFile {
    #[serde(rename = "oauthAccount", default)]
    oauth: Option<AccountId>,
    #[serde(rename = "primaryApiKey", default)]
    api_key: Present,
}
#[derive(Deserialize)]
struct AccountId {
    #[serde(rename = "accountUuid", default)]
    uuid: Option<String>,
}
/// The identity refresh extraction: the account id, email and
/// `primaryApiKey` presence.
#[derive(Deserialize)]
struct IdentityFile {
    #[serde(rename = "oauthAccount", default)]
    oauth: Option<IdentityAccount>,
    #[serde(rename = "primaryApiKey", default)]
    api_key: Present,
}
#[derive(Deserialize)]
struct IdentityAccount {
    #[serde(rename = "accountUuid", default)]
    uuid: Option<String>,
    #[serde(rename = "emailAddress", default)]
    email: Option<String>,
}
/// The collector extraction: the account id, `primaryApiKey` presence and
/// the two cache windows. `limits`, spend, dollar and every other field are
/// skipped.
#[derive(Deserialize)]
struct CollectorFile {
    #[serde(rename = "oauthAccount", default)]
    oauth: Option<AccountId>,
    #[serde(rename = "primaryApiKey", default)]
    api_key: Present,
    #[serde(rename = "cachedUsageUtilization", default)]
    cache: Option<CacheFile>,
}
#[derive(Deserialize)]
struct CacheFile {
    #[serde(rename = "accountUuid", default)]
    uuid: Option<String>,
    #[serde(rename = "fetchedAtMs", default)]
    fetched_at_ms: Option<f64>,
    #[serde(default)]
    utilization: Option<CacheWindows>,
}
#[derive(Deserialize)]
struct CacheWindows {
    #[serde(default)]
    five_hour: Option<CacheWindowFile>,
    #[serde(default)]
    seven_day: Option<CacheWindowFile>,
}
#[derive(Deserialize)]
struct CacheWindowFile {
    #[serde(default)]
    utilization: Option<f64>,
    #[serde(default)]
    resets_at: Option<String>,
}

const UNREADABLE: &str = "Provider state unavailable";
const MALFORMED: &str = "Provider state malformed";
const API_KEY: &str = "Provider state names an API key";
const NO_ACCOUNT: &str = "Provider state has no valid account";

/// `<home>/.claude.json` parsed into `T` (D5): no-follow, owned by the
/// effective user, private and at most 4 MiB.
fn provider_state<T: for<'de> Deserialize<'de>>(home: &Path) -> Result<T, &'static str> {
    let bytes = common::read_owned(&home.join(".claude.json"), PROVIDER_LIMIT, true)
        .map_err(|_| UNREADABLE)?;
    serde_json::from_slice::<Object<T>>(&bytes)
        .map(|object| object.0)
        .map_err(|_| MALFORMED)
}
/// A valid account id: 1 to 256 characters with no control character.
fn valid_id(id: &str) -> bool {
    (1..=256).contains(&id.chars().count()) && !id.chars().any(char::is_control)
}
/// The account key for a valid id; the id is not kept.
pub fn account_key(id: &str) -> String {
    common::sha256(format!("observatory-claude-account-v1:{id}").as_bytes())
}
/// The hashed session id stored in the account state file (D6).
pub fn session_key(id: &str) -> String {
    common::sha256(format!("observatory-claude-session-v1:{id}").as_bytes())
}
fn key_of(uuid: Option<&str>, api_key: &Present) -> Result<String, &'static str> {
    if api_key.0 {
        return Err(API_KEY);
    }
    uuid.filter(|id| valid_id(id))
        .map(account_key)
        .ok_or(NO_ACCOUNT)
}
/// The reporter, key command and attribution check extraction: the account
/// key, or a fixed error.
pub fn account(home: &Path) -> Result<String, &'static str> {
    let file: AccountFile = provider_state(home)?;
    key_of(
        file.oauth.as_ref().and_then(|o| o.uuid.as_deref()),
        &file.api_key,
    )
}
/// The identity refresh extraction: the account key and the unchecked
/// email, if any.
pub fn identity(home: &Path) -> Result<(String, Option<String>), &'static str> {
    let file: IdentityFile = provider_state(home)?;
    let key = key_of(
        file.oauth.as_ref().and_then(|o| o.uuid.as_deref()),
        &file.api_key,
    )?;
    Ok((key, file.oauth.and_then(|o| o.email)))
}
/// One cache window: the raw utilisation and the parsed reset, in epoch
/// seconds. A malformed or absent reset drops the window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CacheWindow {
    pub utilization: Option<f64>,
    pub resets_at: i64,
}
/// Claude Code's usage cache for the profile's own account.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cache {
    pub fetched_at_ms: f64,
    pub five_hour: Option<CacheWindow>,
    pub seven_day: Option<CacheWindow>,
}
/// The collector extraction: the account key and, only when the cache
/// names the same account, its two windows. The ids are dropped here.
pub fn collector(home: &Path) -> Result<(String, Option<Cache>), &'static str> {
    let file: CollectorFile = provider_state(home)?;
    let uuid = file.oauth.as_ref().and_then(|o| o.uuid.as_deref());
    let key = key_of(uuid, &file.api_key)?;
    let window = |window: Option<CacheWindowFile>| {
        let window = window?;
        Some(CacheWindow {
            utilization: window.utilization,
            resets_at: iso_seconds(window.resets_at.as_deref()?)?,
        })
    };
    let cache = file.cache.and_then(|cache| {
        if cache.uuid.as_deref() != uuid {
            return None;
        }
        let windows = cache.utilization;
        let (five_hour, seven_day) = match windows {
            Some(w) => (window(w.five_hour), window(w.seven_day)),
            None => (None, None),
        };
        Some(Cache {
            fetched_at_ms: cache.fetched_at_ms.filter(|v| v.is_finite())?,
            five_hour,
            seven_day,
        })
    });
    Ok((key, cache))
}

/// D4 steps 1 to 5 against `names` and `home`: the account key, or the
/// first refusing step. Session memory (step 6) belongs to the state file.
pub fn attribution<S: AsRef<OsStr>>(names: &[S], home: &Path) -> Result<String, Refusal> {
    if environment(names).refuses() {
        return Err(Refusal::Environment);
    }
    if let Some(refusal) = location_refusal(names, home) {
        return Err(refusal);
    }
    if api_key_helper_refuses(home) {
        return Err(Refusal::ApiKeyHelper);
    }
    account(home).map_err(|_| Refusal::ProviderState)
}

/// Days since 1970-01-01 of a proleptic Gregorian date.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
/// An ISO 8601 time under the D2 grammar
/// `^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{1,9})?(Z|[+-]\d{2}:\d{2})$`
/// with checked ranges, as epoch seconds: the fraction truncated, the offset
/// subtracted.
pub fn iso_seconds(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    let number = |range: std::ops::Range<usize>| -> Option<i64> {
        let part = bytes.get(range)?;
        if !part.iter().all(u8::is_ascii_digit) {
            return None;
        }
        part.iter()
            .try_fold(0i64, |sum, digit| Some(sum * 10 + i64::from(digit - b'0')))
    };
    if bytes.len() < 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return None;
    }
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    let mut rest = &bytes[19..];
    if let Some(fraction) = rest.strip_prefix(b".") {
        let digits = fraction.iter().take_while(|c| c.is_ascii_digit()).count();
        if !(1..=9).contains(&digits) {
            return None;
        }
        rest = &fraction[digits..];
    }
    let offset = match rest {
        b"Z" => 0,
        [sign @ (b'+' | b'-'), h1, h2, b':', m1, m2] => {
            let digit = |c: &u8| c.is_ascii_digit().then(|| i64::from(c - b'0'));
            let hours = digit(h1)? * 10 + digit(h2)?;
            let minutes = digit(m1)? * 10 + digit(m2)?;
            if hours > 23 || minutes > 59 {
                return None;
            }
            let offset = hours * 3600 + minutes * 60;
            if *sign == b'+' { offset } else { -offset }
        }
        _ => return None,
    };
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if !(1..=days_in_month).contains(&day) || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second - offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    const UUID: &str = "00000000-0000-4000-8000-000000000001";
    const EMAIL: &str = "fixture@example.invalid";
    const KEY: &str = "9d1a492dac66ac8c1ae96fe7261a03fc1414d474fe04fc3b8738b28d73e2a090";

    /// A private temporary home, removed on drop.
    struct Home(PathBuf);
    impl Home {
        fn new() -> Self {
            static SEQUENCE: std::sync::atomic::AtomicUsize =
                std::sync::atomic::AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "anton-claude-account-{}-{}-{}",
                std::process::id(),
                common::now().to_bits(),
                SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            fs::create_dir(path.join(".claude")).unwrap();
            Self(path)
        }
        fn write(&self, name: &str, bytes: &[u8], mode: u32) {
            let path = self.0.join(name);
            fs::write(&path, bytes).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        }
        fn provider(&self, value: &str) {
            self.write(".claude.json", value.as_bytes(), 0o600);
        }
        fn settings(&self, value: &str) {
            self.write(".claude/settings.json", value.as_bytes(), 0o644);
        }
    }
    impl Drop for Home {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    /// A synthetic `.claude.json` with `PRIVATE` markers in unlisted keys.
    fn provider_json(extra: &str) -> String {
        format!(
            r#"{{"numStartups":3,"projects":{{"/PRIVATE/path":{{"history":["PRIVATE"]}}}},"oauthAccount":{{"accountUuid":"{UUID}","emailAddress":"{EMAIL}","organizationName":"PRIVATE","organizationUuid":"PRIVATE"}}{extra}}}"#
        )
    }
    /// Asserts no fixture secret appears in `text`.
    fn clean(text: &str) {
        for secret in ["PRIVATE", UUID, EMAIL] {
            assert!(!text.contains(secret), "{secret} in {text}");
        }
    }

    #[test]
    fn hashes_use_their_own_prefixes() {
        assert_eq!(account_key(UUID), KEY);
        assert_eq!(
            session_key("fixture-session"),
            "5d9b9198bdba7f3ce73e63ffde1f256b9f1b6f9a5448946ebef88a6c2a52a9e0"
        );
        assert_ne!(account_key("x"), session_key("x"));
    }

    /// D4 step 1: every pattern refuses, globs match zero characters, the
    /// single exemption passes alone, and only names are considered.
    #[test]
    fn environment_names_refuse_by_pattern_with_one_exemption() {
        for name in [
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_KEY",
            "ANTHROPIC_AUTH_TOKEN",
            "ANTHROPIC_TOKEN",
            "ANTHROPIC_CUSTOM_HEADERS",
            "ANTHROPIC_BASE_URL",
            "CLAUDE_CODE_OAUTH_TOKEN",
            "CLAUDE_CODE_TOKEN",
            "CLAUDE_CODE_API_KEY_FILE_DESCRIPTOR",
            "CLAUDE_CODE__FILE_DESCRIPTOR",
            "CLAUDE_CODE_HOST_",
            "CLAUDE_CODE_HOST_PLATFORM",
            "CCR_OAUTH_TOKEN_FILE",
            "CLAUDE_CODE_CUSTOM_OAUTH_URL",
            "CLAUDE_CODE_USE_BEDROCK",
            "CLAUDE_CODE_USE_",
        ] {
            let result = environment(&["PATH", "HOME", name]);
            assert!(result.refuses(), "{name}");
            assert_eq!(result.matches, [(name.to_owned(), false)], "{name}");
        }
        for name in [
            "PATH",
            "ANTHROPIC_BASE_URL_X",
            "X_ANTHROPIC_BASE_URL",
            "ANTHROPIC_MODEL",
            "anthropic_api_key",
            "CLAUDE_CODE_FILE_DESCRIPTOR",
            "CLAUDE_CODE_ENTRYPOINT",
            "CLAUDE_CODE_HOST",
            "CLAUDE_CODE_USE",
            "CCR_OAUTH_TOKEN_FILE_X",
            "CLAUDE_CODE_CUSTOM_OAUTH_URL_X",
            "CLAUDE_CONFIG_DIR",
        ] {
            let result = environment(&[name]);
            assert!(!result.refuses(), "{name}");
            assert!(result.matches.is_empty(), "{name}");
        }
        // The exemption alone passes and is reported as exempt.
        let exempt = environment(&["CLAUDE_CODE_MESSAGING_TOKEN", "PATH"]);
        assert!(!exempt.refuses());
        assert_eq!(
            exempt.matches,
            [("CLAUDE_CODE_MESSAGING_TOKEN".to_owned(), true)]
        );
        // Beside it, a non-exempt `CLAUDE_CODE_*TOKEN*` still refuses.
        let both = environment(&["CLAUDE_CODE_MESSAGING_TOKEN", "CLAUDE_CODE_SESSION_TOKEN"]);
        assert!(both.refuses());
        assert_eq!(
            both.matches,
            [
                ("CLAUDE_CODE_MESSAGING_TOKEN".to_owned(), true),
                ("CLAUDE_CODE_SESSION_TOKEN".to_owned(), false)
            ]
        );
        // A name that is not valid UTF-8 refuses.
        use std::os::unix::ffi::OsStrExt;
        let invalid = [OsStr::from_bytes(b"PATH"), OsStr::from_bytes(b"X\xff")];
        let result = environment(&invalid);
        assert!(result.invalid && result.refuses());
        assert!(!environment(&[OsStr::from_bytes(b"PATH")]).refuses());
    }

    /// The process environment's names, read without values, match the
    /// standard library's view.
    #[test]
    fn environment_names_are_the_process_names() {
        let mut ours = environment_names();
        let mut std: Vec<OsString> = std::env::vars_os().map(|(name, _)| name).collect();
        ours.sort();
        std.sort();
        assert_eq!(ours, std);
        assert!(
            ours.iter()
                .all(|name| !name.to_string_lossy().contains('='))
        );
    }

    /// D4 steps 2 and 3: `CLAUDE_CONFIG_DIR` by name, and a legacy file in
    /// any form, including a dangling link, refuse.
    #[test]
    fn configuration_location_refuses_config_dir_and_legacy_file() {
        let home = Home::new();
        assert_eq!(location_refusal(&["PATH"], &home.0), None);
        assert_eq!(
            location_refusal(&["PATH", "CLAUDE_CONFIG_DIR"], &home.0),
            Some(Refusal::ConfigDir)
        );
        home.write(".claude/.config.json", b"{}", 0o600);
        assert_eq!(
            location_refusal(&["PATH"], &home.0),
            Some(Refusal::LegacyConfig)
        );
        fs::remove_file(home.0.join(".claude/.config.json")).unwrap();
        std::os::unix::fs::symlink(home.0.join("absent"), home.0.join(".claude/.config.json"))
            .unwrap();
        assert_eq!(
            location_refusal(&["PATH"], &home.0),
            Some(Refusal::LegacyConfig)
        );
        fs::remove_file(home.0.join(".claude/.config.json")).unwrap();
        fs::create_dir(home.0.join(".claude/.config.json")).unwrap();
        assert_eq!(
            location_refusal(&["PATH"], &home.0),
            Some(Refusal::LegacyConfig)
        );
    }

    /// D4 step 4: absent settings or key pass; any `apiKeyHelper` value,
    /// including `null` and a mistyped one, refuses, as do a link, an
    /// oversized file and malformed JSON.
    #[test]
    fn api_key_helper_presence_refuses() {
        let home = Home::new();
        assert!(!api_key_helper_refuses(&home.0));
        home.settings(r#"{"env":{"PRIVATE":"PRIVATE"},"model":"PRIVATE"}"#);
        assert!(!api_key_helper_refuses(&home.0));
        for helper in [r#""/PRIVATE/helper""#, "null", r#"{"PRIVATE":[1,2]}"#, "7"] {
            home.settings(&format!(r#"{{"env":{{}},"apiKeyHelper":{helper}}}"#));
            assert!(api_key_helper_refuses(&home.0), "{helper}");
        }
        for malformed in [
            "",
            "[]",
            "{",
            r#"{"apiKeyHelper":1,"apiKeyHelper":2}"#,
            "null",
        ] {
            home.settings(malformed);
            assert!(api_key_helper_refuses(&home.0), "{malformed}");
        }
        let mut big = String::from(r#"{"pad":""#);
        big.push_str(&"x".repeat(SETTINGS_LIMIT));
        big.push_str("\"}");
        home.settings(&big);
        assert!(api_key_helper_refuses(&home.0));
        fs::remove_file(home.0.join(".claude/settings.json")).unwrap();
        home.write("real-settings.json", b"{}", 0o644);
        std::os::unix::fs::symlink(
            home.0.join("real-settings.json"),
            home.0.join(".claude/settings.json"),
        )
        .unwrap();
        assert!(api_key_helper_refuses(&home.0));
    }

    /// D5: the account key from a valid id; `primaryApiKey` with any value,
    /// a missing or invalid id, a link, loose mode, over 4 MiB or mistyped
    /// allowlisted fields give fixed errors with no value in them.
    #[test]
    fn provider_state_gives_only_the_key_or_a_fixed_error() {
        let home = Home::new();
        assert_eq!(account(&home.0), Err(UNREADABLE));
        home.provider(&provider_json(""));
        assert_eq!(account(&home.0).as_deref(), Ok(KEY));
        for (extra, error) in [
            (r#","primaryApiKey":"PRIVATE""#, API_KEY),
            (r#","primaryApiKey":null"#, API_KEY),
            (r#","primaryApiKey":{"PRIVATE":1}"#, API_KEY),
        ] {
            home.provider(&provider_json(extra));
            assert_eq!(account(&home.0), Err(error), "{extra}");
        }
        let long = "a".repeat(257);
        let exact = "a".repeat(256);
        for (body, result) in [
            (r#"{"projects":{}}"#.to_owned(), Err(NO_ACCOUNT)),
            (r#"{"oauthAccount":null}"#.to_owned(), Err(NO_ACCOUNT)),
            (
                r#"{"oauthAccount":{"emailAddress":"PRIVATE"}}"#.to_owned(),
                Err(NO_ACCOUNT),
            ),
            (
                r#"{"oauthAccount":{"accountUuid":""}}"#.to_owned(),
                Err(NO_ACCOUNT),
            ),
            (
                format!(r#"{{"oauthAccount":{{"accountUuid":"{long}"}}}}"#),
                Err(NO_ACCOUNT),
            ),
            (
                r#"{"oauthAccount":{"accountUuid":"PRIVATE\u0007"}}"#.to_owned(),
                Err(NO_ACCOUNT),
            ),
            (
                r#"{"oauthAccount":{"accountUuid":["PRIVATE"]}}"#.to_owned(),
                Err(MALFORMED),
            ),
            (r#"{"oauthAccount":"PRIVATE"}"#.to_owned(), Err(MALFORMED)),
            ("PRIVATE".to_owned(), Err(MALFORMED)),
            ("[]".to_owned(), Err(MALFORMED)),
            (format!(r#"[{{"accountUuid":"{UUID}"}}]"#), Err(MALFORMED)),
            (
                format!(r#"{{"oauthAccount":{{"accountUuid":"{exact}"}}}}"#),
                Ok(account_key(&exact)),
            ),
        ] {
            home.provider(&body);
            assert_eq!(account(&home.0), result, "{body}");
        }
        // Loose mode, a link and over 4 MiB.
        home.write(".claude.json", provider_json("").as_bytes(), 0o644);
        assert_eq!(account(&home.0), Err(UNREADABLE));
        fs::remove_file(home.0.join(".claude.json")).unwrap();
        home.write("real.json", provider_json("").as_bytes(), 0o600);
        std::os::unix::fs::symlink(home.0.join("real.json"), home.0.join(".claude.json")).unwrap();
        assert_eq!(account(&home.0), Err(UNREADABLE));
        fs::remove_file(home.0.join(".claude.json")).unwrap();
        let pad = "x".repeat(PROVIDER_LIMIT);
        home.provider(&provider_json(&format!(r#","pad":"{pad}""#)));
        assert_eq!(account(&home.0), Err(UNREADABLE));
        // Just under the bound is read.
        let body = provider_json(r#","pad":"""#);
        let pad = "x".repeat(PROVIDER_LIMIT - body.len());
        home.provider(&provider_json(&format!(r#","pad":"{pad}""#)));
        assert_eq!(account(&home.0).as_deref(), Ok(KEY));
        for error in [UNREADABLE, MALFORMED, API_KEY, NO_ACCOUNT] {
            clean(error);
        }
    }

    /// D5: identity refresh gets the key and email; the key extraction
    /// ignores a mistyped email that the identity extraction rejects.
    #[test]
    fn identity_extraction_adds_only_the_email() {
        let home = Home::new();
        home.provider(&provider_json(""));
        assert_eq!(
            identity(&home.0),
            Ok((KEY.to_owned(), Some(EMAIL.to_owned())))
        );
        home.provider(&format!(r#"{{"oauthAccount":{{"accountUuid":"{UUID}"}}}}"#));
        assert_eq!(identity(&home.0), Ok((KEY.to_owned(), None)));
        home.provider(&format!(
            r#"{{"oauthAccount":{{"accountUuid":"{UUID}","emailAddress":{{"PRIVATE":1}}}}}}"#
        ));
        assert_eq!(identity(&home.0), Err(MALFORMED));
        assert_eq!(account(&home.0).as_deref(), Ok(KEY));
        home.provider(&provider_json(r#","primaryApiKey":null"#));
        assert_eq!(identity(&home.0), Err(API_KEY));
    }

    /// D5, D7: the collector reads the cache only for the profile's own
    /// account, parses resets with the D2 grammar and skips every other
    /// field; mistyped allowlisted fields give a fixed error.
    #[test]
    fn collector_extraction_keeps_only_the_matched_cache_windows() {
        let home = Home::new();
        let cache = |uuid: &str, fetched: &str, five: &str| {
            format!(
                r#","cachedUsageUtilization":{{"accountUuid":"{uuid}","fetchedAtMs":{fetched},"utilization":{{"five_hour":{five},"seven_day":{{"utilization":40,"resets_at":"2026-10-04T12:00:00.999+01:00","extra_usage_usd":"PRIVATE","locked_reason":null}},"seven_day_opus":"PRIVATE","limits":[{{"kind":"PRIVATE","percent":5}}]}},"spend":"PRIVATE"}}"#
            )
        };
        let five = r#"{"utilization":12.5,"resets_at":"2024-02-29T23:59:59Z"}"#;
        home.provider(&provider_json(&cache(UUID, "1800000000000", five)));
        let (key, cache_read) = collector(&home.0).unwrap();
        assert_eq!(key, KEY);
        let expected = Cache {
            fetched_at_ms: 1_800_000_000_000.0,
            five_hour: Some(CacheWindow {
                utilization: Some(12.5),
                resets_at: 1_709_251_199,
            }),
            seven_day: Some(CacheWindow {
                utilization: Some(40.0),
                resets_at: 1_791_111_600,
            }),
        };
        assert_eq!(cache_read, Some(expected));
        clean(&format!("{cache_read:?}"));
        // Another account's cache is not used; the key still is.
        home.provider(&provider_json(&cache(
            "PRIVATE-other",
            "1800000000000",
            five,
        )));
        assert_eq!(collector(&home.0), Ok((KEY.to_owned(), None)));
        // A malformed reset drops that window only.
        let bad = r#"{"utilization":1,"resets_at":"2026-02-30T00:00:00Z"}"#;
        home.provider(&provider_json(&cache(UUID, "1800000000000", bad)));
        let (_, read) = collector(&home.0).unwrap();
        let read = read.unwrap();
        assert_eq!((read.five_hour, read.seven_day), (None, expected.seven_day));
        // Mistyped `fetchedAtMs` or `accountUuid`: a fixed error.
        home.provider(&provider_json(&cache(UUID, r#""PRIVATE""#, five)));
        assert_eq!(collector(&home.0), Err(MALFORMED));
        home.provider(&provider_json(
            r#","cachedUsageUtilization":{"accountUuid":7,"fetchedAtMs":1}"#,
        ));
        assert_eq!(collector(&home.0), Err(MALFORMED));
        // `primaryApiKey` skips the whole file, cache included.
        home.provider(&provider_json(&format!(
            r#"{},"primaryApiKey":"PRIVATE""#,
            cache(UUID, "1800000000000", five)
        )));
        assert_eq!(collector(&home.0), Err(API_KEY));
    }

    /// D4 steps 1 to 5 in order; the first refusing step is named.
    #[test]
    fn attribution_names_the_first_refusing_step() {
        let home = Home::new();
        home.provider(&provider_json(""));
        assert_eq!(attribution(&["PATH"], &home.0).as_deref(), Ok(KEY));
        assert_eq!(
            attribution(&["PATH", "CLAUDE_CODE_MESSAGING_TOKEN"], &home.0).as_deref(),
            Ok(KEY)
        );
        assert_eq!(
            attribution(&["ANTHROPIC_BASE_URL", "CLAUDE_CONFIG_DIR"], &home.0),
            Err(Refusal::Environment)
        );
        assert_eq!(
            attribution(&["ANTHROPIC_BASE_URL_X"], &home.0).as_deref(),
            Ok(KEY)
        );
        assert_eq!(
            attribution(&["CLAUDE_CONFIG_DIR"], &home.0),
            Err(Refusal::ConfigDir)
        );
        home.settings(r#"{"apiKeyHelper":"PRIVATE"}"#);
        assert_eq!(attribution(&["PATH"], &home.0), Err(Refusal::ApiKeyHelper));
        home.settings("{}");
        home.provider(&provider_json(r#","primaryApiKey":"PRIVATE""#));
        assert_eq!(attribution(&["PATH"], &home.0), Err(Refusal::ProviderState));
        home.write(".claude/.config.json", b"{}", 0o600);
        assert_eq!(attribution(&["PATH"], &home.0), Err(Refusal::LegacyConfig));
        let names: Vec<_> = [
            Refusal::Environment,
            Refusal::ConfigDir,
            Refusal::LegacyConfig,
            Refusal::ApiKeyHelper,
            Refusal::ProviderState,
        ]
        .map(Refusal::name)
        .into();
        assert_eq!(
            names,
            [
                "environment",
                "config-dir",
                "legacy-config",
                "api-key-helper",
                "provider-state"
            ]
        );
    }

    /// D2, D7: the ISO 8601 grammar with checked ranges, truncated
    /// fractions and offsets subtracted.
    #[test]
    fn iso_times_follow_the_grammar() {
        for (value, seconds) in [
            ("1970-01-01T00:00:00Z", 0),
            ("2024-02-29T23:59:59Z", 1_709_251_199),
            ("2026-10-04T12:00:00.999+01:00", 1_791_111_600),
            ("2000-02-29T00:00:00-23:59", 951_868_740),
            ("1999-12-31T23:59:59.123456789+23:59", 946_598_459),
            ("2026-10-04T11:00:00.5Z", 1_791_111_600),
        ] {
            assert_eq!(iso_seconds(value), Some(seconds), "{value}");
        }
        for value in [
            "",
            "2026-10-04",
            "2026-10-04T12:00Z",
            "2026-10-04 12:00:00Z",
            "2026-10-04t12:00:00Z",
            "2026-10-04T12:00:00z",
            "2026-10-04T12:00:00",
            "2026-10-04T12:00:00+0100",
            "2026-10-04T12:00:00+01",
            "2026-10-04T12:00:00.Z",
            "2026-10-04T12:00:00.1234567890Z",
            "2026-10-04T12:00:00Z ",
            " 2026-10-04T12:00:00Z",
            "2026-13-01T00:00:00Z",
            "2026-00-01T00:00:00Z",
            "2026-04-31T00:00:00Z",
            "2023-02-29T00:00:00Z",
            "2100-02-29T00:00:00Z",
            "2026-10-00T00:00:00Z",
            "2026-10-04T24:00:00Z",
            "2026-10-04T12:60:00Z",
            "2026-10-04T12:00:60Z",
            "2026-10-04T12:00:00+24:00",
            "2026-10-04T12:00:00-00:60",
            "+026-10-04T12:00:00Z",
            "2026-1-04T12:00:00Z",
            "２026-10-04T12:00:00Z",
        ] {
            assert_eq!(iso_seconds(value), None, "{value}");
        }
    }
}
