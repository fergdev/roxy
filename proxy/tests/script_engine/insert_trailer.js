globalThis.Extensions = [{
  request(flow) {
    flow.request.trailers.append("set-cookie", "test-request");
  },
  response(flow) {
    flow.response.trailers.append("set-cookie", "test-response");
  }
}];
