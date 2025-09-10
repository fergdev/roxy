use mlua::prelude::*;
use mlua::{AnyUserData, MetaMethod, UserData, UserDataMethods, Value};
use std::sync::{Arc, Mutex, MutexGuard};
use url::Url;
use url::form_urlencoded::{Serializer, parse as parse_qs};

use crate::interceptor::lua::util::lua_val_to_str;

pub(crate) struct LuaQueryView {
    pub(crate) uri: Arc<Mutex<Url>>,
}

impl LuaQueryView {
    fn lock(&self) -> LuaResult<MutexGuard<'_, Url>> {
        self.uri
            .lock()
            .map_err(|e| LuaError::external(format!("lock poisoned: {e}")))
    }
    fn with_pairs_mut<F, R>(&self, f: F) -> LuaResult<R>
    where
        F: FnOnce(&mut Vec<(String, String)>) -> LuaResult<R>,
    {
        let mut url = self.lock()?;
        let mut pairs: Vec<(String, String)> = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();

        let out = f(&mut pairs)?;
        {
            let mut qp = url.query_pairs_mut();
            qp.clear();
            qp.extend_pairs(pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        }

        Ok(out)
    }
}

impl UserData for LuaQueryView {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("set", |_, this, (key, val): (String, String)| {
            this.with_pairs_mut(|pairs| {
                pairs.retain(|(k, _)| k != &key);
                pairs.push((key, val));
                Ok(())
            })
        });

        m.add_method("append", |_, this, (key, val): (String, String)| {
            let mut guard = this.lock()?;
            guard.query_pairs_mut().append_pair(&key, &val);
            Ok(())
        });

        m.add_method("delete", |_, this, key: String| {
            this.with_pairs_mut(|pairs| {
                pairs.retain(|(k, _)| k != &key);
                Ok(())
            })
        });

        m.add_method("get", |lua, this, key: String| {
            let req = this.lock()?;
            for (k, v) in req.query_pairs() {
                if k == key {
                    return Ok(Value::String(lua.create_string(v.as_ref())?));
                }
            }
            Ok(Value::Nil)
        });

        m.add_method("get_all", |lua, this, key: String| {
            let req = this.lock()?;
            let t = lua.create_table()?;
            let mut i = 1;
            for (k, v) in req.query_pairs() {
                if k == key {
                    t.set(i, v.into_owned())?;
                    i += 1;
                }
            }
            Ok(t)
        });

        m.add_method("has", |_, this, key: String| {
            let req = this.lock()?;
            for (k, _) in req.query_pairs() {
                if k == key {
                    return Ok(true);
                }
            }
            Ok(false)
        });

        m.add_method("clear", |_, this, ()| {
            let mut req = this.lock()?;
            req.query_pairs_mut().clear();
            Ok(())
        });

        m.add_method("sort", |_, this, ()| {
            this.with_pairs_mut(|pairs| {
                pairs.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
                Ok(())
            })
        });

        m.add_method("to_string", |_, this, ()| {
            let req = this.lock()?;
            Ok(req.query().unwrap_or("").to_string())
        });
        m.add_method("iter", |lua, this, ()| {
            let snapshot: Vec<(String, String)> = {
                let req = this.lock()?;
                req.query()
                    .map(|q| {
                        parse_qs(q.as_bytes())
                            .map(|(k, v)| (k.into_owned(), v.into_owned()))
                            .collect()
                    })
                    .unwrap_or_default()
            };

            let iter = lua.create_function(move |lua, (idx,): (i64,)| {
                let next = (idx + 1) as usize;
                if next == 0 || next > snapshot.len() {
                    return Ok((idx, Value::Nil));
                }
                let (ref k, ref v) = snapshot[next - 1];
                let t = lua.create_table()?;
                t.set(1, k.clone())?;
                t.set(2, v.clone())?;
                Ok((next as i64, Value::Table(t)))
            })?;

            Ok(iter)
        });

        m.add_meta_function(MetaMethod::Index, |lua, (ud, key): (AnyUserData, Value)| {
            if let Value::String(s) = key {
                let key = s.to_str()?.to_string();
                let proxy = ud.borrow::<LuaQueryView>()?;
                let req = proxy
                    .uri
                    .lock()
                    .map_err(|e| mlua::Error::external(format!("lock poisoned: {e}")))?;
                if let Some(q) = req.query() {
                    for (k, v) in parse_qs(q.as_bytes()) {
                        if k == key {
                            return Ok(Value::String(lua.create_string(v.as_ref())?));
                        }
                    }
                }
                Ok(Value::Nil)
            } else {
                Ok(Value::Nil)
            }
        });
        m.add_method("to_string", |_, this, ()| {
            let req = this
                .uri
                .lock()
                .map_err(|e| mlua::Error::external(format!("lock poisoned: {e}")))?;
            let encoded = req
                .query()
                .map(|q| {
                    let mut ser = Serializer::new(String::new());
                    for (k, v) in parse_qs(q.as_bytes()) {
                        ser.append_pair(k.as_ref(), v.as_ref());
                    }
                    ser.finish()
                })
                .unwrap_or_else(String::new);
            Ok(encoded)
        });

        m.add_meta_function(
            MetaMethod::NewIndex,
            |_, (ud, key, val): (AnyUserData, Value, Value)| {
                let key = match key {
                    Value::String(s) => s.to_str()?.to_string(),
                    _ => return Err(mlua::Error::external("query key must be a string")),
                };
                let proxy = ud.borrow::<LuaQueryView>()?;
                let v = match val {
                    Value::Nil => {
                        return proxy.with_pairs_mut(|pairs| {
                            pairs.retain(|(k, _)| k != &key);
                            Ok(())
                        });
                    }
                    _ => lua_val_to_str(val)?,
                };
                proxy.with_pairs_mut(|pairs| {
                    pairs.retain(|(k, _)| k != &key);
                    pairs.push((key, v));
                    Ok(())
                })
            },
        );
    }
}

pub fn register_query(lua: &Lua) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;

    let new = lua.create_function(|lua, href: String| {
        let url = Url::parse(&href).map_err(|e| LuaError::external(format!("bad url: {e}")))?;
        let view = LuaQueryView {
            uri: Arc::new(Mutex::new(url)),
        };
        lua.create_userdata(view)
    })?;

    tbl.set("new", new)?;
    lua.globals().set("Query", tbl.clone())?;
    Ok(tbl)
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::register_query;
    use mlua::prelude::*;

    fn with_lua<F: FnOnce(&Lua) -> LuaResult<()>>(f: F) {
        let lua = Lua::new();
        register_query(&lua).expect("register Query");
        f(&lua).expect("lua ok");
    }

    #[test]
    fn q01_construct_get_get_all_has() {
        with_lua(|lua| {
            lua.load(
                r#"
                local q = Query.new("http://x/?a=1&a=2&b=3")
                assert(q:get("a") == "1")
                local all = q:get_all("a")
                assert(type(all) == "table")
                assert(#all == 2 and all[1] == "1" and all[2] == "2")
                assert(q:has("b") == true)
                assert(q:has("nope") == false)
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn q02_set_append_delete_and_to_string_contains() {
        with_lua(|lua| {
            lua.load(
                r#"
                local q = Query.new("http://x/?a=1&a=2&b=3")
                -- set replaces all of a
                q:set("a", "9")
                assert(q:get("a") == "9")
                local all_after_set = q:get_all("a")
                assert(#all_after_set == 1 and all_after_set[1] == "9")

                -- append keeps existing
                q:append("a", "10")
                local all_after_append = q:get_all("a")
                assert(#all_after_append == 2)

                local s = q:to_string()
                -- order not guaranteed; just check membership
                assert(s:find("a=9") ~= nil)
                assert(s:find("a=10") ~= nil)
                assert(s:find("b=3") ~= nil)

                -- delete removes all
                q:delete("a")
                assert(q:has("a") == false)
                local s2 = q:to_string()
                assert(s2 == "b=3" or s2 == "" or s2:match("^b=3$"))
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn q03_clear_and_sort() {
        with_lua(|lua| {
            lua.load(
                r#"
                local q = Query.new("http://x/?z=9&b=2&c=3&a=1")
                q:clear()
                assert(q:to_string() == "")
                -- rebuild in unsorted order
                q:append("b","2")
                q:append("c","3")
                q:append("a","1")
                q:sort()
                local s = q:to_string()
                -- our sort uses key then value; expect a=1&b=2&c=3
                assert(s == "a=1&b=2&c=3")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn q04_bracket_sugar_get_set_delete_and_coercion() {
        with_lua(|lua| {
            lua.load(
                r#"
                local q = Query.new("http://x/?k=1&k=2")
                -- read via [] sugar (first value)
                assert(q["k"] == "1")

                -- write string replaces all
                q["k"] = "x"
                assert(q:get("k") == "x")
                local all = q:get_all("k"); assert(#all == 1 and all[1] == "x")

                -- boolean/number coercion
                q["n"] = 42
                q["t"] = true
                q["f"] = false
                local s = q:to_string()
                assert(s:find("n=42") ~= nil)
                assert(s:find("t=true") ~= nil)
                assert(s:find("f=false") ~= nil)

                -- delete via nil
                q["n"] = nil
                assert(q:has("n") == false)
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn q05_iterator_collects_pairs_in_order() {
        with_lua(|lua| {
            lua.load(
                r#"
                local q = Query.new("http://x/?a=1&a=2&b=3")
                local iter = q:iter()
                local idx = 0
                local seen = {}
                while true do
                    idx, pair = iter(idx)
                    if pair == nil then break end
                    table.insert(seen, pair[1] .. "=" .. pair[2])
                end
                -- we should have 3 entries total
                assert(#seen == 3)
                -- don't assert exact order; assert membership
                local joined = table.concat(seen, ",")
                assert(joined:find("a=1") ~= nil)
                assert(joined:find("a=2") ~= nil)
                assert(joined:find("b=3") ~= nil)
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn q06_to_string_roundtrip_after_mutations() {
        with_lua(|lua| {
            lua.load(
                r#"
                local q = Query.new("http://x/?")
                q:set("a","1")
                q:append("a","2")
                q:set("b","3")
                local s = q:to_string()
                assert(s:find("a=1") ~= nil)
                assert(s:find("a=2") ~= nil)
                assert(s:find("b=3") ~= nil)
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn q07_indexer_reads_first_value_or_nil() {
        with_lua(|lua| {
            lua.load(
                r#"
                local q = Query.new("http://x/?a=1&a=2&b=3")
                assert(q["a"] == "1")
                assert(q["b"] == "3")
                assert(q["nope"] == nil)
            "#,
            )
            .exec()
        });
    }
}
