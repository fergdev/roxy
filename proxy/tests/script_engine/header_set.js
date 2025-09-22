globalThis.extensions = [{
  request(flow) {
    flow.request.headers.set("X-Header1", "request");
  },
  response(flow) {
    flow.response.headers.set("X-Header1", "response");
  }
}];
