use boa_engine::{Context, JsValue, js_string, object::ObjectInitializer, property::Attribute};
use tracing::error;

pub(crate) fn register_constants(ctx: &mut Context) {
    let object = ObjectInitializer::new(ctx)
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

    if let Err(err) =
        ctx.register_global_property(js_string!("Method"), object, Attribute::PERMANENT)
    {
        error!("Error register_global_property {err}");
    }
    let object = ObjectInitializer::new(ctx)
        .property(js_string!("HTTP"), js_string!("http"), Attribute::all())
        .property(js_string!("HTTPS"), js_string!("https"), Attribute::all())
        .build();

    if let Err(err) = ctx.register_global_property(
        js_string!("Protocol"),
        object,
        Attribute::NON_ENUMERABLE | Attribute::CONFIGURABLE,
    ) {
        error!("Error register_global_property {err}");
    }

    let object = ObjectInitializer::new(ctx)
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
    if let Err(err) =
        ctx.register_global_property(js_string!("Version"), object, Attribute::PERMANENT)
    {
        error!("Error register_global_property {err}");
    }
    let status = ObjectInitializer::new(ctx)
        .property(js_string!("CONTINUE"), JsValue::from(100), Attribute::all())
        .property(
            js_string!("SWITCHING_PROTOCOLS"),
            JsValue::from(101),
            Attribute::all(),
        )
        .property(
            js_string!("PROCESSING"),
            JsValue::from(102),
            Attribute::all(),
        )
        .property(js_string!("OK"), JsValue::from(200), Attribute::all())
        .property(js_string!("CREATED"), JsValue::from(201), Attribute::all())
        .property(js_string!("ACCEPTED"), JsValue::from(202), Attribute::all())
        .property(
            js_string!("NON_AUTHORITATIVE_INFORMATION"),
            JsValue::from(203),
            Attribute::all(),
        )
        .property(
            js_string!("NO_CONTENT"),
            JsValue::from(204),
            Attribute::all(),
        )
        .property(
            js_string!("RESET_CONTENT"),
            JsValue::from(205),
            Attribute::all(),
        )
        .property(
            js_string!("PARTIAL_CONTENT"),
            JsValue::from(206),
            Attribute::all(),
        )
        .property(
            js_string!("MULTI_STATUS"),
            JsValue::from(207),
            Attribute::all(),
        )
        .property(
            js_string!("ALREADY_REPORTED"),
            JsValue::from(208),
            Attribute::all(),
        )
        .property(js_string!("IM_USED"), JsValue::from(226), Attribute::all())
        .property(
            js_string!("MULTIPLE_CHOICES"),
            JsValue::from(300),
            Attribute::all(),
        )
        .property(
            js_string!("MOVED_PERMANENTLY"),
            JsValue::from(301),
            Attribute::all(),
        )
        .property(js_string!("FOUND"), JsValue::from(302), Attribute::all())
        .property(
            js_string!("SEE_OTHER"),
            JsValue::from(303),
            Attribute::all(),
        )
        .property(
            js_string!("NOT_MODIFIED"),
            JsValue::from(304),
            Attribute::all(),
        )
        .property(
            js_string!("USE_PROXY"),
            JsValue::from(305),
            Attribute::all(),
        )
        .property(
            js_string!("TEMPORARY_REDIRECT"),
            JsValue::from(307),
            Attribute::all(),
        )
        .property(
            js_string!("PERMANENT_REDIRECT"),
            JsValue::from(308),
            Attribute::all(),
        )
        .property(
            js_string!("BAD_REQUEST"),
            JsValue::from(400),
            Attribute::all(),
        )
        .property(
            js_string!("UNAUTHORIZED"),
            JsValue::from(401),
            Attribute::all(),
        )
        .property(
            js_string!("PAYMENT_REQUIRED"),
            JsValue::from(402),
            Attribute::all(),
        )
        .property(
            js_string!("FORBIDDEN"),
            JsValue::from(403),
            Attribute::all(),
        )
        .property(
            js_string!("NOT_FOUND"),
            JsValue::from(404),
            Attribute::all(),
        )
        .property(
            js_string!("METHOD_NOT_ALLOWED"),
            JsValue::from(405),
            Attribute::all(),
        )
        .property(
            js_string!("NOT_ACCEPTABLE"),
            JsValue::from(406),
            Attribute::all(),
        )
        .property(
            js_string!("PROXY_AUTHENTICATION_REQUIRED"),
            JsValue::from(407),
            Attribute::all(),
        )
        .property(
            js_string!("REQUEST_TIMEOUT"),
            JsValue::from(408),
            Attribute::all(),
        )
        .property(js_string!("CONFLICT"), JsValue::from(409), Attribute::all())
        .property(js_string!("GONE"), JsValue::from(410), Attribute::all())
        .property(
            js_string!("LENGTH_REQUIRED"),
            JsValue::from(411),
            Attribute::all(),
        )
        .property(
            js_string!("PRECONDITION_FAILED"),
            JsValue::from(412),
            Attribute::all(),
        )
        .property(
            js_string!("PAYLOAD_TOO_LARGE"),
            JsValue::from(413),
            Attribute::all(),
        )
        .property(
            js_string!("URI_TOO_LONG"),
            JsValue::from(414),
            Attribute::all(),
        )
        .property(
            js_string!("UNSUPPORTED_MEDIA_TYPE"),
            JsValue::from(415),
            Attribute::all(),
        )
        .property(
            js_string!("RANGE_NOT_SATISFIABLE"),
            JsValue::from(416),
            Attribute::all(),
        )
        .property(
            js_string!("EXPECTATION_FAILED"),
            JsValue::from(417),
            Attribute::all(),
        )
        .property(
            js_string!("IM_A_TEAPOT"),
            JsValue::from(418),
            Attribute::all(),
        )
        .property(
            js_string!("MISDIRECTED_REQUEST"),
            JsValue::from(421),
            Attribute::all(),
        )
        .property(
            js_string!("UNPROCESSABLE_ENTITY"),
            JsValue::from(422),
            Attribute::all(),
        )
        .property(js_string!("LOCKED"), JsValue::from(423), Attribute::all())
        .property(
            js_string!("FAILED_DEPENDENCY"),
            JsValue::from(424),
            Attribute::all(),
        )
        .property(
            js_string!("TOO_EARLY"),
            JsValue::from(425),
            Attribute::all(),
        )
        .property(
            js_string!("UPGRADE_REQUIRED"),
            JsValue::from(426),
            Attribute::all(),
        )
        .property(
            js_string!("PRECONDITION_REQUIRED"),
            JsValue::from(428),
            Attribute::all(),
        )
        .property(
            js_string!("TOO_MANY_REQUESTS"),
            JsValue::from(429),
            Attribute::all(),
        )
        .property(
            js_string!("REQUEST_HEADER_FIELDS_TOO_LARGE"),
            JsValue::from(431),
            Attribute::all(),
        )
        .property(
            js_string!("UNAVAILABLE_FOR_LEGAL_REASONS"),
            JsValue::from(451),
            Attribute::all(),
        )
        .property(
            js_string!("INTERNAL_SERVER_ERROR"),
            JsValue::from(500),
            Attribute::all(),
        )
        .property(
            js_string!("NOT_IMPLEMENTED"),
            JsValue::from(501),
            Attribute::all(),
        )
        .property(
            js_string!("BAD_GATEWAY"),
            JsValue::from(502),
            Attribute::all(),
        )
        .property(
            js_string!("SERVICE_UNAVAILABLE"),
            JsValue::from(503),
            Attribute::all(),
        )
        .property(
            js_string!("GATEWAY_TIMEOUT"),
            JsValue::from(504),
            Attribute::all(),
        )
        .property(
            js_string!("HTTP_VERSION_NOT_SUPPORTED"),
            JsValue::from(505),
            Attribute::all(),
        )
        .property(
            js_string!("VARIANT_ALSO_NEGOTIATES"),
            JsValue::from(506),
            Attribute::all(),
        )
        .property(
            js_string!("INSUFFICIENT_STORAGE"),
            JsValue::from(507),
            Attribute::all(),
        )
        .property(
            js_string!("LOOP_DETECTED"),
            JsValue::from(508),
            Attribute::all(),
        )
        .property(
            js_string!("NOT_EXTENDED"),
            JsValue::from(510),
            Attribute::all(),
        )
        .property(
            js_string!("NETWORK_AUTHENTICATION_REQUIRED"),
            JsValue::from(511),
            Attribute::all(),
        )
        .build();
    if let Err(err) =
        ctx.register_global_property(js_string!("Status"), status, Attribute::PERMANENT)
    {
        error!("Error register_global_property {err}");
    }
}
