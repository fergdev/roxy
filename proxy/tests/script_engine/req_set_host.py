class SetHost:
    def request(self, flow):
        flow.request.url.host = "example.com"


Extensions = [SetHost()]
