globalThis.extensions = [{
  request(flow) {
    flow.request.body.text = flow.request.trailers.toString();
  },
  response(flow) {
    flow.response.body.text = flow.response.trailers.toString();
  }
}];
