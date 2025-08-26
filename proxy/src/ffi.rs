#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use once_cell::sync::OnceCell;
use roxy_shared::{generate_roxy_root_ca_with_path, tls::TlsConfig};
use std::{ffi::CString, os::raw::c_char, ptr};
use tokio::runtime::Runtime;

use crate::{flow::FlowStore, interceptor::ScriptEngine, proxy::ProxyManager};

static RT: OnceCell<Runtime> = OnceCell::new();

#[unsafe(no_mangle)]
pub extern "C" fn rxy_init_runtime() -> i32 {
    let _ = RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime")
    });
    0
}

// Simple per-thread error string
thread_local! {
    static LAST_ERR: std::cell::RefCell<Option<CString>> = const { std::cell::RefCell::new(None) };
}
fn set_err(s: String) {
    LAST_ERR.with(|le| {
        *le.borrow_mut() = Some(CString::new(s).unwrap_or_else(|_| CString::new("err").unwrap()))
    });
}
#[unsafe(no_mangle)]
pub extern "C" fn rxy_last_error_message() -> *const c_char {
    LAST_ERR.with(|le| {
        le.borrow()
            .as_ref()
            .map(|c| c.as_ptr())
            .unwrap_or(ptr::null())
    })
}

#[repr(C)]
pub struct RoxyProxyHandle(*mut ProxyManager);

unsafe fn as_mut<'a, T>(p: *mut T) -> &'a mut T {
    &mut *p
}

fn build_pm(port: u16) -> Result<ProxyManager, String> {
    // TODO: replace with your real init
    // let certs = load_native_certs();
    let ca = generate_roxy_root_ca_with_path(None).map_err(|e| e.to_string())?;
    let script_engine = ScriptEngine::new_no_watch();
    let tls_config = TlsConfig::default();
    let flow_store = FlowStore::default();
    Ok(ProxyManager::new(
        port,
        ca,
        script_engine,
        tls_config,
        flow_store,
    ))
}

#[unsafe(no_mangle)]
pub extern "C" fn rxy_proxy_new_with_defaults(port: u16) -> *mut ProxyManager {
    match build_pm(port) {
        Ok(pm) => Box::into_raw(Box::new(pm)),
        Err(e) => {
            set_err(e);
            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rxy_proxy_free(handle: *mut ProxyManager) {
    if !handle.is_null() {
        // Dropping will abort the internal tasks via Drop impl you already wrote
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rxy_proxy_start_all(handle: *mut ProxyManager) -> i32 {
    if handle.is_null() {
        set_err("null handle".into());
        return -1;
    }
    let rt = RT.get().expect("call rxy_init_runtime first");
    let pm = unsafe { as_mut(handle) };
    match rt.block_on(pm.start_all()) {
        Ok(()) => 0,
        Err(e) => {
            set_err(format!("{e:?}"));
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rxy_proxy_start_tcp(handle: *mut ProxyManager, port: u16) -> i32 {
    if handle.is_null() {
        set_err("null handle".into());
        return -1;
    }
    let rt = RT.get().expect("call rxy_init_runtime first");

    // Create listener inside the lib (FFI can’t build TcpListener)
    match rt.block_on(async {
        use tokio::net::TcpListener;
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
        unsafe { as_mut(handle) }.start_tcp(listener).await
    }) {
        Ok(()) => 0,
        Err(e) => {
            set_err(format!("{e:?}"));
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rxy_proxy_start_udp(handle: *mut ProxyManager, port: u16) -> i32 {
    if handle.is_null() {
        set_err("null handle".into());
        return -1;
    }
    let rt = RT.get().expect("call rxy_init_runtime first");
    match rt.block_on(async {
        use std::net::UdpSocket;
        let sock = UdpSocket::bind((std::net::Ipv4Addr::LOCALHOST, port))?;
        unsafe { as_mut(handle) }.start_udp(sock).await
    }) {
        Ok(()) => 0,
        Err(e) => {
            set_err(format!("{e:?}"));
            -1
        }
    }
}
