from roxy import Extension


class SetTrailer(Extension):
    def request(self, flow):
        flow.request.trailers["X-Trailer1"] = "request"

    def response(self, flow):
        flow.response.trailers["X-Trailer1"] = "response"


Extensions = [SetTrailer()]
