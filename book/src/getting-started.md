# Getting Started

Welcome to **Roxy** — a programmable MITM proxy for HTTP(S), HTTP/2, HTTP/3, WebSockets, and more.  
Roxy makes it easy to inspect, rewrite, and automate traffic using **Rust**, **Lua**, **JavaScript**, or **Python** scripting engines.

---

## Prerequisites

- [Rust](https://www.rust-lang.org/) 1.80 or newer  
- [Cargo](https://doc.rust-lang.org/cargo/)  
- (optional) [Node.js](https://nodejs.org/) if you want to test JavaScript interceptors  
- (optional) [Python 3](https://www.python.org/) if you want Python scripting  
- (optional) [Lua 5.4](https://www.lua.org/) for Lua scripting  

---

## Installation

Clone the repository and build from source:

```sh
git clone https://github.com/fergdev/roxy.git
cd roxy
cargo build --release
