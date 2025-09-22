globalThis.extensions = [{
  response(flow) {
    if (flow.request.url.host === "example.com") {
      flow.response.body.text = "intercepted"
    }
  }
}];
