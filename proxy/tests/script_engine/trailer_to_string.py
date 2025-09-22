class TrailersToString:
    def request(self, flow):
        flow.request.body.text = str(flow.request.trailers)

    def response(self, flow):
        flow.response.body.text = str(flow.response.trailers)


Extensions = [TrailersToString()]
