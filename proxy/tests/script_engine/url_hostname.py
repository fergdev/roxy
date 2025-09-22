from roxy import Extension


class UrlHost(Extension):
    def request(self, flow):
        if flow.request.url.hostname == "localhost":
            flow.request.url.hostname = "example.com"


Extensions = [UrlHost()]
