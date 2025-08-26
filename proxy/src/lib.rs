#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// pub mod ffi;
pub mod flow;
mod h3;
mod http;
pub mod interceptor;
mod peek_stream;
pub mod proxy;
mod ws;

use once_cell::sync::OnceCell;
use tracing_subscriber::EnvFilter;

static TEST_INIT_LOGGER: OnceCell<()> = OnceCell::new();

pub fn init_test_logging() {
    TEST_INIT_LOGGER.get_or_init(|| {
        tracing_subscriber::fmt()
            .without_time()
            .with_line_number(true)
            .with_env_filter(EnvFilter::from_default_env())
            .with_test_writer()
            .init();
    });
}
