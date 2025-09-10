globalThis.Extensions = [{
  request(flow) {
    flow.request.version = "HTTP/3.0";
  },
  response(flow) {
    flow.response.version = "HTTP/3.0";
  }
}];
