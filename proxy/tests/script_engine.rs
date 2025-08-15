use std::{path::PathBuf, sync::atomic::AtomicI16, time::Duration};

use bytes::Bytes;
use chrono::Utc;
use http::{HeaderMap, HeaderName, StatusCode};
use roxy_proxy::{
    flow::{InterceptedRequest, InterceptedResponse},
    init_test_logging,
    interceptor::{FlowNotify, ScriptEngine},
};
use roxy_shared::alpn::AlpnProtocol;
use tempfile::TempDir;
use tokio::sync::mpsc;
use tracing::info;

static EMPTY_SCRIPT: &str = r#"
"#;

static VOID_SCRIPT: &str = r#"

Extensions = {
  {
  intercept_request = function (flow) 
  end,
  intercept_response = function (flow) 
  end,
  }
}
"#;

struct TestContext {
    engine: ScriptEngine,
    _temp_dir: TempDir,
    count: AtomicI16,
    script_file: PathBuf,
}

impl TestContext {
    fn new_engine(engine: ScriptEngine) -> Self {
        init_test_logging();
        let temp_dir = TempDir::new().unwrap();
        let mut script_file = temp_dir.path().to_path_buf();
        script_file.push("test.lua");
        Self {
            engine,
            count: AtomicI16::new(0),
            _temp_dir: temp_dir,
            script_file,
        }
    }
    async fn new() -> Self {
        TestContext::new_engine(ScriptEngine::new().await.unwrap())
    }

    async fn new_with_notify(notify_tx: mpsc::Sender<FlowNotify>) -> Self {
        TestContext::new_engine(ScriptEngine::new_notify(notify_tx).await.unwrap())
    }

    pub async fn set_script_file(&mut self, script: &str) {
        let mut script_file = self._temp_dir.path().to_path_buf();
        let index = self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        script_file.push(format!("test-{index}.lua"));
        info!("new script {script_file:?}");
        self.script_file = script_file;
        tokio::fs::write(&self.script_file, script).await.unwrap();
        self.engine
            .load_script_path(self.script_file.clone())
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    pub async fn update_script_file(&mut self, script: &str) {
        info!("update script file");
        tokio::fs::write(&self.script_file, script).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test]
async fn test_empty_works() {
    let mut cxt = TestContext::new().await;
    cxt.engine.set_script(EMPTY_SCRIPT).await.unwrap();

    let mut req = InterceptedRequest {
        timestamp: Utc::now(),
        uri: "http://localhost:80".parse().unwrap(),
        alpn: AlpnProtocol::None,
        encoding: None,
        method: http::Method::GET,
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        body: bytes::Bytes::new(),
        trailers: None,
    };

    let clone = req.clone();
    cxt.engine.intercept_request(&mut req).await.unwrap();

    assert_eq!(clone, req);
    let mut resp = InterceptedResponse {
        status: StatusCode::OK,
        timestamp: Utc::now(),
        version: http::Version::HTTP_11,
        encoding: None,
        headers: HeaderMap::new(),
        body: bytes::Bytes::new(),
        trailers: None,
    };
    let clone = resp.clone();
    cxt.engine.intercept_response(&mut resp).await.unwrap();
    assert_eq!(clone, resp);
}

#[tokio::test]
async fn test_void_no_chage() {
    let mut cxt = TestContext::new().await;
    cxt.engine.set_script(VOID_SCRIPT).await.unwrap();

    let mut req = InterceptedRequest {
        timestamp: Utc::now(),
        uri: "http://localhost:80".parse().unwrap(),
        alpn: AlpnProtocol::None,
        encoding: None,
        method: http::Method::GET,
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        body: bytes::Bytes::new(),
        trailers: None,
    };

    let clone = req.clone();
    cxt.engine.intercept_request(&mut req).await.unwrap();

    assert_eq!(clone, req);
    let mut resp = InterceptedResponse {
        status: StatusCode::OK,
        timestamp: Utc::now(),
        version: http::Version::HTTP_11,
        encoding: None,
        headers: HeaderMap::new(),
        body: bytes::Bytes::new(),
        trailers: None,
    };
    let clone = resp.clone();
    cxt.engine.intercept_response(&mut resp).await.unwrap();
    assert_eq!(clone, resp);
}

#[tokio::test]
async fn test_no_chage_double_headers() {
    let mut cxt = TestContext::new().await;
    cxt.engine.set_script(VOID_SCRIPT).await.unwrap();

    let mut headers = HeaderMap::new();
    headers.append("set-cookie", "a".parse().unwrap());
    headers.append("set-cookie", "b".parse().unwrap());
    let mut req = InterceptedRequest {
        timestamp: Utc::now(),
        uri: "http://localhost:80".parse().unwrap(),
        alpn: AlpnProtocol::None,
        encoding: None,
        method: http::Method::GET,
        version: http::Version::HTTP_11,
        headers: headers.clone(),
        body: bytes::Bytes::new(),
        trailers: None,
    };

    let clone = req.clone();
    cxt.engine.intercept_request(&mut req).await.unwrap();

    assert_eq!(clone, req);
    let mut resp = InterceptedResponse {
        status: StatusCode::OK,
        timestamp: Utc::now(),
        version: http::Version::HTTP_11,
        encoding: None,
        headers: headers.clone(),
        body: bytes::Bytes::new(),
        trailers: None,
    };
    let clone = resp.clone();
    cxt.engine.intercept_response(&mut resp).await.unwrap();
    assert_eq!(clone, resp);
}

static ADD_COOKIE: &str = r#"
Extensions = {
	{
	  intercept_request = function(flow)
	    local cookies = flow.request.headers["set-cookie"]
	    table.insert(cookies, "test-request")
	    flow.request.headers["set-cookie"] = cookies
	  end,
	  intercept_response = function(flow)
	    local cookies = flow.response.headers["set-cookie"]
	    table.insert(cookies, "test-response")
	    flow.response.headers["set-cookie"] = cookies
	  end,
	},
}
"#;

#[tokio::test]
async fn test_insert_mult_headers() {
    let mut cxt = TestContext::new().await;
    cxt.engine.set_script(ADD_COOKIE).await.unwrap();

    let mut headers = HeaderMap::new();
    headers.append("set-cookie", "a".parse().unwrap());
    headers.append("set-cookie", "b".parse().unwrap());

    let mut expect_headers = headers.clone();
    expect_headers.append("set-cookie", "test-request".parse().unwrap());

    let mut req = InterceptedRequest {
        timestamp: Utc::now(),
        uri: "http://localhost:80".parse().unwrap(),
        alpn: AlpnProtocol::None,
        encoding: None,
        method: http::Method::GET,
        version: http::Version::HTTP_11,
        headers: headers.clone(),
        body: bytes::Bytes::new(),
        trailers: None,
    };

    let expect = InterceptedRequest {
        headers: expect_headers.clone(),
        ..req.clone()
    };
    cxt.engine.intercept_request(&mut req).await.unwrap();
    assert_eq!(expect, req);

    let mut resp = InterceptedResponse {
        status: StatusCode::OK,
        timestamp: Utc::now(),
        version: http::Version::HTTP_11,
        encoding: None,
        headers: headers.clone(),
        body: bytes::Bytes::new(),
        trailers: None,
    };
    let mut expect_headers = headers.clone();
    expect_headers.append("set-cookie", "test-response".parse().unwrap());
    let expect = InterceptedResponse {
        headers: expect_headers,
        ..resp.clone()
    };
    cxt.engine.intercept_response(&mut resp).await.unwrap();
    assert_eq!(expect, resp);
}

static CHANGE_QUERY: &str = r#"
Extensions = {
	{
	  intercept_request = function(flow)
      flow.request.query["foo"] = "bar"
      flow.request.query["a"] = "b"
      flow.request.query["no"] = Nil
      flow.request.query["yes"] = Nil
      flow.request.query["saison"] = Nil
	  end,
	  intercept_response = function(flow)
	  end,
	},
}
"#;

#[tokio::test]
async fn test_change_query() {
    let mut cxt = TestContext::new().await;
    cxt.engine.set_script(CHANGE_QUERY).await.unwrap();

    let mut req = InterceptedRequest {
        timestamp: Utc::now(),
        uri: "http://localhost:80".parse().unwrap(),
        alpn: AlpnProtocol::None,
        encoding: None,
        method: http::Method::GET,
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        body: bytes::Bytes::new(),
        trailers: None,
    };

    let expect = InterceptedRequest {
        uri: "http://localhost:80?a=b&foo=bar".parse().unwrap(),
        ..req.clone()
    };
    cxt.engine.intercept_request(&mut req).await.unwrap();
    assert_eq!(expect, req);

    let mut req = InterceptedRequest {
        uri: "http://localhost:80/?foo=bar+%26+baz&saison=%C3%89t%C3%A9%2Bhiver"
            .parse()
            .unwrap(),
        ..req.clone()
    };
    cxt.engine.intercept_request(&mut req).await.unwrap();
    assert_eq!(expect, req);
}

static ENCODE_QUERY: &str = r#"
Extensions = {
	{
	  intercept_request = function(flow)
      flow.request.query = {
        ["foo"] = "bar & baz",
        ["saison"] = "Été+hiver"
      }
	  end,
	  intercept_response = function(flow)
	  end,
	},
}
"#;

#[tokio::test]
async fn test_encode_query() {
    let mut cxt = TestContext::new().await;
    cxt.engine.set_script(ENCODE_QUERY).await.unwrap();

    let mut req = InterceptedRequest {
        timestamp: Utc::now(),
        uri: "http://localhost:80".parse().unwrap(),
        alpn: AlpnProtocol::None,
        encoding: None,
        method: http::Method::GET,
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        body: bytes::Bytes::new(),
        trailers: None,
    };

    let expect = InterceptedRequest {
        uri: "http://localhost:80/?foo=bar+%26+baz&saison=%C3%89t%C3%A9%2Bhiver"
            .parse()
            .unwrap(),
        ..req.clone()
    };
    cxt.engine.intercept_request(&mut req).await.unwrap();
    assert_eq!(expect, req);

    let mut req = InterceptedRequest {
        uri: "http://localhost:80?no=yes&yes=no".parse().unwrap(),
        ..req.clone()
    };
    cxt.engine.intercept_request(&mut req).await.unwrap();
    assert_eq!(expect, req);
}

static REWRITE_BODY: &str = "rewrite";

static REWRITE_SCRIPT: &str = r#"
Extensions = {
  {
  intercept_request = function (flow) 
    flow.request.body = "rewrite"
  end,
  intercept_response = function (flow) 
    flow.response.body = "rewrite"
  end,
  }
}
"#;

#[tokio::test]
async fn test_change_body() {
    let mut cxt = TestContext::new().await;
    cxt.engine.set_script(REWRITE_SCRIPT).await.unwrap();

    let mut req = InterceptedRequest {
        timestamp: Utc::now(),
        uri: "localhost:80".parse().unwrap(),
        alpn: AlpnProtocol::Http1,
        method: http::Method::GET,
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        encoding: None,
        body: bytes::Bytes::new(),
        trailers: None,
    };

    let expected_req = InterceptedRequest {
        body: Bytes::from(REWRITE_BODY),
        ..req.clone()
    };

    cxt.engine.intercept_request(&mut req).await.unwrap();

    assert_eq!(req, expected_req);
    let mut resp = InterceptedResponse {
        status: StatusCode::OK,
        timestamp: Utc::now(),
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        encoding: None,
        body: bytes::Bytes::new(),
        trailers: None,
    };
    let expected_response = InterceptedResponse {
        body: Bytes::from(REWRITE_BODY),
        ..resp.clone()
    };
    cxt.engine.intercept_response(&mut resp).await.unwrap();
    assert_eq!(expected_response, resp);
}

static REWRITE_SCRIPT_NO_KEYS: &str = r#"
Extensions = {
  {
  function (flow) 
    flow.request.body = "rewrite"
  end,
  function (flow) 
    flow.response.body = "rewrite"
  end,
  }
}
"#;

#[tokio::test]
async fn test_change_body_no_keys() {
    let mut cxt = TestContext::new().await;
    cxt.engine.set_script(REWRITE_SCRIPT_NO_KEYS).await.unwrap();

    let mut req = InterceptedRequest {
        timestamp: Utc::now(),
        uri: "localhost:80".parse().unwrap(),
        alpn: AlpnProtocol::Http1,
        method: http::Method::GET,
        version: http::Version::HTTP_11,
        encoding: None,
        headers: HeaderMap::new(),
        body: bytes::Bytes::new(),
        trailers: None,
    };
    let expected_req = InterceptedRequest {
        body: Bytes::from(REWRITE_BODY),
        ..req.clone()
    };

    cxt.engine.intercept_request(&mut req).await.unwrap();
    assert_eq!(expected_req, req);

    let mut resp = InterceptedResponse {
        status: StatusCode::OK,
        timestamp: Utc::now(),
        version: http::Version::HTTP_11,
        encoding: None,
        headers: HeaderMap::new(),
        body: bytes::Bytes::new(),
        trailers: None,
    };
    let expected_response = InterceptedResponse {
        body: Bytes::from(REWRITE_BODY),
        ..resp.clone()
    };
    cxt.engine.intercept_response(&mut resp).await.unwrap();
    assert_eq!(expected_response, resp);
}

static CASCADE_BODY: &str = r#"
function req(flow) 
flow.request.body = flow.request.body.."rewrite"
end

function resp(flow) 
flow.response.body = flow.response.body.."rewrite"
end
Extensions = {
  {
    intercept_request = req,
    intercept_response = resp
  },
  {
    intercept_request = req,
    intercept_response = resp
  }
}
"#;

#[tokio::test]
async fn test_cascade_body() {
    let mut cxt = TestContext::new().await;
    cxt.engine.set_script(CASCADE_BODY).await.unwrap();

    let mut req = InterceptedRequest {
        timestamp: Utc::now(),
        uri: "localhost:80".parse().unwrap(),
        alpn: AlpnProtocol::Http1,
        method: http::Method::GET,
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        encoding: None,
        body: bytes::Bytes::new(),
        trailers: None,
    };

    let expected_request = InterceptedRequest {
        body: Bytes::from_static(b"rewriterewrite"),
        ..req.clone()
    };

    cxt.engine.intercept_request(&mut req).await.unwrap();
    assert_eq!(req, expected_request);

    let mut resp = InterceptedResponse {
        status: StatusCode::OK,
        timestamp: Utc::now(),
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        encoding: None,
        body: bytes::Bytes::new(),
        trailers: None,
    };
    let expected_response = InterceptedResponse {
        body: Bytes::from_static(b"rewriterewrite"),
        ..resp.clone()
    };
    cxt.engine.intercept_response(&mut resp).await.unwrap();
    assert_eq!(expected_response, resp);
}

static CASCADE_BODY_WITH_EMPTY: &str = r#"
function req(flow) 
flow.request.body = flow.request.body.."rewrite"
end

function resp(flow) 
flow.response.body = flow.response.body.."rewrite"
end
Extensions = {
  {
    intercept_request = req,
    intercept_response = resp
  },
  {},
  {
    intercept_request = req,
    intercept_response = resp
  }
}
"#;

#[tokio::test]
async fn test_cascade_body_with_empty() {
    let mut cxt = TestContext::new().await;
    cxt.engine
        .set_script(CASCADE_BODY_WITH_EMPTY)
        .await
        .unwrap();
    let mut req = InterceptedRequest {
        timestamp: Utc::now(),
        uri: "localhost:80".parse().unwrap(),
        alpn: AlpnProtocol::Http1,
        method: http::Method::GET,
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        encoding: None,
        body: bytes::Bytes::new(),
        trailers: None,
    };

    let expected_request = InterceptedRequest {
        body: Bytes::from_static(b"rewriterewrite"),
        ..req.clone()
    };

    cxt.engine.intercept_request(&mut req).await.unwrap();
    assert_eq!(req, expected_request);

    let mut resp = InterceptedResponse {
        status: StatusCode::OK,
        timestamp: Utc::now(),
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        encoding: None,
        body: bytes::Bytes::new(),
        trailers: None,
    };
    let expected_response = InterceptedResponse {
        body: Bytes::from_static(b"rewriterewrite"),
        ..resp.clone()
    };
    cxt.engine.intercept_response(&mut resp).await.unwrap();
    assert_eq!(expected_response, resp);
}

static NOTIFY_SCRIPT: &str = r#"
function req(flow) 
    Roxy.notify(1, "hi")
end

function resp(flow) 
    Roxy.notify(2, "there")
end
Extensions = {
  {
    intercept_request = req,
    intercept_response = resp
  },
}
"#;

#[tokio::test]
async fn test_notify() {
    let (notify_tx, mut notify_rx) = mpsc::channel(10);

    let mut cxt = TestContext::new_with_notify(notify_tx).await;
    cxt.engine.set_script(NOTIFY_SCRIPT).await.unwrap();

    let mut req = InterceptedRequest {
        timestamp: Utc::now(),
        uri: "localhost:80".parse().unwrap(),
        alpn: AlpnProtocol::Http1,
        method: http::Method::GET,
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        encoding: None,
        body: bytes::Bytes::new(),
        trailers: None,
    };

    cxt.engine.intercept_request(&mut req).await.unwrap();

    let mut resp = InterceptedResponse {
        status: StatusCode::OK,
        timestamp: Utc::now(),
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        encoding: None,
        body: bytes::Bytes::new(),
        trailers: None,
    };
    cxt.engine.intercept_response(&mut resp).await.unwrap();

    let mut notifications = vec![];
    for _ in 0..2 {
        let notification = notify_rx.try_recv().unwrap();
        notifications.push(notification);
    }
    assert_eq!(notifications.len(), 2);
    assert_eq!(
        notifications[0],
        FlowNotify {
            level: 1,
            msg: "hi".to_string()
        }
    );

    assert_eq!(
        notifications[1],
        FlowNotify {
            level: 2,
            msg: "there".to_string()
        }
    );
}

static GSUB_BODY_SCRIPT: &str = r#"
function req(flow) 
    flow.request.body = string.gsub(flow.request.body, "replaceme", "gone")
end

function resp(flow) 
    flow.response.body = string.gsub(flow.response.body, "to_go", "it_went")
end
Extensions = {
  {
    intercept_request = req,
    intercept_response = resp
  },
}
"#;

#[tokio::test]
async fn test_gsub_body() {
    let mut cxt = TestContext::new().await;
    cxt.engine.set_script(GSUB_BODY_SCRIPT).await.unwrap();

    let req_body = Bytes::from_static(b"this replaceme needs to go");
    let mut req = InterceptedRequest {
        timestamp: Utc::now(),
        uri: "localhost:80".parse().unwrap(),
        alpn: AlpnProtocol::Http1,
        method: http::Method::GET,
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        encoding: None,
        body: req_body,
        trailers: None,
    };

    let expected = InterceptedRequest {
        body: Bytes::from_static(b"this gone needs to go"),
        ..req.clone()
    };

    cxt.engine.intercept_request(&mut req).await.unwrap();
    assert_eq!(expected.body, req.body);

    let resp_body = Bytes::from_static(b"this to_go needs to go");
    let mut resp = InterceptedResponse {
        status: StatusCode::OK,
        timestamp: Utc::now(),
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        encoding: None,
        body: resp_body,
        trailers: None,
    };
    cxt.engine.intercept_response(&mut resp).await.unwrap();

    let expected = InterceptedResponse {
        body: Bytes::from_static(b"this it_went needs to go"),
        ..resp.clone()
    };

    cxt.engine.intercept_response(&mut resp).await.unwrap();
    assert_eq!(expected, resp);
}

static RETURN_BODY_EARLY: &str = r#"
Extensions = {
  {
  function (flow) 
    flow.response.body = "early return"
    flow.response.headers = flow.request.headers
  end,
  function (flow) 
  end,
  }
}
"#;

#[tokio::test]
async fn test_early_return() {
    let mut cxt = TestContext::new().await;
    cxt.engine.set_script(RETURN_BODY_EARLY).await.unwrap();

    let req_body = Bytes::from_static(b"hi there");
    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("test"), "test-v".parse().unwrap());

    let mut req = InterceptedRequest {
        timestamp: Utc::now(),
        uri: "localhost:80".parse().unwrap(),
        alpn: AlpnProtocol::Http1,
        method: http::Method::GET,
        version: http::Version::HTTP_11,
        headers: headers.clone(),
        encoding: None,
        body: req_body,
        trailers: None,
    };

    let expected_request = req.clone();

    let mut early_response = cxt
        .engine
        .intercept_request(&mut req)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(expected_request, req);
    let expected_response = InterceptedResponse {
        status: StatusCode::OK,
        timestamp: early_response.timestamp,
        version: http::Version::HTTP_11,
        headers,
        encoding: None,
        body: Bytes::from("early return"),
        trailers: None,
    };
    assert_eq!(early_response, expected_response);

    cxt.engine
        .intercept_response(&mut early_response)
        .await
        .unwrap();
    assert_eq!(early_response, expected_response);
}

#[tokio::test]
async fn test_file_watcher_reloads_script() {
    let mut cxt = TestContext::new().await;
    cxt.set_script_file(VOID_SCRIPT).await;

    let req_body = Bytes::from_static(b"this replaceme needs to go");
    let mut req = InterceptedRequest {
        timestamp: Utc::now(),
        uri: "https://localhost:80".parse().unwrap(),
        alpn: AlpnProtocol::Http1,
        method: http::Method::GET,
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        encoding: None,
        body: req_body,
        trailers: None,
    };

    let expected_request = req.clone();
    cxt.engine.intercept_request(&mut req).await.unwrap();
    assert_eq!(expected_request, req);

    let resp_body = Bytes::from_static(b"this to_go needs to go");
    let mut resp = InterceptedResponse {
        status: StatusCode::OK,
        timestamp: Utc::now(),
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        encoding: None,
        body: resp_body,
        trailers: None,
    };

    let expected_response = resp.clone();
    cxt.engine.intercept_response(&mut resp).await.unwrap();
    assert_eq!(expected_response, resp);

    cxt.update_script_file(GSUB_BODY_SCRIPT).await;

    let expected_request = InterceptedRequest {
        body: Bytes::from_static(b"this gone needs to go"),
        ..req.clone()
    };
    cxt.engine.intercept_request(&mut req).await.unwrap();
    assert_eq!(expected_request, req);

    let expected_response = InterceptedResponse {
        body: Bytes::from_static(b"this it_went needs to go"),
        ..resp.clone()
    };
    cxt.engine.intercept_response(&mut resp).await.unwrap();
    assert_eq!(expected_response, resp);
}

#[tokio::test]
async fn test_file_watcher_updates_new_file() {
    let mut cxt = TestContext::new().await;
    cxt.set_script_file(VOID_SCRIPT).await;

    let req_body = Bytes::from_static(b"this replaceme needs to go");
    let mut req = InterceptedRequest {
        timestamp: Utc::now(),
        uri: "https://localhost:80".parse().unwrap(),
        alpn: AlpnProtocol::Http1,
        method: http::Method::GET,
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        encoding: None,
        body: req_body,
        trailers: None,
    };

    let req_clone = req.clone();
    cxt.engine.intercept_request(&mut req).await.unwrap();
    assert_eq!(req_clone, req);

    let mut resp = InterceptedResponse {
        status: StatusCode::OK,
        timestamp: Utc::now(),
        version: http::Version::HTTP_11,
        headers: HeaderMap::new(),
        encoding: None,
        body: Bytes::from_static(b"this to_go needs to go"),
        trailers: None,
    };

    let resp_clone = resp.clone();
    cxt.engine.intercept_response(&mut resp).await.unwrap();
    assert_eq!(resp_clone, resp);

    cxt.set_script_file(GSUB_BODY_SCRIPT).await;
    let expected_request = InterceptedRequest {
        body: Bytes::from_static(b"this gone needs to go"),
        ..req.clone()
    };
    cxt.engine.intercept_request(&mut req).await.unwrap();
    assert_eq!(expected_request, req);

    let expected_response = InterceptedResponse {
        body: Bytes::from_static(b"this it_went needs to go"),
        ..resp.clone()
    };
    cxt.engine.intercept_response(&mut resp).await.unwrap();
    assert_eq!(expected_response, resp);
}

#[tokio::test]
async fn test_file_watcher_emits_one_event_on_change() {
    let (notify_tx, mut notify_rx) = mpsc::channel(10);
    let mut cxt = TestContext::new_with_notify(notify_tx).await;

    // no event on load
    cxt.set_script_file(VOID_SCRIPT).await;
    assert_eq!(0, notify_rx.len());

    // one event on change
    cxt.update_script_file(VOID_SCRIPT).await;
    assert_eq!(1, notify_rx.len());
    let event = notify_rx.recv().await.unwrap();
    assert_eq!(0, event.level);

    // no event on set script file
    cxt.set_script_file(VOID_SCRIPT).await;
    assert_eq!(0, notify_rx.len());

    for _ in 0..10 {
        // one event on change
        cxt.update_script_file(VOID_SCRIPT).await;
        assert_eq!(1, notify_rx.len());
        let event = notify_rx.recv().await.unwrap();
        assert_eq!(0, event.level);
    }
}
