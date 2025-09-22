from roxy import Extension


class UrlScheme(Extension):
    def request(self, flow):
        if flow.request.url.protocol == "http":
            flow.request.url.protocol = "https"


Extensions = [UrlScheme()]
