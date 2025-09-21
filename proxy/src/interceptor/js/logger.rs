use boa_engine::{Context, JsResult};
use boa_gc::{Finalize, Trace};
use boa_runtime::{ConsoleState, Logger};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone, Trace, Finalize)]
pub(crate) struct JsLogger {}

impl Logger for JsLogger {
    fn debug(&self, msg: String, _state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        debug!("[js] {msg}");
        Ok(())
    }

    fn log(&self, msg: String, _state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        info!("[js] {msg}");
        Ok(())
    }

    fn info(&self, msg: String, _state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        info!("[js] {msg}");
        Ok(())
    }

    fn warn(&self, msg: String, _state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        warn!("[js] {msg}");
        Ok(())
    }

    fn error(&self, msg: String, _state: &ConsoleState, _context: &mut Context) -> JsResult<()> {
        error!("[js] {msg}");
        Ok(())
    }
}
