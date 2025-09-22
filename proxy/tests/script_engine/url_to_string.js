globalThis.extensions = [{
  request(flow) {
    console.log(flow.request.url.toString())
    flow.request.body.text = flow.request.url.toString();
  }
}];
