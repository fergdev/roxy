class LengthHeader:
    def request(self, flow):
        if len(flow.request.headers) == 12:
            flow.request.headers.clear()

    def response(self, flow):
        if len(flow.response.headers) == 12:
            flow.response.headers.clear()


Extensions = [LengthHeader()]
