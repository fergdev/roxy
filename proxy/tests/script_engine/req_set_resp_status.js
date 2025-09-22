globalThis.extensions = [{
  request(flow) {
    flow.response.status = 404
  }
}];
