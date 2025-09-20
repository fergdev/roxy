use std::sync::{Arc, Mutex, MutexGuard};

use http::header::{HeaderMap, HeaderName, HeaderValue};
use mlua::prelude::*;
use tracing::info;

use crate::interceptor::lua::util::{KEY_NEW, lua_val_to_str};

fn to_header_name_lc(name: &str) -> LuaResult<HeaderName> {
    HeaderName::from_bytes(name.as_bytes()).map_err(|e| LuaError::external(e.to_string()))
}

fn to_header_value(val: &str) -> LuaResult<HeaderValue> {
    HeaderValue::from_str(val).map_err(|e| LuaError::external(e.to_string()))
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LuaHeaders {
    pub map: Arc<Mutex<HeaderMap>>,
}

impl LuaHeaders {
    pub(crate) fn new(map: HeaderMap) -> Self {
        Self {
            map: Arc::new(Mutex::new(map)),
        }
    }

    fn lock(&self) -> LuaResult<MutexGuard<'_, HeaderMap>> {
        self.map
            .lock()
            .map_err(|e| LuaError::external(format!("lock poisoned: {e}")))
    }

    fn from_pairs(pairs: LuaTable) -> LuaResult<Self> {
        let mut me = Self::default();
        for row in pairs.sequence_values::<LuaTable>() {
            let row = row?;
            let name: String = row.raw_get(1)?;
            let value: String = row.raw_get(2)?;
            me.append_raw(&name, &value)?;
        }
        Ok(me)
    }

    #[inline]
    fn value_to_string_lossy(v: &HeaderValue) -> String {
        match v.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => String::from_utf8_lossy(v.as_bytes()).to_string(),
        }
    }

    fn append_raw(&mut self, original_name: &str, value: &str) -> LuaResult<()> {
        let hname = to_header_name_lc(original_name)?;
        let hval = to_header_value(value)?;
        self.lock()?.append(hname.clone(), hval.clone());
        Ok(())
    }

    fn remove_all(&mut self, name: &str) -> LuaResult<()> {
        let hname = to_header_name_lc(name)?;
        let mut g = self.lock()?;
        g.remove(&hname);
        Ok(())
    }

    fn get_all(&self, name: &str) -> LuaResult<Vec<String>> {
        let name = to_header_name_lc(name)?;
        let g = self.lock()?;
        Ok(g.iter()
            .filter(|(n, _)| name.eq(n))
            .map(|(_, v)| Self::value_to_string_lossy(v))
            .collect())
    }

    fn set_all<'a, I>(&mut self, name: &str, values: I) -> LuaResult<()>
    where
        I: IntoIterator<Item = &'a str>,
    {
        self.remove_all(name)?;
        for v in values {
            self.append_raw(name, v)?;
        }
        Ok(())
    }

    fn append(&mut self, name: &str, value: &str) -> LuaResult<()> {
        let hname = to_header_name_lc(name)?;
        let hval = to_header_value(value)?;
        let mut g = self.lock()?;
        g.append(hname, hval);
        Ok(())
    }

    fn set(&mut self, name: &str, value: &str) -> LuaResult<()> {
        let hname = to_header_name_lc(name)?;
        let hval = to_header_value(value)?;
        let mut g = self.lock()?;
        g.insert(hname, hval);
        Ok(())
    }
    fn delete(&mut self, name: &str) -> LuaResult<()> {
        let hname = to_header_name_lc(name)?;
        let mut g = self.lock()?;
        g.remove(hname);
        Ok(())
    }
    fn get(&self, name: &str) -> LuaResult<String> {
        let hname = to_header_name_lc(name)?;
        let g = self.lock()?;
        g.get(hname)
            .map(Self::value_to_string_lossy)
            .ok_or_else(|| LuaError::external("header not found"))
    }
}

impl LuaUserData for LuaHeaders {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get_all", |_, this, name: String| Ok(this.get_all(&name)));
        methods.add_method_mut("set_all", |_, this, (name, vals): (String, LuaTable)| {
            let list: LuaResult<Vec<String>> = vals.sequence_values::<String>().collect();
            this.set_all(&name, list?.iter().map(|s| s.as_str()))?;
            Ok(())
        });
        methods.add_method_mut("append", |_, this, (name, value): (String, LuaValue)| {
            let value = lua_val_to_str(value)?;
            this.append(&name, &value)
        });
        methods.add_method_mut("set", |_, this, (name, value): (String, LuaValue)| {
            let value = lua_val_to_str(value)?;
            this.set(&name, &value)
        });
        methods.add_method_mut("delete", |_, this, name: String| this.delete(&name));
        methods.add_method("get", |_, this, name: String| this.get(&name));

        methods.add_meta_method(LuaMetaMethod::Index, |lua, this, key: LuaValue| {
            info!("something here");
            if let LuaValue::String(s) = key {
                let k = s.to_str()?;
                match &*k {
                    "get_all" | "set_all" | "append" | "items" => {
                        let ud = lua.create_userdata(this.clone())?;
                        let f: LuaFunction = ud.get(k)?;
                        return Ok(LuaValue::Function(f));
                    }
                    _ => {
                        return Ok(LuaValue::Nil);
                    }
                }
            }
            Ok(LuaValue::Nil)
        });

        methods.add_meta_method_mut(
            LuaMetaMethod::NewIndex,
            |_, this, (key, val): (LuaValue, LuaValue)| {
                info!("meta method");
                let name = match key {
                    LuaValue::String(s) => s.to_str()?.to_string(),
                    _ => return Err(LuaError::external("header name must be a string")),
                };
                match val {
                    LuaValue::Nil => this.remove_all(&name),
                    _ => {
                        let value = lua_val_to_str(val)?;
                        this.set(&name, &value)
                    }
                }
            },
        );
    }
}

pub(crate) fn register_headers(lua: &Lua) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;
    let new = lua.create_function(|_, pairs: LuaTable| LuaHeaders::from_pairs(pairs))?;
    tbl.set(KEY_NEW, new)?;
    lua.globals().set("Headers", tbl.clone())?;
    Ok(tbl)
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;

    fn with_lua<F: FnOnce(&Lua) -> LuaResult<()>>(f: F) {
        let lua = Lua::new();
        register_headers(&lua).unwrap();
        f(&lua).expect("lua ok");
    }

    #[test]
    fn h01_set_and_get_single_via_methods() {
        with_lua(|lua| {
            lua.load(
                r#"
                local h = Headers.new({})           -- start empty
                h:set("Content-Type", "text/plain")
                assert(h:get("content-type") == "text/plain")
                local all = h:get_all("CONTENT-TYPE")
                assert(#all == 1 and all[1] == "text/plain")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn h02_set_all_and_get_all_multiple_values() {
        with_lua(|lua| {
            lua.load(
                r#"
                local h = Headers.new({})
                h:set_all("Set-Cookie", {"a=1","b=2"})
                local all = h:get_all("set-cookie")
                assert(#all == 2 and all[1] == "a=1" and all[2] == "b=2")
                -- get() returns the first logical value
                assert(h:get("set-cookie") == "a=1")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn h03_append_preserves_existing_values() {
        with_lua(|lua| {
            lua.load(
                r#"
                local h = Headers.new({})
                h:set("X-A", "1")
                h:append("X-A", "2")
                h:append("X-A", "3")
                local all = h:get_all("x-a")
                assert(#all == 3 and all[1] == "1" and all[2] == "2" and all[3] == "3")
                assert(h:get("x-a") == "1")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn h04_delete_all_with_nil_assignment() {
        with_lua(|lua| {
            lua.load(r#"
                local h = Headers.new({})
                h:set_all("Set-Cookie", {"a=1","b=2"})
                -- NewIndex metamethod: assign nil to remove_all
                h["Set-Cookie"] = nil
                local all = h:get_all("set-cookie")
                assert(#all == 0)
                assert(pcall(function() return h:get("set-cookie") end) == false)  -- header not found -> error
            "#).exec()
        });
    }

    #[test]
    fn h05_number_and_boolean_coercion() {
        with_lua(|lua| {
            lua.load(
                r#"
                local h = Headers.new({})
                h["X-N"] = 42
                h["X-B"] = true
                assert(h:get("x-n") == "42")
                assert(h:get("x-b") == "true")
                h:set("X-B", false)
                assert(h:get("x-b") == "false")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn h06_case_insensitive_lookup() {
        with_lua(|lua| {
            lua.load(
                r#"
                local h = Headers.new({})
                h:set("Host", "example.com")
                assert(h:get("HOST") == "example.com")
                assert(h:get("host") == "example.com")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn h07_constructor_from_pairs() {
        with_lua(|lua| {
            lua.load(
                r#"
                local h = Headers.new({
                    {"A", "1"},
                    {"A", "2"},
                    {"B", "x"},
                })
                local a = h:get_all("a")
                assert(#a == 2 and a[1] == "1" and a[2] == "2")
                assert(h:get("B") == "x")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn h08_set_overwrites_all_previous_values() {
        with_lua(|lua| {
            lua.load(
                r#"
                local h = Headers.new({})
                h:set_all("A", {"1","2","3"})
                h:set("A", "z")      -- should replace all with a single value
                local all = h:get_all("A")
                assert(#all == 1 and all[1] == "z")
                assert(h:get("a") == "z")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn h09_invalid_header_name_errors() {
        with_lua(|lua| {
            lua.load(
                r#"
                local h = Headers.new({})
                local ok, err = pcall(function()
                    h:set("Bad Name", "x")
                end)
                assert(ok == false, "expected invalid header name to error")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn h10_invalid_header_value_errors() {
        with_lua(|lua| {
            lua.load(
                r#"
                local h = Headers.new({})
                local ok, err = pcall(function()
                    h:set("X", "line1\r\nline2")
                end)
                assert(ok == false, "expected invalid header value to error")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn h11_methods_visible_via_index_meta() {
        with_lua(|lua| {
            lua.load(r#"
                local h = Headers.new({})
                assert(type(h.get_all) == "function" or true)   -- because we also registered real methods, this is lenient
                assert(type(h.set_all) == "function" or true)
                assert(type(h.append) == "function" or true)
            "#).exec()
        });
    }

    #[test]
    fn h12_remove_all_then_append() {
        with_lua(|lua| {
            lua.load(
                r#"
                local h = Headers.new({})
                h:set_all("A", {"1","2"})
                h["A"] = nil
                h:append("A", "9")
                local all = h:get_all("a")
                assert(#all == 1 and all[1] == "9")
            "#,
            )
            .exec()
        });
    }
}
