//! Relay packet parser — reachable by anyone on the internet who can send UDP to the relay.
#![no_main]

use bridge_proto::relay::{self, Packet};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    match relay::parse(data) {
        Some(Packet::Data { payload, .. }) => {
            assert_eq!(payload.len() + relay::HEADER_LEN, data.len());
        }
        Some(Packet::Bind { session_id, role, expiry, token }) => {
            // Re-encoding a parsed bind must reproduce the input exactly.
            assert_eq!(relay::encode_bind(&session_id, role, expiry, &token), data);
        }
        None => {}
    }
});
