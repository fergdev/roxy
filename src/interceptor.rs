use anyhow::anyhow;
use bytes::Bytes;
use mlua::{Function, Lua, Table, Value, Variadic};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::{debug, error, info, trace, warn};

use crate::flow::{Flow, InterceptedRequest, InterceptedResponse};
use crate::{notify_debug, notify_error, notify_info, notify_trace, notify_warn};

#[derive(Debug, Clone)]
pub struct ScriptEngine {
    lua: Arc<RwLock<Lua>>,
    script_path: String,
}

impl ScriptEngine {
    pub fn new(script_path: impl Into<String>) -> anyhow::Result<Self> {
        let script_path = script_path.into();
        debug!(
            "Initializing Lua script engine with script: {}",
            script_path
        );
        let lua = Lua::new();

        ScriptEngine::register_functions(&lua)?;
        let engine = Self {
            lua: Arc::new(RwLock::new(lua)),
            script_path: script_path.clone(),
        };
        engine.load_script()?;
        engine.watch_script()?;
        Ok(engine)
    }

    pub fn register_functions(lua: &Lua) -> anyhow::Result<()> {
        let globals = lua.globals();

        let notify = lua.create_function(move |_, (msg, level): (String, Option<i32>)| {
            match level {
                Some(0) => notify_trace!("{}", msg),
                Some(1) => notify_debug!("{}", msg),
                Some(2) => notify_info!("{}", msg),
                Some(3) => notify_warn!("{}", msg),
                Some(4) => notify_error!("{}", msg),
                _ => {
                    // Off
                }
            };
            Ok(())
        })?;

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

        globals.set(
            "roxy",
            lua.create_table_from([("notify", notify), ("print", print)])?,
        )?;

        let print_fn = lua.create_function(|_, args: Variadic<Value>| {
            let output: Vec<String> = args.iter().map(|v| format!("{:?}", v)).collect();
            info!("{}", output.join("\t"));
            Ok(())
        })?;

        lua.globals().set("print", print_fn)?;

        Ok(())
    }

    fn load_script(&self) -> anyhow::Result<()> {
        let script_code = fs::read_to_string(&self.script_path)?;
        let lua = self.lua.write().unwrap();
        lua.load(&script_code)
            .exec()
            .map_err(|e| anyhow!("Failed to execute Lua script: {}", e.to_string()))?;
        debug!("Load script: {}", self.script_path);
        Ok(())
    }

    fn watch_script(&self) -> anyhow::Result<()> {
        let path = Path::new(&self.script_path).to_owned();
        let engine = self.clone();
        std::thread::spawn(move || {
            let (tx, rx) = channel();
            let mut watcher: RecommendedWatcher = Watcher::new(tx, Config::default()).unwrap();
            watcher.watch(&path, RecursiveMode::NonRecursive).unwrap();

            loop {
                match rx.recv_timeout(Duration::from_secs(2)) {
                    Ok(_) => {
                        if let Err(e) = engine.load_script() {
                            error!("Failed to reload script: {e}");
                        }
                    }
                    Err(_) => continue,
                }
            }
        });
        Ok(())
    }

    pub fn intercept_request(&self, req: &mut InterceptedRequest) -> anyhow::Result<()> {
        debug!("Intercepting request: {}", req.request_line());
        let lua = self
            .lua
            .read()
            .map_err(|e| anyhow!("Failed to acquire Lua lock: {e}"))?;

        let intercept_fn: Function = lua.globals().get("intercept_request").map_err(|e| {
            anyhow!(
                "Failed to get Lua function 'intercept_request': {}",
                e.to_string()
            )
        })?;

        let req_table = lua
            .create_table()
            .map_err(|e| anyhow!("Failed to create Lua table: {}", e.to_string()))?;

        req_table
            .set("host", req.host.clone())
            .map_err(|e| anyhow!("Failed to set 'host': {}", e.to_string()))?;

        req_table
            .set("port", req.port)
            .map_err(|e| anyhow!("Failed to set 'host': {}", e.to_string()))?;

        // TODO: figure how to do this properly
        let body = String::from_utf8_lossy(&req.body).to_string();
        req_table
            .set("body", body)
            .map_err(|e| anyhow!("Failed to set 'body': {}", e.to_string()))?;

        let headers_table = lua
            .create_table()
            .map_err(|e| anyhow!("Failed to create headers table: {}", e.to_string()))?;

        for (k, v) in &req.headers {
            headers_table
                .set(k.clone(), v.clone())
                .map_err(|e| anyhow!("Failed to set header '{k}': {}", e.to_string()))?;
        }

        req_table
            .set("headers", headers_table)
            .map_err(|e| anyhow!("Failed to attach headers: {}", e.to_string()))?;

        let new_req: Table = intercept_fn
            .call(req_table)
            .map_err(|e| anyhow!("Lua call to 'intercept_request' failed: {}", e.to_string()))?;

        req.host = new_req
            .get("host")
            .map_err(|e| anyhow!("Missing or invalid 'host' in Lua result: {}", e.to_string()))?;

        req.port = new_req
            .get("port")
            .map_err(|e| anyhow!("Missing or invalid 'port' in Lua result: {}", e.to_string()))?;

        let new_body = new_req
            .get("body")
            .map_err(|e| anyhow!("Missing or invalid 'body' in Lua result: {}", e.to_string()))?;
        match new_body {
            Value::String(s) => {
                req.body = s
                    .to_str()
                    .map(|s| Bytes::copy_from_slice(s.as_bytes()))
                    .unwrap_or_else(|_| Bytes::new());
            }
            Value::Nil => {
                req.body.clear();
            }
            _ => {
                return Err(anyhow!(
                    "Invalid 'body' type in Lua result: expected string or nil, got {:?}",
                    new_body
                ));
            }
        }

        let new_headers: Table = new_req.get("headers").map_err(|e| {
            anyhow!(
                "Missing or invalid 'headers' in Lua result: {}",
                e.to_string()
            )
        })?;

        req.headers.clear();
        for pair in new_headers.pairs::<String, String>() {
            let (k, v) =
                pair.map_err(|e| anyhow!("Invalid header entry in Lua result: {}", e.to_string()))?;
            req.headers.insert(k, v);
        }

        Ok(())
    }

    pub fn intercept_response(&self, res: &mut InterceptedResponse) -> anyhow::Result<()> {
        let lua = self
            .lua
            .read()
            .map_err(|e| anyhow!("Failed to acquire Lua lock: {e}"))?;

        let intercept_fn: Function = lua.globals().get("intercept_response").map_err(|e| {
            anyhow!(
                "Failed to get Lua function 'intercept_response': {}",
                e.to_string()
            )
        })?;

        let res_table = lua
            .create_table()
            .map_err(|e| anyhow!("Failed to create Lua table: {}", e.to_string()))?;

        let body = String::from_utf8_lossy(&res.body).to_string();
        res_table
            .set("body", body)
            .map_err(|e| anyhow!("Failed to set 'body' in Lua table: {}", e.to_string()))?;

        let headers_table = lua
            .create_table()
            .map_err(|e| anyhow!("Failed to create Lua headers table: {}", e.to_string()))?;

        for (k, v) in &res.headers {
            headers_table
                .set(k.clone(), v.clone())
                .map_err(|e| anyhow!("Failed to set header '{k}': {}", e.to_string()))?;
        }

        res_table
            .set("headers", headers_table)
            .map_err(|e| anyhow!("Failed to attach headers to Lua table: {}", e.to_string()))?;

        let new_res: Table = intercept_fn
            .call(res_table)
            .map_err(|e| anyhow!("Lua call to 'intercept_response' failed: {}", e.to_string()))?;

        let new_body = new_res
            .get("body")
            .map_err(|e| anyhow!("Missing or invalid 'body' in Lua result: {}", e.to_string()))?;

        match new_body {
            Value::String(s) => {
                res.body = s
                    .to_str()
                    .map(|s| Bytes::copy_from_slice(s.as_bytes()))
                    .unwrap_or_else(|_| Bytes::new());
            }
            Value::Nil => {
                res.body.clear();
            }
            _ => {
                return Err(anyhow!(
                    "Invalid 'body' type in Lua result: expected string or nil, got {:?}",
                    new_body
                ));
            }
        }

        // Update headers
        let new_headers: Table = new_res.get("headers").map_err(|e| {
            anyhow!(
                "Missing or invalid 'headers' in Lua result: {}",
                e.to_string()
            )
        })?;

        res.headers.clear();
        for pair in new_headers.pairs::<String, String>() {
            let (k, v) =
                pair.map_err(|e| anyhow!("Invalid header entry in Lua result: {}", e.to_string()))?;
            res.headers.insert(k, v);
        }

        Ok(())
    }
}
