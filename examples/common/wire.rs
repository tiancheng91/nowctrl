//! Shared lab-only wire helpers. These never authenticate a sender.
use nowctrl::{
    preset::RcAction,
    protocol::{Control, ControlAck, Error, Frame, MessageType},
};

pub const SWITCH_NODE_ID: u32 = 7;
pub const RC_NODE_ID: u32 = 8;

pub fn switch_packet(command: Control, request_id: u32, output: &mut [u8]) -> Result<usize, Error> {
    let mut payload = [0; 5];
    let len = command.encode(&mut payload)?;
    Frame {
        kind: MessageType::Control,
        node_id: SWITCH_NODE_ID,
        seq: request_id,
        request_id,
        payload: &payload[..len],
    }
    .encode(output)
}

pub fn switch_command(input: &[u8]) -> Result<(u32, Control), Error> {
    let frame = Frame::decode(input)?;
    if frame.kind != MessageType::Control || frame.node_id != SWITCH_NODE_ID {
        return Err(Error::InvalidPayload);
    }
    Ok((frame.request_id, Control::decode(frame.payload)?))
}

pub fn ack_packet(request_id: u32, result: ControlAck, output: &mut [u8]) -> Result<usize, Error> {
    Frame {
        kind: MessageType::ControlAck,
        node_id: SWITCH_NODE_ID,
        seq: request_id,
        request_id,
        payload: &result.encode(),
    }
    .encode(output)
}

pub fn switch_ack(input: &[u8]) -> Result<(u32, ControlAck), Error> {
    let frame = Frame::decode(input)?;
    if frame.kind != MessageType::ControlAck || frame.node_id != SWITCH_NODE_ID {
        return Err(Error::InvalidPayload);
    }
    Ok((frame.request_id, ControlAck::decode(frame.payload)?))
}

/// LAB prefix deliberately separates this example payload from a future production RC schema.
pub fn rc_packet(action: RcAction, seq: u32, output: &mut [u8]) -> Result<usize, Error> {
    if seq == 0 {
        return Err(Error::InvalidIdentity);
    }
    let code = match action {
        RcAction::Stop => 0,
        RcAction::Forward => 1,
        RcAction::BoostForward => 2,
        RcAction::Reverse => 3,
        RcAction::Left => 4,
        RcAction::Right => 5,
    };
    Frame {
        kind: MessageType::RcUpdate,
        node_id: RC_NODE_ID,
        seq,
        request_id: 0,
        payload: &[b'L', b'A', b'B', code],
    }
    .encode(output)
}

pub fn rc_update(input: &[u8]) -> Result<(u32, RcAction), Error> {
    let frame = Frame::decode(input)?;
    if frame.kind != MessageType::RcUpdate
        || frame.node_id != RC_NODE_ID
        || frame.request_id != 0
        || frame.seq == 0
    {
        return Err(Error::InvalidIdentity);
    }
    let action = match frame.payload {
        [b'L', b'A', b'B', 0] => RcAction::Stop,
        [b'L', b'A', b'B', 1] => RcAction::Forward,
        [b'L', b'A', b'B', 2] => RcAction::BoostForward,
        [b'L', b'A', b'B', 3] => RcAction::Reverse,
        [b'L', b'A', b'B', 4] => RcAction::Left,
        [b'L', b'A', b'B', 5] => RcAction::Right,
        _ => return Err(Error::InvalidPayload),
    };
    Ok((frame.seq, action))
}
