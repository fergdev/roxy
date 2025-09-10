class SetVersion:
    def request(self, flow):
        flow.request.version = "HTTP/3.0"

    def response(self, flow):
        flow.response.version = "HTTP/3.0"


Extensions = [SetVersion()]
