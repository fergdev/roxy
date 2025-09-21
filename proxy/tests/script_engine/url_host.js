globalThis.Extensions = [{
  request(flow) {
    console.log(flow.request.url.host);
    if (flow.request.url.host = "localhost:1234") {
      flow.request.url.host = "example.com:4321"
    }
    console.log(flow.request.url.host);
  },
}];
