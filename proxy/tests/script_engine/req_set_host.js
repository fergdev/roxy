globalThis.Extensions = [{
  request(flow) {
    flow.request.url.host = "example.com";
  }
}];
