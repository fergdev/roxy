from roxy import Extension


class LengthTrailers(Extension):
    def request(self, flow):
        t = flow.request.trailers
        if len(t) == 12:
            t.clear()

    def response(self, flow):
        t = flow.response.trailers
        if len(t) == 12:
            t.clear()


Extensions = [LengthTrailers()]
