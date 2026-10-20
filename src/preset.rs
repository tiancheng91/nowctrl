//! Logical device presets. No pin mapping or physical output is implied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceType {
    Unknown,
    Switch,
    RcBasic,
}
impl DeviceType {
    pub const fn supports_switch(self) -> bool {
        matches!(self, Self::Switch)
    }
    pub const fn supports_rc(self) -> bool {
        matches!(self, Self::RcBasic)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RcAction {
    Stop,
    Forward,
    BoostForward,
    Reverse,
    Left,
    Right,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Axes {
    pub throttle: i8,
    pub steering: i8,
}
impl RcAction {
    /// Preview percentages, not calibrated PWM/servo values. Steering alone has no throttle.
    pub const fn axes(self) -> Axes {
        match self {
            Self::Stop => Axes {
                throttle: 0,
                steering: 0,
            },
            Self::Forward => Axes {
                throttle: 35,
                steering: 0,
            },
            Self::BoostForward => Axes {
                throttle: 65,
                steering: 0,
            },
            Self::Reverse => Axes {
                throttle: -30,
                steering: 0,
            },
            Self::Left => Axes {
                throttle: 0,
                steering: -100,
            },
            Self::Right => Axes {
                throttle: 0,
                steering: 100,
            },
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RcError {
    StaleSequence,
    ClockWentBackwards,
    InvalidDeadline,
}

/// Authenticated session context and sender validation are caller duties.
/// Recreate only when a new authenticated session is established; no sequence wrap.
#[derive(Default)]
pub struct RcState {
    axes: Axes,
    last_seq: Option<u32>,
    last_time: u64,
    deadline: Option<u64>,
}
impl RcState {
    pub const WATCHDOG_MS: u64 = 300;
    pub fn update(&mut self, seq: u32, action: RcAction, now_ms: u64) -> Result<(), RcError> {
        if now_ms < self.last_time {
            return Err(RcError::ClockWentBackwards);
        }
        self.advance(now_ms)?;
        if self.last_seq.is_some_and(|last| seq <= last) {
            return Err(RcError::StaleSequence);
        }
        let deadline = now_ms
            .checked_add(Self::WATCHDOG_MS)
            .ok_or(RcError::InvalidDeadline)?;
        self.axes = action.axes();
        self.last_seq = Some(seq);
        self.deadline = if action == RcAction::Stop {
            None
        } else {
            Some(deadline)
        };
        Ok(())
    }
    pub fn advance(&mut self, now_ms: u64) -> Result<Axes, RcError> {
        if now_ms < self.last_time {
            return Err(RcError::ClockWentBackwards);
        }
        self.last_time = now_ms;
        if self.deadline.is_some_and(|deadline| now_ms >= deadline) {
            self.stop();
        }
        Ok(self.axes)
    }
    /// Used for disconnect, disable, invalid session and output watchdog events.
    pub fn stop(&mut self) {
        self.axes = Axes::default();
        self.deadline = None;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presets_are_bounded_and_unknown_is_read_only() {
        assert!(!DeviceType::Unknown.supports_switch());
        assert!(!DeviceType::Unknown.supports_rc());
        assert!(DeviceType::Switch.supports_switch());
        assert!(!DeviceType::Switch.supports_rc());
        assert!(DeviceType::RcBasic.supports_rc());
        for action in [
            RcAction::Stop,
            RcAction::Forward,
            RcAction::BoostForward,
            RcAction::Reverse,
            RcAction::Left,
            RcAction::Right,
        ] {
            let axes = action.axes();
            assert!((-100..=100).contains(&axes.throttle));
            assert!((-100..=100).contains(&axes.steering));
        }
        assert!(RcAction::BoostForward.axes().throttle > RcAction::Forward.axes().throttle);
        assert!(RcAction::Reverse.axes().throttle < 0);
        assert_eq!(
            RcAction::Left.axes(),
            Axes {
                throttle: 0,
                steering: -100
            }
        );
    }
    #[test]
    fn latest_frame_wins_and_duplicate_does_not_refresh_watchdog() {
        let mut state = RcState::default();
        assert_eq!(state.advance(0), Ok(Axes::default()));
        state.update(1, RcAction::Forward, 0).unwrap();
        assert_eq!(
            state.update(1, RcAction::BoostForward, 200),
            Err(RcError::StaleSequence)
        );
        assert_eq!(state.advance(299), Ok(RcAction::Forward.axes()));
        assert_eq!(state.advance(300), Ok(Axes::default()));
        state.update(2, RcAction::Reverse, 301).unwrap();
        assert_eq!(state.advance(301), Ok(RcAction::Reverse.axes()));
        assert_eq!(
            state.update(1, RcAction::Forward, 302),
            Err(RcError::StaleSequence)
        );
    }
    #[test]
    fn stop_disable_and_watchdog_neutralize_both_axes() {
        let mut state = RcState::default();
        state.update(1, RcAction::Left, 0).unwrap();
        state.update(2, RcAction::Stop, 1).unwrap();
        assert_eq!(state.advance(1), Ok(Axes::default()));
        state.update(3, RcAction::Right, 2).unwrap();
        state.stop();
        assert_eq!(state.advance(2), Ok(Axes::default()));
        assert_eq!(
            state.update(4, RcAction::Forward, 1),
            Err(RcError::ClockWentBackwards)
        );
    }
}
