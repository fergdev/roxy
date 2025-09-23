from roxy import Extension


class Counter(Extension):
    def __init__(self):
        self.num = 0

    def request(self, flow):
        self.num = self.num + 1

    def response(self, flow):
        self.num = self.num + 1


Extensions = [Counter()]
