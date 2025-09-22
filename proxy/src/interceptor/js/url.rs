use boa_engine::value::Convert;
use boa_engine::{Context, Finalize, JsData, JsResult, JsString, JsValue, Trace, js_error};
use boa_interop::{JsClass, js_class};
use cow_utils::CowUtils;
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;

use crate::interceptor::js::query::UrlSearchParams;
use crate::interceptor::util::set_url_authority;

#[derive(Debug, Clone, JsData, Trace, Finalize)]
#[boa_gc(unsafe_no_drop)]
pub(crate) struct JsUrl(#[unsafe_ignore_trace] Rc<RefCell<url::Url>>);

impl JsUrl {
    fn js_new(Convert(ref url): Convert<String>, base: Option<&Convert<String>>) -> JsResult<Self> {
        if let Some(Convert(base)) = base {
            let base_url = url::Url::parse(base)
                .map_err(|e| js_error!(TypeError: "Failed to parse base URL: {}", e))?;
            if base_url.cannot_be_a_base() {
                return Err(js_error!(TypeError: "Base URL {} cannot be a base", base));
            }

            let url = base_url
                .join(url)
                .map_err(|e| js_error!(TypeError: "Failed to parse URL: {}", e))?;
            Ok(Self(Rc::new(RefCell::new(url))))
        } else {
            let url = url::Url::parse(url)
                .map_err(|e| js_error!(TypeError: "Failed to parse URL: {}", e))?;
            Ok(Self(Rc::new(RefCell::new(url))))
        }
    }
}

impl Display for JsUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.borrow())
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

js_class! {
    class JsUrl as "URL" {
        property hash {
            fn get(this: JsClass<JsUrl>) -> JsString {
                JsString::from(url::quirks::hash(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<JsUrl>, value: Convert<String>) {
                url::quirks::set_hash(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property host {
            fn get(this: JsClass<JsUrl>) -> JsString {
                JsString::from(url::quirks::host(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<JsUrl>, value: Convert<String>) {
                let _ = url::quirks::set_host(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property host_name as "hostname" {
            fn get(this: JsClass<JsUrl>) -> JsString {
                JsString::from(url::quirks::hostname(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<JsUrl>, value: Convert<String>) {
                let _ = url::quirks::set_hostname(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property href {
            fn get(this: JsClass<JsUrl>) -> JsString {
                JsString::from(url::quirks::href(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<JsUrl>, value: Convert<String>) -> JsResult<()> {
                url::quirks::set_href(&mut this.borrow_mut().0.borrow_mut(), &value.0)
                    .map_err(|e| js_error!(TypeError: "Failed to set href: {}", e))
            }
        }

        property authority {
            fn get(this: JsClass<JsUrl>) -> JsString {
                let auth = this.borrow().0.borrow().authority().to_string();
                JsString::from(auth)
            }

            fn set(this: JsClass<JsUrl>, value: Convert<String>) -> JsResult<()> {
                let url = this.borrow_mut();
                set_url_authority(&mut url.0.borrow_mut(), &value.0)
                    .map_err(|e| js_error!(TypeError: "Failed to set authority: {}", e))
            }
        }

        property password {
            fn get(this: JsClass<JsUrl>) -> JsString {
                JsString::from(url::quirks::password(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<JsUrl>, value: Convert<String>) {
                let _ = url::quirks::set_password(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property path {
            fn get(this: JsClass<JsUrl>) -> JsString {
                JsString::from(url::quirks::pathname(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<JsUrl>, value: Convert<String>) {
                let () = url::quirks::set_pathname(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property port {
            fn get(this: JsClass<JsUrl>) -> JsValue {
                let port = this.borrow().0.borrow().port_or_known_default();
                JsValue::Integer(port.map(|p| p as i32).unwrap_or(0))
            }

            fn set(this: JsClass<JsUrl>, value: Convert<String>) {
                let _ = url::quirks::set_port(&mut this.borrow_mut().0.borrow_mut(), &value.0.to_string());
            }
        }

        property protocol {
            fn get(this: JsClass<JsUrl>) -> JsString {
                JsString::from(url::quirks::protocol(&this.borrow().0.borrow()).cow_replace(":", "").to_string())
            }

            fn set(this: JsClass<JsUrl>, value: Convert<String>) {
                let _ = url::quirks::set_protocol(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property search {
            fn get(this: JsClass<JsUrl>) -> JsString {
                JsString::from(url::quirks::search(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<JsUrl>, value: Convert<String>) {
                url::quirks::set_search(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property search_params as "searchParams" {
            fn get(this: JsClass<JsUrl>, context: &mut Context) -> JsResult<JsValue> {
                let url = this.borrow().0.clone();
                let params = UrlSearchParams { url };
                let obj = UrlSearchParams::from_data(params, context)?;
                Ok(obj.into())
            }
        }

        property username {
            fn get(this: JsClass<JsUrl>) -> JsString {
                JsString::from(this.borrow().0.borrow().username())
            }

            fn set(this: JsClass<JsUrl>, value: Convert<String>) {
                let _ = this.borrow_mut().0.borrow_mut().set_username(&value.0);
            }
        }

        constructor(url: Convert<String>, base: Option<Convert<String>>) {
            Self::js_new(url, base.as_ref())
        }

        fn to_string as "toString"(this: JsClass<JsUrl>) -> JsString {
            JsString::from(format!("{}", this.borrow().0.borrow()))
        }
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
            console.log("Starting test");
            const u = new URL("http://x/");
            u.host = "example.com:8080";
            assertEqual(u.host, "example.com:8080", "host with port");
            assertEqual(u.port, 8080, "port getter string");
            u.port = 9090;
            console.log("assertTrue2");
            assertEqual(u.host, "example.com:9090", "host updated via port");
            assertEqual(u.port, 9090, "host updated via port");
            assertEqual(u.href, "http://example.com:9090/", "href reflects port");
            console.log("assertTrue3");
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
