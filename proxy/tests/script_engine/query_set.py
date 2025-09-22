class AppendQuery:
    def request(self, flow):
        if flow.request.url.search_params["foo"] == "bar":
            flow.request.url.search_params.set("foo", "baz")


Extensions = [AppendQuery()]
