#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {}

/// Starts log environment using the given verbosity arguments
pub fn startup_log() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .format_target(true)
        .format_timestamp(None)
        .init();
}
