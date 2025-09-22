globalThis.extensions = [{
  request(flow) {
    flow.request.trailers.delete("X-trailer1");
    flow.request.trailers.set("X-trailer2", undefined);
    flow.request.trailers.set("X-trailer3", null);
  },
  response(flow) {
    flow.response.trailers.delete("X-trailer1");
    flow.response.trailers.set("X-trailer2", undefined);
    flow.response.trailers.set("X-trailer3", null);
  }
}];
