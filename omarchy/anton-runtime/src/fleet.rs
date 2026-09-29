//! Bounded saved-machine discovery and pure effective configuration projection.
use crate::{Result, allowances, common, config};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub const LIMIT: usize = 256 * 1024;
pub fn enabled(config: &Value) -> bool {
    config["fleet_discovery"] == true
}
pub fn profile_id(value: &str) -> bool {
    common::safe_id(value, 40) && value.as_bytes()[0].is_ascii_alphanumeric()
}
pub fn session(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1024
        && !value.starts_with('-')
        && !value.chars().any(char::is_control)
}
pub fn profiles(raw: &Value) -> Result<Vec<Value>> {
    let rows = raw
        .as_array()
        .filter(|v| v.len() <= 64)
        .ok_or("Invalid machine inventory")?;
    let mut ids = BTreeSet::new();
    let mut result = Vec::new();
    for row in rows {
        let id = row["id"]
            .as_str()
            .filter(|s| profile_id(s))
            .ok_or("Invalid machine identity")?;
        if !ids.insert(id) || !row["enabled"].is_boolean() {
            return Err("Ambiguous machine inventory".into());
        }
        let target = row["target"]
            .as_str()
            .filter(|v| v.len() <= 1024 && allowances::valid_target(v))
            .ok_or("Invalid machine target")?;
        let session = match row.get("session") {
            None => "default",
            Some(v) => v
                .as_str()
                .filter(|s| session(s))
                .ok_or("Invalid machine session")?,
        };
        let label = match row.get("label") {
            None => id,
            Some(v) => v
                .as_str()
                .filter(|v| v.len() <= 512 && !v.chars().any(char::is_control))
                .ok_or("Invalid machine label")?,
        };
        result.push(json!({"id":id,"target":target,"session":session,"label":label,"enabled":row["enabled"]}));
    }
    Ok(result)
}
pub fn discover(config: &Value, cancel: Option<&AtomicBool>) -> Result<Vec<Value>> {
    let binary = config["hosts"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|h| {
            h.get("transport")
                .and_then(Value::as_str)
                .unwrap_or("local")
                == "local"
        })
        .and_then(|h| h["herdr"].as_str())
        .unwrap_or("herdr");
    let command = vec![
        crate::collection::herdr_binary(binary),
        "machine".into(),
        "list".into(),
        "--json".into(),
    ];
    let bytes = common::run_bounded(&command, &[], Duration::from_secs(5), LIMIT, cancel)?;
    profiles(&serde_json::from_slice(&bytes).map_err(|_| "Invalid machine inventory JSON")?)
}
/// A None inventory is unknown at startup, never an authoritative empty list.
/// Bound overrides remain dormant until discovery confirms them.
pub fn effective(base: &Value, inventory: Option<&[Value]>) -> Result<Value> {
    if !enabled(base) {
        return Ok(base.clone());
    }
    let profiles: BTreeMap<_, _> = inventory
        .into_iter()
        .flatten()
        .filter(|p| p["enabled"] == true)
        .map(|p| (p["id"].as_str().unwrap(), p))
        .collect();
    let mut hosts = Vec::new();
    let mut bound = BTreeSet::new();
    for host in base["hosts"].as_array().unwrap() {
        if let Some(id) = host["profile_id"].as_str() {
            bound.insert(id);
            if let Some(profile) = profiles.get(id) {
                hosts.push(bind(host.clone(), profile));
            }
        } else {
            hosts.push(host.clone());
        }
    }
    for (id, profile) in &profiles {
        if !bound.contains(id) {
            hosts.push(bind(json!({"id":id}), profile));
        }
    }
    if hosts.len() > 16 {
        return Err("Too many effective fleet hosts".into());
    }
    let mut ids = BTreeSet::new();
    let mut routes = BTreeSet::new();
    for host in &hosts {
        if !ids.insert(host["id"].as_str().unwrap()) {
            return Err("Fleet host identity collision".into());
        }
        if host["transport"] == "ssh"
            && !routes.insert((
                host["target"].as_str().unwrap(),
                host.get("session")
                    .and_then(Value::as_str)
                    .unwrap_or("default"),
            ))
        {
            return Err("Ambiguous fleet route".into());
        }
    }
    let mut result = base.clone();
    result["hosts"] = json!(hosts);
    // Accounts are mappings, not discoveries. Only their source routes change.
    if allowances::configuration(base).is_some() {
        let mut targets = BTreeSet::new();
        for source in base["allowances"]["sources"]
            .as_array()
            .into_iter()
            .flatten()
        {
            if let Some(id) = source["profile_id"].as_str() {
                if let Some(profile) = profiles.get(id) {
                    targets.insert(profile["target"].as_str().unwrap().to_owned());
                }
            } else if let Some(target) = source["target"].as_str() {
                targets.insert(target.to_owned());
            }
        }
        for host in &hosts {
            if host["transport"] == "ssh" {
                targets.insert(host["target"].as_str().unwrap().to_owned());
            }
        }
        if targets.len() > 16 {
            return Err("Too many effective allowance sources".into());
        }
        result["allowances"]["sources"] = json!(
            targets
                .into_iter()
                .map(|target| json!({"target":target}))
                .collect::<Vec<_>>()
        );
    }
    // Effective empty fleets and removed theme hosts are valid at runtime.
    if !hosts.is_empty() {
        let mut validated = result.clone();
        validated.as_object_mut().unwrap().remove("theme_host");
        config::validate(validated)?;
    }
    Ok(result)
}
fn bind(mut host: Value, profile: &Value) -> Value {
    host["profile_id"] = profile["id"].clone();
    host["transport"] = json!("ssh");
    host["target"] = profile["target"].clone();
    host["session"] = profile["session"].clone();
    host["label"] = profile["label"].clone();
    host.as_object_mut().unwrap().remove("socket_path");
    host
}
/// Mutable presentation never restarts collection. Route identity also scopes
/// checkpoint reuse across owner restarts, without changing the peer's wire id.
pub fn route_key(host: &Value) -> String {
    common::sha256(
        serde_json::to_vec(&json!([
            host["profile_id"],
            host.get("transport").cloned().unwrap_or(json!("local")),
            host["target"],
            host.get("session").cloned().unwrap_or(json!("default")),
            host["socket_path"],
            host["herdr"]
        ]))
        .unwrap()
        .as_slice(),
    )
}
pub fn checkpoint_key(host: &Value) -> String {
    let id = host["id"].as_str().unwrap();
    if host["profile_id"].is_string() {
        format!("{id}:{}", route_key(host))
    } else {
        id.to_owned()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn profile(id: &str, target: &str) -> Value {
        json!({"id":id,"target":target,"label":"Same","enabled":true,"session":"default"})
    }
    #[test]
    fn bindings_reconcile_without_resurrection_and_preserve_overrides() {
        let base = json!({"fleet_discovery":true,"hosts":[{"id":"local"},{"id":"legacy","profile_id":"saved","target":"old","transport":"ssh","personal_roots":["/private"]}],"allowances":{"accounts":{"a".repeat(64):"Personal"},"sources":[{"profile_id":"saved","target":"old"}]}});
        assert_eq!(
            effective(&base, None).unwrap()["hosts"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let p = profile("saved", "new");
        let active = effective(&base, Some(&[p.clone(), profile("added", "other")])).unwrap();
        assert_eq!(active["hosts"][1]["id"], "legacy");
        assert_eq!(active["hosts"][1]["target"], "new");
        assert_eq!(active["hosts"][1]["personal_roots"], json!(["/private"]));
        assert_eq!(active["hosts"][2]["id"], "added");
        assert_eq!(
            active["allowances"]["sources"],
            json!([{"target":"new"},{"target":"other"}])
        );
        assert_eq!(
            effective(&base, Some(&[])).unwrap()["hosts"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let mut disabled = p;
        disabled["enabled"] = json!(false);
        assert_eq!(
            effective(&base, Some(&[disabled])).unwrap()["allowances"]["sources"],
            json!([])
        );
        let mut renamed = active["hosts"][1].clone();
        renamed["label"] = json!("Renamed");
        assert_eq!(route_key(&renamed), route_key(&active["hosts"][1]));
        renamed["session"] = json!("other");
        assert_ne!(
            checkpoint_key(&renamed),
            checkpoint_key(&active["hosts"][1])
        );
    }
    #[test]
    fn invalid_inventory_and_collisions_fail_closed() {
        let p = profile("saved", "host");
        assert!(profiles(&json!([p, p])).is_err());
        for field in ["enabled", "target", "session", "id"] {
            let mut row = profile("saved", "host");
            row[field] = Value::Null;
            assert!(profiles(&json!([row])).is_err());
        }
        assert!(profiles(&json!(vec![profile("saved", "host"); 65])).is_err());
        assert!(
            effective(
                &json!({"fleet_discovery":true,"hosts":[{"id":"saved"}]}),
                Some(&[profile("saved", "host")])
            )
            .is_err()
        );
        assert!(effective(&json!({"fleet_discovery":true,"hosts":[{"id":"static","transport":"ssh","target":"host"}]}),Some(&[profile("saved","host")])).is_err());
    }
}
