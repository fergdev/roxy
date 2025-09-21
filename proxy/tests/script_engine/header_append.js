globalThis.Extensions = [{
  request(flow) {
    flow.request.headers.append("X-Header1", "request");
    flow.request.headers.append("X-Header9", "request");
  },
  response(flow) {
    flow.response.headers.append("X-Header1", "response");
    flow.response.headers.append("X-Header9", "response");
  }
}];
