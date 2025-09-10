globalThis.Extensions = [{
  request(flow) {
    flow.response.body.text = "early return"
  }
}];
