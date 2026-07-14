from __future__ import annotations

import json
from json import JSONDecodeError
from typing import Any, Mapping, Optional
from urllib import parse, request
from urllib.error import HTTPError, URLError

AccountType = str
DEFAULT_MAX_RESPONSE_BYTES = 10 << 20


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


class PatchClientV3:
    def __init__(
        self,
        base_url: str = "https://patch-api.conalog.com",
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        timeout: float = 30.0,
        default_headers: Optional[Mapping[str, str]] = None,
        max_response_bytes: int = DEFAULT_MAX_RESPONSE_BYTES,
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
        if follow_redirects:
            self._opener = request.build_opener(_SafeRedirectHandler())
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

    def list_oauth_methods(
        self,
        provider: Optional[str] = None,
        redirect_url: Optional[str] = None,
    ) -> Any:
        return self._request(
            "GET",
            "/api/v3/account/auth-methods",
            query={"provider": provider, "redirect_url": redirect_url},
        )

    def start_oauth_login(
        self,
        provider: str,
        redirect_url: Optional[str] = None,
    ) -> Any:
        try:
            return self._request(
                "GET",
                "/api/v3/account/login-with-oauth2",
                query={"provider": provider, "redirect_url": redirect_url},
            )
        except PatchClientError as err:
            if err.status_code == 302 and isinstance(err.payload, dict):
                location = err.payload.get("Location")
                if isinstance(location, str):
                    return location
            raise

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

    def list_combiner_model_info(
        self,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            "/api/v3/model-info/combiners",
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def list_inverter_model_info(
        self,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            "/api/v3/model-info/inverters",
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def list_module_model_info(
        self,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            "/api/v3/model-info/modules",
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def create_org_member(
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

    def create_organization_member(
        self,
        organization_id: str,
        payload: Mapping[str, Any],
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self.create_org_member(
            organization_id,
            payload,
            access_token=access_token,
            account_type=account_type,
            headers=headers,
        )

    def assign_plant_permission(
        self,
        organization_id: str,
        plant_id: str,
        payload: Mapping[str, Any],
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "POST",
            (
                f"/api/v3/organizations/{_encode_path(organization_id)}/plants/"
                f"{_encode_path(plant_id)}/permissions/grant"
            ),
            json_body=payload,
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def remove_plant_permission(
        self,
        organization_id: str,
        plant_id: str,
        payload: Mapping[str, Any],
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "POST",
            (
                f"/api/v3/organizations/{_encode_path(organization_id)}/plants/"
                f"{_encode_path(plant_id)}/permissions/revoke"
            ),
            json_body=payload,
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_plant_list(
        self,
        page: Optional[int] = None,
        size: Optional[int] = None,
        full: Optional[bool] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            "/api/v3/plants",
            query={"page": page, "size": size, "full": full},
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

    def list_plant_blueprints(
        self,
        plant_id: str,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/blueprints",
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def record_plant_blueprint(
        self,
        plant_id: str,
        payload: Mapping[str, Any],
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "POST",
            f"/api/v3/plants/{_encode_path(plant_id)}/blueprints/record",
            json_body=payload,
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_plant_blueprint_data(
        self,
        plant_id: str,
        blueprint_id: str,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            (
                f"/api/v3/plants/{_encode_path(plant_id)}/blueprints/"
                f"{_encode_path(blueprint_id)}"
            ),
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def list_plant_comments(
        self,
        plant_id: str,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/comments",
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def start_plant_comment_thread(
        self,
        plant_id: str,
        payload: Mapping[str, Any],
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "POST",
            f"/api/v3/plants/{_encode_path(plant_id)}/comments/start_thread",
            json_body=payload,
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def edit_plant_comment(
        self,
        plant_id: str,
        comment_id: str,
        payload: Optional[Mapping[str, Any]] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "POST",
            (
                f"/api/v3/plants/{_encode_path(plant_id)}/comments/"
                f"{_encode_path(comment_id)}/edit"
            ),
            json_body=payload,
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def reply_plant_comment(
        self,
        plant_id: str,
        comment_id: str,
        payload: Mapping[str, Any],
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "POST",
            (
                f"/api/v3/plants/{_encode_path(plant_id)}/comments/"
                f"{_encode_path(comment_id)}/reply"
            ),
            json_body=payload,
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def change_plant_comment_state(
        self,
        plant_id: str,
        comment_id: str,
        payload: Mapping[str, Any],
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "POST",
            (
                f"/api/v3/plants/{_encode_path(plant_id)}/comments/"
                f"{_encode_path(comment_id)}/state"
            ),
            json_body=payload,
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def list_plant_filters(
        self,
        plant_id: str,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/filters",
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def create_plant_filter(
        self,
        plant_id: str,
        payload: Mapping[str, Any],
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "POST",
            f"/api/v3/plants/{_encode_path(plant_id)}/filters/create",
            json_body=payload,
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def delete_plant_filter(
        self,
        plant_id: str,
        filter_id: str,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "DELETE",
            f"/api/v3/plants/{_encode_path(plant_id)}/filters/{_encode_path(filter_id)}",
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def rename_plant_filter(
        self,
        plant_id: str,
        filter_id: str,
        payload: Mapping[str, Any],
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "POST",
            (
                f"/api/v3/plants/{_encode_path(plant_id)}/filters/"
                f"{_encode_path(filter_id)}/rename"
            ),
            json_body=payload,
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_plant_anomaly_timeline(
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
            f"/api/v3/plants/{_encode_path(plant_id)}/indicator/anomaly",
            query={"date": date},
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_plant_anomaly_logs(
        self,
        plant_id: str,
        date: str,
        map_id: Optional[str] = None,
        map_type: Optional[str] = None,
        type: Optional[str] = None,
        severity: Optional[str] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/indicator/anomaly/logs",
            query={
                "date": date,
                "map_id": map_id,
                "map_type": map_type,
                "type": type,
                "severity": severity,
            },
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def filter_plant_anomaly_logs(
        self,
        plant_id: str,
        date: str,
        map_id: Optional[str] = None,
        map_type: Optional[str] = None,
        type: Optional[str] = None,
        severity: Optional[str] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/indicator/anomaly/logs/filter",
            query={
                "date": date,
                "map_id": map_id,
                "map_type": map_type,
                "type": type,
                "severity": severity,
            },
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_plant_anomaly_snapshots(
        self,
        plant_id: str,
        date: str,
        map_id: Optional[str] = None,
        map_type: Optional[str] = None,
        type: Optional[str] = None,
        severity: Optional[str] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/indicator/anomaly/snapshots",
            query={
                "date": date,
                "map_id": map_id,
                "map_type": map_type,
                "type": type,
                "severity": severity,
            },
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_device_state(
        self,
        plant_id: str,
        date: str,
        fields: Optional[list[str]] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/indicator/device-state",
            query={"date": date, "fields": ",".join(fields) if fields else None},
            headers=self._merge_headers(headers, access_token, account_type),
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
        ids: Optional[list[str]] = None,
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
            query={
                "date": date,
                "before": before,
                "fields": ",".join(fields) if fields else None,
                "id": ids,
            },
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_plant_registry_timeline(
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
            f"/api/v3/plants/{_encode_path(plant_id)}/registry",
            query={"date": date},
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_plant_registry_logs(
        self,
        plant_id: str,
        date: str,
        asset_id: Optional[str] = None,
        map_id: Optional[str] = None,
        asset_type: Optional[str] = None,
        map_type: Optional[str] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/registry/logs",
            query={
                "date": date,
                "asset_id": asset_id,
                "map_id": map_id,
                "asset_type": asset_type,
                "map_type": map_type,
            },
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def filter_plant_registry_logs(
        self,
        plant_id: str,
        date: Optional[str] = None,
        asset_id: Optional[str] = None,
        map_id: Optional[str] = None,
        asset_type: Optional[str] = None,
        map_type: Optional[str] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/registry/logs/filter",
            query={
                "date": date,
                "asset_id": asset_id,
                "map_id": map_id,
                "asset_type": asset_type,
                "map_type": map_type,
            },
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def register_asset_to_plant(
        self,
        plant_id: str,
        payload: Mapping[str, Any],
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "POST",
            f"/api/v3/plants/{_encode_path(plant_id)}/registry/register",
            json_body=payload,
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_plant_registry_snapshots(
        self,
        plant_id: str,
        date: str,
        asset_id: Optional[str] = None,
        map_id: Optional[str] = None,
        asset_type: Optional[str] = None,
        map_type: Optional[str] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/registry/snapshots",
            query={
                "date": date,
                "asset_id": asset_id,
                "map_id": map_id,
                "asset_type": asset_type,
                "map_type": map_type,
            },
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_plant_registry_stat(
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
            f"/api/v3/plants/{_encode_path(plant_id)}/registry/stat",
            query={"date": date},
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def unregister_asset_from_plant(
        self,
        plant_id: str,
        payload: Mapping[str, Any],
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "POST",
            f"/api/v3/plants/{_encode_path(plant_id)}/registry/unregister",
            json_body=payload,
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_plant_weather_forecast(
        self,
        plant_id: str,
        days: Optional[int] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/weather/forecast",
            query={"days": days},
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def get_plant_weather_observed(
        self,
        plant_id: str,
        date: str,
        before: Optional[int] = None,
        *,
        access_token: Optional[str] = None,
        account_type: Optional[AccountType] = None,
        headers: Optional[Mapping[str, str]] = None,
    ) -> Any:
        return self._request(
            "GET",
            f"/api/v3/plants/{_encode_path(plant_id)}/weather/observed",
            query={"date": date, "before": before},
            headers=self._merge_headers(headers, access_token, account_type),
        )

    def _request(
        self,
        method: str,
        path: str,
        *,
        query: Optional[Mapping[str, Any]] = None,
        json_body: Optional[Any] = None,
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
        body: Optional[bytes] = None

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
            location = err.headers.get("Location") if err.headers else None
            if err.code == 302 and location:
                payload = {"Location": location}
                err.close()
            else:
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

class _NoRedirectHandler(request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):  # type: ignore[override]
        return None


class _SafeRedirectHandler(request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):  # type: ignore[override]
        old_url = parse.urlsplit(req.full_url)
        new_url = parse.urlsplit(newurl)
        if new_url.scheme not in {"http", "https"}:
            return None
        # Never follow HTTPS->HTTP downgrades.
        is_downgrade = old_url.scheme == "https" and new_url.scheme != "https"
        if is_downgrade:
            return None
        # Never follow cross-origin redirects.
        same_host = (
            old_url.hostname == new_url.hostname
            and _normalized_port(old_url) == _normalized_port(new_url)
        )
        if not same_host:
            return None
        # Do not follow redirects for auth-bearing requests.
        if _has_non_empty_header(req.headers, "Authorization"):
            return None
        # For redirects that preserve method/body (307/308), do not replay body-bearing requests.
        if code in {307, 308} and req.data is not None:
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
