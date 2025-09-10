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
        url::Url,
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

    let url_ctor = ctx.global_object().get(js_string!(Url::NAME), ctx)?;
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
                        let url_ctor = context.global_object().get(js_string!(Url::NAME), context)?;
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
            Err(js_error!(TypeError: "Illegal constructor"))
        }

        init(_class: &mut ClassBuilder) -> JsResult<()> {
            Ok(())
        }
    }
}
