use libjuice_sys as sys;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AgentState {
    Disconnected,
    Gathering,
    Connecting,
    Connected,
    Completed,
    Failed,
}

impl TryFrom<sys::juice_state> for AgentState {
    type Error = ();

    fn try_from(value: sys::juice_state) -> Result<Self, Self::Error> {
        Ok(match value {
            sys::juice_state_JUICE_STATE_DISCONNECTED => AgentState::Disconnected,
            sys::juice_state_JUICE_STATE_GATHERING => AgentState::Gathering,
            sys::juice_state_JUICE_STATE_CONNECTING => AgentState::Connecting,
            sys::juice_state_JUICE_STATE_CONNECTED => AgentState::Connected,
            sys::juice_state_JUICE_STATE_COMPLETED => AgentState::Completed,
            sys::juice_state_JUICE_STATE_FAILED => AgentState::Failed,
            _ => return Err(()),
        })
    }
}

impl From<AgentState> for sys::juice_state {
    fn from(state: AgentState) -> Self {
        match state {
            AgentState::Disconnected => sys::juice_state_JUICE_STATE_DISCONNECTED,
            AgentState::Gathering => sys::juice_state_JUICE_STATE_GATHERING,
            AgentState::Connecting => sys::juice_state_JUICE_STATE_CONNECTING,
            AgentState::Connected => sys::juice_state_JUICE_STATE_CONNECTED,
            AgentState::Completed => sys::juice_state_JUICE_STATE_COMPLETED,
            AgentState::Failed => sys::juice_state_JUICE_STATE_FAILED,
        }
    }
}
