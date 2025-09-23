from roxy import Extension


class HeadersToString(Extension):
    def request(self, flow):
        flow.request.body.text = str(flow.request.headers)

    def response(self, flow):
        flow.response.body.text = str(flow.response.headers)


Extensions = [HeadersToString()]
