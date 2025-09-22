class ClearTrailer:
    def request(self, flow):
        flow.request.trailers.clear()

    def response(self, flow):
        flow.response.trailers.clear()


Extensions = [ClearTrailer()]
