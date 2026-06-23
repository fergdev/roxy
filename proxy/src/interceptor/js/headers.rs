use std::{cell::RefCell, rc::Rc};

use boa_engine::{
    Context, JsData, JsResult, JsValue, boa_class, js_error, js_string, object::builtins::JsArray,
};
use boa_gc::{Finalize, Trace};
use http::{HeaderMap, HeaderName, HeaderValue};

fn to_header_name(name: &str) -> JsResult<HeaderName> {
    HeaderName::from_bytes(name.as_bytes())
        .map_err(|e| js_error!(TypeError: "Invalid header name: {}", e))
}

fn to_header_value(val: &str) -> JsResult<HeaderValue> {
    HeaderValue::from_str(val).map_err(|e| js_error!(TypeError: "Invalid header value: {}", e))
}
pub(crate) type HeaderList = Rc<RefCell<HeaderMap>>;

#[derive(Debug, Trace, Finalize, JsData, Clone)]
#[boa_gc(unsafe_no_drop)]
pub(crate) struct JsHeaders {
    #[unsafe_ignore_trace]
    pub headers: HeaderList,
}

// TODO: implement proxy for headers h["a"] = "a", delete h["a"]
// see JsProxyBuilder
impl Default for JsHeaders {
    fn default() -> Self {
        Self {
            headers: Rc::new(RefCell::new(HeaderMap::new())),
        }
    }
}

#[boa_class(rename = "Headers")]
impl JsHeaders {
    #[boa(constructor)]
    fn new() -> Self {
        Self::default()
    }

    #[boa(getter)]
    fn length(&self) -> usize {
        self.headers.borrow().len()
    }

    fn get(&self, name: String) -> JsResult<JsValue> {
        let found = self
            .headers
            .borrow()
            .get(&name)
            .and_then(|v| v.to_str().ok().map(ToString::to_string));

        Ok(match found {
            Some(v) => JsValue::new(js_string!(v)),
            None => JsValue::null(),
        })
    }

    #[boa(rename = "getAll")]
    fn get_all(&self, name: String, context: &mut Context) -> JsResult<JsValue> {
        let name = to_header_name(&name)?;
        let arr = JsArray::new(context);
        for (_, v) in self.headers.borrow().iter().filter(|(k, _)| name.eq(k)) {
            if let Ok(v) = v.to_str() {
                arr.push(JsValue::new(js_string!(v)), context)?;
            }
        }
        Ok(arr.into())
    }

    fn set(&self, name: String, value: JsValue, context: &mut Context) -> JsResult<()> {
        let name = to_header_name(&name)?;
        let mut list = self.headers.borrow_mut();
        list.remove(&name);
        if !value.is_null() && !value.is_undefined() {
            let js_string = value.to_string(context)?;
            let a = js_string.to_std_string_lossy();
            let value = to_header_value(&a)?;
            list.insert(name, value);
        }
        Ok(())
    }

    fn append(&self, name: String, value: JsValue, context: &mut Context) -> JsResult<()> {
        let name = to_header_name(&name)?;
        let value = to_header_value(&value.to_string(context)?.to_std_string_lossy())?;
        self.headers.borrow_mut().append(name, value);
        Ok(())
    }

    fn delete(&self, name: String) -> JsResult<()> {
        let name = to_header_name(&name)?;
        let mut list = self.headers.borrow_mut();
        list.remove(name);
        Ok(())
    }
    fn has(&self, name: String) -> JsResult<bool> {
        let name: HeaderName = name
            .parse()
            .map_err(|e| js_error!(TypeError: "Invalid header name: {}", e))?;
        let has = self.headers.borrow().iter().any(|(k, _)| k.eq(&name));
        Ok(has)
    }
    fn clear(&self) {
        self.headers.borrow_mut().clear();
    }

    #[boa(rename = "toString")]
    fn print(&self) -> String {
        let headers = self.headers.borrow();
        format!("{headers:?}")
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::interceptor::js::tests::setup;
    use boa_engine::Source;

    #[test]
    fn headers_constructor_creates_empty_map() {
        let mut ctx = setup();
        let ok = ctx
            .eval(Source::from_bytes(
                r#"
                const h = new Headers();
                assertTrue(typeof h === "object", "Headers should construct an object");
                // brand new: shouldn't have host
                assertEqual(h.has("host"), false, "no host by default");
                true
                "#,
            ))
            .unwrap();
        assert!(ok.is_boolean() && ok.as_boolean().unwrap());
    }

    #[test]
    fn set_and_get_roundtrip() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const h = new Headers();
            h.set("X-Trace", "abc123");
            assertEqual(h.get("X-Trace"), "abc123", "set/get roundtrip");
            assertEqual(h.has("X-Trace"), true, "has after set");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn append_and_get_all_multiple_values() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const h = new Headers();
            h.append("set-cookie", "a=1");
            h.append("set-cookie", "b=2");

            const all = h.getAll("set-cookie");
            assertTrue(Array.isArray(all), "getAll returns array");
            assertEqual(all.length, 2, "two values");

            // Order is insertion order for http::HeaderMap iteration, which is typically the order inserted.
            assertTrue(all.includes("a=1"), "contains a=1");
            assertTrue(all.includes("b=2"), "contains b=2");

            // get() returns a single value (first)
            const first = h.get("set-cookie");
            assertEqual(first, "a=1" || first === "b=2", "get returns one of values");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn delete_removes_all_values() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const h = new Headers();
            h.append("X-Foo", "1");
            h.append("X-Foo", "2");
            assertEqual(h.has("X-Foo"), true, "precondition: has X-Foo");
            h.delete("X-Foo");
            assertEqual(h.has("X-Foo"), false, "deleted all X-Foo");
            assertEqual(h.get("X-Foo"), null, "get null after delete");
            const all = h.getAll("X-Foo");
            assertTrue(Array.isArray(all) && all.length === 0, "getAll empty after delete");
            "#,
        ))
        .unwrap_or_else(|e| panic!("JS failed {e:?}"));
    }

    #[test]
    fn case_insensitive_names() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const h = new Headers();
            h.set("content-type", "text/plain");
            assertEqual(h.has("Content-Type"), true, "has is case-insensitive");
            assertEqual(h.get("CONTENT-TYPE"), "text/plain", "get is case-insensitive");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn value_coercion_to_string() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const h = new Headers();
            h.set("X-Num", 123);
            assertEqual(h.get("X-Num"), "123", "number coerces to string");

            h.set("X-BoolTrue", true);
            assertEqual(h.get("X-BoolTrue"), "true", "boolean true coerces");

            h.set("X-BoolFalse", false);
            assertEqual(h.get("X-BoolFalse"), "false", "boolean false coerces");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn invalid_header_name_throws_type_error() {
        let mut ctx = setup();
        let res = ctx
            .eval(Source::from_bytes(
                r#"
            try {
                const h = new Headers();
                h.set("Bad Name", "x"); // invalid due to space
                assertTrue(false, "expected TypeError");
            } catch (e) {
                assertTrue(e instanceof TypeError, "TypeError for invalid header name");
                true
            }
            "#,
            ))
            .unwrap();
        assert!(res.is_boolean() && res.as_boolean().unwrap());
    }

    #[test]
    fn invalid_header_value_throws_type_error() {
        let mut ctx = setup();
        let res = ctx
            .eval(Source::from_bytes(
                r#"
            try {
                const h = new Headers();
                h.set("X", "line1\r\nline2"); // CRLF not allowed
                assertTrue(false, "expected TypeError");
            } catch (e) {
                assertTrue(e instanceof TypeError, "TypeError for invalid header value");
                true
            }
            "#,
            ))
            .unwrap();
        assert!(res.is_boolean() && res.as_boolean().unwrap());
    }

    #[test]
    fn append_and_get_all_case_insensitive_name_matching() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const h = new Headers();
            h.append("Set-Cookie", "a=1");
            h.append("set-cookie", "b=2");
            const all = h.getAll("SET-COOKIE");
            assertTrue(Array.isArray(all), "Is array");
            assertTrue(all.length === 2, "both values under case-insensitive key");
            "#,
        ))
        .unwrap_or_else(|e| panic!("JS failed {e:?}"));
    }

    #[test]
    fn has_false_when_absent() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const h = new Headers();
            assertEqual(h.has("Not-There"), false, "missing returns false");
            assertEqual(h.get("Not-There"), null, "get null when missing");
            const all = h.getAll("Not-There");
            assertTrue(Array.isArray(all) && all.length === 0, "empty getAll when missing");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn clear_empties_headers() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const h = new Headers();
            h.append("Set-Cookie", "a=1");
            h.append("set-cookie", "b=2");
            assertTrue(h.length === 2, "length should be 2");
            h.clear();
            assertTrue(h.length === 0, "length should be 0");
            "#,
        ))
        .unwrap();
    }
}
