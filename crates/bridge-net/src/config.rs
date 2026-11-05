//! Client configuration and TOFU key stores.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use bridge_crypto::keys::{self, PublicKey};
use serde::{Deserialize, Serialize};

pub const DEFAULT_PORT_RENDEZVOUS: u16 = 21116;
pub const DEFAULT_PORT_RELAY: u16 = 21117;

/// `%APPDATA%\Bridge` on Windows, `~/.config/Bridge` on Linux, … (override with `--config-dir`).
pub fn default_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("Bridge")
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Rendezvous server `host:port`.
    pub server: Option<String>,
    /// Pinned rendezvous key (base64url). If absent, pinned on first use.
    pub server_key: Option<String>,
    /// Device ID assigned by the rendezvous server (host only).
    pub device_id: Option<String>,
}

impl ClientConfig {
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join("config.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        write_atomic(&dir.join("config.json"), &serde_json::to_vec_pretty(self)?)
    }
}

/// Name → public key map persisted as JSON (known_hosts.json / known_servers.json).
pub struct KnownKeys {
    path: PathBuf,
    map: BTreeMap<String, String>,
}

impl KnownKeys {
    pub fn load(path: PathBuf) -> Result<Self> {
        let map = if path.exists() {
            serde_json::from_slice(&std::fs::read(&path)?).with_context(|| format!("parsing {}", path.display()))?
        } else {
            BTreeMap::new()
        };
        Ok(Self { path, map })
    }

    pub fn get(&self, name: &str) -> Option<PublicKey> {
        self.map.get(name).and_then(|k| keys::decode_public(k))
    }

    pub fn insert(&mut self, name: &str, key: &PublicKey) -> Result<()> {
        self.map.insert(name.to_string(), keys::encode_public(key));
        write_atomic(&self.path, &serde_json::to_vec_pretty(&self.map)?)
    }
}

pub fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Append the default port if `addr` has none.
pub fn with_default_port(addr: &str, port: u16) -> String {
    if addr.rsplit_once(':').is_some_and(|(_, p)| p.parse::<u16>().is_ok()) {
        addr.to_string()
    } else {
        format!("{addr}:{port}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_port() {
        assert_eq!(with_default_port("example.org", 1), "example.org:1");
        assert_eq!(with_default_port("1.2.3.4:5", 1), "1.2.3.4:5");
    }

    #[test]
    fn known_keys_roundtrip() {
        let path = std::env::temp_dir().join(format!("bridge-known-{}.json", std::process::id()));
        let mut k = KnownKeys::load(path.clone()).unwrap();
        k.insert("123", &[9; 32]).unwrap();
        assert_eq!(KnownKeys::load(path.clone()).unwrap().get("123"), Some([9; 32]));
        let _ = std::fs::remove_file(path);
    }
}
