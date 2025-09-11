use std::{cell::RefCell, ops::Deref, rc::Rc};

use boa_engine::{Context, JsObject, JsResult, JsString, JsValue, js_error, js_string};
use boa_interop::{JsClass, js_class};
use bytes::Bytes;
use http::StatusCode;
use roxy_shared::version::HttpVersion;

use crate::flow::InterceptedResponse;
use crate::interceptor::js::body::JsBody;
use crate::interceptor::js::headers::{HeaderList, JsHeaders};
use crate::interceptor::js::util::class_proto;

#[derive(Debug, Clone, boa_engine::Trace, boa_engine::Finalize, boa_engine::JsData)]
#[boa_gc(unsafe_no_drop)]
pub(crate) struct JsResponse {
    #[unsafe_ignore_trace]
    pub(crate) resp: Rc<RefCell<Option<InterceptedResponse>>>,
    #[unsafe_ignore_trace]
    pub(crate) body: JsBody,
    #[unsafe_ignore_trace]
    pub(crate) headers: HeaderList,
    #[unsafe_ignore_trace]
    pub(crate) trailers: HeaderList,
}

impl Default for JsResponse {
    fn default() -> Self {
        Self {
            resp: Rc::new(RefCell::new(None)),
            body: JsBody {
                inner: Rc::new(RefCell::new(Bytes::new())),
            },
            headers: Rc::new(RefCell::new(http::HeaderMap::new())),
            trailers: Rc::new(RefCell::new(http::HeaderMap::new())),
        }
    }
}

impl JsResponse {
    pub fn into_intercepted(self) -> Option<InterceptedResponse> {
        let mut resp = self.resp.borrow().clone().unwrap_or_default();

        let body_bytes = self.body.inner.borrow();
        if !body_bytes.is_empty() {
            resp.body = body_bytes.clone();
        }

        let headers = self.headers.borrow();
        if !headers.is_empty() {
            resp.headers = headers.clone();
        }

        let trailers = self.trailers.borrow();
        if !trailers.is_empty() {
            resp.trailers = Some(trailers.clone());
        }

        if resp.status != 0
            || !resp.body.is_empty()
            || !resp.headers.is_empty()
            || resp.trailers.is_some()
        {
            Some(resp)
        } else {
            None
        }
    }
}

js_class! {
    class JsResponse as "Response" {
        property version {
            fn get(this: JsClass<JsResponse>) -> JsString {
                let version = if let Some(res) = this.borrow().resp.borrow().deref() {
                    res.version.to_string()
                } else {
                    String::new()
                };
                js_string!(version)
            }

            fn set(this: JsClass<JsResponse>, value: JsValue, context: &mut Context) -> JsResult<()> {
                if value.is_string() {
                    let version : HttpVersion = value.to_string(context)?.to_std_string_escaped().parse()
                        .map_err(|_| js_error!(TypeError: "Invalid HTTP version"))?;
                    let this = this.borrow();
                    let mut opt = this.resp.borrow_mut();
                    let resp = opt.get_or_insert_with(InterceptedResponse::default);
                    resp.version = version;
                    return Ok(());
                }
                Err(js_error!(TypeError: "Request.method must be a string"))
            }
        }
        property headers {
            fn get(this: JsClass<JsResponse>, context: &mut Context) -> JsResult<JsValue> {
                let list = this.borrow().headers.clone();
                JsHeaders::from_data(JsHeaders { headers: list }, context).map(JsValue::from)
            }
        }

        property trailers {
            fn get(this: JsClass<JsResponse>, context: &mut Context) -> JsResult<JsValue> {
                let list = this.borrow().trailers.clone();
                JsHeaders::from_data(JsHeaders { headers: list }, context).map(JsValue::from)
            }
        }

        property status {
            fn get(this: JsClass<JsResponse>) -> JsResult<JsValue> {
                let status = if let Some(res) = this.borrow().resp.borrow().deref() {
                    res.status.as_u16() as i32
                } else {
                    0
                };
                Ok(JsValue::Integer(status))
            }

            fn set(this: JsClass<JsResponse>, value: JsValue, context: &mut Context) -> JsResult<()> {
                let this = this.borrow();
                let mut opt = this.resp.borrow_mut();
                let resp = opt.get_or_insert_with(InterceptedResponse::default);

                let code = if value.is_integer() || value.is_number() {
                    value.to_i32(context)?
                } else {
                    return Err(js_error!(TypeError: "status must be a number"));
                };

                resp.status = StatusCode::from_u16(code as u16)
                    .map_err(|_| js_error!(TypeError: "invalid HTTP status code"))?;
                Ok(())
            }
        }

        property body {
            fn get(this: JsClass<JsResponse>, context: &mut Context) -> JsResult<JsValue> {
                let proto = class_proto(context, JsBody::NAME)?;
                let h = this.borrow().body.clone();
                let obj = JsObject::from_proto_and_data(proto, h);
                Ok(JsValue::Object(obj))
            }
        }
        constructor() {
            Ok(Self::default())
        }

        init(_class: &mut ClassBuilder) -> JsResult<()> {
            Ok(())
        }
    }
}
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::interceptor::js::engine::register_classes;
    use boa_engine::{Context, Source};

    fn setup() -> Context {
        crate::init_test_logging();
        let mut ctx = Context::default();
        ctx.eval(Source::from_bytes(
            r#"
            function must(cond, msg) { if (!cond) throw new Error(msg || "assert failed"); }
            "#,
        ))
        .unwrap();
        register_classes(&mut ctx).expect("register classes");
        ctx
    }

    #[test]
    fn response_constructor_defaults() {
        let mut ctx = setup();

        ctx.eval(Source::from_bytes(
            r#"
            const r = new Response();
            must(typeof r === "object", "Response constructed");

            // default status is 0 (meaning unset)
            must(r.status === 0, "default status is 0");

            // headers/trailers are objects
            must(r.headers && typeof r.headers === "object", "headers object");
            must(r.trailers && typeof r.trailers === "object", "trailers object");

            // body exists and has text/raw accessors via Body
            must(r.body && typeof r.body === "object", "body object");
            must(typeof r.body.text === "string", "body.text readable (string)");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn response_status_set_get_valid() {
        let mut ctx = setup();

        ctx.eval(Source::from_bytes(
            r#"
            const r = new Response();
            r.status = 201;
            must(r.status === 201, "status roundtrip 201");
            r.status = 404;
            must(r.status === 404, "status roundtrip 404");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn response_status_set_string_throws() {
        let mut ctx = setup();

        ctx.eval(Source::from_bytes(
            r#"
            const r = new Response();
            let threw = false;
            try { r.status = "nope"; } catch (e) { threw = true; }
            must(threw, "setting status to non-number must throw");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn response_status_set_out_of_range_throws() {
        let mut ctx = setup();

        ctx.eval(Source::from_bytes(
            r#"
            const r = new Response();
            let threw = false;
            try { r.status = 9999; } catch (e) { threw = true; }
            must(threw, "invalid HTTP status must throw");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn response_version_set_get() {
        let mut ctx = setup();

        ctx.eval(Source::from_bytes(
            r#"
            const r = new Response();
            // initially empty string when no underlying response set
            must(typeof r.version === "string", "version is string");
            must(r.version === "", "default version empty");

            r.version = "HTTP/1.1";
            must(r.version === "HTTP/1.1", "version set/get HTTP/1.1");

            r.version = "HTTP/2.0";
            must(r.version === "HTTP/2.0", "version set/get HTTP/2.0");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn response_headers_set_get() {
        let mut ctx = setup();

        ctx.eval(Source::from_bytes(
            r#"
            const r = new Response();
            const h = r.headers;
            must(typeof h === "object", "headers object");

            // set & get
            h.set("X-Test", "v1");
            must(h.get("X-Test") === "v1", "headers.set/get single value");

            // append additional values and get_all
            h.append("X-Test", "v2");
            const all = h.getAll("X-Test");
            must(Array.isArray(all) && all.length === 2, "headers.getAll returns both values");
            must(all[0] === "v1" && all[1] === "v2", "ordered values as appended");

            // has / delete
            must(h.has("X-Test") === true, "has before delete");
            h.delete("X-Test");
            must(h.has("X-Test") === false, "has after delete");
            must(h.get("X-Test") === null, "get returns null when missing");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn response_trailers_set_get() {
        let mut ctx = setup();

        ctx.eval(Source::from_bytes(
            r#"
            const r = new Response();
            const t = r.trailers;
            must(typeof t === "object", "trailers object");

            t.set("X-Trailer", "tv1");
            must(t.get("X-Trailer") === "tv1", "trailers.set/get");
            t.append("X-Trailer", "tv2");
            const all = t.getAll("X-Trailer");
            must(Array.isArray(all) && all.length === 2, "trailers.getAll collects both");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn response_body_text_and_raw_roundtrip() {
        let mut ctx = setup();

        ctx.eval(Source::from_bytes(
            r#"
            const r = new Response();
            // text path
            r.body.text = "hello";
            must(r.body.text === "hello", "body.text roundtrip");

            // raw path
            const bytes = new Uint8Array([0x61, 0x00, 0x62, 0xFF]); // a \0 b 0xFF
            r.body.raw = bytes.buffer;

            const got = new Uint8Array(r.body.raw);
            must(got.length === 4, "raw length ok");
            must(got[0] === 0x61 && got[1] === 0x00 && got[2] === 0x62 && got[3] === 0xFF, "raw content ok");

            // after raw set, text is still a string (lossy decode is fine)
            must(typeof r.body.text === "string", "text readable after raw set");
            "#,
        ))
        .unwrap();
    }
}
