use libjuice_rs::{Agent, Handler, Server, ServerCredentials};
use std::sync::mpsc::channel;

include!("../src/test_util.rs");

const USER: &str = "server_test";
const PASS: &str = "79874638521694";
const SERVER_ADDRESS: &str = "127.0.0.1:3478";

fn server_credentials() -> ServerCredentials {
    ServerCredentials::new(USER, PASS, None).unwrap()
}

fn run_server(server: Server) {
    let server_port = server.get_port();
    assert_eq!(server_port, 3478);

    let (gathering_tx, gathering_rx) = channel();

    let (first_tx, first_rx) = channel();
    let first_handler = Handler::default()
        .gathering_done_handler({
            let mut gathering_tx = Some(gathering_tx.clone());
            move || {
                log::info!("first agent finished gathering");
                // send only once
                if let Some(ch) = gathering_tx.take() {
                    ch.send(()).ok();
                }
            }
        })
        .state_handler(move |state| log::info!("first changed state to: {:?}", state))
        .candidate_handler(move |sdp| {
            log::debug!("first received candidate: {:?}", sdp);
            let _ = first_tx.send(sdp);
        });

    let first = Agent::builder(first_handler)
        .with_stun("127.0.0.1".into(), 3478)
        .add_turn_server("127.0.0.1", server_port, USER, PASS)
        .unwrap()
        .build()
        .unwrap();

    let (second_tx, second_rx) = channel();
    let second_handler = Handler::default()
        .gathering_done_handler({
            let mut gathering_tx = Some(gathering_tx);
            move || {
                log::info!("second agent finished gathering");
                // send only once
                if let Some(ch) = gathering_tx.take() {
                    ch.send(()).ok();
                }
            }
        })
        .state_handler(move |state| log::info!("second changed state to: {:?}", state))
        .candidate_handler(move |sdp| {
            log::debug!("second received candidate: {:?}", sdp);
            let _ = second_tx.send(sdp);
        });
    let second = Agent::builder(second_handler)
        .with_stun("127.0.0.1".into(), 3478)
        .add_turn_server("127.0.0.1", server_port, USER, PASS)
        .unwrap()
        .with_port_range(5000, 5010)
        .build()
        .unwrap();

    first.gather_candidates().unwrap();
    second.gather_candidates().unwrap();

    for _ in 0..2 {
        gathering_rx.recv().unwrap();
    }

    let has_relayed = loop {
        if let Ok(candidate) = first_rx.try_recv() {
            if candidate.contains("typ relay") {
                break true;
            }
        } else {
            break false;
        }
    };

    assert!(has_relayed);

    let has_relayed = loop {
        if let Ok(candidate) = second_rx.try_recv() {
            if candidate.contains("typ relay") {
                break true;
            }
        } else {
            break false;
        }
    };

    assert!(has_relayed);
}

#[test]
fn test_server() {
    logger_init();

    let server_address = SERVER_ADDRESS.parse().unwrap();
    let server = Server::builder()
        .bind_address(&server_address)
        .with_port_range(6000, 7000)
        .add_credentials(server_credentials())
        .with_realm("Juice test server")
        .unwrap()
        .build()
        .unwrap();

    run_server(server);
}
