globalThis.extensions = [{
  request(flow) {
    flow.response.body.text = "early return"
  }
}];
