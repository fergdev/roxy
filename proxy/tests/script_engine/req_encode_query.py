class EncodeQuery:
    def request(self, flow):
        flow.request.url.search_params.set("foo", "bar & baz")
        flow.request.url.search_params.set("saison", "Été+hiver")


Extensions = [EncodeQuery()]
