globalThis.Extensions = [
  {
    request(flow) {
      if (flow.request.url.port == 80) {
        flow.request.url.port = 8080
      }
    },
  }
]
