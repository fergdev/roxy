globalThis.Extensions = [
  {
    request(flow) {
      if (flow.request.url.username == "dave") {
        flow.request.url.username = "damo";
      }
    },
  }
]
