class InsertHeader:
    def request(self, flow):
        flow.request.trailers.append("set-cookie", "test-request")

    def response(self, flow):
        flow.response.trailers.append("set-cookie", "test-response")


Extensions = [InsertHeader()]
