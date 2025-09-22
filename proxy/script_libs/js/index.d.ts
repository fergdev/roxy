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
    protocol: Protocol | undefined;
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
    HTTP0_9 = "HTTP/0.9",
    HTTP1_0 = "HTTP/1.0",
    HTTP1_1 = "HTTP/1.1",
    HTTP2_0 = "HTTP/2.0",
    HTTP3_0 = "HTTP/3.0",
  }

  enum Method {
    CONNECT = "CONNECT",
    DELETE = "DELETE",
    GET = "GET",
    HEAD = "HEAD",
    OPTIONS = "OPTIONS",
    PATCH = "PATCH",
    POST = "POST",
    PUT = "PUT",
    TRACE = "TRACE",
  }
  enum Protocol {
    HTTP = "http",
    HTTPS = "https",
  }

  var extensions: Extensions[];
}

export { };
