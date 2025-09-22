globalThis.extensions = [{
  request(flow) {
    if (flow.request.trailers.has("X-trailer1")) {
      flow.request.body.text = "has"
    }
  },
  response(flow) {
    if (flow.response.trailers.has("X-trailer1")) {
      flow.response.body.text = "has"
    }
  }
}];
