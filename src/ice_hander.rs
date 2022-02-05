use crate::agent_state::AgentState;

pub trait IceHander {
    fn on_state_changed(&mut self, state: AgentState);

    fn on_candidate(&mut self, candidate: String);

    fn on_gathering_done(&mut self);

    fn on_recv(&mut self);
}
