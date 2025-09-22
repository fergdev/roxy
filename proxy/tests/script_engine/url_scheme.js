globalThis.extensions = [
  {
    request(flow) {
      if (flow.request.url.protocol == "http") {
        flow.request.url.protocol = "https";
      }
    },
  }
]
