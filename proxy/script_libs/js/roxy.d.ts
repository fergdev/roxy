declare global {
  interface Extensions {
    request?(flow: Flow): void;
    response?(flow: Flow): void;
  }

  interface Extension {
    start(): void;
    request(flow: Flow): void;
    response(flow: Flow): void;
    stop(): void;
  }

  interface Flow {
    request: Request;
    response: Response | undefined;
  }

  interface Request {
    url: Url;
    method: Method;
    headers: Headers;
    body: Body;
    version: Version,
    method: Method,
    trailers: Headers | undefined;
  }

  interface Url {
    protocol: String | undefined;
    authority: String | undefined;
    username: String | undefined;
    password: String | undefined;
    hostname: String | undefined;
    host: String | undefined;
    port: number | undefined;
    path: String | undefined;
    search: String;
    searchParams: URLSearchParams;
  }
  interface URLSearchParams {

  }
  interface Response {
    statusCode: number;
    version: Version,
    headers: Headers;
    body: Body;
    trailers: Headers | undefined;
  }

  interface Headers {
    isEmpty: boolean;
    clear(): void;
    append(name: string, value: string): void;
    get(name: string): string | undefined;
    set(name: string, value: string): void;
    delete(name: string): void;
    has(name: string): boolean;
    length: number;
    toString(): string;
  }

  interface Body {
    text: string;
    raw: Uint8Array;
    length: number;
    clear(): void;
    isEmpty(): boolean;
  }

  enum Version {
    Http0_9 = 0,
    Http1_0 = 1,
    Http1_1 = 2,
    Http2_0 = 3,
    Http3_0 = 4,
  }

  enum Method {
    Options = 0,
    Get = 1,
    Post = 2,
    Put = 3,
    Delete = 4,
    Head = 5,
    Trace = 6,
    Connect = 7,
    Patch = 8,
  }

  var extensions: Extensions[];
}

export { };
