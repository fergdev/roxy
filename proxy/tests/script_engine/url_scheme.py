class UrlScheme:
    def request(self, flow):
        if flow.request.url.scheme == "http":
            flow.request.url.scheme = "https"


Extensions = [UrlScheme()]
