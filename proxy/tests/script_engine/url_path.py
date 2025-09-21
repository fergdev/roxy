class UrlPath:
    def request(self, flow):
        if flow.request.url.path == "/some/path":
            flow.request.url.path = "/another/path"


Extensions = [UrlPath()]
