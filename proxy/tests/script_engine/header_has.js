globalThis.extensions = [{
  request(flow) {
    if (flow.request.headers.has("X-Header1")) {
      flow.request.body.text = "has"
    }
  },
  response(flow) {
    if (flow.response.headers.has("X-Header1")) {
      flow.response.body.text = "has"
    }
  }
}];
