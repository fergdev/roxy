use std::{cell::RefCell, rc::Rc, str::FromStr};

use boa_engine::{
    Context, JsObject, JsResult, JsString, JsValue, class::Class, js_error, js_string,
};
use boa_interop::{JsClass, js_class};
use roxy_shared::version::HttpVersion;

use crate::{
    flow::InterceptedRequest,
    interceptor::js::{
        body::JsBody,
        headers::{HeaderList, JsHeaders},
        url::JsUrl,
    },
};

#[derive(Debug, Clone, boa_engine::Trace, boa_engine::Finalize, boa_engine::JsData)]
#[boa_gc(unsafe_no_drop)]
pub(crate) struct JsRequest {
    #[unsafe_ignore_trace]
    pub(crate) req: Rc<RefCell<InterceptedRequest>>,
    #[unsafe_ignore_trace]
    pub(crate) body: JsBody,
    #[unsafe_ignore_trace]
    pub(crate) url_obj: Rc<RefCell<Option<JsObject>>>,
    #[unsafe_ignore_trace]
    pub(crate) headers: HeaderList,
    #[unsafe_ignore_trace]
    pub(crate) trailers: HeaderList,
}

impl Default for JsRequest {
    fn default() -> Self {
        Self {
            req: Rc::new(RefCell::new(InterceptedRequest::default())),
            body: JsBody::default(),
            url_obj: Rc::new(RefCell::new(None)),
            headers: HeaderList::default(),
            trailers: HeaderList::default(),
        }
    }
}

impl JsRequest {
    fn ensure_url(&self, ctx: &mut Context) -> JsResult<JsObject> {
        if let Some(o) = self.url_obj.borrow().clone() {
            return Ok(o);
        }
        let o = make_url_for_request(ctx, &self.req.borrow())?;
        *self.url_obj.borrow_mut() = Some(o.clone());
        Ok(o)
    }
}

fn make_url_for_request(ctx: &mut Context, req: &InterceptedRequest) -> JsResult<JsObject> {
    let href = req.uri.to_string();
    let base = "http://localhost";

    let url_ctor = ctx.global_object().get(js_string!(JsUrl::NAME), ctx)?;
    let url_obj = url_ctor
        .as_object()
        .ok_or_else(|| js_error!("URL constructor missing"))?
        .construct(
            &[
                JsValue::String(js_string!(href)),
                JsValue::String(js_string!(base)),
            ],
            None,
            ctx,
        )?;

    Ok(url_obj)
}

js_class! {
    class JsRequest as "Request" {
        property method {
            fn get(this: JsClass<JsRequest>) -> JsString {
                js_string!(this.borrow().req.borrow().method.to_string())
            }

            fn set(this: JsClass<JsRequest>, value: JsValue, context: &mut Context) -> JsResult<()> {
                if value.is_string() {
                    let s = value.to_string(context)?.to_std_string_escaped();
                    let m = http::Method::from_str(&s)
                        .map_err(|e| js_error!(TypeError: "Invalid method: {}", e))?;
                    this.borrow().req.borrow_mut().method = m;
                    return Ok(());
                }
                Err(js_error!(TypeError: "Request.method must be a string"))
            }
        }
        property version {
            fn get(this: JsClass<JsRequest>) -> JsString {
                 js_string!(this.borrow().req.borrow().version.to_string())
            }

            fn set(this: JsClass<JsRequest>, value: JsValue, context: &mut Context) -> JsResult<()> {
                if value.is_string() {
                    let version : HttpVersion = value.to_string(context)?.to_std_string_escaped().parse()
                        .map_err(|_| js_error!(TypeError: "Invalid HTTP version"))?;
                    this.borrow().req.borrow_mut().version = version;
                    return Ok(());
                }
                Err(js_error!(TypeError: "Request.method must be a string"))
            }
        }

        property headers {
            fn get(this: JsClass<JsRequest>, context: &mut Context) -> JsResult<JsValue> {
                let proto = crate::interceptor::js::util::class_proto(context, JsHeaders::NAME)?;
                let h = JsHeaders { headers: this.borrow().headers.clone() };
                let obj = JsObject::from_proto_and_data(proto, h);
                Ok(JsValue::Object(obj))
            }
        }

        property trailers {
            fn get(this: JsClass<JsRequest>, context: &mut Context) -> JsResult<JsValue> {
                let proto = crate::interceptor::js::util::class_proto(context, JsHeaders::NAME)?;
                let h = JsHeaders { headers: this.borrow().trailers.clone() };
                let obj = JsObject::from_proto_and_data(proto, h);
                Ok(JsValue::Object(obj))
            }
        }

        property body {
            fn get(this: JsClass<JsRequest>, context: &mut Context) -> JsResult<JsValue> {
                let proto = crate::interceptor::js::util::class_proto(context, JsBody::NAME)?;
                let h = this.borrow().body.clone();
                let obj = JsObject::from_proto_and_data(proto, h);
                Ok(JsValue::Object(obj))
            }
        }
        property url {
            fn get(this: JsClass<JsRequest>, context: &mut Context) -> JsResult<JsValue> {
                let url_obj = this.borrow().ensure_url(context)?;
                Ok(JsValue::Object(url_obj))
            }

            fn set(this: JsClass<JsRequest>, value: JsValue, context: &mut Context) -> JsResult<()> {
                if let Some(o) = value.as_object() {
                    *this.borrow().url_obj.borrow_mut() = Some(o.clone());
                    return Ok(());
                }

                if value.is_string() {
                    let href = value.to_string(context)?.to_std_string_escaped();
                    let url_obj = {
                        let base = "http://localhost";
                        let url_ctor = context.global_object().get(js_string!(JsUrl::NAME), context)?;
                        url_ctor.as_object()
                            .ok_or_else(|| js_error!("URL constructor missing"))?
                            .construct(&[
                                JsValue::String(js_string!(href)),
                                JsValue::String(js_string!(base)),
                            ], None, context)?
                    };
                    *this.borrow().url_obj.borrow_mut() = Some(url_obj);
                    return Ok(());
                }

                Err(js_error!(TypeError: "Request.url must be a URL or string"))
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
    use crate::interceptor::js::tests::setup;
    use boa_engine::{JsValue, Source};

    #[test]
    fn request_constructor_default_succeeds() {
        let mut ctx = setup();
        let ok = ctx
            .eval(Source::from_bytes(
                r#"
                const r = new Request();
                assertTrue(typeof r === "object", "Request should construct an object");
                // default method and version are readable (format, not asserting exact)
                assertTrue(typeof r.method === "string", "method is string");
                assertTrue(typeof r.version === "string", "version is string");
                true
            "#,
            ))
            .unwrap();
        assert!(ok.is_boolean() && ok.as_boolean().unwrap());
    }

    #[test]
    fn request_method_set_get_roundtrip() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const r = new Request();
            r.method = "POST";
            assertEqual(r.method, "POST", "method roundtrip");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_method_invalid_type_throws() {
        let mut ctx = setup();
        let res = ctx.eval(Source::from_bytes(
            r#"
            try {
              const r = new Request();
              r.method = 123; // not a string
              assertTrue(false, "expected TypeError");
            } catch (e) {
              assertTrue(e instanceof TypeError, "TypeError on non-string method");
              true
            }
            "#,
        ));
        assert!(matches!(res, Ok(JsValue::Boolean(true))));
    }

    #[test]
    fn request_method_invalid_value_throws() {
        let mut ctx = setup();
        let res = ctx.eval(Source::from_bytes(
            r#"
            try {
              const r = new Request();
              r.method = " NOT_A_METHOD ";
              assertTrue(false, "expected TypeError for invalid method");
            } catch (e) {
              assertTrue(e instanceof TypeError, "TypeError for invalid method");
              true
            }
            "#,
        ));
        assert!(matches!(res, Ok(JsValue::Boolean(true))));
    }

    #[test]
    fn request_version_set_get_roundtrip() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const r = new Request();
            r.version = "HTTP/2.0";
            assertEqual(r.version, "HTTP/2.0", "version roundtrip to string format");
            r.version = "HTTP/1.1";
            assertEqual(r.version, "HTTP/1.1", "version downgraded ok");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_version_rejects_bad_values() {
        let mut ctx = setup();
        let res = ctx.eval(Source::from_bytes(
            r#"
            try {
              const r = new Request();
              r.version = "HTTP/9.9";
              assertTrue(false, "expected TypeError for unsupported version");
            } catch (e) {
              assertTrue(e instanceof TypeError, "TypeError for bad version");
              true
            }
            "#,
        ));
        assert!(matches!(res, Ok(JsValue::Boolean(true))));
    }

    #[test]
    fn request_headers_returns_headers_object() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const r = new Request();
            const h = r.headers;
            assertTrue(h && typeof h === "object", "headers is object");
            h.set("X-Test", "1");
            assertEqual(h.get("X-Test"), "1", "headers.get after set");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_trailers_returns_headers_object() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const r = new Request();
            const t = r.trailers;
            assertTrue(t && typeof t === "object", "trailers is object");
            t.append("X-Trailer", "A");
            t.append("X-Trailer", "B");
            const all = t.getAll("X-Trailer");
            assertTrue(Array.isArray(all) && all.length === 2, "two trailer values");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_body_returns_body_object_and_roundtrips_text() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const r = new Request();
            const b = r.body;
            assertTrue(b && typeof b === "object", "body is object");
            b.text = "hello";
            assertEqual(b.text, "hello", "body text roundtrip");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_url_getter_returns_url_object() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const r = new Request();
            const u = r.url;
            assertTrue(u && typeof u === "object", "url is object");
            // Not asserting exact fields, just that it behaves like URL (has href/toString)
            assertTrue(typeof u.href === "string", "url.href is string");
            assertTrue(typeof u.toString === "function", "url.toString exists");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_url_set_accepts_string() {
        let mut ctx = setup();

        ctx.eval(Source::from_bytes(
            r#"
            const r = new Request();
            r.url = "http://example.com/path?x=1";
            const u1 = r.url;
            assertTrue(u1 && typeof u1 === "object", "url object after string set");
            assertTrue(typeof u1.href === "string", "url.href exists");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_url_set_accepts_url_object() {
        let mut ctx = setup();

        let ok = ctx
            .eval(Source::from_bytes(
                r#"
                const u = new URL("http://localhost/base", "http://localhost");
                const r = new Request();
                r.url = u;    // assign the object
                const got = r.url;
                assertTrue(got && typeof got === "object", "url is object after object set");
                true
            "#,
            ))
            .unwrap();
        assert!(ok.is_boolean() && ok.as_boolean().unwrap());
    }

    #[test]
    fn request_url_set_invalid_type_throws() {
        let mut ctx = setup();
        let res = ctx.eval(Source::from_bytes(
            r#"
            try {
              const r = new Request();
              r.url = 42; // not a URL object and not a string
              assertTrue(false, "expected TypeError");
            } catch (e) {
              assertTrue(e instanceof TypeError, "TypeError for invalid url assignment");
              true
            }
            "#,
        ));
        assert!(matches!(res, Ok(JsValue::Boolean(true))));
    }

    #[test]
    fn request_properties_live_views_not_copies() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const r = new Request();
            const h1 = r.headers;
            const h2 = r.headers;
            h1.set("X-Live", "yes");
            assertEqual(h2.get("X-Live"), "yes", "same live headers view");

            const t1 = r.trailers;
            const t2 = r.trailers;
            t1.append("X-Trail", "A");
            assertEqual(t2.has("X-Trail"), true, "same live trailers view");

            const b1 = r.body;
            const b2 = r.body;
            b1.text = "ok";
            assertEqual(b2.text, "ok", "same live body view");
            "#,
        ))
        .unwrap();
    }
}
