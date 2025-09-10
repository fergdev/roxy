globalThis.Extensions = [{
  request(flow) {
    globalThis.notify(1, "hi")
  },
  response(flow) {
    globalThis.notify(2, "there")
  }
}];
