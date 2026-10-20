//! GPIO-independent button gestures and finite LED patterns.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Gesture {
    NextChannel,
    Trigger,
    Pairing,
    WebConfig,
    ResetRequested,
}

#[derive(Clone, Copy, Debug)]
pub struct ButtonConfig {
    pub debounce_ms: u64,
    pub double_click_ms: u64,
    pub pairing_ms: u64,
    pub web_ms: u64,
    pub reset_ms: u64,
}
impl Default for ButtonConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 30,
            double_click_ms: 300,
            pairing_ms: 3000,
            web_ms: 8000,
            reset_ms: 15000,
        }
    }
}

pub struct Button {
    config: ButtonConfig,
    raw: bool,
    stable: bool,
    changed_at: u64,
    pressed_at: u64,
    released_at: Option<u64>,
    second_click: bool,
    last_time: u64,
}

impl Button {
    pub fn new(config: ButtonConfig) -> Option<Self> {
        if config.debounce_ms == 0
            || config.double_click_ms <= config.debounce_ms
            || config.pairing_ms <= config.double_click_ms
            || config.web_ms <= config.pairing_ms
            || config.reset_ms <= config.web_ms
        {
            return None;
        }
        Some(Self {
            config,
            raw: false,
            stable: false,
            changed_at: 0,
            pressed_at: 0,
            released_at: None,
            second_click: false,
            last_time: 0,
        })
    }

    /// Poll with monotonic milliseconds and normalized pressed state (not pin level).
    /// A backward clock returns None without changing state. Long gestures fire only
    /// on release; ResetRequested must enter a separate physical confirmation flow.
    pub fn update(&mut self, now: u64, pressed: bool) -> Option<Gesture> {
        if now < self.last_time {
            return None;
        }
        self.last_time = now;
        if pressed != self.raw {
            self.raw = pressed;
            self.changed_at = now;
        }
        let mut event = None;
        if self.raw != self.stable && now - self.changed_at >= self.config.debounce_ms {
            self.stable = self.raw;
            if self.stable {
                self.pressed_at = now;
                self.second_click = self
                    .released_at
                    .is_some_and(|at| now - at <= self.config.double_click_ms);
                if self.released_at.is_some() && !self.second_click {
                    self.released_at = None;
                    event = Some(Gesture::NextChannel);
                }
            } else {
                let held = now - self.pressed_at;
                let long = if held >= self.config.reset_ms {
                    Some(Gesture::ResetRequested)
                } else if held >= self.config.web_ms {
                    Some(Gesture::WebConfig)
                } else if held >= self.config.pairing_ms {
                    Some(Gesture::Pairing)
                } else {
                    None
                };
                if long.is_some() {
                    self.released_at = None;
                    self.second_click = false;
                    return long;
                }
                if self.second_click {
                    self.released_at = None;
                    self.second_click = false;
                    return Some(Gesture::Trigger);
                }
                self.released_at = Some(now);
            }
        }
        if !self.stable
            && !self.raw
            && self
                .released_at
                .is_some_and(|at| now - at >= self.config.double_click_ms)
        {
            self.released_at = None;
            return Some(Gesture::NextChannel);
        }
        event
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LedPattern {
    Channel(u8),
    Success,
    Failure,
    Offline,
    Pairing,
    WebConfig,
    ResetConfirm,
}

impl LedPattern {
    /// Logical LED state at elapsed milliseconds. None means finished/off.
    /// Does not choose a GPIO, polarity, or WS2812 driver.
    pub fn level(self, elapsed: u64) -> Option<bool> {
        let (pulses, on, off, groups, gap) = match self {
            Self::Channel(n) if (1..=8).contains(&n) => (u64::from(n), 120, 180, 2, 600),
            Self::Channel(_) => return None,
            Self::Success => (1, 250, 150, 1, 0),
            Self::Failure => (3, 60, 90, 1, 0),
            Self::Offline => (2, 400, 200, 1, 0),
            Self::Pairing => (2, 100, 100, 1, 0),
            Self::WebConfig => (1, 600, 200, 1, 0),
            Self::ResetConfirm => (5, 50, 50, 1, 0),
        };
        let group_ms = pulses * (on + off);
        let span = group_ms + gap;
        if elapsed >= groups * span - gap {
            return None;
        }
        let within = elapsed % span;
        Some(within < group_ms && within % (on + off) < on)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(button: &mut Button, time: u64, pressed: bool) -> Option<Gesture> {
        assert_eq!(button.update(time, pressed), None);
        button.update(time + 30, pressed)
    }
    #[test]
    fn bounce_and_click_window() {
        let mut b = Button::new(ButtonConfig::default()).unwrap();
        for (t, v) in [(0, true), (10, false), (20, true), (29, false)] {
            assert_eq!(b.update(t, v), None);
        }
        assert_eq!(sample(&mut b, 100, true), None);
        assert_eq!(sample(&mut b, 200, false), None);
        assert_eq!(b.update(529, false), None);
        assert_eq!(b.update(530, false), Some(Gesture::NextChannel));
        assert_eq!(b.update(1000, false), None);
    }
    #[test]
    fn double_click_consumes_single() {
        let mut b = Button::new(ButtonConfig::default()).unwrap();
        sample(&mut b, 0, true);
        sample(&mut b, 100, false);
        sample(&mut b, 200, true);
        assert_eq!(sample(&mut b, 300, false), Some(Gesture::Trigger));
        assert_eq!(b.update(1000, false), None);
    }
    #[test]
    fn hold_priorities_resolve_once_on_release() {
        for (held, event) in [
            (3000, Gesture::Pairing),
            (8000, Gesture::WebConfig),
            (15000, Gesture::ResetRequested),
            (20000, Gesture::ResetRequested),
        ] {
            let mut b = Button::new(ButtonConfig::default()).unwrap();
            sample(&mut b, 0, true);
            assert_eq!(b.update(held, true), None);
            assert_eq!(sample(&mut b, held, false), Some(event));
            assert_eq!(b.update(held + 1000, false), None);
        }
    }
    #[test]
    fn long_second_press_suppresses_click_and_shorter_holds() {
        let mut b = Button::new(ButtonConfig::default()).unwrap();
        sample(&mut b, 0, true);
        sample(&mut b, 100, false);
        sample(&mut b, 200, true);
        assert_eq!(sample(&mut b, 8200, false), Some(Gesture::WebConfig));
        assert_eq!(b.update(9000, false), None);
        assert!(
            Button::new(ButtonConfig {
                reset_ms: 1,
                ..ButtonConfig::default()
            })
            .is_none()
        );
    }
    #[test]
    fn channel_led_finishes_after_two_groups_and_idle_is_off() {
        let p = LedPattern::Channel(2);
        assert_eq!(p.level(0), Some(true));
        assert_eq!(p.level(120), Some(false));
        assert_eq!(p.level(300), Some(true));
        assert_eq!(p.level(600), Some(false));
        assert_eq!(p.level(1200), Some(true));
        assert_eq!(p.level(1800), None);
        assert_eq!(LedPattern::Channel(0).level(0), None);
        assert_eq!(LedPattern::Channel(9).level(0), None);
        for p in [
            LedPattern::Success,
            LedPattern::Failure,
            LedPattern::Offline,
            LedPattern::Pairing,
            LedPattern::WebConfig,
            LedPattern::ResetConfirm,
        ] {
            assert_eq!(p.level(0), Some(true));
            assert_eq!(p.level(10000), None);
        }
    }
}
