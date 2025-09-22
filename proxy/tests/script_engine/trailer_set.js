globalThis.extensions = [{
  request(flow) {
    flow.request.trailers.set("X-trailer1", "request");
  },
  response(flow) {
    flow.response.trailers.set("X-trailer1", "response");
  }
}];
