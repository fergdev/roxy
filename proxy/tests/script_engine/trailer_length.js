globalThis.extensions = [{
  request(flow) {
    const t = flow.request.trailers;
    if (t.length == 12) {
      t.clear();
    }
  },
  response(flow) {
    const t = flow.response.trailers;
    if (t.length == 12) {
      t.clear();
    }
  }
}]
