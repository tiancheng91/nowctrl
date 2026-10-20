//! Version 1 wire envelope and P1 switch payloads. Decoding is not authentication.

pub const VERSION: u8 = 1;
pub const HEADER_LEN: usize = 18;
pub const MAX_FRAME_LEN: usize = 200;
pub const MAX_PAYLOAD_LEN: usize = MAX_FRAME_LEN - HEADER_LEN;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MessageType {
    Discover = 1,
    PairReq = 2,
    PairChallenge = 3,
    PairConfirm = 4,
    PairAccept = 5,
    Hello = 6,
    Heartbeat = 7,
    Control = 8,
    ControlAck = 9,
    ConfigSet = 10,
    ConfigAck = 11,
    StatusReq = 12,
    StatusResp = 13,
    RcUpdate = 14,
}

impl TryFrom<u8> for MessageType {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Error> {
        match value {
            1 => Ok(Self::Discover),
            2 => Ok(Self::PairReq),
            3 => Ok(Self::PairChallenge),
            4 => Ok(Self::PairConfirm),
            5 => Ok(Self::PairAccept),
            6 => Ok(Self::Hello),
            7 => Ok(Self::Heartbeat),
            8 => Ok(Self::Control),
            9 => Ok(Self::ControlAck),
            10 => Ok(Self::ConfigSet),
            11 => Ok(Self::ConfigAck),
            12 => Ok(Self::StatusReq),
            13 => Ok(Self::StatusResp),
            14 => Ok(Self::RcUpdate),
            _ => Err(Error::UnknownType),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    InvalidLength,
    UnsupportedVersion,
    UnknownType,
    UnsupportedFlags,
    BufferTooSmall,
    InvalidIdentity,
    InvalidPayload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Frame<'a> {
    pub kind: MessageType,
    pub node_id: u32,
    pub seq: u32,
    pub request_id: u32,
    pub payload: &'a [u8],
}

impl<'a> Frame<'a> {
    fn validate(&self) -> Result<(), Error> {
        if self.payload.len() > MAX_PAYLOAD_LEN {
            return Err(Error::InvalidLength);
        }
        match self.kind {
            MessageType::Control | MessageType::ControlAck => {
                if self.node_id == 0 || self.request_id == 0 {
                    return Err(Error::InvalidIdentity);
                }
                if self.kind == MessageType::Control {
                    Control::decode(self.payload)?;
                } else {
                    ControlAck::decode(self.payload)?;
                }
            }
            // Other message payload schemas are defined by their later phases.
            _ => {}
        }
        Ok(())
    }

    /// Validates before writing, leaving the output untouched on error.
    pub fn encode(&self, output: &mut [u8]) -> Result<usize, Error> {
        self.validate()?;
        let len = HEADER_LEN + self.payload.len();
        if output.len() < len {
            return Err(Error::BufferTooSmall);
        }
        output[0] = VERSION;
        output[1] = self.kind as u8;
        output[2..4].copy_from_slice(&0u16.to_le_bytes());
        output[4..8].copy_from_slice(&self.node_id.to_le_bytes());
        output[8..12].copy_from_slice(&self.seq.to_le_bytes());
        output[12..16].copy_from_slice(&self.request_id.to_le_bytes());
        output[16..18].copy_from_slice(&(self.payload.len() as u16).to_le_bytes());
        output[HEADER_LEN..len].copy_from_slice(self.payload);
        Ok(len)
    }

    /// Accepts exactly one frame, rejecting truncation and trailing bytes.
    pub fn decode(input: &'a [u8]) -> Result<Self, Error> {
        if !(HEADER_LEN..=MAX_FRAME_LEN).contains(&input.len()) {
            return Err(Error::InvalidLength);
        }
        if input[0] != VERSION {
            return Err(Error::UnsupportedVersion);
        }
        let kind = MessageType::try_from(input[1])?;
        if input[2..4] != [0, 0] {
            return Err(Error::UnsupportedFlags);
        }
        let payload_len = u16::from_le_bytes([input[16], input[17]]) as usize;
        if input.len() != HEADER_LEN + payload_len {
            return Err(Error::InvalidLength);
        }
        let read_u32 = |offset| {
            u32::from_le_bytes([
                input[offset],
                input[offset + 1],
                input[offset + 2],
                input[offset + 3],
            ])
        };
        let frame = Self {
            kind,
            node_id: read_u32(4),
            seq: read_u32(8),
            request_id: read_u32(12),
            payload: &input[HEADER_LEN..],
        };
        frame.validate()?;
        Ok(frame)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Control {
    Off,
    On,
    Toggle,
    Pulse { duration_ms: u32 },
}

impl Control {
    pub fn decode(payload: &[u8]) -> Result<Self, Error> {
        match payload {
            [1] => Ok(Self::Off),
            [2] => Ok(Self::On),
            [3] => Ok(Self::Toggle),
            [4, a, b, c, d] => {
                let duration_ms = u32::from_le_bytes([*a, *b, *c, *d]);
                if duration_ms == 0 {
                    return Err(Error::InvalidPayload);
                }
                Ok(Self::Pulse { duration_ms })
            }
            _ => Err(Error::InvalidPayload),
        }
    }

    pub fn encode(self, output: &mut [u8]) -> Result<usize, Error> {
        if self == (Self::Pulse { duration_ms: 0 }) {
            return Err(Error::InvalidPayload);
        }
        let len = if matches!(self, Self::Pulse { .. }) {
            5
        } else {
            1
        };
        if output.len() < len {
            return Err(Error::BufferTooSmall);
        }
        output[0] = match self {
            Self::Off => 1,
            Self::On => 2,
            Self::Toggle => 3,
            Self::Pulse { duration_ms } => {
                output[1..5].copy_from_slice(&duration_ms.to_le_bytes());
                4
            }
        };
        Ok(len)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum ControlAck {
    Applied = 0,
    Rejected = 1,
}

impl ControlAck {
    pub fn decode(payload: &[u8]) -> Result<Self, Error> {
        match payload {
            [0] => Ok(Self::Applied),
            [1] => Ok(Self::Rejected),
            _ => Err(Error::InvalidPayload),
        }
    }

    pub const fn encode(self) -> [u8; 1] {
        [self as u8]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOLDEN: [u8; 19] = [
        1, 8, 0, 0, 0x44, 0x33, 0x22, 0x11, 0x88, 0x77, 0x66, 0x55, 0xcc, 0xbb, 0xaa, 0x99, 1, 0, 3,
    ];

    fn control_frame(payload: &[u8]) -> Frame<'_> {
        Frame {
            kind: MessageType::Control,
            node_id: 0x11223344,
            seq: 0x55667788,
            request_id: 0x99aabbcc,
            payload,
        }
    }

    #[test]
    fn golden_wire_format_is_little_endian() {
        let frame = control_frame(&[3]);
        let mut output = [0; MAX_FRAME_LEN];
        let len = frame.encode(&mut output).unwrap();
        assert_eq!(len, GOLDEN.len());
        assert_eq!(&output[..len], GOLDEN);
        assert_eq!(Frame::decode(&GOLDEN), Ok(frame));
    }

    #[test]
    fn rejects_every_truncated_prefix_and_trailing_bytes() {
        for end in 0..GOLDEN.len() {
            assert_eq!(Frame::decode(&GOLDEN[..end]), Err(Error::InvalidLength));
        }
        let mut extra = [0; 20];
        extra[..19].copy_from_slice(&GOLDEN);
        assert_eq!(Frame::decode(&extra), Err(Error::InvalidLength));
        let mut bad = GOLDEN;
        bad[17] = 0xff;
        assert_eq!(Frame::decode(&bad), Err(Error::InvalidLength));
    }

    #[test]
    fn rejects_unknown_version_type_and_reserved_flags() {
        for (offset, value, expected) in [
            (0, 2, Error::UnsupportedVersion),
            (1, 0, Error::UnknownType),
            (1, 15, Error::UnknownType),
            (2, 1, Error::UnsupportedFlags),
            (3, 1, Error::UnsupportedFlags),
        ] {
            let mut bad = GOLDEN;
            bad[offset] = value;
            assert_eq!(Frame::decode(&bad), Err(expected));
        }
    }

    #[test]
    fn frame_boundary_and_output_capacity_are_checked() {
        let payload = [0; MAX_PAYLOAD_LEN];
        let frame = Frame {
            kind: MessageType::Discover,
            node_id: 0,
            seq: u32::MAX,
            request_id: 0,
            payload: &payload,
        };
        let mut output = [0; MAX_FRAME_LEN];
        assert_eq!(frame.encode(&mut output), Ok(MAX_FRAME_LEN));
        assert_eq!(Frame::decode(&output), Ok(frame));
        assert_eq!(frame.encode(&mut output[..199]), Err(Error::BufferTooSmall));
        let large_payload = [0; MAX_PAYLOAD_LEN + 1];
        let large = Frame {
            payload: &large_payload,
            ..frame
        };
        assert_eq!(large.encode(&mut output), Err(Error::InvalidLength));
        assert_eq!(
            Frame::decode(&[0; MAX_FRAME_LEN + 1]),
            Err(Error::InvalidLength)
        );
    }

    #[test]
    fn all_assigned_message_types_round_trip() {
        for byte in 1..=14 {
            let kind = MessageType::try_from(byte).unwrap();
            let payload: &[u8] = match kind {
                MessageType::Control => &[3],
                MessageType::ControlAck => &[0],
                _ => &[],
            };
            let frame = Frame {
                kind,
                ..control_frame(payload)
            };
            let mut output = [0; MAX_FRAME_LEN];
            let len = frame.encode(&mut output).unwrap();
            assert_eq!(Frame::decode(&output[..len]), Ok(frame));
        }
    }

    #[test]
    fn switch_actions_round_trip_with_exact_payload_lengths() {
        for action in [
            Control::Off,
            Control::On,
            Control::Toggle,
            Control::Pulse { duration_ms: 1 },
            Control::Pulse {
                duration_ms: u32::MAX,
            },
        ] {
            let mut output = [0; 5];
            let len = action.encode(&mut output).unwrap();
            assert_eq!(Control::decode(&output[..len]), Ok(action));
            for end in 0..len {
                assert_eq!(Control::decode(&output[..end]), Err(Error::InvalidPayload));
            }
        }
        let mut output = [0; 5];
        Control::Pulse {
            duration_ms: 0x12345678,
        }
        .encode(&mut output)
        .unwrap();
        assert_eq!(output, [4, 0x78, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn invalid_payloads_and_zero_duration_are_rejected() {
        for payload in [
            &[][..],
            &[0],
            &[5],
            &[1, 0],
            &[4],
            &[4, 0, 0, 0, 0],
            &[4, 1, 0, 0, 0, 0],
        ] {
            assert_eq!(Control::decode(payload), Err(Error::InvalidPayload));
            assert_eq!(
                control_frame(payload).encode(&mut [0; 200]),
                Err(Error::InvalidPayload)
            );
        }
        assert_eq!(
            Control::Pulse { duration_ms: 0 }.encode(&mut [0; 5]),
            Err(Error::InvalidPayload)
        );
        let mut bad = GOLDEN;
        bad[18] = 0;
        assert_eq!(Frame::decode(&bad), Err(Error::InvalidPayload));
    }

    #[test]
    fn control_and_ack_require_nonzero_destination_and_request() {
        for kind in [MessageType::Control, MessageType::ControlAck] {
            let payload = if kind == MessageType::Control {
                &[3][..]
            } else {
                &[0][..]
            };
            let valid = Frame {
                kind,
                ..control_frame(payload)
            };
            for invalid in [
                Frame {
                    node_id: 0,
                    ..valid
                },
                Frame {
                    request_id: 0,
                    ..valid
                },
            ] {
                assert_eq!(invalid.encode(&mut [0; 200]), Err(Error::InvalidIdentity));
            }
        }
        for offset in [4, 12] {
            let mut bad = GOLDEN;
            bad[offset..offset + 4].fill(0);
            assert_eq!(Frame::decode(&bad), Err(Error::InvalidIdentity));
        }
    }

    #[test]
    fn acknowledgments_reject_unknown_results_and_extra_bytes() {
        for ack in [ControlAck::Applied, ControlAck::Rejected] {
            assert_eq!(ControlAck::decode(&ack.encode()), Ok(ack));
        }
        for payload in [&[][..], &[2], &[0, 0]] {
            assert_eq!(ControlAck::decode(payload), Err(Error::InvalidPayload));
            let frame = Frame {
                kind: MessageType::ControlAck,
                ..control_frame(payload)
            };
            assert_eq!(frame.encode(&mut [0; 200]), Err(Error::InvalidPayload));
        }
    }

    #[test]
    fn encode_errors_do_not_partially_write_output() {
        let mut output = [0xa5; 19];
        let invalid = control_frame(&[9]);
        assert_eq!(invalid.encode(&mut output), Err(Error::InvalidPayload));
        assert_eq!(
            control_frame(&[3]).encode(&mut output[..18]),
            Err(Error::BufferTooSmall)
        );
        assert_eq!(
            Control::Pulse { duration_ms: 1 }.encode(&mut output[..4]),
            Err(Error::BufferTooSmall)
        );
        assert_eq!(output, [0xa5; 19]);
    }

    #[test]
    fn arbitrary_bounded_bytes_never_panic_and_accepted_frames_reencode() {
        for offset in 0..GOLDEN.len() {
            for value in 0..=255 {
                let mut input = GOLDEN;
                input[offset] = value;
                if let Ok(frame) = Frame::decode(&input) {
                    let mut output = [0; MAX_FRAME_LEN];
                    let written = frame.encode(&mut output).unwrap();
                    assert_eq!(&output[..written], input);
                }
            }
        }
        let mut bytes = [0; 256];
        let mut seed = 0x12345678u32;
        for len in 0..=bytes.len() {
            for _ in 0..16 {
                for byte in &mut bytes[..len] {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    *byte = seed as u8;
                }
                if let Ok(frame) = Frame::decode(&bytes[..len]) {
                    let mut output = [0; MAX_FRAME_LEN];
                    let written = frame.encode(&mut output).unwrap();
                    assert_eq!(&output[..written], &bytes[..len]);
                }
            }
        }
    }
}
