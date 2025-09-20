globalThis.Extensions = [{
  request(flow) {
    flow.request.headers.delete("X-header1");
    flow.request.headers.set("X-header2", undefined);
    flow.request.headers.set("X-header3", null);
  },
  response(flow) {
    flow.response.headers.delete("X-header1");
    flow.response.headers.set("X-header2", undefined);
    flow.response.headers.set("X-header3", null);
  }
}];
