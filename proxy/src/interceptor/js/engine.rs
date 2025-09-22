use std::{cell::RefCell, rc::Rc, str::FromStr};

use boa_engine::{
    Context, JsObject, JsResult, JsValue, NativeFunction, Source,
    class::Class,
    js_error, js_string,
    object::{FunctionObjectBuilder, builtins::JsArray},
    property::Attribute,
};
use boa_runtime::Console;
use bytes::Bytes;
use http::HeaderMap;
use roxy_shared::uri::RUri;
use tracing::{debug, error, trace};

use crate::{
    flow::{InterceptedRequest, InterceptedResponse},
    interceptor::{
        Error, FlowNotify, KEY_INTERCEPT_REQUEST, KEY_INTERCEPT_RESPONSE, KEY_NOTIFY, KEY_START,
        KEY_STOP, RoxyEngine,
        js::{
            body::JsBody, flow::JsFlow, headers::JsHeaders, logger::JsLogger,
            query::UrlSearchParams, request::JsRequest, response::JsResponse, url::JsUrl,
        },
    },
};
use tokio::sync::{mpsc, oneshot};

struct ReqCmd {
    req: InterceptedRequest,
    resp: oneshot::Sender<Result<(InterceptedRequest, Option<InterceptedResponse>), Error>>,
}

impl ReqCmd {
    fn new(
        req: InterceptedRequest,
        resp: oneshot::Sender<Result<(InterceptedRequest, Option<InterceptedResponse>), Error>>,
    ) -> Box<Self> {
        Box::new(ReqCmd { req, resp })
    }
}

struct ResCmd {
    req: InterceptedRequest,
    res: InterceptedResponse,
    resp: oneshot::Sender<Result<InterceptedResponse, Error>>,
}

impl ResCmd {
    fn new(
        req: InterceptedRequest,
        res: InterceptedResponse,
        resp: oneshot::Sender<Result<InterceptedResponse, Error>>,
    ) -> Box<Self> {
        Box::new(ResCmd { req, res, resp })
    }
}

struct ScriptCmd {
    script: String,
    resp: oneshot::Sender<Result<(), Error>>,
}

impl ScriptCmd {
    fn new(script: String, resp: oneshot::Sender<Result<(), Error>>) -> Box<Self> {
        Box::new(ScriptCmd { script, resp })
    }
}

struct StopCmd {
    resp: oneshot::Sender<Result<(), Error>>,
}

impl StopCmd {
    fn new(resp: oneshot::Sender<Result<(), Error>>) -> Box<Self> {
        Box::new(StopCmd { resp })
    }
}

enum Cmd {
    InterceptReq { data: Box<ReqCmd> },
    InterceptRes { data: Box<ResCmd> },
    SetScript { data: Box<ScriptCmd> },
    OnStop { data: Box<StopCmd> },
}

pub(crate) fn register_classes(ctx: &mut Context) -> JsResult<()> {
    Console::register_with_logger(ctx, JsLogger {})?;
    ctx.register_global_class::<UrlSearchParams>()?;
    ctx.register_global_class::<JsUrl>()?;
    ctx.register_global_class::<JsBody>()?;
    ctx.register_global_class::<JsFlow>()?;
    ctx.register_global_class::<JsRequest>()?;
    ctx.register_global_class::<JsResponse>()?;
    ctx.register_global_class::<JsHeaders>()?;
    Ok(())
}

#[derive(Clone)]
pub struct JsEngine {
    tx: mpsc::Sender<Cmd>,
}

impl JsEngine {
    pub fn new(notify_tx: Option<mpsc::Sender<FlowNotify>>) -> Self {
        let (tx, mut rx) = mpsc::channel::<Cmd>(128);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();

            let mut ctx = Context::default();

            if let Err(e) = register_classes(&mut ctx) {
                error!("Error register_classes {e}");
            }

            let notify_fn = FunctionObjectBuilder::new(ctx.realm(), unsafe {
                NativeFunction::from_closure(move |_this, args, ctx| -> JsResult<JsValue> {
                    let level = args
                        .first()
                        .cloned()
                        .unwrap_or_default()
                        .to_i32(ctx)
                        .unwrap_or(0);

                    let msg = args
                        .get(1)
                        .cloned()
                        .unwrap_or_default()
                        .to_string(ctx)?
                        .to_std_string_escaped();

                    if let Some(tx) = notify_tx.as_ref() {
                        let _ = tx.try_send(FlowNotify {
                            level: level.into(),
                            msg,
                        });
                    }
                    Ok(JsValue::Undefined)
                })
            })
            .length(2)
            .name(js_string!(KEY_NOTIFY))
            .build();

            if let Err(err) = ctx.register_global_property(
                js_string!(KEY_NOTIFY),
                notify_fn,
                Attribute::WRITABLE | Attribute::NON_ENUMERABLE | Attribute::CONFIGURABLE,
            ) {
                error!("Error register_global_property {err}");
            }

            let write_file_fn = FunctionObjectBuilder::new(ctx.realm(), unsafe {
                NativeFunction::from_closure(move |_this, args, ctx| -> JsResult<JsValue> {
                    let path = args
                        .first()
                        .ok_or(js_error!("No path provided"))?
                        .to_string(ctx)?
                        .to_std_string_escaped();
                    let data = args
                        .get(1)
                        .ok_or(js_error!("No data provided"))?
                        .to_string(ctx)?
                        .to_std_string_escaped();
                    std::fs::write(&path, data)
                        .map(|_| JsValue::undefined())
                        .map_err(|e| js_error!("error writing file {}", e))
                })
            })
            .length(2)
            .name("writeFile")
            .build();

            if let Err(err) = ctx.register_global_property(
                js_string!("writeFile"),
                write_file_fn,
                Attribute::WRITABLE | Attribute::NON_ENUMERABLE | Attribute::CONFIGURABLE,
            ) {
                error!("Error register_global_property {err}");
            }

            if let Ok(rt) = rt {
                rt.block_on(async move {
                    while let Some(cmd) = rx.recv().await {
                        match cmd {
                            Cmd::InterceptReq { data } => {
                                let result = handle_intercept_req(&mut ctx, data.req).await;
                                let _ = data.resp.send(result);
                            }
                            Cmd::InterceptRes { data } => {
                                let result =
                                    handle_intercept_resp(&mut ctx, data.req, data.res).await;
                                let _ = data.resp.send(result);
                            }
                            Cmd::SetScript { data } => {
                                if let Err(e) = ctx.create_realm() {
                                    error!("Error creating JS realm {e}");
                                }
                                let result = ctx.eval(Source::from_bytes(data.script.as_bytes()));
                                if let Err(e) = &result {
                                    error!("Script error {e}");
                                };
                                if let Err(e) = run_start_handles(&mut ctx) {
                                    error!("Error running start handles {e}");
                                }

                                let _ = data
                                    .resp
                                    .send(result.map(|_| ()).map_err(|_| Error::LoadError));
                            }
                            Cmd::OnStop { data } => {
                                on_stop(&mut ctx).await.unwrap_or_else(|e| {
                                    error!("Error running stop handles {e}");
                                });
                                let _ = data.resp.send(Ok(()));
                            }
                        }
                    }
                });
            }
        });

        JsEngine { tx }
    }
}

impl Default for JsEngine {
    fn default() -> Self {
        Self::new(None)
    }
}

pub async fn handle_intercept_req(
    ctx: &mut Context,
    req: InterceptedRequest,
) -> Result<(InterceptedRequest, Option<InterceptedResponse>), Error> {
    debug!("handle_intercept_req");

    let header_cell = Rc::new(RefCell::new(req.headers.clone()));
    let trailers_cell = Rc::new(RefCell::new(req.trailers.clone().unwrap_or_default()));

    let body = JsBody::new(req.body.clone());
    let req_cell = Rc::new(RefCell::new(req));
    let resp_cell = Rc::new(RefCell::new(None));
    let url_cell: Rc<RefCell<Option<JsObject>>> = Rc::new(RefCell::new(None));

    let req_handle = Rc::clone(&req_cell);
    let header_handle = Rc::clone(&header_cell);
    let url_handle = Rc::clone(&url_cell);
    let trailers_handle = Rc::clone(&trailers_cell);

    let request = JsRequest {
        req: req_cell,
        body: body.clone(),
        url_obj: url_cell,
        headers: header_cell,
        trailers: trailers_cell,
    };
    let response = JsResponse {
        resp: resp_cell,
        body: JsBody::new(Bytes::new()),
        headers: Rc::new(RefCell::new(HeaderMap::default())),
        trailers: Rc::new(RefCell::new(HeaderMap::default())),
    };
    let flow = JsFlow {
        request,
        response: response.clone(),
    };

    let proto = crate::interceptor::js::util::class_proto(ctx, JsFlow::NAME)
        .map_err(|_| Error::InterceptedRequest)?;
    let js_flow_obj = JsObject::from_proto_and_data(proto, flow);

    let flow_arg = JsValue::Object(js_flow_obj.clone());

    let _ = run_request_handlers(ctx, flow_arg);
    let trailers = {
        let m = trailers_handle.borrow().clone();
        if m.is_empty() { None } else { Some(m) }
    };

    let url = url_handle.borrow_mut().take();
    let mut final_req = req_handle.borrow().clone();
    final_req.body = body.inner.borrow().clone();
    final_req.headers = header_handle.borrow().clone();
    final_req.trailers = trailers;
    if let Some(uri) = url.and_then(|u| u.downcast::<JsUrl>().ok()).and_then(|u| {
        let url_ref = u.borrow();
        let value = url_ref.data().to_string();
        RUri::from_str(&value).ok()
    }) {
        final_req.uri = uri;
    }
    let final_resp = response.into_intercepted();

    Ok((final_req, final_resp))
}

fn get_extensions(ctx: &mut Context) -> JsResult<JsArray> {
    let ext_val = ctx.global_object().get(js_string!("extensions"), ctx)?;
    let Some(ext_obj) = ext_val.as_object() else {
        return Err(js_error!(TypeError: "`extensions` must be an Array"));
    };

    let ext_arr = JsArray::from_object(ext_obj.clone())
        .map_err(|_| js_error!(TypeError: "`extensions` must be an Array"))?;
    Ok(ext_arr)
}

fn run_request_handlers(ctx: &mut Context, flow_arg: JsValue) -> JsResult<()> {
    let ext_arr = get_extensions(ctx)?;

    let len = ext_arr.length(ctx)?;
    for i in 0..len {
        let addon = ext_arr.get(i, ctx)?;
        if addon.is_undefined() || addon.is_null() {
            continue;
        }
        if let Err(err) = call_method_if_callable(
            ctx,
            &addon,
            KEY_INTERCEPT_REQUEST,
            std::slice::from_ref(&flow_arg),
        ) {
            error!("Error invoking request: {err}");
        }
    }

    Ok(())
}

fn run_start_handles(ctx: &mut Context) -> JsResult<()> {
    let ext_arr = get_extensions(ctx)?;

    let len = ext_arr.length(ctx)?;
    for i in 0..len {
        let addon = ext_arr.get(i, ctx)?;
        if addon.is_undefined() || addon.is_null() {
            continue;
        }
        if let Err(err) = call_method_if_callable(ctx, &addon, KEY_START, &[]) {
            error!("Error invoking request: {err}");
        }
    }

    Ok(())
}

fn call_method_if_callable(
    ctx: &mut Context,
    this: &JsValue,
    name: &str,
    args: &[JsValue],
) -> JsResult<()> {
    let Some(obj) = this.as_object() else {
        return Ok(());
    };
    let method = obj.get(js_string!(name), ctx)?;
    if let Some(fun) = method.as_callable() {
        let _ = fun.call(this, args, ctx)?;
    }
    Ok(())
}

async fn on_stop(ctx: &mut Context) -> JsResult<()> {
    let ext_arr = get_extensions(ctx)?;
    let len = ext_arr.length(ctx)?;
    for i in 0..len {
        let addon = ext_arr.get(i, ctx)?;
        if addon.is_undefined() || addon.is_null() {
            continue;
        }
        if let Err(err) = call_method_if_callable(ctx, &addon, KEY_STOP, &[]) {
            error!("Error invoking on: {err}");
        }
    }
    Ok(())
}

async fn handle_intercept_resp(
    ctx: &mut Context,
    req: InterceptedRequest,
    res: InterceptedResponse,
) -> Result<InterceptedResponse, Error> {
    trace!("handle_intercept_req");
    let header_cell = Rc::new(RefCell::new(res.headers.clone()));
    let body = JsBody::new(res.body.clone());
    let trailers_cell = Rc::new(RefCell::new(res.trailers.clone().unwrap_or_default()));
    let req_cell = Rc::new(RefCell::new(req));
    let resp_cell = Rc::new(RefCell::new(Some(res)));

    let trailer_handle = Rc::clone(&trailers_cell);
    let resp_handle = Rc::clone(&resp_cell);

    let request = JsRequest {
        req: req_cell,
        body: JsBody::new(Bytes::new()),
        url_obj: Rc::new(RefCell::new(None)),
        headers: Rc::new(RefCell::new(HeaderMap::default())),
        trailers: Rc::new(RefCell::new(HeaderMap::default())),
    };
    let response = JsResponse {
        resp: resp_cell,
        body: body.clone(),
        headers: header_cell.clone(),
        trailers: trailers_cell,
    };
    let flow = JsFlow { request, response };

    let proto = crate::interceptor::js::util::class_proto(ctx, JsFlow::NAME)
        .map_err(|_| Error::InterceptedRequest)?;
    let js_flow_obj = JsObject::from_proto_and_data(proto, flow);
    let flow_arg = JsValue::Object(js_flow_obj.clone());

    let _ = run_response_handlers(ctx, flow_arg);
    let trailers = {
        let m = trailer_handle.borrow().clone();
        if m.is_empty() { None } else { Some(m) }
    };
    let mut final_resp = resp_handle.borrow().clone().unwrap_or_default();
    final_resp.body = body.inner.borrow().clone();
    final_resp.headers = header_cell.borrow().clone();
    final_resp.trailers = trailers;

    Ok(final_resp)
}

fn run_response_handlers(ctx: &mut Context, flow_arg: JsValue) -> JsResult<()> {
    let ext_arr = get_extensions(ctx)?;

    let len = ext_arr.length(ctx)?;
    for i in 0..len {
        let addon = ext_arr.get(i, ctx)?;
        if addon.is_undefined() || addon.is_null() {
            continue;
        }
        if let Err(err) = call_method_if_callable(
            ctx,
            &addon,
            KEY_INTERCEPT_RESPONSE,
            std::slice::from_ref(&flow_arg),
        ) {
            error!("Error invoking response: {err}");
        }
    }
    Ok(())
}

#[async_trait::async_trait]
impl RoxyEngine for JsEngine {
    async fn intercept_request(
        &self,
        req: &mut InterceptedRequest,
    ) -> Result<Option<InterceptedResponse>, Error> {
        debug!("JS engine intercept_request");
        let (txr, rxr) = oneshot::channel();
        self.tx
            .send(Cmd::InterceptReq {
                data: ReqCmd::new(req.clone(), txr),
            })
            .await
            .map_err(|_| Error::InterceptedRequest)?;
        if let Ok(resdto) = rxr.await.map_err(|_| Error::InterceptedRequest)? {
            req.version = resdto.0.version;
            req.headers = resdto.0.headers;
            req.trailers = resdto.0.trailers;
            req.uri = resdto.0.uri;
            req.method = resdto.0.method;
            req.body = resdto.0.body;
            Ok(resdto.1)
        } else {
            Ok(None)
        }
    }

    async fn intercept_response(
        &self,
        req: &InterceptedRequest,
        res: &mut InterceptedResponse,
    ) -> Result<(), Error> {
        let (txr, rxr) = oneshot::channel();
        self.tx
            .send(Cmd::InterceptRes {
                data: ResCmd::new(req.clone(), res.clone(), txr),
            })
            .await
            .map_err(|_| Error::InterceptResponse)?;
        let resp = rxr.await.map_err(|_| Error::InterceptResponse)??;

        res.version = resp.version;
        res.headers = resp.headers;
        res.trailers = resp.trailers;
        res.status = resp.status;
        res.body = resp.body;
        Ok(())
    }

    async fn set_script(&self, script: &str) -> Result<(), Error> {
        let (txr, rxr) = oneshot::channel();
        self.tx
            .send(Cmd::SetScript {
                data: ScriptCmd::new(script.to_string(), txr),
            })
            .await
            .map_err(|_| Error::LoadError)?;
        let _resp = rxr
            .await
            .map_err(|_| Error::LoadError)?
            .map_err(|_| Error::LoadError)?;
        Ok(())
    }

    async fn on_stop(&self) -> Result<(), Error> {
        let (txr, rxr) = oneshot::channel();
        self.tx
            .send(Cmd::OnStop {
                data: StopCmd::new(txr),
            })
            .await
            .map_err(|_| Error::LoadError)?;
        let _resp = rxr
            .await
            .map_err(|_| Error::LoadError)?
            .map_err(|_| Error::LoadError)?;
        Ok(())
    }
}
