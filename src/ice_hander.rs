use crate::agent_state::AgentState;

/// Closures based agent's event handler.
/// # Example
/// ```
/// # use libjuice::Handler;
/// let h: Box<Handler> = Handler::default()
///     .state_handler(|s| println!("State changed to: {:?}", s))
///     .candidate_handler(|c| println!("Local candidate: {:?}", c))
///     .to_box();
/// ```
#[derive(Default)]
pub struct Handler {
    /// ICE state change handler
    on_state_change: Option<Box<dyn FnMut(AgentState)>>,
    /// Local ICE candidate handler
    on_candidate: Option<Box<dyn FnMut(String)>>,
    /// Gathering stage finish handler
    on_gathering_done: Option<Box<dyn FnMut()>>,
    /// Incoming packet
    on_recv: Option<Box<dyn FnMut(&[u8])>>,
}

impl Handler {
    /// Set ICE state change handler
    pub fn state_handler(mut self, f: impl FnMut(AgentState) + 'static) -> Self {
        self.on_state_change = Some(Box::new(f));
        self
    }

    /// Set local candidate handler
    pub fn candidate_handler(mut self, f: impl FnMut(String) + 'static) -> Self {
        self.on_candidate = Some(Box::new(f));
        self
    }

    /// Set gathering finish handler
    pub fn gathering_finished_handler(mut self, f: impl FnMut() + 'static) -> Self {
        self.on_gathering_done = Some(Box::new(f));
        self
    }

    /// Set incoming packet handler
    pub fn recv_handler(mut self, f: impl FnMut(&[u8]) + 'static) -> Self {
        self.on_recv = Some(Box::new(f));
        self
    }

    /// Convert to box.
    pub fn to_box(self) -> Box<Self> {
        Box::new(self)
    }

    pub(crate) fn on_state_changed(&mut self, state: AgentState) {
        if let Some(f) = &mut self.on_state_change {
            f(state)
        }
    }

    pub(crate) fn on_candidate(&mut self, candidate: String) {
        if let Some(f) = &mut self.on_candidate {
            f(candidate)
        }
    }

    pub(crate) fn on_gathering_done(&mut self) {
        if let Some(f) = &mut self.on_gathering_done {
            f()
        }
    }

    pub(crate) fn on_recv(&mut self, packet: &[u8]) {
        if let Some(f) = &mut self.on_recv {
            f(packet)
        }
    }
}
