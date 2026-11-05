//! UDP relay packet format (see `docs/02 - Architecture/Components/Relay Server.md`).
//!
//! ```text
//! BIND: "BRLY" | 0x01 | session_id[16] | role u8 | expiry u64 BE | token[32]
//! DATA: "BRLY" | 0x02 | session_id[16] | role u8 | payload (opaque QUIC packet)
//! ```
//! The relay never sees anything but opaque payloads (ADR-005).

pub const MAGIC: &[u8; 4] = b"BRLY";
pub const TYPE_BIND: u8 = 1;
pub const TYPE_DATA: u8 = 2;
pub const HEADER_LEN: usize = 4 + 1 + 16 + 1;
pub const BIND_LEN: usize = HEADER_LEN + 8 + 32;
/// Upper bound for a relayed datagram (header + QUIC packet).
pub const MAX_PACKET: usize = 1500;

pub const ROLE_HOST: u8 = 1;
pub const ROLE_VIEWER: u8 = 2;

pub type SessionId = [u8; 16];

#[derive(Debug, PartialEq, Eq)]
pub enum Packet<'a> {
    Bind { session_id: SessionId, role: u8, expiry: u64, token: [u8; 32] },
    Data { session_id: SessionId, role: u8, payload: &'a [u8] },
}

fn header(kind: u8, session_id: &SessionId, role: u8, out: &mut Vec<u8>) {
    out.extend_from_slice(MAGIC);
    out.push(kind);
    out.extend_from_slice(session_id);
    out.push(role);
}

pub fn encode_bind(session_id: &SessionId, role: u8, expiry: u64, token: &[u8; 32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(BIND_LEN);
    header(TYPE_BIND, session_id, role, &mut out);
    out.extend_from_slice(&expiry.to_be_bytes());
    out.extend_from_slice(token);
    out
}

pub fn encode_data(session_id: &SessionId, role: u8, payload: &[u8], out: &mut Vec<u8>) {
    out.clear();
    header(TYPE_DATA, session_id, role, out);
    out.extend_from_slice(payload);
}

pub fn parse(buf: &[u8]) -> Option<Packet<'_>> {
    if buf.len() < HEADER_LEN || buf.len() > MAX_PACKET || &buf[..4] != MAGIC {
        return None;
    }
    let kind = buf[4];
    let session_id: SessionId = buf[5..21].try_into().ok()?;
    let role = buf[21];
    if role != ROLE_HOST && role != ROLE_VIEWER {
        return None;
    }
    match kind {
        TYPE_BIND if buf.len() == BIND_LEN => {
            let expiry = u64::from_be_bytes(buf[22..30].try_into().ok()?);
            let token: [u8; 32] = buf[30..62].try_into().ok()?;
            Some(Packet::Bind { session_id, role, expiry, token })
        }
        TYPE_DATA => Some(Packet::Data { session_id, role, payload: &buf[HEADER_LEN..] }),
        _ => None,
    }
}

pub fn other_role(role: u8) -> u8 {
    if role == ROLE_HOST {
        ROLE_VIEWER
    } else {
        ROLE_HOST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_roundtrip() {
        let sid = [7u8; 16];
        let tok = [9u8; 32];
        let b = encode_bind(&sid, ROLE_VIEWER, 1234, &tok);
        assert_eq!(parse(&b), Some(Packet::Bind { session_id: sid, role: ROLE_VIEWER, expiry: 1234, token: tok }));
    }

    #[test]
    fn data_roundtrip_and_garbage() {
        let sid = [1u8; 16];
        let mut out = Vec::new();
        encode_data(&sid, ROLE_HOST, b"quic", &mut out);
        assert_eq!(parse(&out), Some(Packet::Data { session_id: sid, role: ROLE_HOST, payload: b"quic" }));
        assert_eq!(parse(b"BRLY"), None);
        let mut bad_role = out.clone();
        bad_role[21] = 9;
        assert_eq!(parse(&bad_role), None);
        assert_eq!(parse(&vec![0u8; 3000]), None);
    }
}
