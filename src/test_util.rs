/// Init logger for tests
pub fn logger_init() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Trace)
        .format_timestamp_micros()
        .try_init();
}
