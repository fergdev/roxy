use std::{cell::RefCell, rc::Rc};

use boa_engine::{
    Context, JsData, JsResult, JsValue, js_error, js_string, object::builtins::JsArray,
    value::Convert,
};
use boa_gc::{Finalize, Trace};
use boa_interop::{JsClass, js_class};
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

js_class! {
    class JsHeaders as "Headers" {
        constructor() {
            Ok(Self::default())
        }

        init(_class: &mut ClassBuilder) -> JsResult<()> {
            Ok(())
        }

        fn get(this: JsClass<JsHeaders>, name: Convert<String>) -> JsResult<JsValue> {
            let name = name.0.clone();
            let found = this.borrow().headers
                .borrow()
                .get(&name)
                .and_then(|v| v.to_str().ok().map(ToString::to_string));

            Ok(match found {
                Some(v) => JsValue::String(js_string!(v)),
                None => JsValue::Null,
            })
        }

        fn get_all as "getAll" (this: JsClass<JsHeaders>, name: Convert<String>, context: &mut Context) -> JsResult<JsValue> {
            let name = to_header_name(&name.0)?;
            let arr = JsArray::new(context);
            for (_, v) in this.borrow().headers.borrow().iter().filter(|(k, _)| name.eq(k)) {
                if let Ok(v) = v.to_str() {
                    arr.push(JsValue::String(js_string!(v)), context)?;
                }
            }
            Ok(arr.into())
        }

        fn set(this: JsClass<JsHeaders>, name: Convert<String>, value: JsValue, context: &mut Context) -> JsResult<()> {
            let name = to_header_name(&name.0)?;
            let this = this.borrow();
            let mut list = this.headers.borrow_mut();
            list.remove(&name);
            if !value.is_null() && !value.is_undefined() {
                let js_string = value.to_string(context)?;
                let a = js_string.to_std_string_lossy();
                let value = to_header_value(&a)?;
                list.insert(name, value);
            }
            Ok(())
        }

        fn append(this: JsClass<JsHeaders>, name: Convert<String>, value: Convert<String>) -> JsResult<()> {
            let name = to_header_name(&name.0)?;
            let value = to_header_value(&value.0)?;
            this.borrow().headers.borrow_mut().append(name, value);
            Ok(())
        }

        fn delete(this: JsClass<JsHeaders>, name: Convert<String>) -> JsResult<()> {
            let name = to_header_name(&name.0)?;
            let this = this.borrow();
            let mut list = this.headers.borrow_mut();
            list.remove(name);
            Ok(())
        }

        fn has(this: JsClass<JsHeaders>, name: Convert<String>) -> JsResult<bool> {
            let name : HeaderName = name.0.clone().parse().map_err(|e| js_error!(TypeError: "Invalid header name: {}", e))?;
            let has = this.borrow().headers
                .borrow()
                .iter()
                .any(|(k, _)| k.eq(&name));
            Ok(has)
        }
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;
    use boa_engine::{Context, JsValue, Source};

    fn setup() -> Context {
        let mut ctx = Context::default();
        ctx.eval(Source::from_bytes(
            r#"
            function must(cond, msg) { if (!cond) throw new Error(msg || "assert failed"); }
            "#,
        ))
        .unwrap();
        ctx.register_global_class::<JsHeaders>()
            .expect("register Headers");
        ctx
    }

    #[test]
    fn headers_constructor_creates_empty_map() {
        let mut ctx = setup();
        let ok = ctx
            .eval(Source::from_bytes(
                r#"
                const h = new Headers();
                must(typeof h === "object", "Headers should construct an object");
                // brand new: shouldn't have host
                must(h.has("host") === false, "no host by default");
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
            must(h.get("X-Trace") === "abc123", "set/get roundtrip");
            must(h.has("X-Trace") === true, "has after set");
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
            must(Array.isArray(all), "getAll returns array");
            must(all.length === 2, "two values");
            // Order is insertion order for http::HeaderMap iteration, which is typically the order inserted.
            must(all.includes("a=1"), "contains a=1");
            must(all.includes("b=2"), "contains b=2");

            // get() returns a single value (first)
            const first = h.get("set-cookie");
            must(first === "a=1" || first === "b=2", "get returns one of values");
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
            must(h.has("X-Foo") === true, "precondition: has X-Foo");
            h.delete("X-Foo");
            must(h.has("X-Foo") === false, "deleted all X-Foo");
            must(h.get("X-Foo") === null, "get null after delete");
            const all = h.getAll("X-Foo");
            must(Array.isArray(all) && all.length === 0, "getAll empty after delete");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn case_insensitive_names() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const h = new Headers();
            h.set("content-type", "text/plain");
            must(h.has("Content-Type") === true, "has is case-insensitive");
            must(h.get("CONTENT-TYPE") === "text/plain", "get is case-insensitive");
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
            must(h.get("X-Num") === "123", "number coerces to string");

            h.set("X-BoolTrue", true);
            must(h.get("X-BoolTrue") === "true", "boolean true coerces");

            h.set("X-BoolFalse", false);
            must(h.get("X-BoolFalse") === "false", "boolean false coerces");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn invalid_header_name_throws_type_error() {
        let mut ctx = setup();
        let res = ctx.eval(Source::from_bytes(
            r#"
            try {
                const h = new Headers();
                h.set("Bad Name", "x"); // invalid due to space
                must(false, "expected TypeError");
            } catch (e) {
                must(e instanceof TypeError, "TypeError for invalid header name");
                true
            }
            "#,
        ));
        assert!(matches!(res, Ok(JsValue::Boolean(true))));
    }

    #[test]
    fn invalid_header_value_throws_type_error() {
        let mut ctx = setup();
        let res = ctx.eval(Source::from_bytes(
            r#"
            try {
                const h = new Headers();
                h.set("X", "line1\r\nline2"); // CRLF not allowed
                must(false, "expected TypeError");
            } catch (e) {
                must(e instanceof TypeError, "TypeError for invalid header value");
                true
            }
            "#,
        ));
        assert!(matches!(res, Ok(JsValue::Boolean(true))));
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
            must(Array.isArray(all) && all.length === 2, "both values under case-insensitive key");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn has_false_when_absent() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const h = new Headers();
            must(h.has("Not-There") === false, "missing returns false");
            must(h.get("Not-There") === null, "get null when missing");
            const all = h.getAll("Not-There");
            must(Array.isArray(all) && all.length === 0, "empty getAll when missing");
            "#,
        ))
        .unwrap();
    }
}
