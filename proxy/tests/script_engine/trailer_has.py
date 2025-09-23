from roxy import Extension


class HasTrailer(Extension):
    def request(self, flow):
        if flow.request.trailers.has("X-trailer1"):
            flow.request.body.text = "has"

    def response(self, flow):
        if flow.response.trailers.has("X-trailer1"):
            flow.response.body.text = "has"


Extensions = [HasTrailer()]
