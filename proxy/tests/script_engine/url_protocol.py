from roxy import Extension, Protocol


class UrlScheme(Extension):
    def request(self, flow):
        print("protocol " + str(flow.request.url.protocol) + " " + str(Protocol.HTTP))
        if flow.request.url.protocol == Protocol.HTTP:
            print("setting https")
            flow.request.url.protocol = Protocol.HTTPS
        else:
            print("not eq")


Extensions = [UrlScheme()]
