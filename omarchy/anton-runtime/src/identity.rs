//! Explicit local-only account identity. Never included in collector snapshots.
use crate::{Result, allowances, common};
use serde_json::{Value, json};
use std::collections::BTreeSet;
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
pub fn mapped(rows: &[Value], accounts: &Value) -> Value {
    let mut result = serde_json::Map::new();
    for row in rows {
        let Some(key) = row["account_key"].as_str() else {
            continue;
        };
        let Some(value) = accounts.get(key) else {
            continue;
        };
        let Ok(mapping) = allowances::mapping(value) else {
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
    let result = mapped(&rows, &cfg["accounts"]);
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
}
