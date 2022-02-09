use libjuice_rs::{Server, ServerCredentials};

include!("../src/test_util.rs");

const USER: &str = "server_test";
const PASS: &str = "79874638521694";

fn server_credentials() -> ServerCredentials {
    ServerCredentials::new(USER, PASS, None).unwrap()
}

#[test]
fn test_server() {
    logger_init();

    let server_address = "127.0.0.1:3478".parse().unwrap();
    let external_address = "192.168.1.1".parse().unwrap();
    let server = Server::builder()
        .bind_address(&server_address)
        .with_external_address(&external_address)
        .with_port_range(6000, 7000)
        .add_credentials(server_credentials())
        .with_realm("Juice test server")
        .unwrap()
        .build()
        .unwrap();

    let server_port = server.get_port();
    assert_ne!(server_port, 0);
}
