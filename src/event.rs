// SPDX-License-Identifier: GPL-2.0-or-later

use std::fmt;
use crate::domain::Domain;
use crate::ecodes;

pub type EventValue = i32;
pub type Channel = (EventCode, Domain);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EventType(u16);

impl EventType {
    pub const KEY: EventType = EventType(ecodes::EV_KEY);
    pub const ABS: EventType = EventType(ecodes::EV_ABS);
    pub const REL: EventType = EventType(ecodes::EV_REL);
    pub const REP: EventType = EventType(ecodes::EV_REP);
    pub const SYN: EventType = EventType(ecodes::EV_SYN);
    pub const MSC: EventType = EventType(ecodes::EV_MSC);

    pub fn is_key(self) -> bool {
        self == EventType::KEY
    }
    pub fn is_abs(self) -> bool {
        self == EventType::ABS
    }
    pub fn is_rel(self) -> bool {
        self == EventType::REL
    }
    pub fn is_rep(self) -> bool {
        self == EventType::REP
    }
    pub fn is_syn(self) -> bool {
        self == EventType::SYN
    }
}

impl EventType {
    pub const fn new(value: u16) -> EventType {
        debug_assert!(value <= ecodes::EV_MAX);
        EventType(value)
    }
}

/// Internally, all event codes have their respective type attached to them. This avoids
/// logic errors, since some codes make no sense for types other than the one they were
/// intended for, and various FFI calls may emit undefined behaviour if we provide them
/// with invalid event types.
///
/// Upholds invariant: the code is a valid code for this type.
/// Creating an EventCode with a nonexistent (type, code) pair is considered undefined behaviour.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EventCode {
    ev_type: EventType,
    code: u16,
}

impl EventCode {
    pub const MSC_SCAN: EventCode = EventCode {
        ev_type: EventType::MSC,
        code: ecodes::MSC_SCAN,
    };

    pub const fn new(ev_type: EventType, code: u16) -> EventCode {
        EventCode { ev_type, code }
    }

    pub const fn ev_type(self) -> EventType {
        self.ev_type
    }

    pub const fn code(self) -> u16 {
        self.code
    }

    pub fn virtual_ev_type(self) -> VirtualEventType {
        if self.ev_type().is_key() {
            if ecodes::is_button_code(self) {
                VirtualEventType::Button
            } else {
                VirtualEventType::Key
            }
        } else {
            VirtualEventType::Other(self.ev_type())
        }
    }
}

impl From<EventType> for u16 {
    fn from(ev_type: EventType) -> u16 {
        ev_type.0
    }
}

impl From<EventType> for u32 {
    fn from(ev_type: EventType) -> u32 {
        u16::from(ev_type) as u32
    }
}

/// In the kernel, the type EV_KEY is used for both keys like key:a and buttons like btn:left.
/// This could confuse the user if a "--map key" argument were to also match btn:something events.
///
/// To resolve this, we introduce a virtual event type which is equivalent to the kernel type
/// except that EV_KEY is split into two different types, one for events with EV_KEY, KEY_* codes
/// and one for events with EV_KEY, BTN_* codes.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum VirtualEventType {
    Key,
    Button,
    Other(EventType),
}

impl VirtualEventType {
    pub const KEY: &'static str = "key";
    pub const BUTTON: &'static str = "btn";

    pub fn ev_type(self) -> EventType {
        match self {
            VirtualEventType::Key | VirtualEventType::Button => EventType::KEY,
            VirtualEventType::Other(ev_type) => ev_type,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Event {
    pub code: EventCode,
    pub value: EventValue,

    /// The value this event had the last time it was emitted by a device.
    pub previous_value: EventValue,

    pub domain: Domain,
    pub namespace: Namespace,
}

impl Event {
    pub fn new(code: EventCode,
               value: EventValue,
               previous_value: EventValue,
               domain: Domain,
               namespace: Namespace
    ) -> Event {
        Event { code, value, previous_value, domain, namespace }
    }

    pub fn with_domain(mut self, new_domain: Domain) -> Event {
        self.domain = new_domain;
        self
    }

    pub fn ev_type(self) -> EventType {
        self.code.ev_type()
    }

    pub fn channel(self) -> Channel {
        (self.code, self.domain)
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = ecodes::event_name(self.code);
        write!(f, "{}:{}", name, self.value)
    }
}

impl fmt::Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = ecodes::event_name(self.code);
        write!(f, "{}:{}..{}@{:?}", name, self.previous_value, self.value, self.domain)
    }
}


/// Namespaces are an internal concept that is not visible to the user. They are like domains, but
/// then on a higher level such that even a filter with an empty domain cannot match events within a
/// different namespace.
#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum Namespace {
    /// This event was generated by an input device and has not yet entered the processing stream
    /// from the end user's perspective. It is not affected by any `StreamEntry` except `StreamEntry::Input`.
    Input,
    /// This event is in the processing stream.
    User,
    /// This event was generated by --map yield or similar. It is not affected by any `StreamEntry`
    /// except for `StreamEntry::Output`.
    Yielded,
    /// This event was caught by an --output and shall now be sent to an output device. It is not
    /// affected by any StreamEntry.
    Output,
}