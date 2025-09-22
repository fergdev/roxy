class DeleteQuery:
    def request(self, flow):
        if flow.request.url.search_params["foo"] == "bar":
            flow.request.url.search_params.delete("foo")


Extensions = [DeleteQuery()]
