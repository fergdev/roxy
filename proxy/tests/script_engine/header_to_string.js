globalThis.extensions = [{
  request(flow) {
    flow.request.body.text = flow.request.headers.toString();
  },
  response(flow) {
    flow.response.body.text = flow.response.headers.toString();
  }
}];
