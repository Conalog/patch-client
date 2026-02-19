from __future__ import annotations

import json
import uuid
from dataclasses import dataclass
from json import JSONDecodeError
from typing import Any, Mapping, Optional
from urllib import parse, request
from urllib.error import HTTPError, URLError

AccountType = str


class PatchClientError(Exception):
    def __init__(self, status_code: int, payload: Any):
        self.status_code = status_code
        self.payload = payload
        super().__init__(f"PATCH API request failed with status {status_code}")


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
    ):
        self.base_url = base_url.rstrip("/")
        self.access_token = access_token
        self.account_type = account_type
        self.timeout = timeout
        self.default_headers = dict(default_headers or {})

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
        content_type, body = _encode_multipart(fields or {}, files)
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
            query={"date": date, "before": before, "fields": fields},
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
        raw_body: Optional[bytes] = None,
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
            with request.urlopen(req, timeout=self.timeout) as resp:
                return _decode_response(resp.read(), resp.headers.get("Content-Type", ""))
        except HTTPError as err:
            payload = _decode_response(err.read(), err.headers.get("Content-Type", ""))
            raise PatchClientError(err.code, payload) from err
        except URLError as err:
            raise RuntimeError(f"Request failed: {err}") from err

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
            headers["Authorization"] = (
                resolved_token
                if _has_bearer_prefix(resolved_token)
                else f"Bearer {resolved_token}"
            )
        if resolved_account_type:
            headers["Account-Type"] = resolved_account_type

        return headers


def _encode_multipart(fields: Mapping[str, str], files: Mapping[str, FilePart]) -> tuple[str, bytes]:
    boundary = f"----patchclient{uuid.uuid4().hex}"
    body = bytearray()

    for name, value in fields.items():
        safe_name = _quote_header_value(_reject_crlf(name, "multipart field name"))
        body.extend(f"--{boundary}\r\n".encode("utf-8"))
        body.extend(
            f'Content-Disposition: form-data; name="{safe_name}"\r\n\r\n'.encode("utf-8")
        )
        body.extend(value.encode("utf-8"))
        body.extend(b"\r\n")

    for name, file_part in files.items():
        safe_name = _quote_header_value(_reject_crlf(name, "multipart file field name"))
        safe_filename = _quote_header_value(_reject_crlf(file_part.filename, "multipart filename"))
        safe_content_type = _reject_crlf(file_part.content_type, "multipart content type")
        body.extend(f"--{boundary}\r\n".encode("utf-8"))
        body.extend(
            (
                f'Content-Disposition: form-data; name="{safe_name}"; '
                f'filename="{safe_filename}"\r\n'
            ).encode("utf-8")
        )
        body.extend(f"Content-Type: {safe_content_type}\r\n\r\n".encode("utf-8"))
        body.extend(file_part.content)
        body.extend(b"\r\n")

    body.extend(f"--{boundary}--\r\n".encode("utf-8"))
    return f"multipart/form-data; boundary={boundary}", bytes(body)


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


def _has_bearer_prefix(value: str) -> bool:
    return len(value) >= 7 and value[:7].lower() == "bearer "


def _quote_header_value(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')


def _reject_crlf(value: str, field_name: str) -> str:
    if "\r" in value or "\n" in value:
        raise ValueError(f"{field_name} must not contain CR or LF characters")
    return value
