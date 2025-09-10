globalThis.Extensions = [{
  request(flow) {
    flow.request.url.searchParams.set("foo", "bar");
    flow.request.url.searchParams.set("a", "b");
    flow.request.url.searchParams.delete("no");
    flow.request.url.searchParams.delete("yes");
    flow.request.url.searchParams.delete("saison");
  }
}];
