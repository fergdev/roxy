class AppendHeader:
    def request(self, flow):
        flow.request.headers.append("X-Header1", "request")
        flow.request.headers.append("X-Header9", "request")

    def response(self, flow):
        flow.response.headers.append("X-Header1", "response")
        flow.response.headers.append("X-Header9", "response")


Extensions = [AppendHeader()]
