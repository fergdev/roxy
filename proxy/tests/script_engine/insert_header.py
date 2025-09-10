class InsertHeader:
    def request(self, flow):
        flow.request.headers.append("set-cookie", "test-request")

    def response(self, flow):
        flow.response.headers.append("set-cookie", "test-response")


Extensions = [InsertHeader()]
