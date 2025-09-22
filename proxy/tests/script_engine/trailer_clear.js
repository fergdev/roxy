globalThis.extensions = [{
  request(flow) {
    flow.request.trailers.clear();
  },
  response(flow) {
    flow.response.trailers.clear();
  }
}]

