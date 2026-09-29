use crate::Result;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Component, Path};
/// Validates private plugin configuration.
///
/// `theme_host` and `hosts[].theme_path` are retired: the runtime no longer
/// collects the Omarchy theme. They are still validated exactly as before
/// (`theme_host` must name a configured host, `theme_path` must be a non-empty
/// string) so installed configurations keep loading, and are otherwise ignored.
pub fn validate(config: Value) -> Result<Value> {
    if !config.is_object() {
        return Err("Invalid configuration".into());
    }
    if ["publish", "music"]
        .iter()
        .any(|key| config.get(*key).is_some())
    {
        return Err("Legacy forwarding configuration requires migration".into());
    }
    if config
        .get("fleet_discovery")
        .is_some_and(|v| !v.is_boolean())
    {
        return Err("Invalid fleet discovery switch".into());
    }
    let mut profiles = BTreeSet::new();
    let hosts = config["hosts"]
        .as_array()
        .filter(|hosts| !hosts.is_empty() && hosts.len() <= 16)
        .ok_or("Configure one to sixteen hosts")?;
    let mut ids = BTreeSet::new();
    for host in hosts {
        let id = host["id"]
            .as_str()
            .filter(|s| crate::fleet::profile_id(s))
            .ok_or("Invalid host identifier")?;
        if !ids.insert(id) {
            return Err("Duplicate host identifier".into());
        }
        if let Some(profile) = host.get("profile_id") {
            let id = profile
                .as_str()
                .filter(|s| crate::fleet::profile_id(s))
                .ok_or("Invalid profile binding")?;
            if !profiles.insert(id) {
                return Err("Duplicate profile binding".into());
            }
        }
        let transport = match host.get("transport") {
            None => "local",
            Some(value) => value.as_str().ok_or("Invalid host transport")?,
        };
        if !["local", "ssh"].contains(&transport) {
            return Err("Native transport must be local or ssh".into());
        }
        if transport == "ssh" {
            let target = host["target"].as_str().ok_or("Missing SSH target")?;
            if target.is_empty()
                || target.starts_with('-')
                || !target
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_.@:-".contains(&c))
            {
                return Err("Invalid SSH target".into());
            }
        }
        for key in ["work_roots", "personal_roots"] {
            if let Some(roots) = host.get(key) {
                if roots.as_array().is_none_or(|rows| {
                    rows.iter().any(|root| {
                        root.as_str().is_none_or(|root| {
                            !Path::new(root).is_absolute()
                                || Path::new(root)
                                    .components()
                                    .any(|v| v == Component::ParentDir)
                        })
                    })
                }) {
                    return Err("Invalid project roots".into());
                }
            }
        }
        if host.get("socket_path").is_some() && host.get("session").is_some() {
            return Err("Select socket or session".into());
        }
        for key in ["herdr", "session", "socket_path", "theme_path"] {
            if host
                .get(key)
                .is_some_and(|value| value.as_str().is_none_or(str::is_empty))
            {
                return Err(format!("Invalid {key}"));
            }
        }
    }
    let interval = config
        .get("interval")
        .and_then(Value::as_f64)
        .unwrap_or(5.0);
    if !(2.0..=60.0).contains(&interval) || config.get("interval").is_some_and(|v| !v.is_number()) {
        return Err("Interval must be two to sixty seconds".into());
    }
    if config
        .get("theme_host")
        .is_some_and(|v| v.as_str().is_none_or(|s| !ids.contains(s)))
    {
        return Err("Unknown theme host".into());
    }
    crate::allowances::validate_config(config.get("allowances"))?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn retired_theme_keys_still_load_and_still_validate() {
        let valid = json!({"theme_host":"local","hosts":[{"id":"local","theme_path":"~/theme"},{"id":"remote","transport":"ssh","target":"fixture","theme_path":"/srv/theme"}]});
        assert_eq!(validate(valid.clone()).unwrap(), valid);
        for theme_host in [json!("missing"), json!(12), Value::Null, json!("")] {
            assert!(validate(json!({"theme_host":theme_host,"hosts":[{"id":"local"}]})).is_err());
        }
    }
    #[test]
    fn host_transport_and_source_selectors_fail_closed() {
        assert!(validate(json!({"hosts":[{"id":"local"}]})).is_ok());
        for transport in [
            Value::Null,
            json!(12),
            json!(true),
            json!({}),
            json!("file"),
        ] {
            assert!(validate(json!({"hosts":[{"id":"local","transport":transport}]})).is_err());
        }
        for key in ["herdr", "session", "socket_path", "theme_path"] {
            for value in [Value::Null, json!(12), json!("")] {
                let mut host = json!({"id":"local"});
                host[key] = value;
                assert!(validate(json!({"hosts":[host]})).is_err());
            }
        }
        assert!(
            validate(json!({"hosts":[{"id":"local","session":"a","socket_path":"/tmp/a"}]}))
                .is_err()
        );
        assert!(
            validate(json!({"hosts":[{"id":"remote","transport":"ssh","target":"-bad"}]})).is_err()
        );
    }
}
