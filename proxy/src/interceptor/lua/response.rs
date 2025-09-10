use http::StatusCode;
use mlua::prelude::*;
use roxy_shared::version::HttpVersion;
use std::sync::{Arc, Mutex};

use crate::flow::InterceptedResponse;
use crate::interceptor::lua::body::LuaBody;
use crate::interceptor::lua::headers::LuaHeaders;
use crate::interceptor::{KEY_BODY, KEY_HEADERS, KEY_STATUS, KEY_TRAILERS, KEY_VERSION};

#[derive(Clone, Debug)]
pub(crate) struct LuaResponse {
    inner: Arc<Mutex<InterceptedResponse>>,
    pub body: LuaBody,
    pub headers: LuaHeaders,
    pub trailers: LuaHeaders,
}

impl Default for LuaResponse {
    fn default() -> Self {
        let inner = Arc::new(Mutex::new(InterceptedResponse::default()));
        Self {
            inner,
            body: LuaBody::default(),
            headers: LuaHeaders::default(),
            trailers: LuaHeaders::default(),
        }
    }
}

impl LuaResponse {
    pub(crate) fn get_inner(&self) -> LuaResult<InterceptedResponse> {
        let mut res = self.lock()?;
        res.body = self
            .body
            .inner
            .lock()
            .map_err(|e| LuaError::external(format!("lock poisoned: {e}")))?
            .clone();
        res.headers = self
            .headers
            .map
            .lock()
            .map_err(|e| LuaError::external(format!("lock poisoned: {e}")))?
            .clone();
        let trailers = self
            .trailers
            .map
            .lock()
            .map_err(|e| LuaError::external(format!("lock poisoned: {e}")))?
            .clone();
        res.trailers = if trailers.is_empty() {
            None
        } else {
            Some(trailers)
        };
        Ok(res.clone())
    }

    pub fn from_parts(inner: Arc<Mutex<InterceptedResponse>>) -> LuaResult<Self> {
        let (hdr_arc, trl_arc, body) = {
            let g = inner
                .lock()
                .map_err(|e| LuaError::external(format!("lock poisoned: {e}")))?;
            (
                g.headers.clone(),
                g.trailers.clone().unwrap_or_default(),
                g.body.clone(),
            )
        };

        Ok(Self {
            inner,
            body: LuaBody::from_bytes(body),
            headers: LuaHeaders::new(hdr_arc),
            trailers: LuaHeaders::new(trl_arc),
        })
    }

    fn lock(&self) -> LuaResult<std::sync::MutexGuard<'_, InterceptedResponse>> {
        self.inner
            .lock()
            .map_err(|e| LuaError::external(format!("lock poisoned: {e}")))
    }
}

impl LuaUserData for LuaResponse {
    fn add_methods<M: LuaUserDataMethods<Self>>(m: &mut M) {
        m.add_meta_method(LuaMetaMethod::Index, |lua, this, key: LuaValue| {
            if let LuaValue::String(s) = key {
                match &*s.to_str()? {
                    KEY_STATUS => {
                        let g = this.lock()?;
                        return Ok(LuaValue::Integer(g.status.as_u16() as i64));
                    }
                    KEY_HEADERS => {
                        return Ok(LuaValue::UserData(
                            lua.create_userdata(this.headers.clone())?,
                        ));
                    }
                    KEY_TRAILERS => {
                        return Ok(LuaValue::UserData(
                            lua.create_userdata(this.trailers.clone())?,
                        ));
                    }
                    KEY_BODY => {
                        return Ok(LuaValue::UserData(lua.create_userdata(this.body.clone())?));
                    }
                    _ => {}
                }
            }
            Ok(LuaValue::Nil)
        });

        m.add_meta_method_mut(
            LuaMetaMethod::NewIndex,
            |_, this, (key, val): (LuaValue, LuaValue)| {
                let k = match key {
                    LuaValue::String(s) => s.to_str()?.to_string(),
                    _ => return Err(LuaError::external("property name must be string")),
                };
                match (k.as_str(), val) {
                    (KEY_STATUS, LuaValue::Integer(i)) => {
                        let mut g = this.lock()?;
                        g.status = StatusCode::from_u16(i as u16)
                            .map_err(|e| LuaError::RuntimeError(format!("{e}")))?;
                    }
                    (KEY_VERSION, LuaValue::String(s)) => {
                        let value = &*s.to_str()?;
                        let version: HttpVersion = value.parse().map_err(|_| {
                            LuaError::RuntimeError(format!("invalid HTTP version '{}'", value))
                        })?;
                        let mut g = this.lock()?;
                        g.version = version;
                    }
                    (KEY_HEADERS | KEY_TRAILERS | KEY_BODY, _) => {
                        return Err(LuaError::external(
                            "property is read-only; mutate its fields instead",
                        ));
                    }
                    _ => {
                        return Err(LuaError::external(format!(
                            "unsupported assignment to {}",
                            k
                        )));
                    }
                }
                Ok(())
            },
        );
    }
}

pub fn register_response(lua: &Lua) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;
    let new = lua.create_function(|_, code: Option<i32>| {
        let mut inner = InterceptedResponse::default();
        if let Some(c) = code {
            inner.status = StatusCode::from_u16(c as u16).unwrap_or(StatusCode::OK);
        }
        Ok(LuaResponse::from_parts(Arc::new(Mutex::new(inner))))
    })?;
    tbl.set("new", new)?;
    lua.globals().set("Response", tbl.clone())?;
    Ok(tbl)
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::interceptor::lua::response::register_response;

    fn with_lua<F: FnOnce(&Lua) -> LuaResult<()>>(f: F) {
        let lua = Lua::new();
        register_response(&lua).unwrap();
        f(&lua).expect("lua ok");
    }

    #[test]
    fn s01_defaults_and_constructor() {
        with_lua(|lua| {
            lua.load(
                r#"
                local r = Response.new()
                assert(type(r) == "userdata")
                assert(r.status == 200)
                assert(type(r.headers) == "userdata")
                assert(type(r.trailers) == "userdata")
                assert(type(r.body) == "userdata")

                local r2 = Response.new(404)
                assert(r2.status == 404)
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn s02_set_status_and_version() {
        with_lua(|lua| {
            lua.load(
                r#"
                local r = Response.new()
                r.status = 201
                assert(r.status == 201)

                -- version setter exists; we don't have a getter, just verify it doesn't throw.
                r.version = "HTTP/2.0"
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn s03_headers_and_trailers_userdata_roundtrip() {
        with_lua(|lua| {
            lua.load(
                r#"
                local r = Response.new()
                local h = r.headers
                h:set_all("Set-Cookie", {"a=1", "b=2"})
                local all = h:get_all("set-cookie")
                assert(#all == 2)
                assert(all[1] == "a=1")
                assert(all[2] == "b=2")

                local t = r.trailers
                t:set_all("X-T", {"1"})
                local tt = t:get_all("x-t")
                assert(#tt == 1 and tt[1] == "1")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn s04_body_text_and_raw() {
        with_lua(|lua| {
            lua.load(
                r#"
                local r = Response.new()

                -- text roundtrip
                r.body.text = "hello"
                assert(r.body.text == "hello")

                -- raw accepts bytes (Lua string)
                r.body.raw = "x\0y"
                local raw = r.body.raw
                assert(#raw == 3)
                -- length/len should reflect the raw body size
                assert(r.body.length == 3 or r.body.len == 3)

                -- do not assert text after binary write (could be invalid UTF-8)
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn s05_invalid_status_raises() {
        with_lua(|lua| {
            lua.load(
                r#"
                local ok, err = pcall(function()
                    local r = Response.new()
                    r.status = 1000   -- invalid HTTP status
                end)
                assert(ok == false)
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn s06_get_inner_collects_body_headers_trailers() {
        let lua = Lua::new();
        register_response(&lua).unwrap();

        let inner = Arc::new(Mutex::new(InterceptedResponse::default()));
        let resp = LuaResponse::from_parts(inner.clone()).expect("parts");
        let ud = lua.create_userdata(resp.clone()).expect("ud");
        lua.globals().set("resp_obj", ud).expect("set global");

        lua.load(
            r#"
            resp_obj.status = 204
            resp_obj.headers:set_all("A", {"1","2"})
            resp_obj.trailers:set_all("B", {"x"})
            resp_obj.body.text = "payload"
        "#,
        )
        .exec()
        .expect("lua exec ok");

        let merged = resp.get_inner().expect("get_inner");
        assert_eq!(merged.status, StatusCode::NO_CONTENT);
        assert_eq!(String::from_utf8_lossy(&merged.body), "payload");

        assert_eq!(
            merged
                .headers
                .get_all("A")
                .iter()
                .map(|v| v.to_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["1", "2"]
        );
        let trailers = merged.trailers.as_ref().expect("some trailers");
        assert_eq!(
            trailers.get("B").and_then(|v| v.to_str().ok()).unwrap(),
            "x"
        );
    }
}
