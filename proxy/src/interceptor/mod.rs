use std::fmt::Display;

use async_trait::async_trait;
use strum::EnumIter;

use crate::{
    flow::{InterceptedRequest, InterceptedResponse},
    interceptor::{js::engine::JsEngine, lua::engine::LuaEngine, py::engine::PythonEngine},
};

mod js;
mod lua;
mod py;

use std::{fmt::Debug, sync::Arc};
use tokio::sync::{
    Mutex,
    mpsc::{self},
};
use tracing::{trace, warn};

const KEY_EXTENSIONS: &str = "Extensions";

const KEY_INTERCEPT_REQUEST: &str = "request";
const KEY_INTERCEPT_RESPONSE: &str = "response";

const KEY_REQUEST: &str = "request";
const KEY_RESPONSE: &str = "response";

const KEY_URL: &str = "url";
const KEY_METHOD: &str = "method";
const KEY_VERSION: &str = "version";

const KEY_SCHEME: &str = "scheme";
const KEY_HOST: &str = "host";
const KEY_PORT: &str = "port";
const KEY_PATH: &str = "path";
const KEY_USERNAME: &str = "username";
const KEY_PASSWORD: &str = "password";

const KEY_HEADERS: &str = "headers";
const KEY_BODY: &str = "body";
const KEY_TRAILERS: &str = "trailers";

const KEY_STATUS: &str = "status";

#[async_trait]
pub trait RoxyEngine: Send + Sync {
    async fn intercept_request(
        &self,
        req: &mut InterceptedRequest,
    ) -> Result<Option<InterceptedResponse>, Error>;

    async fn intercept_response(&self, res: &mut InterceptedResponse) -> Result<(), Error>;

    async fn set_script(&self, script: &str) -> Result<(), Error>;
}

struct NoopEngine {}

#[async_trait]
impl RoxyEngine for NoopEngine {
    async fn intercept_request(
        &self,
        _req: &mut InterceptedRequest,
    ) -> Result<Option<InterceptedResponse>, Error> {
        warn!("Noop intercept_request");
        Ok(None)
    }

    async fn intercept_response(&self, _res: &mut InterceptedResponse) -> Result<(), Error> {
        warn!("Noop intercept_response");
        Ok(())
    }

    async fn set_script(&self, _script: &str) -> Result<(), Error> {
        warn!("Noop set script");
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct FlowNotify {
    pub level: i32, // TODO: should be an enum
    pub msg: String,
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Lua(mlua::Error),
    LoadError,
    InterceptResponse,
    InterceptedRequest,
    Other(String),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<mlua::Error> for Error {
    fn from(value: mlua::Error) -> Self {
        Error::Lua(value)
    }
}

impl From<pyo3::PyErr> for Error {
    fn from(value: pyo3::PyErr) -> Self {
        Error::Other(format!("pyo3 error: {value}"))
    }
}

#[derive(Debug, Clone, Copy, EnumIter)]
pub enum ScriptType {
    Lua,
    Js,
    Python,
}

impl ScriptType {
    pub fn ext(&self) -> &str {
        match self {
            ScriptType::Lua => "lua",
            ScriptType::Js => "js",
            ScriptType::Python => "py",
        }
    }
}

impl Display for ScriptType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ext())
    }
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone)]
pub struct ScriptEngine {
    notify_tx: Option<mpsc::Sender<FlowNotify>>,
    inner: Arc<Mutex<Box<dyn RoxyEngine>>>,
}

impl Debug for ScriptEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptEngine").finish()
    }
}

impl ScriptEngine {
    pub fn new() -> Self {
        ScriptEngine::new_inner(None)
    }

    pub fn new_notify(notify_tx: mpsc::Sender<FlowNotify>) -> Self {
        ScriptEngine::new_inner(Some(notify_tx))
    }

    fn new_inner(notify_tx: Option<mpsc::Sender<FlowNotify>>) -> Self {
        Self {
            notify_tx,
            inner: Arc::new(Mutex::new(Box::new(NoopEngine {}))),
        }
    }

    pub async fn intercept_request(
        &self,
        req: &mut InterceptedRequest,
    ) -> Result<Option<InterceptedResponse>, Error> {
        trace!("intercept_request");
        let guard = self.inner.lock().await;
        guard.intercept_request(req).await
    }

    pub async fn intercept_response(&self, res: &mut InterceptedResponse) -> Result<(), Error> {
        trace!("intercept_response");
        let guard = self.inner.lock().await;
        guard.intercept_response(res).await
    }

    pub async fn set_script(&mut self, script: &str, script_type: ScriptType) -> Result<(), Error> {
        let engine: Box<dyn RoxyEngine> = match script_type {
            ScriptType::Lua => Box::new(LuaEngine::new(self.notify_tx.clone())),
            ScriptType::Js => Box::new(JsEngine::new(self.notify_tx.clone())),
            ScriptType::Python => Box::new(PythonEngine::new(self.notify_tx.clone())),
        };
        engine.set_script(script).await?;
        let mut guard = self.inner.lock().await;
        *guard = engine;
        Ok(())
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}
