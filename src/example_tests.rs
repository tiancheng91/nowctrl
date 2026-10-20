//! Tests compile the same wire helpers used by the device examples.
#[path = "../examples/common/wire.rs"]
mod wire;
use crate::{
    control::VirtualNode,
    preset::{Axes, RcAction, RcState},
    protocol::{Control, ControlAck, Error, Frame, MAX_FRAME_LEN, MessageType},
};

#[test]
fn switch_client_server_roundtrip_retries_and_ack_identity() {
    let mut output = [0; MAX_FRAME_LEN];
    let mut node = VirtualNode::new(wire::SWITCH_NODE_ID).unwrap();
    for now in [0, 200, 600] {
        let len = wire::switch_packet(Control::Toggle, 1, &mut output).unwrap();
        let (request_id, command) = wire::switch_command(&output[..len]).unwrap();
        let ack = node
            .apply(wire::SWITCH_NODE_ID, request_id, command, now)
            .unwrap();
        assert!(node.output());
        let len = wire::ack_packet(request_id, ack, &mut output).unwrap();
        assert_eq!(
            wire::switch_ack(&output[..len]),
            Ok((1, ControlAck::Applied))
        );
    }
    let len = wire::switch_packet(Control::Off, 2, &mut output).unwrap();
    assert_eq!(wire::switch_ack(&output[..len]), Err(Error::InvalidPayload));
}

#[test]
fn rc_client_server_all_actions_and_watchdog() {
    let mut output = [0; MAX_FRAME_LEN];
    let mut state = RcState::default();
    for (index, action) in [
        RcAction::Stop,
        RcAction::Forward,
        RcAction::BoostForward,
        RcAction::Reverse,
        RcAction::Left,
        RcAction::Right,
    ]
    .into_iter()
    .enumerate()
    {
        let seq = index as u32 + 1;
        let len = wire::rc_packet(action, seq, &mut output).unwrap();
        let (received_seq, received_action) = wire::rc_update(&output[..len]).unwrap();
        assert_eq!((received_seq, received_action), (seq, action));
        state
            .update(received_seq, received_action, u64::from(seq) * 1000)
            .unwrap();
        assert_eq!(state.advance(u64::from(seq) * 1000), Ok(action.axes()));
        assert_eq!(
            state.advance(u64::from(seq) * 1000 + 300),
            Ok(Axes::default())
        );
    }
}

#[test]
fn rc_example_schema_rejects_other_payloads_destinations_and_zero_sequence() {
    let mut output = [0; MAX_FRAME_LEN];
    assert_eq!(
        wire::rc_packet(RcAction::Forward, 0, &mut output),
        Err(Error::InvalidIdentity)
    );
    for (node_id, seq, request_id, payload) in [
        (wire::RC_NODE_ID, 1, 0, &[1][..]),
        (wire::RC_NODE_ID, 1, 0, &b"LAB\x06"[..]),
        (wire::RC_NODE_ID, 1, 0, &b"LAB\x01\x00"[..]),
        (99, 1, 0, &b"LAB\x01"[..]),
        (wire::RC_NODE_ID, 0, 0, &b"LAB\x01"[..]),
        (wire::RC_NODE_ID, 1, 1, &b"LAB\x01"[..]),
    ] {
        let len = Frame {
            kind: MessageType::RcUpdate,
            node_id,
            seq,
            request_id,
            payload,
        }
        .encode(&mut output)
        .unwrap();
        assert!(wire::rc_update(&output[..len]).is_err());
    }
    let len = wire::rc_packet(RcAction::Forward, 1, &mut output).unwrap();
    for end in 0..len {
        assert!(wire::rc_update(&output[..end]).is_err());
    }
    let len = wire::switch_packet(Control::Toggle, 1, &mut output).unwrap();
    assert!(wire::rc_update(&output[..len]).is_err());
}
