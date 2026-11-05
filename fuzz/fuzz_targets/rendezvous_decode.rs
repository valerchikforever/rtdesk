//! Rendezvous messages: decoded by the server (requests) and by clients (responses).
#![no_main]

use bridge_proto::msg::{RvRequest, RvResponse};
use libfuzzer_sys::fuzz_target;
use prost::Message;

fuzz_target!(|data: &[u8]| {
    if let Ok(req) = RvRequest::decode(data) {
        let again = RvRequest::decode(req.encode_to_vec().as_slice()).expect("re-encoded request decodes");
        assert_eq!(req, again);
    }
    if let Ok(resp) = RvResponse::decode(data) {
        let again = RvResponse::decode(resp.encode_to_vec().as_slice()).expect("re-encoded response decodes");
        assert_eq!(resp, again);
    }
});
