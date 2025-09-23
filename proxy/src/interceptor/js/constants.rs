use boa_engine::{Context, js_string, object::ObjectInitializer, property::Attribute};
use tracing::error;

pub(crate) fn register_constants(cxt: &mut Context) {
    let object = ObjectInitializer::new(cxt)
        .property(js_string!("GET"), js_string!("GET"), Attribute::all())
        .property(js_string!("POST"), js_string!("POST"), Attribute::all())
        .property(
            js_string!("CONNECT"),
            js_string!("CONNECT"),
            Attribute::all(),
        )
        .property(js_string!("DELETE"), js_string!("DELETE"), Attribute::all())
        .property(js_string!("HEAD"), js_string!("HEAD"), Attribute::all())
        .property(
            js_string!("OPTIONS"),
            js_string!("OPTIONS"),
            Attribute::all(),
        )
        .property(js_string!("PATCH"), js_string!("PATCH"), Attribute::all())
        .property(js_string!("PUT"), js_string!("PUT"), Attribute::all())
        .property(js_string!("TRACE"), js_string!("TRACE"), Attribute::all())
        .build();

    if let Err(err) = cxt.register_global_property(
        js_string!("Method"),
        object,
        Attribute::NON_ENUMERABLE | Attribute::CONFIGURABLE,
    ) {
        error!("Error register_global_property {err}");
    }
    let object = ObjectInitializer::new(cxt)
        .property(js_string!("HTTP"), js_string!("http"), Attribute::all())
        .property(js_string!("HTTPS"), js_string!("https"), Attribute::all())
        .build();

    if let Err(err) = cxt.register_global_property(
        js_string!("Protocol"),
        object,
        Attribute::NON_ENUMERABLE | Attribute::CONFIGURABLE,
    ) {
        error!("Error register_global_property {err}");
    }

    let object = ObjectInitializer::new(cxt)
        .property(
            js_string!("HTTP/0.9"),
            js_string!("HTTP/0.9"),
            Attribute::all(),
        )
        .property(
            js_string!("HTTP1_0"),
            js_string!("HTTP/1.0"),
            Attribute::all(),
        )
        .property(
            js_string!("HTTP1_1"),
            js_string!("HTTP/1.1"),
            Attribute::all(),
        )
        .property(
            js_string!("HTTP2_0"),
            js_string!("HTTP/2.0"),
            Attribute::all(),
        )
        .property(
            js_string!("HTTP3_0"),
            js_string!("HTTP/3.0"),
            Attribute::all(),
        )
        .build();
    if let Err(err) = cxt.register_global_property(
        js_string!("Version"),
        object,
        Attribute::NON_ENUMERABLE | Attribute::CONFIGURABLE,
    ) {
        error!("Error register_global_property {err}");
    }
}
