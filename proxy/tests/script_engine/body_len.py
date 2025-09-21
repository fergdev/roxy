class BodyLen:
    def request(self, flow):
        len = flow.request.body.len()
        print("Request body length:", len)
        if len == 10:
            flow.request.body.text = "len is 10 request"

    def response(self, flow):
        len = flow.response.body.len()
        print("Response body length:", len)
        if len == 10:
            flow.response.body.text = "len is 10 response"


Extensions = [BodyLen()]
