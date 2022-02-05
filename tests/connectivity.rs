use std::sync::{Arc, Barrier};

use libjuice::{Builder, Handler};

include!("../src/test_util.rs");

#[test]
fn connectivity_no_trickle() {
    logger_init();

    let barrier = Arc::new(Barrier::new(3));

    let first_handler = Handler::default().gathering_finished_handler({
        let barrier = barrier.clone();
        move || {
            log::debug!("first agent finished gathering");
            barrier.wait();
        }
    });
    let mut first = Builder::new(first_handler.to_box()).build();

    let second_handler = Handler::default().gathering_finished_handler({
        let barrier = barrier.clone();
        move || {
            log::debug!("second agent finished gathering");
            barrier.wait();
        }
    });
    let mut second = Builder::new(second_handler.to_box()).build();

    first.gather_candidates().unwrap();
    second.gather_candidates().unwrap();

    barrier.wait();

    let _first_desc = first.local_description().unwrap();
    let _second_desc = second.local_description().unwrap();

    todo!()
}
