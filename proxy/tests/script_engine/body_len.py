from roxy import Extension


class BodyLen(Extension):
    def request(self, flow):
        if len(flow.request.body) == 10:
            flow.request.body.text = "len is 10 request"

    def response(self, flow):
        if len(flow.response.body) == 10:
            flow.response.body.text = "len is 10 response"


Extensions = [BodyLen()]
