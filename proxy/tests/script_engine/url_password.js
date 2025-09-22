globalThis.extensions = [
  {
    request(flow) {
      if (flow.request.url.password == "1234") {
        flow.request.url.password = "abcd";
      }
    },
  }
]
