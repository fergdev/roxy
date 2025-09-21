globalThis.Extensions = [
  {
    request(flow) {
      flow.request.body.clear();
    },
    response(flow) {
      flow.response.body.clear();
    }
  },
];
