let count = 0
globalThis.extensions = [{
  start() {
    count = 10
  },
  request(flow) {
    flow.request.body.text = count;
    count += 1;
  },
  response(flow) {
    flow.response.body.text = count;
  }
}]
