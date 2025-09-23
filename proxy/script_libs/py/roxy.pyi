import enum
from typing import Optional, Protocol as ProtocolType, runtime_checkable, List

class Body:
    text: str
    bytes: bytes

    def clear(self) -> None: ...
    def __len__(self) -> int: ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

class Headers:
    def get(self, key: str) -> Optional[str]: ...
    def set(self, key: str, value: str) -> None: ...
    def append(self, key: str, value: str) -> None: ...
    def delete(self, key: str) -> None: ...
    def has(self, key: str) -> bool: ...
    def clear(self) -> None: ...
    def __len__(self) -> int: ...
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...
    def __getitem__(self, key: str) -> None: ...
    def __setitem__(self, key: str, value: str | None) -> None: ...
    def __delitem__(self, key: str) -> None: ...

class Request:
    method: Method
    url: Url
    version: Version
    headers: Headers
    trailers: Headers
    body: Body
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

class Version(enum.Enum):
    HTTP0_9 = ("HTTP/0.9",)
    HTTP1_0 = ("HTTP/1.0",)
    HTTP1_1 = ("HTTP/1.1",)
    HTTP2_0 = ("HTTP/2.0",)
    HTTP3 = ("HTTP/3.0",)
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

class Method(enum.Enum):
    CONNECT = ("CONNECT",)
    DELETE = ("DELETE",)
    GET = ("GET",)
    HEAD = ("HEAD",)
    OPTIONS = ("OPTIONS",)
    PATCH = ("PATCH",)
    POST = ("POST",)
    PUT = ("PUT",)
    TRACE = ("TRACE",)
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

class Protocol(enum.Enum):
    HTTP = ("http",)
    HTTPS = ("https",)
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

class Status(enum.Enum):
    CONTINUE = (100,)
    SWITCHING_PROTOCOLS = (101,)
    PROCESSING = (102,)
    OK = (200,)
    CREATED = (201,)
    ACCEPTED = (202,)
    NON_AUTHORITATIVE_INFORMATION = (203,)
    NO_CONTENT = (204,)
    RESET_CONTENT = (205,)
    PARTIAL_CONTENT = (206,)
    MULTI_STATUS = (207,)
    ALREADY_REPORTED = (208,)
    IM_USED = (226,)
    MULTIPLE_CHOICES = (300,)
    MOVED_PERMANENTLY = (301,)
    FOUND = (302,)
    SEE_OTHER = (303,)
    NOT_MODIFIED = (304,)
    USE_PROXY = (305,)
    TEMPORARY_REDIRECT = (307,)
    PERMANENT_REDIRECT = (308,)
    BAD_REQUEST = (400,)
    UNAUTHORIZED = (401,)
    PAYMENT_REQUIRED = (402,)
    FORBIDDEN = (403,)
    NOT_FOUND = (404,)
    METHOD_NOT_ALLOWED = (405,)
    NOT_ACCEPTABLE = (406,)
    PROXY_AUTHENTICATION_REQUIRED = (407,)
    REQUEST_TIMEOUT = (408,)
    CONFLICT = (409,)
    GONE = (410,)
    LENGTH_REQUIRED = (411,)
    PRECONDITION_FAILED = (412,)
    PAYLOAD_TOO_LARGE = (413,)
    URI_TOO_LONG = (414,)
    UNSUPPORTED_MEDIA_TYPE = (415,)
    RANGE_NOT_SATISFIABLE = (416,)
    EXPECTATION_FAILED = (417,)
    IM_A_TEAPOT = (418,)
    MISDIRECTED_REQUEST = (421,)
    UNPROCESSABLE_ENTITY = (422,)
    LOCKED = (423,)
    FAILED_DEPENDENCY = (424,)
    TOO_EARLY = (425,)
    UPGRADE_REQUIRED = (426,)
    PRECONDITION_REQUIRED = (428,)
    TOO_MANY_REQUESTS = (429,)
    REQUEST_HEADER_FIELDS_TOO_LARGE = (431,)
    UNAVAILABLE_FOR_LEGAL_REASONS = (451,)
    INTERNAL_SERVER_ERROR = (500,)
    NOT_IMPLEMENTED = (501,)
    BAD_GATEWAY = (502,)
    SERVICE_UNAVAILABLE = (503,)
    GATEWAY_TIMEOUT = (504,)
    HTTP_VERSION_NOT_SUPPORTED = (505,)
    VARIANT_ALSO_NEGOTIATES = (506,)
    INSUFFICIENT_STORAGE = (507,)
    LOOP_DETECTED = (508,)
    NOT_EXTENDED = (510,)
    NETWORK_AUTHENTICATION_REQUIRED = (511,)

class Url:
    protocol: Optional[Protocol]
    authority: Optional[str]
    username: Optional[str]
    password: Optional[str]
    hostname: Optional[str]
    host: str
    port: Optional[int]
    path: Optional[str]
    search_params: UrlSearchParams
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

class UrlSearchParams:
    def append(self, key: str, value: str) -> None: ...
    def clear(self) -> None: ...
    def delete(self, key: str) -> None: ...
    def __repr__(self) -> str: ...
    def __getitem__(self, key: str) -> Optional[str]: ...
    def __setitem__(self, key: str, value: str) -> None: ...
    def __str__(self) -> str: ...
    def __len__(self) -> int: ...

class Response:
    status: int
    version: Version
    headers: Headers
    trailers: Headers
    body: Body
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

class Flow:
    request: Request
    response: Response
    def __str__(self) -> str: ...
    def __repr__(self) -> str: ...

@runtime_checkable
class Extension(ProtocolType):
    def start(self) -> None: ...
    def stop(self) -> None: ...
    def request(self, flow: Flow) -> None: ...
    def response(self, flow: Flow) -> None: ...

def notify(level: int, msg: str) -> None: ...

# Roxy discovers this global list at load
Extensions: List[Extension]
