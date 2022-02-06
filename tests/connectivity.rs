use std::sync::mpsc::channel;
use std::sync::{Arc, Barrier};
use std::thread::sleep;
use std::time::Duration;

use libjuice::agent_state::AgentState;
use libjuice::{Builder, Handler};

include!("../src/test_util.rs");

#[test]
fn connectivity_no_trickle() {
    logger_init();

    let gathering_barrier = Arc::new(Barrier::new(3));

    let (first_tx, first_rx) = channel::<Vec<u8>>();
    let first_handler = Handler::default()
        .gathering_finished_handler({
            let barrier = gathering_barrier.clone();
            move || {
                log::info!("first agent finished gathering");
                barrier.wait();
            }
        })
        .state_handler(move |state| log::info!("first changed state to: {:?}", state))
        .recv_handler(move |packet| {
            log::debug!("first received {:?}", packet);
            let _ = first_tx.send(packet.to_vec());
        });

    let first = Builder::new(first_handler.to_box()).build();

    let (second_tx, second_rx) = channel::<Vec<u8>>();
    let second_handler = Handler::default()
        .gathering_finished_handler({
            let barrier = gathering_barrier.clone();
            move || {
                log::info!("second agent finished gathering");
                barrier.wait();
            }
        })
        .state_handler(move |state| log::info!("second changed state to: {:?}", state))
        .recv_handler(move |packet| {
            log::debug!("second received {:?}", packet);
            let _ = second_tx.send(packet.to_vec());
        });
    let second = Builder::new(second_handler.to_box()).build();

    first.gather_candidates().unwrap();
    second.gather_candidates().unwrap();

    gathering_barrier.wait();

    let first_desc = first.get_local_description().unwrap();
    second.set_remote_description(first_desc).unwrap();

    let second_desc = second.get_local_description().unwrap();
    first.set_remote_description(second_desc).unwrap();

    sleep(Duration::from_secs(2));

    assert_eq!(first.state(), AgentState::Completed);
    assert_eq!(second.state(), AgentState::Completed);

    first.send("hello".as_bytes()).unwrap();
    assert_eq!(
        second_rx.recv_timeout(Duration::from_secs(1)),
        Ok("hello".into())
    );

    second.send("world".as_bytes()).unwrap();
    assert_eq!(
        first_rx.recv_timeout(Duration::from_secs(1)),
        Ok("world".into())
    );
}
