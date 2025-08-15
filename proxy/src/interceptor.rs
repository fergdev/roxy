use async_watcher::{
    AsyncDebouncer,
    notify::{FsEventWatcher, RecursiveMode},
};
use bytes::Bytes;
use chrono::Utc;
use http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Version};
use mlua::{Function, Lua, Table, Value, Variadic};
use roxy_shared::{
    alpn::AlpnProtocol,
    uri::{RUri, Scheme},
};
use std::{
    fmt::{Debug, Display},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    sync::mpsc::{self},
    task::JoinHandle,
};
use tracing::{debug, error, info, trace, warn};
use url::{Url, form_urlencoded};

use crate::flow::{InterceptedRequest, InterceptedResponse};

#[derive(Clone)]
pub struct ScriptEngine {
    inner: Arc<Mutex<Inner>>,
    debouncer: Option<Arc<Mutex<AsyncDebouncer<FsEventWatcher>>>>,
    watcher: Option<Arc<JoinHandle<()>>>,
    script_path: Arc<Mutex<Option<PathBuf>>>,
}

impl Debug for ScriptEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptEngine")
            .field("inner", &self.inner)
            .field("watcher", &self.watcher)
            .field("script_path", &self.script_path)
            .finish()
    }
}

impl Drop for ScriptEngine {
    fn drop(&mut self) {
        if let Some(watcher) = &self.watcher {
            watcher.abort();
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct FlowNotify {
    pub level: i32, // TODO: should be an enum
    pub msg: String,
}

#[derive(Debug)]
struct Inner {
    lua: Option<Lua>,
    notify_tx: Option<mpsc::Sender<FlowNotify>>,
}

const ROXY: &str = "Roxy";
const NOTIFY: &str = "notify";
const PRINT: &str = "print";

impl ScriptEngine {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        ScriptEngine::new_inner(None).await
    }

    pub async fn new_notify(
        notify_tx: mpsc::Sender<FlowNotify>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        ScriptEngine::new_inner(Some(notify_tx)).await
    }

    async fn new_inner(
        notify_tx: Option<mpsc::Sender<FlowNotify>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (debouncer, mut file_events) =
            AsyncDebouncer::new_with_channel(Duration::from_millis(10), None).await?;

        let mut out = Self {
            debouncer: Some(Arc::new(Mutex::new(debouncer))),
            watcher: None,
            script_path: Arc::new(Mutex::new(None)),
            inner: Arc::new(Mutex::new(Inner {
                lua: None,
                notify_tx,
            })),
        };

        let mut t_out = out.clone();
        let handle = tokio::spawn(async move {
            while let Some(Ok(events)) = file_events.recv().await {
                info!("Events {}", events.len());
                for _ in events {
                    if let Err(err) = t_out.reload_script().await {
                        error!("reload err {err}");
                    }
                }
            }
        });

        out.watcher = Some(Arc::new(handle));

        Ok(out)
    }

    pub async fn intercept_request(
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

    pub async fn intercept_response(&self, res: &mut InterceptedResponse) -> Result<(), Error> {
        trace!("intercept_response");
        let guard = self.inner.lock().map_err(|_| Error::InterceptedRequest)?;
        if let Some(lua) = &guard.lua {
            trace!("intercept_response rewrite");
            intercept_response_inner(lua, res).map_err(|e| {
                error!("ScriptEngine intercept_response {}", e);
                e
            })?
        }
        Ok(())
    }

    async fn reload_script(&mut self) -> Result<(), Error> {
        let path = {
            if let Ok(p) = self.script_path.lock() {
                (*p).clone()
            } else {
                None
            }
        };

        if let Some(path) = path {
            let script = tokio::fs::read_to_string(&path).await?;
            let mut guard = self.inner.lock().map_err(|_| Error::InterceptedRequest)?;
            if let Some(n) = &guard.notify_tx {
                if let Err(err) = n.try_send(FlowNotify {
                    level: 0,
                    msg: "Udpate".to_string(),
                }) {
                    error!("Error sending notification {err}");
                }
            }
            guard.set_script(&script)
        } else {
            error!("no path");
            Ok(())
        }
    }

    pub async fn set_script(&mut self, script: &str) -> Result<(), Error> {
        self.unwatch_curr();

        {
            if let Ok(mut g) = self.script_path.lock() {
                g.take();
            }
        }

        let mut guard = self.inner.lock().map_err(|_| Error::InterceptedRequest)?;
        guard.set_script(script)
    }

    fn unwatch_curr(&self) {
        let path = {
            if let Ok(p) = self.script_path.lock() {
                (*p).clone()
            } else {
                None
            }
        };
        if let Some(p) = path {
            if let Some(debouncer) = &self.debouncer {
                if let Ok(mut guard) = debouncer.lock() {
                    if let Err(err) = guard.watcher().unwatch(&p) {
                        error!("Failed to stop watcher {err}");
                    }
                }
            }
        }
    }

    pub async fn load_script_path(&mut self, path: PathBuf) -> Result<(), Error> {
        if let Some(debouncer) = &self.debouncer {
            if let Ok(mut guard) = debouncer.lock() {
                {
                    if let Ok(mut p) = self.script_path.lock() {
                        if let Some(p) = p.as_ref() {
                            info!("Watching {:?}", path);
                            if let Err(err) = guard.watcher().unwatch(p) {
                                error!("Error unwatching {err}");
                            }
                        }
                        let _ = p.insert(path.clone());
                    }
                }
                info!("watching");
                if let Err(err) = guard
                    .watcher()
                    .watch(path.as_path(), RecursiveMode::NonRecursive)
                {
                    error!("Watch err {err}");
                }
            } else {
                error!("lock failed");
            }
        }

        let script = tokio::fs::read_to_string(&path).await?;
        let mut guard = self.inner.lock().map_err(|_| Error::InterceptedRequest)?;
        guard.set_script(&script)
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

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Lua(mlua::Error),
    LoadError,
    InterceptResponse,
    InterceptedRequest,
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

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

const EXTENSIONS: &str = "Extensions";

const INTERCEPT_REQUEST: &str = "intercept_request";
const INTERCEPT_RESPONSE: &str = "intercept_response";

const REQUEST: &str = "request";
const RESPONSE: &str = "response";

const METHOD: &str = "method";
const VERSION: &str = "version";

const SCHEME: &str = "scheme";
const QUERY: &str = "query";
const HOST: &str = "host";
const PORT: &str = "port";
const PATH: &str = "path";

const HEADERS: &str = "headers";
const BODY: &str = "body";
const TRAILERS: &str = "trailers";

const STATUS: &str = "status";

fn parse_version(version: &str) -> Option<Version> {
    match version {
        "HTTP/0.9" => Some(Version::HTTP_09),
        "HTTP/1.0" => Some(Version::HTTP_10),
        "HTTP/1.1" => Some(Version::HTTP_11),
        "HTTP/2.0" => Some(Version::HTTP_2),
        "HTTP/3.0" => Some(Version::HTTP_3),
        _ => None,
    }
}

fn intercept_request_inner(
    lua: &Lua,
    req: &mut InterceptedRequest,
) -> Result<Option<InterceptedResponse>, Error> {
    trace!("intercept_request_inner");
    let extensions = lua.globals().get::<Table>(EXTENSIONS)?;

    if extensions.is_empty() {
        return Ok(None);
    }

    let mut fns = vec![];
    for pairs in extensions.pairs::<Value, Table>() {
        let Ok((_, ext)) = pairs else { continue };
        if let Ok(inter) = ext
            .get::<Function>(INTERCEPT_REQUEST)
            .or(ext.get::<Function>(1))
        {
            fns.push(inter);
        }
    }

    if fns.is_empty() {
        return Ok(None);
    }

    let request_table = gen_request_table(lua, req)?;
    let response_table = lua.create_table()?;

    let flow_table = lua.create_table()?;
    flow_table.set(REQUEST, &request_table)?;
    flow_table.set(RESPONSE, &response_table)?;
    for f in fns {
        f.call::<Value>(&flow_table)?;
    }

    update_request(request_table, req)?;
    if response_table.is_empty() {
        Ok(None)
    } else {
        Ok(Some(gen_response_from_intercept_request(
            response_table,
            req,
        )))
    }
}

fn gen_response_from_intercept_request(
    response_table: Table,
    intercepted_request: &InterceptedRequest,
) -> InterceptedResponse {
    let new_status: u16 = response_table.get(STATUS).unwrap_or(200);
    let new_status = StatusCode::from_u16(new_status).unwrap_or_default();

    let new_version: Version = response_table
        .get::<String>(VERSION)
        .map(|v| parse_version(&v).unwrap_or(intercepted_request.version))
        .unwrap_or(intercepted_request.version);

    let new_headers: HeaderMap = response_table
        .get::<Table>(HEADERS)
        .map(table_to_header_map)
        .unwrap_or_default();

    let new_body = response_table.get(BODY).map(parse_body).unwrap_or_default();

    let new_trailers: Option<HeaderMap> = response_table
        .get::<Table>(TRAILERS)
        .map(table_to_header_map_opt)
        .unwrap_or(None);

    InterceptedResponse {
        timestamp: Utc::now(),
        encoding: None,
        status: new_status,
        version: new_version,
        body: new_body,
        headers: new_headers,
        trailers: new_trailers,
    }
}

fn gen_request_table(lua: &Lua, req: &mut InterceptedRequest) -> Result<Table, Error> {
    trace!("gen request table");

    let url = Url::parse(&req.uri.inner.to_string()).map_err(|_| Error::InterceptedRequest)?;

    let request_table = lua.create_table()?;

    request_table.set(METHOD, req.method.to_string())?;
    request_table.set(VERSION, format!("{:?}", req.version))?;

    request_table.set(SCHEME, req.uri.scheme_str().unwrap_or(""))?;
    request_table.set(HOST, req.uri.host())?;
    request_table.set(PORT, req.uri.port())?;
    request_table.set(PATH, req.uri.path())?;
    request_table.set(QUERY, query_to_table(lua, url)?)?;
    request_table.set(BODY, String::from_utf8_lossy(&req.body).to_string())?;

    request_table.set(HEADERS, create_table(lua, &req.headers)?)?;
    request_table.set(TRAILERS, create_table_opt(lua, &req.trailers)?)?;

    Ok(request_table)
}

fn update_request(request_table: Table, req: &mut InterceptedRequest) -> Result<(), Error> {
    trace!("Update request");
    let new_scheme = request_table
        .get::<String>(SCHEME)
        .map(|s| Scheme::parse(s.as_str()))
        .unwrap_or(None);

    if let Some(Scheme::Http) = new_scheme {
        req.alpn = AlpnProtocol::None
    }

    let new_method: String = request_table.get(METHOD)?;
    let new_method: Method = new_method
        .as_str()
        .try_into()
        .map_err(|_| Error::InterceptedRequest)?;
    let new_version: String = request_table.get(VERSION)?;
    let new_version = parse_version(&new_version).unwrap_or(req.version);

    let new_host: String = request_table.get(HOST)?;
    let new_port: u16 = request_table.get(PORT)?;
    let new_path: String = request_table.get(PATH)?;
    let new_query = encode_query_table(request_table.get::<Table>(QUERY)?)?;

    let new_headers: Table = request_table.get(HEADERS)?;
    let new_body = request_table.get(BODY)?;
    let new_trailers: Table = request_table.get(TRAILERS)?;

    let mut uri_build = String::new();
    if let Some(scheme) = new_scheme {
        uri_build.push_str(scheme.to_string().as_str());
        uri_build.push_str("://");
    }
    uri_build.push_str(&new_host);
    uri_build.push(':');
    uri_build.push_str(&new_port.to_string());

    if !new_path.is_empty() {
        if !new_path.starts_with("/") {
            uri_build.push('/');
        }
        uri_build.push_str(&new_path);
    }

    if !new_query.is_empty() {
        if !new_query.starts_with("?") {
            uri_build.push('?');
        }
        uri_build.push_str(&new_query);
    }

    let uri: RUri = uri_build.parse().map_err(|_| Error::InterceptedRequest)?;

    req.method = new_method;
    req.uri = uri;
    req.version = new_version;
    req.body = parse_body(new_body);

    let mut new_headers = table_to_header_map(new_headers);
    if new_headers.get(HOST) != req.headers.get(HOST) {
        let header_host =
            HeaderValue::from_str(&new_host).map_err(|_| Error::InterceptedRequest)?;
        new_headers.insert(HOST, header_host);
    }
    req.headers = new_headers;
    req.trailers = table_to_header_map_opt(new_trailers);

    Ok(())
}

fn intercept_response_inner(lua: &Lua, res: &mut InterceptedResponse) -> Result<(), Error> {
    let extensions = lua.globals().get::<Table>(EXTENSIONS)?;

    if extensions.is_empty() {
        return Ok(());
    }

    let mut fns = vec![];

    for pairs in extensions.pairs::<Value, Table>() {
        let Ok((_, ext)) = pairs else { continue };
        if let Ok(inter) = ext
            .get::<Function>(INTERCEPT_RESPONSE)
            .or(ext.get::<Function>(2))
        {
            fns.push(inter);
        }
    }
    if fns.is_empty() {
        return Ok(());
    }

    let response_table = gen_response_table(lua, res)?;
    let request_table = lua.create_table()?;

    let flow_table = lua.create_table()?;
    flow_table.set(REQUEST, &request_table)?;
    flow_table.set(RESPONSE, &response_table)?;

    for f in fns {
        f.call::<Value>(&flow_table)?;
    }
    update_response(response_table, res)
}

fn gen_response_table(lua: &Lua, res: &mut InterceptedResponse) -> Result<Table, Error> {
    let response_table = lua.create_table()?;
    response_table.set(STATUS, res.status.as_u16())?;
    response_table.set(VERSION, format!("{:?}", res.version))?;
    response_table.set(BODY, String::from_utf8_lossy(&res.body).to_string())?;
    response_table.set(HEADERS, create_table(lua, &res.headers)?)?;
    response_table.set(TRAILERS, create_table_opt(lua, &res.trailers)?)?;
    Ok(response_table)
}

fn update_response(response_table: Table, res: &mut InterceptedResponse) -> Result<(), Error> {
    let new_status: u16 = response_table.get(STATUS)?;

    let new_version: String = response_table.get(VERSION)?;
    let new_version = parse_version(&new_version).unwrap_or(res.version);
    let new_headers: Table = response_table.get(HEADERS)?;
    let new_body = response_table.get(BODY)?;
    let new_trailers: Table = response_table.get(TRAILERS)?;

    res.status = StatusCode::from_u16(new_status).unwrap_or_default();
    res.version = new_version;
    res.body = parse_body(new_body);
    res.headers = table_to_header_map(new_headers);
    res.trailers = table_to_header_map_opt(new_trailers);

    Ok(())
}

fn parse_body(value: Value) -> bytes::Bytes {
    match value {
        Value::String(s) => s
            .to_str()
            .map(|s| Bytes::copy_from_slice(s.as_bytes()))
            .unwrap_or_else(|_| Bytes::new()),
        _ => Bytes::new(),
    }
}

fn create_table_opt(lua: &Lua, map: &Option<HeaderMap>) -> Result<Table, mlua::Error> {
    map.as_ref()
        .map(|m| create_table(lua, m))
        .unwrap_or(lua.create_table())
}

fn create_table(lua: &Lua, map: &HeaderMap) -> Result<Table, mlua::Error> {
    let table = lua.create_table()?;
    for k in map.keys() {
        let all_values = map.get_all(k);
        let mut values = vec![];
        for value in all_values {
            if let Ok(value) = value.to_str() {
                values.push(value.to_string());
            }
        }
        if values.len() == 1 {
            let first = values.first();
            if let Some(first) = first.cloned() {
                table.set(k.as_str(), first)?;
            }
        } else {
            let inner = lua.create_table()?;
            for v in values {
                inner.push(v)?;
            }
            table.set(k.as_str(), inner)?;
        }
    }
    Ok(table)
}

fn table_to_header_map(table: Table) -> HeaderMap {
    let mut map = HeaderMap::new();
    for pair in table.pairs::<String, Value>() {
        match pair {
            Ok((k, v)) => {
                let Ok(h) = HeaderName::from_bytes(k.as_bytes()) else {
                    continue;
                };
                match v {
                    Value::Table(table) => {
                        for pair in table.pairs::<String, String>() {
                            let Ok((_, v)) = pair else { continue };
                            let Ok(v) = v.parse() else { continue };
                            map.append(&h, v);
                        }
                    }
                    Value::String(s) => {
                        let Ok(s) = s.to_str() else {
                            continue;
                        };
                        let Ok(s) = HeaderValue::from_str(s.as_ref()) else {
                            continue;
                        };
                        map.append(h, s);
                    }
                    _ => {
                        error!("Unhandle header type {v:?}");
                    }
                }
            }
            Err(e) => {
                warn!("Invalid header {e}");
            }
        }
    }
    map
}

fn table_to_header_map_opt(table: Table) -> Option<HeaderMap> {
    let hm = table_to_header_map(table);
    if hm.is_empty() { None } else { Some(hm) }
}

fn query_to_table(lua: &Lua, url: Url) -> Result<Table, mlua::Error> {
    let table = lua.create_table()?;
    for qp in url.query_pairs() {
        table.set(qp.0.into_owned(), qp.1.into_owned())?;
    }
    Ok(table)
}

fn encode_query_table(table: Table) -> Result<String, Error> {
    let mut collected = vec![];
    for pairs in table.pairs::<String, String>() {
        let Ok((k, v)) = pairs else { continue };
        collected.push((k, v));
    }
    collected.sort();
    let mut builder = form_urlencoded::Serializer::new(String::new());
    for (k, v) in collected {
        builder.append_pair(&k, &v);
    }

    Ok(builder.finish())
}

fn register_functions(
    lua: &Lua,
    notify: Option<mpsc::Sender<FlowNotify>>,
) -> Result<(), mlua::Error> {
    let globals = lua.globals();

    let lua_notify = if let Some(notify) = notify {
        lua.create_function(move |_, (level, msg): (i32, String)| {
            if let Err(e) = notify.try_send(FlowNotify { level, msg }) {
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

    globals.set(EXTENSIONS, lua.create_table()?)?;
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

    Ok(())
}
