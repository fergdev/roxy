globalThis.extensions = [{
  request(flow) {
    if (flow.request.url.searchParams.get("foo") == "bar") {
      flow.request.url.searchParams.append("foo", "baz");
    }
  }
}];
