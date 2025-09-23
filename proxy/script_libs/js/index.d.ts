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

  enum Status {
    CONTINUE = 100,
    SWITCHING_PROTOCOLS = 101,
    PROCESSING = 102,
    OK = 200,
    CREATED = 201,
    ACCEPTED = 202,
    NON_AUTHORITATIVE_INFORMATION = 203,
    NO_CONTENT = 204,
    RESET_CONTENT = 205,
    PARTIAL_CONTENT = 206,
    MULTI_STATUS = 207,
    ALREADY_REPORTED = 208,
    IM_USED = 226,
    MULTIPLE_CHOICES = 300,
    MOVED_PERMANENTLY = 301,
    FOUND = 302,
    SEE_OTHER = 303,
    NOT_MODIFIED = 304,
    USE_PROXY = 305,
    TEMPORARY_REDIRECT = 307,
    PERMANENT_REDIRECT = 308,
    BAD_REQUEST = 400,
    UNAUTHORIZED = 401,
    PAYMENT_REQUIRED = 402,
    FORBIDDEN = 403,
    NOT_FOUND = 404,
    METHOD_NOT_ALLOWED = 405,
    NOT_ACCEPTABLE = 406,
    PROXY_AUTHENTICATION_REQUIRED = 407,
    REQUEST_TIMEOUT = 408,
    CONFLICT = 409,
    GONE = 410,
    LENGTH_REQUIRED = 411,
    PRECONDITION_FAILED = 412,
    PAYLOAD_TOO_LARGE = 413,
    URI_TOO_LONG = 414,
    UNSUPPORTED_MEDIA_TYPE = 415,
    RANGE_NOT_SATISFIABLE = 416,
    EXPECTATION_FAILED = 417,
    IM_A_TEAPOT = 418,
    MISDIRECTED_REQUEST = 421,
    UNPROCESSABLE_ENTITY = 422,
    LOCKED = 423,
    FAILED_DEPENDENCY = 424,
    TOO_EARLY = 425,
    UPGRADE_REQUIRED = 426,
    PRECONDITION_REQUIRED = 428,
    TOO_MANY_REQUESTS = 429,
    REQUEST_HEADER_FIELDS_TOO_LARGE = 431,
    UNAVAILABLE_FOR_LEGAL_REASONS = 451,
    INTERNAL_SERVER_ERROR = 500,
    NOT_IMPLEMENTED = 501,
    BAD_GATEWAY = 502,
    SERVICE_UNAVAILABLE = 503,
    GATEWAY_TIMEOUT = 504,
    HTTP_VERSION_NOT_SUPPORTED = 505,
    VARIANT_ALSO_NEGOTIATES = 506,
    INSUFFICIENT_STORAGE = 507,
    LOOP_DETECTED = 508,
    NOT_EXTENDED = 510,
    NETWORK_AUTHENTICATION_REQUIRED = 511,
  }

  var extensions: Extensions[];
}

export { };
