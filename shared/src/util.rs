use std::fmt::Write;
use tracing::error;

pub fn report(mut err: &dyn (std::error::Error)) -> String {
    let mut s = format!("{err}");
    while let Some(src) = err.source() {
        let _ = write!(s, "\n\nCaused by: {src}");
        err = src;
    }
    error!("\n\nreport {}\n\n", s);
    s
}
