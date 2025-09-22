globalThis.extensions = [{
  response(flow) {
    flow.response.status = 404;
  }
}];
