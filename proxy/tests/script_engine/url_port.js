globalThis.extensions = [
  {
    request(flow) {
      if (flow.request.url.port == 1234) {
        flow.request.url.port = 8080
      }
    },
  }
]
