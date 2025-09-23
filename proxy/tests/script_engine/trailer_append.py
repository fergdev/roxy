from roxy import Extension


class AppendHeader(Extension):
    def request(self, flow):
        flow.request.trailers.append("X-Trailer1", "request")
        flow.request.trailers.append("X-Trailer9", "request")

    def response(self, flow):
        flow.response.trailers.append("X-Trailer1", "response")
        flow.response.trailers.append("X-Trailer9", "response")


Extensions = [AppendHeader()]
