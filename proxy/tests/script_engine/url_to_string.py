from roxy import Extension


class UrlToString(Extension):
    def request(self, flow):
        flow.request.body.text = str(flow.request.url)


Extensions = [UrlToString()]
