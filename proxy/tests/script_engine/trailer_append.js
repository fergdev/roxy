globalThis.extensions = [{
  request(flow) {
    flow.request.trailers.append("X-Trailer1", "request");
    flow.request.trailers.append("X-Trailer9", "request");
  },
  response(flow) {
    flow.response.trailers.append("X-Trailer1", "response");
    flow.response.trailers.append("X-Trailer9", "response");
  }
}];
