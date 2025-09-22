from roxy import Extension


class UrlScheme(Extension):
    def request(self, flow):
        if flow.request.url.scheme == "http":
            flow.request.url.scheme = "https"


Extensions = [UrlScheme()]
