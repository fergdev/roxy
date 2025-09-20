use std::env;

use bytes::Bytes;
use http::{HeaderMap, HeaderName, Method, StatusCode};
use roxy_proxy::{
    flow::{InterceptedRequest, InterceptedResponse},
    init_test_logging,
    interceptor::{FlowNotify, FlowNotifyLevel, ScriptEngine, ScriptType},
};
use roxy_shared::alpn::AlpnProtocol;
use strum::IntoEnumIterator;
use time::OffsetDateTime;
use tokio::sync::mpsc;

const SCRIPT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/script_engine");

struct TestContext {
    engine: ScriptEngine,
    default_req: InterceptedRequest,
    default_resp: InterceptedResponse,
}

impl TestContext {
    fn new_engine(engine: ScriptEngine) -> Self {
        init_test_logging();
        let mut headers = HeaderMap::new();
        let mut trailers = HeaderMap::new();
        for i in 1..=3 {
            for j in &["a", "b", "c"] {
                headers.append(
                    format!("X-Header{i}").parse::<HeaderName>().unwrap(),
                    j.parse().unwrap(),
                );
                trailers.append(
                    format!("X-Trailer{i}").parse::<HeaderName>().unwrap(),
                    j.parse().unwrap(),
                );
            }
        }

        let expect_req = InterceptedRequest {
            timestamp: OffsetDateTime::now_utc(),
            uri: "http://localhost".parse().unwrap(),
            alpn: AlpnProtocol::None,
            encoding: None,
            method: http::Method::GET,
            version: http::Version::HTTP_11.into(),
            headers: headers.clone(),
            body: bytes::Bytes::new(),
            trailers: Some(trailers.clone()),
        };

        let expect_resp = InterceptedResponse {
            status: StatusCode::OK,
            timestamp: OffsetDateTime::now_utc(),
            version: http::Version::HTTP_11.into(),
            encoding: None,
            headers,
            body: bytes::Bytes::new(),
            trailers: Some(trailers),
        };
        Self {
            engine,
            default_req: expect_req,
            default_resp: expect_resp,
        }
    }
    async fn new() -> Self {
        TestContext::new_engine(ScriptEngine::new())
    }

    async fn new_with_notify(notify_tx: mpsc::Sender<FlowNotify>) -> Self {
        TestContext::new_engine(ScriptEngine::new_notify(notify_tx))
    }

    async fn load_script(test_prefix: &str, st: ScriptType) -> String {
        let script_path = format!("{}/{}.{}", SCRIPT_DIR, test_prefix, st.ext());
        tokio::fs::read_to_string(&script_path).await.unwrap()
    }

    async fn run_test(
        &mut self,
        test_prefix: &str,
        init_req: &InterceptedRequest,
        expect_req: &InterceptedRequest,
        init_res: &InterceptedResponse,
        expect_res: &InterceptedResponse,
    ) {
        for st in ScriptType::iter() {
            let script = Self::load_script(test_prefix, st).await;
            self.engine.set_script(&script, st).await.unwrap();

            let mut actual_req = init_req.clone();
            self.engine
                .intercept_request(&mut actual_req)
                .await
                .unwrap();
            assert_eq!(expect_req, &actual_req);

            let mut actual_res = init_res.clone();
            self.engine
                .intercept_response(expect_req, &mut actual_res)
                .await
                .unwrap();
            assert_eq!(expect_res, &actual_res);
        }
    }
}

#[tokio::test]
async fn test_empty() {
    let mut cxt = TestContext::new().await;
    let req = cxt.default_req.clone();
    let res = cxt.default_resp.clone();
    cxt.run_test("empty", &req, &req, &res, &res).await;
}

#[tokio::test]
async fn test_void_no_chage() {
    let mut cxt = TestContext::new().await;
    let req = cxt.default_req.clone();
    let res = cxt.default_resp.clone();
    cxt.run_test("void", &req, &req, &res, &res).await;
}

#[tokio::test]
async fn test_no_change_double_headers() {
    let mut cxt = TestContext::new().await;

    let mut headers = HeaderMap::new();
    headers.append("set-cookie", "a".parse().unwrap());
    headers.append("set-cookie", "b".parse().unwrap());
    let init_req = InterceptedRequest {
        headers: headers.clone(),
        ..cxt.default_req.clone()
    };
    let expect_req = init_req.clone();
    let init_res = InterceptedResponse {
        headers: headers.clone(),
        ..cxt.default_resp.clone()
    };
    let expect_res = init_res.clone();
    cxt.run_test("void", &init_req, &expect_req, &init_res, &expect_res)
        .await;
}

#[tokio::test]
async fn test_header_append() {
    let mut cxt = TestContext::new().await;

    let init_req = cxt.default_req.clone();
    let init_res = cxt.default_resp.clone();

    let mut expect_req_headers = init_req.headers.clone();
    expect_req_headers.append("X-Header1", "request".parse().unwrap());
    expect_req_headers.append("X-Header9", "request".parse().unwrap());
    let expect_req = InterceptedRequest {
        headers: expect_req_headers,
        ..cxt.default_req.clone()
    };
    let mut expect_res_headers = init_res.headers.clone();
    expect_res_headers.append("X-Header1", "response".parse().unwrap());
    expect_res_headers.append("X-Header9", "response".parse().unwrap());
    let expect_res = InterceptedResponse {
        headers: expect_res_headers,
        ..cxt.default_resp.clone()
    };
    cxt.run_test(
        "header_append",
        &init_req,
        &expect_req,
        &init_res,
        &expect_res,
    )
    .await;
}

#[tokio::test]
async fn test_header_set() {
    let mut cxt = TestContext::new().await;

    let init_req = cxt.default_req.clone();
    let init_res = cxt.default_resp.clone();

    let mut expect_req_headers = init_req.headers.clone();
    expect_req_headers.remove("X-Header1").unwrap();
    expect_req_headers.append("X-Header1", "request".parse().unwrap());
    let expect_req = InterceptedRequest {
        headers: expect_req_headers,
        ..cxt.default_req.clone()
    };
    let mut expect_res_headers = init_res.headers.clone();
    expect_res_headers.remove("X-Header1").unwrap();
    expect_res_headers.append("X-Header1", "response".parse().unwrap());
    let expect_res = InterceptedResponse {
        headers: expect_res_headers,
        ..cxt.default_resp.clone()
    };
    cxt.run_test("header_set", &init_req, &expect_req, &init_res, &expect_res)
        .await;
}

#[tokio::test]
async fn test_header_delete() {
    let mut cxt = TestContext::new().await;

    let init_req = cxt.default_req.clone();
    let init_res = cxt.default_resp.clone();
    let mut req_headers = init_req.headers.clone();
    req_headers.remove("X-Header1");
    req_headers.remove("X-Header2");
    req_headers.remove("X-Header3");

    let expect_req = InterceptedRequest {
        headers: req_headers,
        ..cxt.default_req.clone()
    };
    let mut res_headers = init_res.headers.clone();
    res_headers.remove("X-Header1");
    res_headers.remove("X-Header2");
    res_headers.remove("X-Header3");
    let expect_res = InterceptedResponse {
        headers: res_headers,
        ..cxt.default_resp.clone()
    };

    cxt.run_test(
        "header_delete",
        &init_req,
        &expect_req,
        &init_res,
        &expect_res,
    )
    .await;
}

#[tokio::test]
async fn test_trailer_append() {
    let mut cxt = TestContext::new().await;

    let init_req = cxt.default_req.clone();
    let init_res = cxt.default_resp.clone();

    let mut req_trailers = init_req.trailers.clone().unwrap();
    req_trailers.append("X-Trailer1", "request".parse().unwrap());
    req_trailers.append("X-Trailer9", "request".parse().unwrap());
    let expect_req = InterceptedRequest {
        trailers: Some(req_trailers),
        ..cxt.default_req.clone()
    };
    let mut res_trailers = init_res.trailers.clone().unwrap();
    res_trailers.append("X-Trailer1", "response".parse().unwrap());
    res_trailers.append("X-Trailer9", "response".parse().unwrap());
    let expect_res = InterceptedResponse {
        trailers: Some(res_trailers),
        ..cxt.default_resp.clone()
    };
    cxt.run_test(
        "trailer_append",
        &init_req,
        &expect_req,
        &init_res,
        &expect_res,
    )
    .await;
}

#[tokio::test]
async fn test_req_change_query() {
    let mut cxt = TestContext::new().await;

    let init_req = InterceptedRequest {
        uri: "http://localhost/?foo=bar+%26+baz&saison=%C3%89t%C3%A9%2Bhiver"
            .parse()
            .unwrap(),
        ..cxt.default_req.clone()
    };
    let expect_req = InterceptedRequest {
        uri: "http://localhost/?foo=bar&a=b".parse().unwrap(),
        ..cxt.default_req.clone()
    };
    let expect_res = cxt.default_resp.clone();

    cxt.run_test(
        "req_change_query",
        &init_req,
        &expect_req,
        &expect_res,
        &expect_res,
    )
    .await;
}

#[tokio::test]
async fn test_req_encode_query() {
    let mut cxt = TestContext::new().await;

    let init_req = InterceptedRequest {
        uri: "http://localhost".parse().unwrap(),
        ..cxt.default_req.clone()
    };
    let expect_req = InterceptedRequest {
        uri: "http://localhost/?foo=bar+%26+baz&saison=%C3%89t%C3%A9%2Bhiver"
            .parse()
            .unwrap(),
        ..cxt.default_req.clone()
    };
    let init_res = cxt.default_resp.clone();

    cxt.run_test(
        "req_encode_query",
        &init_req,
        &expect_req,
        &init_res,
        &init_res,
    )
    .await;
}

#[tokio::test]
async fn test_body_set() {
    let mut cxt = TestContext::new().await;

    let init_req = cxt.default_req.clone();
    let expect_req = InterceptedRequest {
        body: Bytes::from("rewrite request"),
        ..cxt.default_req.clone()
    };

    let init_res = cxt.default_resp.clone();
    let expect_res = InterceptedResponse {
        body: Bytes::from("rewrite response"),
        ..cxt.default_resp.clone()
    };

    cxt.run_test("body_set", &init_req, &expect_req, &init_res, &expect_res)
        .await;
}

#[tokio::test]
async fn test_body_cascade() {
    let mut cxt = TestContext::new().await;

    let init_req = InterceptedRequest {
        body: Bytes::from("request0"),
        ..cxt.default_req.clone()
    };
    let expect_req = InterceptedRequest {
        body: Bytes::from("request0 request1 request2"),
        ..cxt.default_req.clone()
    };

    let init_res = InterceptedResponse {
        body: Bytes::from("response0"),
        ..cxt.default_resp.clone()
    };
    let expect_res = InterceptedResponse {
        body: Bytes::from("response0 response1 response2"),
        ..cxt.default_resp.clone()
    };

    cxt.run_test(
        "body_cascade",
        &init_req,
        &expect_req,
        &init_res,
        &expect_res,
    )
    .await;
}

#[tokio::test]
async fn test_body_cascade_with_empty() {
    let mut cxt = TestContext::new().await;

    let init_req = InterceptedRequest {
        body: Bytes::from("request0"),
        ..cxt.default_req.clone()
    };
    let expect_req = InterceptedRequest {
        body: Bytes::from("request0 request1 request2"),
        ..cxt.default_req.clone()
    };

    let init_res = InterceptedResponse {
        body: Bytes::from("response0"),
        ..cxt.default_resp.clone()
    };
    let expect_res = InterceptedResponse {
        body: Bytes::from("response0 response1 response2"),
        ..cxt.default_resp.clone()
    };

    cxt.run_test(
        "body_cascade_with_empty",
        &init_req,
        &expect_req,
        &init_res,
        &expect_res,
    )
    .await;
}

#[tokio::test]
async fn test_response_set_status() {
    let mut cxt = TestContext::new().await;

    let init_req = cxt.default_req.clone();
    let expect_req = cxt.default_req.clone();

    let init_res = InterceptedResponse {
        status: StatusCode::OK,
        ..cxt.default_resp.clone()
    };
    let expect_res = InterceptedResponse {
        status: StatusCode::NOT_FOUND,
        ..cxt.default_resp.clone()
    };

    cxt.run_test(
        "response_set_status",
        &init_req,
        &expect_req,
        &init_res,
        &expect_res,
    )
    .await;
}

#[tokio::test]
async fn test_req_set_method() {
    let mut cxt = TestContext::new().await;

    let init_req = cxt.default_req.clone();
    let expect_req = InterceptedRequest {
        method: Method::POST,
        ..cxt.default_req.clone()
    };

    let init_res = cxt.default_resp.clone();

    cxt.run_test(
        "req_set_method",
        &init_req,
        &expect_req,
        &init_res,
        &init_res,
    )
    .await;
}

#[tokio::test]
async fn test_version_set() {
    let mut cxt = TestContext::new().await;

    let init_req = cxt.default_req.clone();
    let expect_req = InterceptedRequest {
        version: http::Version::HTTP_3.into(),
        ..cxt.default_req.clone()
    };

    let init_res = cxt.default_resp.clone();
    let expect_res = InterceptedResponse {
        version: http::Version::HTTP_3.into(),
        ..init_res.clone()
    };

    cxt.run_test(
        "version_set",
        &init_req,
        &expect_req,
        &init_res,
        &expect_res,
    )
    .await;
}

#[tokio::test]
async fn test_req_set_host() {
    let mut cxt = TestContext::new().await;

    let init_req = InterceptedRequest {
        uri: "http://localhost".parse().unwrap(),
        ..cxt.default_req.clone()
    };
    let expect_req = InterceptedRequest {
        uri: "http://example.com".parse().unwrap(),
        ..cxt.default_req.clone()
    };

    let init_res = cxt.default_resp.clone();

    cxt.run_test("req_set_host", &init_req, &expect_req, &init_res, &init_res)
        .await;
}
#[tokio::test]
async fn test_resp_set_body_based_on_req() {
    let mut cxt = TestContext::new().await;

    let init_req = InterceptedRequest {
        uri: "http://example.com".parse().unwrap(),
        ..cxt.default_req.clone()
    };
    let expect_req = init_req.clone();

    let init_res = cxt.default_resp.clone();
    let expect_res = InterceptedResponse {
        body: Bytes::from("intercepted"),
        ..cxt.default_resp.clone()
    };

    cxt.run_test(
        "resp_set_body_based_on_req",
        &init_req,
        &expect_req,
        &init_res,
        &expect_res,
    )
    .await;
}

#[tokio::test]
async fn test_notify() {
    let (notify_tx, mut notify_rx) = mpsc::channel(10);
    let mut cxt = TestContext::new_with_notify(notify_tx).await;
    let expect_req = InterceptedRequest {
        ..cxt.default_req.clone()
    };

    let expect_resp = InterceptedResponse {
        ..cxt.default_resp.clone()
    };

    let tests = vec![
        (
            TestContext::load_script("notify", ScriptType::Lua).await,
            ScriptType::Lua,
        ),
        (
            TestContext::load_script("notify", ScriptType::Js).await,
            ScriptType::Js,
        ),
        (
            TestContext::load_script("notify", ScriptType::Python).await,
            ScriptType::Python,
        ),
    ];
    for t in tests {
        cxt.engine.set_script(&t.0, t.1).await.unwrap();

        let mut actual_req = cxt.default_req.clone();
        cxt.engine.intercept_request(&mut actual_req).await.unwrap();
        assert_eq!(expect_req, actual_req);

        let mut actual_resp = cxt.default_resp.clone();
        cxt.engine
            .intercept_response(&expect_req, &mut actual_resp)
            .await
            .unwrap();
        assert_eq!(expect_resp, actual_resp);
        let mut notifications = vec![];
        for _ in 0..2 {
            let notification = notify_rx.try_recv().unwrap();
            notifications.push(notification);
        }
        assert_eq!(notifications.len(), 2);
        assert_eq!(
            notifications[0],
            FlowNotify {
                level: FlowNotifyLevel::Warn,
                msg: "hi".to_string()
            }
        );

        assert_eq!(
            notifications[1],
            FlowNotify {
                level: FlowNotifyLevel::Error,
                msg: "there".to_string()
            }
        );
    }
}

#[tokio::test]
async fn test_body_sub() {
    let mut cxt = TestContext::new().await;

    let init_req = InterceptedRequest {
        body: Bytes::from_static(b"this replaceme needs to go"),
        ..cxt.default_req.clone()
    };
    let expect_req = InterceptedRequest {
        body: Bytes::from_static(b"this gone needs to go"),
        ..cxt.default_req.clone()
    };

    let init_res = InterceptedResponse {
        body: Bytes::from_static(b"this to_go needs to go"),
        ..cxt.default_resp.clone()
    };
    let expect_res = InterceptedResponse {
        body: Bytes::from_static(b"this it_went needs to go"),
        ..cxt.default_resp.clone()
    };
    cxt.run_test("body_sub", &init_req, &expect_req, &init_res, &expect_res)
        .await;
}

#[tokio::test]
async fn test_start_invoked() {
    let mut cxt = TestContext::new().await;

    let init_req = InterceptedRequest {
        body: Bytes::from_static(b"body"),
        ..cxt.default_req.clone()
    };
    let expect_req = InterceptedRequest {
        body: Bytes::from_static(b"10"),
        ..cxt.default_req.clone()
    };

    let init_res = InterceptedResponse {
        body: Bytes::from_static(b"body"),
        ..cxt.default_resp.clone()
    };
    let expect_res = InterceptedResponse {
        body: Bytes::from_static(b"11"),
        ..cxt.default_resp.clone()
    };
    cxt.run_test(
        "start_invoked",
        &init_req,
        &expect_req,
        &init_res,
        &expect_res,
    )
    .await;
}

#[tokio::test]
async fn test_stop_invoked() {
    let mut cxt = TestContext::new().await;
    let temp = tempfile::tempdir().unwrap();
    let expect_file = temp.path().join("stop.json");

    let init_req = InterceptedRequest {
        body: Bytes::from_static(b"body"),
        ..cxt.default_req.clone()
    };
    let expect_req = InterceptedRequest {
        body: Bytes::from_static(b"10"),
        ..cxt.default_req.clone()
    };

    let init_res = InterceptedResponse {
        body: Bytes::from(expect_file.to_string_lossy().into_owned()),
        ..cxt.default_resp.clone()
    };
    let expect_res = InterceptedResponse {
        body: Bytes::from_static(b"11"),
        ..cxt.default_resp.clone()
    };
    for st in ScriptType::iter() {
        let script = TestContext::load_script("stop_invoked", st).await;
        cxt.engine.set_script(&script, st).await.unwrap();

        let mut actual_req = init_req.clone();
        cxt.engine.intercept_request(&mut actual_req).await.unwrap();
        assert_eq!(expect_req, actual_req);

        let mut actual_res = init_res.clone();
        cxt.engine
            .intercept_response(&expect_req, &mut actual_res)
            .await
            .unwrap();

        cxt.engine.set_script("", ScriptType::Lua).await.unwrap();
        assert_eq!(expect_res, actual_res);
        assert!(expect_file.exists());
        tokio::fs::remove_file(&expect_file).await.unwrap();
    }
}

#[tokio::test]
async fn test_req_set_resp_body() {
    let mut cxt = TestContext::new().await;
    let req_body = Bytes::from_static(b"hi there");

    let mut req = InterceptedRequest {
        body: req_body,
        ..cxt.default_req.clone()
    };

    let expected_request = req.clone();

    let tests = vec![
        (
            TestContext::load_script("req_set_resp_body", ScriptType::Lua).await,
            ScriptType::Lua,
        ),
        (
            TestContext::load_script("req_set_resp_body", ScriptType::Js).await,
            ScriptType::Js,
        ),
        (
            TestContext::load_script("req_set_resp_body", ScriptType::Python).await,
            ScriptType::Python,
        ),
    ];
    for t in tests {
        cxt.engine.set_script(&t.0, t.1).await.unwrap();
        let early_response = cxt
            .engine
            .intercept_request(&mut req)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(expected_request, req);
        let expected_response = InterceptedResponse {
            status: StatusCode::OK,
            timestamp: early_response.timestamp,
            version: http::Version::HTTP_11.into(),
            headers: HeaderMap::new(),
            encoding: None,
            body: Bytes::from("early return"),
            trailers: None,
        };
        assert_eq!(early_response, expected_response);
    }
}

#[tokio::test]
async fn test_req_set_resp_status() {
    let mut cxt = TestContext::new().await;

    let req_body = Bytes::from_static(b"hi there");

    let mut req = InterceptedRequest {
        body: req_body,
        ..cxt.default_req.clone()
    };

    let expected_request = req.clone();
    let test_name = "req_set_resp_status";

    let tests = vec![
        (
            TestContext::load_script(test_name, ScriptType::Lua).await,
            ScriptType::Lua,
        ),
        (
            TestContext::load_script(test_name, ScriptType::Js).await,
            ScriptType::Js,
        ),
        (
            TestContext::load_script(test_name, ScriptType::Python).await,
            ScriptType::Python,
        ),
    ];
    for t in tests {
        cxt.engine.set_script(&t.0, t.1).await.unwrap();
        let early_response = cxt
            .engine
            .intercept_request(&mut req)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(expected_request, req);
        let expected_response = InterceptedResponse {
            status: StatusCode::NOT_FOUND,
            timestamp: early_response.timestamp,
            version: http::Version::HTTP_11.into(),
            headers: HeaderMap::new(),
            encoding: None,
            body: Bytes::new(),
            trailers: None,
        };
        assert_eq!(early_response, expected_response);
    }
}
