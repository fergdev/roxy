globalThis.Extensions = [{
  request(flow) {
    if (flow.request.url.hostname = "localhost") {
      flow.request.url.hostname = "example.com"
    }
  },
}];
