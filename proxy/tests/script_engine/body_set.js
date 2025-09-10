globalThis.Extensions = [{
  request(flow) {
    flow.request.body.text = "rewrite request";
  },
  response(flow) {
    flow.response.body.text = "rewrite response";
  }
}];
