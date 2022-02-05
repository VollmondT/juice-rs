pub trait IceHander {
    fn on_state_changed(&mut self);

    fn on_candidate(&mut self);

    fn on_gathering_done(&mut self);

    fn on_recv(&mut self);
}
