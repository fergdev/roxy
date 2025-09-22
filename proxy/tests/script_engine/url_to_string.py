class UrlToString:
    def request(self, flow):
        flow.request.body.text = str(flow.request.url)


Extensions = [UrlToString()]
