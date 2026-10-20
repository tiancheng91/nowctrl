//! Virtual switch execution and bounded delivery policy; caller authenticates peers.
use crate::protocol::{Control, ControlAck};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplyError {
    WrongNode,
    InvalidRequest,
    ClockWentBackwards,
    StaleRequest,
    ConflictingRetry,
    InvalidPulse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Cached {
    request_id: u32,
    command: Control,
    result: ControlAck,
}

/// One paired controller session per node. Recreate on a newly authenticated session.
/// Monotonic request IDs must not wrap within a session; rekey before exhaustion.
pub struct VirtualNode {
    node_id: u32,
    output: bool,
    pulse_until: Option<u64>,
    last_time: u64,
    last_request: Option<Cached>,
}

impl VirtualNode {
    pub fn new(node_id: u32) -> Option<Self> {
        (node_id != 0).then_some(Self {
            node_id,
            output: false,
            pulse_until: None,
            last_time: 0,
            last_request: None,
        })
    }

    pub fn output(&self) -> bool {
        self.output
    }

    pub fn advance(&mut self, now_ms: u64) -> Result<(), ApplyError> {
        if now_ms < self.last_time {
            return Err(ApplyError::ClockWentBackwards);
        }
        self.last_time = now_ms;
        if self.pulse_until.is_some_and(|until| now_ms >= until) {
            self.output = false;
            self.pulse_until = None;
        }
        Ok(())
    }

    pub fn apply(
        &mut self,
        node_id: u32,
        request_id: u32,
        command: Control,
        now_ms: u64,
    ) -> Result<ControlAck, ApplyError> {
        if node_id != self.node_id {
            return Err(ApplyError::WrongNode);
        }
        if request_id == 0 {
            return Err(ApplyError::InvalidRequest);
        }
        self.advance(now_ms)?;
        if let Some(last) = self.last_request {
            if request_id < last.request_id {
                return Err(ApplyError::StaleRequest);
            }
            if request_id == last.request_id {
                return if command == last.command {
                    Ok(last.result)
                } else {
                    Err(ApplyError::ConflictingRetry)
                };
            }
        }
        let deadline = match command {
            Control::Pulse { duration_ms } => Some(
                now_ms
                    .checked_add(u64::from(duration_ms))
                    .filter(|_| duration_ms != 0)
                    .ok_or(ApplyError::InvalidPulse)?,
            ),
            _ => None,
        };
        self.output = match command {
            Control::Off => false,
            Control::On | Control::Pulse { .. } => true,
            Control::Toggle => !self.output,
        };
        self.pulse_until = deadline;
        self.last_request = Some(Cached {
            request_id,
            command,
            result: ControlAck::Applied,
        });
        Ok(ControlAck::Applied)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Request {
    pub node_id: u32,
    pub request_id: u32,
    pub command: Control,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Delivery {
    Send(Request),
    Waiting,
    TimedOut,
    Complete(ControlAck),
}

/// Stop-and-wait delivery: at most one in-flight request per controller session.
/// Attempts are spaced 200ms, 400ms, then an 800ms final ACK window.
pub struct Pending {
    request: Request,
    attempts: u8,
    deadline: u64,
    last_time: u64,
    result: Option<ControlAck>,
    expired: bool,
}

impl Pending {
    pub fn new(request: Request, now_ms: u64) -> Option<Self> {
        if request.node_id == 0
            || request.request_id == 0
            || request.command == (Control::Pulse { duration_ms: 0 })
        {
            return None;
        }
        Some(Self {
            request,
            attempts: 0,
            deadline: now_ms,
            last_time: now_ms,
            result: None,
            expired: false,
        })
    }

    pub fn poll(&mut self, now_ms: u64) -> Result<Delivery, ApplyError> {
        if now_ms < self.last_time {
            return Err(ApplyError::ClockWentBackwards);
        }
        self.last_time = now_ms;
        if let Some(result) = self.result {
            return Ok(Delivery::Complete(result));
        }
        if self.expired {
            return Ok(Delivery::TimedOut);
        }
        if now_ms < self.deadline {
            return Ok(Delivery::Waiting);
        }
        if self.attempts == 3 {
            self.expired = true;
            return Ok(Delivery::TimedOut);
        }
        let delay = 200u64 << self.attempts;
        self.deadline = now_ms.saturating_add(delay);
        self.attempts += 1;
        Ok(Delivery::Send(self.request))
    }

    /// Only call after transport identity/session authentication and frame validation.
    pub fn acknowledge(
        &mut self,
        node_id: u32,
        request_id: u32,
        result: ControlAck,
        now_ms: u64,
    ) -> bool {
        if self.expired
            || self.result.is_some()
            || self.attempts == 0
            || now_ms < self.last_time
            || (self.attempts == 3 && now_ms >= self.deadline)
            || node_id != self.request.node_id
            || request_id != self.request.request_id
        {
            return false;
        }
        self.last_time = now_ms;
        self.result = Some(result);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toggle_retry_and_stale_requests_never_reexecute() {
        let mut node = VirtualNode::new(7).unwrap();
        assert_eq!(
            node.apply(7, 1, Control::Toggle, 0),
            Ok(ControlAck::Applied)
        );
        assert_eq!(
            node.apply(7, 1, Control::Toggle, 1),
            Ok(ControlAck::Applied)
        );
        assert!(node.output());
        assert_eq!(
            node.apply(7, 1, Control::Off, 2),
            Err(ApplyError::ConflictingRetry)
        );
        node.apply(7, 2, Control::Off, 3).unwrap();
        assert_eq!(
            node.apply(7, 1, Control::Toggle, 4),
            Err(ApplyError::StaleRequest)
        );
        assert!(!node.output());
    }
    #[test]
    fn pulse_retries_do_not_extend_deadline() {
        let mut node = VirtualNode::new(7).unwrap();
        node.apply(7, 1, Control::Pulse { duration_ms: 100 }, 0)
            .unwrap();
        node.apply(7, 1, Control::Pulse { duration_ms: 100 }, 99)
            .unwrap();
        node.advance(100).unwrap();
        assert!(!node.output());
        node.apply(7, 1, Control::Pulse { duration_ms: 100 }, 101)
            .unwrap();
        assert!(!node.output());
        node.apply(7, 2, Control::On, 102).unwrap();
        node.advance(1000).unwrap();
        assert!(node.output());
    }
    #[test]
    fn rejected_inputs_never_change_output() {
        let mut node = VirtualNode::new(7).unwrap();
        assert_eq!(node.apply(8, 1, Control::On, 0), Err(ApplyError::WrongNode));
        assert_eq!(
            node.apply(7, 0, Control::On, 0),
            Err(ApplyError::InvalidRequest)
        );
        assert_eq!(
            node.apply(7, 1, Control::Pulse { duration_ms: 0 }, 0),
            Err(ApplyError::InvalidPulse)
        );
        node.advance(10).unwrap();
        assert_eq!(
            node.apply(7, 1, Control::On, 9),
            Err(ApplyError::ClockWentBackwards)
        );
        assert!(!node.output());
        assert!(VirtualNode::new(0).is_none());
    }
    #[test]
    fn bounded_retry_and_ack_correlation() {
        let request = Request {
            node_id: 7,
            request_id: 1,
            command: Control::Toggle,
        };
        let mut pending = Pending::new(request, 0).unwrap();
        assert!(!pending.acknowledge(7, 1, ControlAck::Applied, 0));
        assert_eq!(pending.poll(0), Ok(Delivery::Send(request)));
        assert!(!pending.acknowledge(8, 1, ControlAck::Applied, 1));
        assert!(!pending.acknowledge(7, 2, ControlAck::Applied, 1));
        assert_eq!(pending.poll(199), Ok(Delivery::Waiting));
        assert_eq!(pending.poll(200), Ok(Delivery::Send(request)));
        assert_eq!(pending.poll(600), Ok(Delivery::Send(request)));
        assert_eq!(pending.poll(1400), Ok(Delivery::TimedOut));
        assert!(!pending.acknowledge(7, 1, ControlAck::Applied, 1401));
        assert_eq!(pending.poll(5000), Ok(Delivery::TimedOut));
    }
    #[test]
    fn ack_is_terminal_and_late_ack_is_rejected_without_poll() {
        let request = Request {
            node_id: 7,
            request_id: 1,
            command: Control::On,
        };
        let mut p = Pending::new(request, 0).unwrap();
        p.poll(0).unwrap();
        assert!(p.acknowledge(7, 1, ControlAck::Rejected, 10));
        assert_eq!(p.poll(1000), Ok(Delivery::Complete(ControlAck::Rejected)));
        assert!(!p.acknowledge(7, 1, ControlAck::Applied, 1001));
        let mut p = Pending::new(request, 0).unwrap();
        for now in [0, 200, 600] {
            p.poll(now).unwrap();
        }
        assert!(!p.acknowledge(7, 1, ControlAck::Applied, 1400));
    }
    #[test]
    fn hundred_commands_with_lost_acks_duplicate_packets_and_wrong_nodes() {
        let mut node = VirtualNode::new(7).unwrap();
        let mut other = VirtualNode::new(8).unwrap();
        for id in 1..=100 {
            let start = u64::from(id) * 2000;
            let request = Request {
                node_id: 7,
                request_id: id,
                command: Control::Toggle,
            };
            let mut p = Pending::new(request, start).unwrap();
            for now in [start, start + 200, start + 600] {
                assert_eq!(p.poll(now), Ok(Delivery::Send(request)));
                assert_eq!(
                    other.apply(7, id, Control::Toggle, now),
                    Err(ApplyError::WrongNode)
                );
                let ack = node.apply(7, id, Control::Toggle, now).unwrap();
                node.apply(7, id, Control::Toggle, now).unwrap();
                if now == start + 600 {
                    assert!(p.acknowledge(7, id, ack, now + 1));
                }
            }
            assert_eq!(
                p.poll(start + 601),
                Ok(Delivery::Complete(ControlAck::Applied))
            );
            assert_eq!(node.output(), id % 2 == 1);
            assert!(!other.output());
        }
    }
}
