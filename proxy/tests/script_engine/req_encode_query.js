globalThis.Extensions = [{
  request(flow) {
    flow.request.url.searchParams.set("foo", "bar & baz")
    flow.request.url.searchParams.set("saison", "Été+hiver")
  }
}];
