globalThis.extensions = [{
  request(flow) {
    if (flow.request.method == "GET") {
      flow.request.method = "POST";
    }
  }
}];
