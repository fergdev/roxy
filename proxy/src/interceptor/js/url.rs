use boa_engine::{
    Context, Finalize, JsData, JsResult, JsString, JsValue, Trace, boa_class, js_error, js_string,
};
use cow_utils::CowUtils;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::info;

use crate::interceptor::js::query::UrlSearchParams;
use crate::interceptor::util::set_url_authority;

#[derive(Debug, Clone, JsData, Trace, Finalize)]
#[boa_gc(unsafe_no_drop)]
pub(crate) struct JsUrl(#[unsafe_ignore_trace] Rc<RefCell<url::Url>>);

#[boa_class(rename = "URL")]
#[boa(rename_all = "camelCase")]
impl JsUrl {
    #[boa(constructor)]
    fn new(url: String, base: JsValue) -> JsResult<Self> {
        if let Some(base) = base.as_string() {
            let base = base.to_std_string_lossy();
            let base_url = url::Url::parse(&base)
                .map_err(|e| js_error!(TypeError: "Failed to parse base URL: {}", e))?;
            if base_url.cannot_be_a_base() {
                return Err(js_error!(TypeError: "Base URL {} cannot be a base", base));
            }

            let url = base_url
                .join(&url)
                .map_err(|e| js_error!(TypeError: "Failed to parse URL: {}", e))?;
            Ok(Self(Rc::new(RefCell::new(url))))
        } else {
            let url = url::Url::parse(&url)
                .map_err(|e| js_error!(TypeError: "Failed to parse URL: {}", e))?;
            Ok(Self(Rc::new(RefCell::new(url))))
        }
    }

    #[boa(getter)]
    pub fn hash(&self) -> String {
        url::quirks::hash(&self.0.borrow()).to_string()
    }

    #[boa(setter)]
    #[boa(method)]
    #[boa(rename = "hash")]
    pub fn set_hash(&self, value: String) {
        url::quirks::set_hash(&mut self.0.borrow_mut(), &value);
    }

    #[boa(getter)]
    pub fn host(&self) -> JsString {
        js_string!(url::quirks::host(&self.0.borrow()))
    }

    #[boa(setter)]
    #[boa(method)]
    #[boa(rename = "host")]
    pub fn set_host(&self, value: String) -> JsResult<()> {
        url::quirks::set_host(&mut self.0.borrow_mut(), &value)
            .map_err(|_| js_error!(TypeError: "Failed to set host with value '{}'", value))
    }

    #[boa(getter)]
    #[boa(rename = "hostname")]
    pub fn host_name(&self) -> String {
        url::quirks::hostname(&self.0.borrow()).into()
    }

    #[boa(setter)]
    #[boa(method)]
    #[boa(rename = "hostname")]
    pub fn set_host_name(&self, value: String) -> JsResult<()> {
        url::quirks::set_hostname(&mut self.0.borrow_mut(), &value)
            .map_err(|_| js_error!(TypeError: "Failed to set hostname with value '{}'", value))
    }

    #[boa(getter)]
    pub fn href(&self) -> String {
        url::quirks::href(&self.0.borrow()).into()
    }

    #[boa(setter)]
    #[boa(method)]
    #[boa(rename = "href")]
    pub fn set_href(&self, value: String) -> JsResult<()> {
        url::quirks::set_href(&mut self.0.borrow_mut(), &value)
            .map_err(|_| js_error!(TypeError: "Failed to set href with value '{}'", value))
    }

    #[boa(getter)]
    pub fn authority(&self) -> String {
        self.0.borrow().authority().to_string()
    }

    #[boa(setter)]
    #[boa(method)]
    #[boa(rename = "authority")]
    pub fn set_authority(&self, value: String) -> JsResult<()> {
        let mut url = self.0.borrow_mut();
        set_url_authority(&mut url, &value)
            .map_err(|e| js_error!(TypeError: "Failed to set authority: {}", e))
    }

    #[boa(getter)]
    pub fn password(&self) -> String {
        url::quirks::password(&self.0.borrow()).into()
    }

    #[boa(setter)]
    #[boa(method)]
    #[boa(rename = "password")]
    pub fn set_password(&self, value: String) -> JsResult<()> {
        let mut url = self.0.borrow_mut();
        url::quirks::set_password(&mut url, &value)
            .map_err(|_| js_error!(TypeError: "Failed to set password: {value}"))
    }

    #[boa(getter)]
    pub fn path(&self) -> String {
        url::quirks::pathname(&self.0.borrow()).into()
    }

    #[boa(setter)]
    #[boa(method)]
    #[boa(rename = "path")]
    pub fn set_path(&self, value: String) -> JsResult<()> {
        url::quirks::set_pathname(&mut self.0.borrow_mut(), &value);
        Ok(())
    }

    #[boa(getter)]
    pub fn port(&self) -> i32 {
        url::quirks::port(&self.0.borrow())
            .parse::<i32>()
            .unwrap_or(0)
    }

    #[boa(setter)]
    #[boa(method)]
    #[boa(rename = "port")]
    pub fn set_port(&self, value: JsValue, context: &mut Context) -> JsResult<()> {
        info!("set port {value:?}");
        url::quirks::set_port(
            &mut self.0.borrow_mut(),
            &value.to_string(context)?.to_std_string_lossy(),
        )
        .map_err(|_| js_error!(TypeError: "Failed to set port with value '{:?}'", value))
    }

    #[boa(getter)]
    pub fn protocol(&self) -> String {
        url::quirks::protocol(&self.0.borrow())
            .cow_replace(":", "")
            .into()
    }

    #[boa(setter)]
    #[boa(method)]
    #[boa(rename = "protocol")]
    pub fn set_protocol(&self, value: String) -> JsResult<()> {
        url::quirks::set_protocol(&mut self.0.borrow_mut(), &value)
            .map_err(|_| js_error!(TypeError: "Failed to set port with value '{}'", value))
    }

    #[boa(getter)]
    pub fn search(&self) -> String {
        url::quirks::search(&self.0.borrow()).into()
    }

    #[boa(setter)]
    #[boa(method)]
    #[boa(rename = "search")]
    pub fn set_search(&self, value: String) -> JsResult<()> {
        url::quirks::set_search(&mut self.0.borrow_mut(), &value);
        Ok(())
    }

    #[boa(getter)]
    #[boa(rename = "searchParams")]
    pub fn search_params(&self) -> JsResult<UrlSearchParams> {
        let url = self.0.clone();
        let params = UrlSearchParams { url };
        // let obj = UrlSearchParams::from_data(params, context)?;
        // Ok(obj.into())
        Ok(params)
    }

    #[boa(getter)]
    pub fn username(&self) -> String {
        self.0.borrow().username().into()
    }

    #[boa(setter)]
    #[boa(method)]
    #[boa(rename = "username")]
    pub fn set_username(&self, value: String) -> JsResult<()> {
        // url::quirks::set_username(&mut self.0.borrow_mut(), &value);
        self.0
            .borrow_mut()
            .set_username(&value)
            .map_err(|_| js_error!(TypeError: "Failed to set username with value '{}'", value))
    }

    #[boa(rename = "toString")]
    pub fn print(&self) -> String {
        format!("{}", self.0.borrow())
    }
}

impl From<url::Url> for JsUrl {
    fn from(url: url::Url) -> Self {
        Self(Rc::new(RefCell::new(url)))
    }
}

impl From<JsUrl> for url::Url {
    fn from(url: JsUrl) -> url::Url {
        url.0.borrow().clone()
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::interceptor::js::tests::setup;
    use boa_engine::Source;

    #[test]
    fn url_constructor_without_base() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const u = new URL("http://example.com/a?x=1");
            assertEqual(u.href, "http://example.com/a?x=1", "href roundtrip");
            assertEqual(u.protocol, "http", "protocol");
            assertEqual(u.host, "example.com", "host");
            assertEqual(u.path, "/a", "path");
            assertEqual(u.search, "?x=1", "search");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn url_constructor_with_base() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const u = new URL("/x/y", "http://example.com/base");
            assertEqual(u.href, "http://example.com/x/y", "base-join");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn url_href_setter_parses() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const u = new URL("http://a/");
            u.href = "https://b.dev/p?q=1#h";
            assertEqual(u.href, "https://b.dev/p?q=1#h", "href set");
            assertEqual(u.protocol, "https", "proto updated");
            assertEqual(u.host, "b.dev", "host updated");
            assertEqual(u.path, "/p", "path updated");
            assertEqual(u.search, "?q=1", "search updated");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn url_protocol_get_set() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const u = new URL("http://x/");
            u.protocol = "https";
            assertEqual(u.protocol, "https", "protocol set");
            assertTrue(u.href.startsWith("https://"), "href reflects protocol");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn url_username_password_get_set() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const u = new URL("http://x/");
            u.username = "alice";
            u.password = "s3cr3t";
            assertEqual(u.username, "alice", "username");
            assertEqual(u.password, "s3cr3t", "password");
            assertTrue(u.href.startsWith("http://alice:s3cr3t@"), "href has creds");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn url_host_and_port_get_set() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const u = new URL("http://x/");
            u.host = "example.com:8080";
            assertEqual(u.host, "example.com:8080", "host with port");
            assertEqual(u.port, 8080, "port getter string");
            u.port = 9090;
            assertEqual(u.host, "example.com:9090", "host updated via port");
            assertEqual(u.port, 9090, "host updated via port");
            assertEqual(u.href, "http://example.com:9090/", "href reflects port");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn url_path_get_set() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const u = new URL("http://x/");
            u.path = "/api/v1";
            assertEqual(u.path, "/api/v1", "path set");
            assertEqual(u.href, "http://x/api/v1", "href reflects path");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn url_search_get_set() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const u = new URL("http://x/");
            u.search = "?a=1&b=2";
            assertEqual(u.search, "?a=1&b=2", "search set");
            u.search = "";
            assertEqual(u.search, "", "search cleared");
            assertTrue(!u.href.includes("?"), "href without search");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn url_searchparams_bridge_get() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const u = new URL("http://example.com/p?a=1&a=2&b=3");
            const sp = u.searchParams;
            assertEqual(sp.get("a"), "1", "first a");
            const all = sp.getAll("a");
            assertTrue(Array.isArray(all) && all.length === 2 && all[0] === "1" && all[1] === "2", "getAll");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn url_searchparams_bridge_set_append_delete_clear() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const u = new URL("http://example.com/p?a=1&x=9");
            const sp = u.searchParams;
            sp.set("a","42");
            assertEqual(sp.get("a"), "42", "set overrides");
            sp.append("a","99");
            const all = sp.getAll("a");
            assertTrue(all.length, 2 && all[0] === "42" && all[1] === "99", "append works");
            sp.delete("x");
            assertEqual(sp.get("x"), null, "delete removes");
            sp.clear();
            assertEqual(u.search, "", "clear removes all");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn url_to_string_matches_href() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const u = new URL("http://example.com/a?b=1#h");
            assertEqual(String(u), u.href, "String(u) equals href");
            assertEqual(u.toString(), u.href, "toString equals href");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn url_href_set_invalid_throws() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const u = new URL("http://ok/");
            let threw = false;
            try { u.href = "http://exa mple.com/"; } catch (e) { threw = true; }
            assertTrue(threw, "invalid href assertTrue throw");
        "#,
        ))
        .unwrap();
    }
}
