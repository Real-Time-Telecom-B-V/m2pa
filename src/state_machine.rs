use std::fmt;

use crate::link_status::LinkState;

/// M2PA link states per RFC 4165 Section 4.
///
/// State transitions:
/// ```ignore
/// Out of Service → Not Aligned → Aligned → Proving → Aligned Ready → In Service
///                                                          ↑              |
///                                                          └──────────────┘
///                                                        (processor outage recovery)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M2paState {
    OutOfService,
    NotAligned,
    Aligned,
    Proving,
    AlignedReady,
    InService,
}

impl fmt::Display for M2paState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfService => write!(f, "Out of Service"),
            Self::NotAligned => write!(f, "Not Aligned"),
            Self::Aligned => write!(f, "Aligned"),
            Self::Proving => write!(f, "Proving"),
            Self::AlignedReady => write!(f, "Aligned Ready"),
            Self::InService => write!(f, "In Service"),
        }
    }
}

/// M2PA link state machine.
///
/// Tracks the current state and provides state transitions based on
/// received Link Status messages per RFC 4165 Section 4.
#[derive(Debug)]
pub struct M2paStateMachine {
    state: M2paState,
}

impl M2paStateMachine {
    pub fn new() -> Self {
        Self {
            state: M2paState::OutOfService,
        }
    }

    pub fn state(&self) -> M2paState {
        self.state
    }

    /// Process a received LinkState and transition the state machine.
    ///
    /// Returns the new state after the transition.
    pub fn on_link_status(&mut self, link_state: LinkState) -> M2paState {
        self.state = match (self.state, link_state) {
            // Out of Service: start alignment when we receive Alignment
            (M2paState::OutOfService, LinkState::Alignment) => M2paState::NotAligned,

            // Not Aligned: peer sent Alignment, we're now aligned
            (M2paState::NotAligned, LinkState::Alignment) => M2paState::Aligned,

            // Aligned: peer starts proving
            (M2paState::Aligned, LinkState::ProvingNormal)
            | (M2paState::Aligned, LinkState::ProvingEmergency) => M2paState::Proving,

            // Proving: peer says Ready, we're aligned and ready
            (M2paState::Proving, LinkState::Ready) => M2paState::AlignedReady,

            // Proving: peer continues proving, stay in Proving
            (M2paState::Proving, LinkState::ProvingNormal)
            | (M2paState::Proving, LinkState::ProvingEmergency) => M2paState::Proving,

            // Aligned Ready: transition to In Service
            (M2paState::AlignedReady, LinkState::Ready) => M2paState::InService,

            // Processor outage from In Service
            (M2paState::InService, LinkState::ProcessorOutage) => M2paState::AlignedReady,

            // Processor recovered back to In Service
            (M2paState::AlignedReady, LinkState::ProcessorRecovered) => M2paState::InService,

            // Busy/Busy Ended in In Service: stay in service (flow control)
            (M2paState::InService, LinkState::Busy)
            | (M2paState::InService, LinkState::BusyEnded) => M2paState::InService,

            // Any unexpected transition: go Out of Service
            (_, _) => M2paState::OutOfService,
        };
        self.state
    }

    /// Initiate alignment from Out of Service state.
    /// Returns true if the transition was valid.
    pub fn start(&mut self) -> bool {
        if self.state == M2paState::OutOfService {
            self.state = M2paState::NotAligned;
            true
        } else {
            false
        }
    }

    /// Force the link out of service.
    pub fn stop(&mut self) {
        self.state = M2paState::OutOfService;
    }
}

impl Default for M2paStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for M2paStateMachine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "M2PA State Machine [state={}]", self.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state() {
        let sm = M2paStateMachine::new();
        assert_eq!(sm.state(), M2paState::OutOfService);
    }

    #[test]
    fn normal_alignment_sequence() {
        let mut sm = M2paStateMachine::new();

        // Start alignment
        assert!(sm.start());
        assert_eq!(sm.state(), M2paState::NotAligned);

        // Receive Alignment from peer
        sm.on_link_status(LinkState::Alignment);
        assert_eq!(sm.state(), M2paState::Aligned);

        // Receive Proving Normal from peer
        sm.on_link_status(LinkState::ProvingNormal);
        assert_eq!(sm.state(), M2paState::Proving);

        // Receive Ready from peer
        sm.on_link_status(LinkState::Ready);
        assert_eq!(sm.state(), M2paState::AlignedReady);

        // Receive Ready again -> In Service
        sm.on_link_status(LinkState::Ready);
        assert_eq!(sm.state(), M2paState::InService);
    }

    #[test]
    fn emergency_proving() {
        let mut sm = M2paStateMachine::new();
        sm.start();
        sm.on_link_status(LinkState::Alignment);
        sm.on_link_status(LinkState::ProvingEmergency);
        assert_eq!(sm.state(), M2paState::Proving);
    }

    #[test]
    fn processor_outage_and_recovery() {
        let mut sm = M2paStateMachine::new();
        sm.start();
        sm.on_link_status(LinkState::Alignment);
        sm.on_link_status(LinkState::ProvingNormal);
        sm.on_link_status(LinkState::Ready);
        sm.on_link_status(LinkState::Ready);
        assert_eq!(sm.state(), M2paState::InService);

        // Processor outage
        sm.on_link_status(LinkState::ProcessorOutage);
        assert_eq!(sm.state(), M2paState::AlignedReady);

        // Recovery
        sm.on_link_status(LinkState::ProcessorRecovered);
        assert_eq!(sm.state(), M2paState::InService);
    }

    #[test]
    fn busy_stays_in_service() {
        let mut sm = M2paStateMachine::new();
        sm.start();
        sm.on_link_status(LinkState::Alignment);
        sm.on_link_status(LinkState::ProvingNormal);
        sm.on_link_status(LinkState::Ready);
        sm.on_link_status(LinkState::Ready);

        sm.on_link_status(LinkState::Busy);
        assert_eq!(sm.state(), M2paState::InService);

        sm.on_link_status(LinkState::BusyEnded);
        assert_eq!(sm.state(), M2paState::InService);
    }

    #[test]
    fn stop_forces_out_of_service() {
        let mut sm = M2paStateMachine::new();
        sm.start();
        sm.on_link_status(LinkState::Alignment);
        sm.stop();
        assert_eq!(sm.state(), M2paState::OutOfService);
    }

    #[test]
    fn start_only_from_oos() {
        let mut sm = M2paStateMachine::new();
        sm.start();
        assert!(!sm.start()); // Already NotAligned, can't start again
    }

    #[test]
    fn display() {
        let sm = M2paStateMachine::new();
        assert_eq!(format!("{sm}"), "M2PA State Machine [state=Out of Service]");
    }
}
