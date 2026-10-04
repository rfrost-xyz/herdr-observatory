//! Explicit local-only account identity. Never included in collector snapshots.
use crate::{Result, allowances, claude_account, common};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

fn email(value: &str) -> bool {
    if value.chars().count() > 254
        || value
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '<' | '>'))
    {
        return false;
    }
    let mut parts = value.split('@');
    let Some(local) = parts.next() else {
        return false;
    };
    let Some(domain) = parts.next() else {
        return false;
    };
    !local.is_empty()
        && parts.next().is_none()
        && domain
            .rsplit_once('.')
            .is_some_and(|(left, right)| !left.is_empty() && !right.is_empty())
}
pub fn identity(account: &Value, limits: &Value) -> Option<Value> {
    let account = &account["account"];
    if account["type"] != "chatgpt" {
        return None;
    }
    let key = allowances::account_key(limits["accountId"].as_str()?)?;
    let email = account["email"].as_str().filter(|v| email(v))?;
    Some(json!({"account_key":key,"email":email}))
}
/// Emails of the Codex RPC and peer `--identity-probe` rows, by mapping
/// id. Only Codex mappings match, so such a row carrying a Claude
/// mapping's key stores nothing (D8).
pub fn mapped(rows: &[Value], accounts: &Value) -> Value {
    let mut result = serde_json::Map::new();
    for row in rows {
        let Some(key) = row["account_key"].as_str() else {
            continue;
        };
        let Some(mapping) = allowances::mapped(accounts, key, "codex") else {
            continue;
        };
        let Some(address) = row["email"].as_str().filter(|v| email(v)) else {
            continue;
        };
        result.insert(mapping["id"].as_str().unwrap().to_owned(), json!(address));
    }
    Value::Object(result)
}
pub fn probe(cancel: Option<&AtomicBool>) -> Result<Value> {
    let replies = allowances::codex_rpc(true, cancel)?;
    identity(
        replies.get(&3).unwrap_or(&Value::Null),
        replies.get(&2).unwrap_or(&Value::Null),
    )
    .ok_or("Account identity unavailable".into())
}
/// The local Claude email (D8) as (mapping id, email): `read` runs only
/// when `accounts` maps a Claude account, `home` is known and D4 steps 2
/// and 3 pass for `names`; the email is kept only for a key that a Claude
/// mapping names and only when it passes the email check.
fn claude_email<S: AsRef<OsStr>>(
    accounts: &Value,
    names: &[S],
    home: Option<&Path>,
    read: impl FnOnce(&Path) -> std::result::Result<(String, Option<String>), &'static str>,
) -> Option<(String, String)> {
    let claude = accounts.as_object()?.values().any(|value| {
        allowances::mapping(value).is_ok_and(|mapping| allowances::provider(&mapping) == "claude")
    });
    if !claude {
        return None;
    }
    let home = home?;
    if claude_account::location_refusal(names, home).is_some() {
        return None;
    }
    let (key, address) = read(home).ok()?;
    let mapping = allowances::mapped(accounts, &key, "claude")?;
    let address = address.filter(|v| email(v))?;
    Some((mapping["id"].as_str()?.to_owned(), address))
}
pub fn refresh(config: &Value, output: &Path, cancel: Option<&AtomicBool>) -> Result<usize> {
    let cfg = allowances::configuration(config).ok_or("No configured allowance identities")?;
    allowances::validate_config(Some(cfg))?;
    let mut rows = Vec::new();
    if let Ok(row) = probe(cancel) {
        rows.push(row);
    }
    let mut seen = BTreeSet::new();
    for source in cfg["sources"].as_array().into_iter().flatten().take(16) {
        if cancel.is_some_and(|value| value.load(Ordering::Relaxed)) {
            return Err("Identity read cancelled".into());
        }
        let Some(target) = source["target"]
            .as_str()
            .filter(|v| allowances::valid_target(v))
        else {
            continue;
        };
        if !seen.insert(target) {
            continue;
        }
        let command = vec![
            "ssh".into(),
            "-T".into(),
            "-o".into(),
            "BatchMode=yes".into(),
            "-o".into(),
            "ConnectTimeout=5".into(),
            "--".into(),
            target.into(),
            format!("exec {} --identity-probe", allowances::PEER),
        ];
        if let Ok(bytes) =
            common::run_bounded(&command, b"{}", Duration::from_secs(12), 4096, cancel)
        {
            if let Ok(row) = serde_json::from_slice::<Value>(&bytes) {
                rows.push(row);
            }
        }
    }
    let mut result = mapped(&rows, &cfg["accounts"]);
    // No peer is asked for Claude identity: only the local provider state.
    if let Some((id, address)) = claude_email(
        &cfg["accounts"],
        &claude_account::environment_names(),
        claude_account::absolute_home().as_deref(),
        claude_account::identity,
    ) {
        result[id] = json!(address);
    }
    let count = result.as_object().unwrap().len();
    if count == 0 {
        return Err("No account identities matched configured allowances".into());
    }
    common::atomic_owned_write(
        output,
        &serde_json::to_vec(&result).map_err(|_| "Invalid account identities")?,
    )?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_verified_mapping_discloses_email() {
        let row = identity(
            &json!({"account":{"type":"chatgpt","email":"fixture@example.invalid"}}),
            &json!({"accountId":"fixture"}),
        )
        .unwrap();
        let key = row["account_key"].as_str().unwrap();
        assert_eq!(
            mapped(std::slice::from_ref(&row), &json!({key:"Personal"})),
            json!({"Personal":"fixture@example.invalid"})
        );
        assert_eq!(mapped(&[row], &json!({"unmatched":"Personal"})), json!({}));
    }
    #[test]
    fn invalid_accounts_and_email_are_rejected() {
        for value in ["missing", "a@b", "a@@b.test", "a @b.test", "<a>@b.test"] {
            assert!(!email(value));
        }
        assert!(
            identity(
                &json!({"account":{"type":"apiKey","email":"a@b.test"}}),
                &json!({"accountId":"fixture"})
            )
            .is_none()
        );
        assert!(
            identity(
                &json!({"account":{"type":"chatgpt","email":"a@b.test"}}),
                &json!({})
            )
            .is_none()
        );
    }

    const CLAUDE_KEY: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    fn accounts(claude: bool) -> Value {
        let mut accounts = json!({"a".repeat(64):{"id":"codex","label":"Codex","category":"Work"}});
        if claude {
            accounts[CLAUDE_KEY] =
                json!({"id":"claude","label":"Claude","category":"Personal","provider":"claude"});
        }
        accounts
    }
    /// A private temporary home, removed on drop.
    struct Home(std::path::PathBuf);
    impl Home {
        fn new() -> Self {
            use std::os::unix::fs::PermissionsExt;
            static SEQUENCE: std::sync::atomic::AtomicUsize =
                std::sync::atomic::AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "anton-identity-{}-{}-{}",
                std::process::id(),
                common::now().to_bits(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
    }
    impl Drop for Home {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// D8: the provider-state reader runs only with a Claude mapping, a
    /// home and D4 steps 2 and 3 passing; the email is kept only for a key
    /// a Claude mapping names and only when valid.
    #[test]
    fn claude_email_reads_only_behind_its_gate() {
        let home = Home::new();
        let found = |key: &str, address: Option<&str>| {
            let result = (key.to_owned(), address.map(str::to_owned));
            move |_: &Path| Ok(result)
        };
        let never = |_: &Path| -> std::result::Result<(String, Option<String>), &'static str> {
            panic!("provider state read")
        };
        assert_eq!(
            claude_email(
                &accounts(true),
                &["PATH"],
                Some(&home.0),
                found(CLAUDE_KEY, Some("fixture@example.invalid"))
            ),
            Some(("claude".to_owned(), "fixture@example.invalid".to_owned()))
        );
        // No Claude mapping, no home, `CLAUDE_CONFIG_DIR` or a legacy file:
        // the reader is never called.
        assert_eq!(
            claude_email(&accounts(false), &["PATH"], Some(&home.0), never),
            None
        );
        assert_eq!(claude_email(&accounts(true), &["PATH"], None, never), None);
        assert_eq!(
            claude_email(
                &accounts(true),
                &["PATH", "CLAUDE_CONFIG_DIR"],
                Some(&home.0),
                never
            ),
            None
        );
        // `ANTHROPIC_BASE_URL` does not stop identity refresh (G1).
        assert!(
            claude_email(
                &accounts(true),
                &["ANTHROPIC_BASE_URL"],
                Some(&home.0),
                found(CLAUDE_KEY, Some("fixture@example.invalid"))
            )
            .is_some()
        );
        std::fs::create_dir(home.0.join(".claude")).unwrap();
        std::fs::write(home.0.join(".claude/.config.json"), "{}").unwrap();
        assert_eq!(
            claude_email(&accounts(true), &["PATH"], Some(&home.0), never),
            None
        );
        std::fs::remove_file(home.0.join(".claude/.config.json")).unwrap();
        // An unmapped key, a Codex-mapped key, an invalid or absent email,
        // or a read error store nothing.
        for (key, address) in [
            ("d".repeat(64), Some("fixture@example.invalid")),
            ("a".repeat(64), Some("fixture@example.invalid")),
            (CLAUDE_KEY.to_owned(), Some("not-an-email")),
            (CLAUDE_KEY.to_owned(), None),
        ] {
            assert_eq!(
                claude_email(
                    &accounts(true),
                    &["PATH"],
                    Some(&home.0),
                    found(&key, address)
                ),
                None,
                "{key} {address:?}"
            );
        }
        assert_eq!(
            claude_email(&accounts(true), &["PATH"], Some(&home.0), |_| Err(
                "Provider state names an API key"
            )),
            None
        );
    }

    /// D8: a Codex RPC or peer identity row carrying a Claude mapping's key
    /// stores nothing, beside a Codex row that is stored.
    #[test]
    fn codex_identity_rows_match_codex_mappings_only() {
        let rows = [
            json!({"account_key":CLAUDE_KEY,"email":"peer@example.invalid"}),
            json!({"account_key":"a".repeat(64),"email":"codex@example.invalid"}),
        ];
        assert_eq!(
            mapped(&rows, &accounts(true)),
            json!({"codex":"codex@example.invalid"})
        );
    }
}
