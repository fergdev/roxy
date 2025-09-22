class UrlPort:
    def request(self, flow):
        if flow.request.url.port == 1234:
            flow.request.url.port = 8080


Extensions = [UrlPort()]
