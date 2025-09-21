globalThis.Extensions = [
  {
    request(flow) {
      if (flow.request.url.authority == "dave:1234@localhost:1234") {
        flow.request.url.authority = "damo:abcd@localhost:4321";
      }
    },
  }
]
