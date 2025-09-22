class ClearHeader:
    def request(self, flow):
        flow.request.headers.clear()

    def response(self, flow):
        flow.response.headers.clear()


Extensions = [ClearHeader()]
