globalThis.extensions = [{
  request(flow) {
    flow.request.body.text = flow.request.url.searchParams.toString();
  }
}];
