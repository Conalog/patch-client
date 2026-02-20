from __future__ import annotations

import json
import uuid
from dataclasses import dataclass
from json import JSONDecodeError
from typing import Any, Mapping, Optional, Union
from urllib import parse, request
from urllib.error import HTTPError, URLError

AccountType = str
DEFAULT_MAX_RESPONSE_BYTES = 10 << 20
DEFAULT_MAX_MULTIPART_BYTES = 20 << 20


class PatchClientError(Exception):
    def __init__(
        self,
        status_code: int,
        payload: Any,
        *,
        method: Optional[str] = None,
        url: Optional[str] = None,
    ):
        self.status_code = status_code
        self.payload = payload
        self.method = method
        self.url = url
        context = ""
        if method and url:
            context = f" ({method} {url})"
        super().__init__(f"PATCH API request failed with status {status_code}{context}")


@dataclass(frozen=True)
class FilePart:
    filename: str
    content: bytes
    content_type: str = "application/octet-stream"


class PatchClientV3:
    def __init__(
        self,
        base_url: str = "https://patch-api.conalog.com",
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        timeout: float = 30.0,
        default_headers: Optional[Mapping[str, str]] = None,
        max_response_bytes: int = DEFAULT_MAX_RESPONSE_BYTES,
        max_multipart_bytes: int = DEFAULT_MAX_MULTIPART_BYTES,
        allow_insecure_http: bool = False,
        follow_redirects: bool = True,
    ):
        parsed = parse.urlsplit(base_url)
        if parsed.scheme not in {"http", "https"}:
            raise ValueError("base_url must use http:// or https://")
        if not parsed.hostname:
            raise ValueError("base_url must include a hostname")
        if parsed.username or parsed.password:
            raise ValueError("base_url must not include credentials")
        if parsed.query or parsed.fragment:
            raise ValueError("base_url must not include query or fragment")
        try:
            _ = parsed.port
        except ValueError as err:
            raise ValueError("base_url must include a valid port") from err
        if parsed.scheme != "https" and not allow_insecure_http:
            raise ValueError("insecure http base_url requires allow_insecure_http=True")

        self.base_url = base_url.rstrip("/")
        self.access_token = access_token
        self.account_type = account_type
        self.timeout = timeout
        self.allow_insecure_http = allow_insecure_http
        self.default_headers = dict(default_headers or {})
        self.max_response_bytes = (
            max_response_bytes if max_response_bytes > 0 else DEFAULT_MAX_RESPONSE_BYTES
        )
        self.max_multipart_bytes = (
            max_multipart_bytes if max_multipart_bytes > 0 else DEFAULT_MAX_MULTIPART_BYTES
        )
        if follow_redirects:
            self._opener = request.build_opener(
                _SafeRedirectHandler(allow_insecure_http=allow_insecure_http)
            )
        else:
            self._opener = request.build_opener(_NoRedirectHandler())

    def set_access_token(self, token: Optional[str]) -> None:
        self.access_token = token

    def set_account_type(self, account_type: Optional[AccountType]) -> None:
        self.account_type = account_type

    def authenticate_user(self, payload: Mapping[str, Any]) -> Any:
        return self._request(
            "POST",
            "/api/v3/account/auth-with-password",
            json_body=payload,
        )

    def refresh_user_token(
        self,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "POST",
            "/api/v3/account/refresh-token",
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_account_info(
        self,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            "/api/v3/account/",
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def create_organization_member(
        self,
        organization_id: str,
        payload: Mapping[str, Any],
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "POST",
            f"/api/v3/organizations/{_encode_path(organization_id)}/members",
            json_body=payload,
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def assign_plant_permission(
        self,
        organization_id: str,
        payload: Mapping[str, Any],
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "POST",
            f"/api/v3/organizations/{_encode_path(organization_id)}/permissions",
            json_body=payload,
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_plant_list(
        self,
        page: Optional[int] = None,
        size: Optional[int] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            "/api/v3/plants",
            query={"page": page, "size": size},
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def create_plant(
        self,
        payload: Mapping[str, Any],
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "POST",
            "/api/v3/plants",
            json_body=payload,
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_plant_details(
        self,
        plant_id: str,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}",
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_plant_blueprint(
        self,
        plant_id: str,
        date: str,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/blueprint",
            query={"date": date},
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def upload_plant_files(
        self,
        plant_id: str,
        files: Mapping[str, FilePart],
        fields: Optional[Mapping[str, str]] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        if not files:
            raise ValueError("files must not be empty")
        content_type, body = _encode_multipart(
            fields or {}, files, max_total_bytes=self.max_multipart_bytes
        )
        merged_headers = self._merge_headers(headers, access_token, account_type)
        merged_headers["Content-Type"] = content_type
        return self._request(
            "POST",
            f"/api/v3/plants/{_encode_path(plant_id)}/files",
            raw_body=body,
            headers=merged_headers,
        )

    def get_asset_health_level(
        self,
        plant_id: str,
        unit: str,
        date: str,
        view: Optional[str] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/indicator/health-level/{_encode_path(unit)}",
            query={"date": date, "view": view},
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_panel_seqnum(
        self,
        plant_id: str,
        date: str,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/indicator/seqnum",
            query={"date": date},
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def list_inverter_logs(
        self,
        plant_id: str,
        page: Optional[int] = None,
        size: Optional[int] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/logs/inverter",
            query={"page": page, "size": size},
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def list_inverter_logs_by_id(
        self,
        plant_id: str,
        inverter_id: str,
        page: Optional[int] = None,
        size: Optional[int] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/logs/inverters/{_encode_path(inverter_id)}",
            query={"page": page, "size": size},
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_latest_device_metrics(
        self,
        plant_id: str,
        include_state: Optional[bool] = None,
        ago: Optional[int] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/metrics/device/latest",
            query={"includeState": include_state, "ago": ago},
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_latest_inverter_metrics(
        self,
        plant_id: str,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/metrics/inverter/latest",
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_metrics_by_date(
        self,
        plant_id: str,
        source: str,
        unit: str,
        interval: str,
        date: str,
        before: Optional[int] = None,
        fields: Optional[list[str]] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            (
                f"/api/v3/plants/{_encode_path(plant_id)}/metrics/"
                f"{_encode_path(source)}/{_encode_path(unit)}-{_encode_path(interval)}"
            ),
            query={"date": date, "before": before, "fields": ",".join(fields) if fields else None},
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_asset_registration_on_plant(
        self,
        plant_id: str,
        record_type: str,
        date: str,
        asset_id: Optional[str] = None,
        map_id: Optional[str] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/registry/{_encode_path(record_type)}",
            query={"date": date, "asset_id": asset_id, "map_id": map_id},
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def _request(
        self,
        method: str,
        path: str,
        *,
        query: Optional[Mapping[str, Any]] = None,
        json_body: Optional[Any] = None,
        raw_body: Optional[Union[bytes, bytearray]] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        url = f"{self.base_url}{path}"
        if query:
            query_items: list[tuple[str, str]] = []
            for key, value in query.items():
                if value is None:
                    continue
                if isinstance(value, (list, tuple)):
                    for item in value:
                        if item is not None:
                            query_items.append((key, _serialize_query_value(item)))
                else:
                    query_items.append((key, _serialize_query_value(value)))
            if query_items:
                url = f"{url}?{parse.urlencode(query_items, doseq=True)}"

        merged_headers = {"Accept": "application/json", **self.default_headers, **(headers or {})}
        body: Optional[bytes] = raw_body

        if json_body is not None:
            merged_headers["Content-Type"] = "application/json"
            body = json.dumps(json_body).encode("utf-8")

        req = request.Request(url=url, method=method, headers=merged_headers, data=body)

        try:
            with self._opener.open(req, timeout=self.timeout) as resp:
                try:
                    payload = self._read_limited(resp)
                except OverflowError as err:
                    raise PatchClientError(
                        0,
                        {"error": str(err)},
                        method=method,
                        url=url,
                    ) from err
                content_type = resp.headers.get("Content-Type", "")
                decoded = _decode_response(payload, content_type)
                status_code = _response_status_code(resp)
                if status_code is not None and (status_code < 200 or status_code >= 300):
                    raise PatchClientError(status_code, decoded, method=method, url=url)
                return decoded
        except HTTPError as err:
            payload: Any
            try:
                payload_bytes = self._read_limited(err)
                content_type = err.headers.get("Content-Type", "") if err.headers else ""
                payload = _decode_response(payload_bytes, content_type)
            except OverflowError as size_err:
                payload = {"error": str(size_err)}
            except Exception as read_err:
                payload = {"error": f"failed to read error response: {read_err}"}
            finally:
                err.close()
            raise PatchClientError(err.code, payload, method=method, url=url) from err
        except URLError as err:
            raise PatchClientError(
                0,
                {"error": str(err.reason) if getattr(err, "reason", None) else str(err)},
                method=method,
                url=url,
            ) from err
        except PatchClientError:
            raise
        except Exception as err:
            raise PatchClientError(
                0,
                {"error": str(err)},
                method=method,
                url=url,
            ) from err

    def _read_limited(self, response: Any) -> bytes:
        payload = response.read(self.max_response_bytes + 1)
        if len(payload) > self.max_response_bytes:
            raise OverflowError(f"response exceeded {self.max_response_bytes} bytes")
        return payload

    def _merge_headers(
        self,
        extra: Optional[Mapping[str, str]],
        access_token: Optional[str],
        account_type: Optional[AccountType],
    ) -> dict[str, str]:
        headers: dict[str, str] = dict(extra or {})

        resolved_token = access_token if access_token is not None else self.access_token
        resolved_account_type = account_type if account_type is not None else self.account_type

        if resolved_token:
            normalized_token = resolved_token.strip()
            if normalized_token:
                headers["Authorization"] = (
                    normalized_token
                    if normalized_token.lower().startswith("bearer ")
                    else f"Bearer {normalized_token}"
                )
        if resolved_account_type:
            headers["Account-Type"] = resolved_account_type

        return headers


def _encode_multipart(
    fields: Mapping[str, str],
    files: Mapping[str, FilePart],
    *,
    max_total_bytes: int = DEFAULT_MAX_MULTIPART_BYTES,
) -> tuple[str, bytearray]:
    boundary = f"----patchclient{uuid.uuid4().hex}"
    body = bytearray()

    def append_checked(chunk: bytes) -> None:
        if len(body) + len(chunk) > max_total_bytes:
            raise ValueError(f"multipart payload exceeds {max_total_bytes} bytes")
        body.extend(chunk)

    for name, value in fields.items():
        safe_name = _quote_header_value(_reject_crlf(name, "multipart field name"))
        append_checked(f"--{boundary}\r\n".encode("utf-8"))
        append_checked(
            f'Content-Disposition: form-data; name="{safe_name}"\r\n\r\n'.encode("utf-8")
        )
        append_checked(value.encode("utf-8"))
        append_checked(b"\r\n")

    for name, file_part in files.items():
        safe_name = _quote_header_value(_reject_crlf(name, "multipart file field name"))
        safe_filename = _quote_header_value(_reject_crlf(file_part.filename, "multipart filename"))
        safe_content_type = _reject_crlf(file_part.content_type, "multipart content type")
        append_checked(f"--{boundary}\r\n".encode("utf-8"))
        append_checked(
            (
                f'Content-Disposition: form-data; name="{safe_name}"; '
                f'filename="{safe_filename}"\r\n'
            ).encode("utf-8")
        )
        append_checked(f"Content-Type: {safe_content_type}\r\n\r\n".encode("utf-8"))
        append_checked(file_part.content)
        append_checked(b"\r\n")

    append_checked(f"--{boundary}--\r\n".encode("utf-8"))
    return f"multipart/form-data; boundary={boundary}", body


def _decode_response(payload: bytes, content_type: str) -> Any:
    if not payload:
        return None
    normalized_content_type = content_type.lower()
    if "json" in normalized_content_type:
        text = payload.decode("utf-8", errors="replace")
        try:
            return json.loads(text)
        except JSONDecodeError:
            return text
    if (
        normalized_content_type.startswith("text/")
        or "xml" in normalized_content_type
        or "html" in normalized_content_type
    ):
        return payload.decode("utf-8", errors="replace")
    return payload


def _encode_path(value: str) -> str:
    return parse.quote(value, safe="")


def _serialize_query_value(value: Any) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    return str(value)


def _quote_header_value(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')


def _reject_crlf(value: str, field_name: str) -> str:
    if "\r" in value or "\n" in value:
        raise ValueError(f"{field_name} must not contain CR or LF characters")
    return value


class _NoRedirectHandler(request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):  # type: ignore[override]
        return None


class _SafeRedirectHandler(request.HTTPRedirectHandler):
    def __init__(self, *, allow_insecure_http: bool = False):
        super().__init__()
        self._allow_insecure_http = allow_insecure_http

    def redirect_request(self, req, fp, code, msg, headers, newurl):  # type: ignore[override]
        old_url = parse.urlsplit(req.full_url)
        new_url = parse.urlsplit(newurl)
        if new_url.scheme not in {"http", "https"}:
            return None
        # Do not replay auth-bearing or body-bearing requests through redirects.
        if _has_non_empty_header(req.headers, "Authorization") or req.data is not None:
            return None
        # Never forward auth headers across origin changes or HTTPS->HTTP downgrade.
        same_host = (
            old_url.hostname == new_url.hostname
            and _normalized_port(old_url) == _normalized_port(new_url)
        )
        is_downgrade = old_url.scheme == "https" and new_url.scheme != "https"
        if not same_host:
            return None
        # Never follow HTTPS->HTTP downgrades, even in insecure base-url mode.
        if is_downgrade:
            return None
        redirected = super().redirect_request(req, fp, code, msg, headers, newurl)
        if redirected is None:
            return None
        return redirected


def _has_non_empty_header(headers: Mapping[str, str], name: str) -> bool:
    lowered = name.lower()
    for key, value in headers.items():
        if key.lower() == lowered and bool(str(value).strip()):
            return True
    return False


def _normalized_port(parts: parse.SplitResult) -> Optional[int]:
    if parts.port is not None:
        return parts.port
    if parts.scheme == "http":
        return 80
    if parts.scheme == "https":
        return 443
    return None


def _response_status_code(response: Any) -> Optional[int]:
    status = getattr(response, "status", None)
    if isinstance(status, int):
        return status
    getcode = getattr(response, "getcode", None)
    if callable(getcode):
        value = getcode()
        if isinstance(value, int):
            return value
    return None
