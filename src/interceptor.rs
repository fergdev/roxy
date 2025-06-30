use anyhow::{Result, anyhow};
use mlua::{Function, Lua, Table};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::{debug, error};

use crate::flow::Flow;

#[derive(Debug, Clone)]
pub struct ScriptEngine {
    lua: Arc<RwLock<Lua>>,
    script_path: String,
}

impl ScriptEngine {
    pub fn new(script_path: impl Into<String>) -> anyhow::Result<Self> {
        let script_path = script_path.into();
        println!(
            "Initializing Lua script engine with script: {}",
            script_path
        );
        debug!(
            "Initializing Lua script engine with script: {}",
            script_path
        );
        let lua = Arc::new(RwLock::new(Lua::new()));
        let engine = Self {
            lua,
            script_path: script_path.clone(),
        };
        engine.load_script()?;
        engine.watch_script()?;
        Ok(engine)
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

    pub fn intercept_request(&self, flow: &mut Flow) -> Result<()> {
        let req = match &mut flow.request {
            Some(resp) => resp,
            None => panic!("No response to intercept"),
        };
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

        req_table
            .set("body", req.body.clone().unwrap_or_default())
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

        req.body =
            Some(new_req.get("body").map_err(|e| {
                anyhow!("Missing or invalid 'body' in Lua result: {}", e.to_string())
            })?);

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

    pub fn intercept_response(&self, flow: &mut Flow) -> Result<()> {
        let res = match &mut flow.response {
            Some(resp) => resp,
            None => panic!("No response to intercept"),
        };

        println!("Intercepting response: {}", res.request_line());
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

        res_table
            .set("body", res.body.clone().unwrap_or_default())
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

        // Update body
        res.body =
            Some(new_res.get("body").map_err(|e| {
                anyhow!("Missing or invalid 'body' in Lua result: {}", e.to_string())
            })?);

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
