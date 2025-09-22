globalThis.extensions = [{
  request(flow) {
    flow.request.headers.clear();
  },
  response(flow) {
    flow.response.headers.clear();
  }
}];
