class UrlPort:
    def request(self, flow):
        if flow.request.url.port == 80:
            flow.request.url.port = 8080


Extensions = [UrlPort()]
