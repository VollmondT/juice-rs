use libjuice_rs::Server;

include!("../src/test_util.rs");

#[test]
fn test_server() {
    logger_init();
    let server = Server::builder().build().unwrap();
    assert_ne!(server.get_port(), 0);
}
