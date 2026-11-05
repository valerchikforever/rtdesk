//! Device ID ↔ public key bindings, persisted as JSON (SQLite/PostgreSQL in later phases).

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use bridge_crypto::keys::{self, PublicKey};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Record {
    id: String,
    key: String,
}

pub struct Registry {
    path: PathBuf,
    by_id: HashMap<String, PublicKey>,
    by_key: HashMap<PublicKey, String>,
}

impl Registry {
    pub fn load(path: PathBuf) -> Result<Self> {
        let mut r = Self { path, by_id: HashMap::new(), by_key: HashMap::new() };
        if r.path.exists() {
            let records: Vec<Record> = serde_json::from_slice(&std::fs::read(&r.path)?)
                .with_context(|| format!("parsing {}", r.path.display()))?;
            for rec in records {
                if let Some(k) = keys::decode_public(&rec.key) {
                    r.by_key.insert(k, rec.id.clone());
                    r.by_id.insert(rec.id, k);
                }
            }
        }
        Ok(r)
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn key_of(&self, id: &str) -> Option<PublicKey> {
        self.by_id.get(id).copied()
    }

    /// Return the ID bound to `key`, binding a new one if needed.
    /// An ID already bound to a different key is never re-assigned (SEC: T-02).
    pub fn bind(&mut self, key: &PublicKey, wanted: Option<&str>) -> std::result::Result<String, &'static str> {
        if let Some(id) = self.by_key.get(key) {
            if wanted.is_some_and(|w| w != id) {
                return Err("this key is registered under a different ID");
            }
            return Ok(id.clone());
        }
        let id = match wanted {
            Some(w) if self.by_id.contains_key(w) => return Err("device ID belongs to another key"),
            Some(w) => w.to_string(),
            None => loop {
                let candidate = rand::rngs::OsRng.gen_range(100_000_000u32..1_000_000_000).to_string();
                if !self.by_id.contains_key(&candidate) {
                    break candidate;
                }
            },
        };
        self.by_id.insert(id.clone(), *key);
        self.by_key.insert(*key, id.clone());
        self.save().map_err(|_| "failed to persist registry")?;
        Ok(id)
    }

    fn save(&self) -> Result<()> {
        let mut records: Vec<Record> =
            self.by_id.iter().map(|(id, k)| Record { id: id.clone(), key: keys::encode_public(k) }).collect();
        records.sort_by(|a, b| a.id.cmp(&b.id));
        bridge_net::config::write_atomic(&self.path, &serde_json::to_vec_pretty(&records)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_rules() {
        let path = std::env::temp_dir().join(format!("bridge-reg-{}.json", std::process::id()));
        let mut r = Registry::load(path.clone()).unwrap();
        let id = r.bind(&[1; 32], None).unwrap();
        assert_eq!(r.bind(&[1; 32], None).unwrap(), id, "stable");
        assert_eq!(r.bind(&[1; 32], Some(&id)).unwrap(), id);
        assert!(r.bind(&[2; 32], Some(&id)).is_err(), "cannot hijack an ID");
        assert!(r.bind(&[1; 32], Some("999999999")).is_err(), "cannot switch ID");
        let r2 = Registry::load(path.clone()).unwrap();
        assert_eq!(r2.key_of(&id), Some([1; 32]));
        let _ = std::fs::remove_file(path);
    }
}
