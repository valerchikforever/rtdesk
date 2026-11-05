//! Protected storage for secrets at rest (SR-10).
//! Windows: DPAPI (bound to the current user account). Other OSes: file with 0600 permissions
//! (OS keychain integration is planned for the Beta phase).

use std::path::Path;

use zeroize::Zeroizing;

use crate::keys::DeviceKey;
use crate::{CryptoError, Result};

/// Load the device key from `path`, creating and protecting a new one if absent.
pub fn load_or_create_device_key(path: &Path) -> Result<DeviceKey> {
    if path.exists() {
        let blob = std::fs::read(path)?;
        let der = unprotect(&blob)?;
        return DeviceKey::from_pkcs8(&der);
    }
    let key = DeviceKey::generate()?;
    let blob = protect(key.pkcs8())?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    write_private(path, &blob)?;
    Ok(key)
}

/// Write a file readable only by the owner.
pub fn write_private(path: &Path, data: &[u8]) -> Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
        f.write_all(data)?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        // %APPDATA% is per-user ACL'd; content is additionally DPAPI-protected.
        std::fs::write(path, data)?;
        Ok(())
    }
}

#[cfg(windows)]
pub fn protect(data: &[u8]) -> Result<Vec<u8>> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB};

    let len = u32::try_from(data.len()).map_err(|_| CryptoError::Storage("secret too large".into()))?;
    let input = CRYPT_INTEGER_BLOB { cbData: len, pbData: data.as_ptr() as *mut u8 };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // SAFETY: `input` points to `data`, which outlives the call; DPAPI only reads it.
    // `output` is allocated by DPAPI with LocalAlloc and freed below after copying.
    unsafe {
        CryptProtectData(&input, PCWSTR::null(), None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)
            .map_err(|e| CryptoError::Storage(format!("CryptProtectData: {e}")))?;
        let out = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        let _ = LocalFree(HLOCAL(output.pbData as *mut core::ffi::c_void));
        Ok(out)
    }
}

#[cfg(windows)]
pub fn unprotect(data: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB};

    let len = u32::try_from(data.len()).map_err(|_| CryptoError::Storage("blob too large".into()))?;
    let input = CRYPT_INTEGER_BLOB { cbData: len, pbData: data.as_ptr() as *mut u8 };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // SAFETY: as in `protect`; the plaintext buffer is zeroed before being freed.
    unsafe {
        CryptUnprotectData(&input, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)
            .map_err(|e| CryptoError::Storage(format!("CryptUnprotectData: {e}")))?;
        let plain = std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize);
        let out = Zeroizing::new(plain.to_vec());
        zeroize::Zeroize::zeroize(plain);
        let _ = LocalFree(HLOCAL(output.pbData as *mut core::ffi::c_void));
        Ok(out)
    }
}

#[cfg(not(windows))]
pub fn protect(data: &[u8]) -> Result<Vec<u8>> {
    Ok(data.to_vec())
}

#[cfg(not(windows))]
pub fn unprotect(data: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    Ok(Zeroizing::new(data.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protect_roundtrip() {
        let blob = protect(b"secret").unwrap();
        assert_eq!(unprotect(&blob).unwrap().as_slice(), b"secret");
    }

    #[test]
    fn key_persists() {
        let dir = std::env::temp_dir().join(format!("bridge-ks-{}", std::process::id()));
        let path = dir.join("device.key");
        let k1 = load_or_create_device_key(&path).unwrap();
        let k2 = load_or_create_device_key(&path).unwrap();
        assert_eq!(k1.public(), k2.public());
        let _ = std::fs::remove_dir_all(dir);
    }
}
