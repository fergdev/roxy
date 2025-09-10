# Roxy

CLI MITM proxy written in Rust.

[![GitHub Pages](https://img.shields.io/badge/docs-GitHub%20Pages-blue?logo=github)](https://fergdev.github.io/roxy/)

## Status

Currently in a stabilization phase and looking to solidify the core features.

## Demo

<https://github.com/user-attachments/assets/a98a5a57-775d-424c-ab91-97d7a17793ea>

## 📖 Documentation

Full docs are available here:  
[Getting Started with Roxy](https://fergdev.github.io/roxy/)

## Testing

Run with

```bash
cargo run -- --port 6969 --script scripts/logger.lua
```

Or with debug logging:

```bash
RUST_LOG=debug RUST_BACKTRACE=0 cargo run -- --port 6969 --script scripts/logger.lua
```

```bash

### HTTP

```bash
curl -v --proxy http://localhost:6969 http://example.com
```

Tests require single thread to prevent too many open files error.

```bash
RUST_TEST_THREADS=1 cargo test
```

### HTTPS

```bash
export CURL_CA_BUNDLE=~/.roxy/roxy-ca-cert.pem
curl -v --proxy http://localhost:6969 https://example.com --insecure
```

## Goals

This is a learning project to explore building a fast, flexible proxy with scriptable interception and inspection capabilities. The goal is to create a tool that can be used for debugging, testing, and learning about network protocols.

Ideal goals is feature parity with existing tools like mitmproxy, but with a focus on performance, flexibility and style.

I love MITM proxy, but do not enjoy the CLI. I am aiming to improve on the CLI experience while maintaining the powerful features that make it great.

## Contributing

Contributing

This project is in its early stages, and contributions of all kinds are welcome — whether you’re fixing bugs, adding features, improving performance, or suggesting ideas.

The goal is to build a fast, flexible proxy with scriptable interception and inspection. If that sounds interesting to you, feel free to explore the codebase, open an issue, or start a pull request.

I would genuinely enjoy collaborating on this with others — whether you’re experimenting, learning, or building something serious. Almost any improvement is helpful right now. If you’re not sure where to begin, reach out.

## License

This project is licensed under the MIT license ([LICENSE] or <http://opensource.org/licenses/MIT>)

[LICENSE]: ./LICENSE
