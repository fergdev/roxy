globalThis.Extensions = [
  {
    request(flow) {
      if (flow.request.body.len == 10) {
        flow.request.body.text = "len is 10 request";
      }
    },
    response(flow) {
      if (flow.response.body.len == 10) {
        flow.response.body.text = "len is 10 response";
      }
    }
  },
];
