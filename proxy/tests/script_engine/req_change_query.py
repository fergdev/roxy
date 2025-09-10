class ChangeQuery:
    def request(self, flow):
        flow.request.url.search_params.set("foo", "bar")
        flow.request.url.search_params.set("a", "b")
        flow.request.url.search_params.delete("no")
        flow.request.url.search_params.delete("yes")
        flow.request.url.search_params.delete("saison")


Extensions = [ChangeQuery()]
