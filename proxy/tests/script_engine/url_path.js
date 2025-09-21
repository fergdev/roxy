globalThis.Extensions = [
  {
    request(flow) {
      if (flow.request.url.path == "/some/path") {
        flow.request.url.path = "/another/path";
      }
    }
  }
]
