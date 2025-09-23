from roxy import Extension


class UrlUsername(Extension):
    def request(self, flow):
        if flow.request.url.username == "dave":
            flow.request.url.username = "damo"


Extensions = [UrlUsername()]
