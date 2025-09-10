use boa_engine::{Context, JsObject, JsResult, js_error, js_string};

pub(crate) fn class_proto(ctx: &mut Context, ctor_name: &str) -> JsResult<JsObject> {
    let s = js_string!(ctor_name);
    let ctor_val = ctx.global_object().get(s, ctx)?;
    let ctor = ctor_val
        .as_object()
        .ok_or_else(|| js_error!("constructor is not an object"))?;
    let proto_val = ctor.get(js_string!("prototype"), ctx)?;
    let proto = proto_val
        .as_object()
        .ok_or_else(|| js_error!("prototype is not an object"))?;
    Ok(proto.clone())
}
