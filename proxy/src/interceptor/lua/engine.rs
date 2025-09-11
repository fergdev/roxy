use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use mlua::{Function, Lua, Table, Value, Variadic};
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

use crate::{
    flow::{InterceptedRequest, InterceptedResponse},
    interceptor::{
        Error, FlowNotify, KEY_EXTENSIONS, KEY_INTERCEPT_REQUEST, KEY_INTERCEPT_RESPONSE,
        RoxyEngine,
        lua::{
            body::register_body,
            flow::{LuaFlow, register_flow},
            headers::register_headers,
            query::register_query,
            request::{LuaRequest, register_request},
            response::{LuaResponse, register_response},
            url::register_url,
        },
    },
};

const ROXY: &str = "Roxy";
const NOTIFY: &str = "notify";
const PRINT: &str = "print";

#[derive(Debug)]
pub struct LuaEngine {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Debug)]
struct Inner {
    lua: Option<Lua>,
    notify_tx: Option<mpsc::Sender<FlowNotify>>,
}

#[async_trait]
impl RoxyEngine for LuaEngine {
    async fn set_script(&self, script: &str) -> Result<(), Error> {
        let mut guard = self.inner.lock().map_err(|_| Error::InterceptedRequest)?;
        guard.set_script(script)
    }

    async fn intercept_request(
        &self,
        req: &mut InterceptedRequest,
    ) -> Result<Option<InterceptedResponse>, Error> {
        trace!("intercept_request");
        let guard = self.inner.lock().map_err(|_| Error::InterceptedRequest)?;
        if let Some(lua) = &guard.lua {
            trace!("doing intercept_request");
            intercept_request_inner(lua, req).map_err(|e| {
                error!("ScriptEngine intercept error {}", e);
                e
            })
        } else {
            Ok(None)
        }
    }

    async fn intercept_response(
        &self,
        req: &InterceptedRequest,
        res: &mut InterceptedResponse,
    ) -> Result<(), Error> {
        trace!("intercept_response");
        let guard = self.inner.lock().map_err(|_| Error::InterceptedRequest)?;
        if let Some(lua) = &guard.lua {
            trace!("intercept_response rewrite");
            intercept_response_inner(lua, req, res).map_err(|e| {
                error!("ScriptEngine intercept_response {}", e);
                e
            })?
        } else {
            error!("no lua");
        }
        Ok(())
    }
}

impl Inner {
    fn set_script(&mut self, script: &str) -> Result<(), Error> {
        trace!("Set script {script}");
        let lua = Lua::new();
        register_functions(&lua, self.notify_tx.clone())?;
        lua.load(script).exec()?;
        self.lua = Some(lua);
        trace!("Loaded script");
        Ok(())
    }
}

impl LuaEngine {
    pub fn new(notify_tx: Option<mpsc::Sender<FlowNotify>>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                lua: None,
                notify_tx,
            })),
        }
    }
}
fn intercept_request_inner(
    lua: &Lua,
    req: &mut InterceptedRequest,
) -> Result<Option<InterceptedResponse>, Error> {
    trace!("intercept_request_inner");

    let extensions: Table = lua.globals().get(KEY_EXTENSIONS)?;
    if extensions.is_empty() {
        return Ok(None);
    }

    let req_arc = Arc::new(Mutex::new(std::mem::take(req)));

    let resp_inner = InterceptedResponse::default();
    let resp_arc = Arc::new(Mutex::new(resp_inner));

    let lua_req = LuaRequest::from_parts(req_arc.clone())?;
    let lua_resp = LuaResponse::from_parts(resp_arc.clone())?;
    let flow_ud = lua.create_userdata(LuaFlow::from_views(lua_req.clone(), lua_resp.clone()))?;

    let mut handlers: Vec<Function> = Vec::new();
    for pair in extensions.pairs::<Value, Table>() {
        let (_, ext) = match pair {
            Ok(x) => x,
            Err(_) => continue,
        };
        if let Ok(f) = ext
            .get::<Function>(KEY_INTERCEPT_REQUEST)
            .or_else(|_| ext.get::<Function>(1))
        {
            handlers.push(f);
        }
    }

    for f in handlers {
        if let Err(e) = f.call::<()>(flow_ud.clone()) {
            error!("Error invoking request handler: {e}");
        }
        if response_ready(
            &*resp_arc
                .lock()
                .map_err(|_| Error::Other("resp lock poisoned".into()))?,
        ) {
            break;
        }
    }
    {
        let guard = req_arc
            .lock()
            .map_err(|e| Error::Other(format!("lock: {e}")))?;
        *req = guard.clone();

        req.headers = lua_req
            .headers
            .map
            .lock()
            .map_err(|_| Error::Other("resp lock poisoned".into()))?
            .clone();

        req.uri = lua_req.uri.to_ruri()?;
        req.body = lua_req
            .body
            .inner
            .lock()
            .map_err(|_| Error::Other("resp lock poisoned".into()))?
            .clone();

        let trailers = lua_req
            .trailers
            .map
            .lock()
            .map_err(|_| Error::Other("req lock poisoned".into()))?;
        if trailers.is_empty() {
            req.trailers = None;
        } else {
            req.trailers = Some(trailers.clone());
        }
    }

    info!("Updating response from Lua");
    let updated_resp = lua_resp
        .get_inner()
        .map_err(|_| Error::Other("req lock poisoned".into()))?;

    if response_ready(&updated_resp) {
        Ok(Some(updated_resp))
    } else {
        Ok(None)
    }
}

fn response_ready(r: &InterceptedResponse) -> bool {
    r.status != 200 || !r.body.is_empty()
}

pub fn intercept_response_inner(
    lua: &Lua,
    req: &InterceptedRequest,
    res: &mut InterceptedResponse,
) -> Result<(), Error> {
    let extensions: Table = lua
        .globals()
        .get(KEY_EXTENSIONS)
        .map_err(|e| Error::Other(format!("missing Extensions: {e}")))?;

    if extensions.is_empty() {
        return Ok(());
    }

    let mut handlers: Vec<Function> = Vec::new();
    for pair in extensions.sequence_values::<Table>() {
        let ext = match pair {
            Ok(t) => t,
            Err(_) => continue,
        };
        if let Ok(f) = ext.get::<Function>(KEY_INTERCEPT_RESPONSE) {
            handlers.push(f);
        } else if let Ok(f) = ext.get::<Function>(2) {
            handlers.push(f);
        }
    }
    if handlers.is_empty() {
        return Ok(());
    }

    let res_arc = Arc::new(Mutex::new(res.clone()));
    let lua_req = LuaRequest::from_parts(Arc::new(Mutex::new(req.clone())))?; // TODO: not clone
    let lua_resp = LuaResponse::from_parts(res_arc.clone())
        .map_err(|e| Error::Other(format!("LuaResponse::from_parts: {e}")))?;

    let flow_ud = lua
        .create_userdata(LuaFlow::from_views(lua_req, lua_resp.clone()))
        .map_err(|e| Error::Other(format!("create flow userdata: {e}")))?;

    for h in handlers {
        h.call::<()>(flow_ud.clone())
            .map_err(|e| Error::Other(format!("response handler error: {e}")))?;
    }

    {
        let guard = res_arc
            .lock()
            .map_err(|e| Error::Other(format!("lock poisoned: {e}")))?;
        *res = guard.clone();
        res.body = lua_resp
            .body
            .inner
            .lock()
            .map_err(|_| Error::Other("resp lock poisoned".into()))?
            .clone();
        res.headers = lua_resp
            .headers
            .map
            .lock()
            .map_err(|_| Error::Other("resp lock poisoned".into()))?
            .clone();
        let trailers = lua_resp
            .trailers
            .map
            .lock()
            .map_err(|_| Error::Other("req lock poisoned".into()))?;
        info!("trailers {:?}", trailers);
        if trailers.is_empty() {
            res.trailers = None;
        } else {
            res.trailers = Some(trailers.clone());
        }
    }

    Ok(())
}

fn register_functions(
    lua: &Lua,
    notify: Option<mpsc::Sender<FlowNotify>>,
) -> Result<(), mlua::Error> {
    let globals = lua.globals();

    let lua_notify = if let Some(notify) = notify {
        lua.create_function(move |_, (level, msg): (i32, String)| {
            if let Err(e) = notify.try_send(FlowNotify {
                level: level.into(),
                msg,
            }) {
                error!("Notify error {e}");
            }
            Ok(())
        })?
    } else {
        lua.create_function(move |_, (level, msg): (i32, String)| {
            match level {
                0 => trace!("{}", msg),
                1 => debug!("{}", msg),
                2 => info!("{}", msg),
                3 => warn!("{}", msg),
                4 => error!("{}", msg),
                _ => {
                    // Off
                }
            };
            Ok(())
        })?
    };

    let print = lua.create_function(move |_, (msg, level): (String, Option<i32>)| {
        match level {
            Some(0) => trace!("{}", msg),
            Some(1) => debug!("{}", msg),
            Some(2) => info!("{}", msg),
            Some(3) => warn!("{}", msg),
            Some(4) => error!("{}", msg),
            _ => {
                // Off
            }
        };
        Ok(())
    })?;

    globals.set(KEY_EXTENSIONS, lua.create_table()?)?;
    globals.set(
        ROXY,
        lua.create_table_from([(NOTIFY, lua_notify), (PRINT, print)])?,
    )?;

    let print_fn = lua.create_function(|_, args: Variadic<Value>| {
        let output: Vec<String> = args.iter().map(|v| format!("{v:?}")).collect();
        info!("{}", output.join("\t"));
        Ok(())
    })?;

    lua.globals().set(PRINT, print_fn)?;
    register_flow(lua)?;
    register_headers(lua)?;
    register_response(lua)?;
    register_request(lua)?;
    register_body(lua)?;
    register_url(lua)?;
    register_query(lua)?;

    Ok(())
}
