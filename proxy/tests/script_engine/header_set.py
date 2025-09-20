class AppendHeader:
    def request(self, flow):
        flow.request.headers.set("X-Header1", "request")

    def response(self, flow):
        flow.response.headers.set("X-Header1", "response")


Extensions = [AppendHeader()]
