use crate::agent::state::AgentState;

/// Closures based event handler.
///
/// Any closure from given handler can be invoked in any thread, usually from internal dedicated
/// libjuice thread.
///
/// # Example
/// ```
/// # use libjuice_rs::Handler;
/// let h: Handler = Handler::default()
///     .state_handler(|s| println!("State changed to: {:?}", s))
///     .candidate_handler(|c| println!("Local candidate: {:?}", c));
/// ```
#[derive(Default)]
pub struct Handler {
    /// ICE state change handler
    on_state_change: Option<Box<dyn FnMut(AgentState) + Send + 'static>>,
    /// Local ICE candidate handler
    on_candidate: Option<Box<dyn FnMut(String) + Send + 'static>>,
    /// Gathering stage finish handler
    on_gathering_done: Option<Box<dyn FnMut() + Send + 'static>>,
    /// Incoming packet
    on_recv: Option<Box<dyn FnMut(&[u8]) + Send + 'static>>,
}

impl Handler {
    /// Set ICE state change handler
    pub fn state_handler<F>(mut self, f: F) -> Self
    where
        F: FnMut(AgentState),
        F: Send + Sync + 'static,
    {
        self.on_state_change = Some(Box::new(f));
        self
    }

    /// Set local candidate handler
    pub fn candidate_handler<F>(mut self, f: F) -> Self
    where
        F: FnMut(String),
        F: Send + 'static,
    {
        self.on_candidate = Some(Box::new(f));
        self
    }

    /// Set gathering finish handler
    pub fn gathering_finished_handler<F>(mut self, f: F) -> Self
    where
        F: FnMut(),
        F: Send + 'static,
    {
        self.on_gathering_done = Some(Box::new(f));
        self
    }

    /// Set incoming packet handler
    pub fn recv_handler<F>(mut self, f: F) -> Self
    where
        F: FnMut(&[u8]),
        F: Send + 'static,
    {
        self.on_recv = Some(Box::new(f));
        self
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
