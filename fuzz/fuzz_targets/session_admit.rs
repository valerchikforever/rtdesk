//! Host session state machine fed with arbitrary control messages.
//! Invariants: input is only admitted in the Active phase with CONTROL permission, clipboard only
//! with CLIPBOARD permission, and nothing is admitted after the session closed.
#![no_main]

use bridge_proto::msg::control::Kind;
use bridge_proto::msg::Control;
use bridge_session::{HostSession, Permissions, Phase};
use libfuzzer_sys::fuzz_target;
use prost::Message;

fuzz_target!(|data: &[u8]| {
    let Some((&perm_byte, mut rest)) = data.split_first() else { return };
    let granted = Permissions(u32::from(perm_byte));
    let mut s = HostSession::new();

    // Input format: repeated [len: u8][protobuf Control]
    while let Some((&len, tail)) = rest.split_first() {
        let len = usize::from(len).min(tail.len());
        let (msg, next) = tail.split_at(len);
        rest = next;
        let Ok(ctrl) = Control::decode(msg) else { continue };
        let Some(kind) = ctrl.kind else { continue };

        let before = s.phase();
        let perms = s.permissions();
        let ok = s.admit(&kind).is_ok();

        if ok {
            let is_input =
                matches!(kind, Kind::MouseMove(_) | Kind::MouseButton(_) | Kind::MouseWheel(_) | Kind::Key(_));
            if is_input {
                assert_eq!(before, Phase::Active);
                assert!(perms.contains(Permissions::CONTROL));
            }
            if matches!(kind, Kind::Clipboard(_)) {
                assert_eq!(before, Phase::Active);
                assert!(perms.contains(Permissions::CLIPBOARD));
            }
            assert_ne!(before, Phase::Closed);
        } else {
            assert_eq!(s.phase(), Phase::Closed);
        }

        if s.phase() == Phase::AwaitConsent {
            s.grant(granted).expect("grant is valid after confirmation");
            assert!(Permissions::ALL.contains(s.permissions()));
        }
    }
});
