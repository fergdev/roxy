use std::sync::{Arc, Mutex};

use mlua::prelude::*;

use crate::interceptor::{
    KEY_REQUEST, KEY_RESPONSE,
    lua::{request::LuaRequest, response::LuaResponse, util::KEY_NEW},
};

#[derive(Clone, Debug, Default)]
pub(crate) struct LuaFlow {
    inner: Arc<Mutex<FlowInner>>,
}

#[derive(Clone, Debug, Default)]
struct FlowInner {
    request: LuaRequest,
    response: LuaResponse,
}

impl LuaFlow {
    pub fn from_views(request: LuaRequest, response: LuaResponse) -> Self {
        Self {
            inner: Arc::new(Mutex::new(FlowInner { request, response })),
        }
    }

    fn lock(&self) -> LuaResult<std::sync::MutexGuard<'_, FlowInner>> {
        self.inner
            .lock()
            .map_err(|e| LuaError::external(format!("lock poisoned: {e}")))
    }
}

impl LuaUserData for LuaFlow {
    fn add_methods<M: LuaUserDataMethods<Self>>(m: &mut M) {
        m.add_meta_method(LuaMetaMethod::Index, |lua, this, key: LuaValue| {
            if let LuaValue::String(s) = key {
                let k = s.to_str()?;
                match &*k {
                    KEY_REQUEST => {
                        let req = this.lock()?.request.clone();
                        let ud = lua.create_userdata(req)?;
                        return Ok(LuaValue::UserData(ud));
                    }
                    KEY_RESPONSE => {
                        let resp = this.lock()?.response.clone();
                        let ud = lua.create_userdata(resp)?;
                        return Ok(LuaValue::UserData(ud));
                    }
                    _ => {}
                }
            }
            Ok(LuaValue::Nil)
        });
        // TODO: implement
        // m.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| this.get_text());
    }
}

pub(crate) fn register_flow(lua: &Lua) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;
    let new = lua.create_function(move |lua, ()| {
        let flow = LuaFlow::default();
        lua.create_userdata(flow)
    })?;
    tbl.set(KEY_NEW, new)?;
    lua.globals().set("Flow", tbl.clone())?;
    Ok(tbl)
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::interceptor::lua::tests::with_lua;

    #[test]
    fn f01_flow_has_request_and_response_userdata() {
        with_lua(|lua| {
            lua.load(
                r#"
                local flow = Flow.new()
                assert(type(flow) == "userdata", "flow must be userdata, got "..type(flow))
                assert(type(flow.request) == "userdata", "flow.request must be userdata")
                assert(type(flow.response) == "userdata", "flow.response must be userdata")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn f02_set_request_body_through_flow_property() {
        with_lua(|lua| {
            lua.load(
                r#"
                local flow = Flow.new()
                flow.request.body.text = "rewrite request"
                assert(flow.request.body.text == "rewrite request")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn f03_set_response_body_through_flow_property() {
        with_lua(|lua| {
            lua.load(
                r#"
                local flow = Flow.new()
                flow.response.body.text = "rewrite response"
                assert(flow.response.body.text == "rewrite response")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn f04_request_handles_alias_same_state() {
        with_lua(|lua| {
            lua.load(
                r#"
                local flow = Flow.new()
                local r1 = flow.request
                local r2 = flow.request   -- a second handle
                r1.body.text = "hello"
                assert(r2.body.text == "hello", "both handles must see same underlying state")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn f05_cannot_replace_request_or_response() {
        with_lua(|lua| {
            lua.load(
                r#"
                local flow = Flow.new()
                local ok1, err1 = pcall(function() flow.request = {} end)
                local ok2, err2 = pcall(function() flow.response = {} end)
                -- We expect both to fail. The exact message depends on your __newindex,
                -- but pcall must return false.
                assert(not ok1, "replacing flow.request should fail")
                assert(not ok2, "replacing flow.response should fail")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn f06_request_and_response_are_not_functions() {
        with_lua(|lua| {
            lua.load(
                r#"
                local flow = Flow.new()
                assert(type(flow.request) ~= "function", "flow.request must not be a function")
                assert(type(flow.response) ~= "function", "flow.response must not be a function")
            "#,
            )
            .exec()
        });
    }
}
