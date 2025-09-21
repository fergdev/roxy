# Scripting Reference

Roxy includes a composable scripting system so you can extend, automate and prototype protocol logic without rebuilding the proxy. Roxy exposes three script engines that cover a wide range of use cases:

- JavaScript — a familiar, full-featured runtime for ecosystem libraries and concise logic.
- Lua — lightweight and fast for tiny hooks and quick iteration.
- Python — expressive and powerful for complex processing and integrations.

This section explains the extensions model, shows concise examples in each language, and gives practical tips so you can copy/paste and get started quickly.

## Core concepts

- Extensions / scripts register callbacks for events (for example request, response, connect, tls_handshake) and can inspect, mutate, or replace flows.
- Options are configuration knobs a script can expose; Roxy surfaces those in the config file, CLI, and UI.
- Commands are functions a script exposes that users can invoke interactively (or bind to keys).
- Scripts can be loaded at startup or attached dynamically depending on your run mode.
- Scripts run in their engine sandbox and are invoked for each matching event.

Security note: scripts can access request/response data and, depending on host policy, filesystem or network APIs. Treat third-party scripts like local code — review them before enabling in shared environments.

## Running scripts

Roxy accepts scripts on the CLI or via roxy.toml:

```bash
roxy --script ./examples/extensions/counter.py
```

Anatomy of an extension.

A Roxy extension is just a script implementing one or more event handlers. Handlers are ordinary functions (or methods on an exported object) named for the event they handle.

Common events:

- start() / stop() — lifecycle hooks
- request(flow) — before a request is sent upstream
- response(flow) — after a response is received (before returning to client)
- connect(flow) / tls_handshake(flow) — low-level transport events
- error(ctx) — runtime errors or engine-level notifications

Roxy converts types to idiomatic host-language objects (tables in Lua, dict-like objects in Python, plain objects in JS). The API surface aims to be consistent across engines.

## Counter example

{{#tabs global="language"}}
{{#tab name=JS}}

```js
module.exports = {
  start() {
    this.count = 0;
    console.log("counter started");
  },

  request(flow) {
    this.count += 1;
    console.log(`seen ${this.count} requests`);
  }
};
```

{{#endtab}}
{{#tab name=Lua}}

```lua
local count = 0
function request(flow)
  count = count + 1
  flow.request.headers["x-roxy-example"] = "hello-from-roxy"
end
Extensions = {
 {
  request = request,
 },
}
```

{{#endtab}}
{{#tab name=Python}}

```python
import logging

class Counter:
    def __init__(self):
        self.count = 0

    def start(self):
        logging.info("counter started")
        self.count = 0

    def request(self, flow):
        self.count += 1
        logging.info("seen %d requests", self.count)

Extensions = [Counter()]
```

{{#endtab}}
{{#endtabs}}

## Flow object basics

All engines receive a flow object representing a transaction. Typical fields and helpers:

- flow.request — request (method, url, headers, body, scheme, host, port)
- flow.response — response (status_code, headers, body)
- flow.client_addr, flow.server_conn — transport metadata
- Helpers: flow.reply(), flow.kill(), flow.replace(), flow.resume() — control flow lifecycle

Bindings convert types to idiomatic objects:

- Python: mapping-like headers, bytes-like bodies
- JS: plain objects and strings/ArrayBuffers
- Lua: table-like headers and strings

## Identical behavior in 3 languages

Add header x-roxy-example: true to every request.

{{#tabs global="language"}}
{{#tab name=JS}}

```js
function request(flow) {
  flow.request.headers["x-roxy-example"] = "true";
}
module.exports = { request };
```

{{#endtab}}
{{#tab name=Lua}}

```lua
function request(flow)
  flow.request.headers["x-roxy-example"] = "true"
end
```

{{#endtab}}
{{#tab name=Python}}

```py
def request(flow):
    flow.request.headers["x-roxy-example"] = "true"
```

{{#endtab}}
{{#endtabs}}

## Debugging and tips

- Use the engine’s logging APIs (console.log, print, Python logging) to surface runtime info.
- Start with module-level handlers for quick iteration; move to object/class form for stateful addons.
- Keep scripts focused — split complex logic across multiple scripts.
- For high-performance paths, prefer Lua or precompiled JS; heavy CPU work should run in native code or an external service.
- Be mindful of concurrency: handlers may be invoked from multiple workers; rely on documented concurrency rules or use engine-provided sync primitives.

## Packaging & sharing

- Store small scripts in examples/addons/ inside the repo for easy teammate access.
- Version scripts and document their options/commands so upgrades are predictable.
- Consider providing a roxy-scripts collection with utilities and standard helpers for manipulating flows.

## Quick checklist for adding a script to the repo

 1. Add the script under examples/addons/ (e.g., examples/addons/add_header.py).
 2. Test locally:

```bash
roxy --script ./examples/addons/add_header.py
```

 3. Verify behavior with a curl request through Roxy:

```bash
curl --proxy 127.0.0.1:8080 -v <https://example.com/>
```

 4. Commit and add a short README describing the script, its options, and any commands.
