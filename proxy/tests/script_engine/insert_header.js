globalThis.Extensions = [{
  request(flow) {
    flow.request.headers.append("set-cookie", "test-request");
  },
  response(flow) {
    flow.response.headers.append("set-cookie", "test-response");
  }
}];
