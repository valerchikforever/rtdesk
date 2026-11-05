//! Viewer-side video decoding: a malicious host controls every byte of the frame update.
#![no_main]

use bridge_codec::FrameDecoder;
use bridge_proto::msg::FrameUpdate;
use libfuzzer_sys::fuzz_target;
use prost::Message;

fuzz_target!(|data: &[u8]| {
    let Ok(mut update) = FrameUpdate::decode(data) else { return };
    // Keep allocations within fuzzer limits; real limits are enforced by the decoder too.
    update.width = update.width.min(1024);
    update.height = update.height.min(1024);
    let mut dec = FrameDecoder::new();
    if dec.apply(&update).is_ok() {
        assert_eq!(dec.pixels().len(), dec.width() as usize * dec.height() as usize);
    }
});
