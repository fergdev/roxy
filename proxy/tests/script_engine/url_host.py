from roxy import Extension


class UrlHost(Extension):
    def request(self, flow):
        if flow.request.url.host == "localhost:1234":
            flow.request.url.host = "example.com:4321"


Extensions = [UrlHost()]
