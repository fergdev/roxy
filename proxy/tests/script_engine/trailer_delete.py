from roxy import Extension


class Deletetrailer(Extension):
    def request(self, flow):
        flow.request.trailers.delete("X-trailer1")
        flow.request.trailers["X-trailer2"] = None
        del flow.request.trailers["X-trailer3"]

    def response(self, flow):
        flow.response.trailers.delete("X-trailer1")
        flow.response.trailers["X-trailer2"] = None
        del flow.response.trailers["X-trailer3"]


Extensions = [Deletetrailer()]
