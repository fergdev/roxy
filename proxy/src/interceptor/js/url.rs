use boa_engine::value::Convert;
use boa_engine::{Context, Finalize, JsData, JsResult, JsString, JsValue, Trace, js_error};
use boa_interop::{JsClass, js_class};
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;

use crate::interceptor::js::query::UrlSearchParams;

#[derive(Debug, Clone, JsData, Trace, Finalize)]
#[boa_gc(unsafe_no_drop)]
pub(crate) struct Url(#[unsafe_ignore_trace] Rc<RefCell<url::Url>>);

impl Url {
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

impl Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.borrow())
    }
}

impl From<url::Url> for Url {
    fn from(url: url::Url) -> Self {
        Self(Rc::new(RefCell::new(url)))
    }
}

impl From<Url> for url::Url {
    fn from(url: Url) -> url::Url {
        url.0.borrow().clone()
    }
}

js_class! {
    class Url as "URL" {
        property hash {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::hash(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<String>) {
                url::quirks::set_hash(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property host {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::host(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<String>) {
                let _ = url::quirks::set_host(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property href {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::href(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<String>) -> JsResult<()> {
                url::quirks::set_href(&mut this.borrow_mut().0.borrow_mut(), &value.0)
                    .map_err(|e| js_error!(TypeError: "Failed to set href: {}", e))
            }
        }

        property origin {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::origin(&this.borrow().0.borrow()))
            }
        }

        property password {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::password(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<String>) {
                let _ = url::quirks::set_password(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property path {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::pathname(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<String>) {
                let () = url::quirks::set_pathname(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property port {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::port(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<JsString>) {
                let _ = url::quirks::set_port(&mut this.borrow_mut().0.borrow_mut(), &value.0.to_std_string_lossy());
            }
        }

        property protocol {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::protocol(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<String>) {
                let _ = url::quirks::set_protocol(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property search {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::search(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<String>) {
                url::quirks::set_search(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property search_params as "searchParams" {
            fn get(this: JsClass<Url>, context: &mut Context) -> JsResult<JsValue> {
                let url = this.borrow().0.clone();
                let params = UrlSearchParams { url };
                let obj = UrlSearchParams::from_data(params, context)?;
                Ok(obj.into())
            }
        }

        property username {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(this.borrow().0.borrow().username())
            }

            fn set(this: JsClass<Url>, value: Convert<String>) {
                let _ = this.borrow_mut().0.borrow_mut().set_username(&value.0);
            }
        }

        constructor(url: Convert<String>, base: Option<Convert<String>>) {
            Self::js_new(url, base.as_ref())
        }

        fn to_string as "toString"(this: JsClass<Url>) -> JsString {
            JsString::from(format!("{}", this.borrow().0.borrow()))
        }
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::{init_test_logging, interceptor::js::engine::register_classes};
    use boa_engine::{Context, Source};

    fn setup() -> Context {
        init_test_logging();
        let mut ctx = Context::default();
        ctx.eval(Source::from_bytes(
            r#"function must(c,m){ if(!c) throw new Error(m||"assert"); }"#,
        ))
        .unwrap();
        register_classes(&mut ctx).unwrap();
        ctx
    }

    #[test]
    fn url_constructor_without_base() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const u = new URL("http://example.com/a?x=1#frag");
            must(u.href === "http://example.com/a?x=1#frag", "href roundtrip");
            must(u.protocol === "http:", "protocol");
            must(u.host === "example.com", "host");
            must(u.path === "/a", "path");
            must(u.search === "?x=1", "search");
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
            must(u.href === "http://example.com/x/y", "base-join");
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
            must(u.href === "https://b.dev/p?q=1#h", "href set");
            must(u.protocol === "https:", "proto updated");
            must(u.host === "b.dev", "host updated");
            must(u.path === "/p", "path updated");
            must(u.search === "?q=1", "search updated");
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
            u.protocol = "https:";
            must(u.protocol === "https:", "protocol set");
            must(u.href.startsWith("https://"), "href reflects protocol");
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
            must(u.username === "alice", "username");
            must(u.password === "s3cr3t", "password");
            must(u.href.startsWith("http://alice:s3cr3t@"), "href has creds");
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
            must(u.host === "example.com:8080", "host with port");
            must(u.port === "8080", "port getter string");
            // update port only
            u.port = "9090";
            must(u.host === "example.com:9090", "host updated via port");
            must(u.href === "http://example.com:9090/", "href reflects port");
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
            must(u.path === "/api/v1", "path set");
            must(u.href === "http://x/api/v1", "href reflects path");
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
            must(u.search === "?a=1&b=2", "search set");
            // clearing
            u.search = "";
            must(u.search === "", "search cleared");
            must(!u.href.includes("?"), "href without search");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn url_origin_is_readonly() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const u = new URL("http://example.com:1234/x");
            must(u.origin === "http://example.com:1234", "origin value");
            u.origin = "http://nope"; 
            must(u.origin === "http://example.com:1234", "origin value");
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
            must(sp.get("a") === "1", "first a");
            const all = sp.getAll("a");
            must(Array.isArray(all) && all.length === 2 && all[0] === "1" && all[1] === "2", "getAll");
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
            must(sp.get("a") === "42", "set overrides");
            sp.append("a","99");
            const all = sp.getAll("a");
            must(all.length === 2 && all[0] === "42" && all[1] === "99", "append works");
            sp.delete("x");
            must(sp.get("x") === null, "delete removes");
            sp.clear();
            must(u.search === "", "clear removes all");
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
            must(String(u) === u.href, "String(u) equals href");
            must(u.toString() === u.href, "toString equals href");
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
            must(threw, "invalid href must throw");
        "#,
        ))
        .unwrap();
    }
}
