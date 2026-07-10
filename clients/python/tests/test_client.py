import ast
from pathlib import Path
import unittest
from io import BytesIO
from unittest.mock import patch
from urllib.error import HTTPError, URLError

from patch_client.client import (
    PatchClientError,
    PatchClientV3,
    _SafeRedirectHandler,
    _decode_response,
)


class ClientSafetyTests(unittest.TestCase):
    def test_rejects_insecure_http_base_url_without_opt_in(self) -> None:
        with self.assertRaises(ValueError):
            PatchClientV3(base_url="http://example.com")

    def test_allows_insecure_http_base_url_with_opt_in(self) -> None:
        client = PatchClientV3(base_url="http://example.com", allow_insecure_http=True)
        self.assertEqual(client.base_url, "http://example.com")

    def test_rejects_base_url_with_query_or_fragment(self) -> None:
        with self.assertRaises(ValueError):
            PatchClientV3(base_url="https://example.com?x=1")
        with self.assertRaises(ValueError):
            PatchClientV3(base_url="https://example.com#frag")

    def test_rejects_base_url_with_invalid_port(self) -> None:
        with self.assertRaises(ValueError):
            PatchClientV3(base_url="https://example.com:badport")

    def test_rejects_base_url_with_credentials(self) -> None:
        with self.assertRaises(ValueError):
            PatchClientV3(base_url="https://user:pass@example.com")

    def test_decode_response_handles_invalid_utf8_json_payload(self) -> None:
        result = _decode_response(b"\xff", "application/json")
        self.assertIsInstance(result, str)

    def test_decode_response_handles_case_insensitive_json_content_type(self) -> None:
        result = _decode_response(b'{"ok": true}', "Application/JSON; charset=utf-8")
        self.assertEqual(result, {"ok": True})

    def test_get_metrics_by_date_serializes_fields_as_csv(self) -> None:
        class StubClient(PatchClientV3):
            def __init__(self) -> None:
                super().__init__(base_url="https://example.com")
                self.captured_query = None

            def _request(self, method, path, **kwargs):  # type: ignore[override]
                self.captured_query = kwargs.get("query")
                return None

        client = StubClient()
        client.get_metrics_by_date(
            "plant-id", "device", "plant", "1d", "2024-01-24", fields=["i_out", "p"]
        )
        self.assertEqual(client.captured_query["fields"], "i_out,p")

    def test_get_metrics_by_date_forwards_id_filters(self) -> None:
        class StubClient(PatchClientV3):
            def __init__(self) -> None:
                super().__init__(base_url="https://example.com")
                self.captured_query = None

            def _request(self, method, path, **kwargs):  # type: ignore[override]
                self.captured_query = kwargs.get("query")
                return None

        client = StubClient()
        client.get_metrics_by_date(
            "plant-id", "device", "panel", "5m", "2024-01-24", ids=["p1", "p2"]
        )
        self.assertEqual(client.captured_query["id"], ["p1", "p2"])

    def test_new_spec_methods_route_to_expected_paths(self) -> None:
        class StubClient(PatchClientV3):
            def __init__(self) -> None:
                super().__init__(base_url="https://example.com")
                self.calls = []

            def _request(self, method, path, **kwargs):  # type: ignore[override]
                self.calls.append((method, path, kwargs))
                return None

        payload = {"value": "x"}
        cases = [
            (
                lambda client: client.list_oauth_methods("google", "https://app/callback"),
                "GET",
                "/api/v3/account/auth-methods",
                {"provider": "google", "redirect_url": "https://app/callback"},
                None,
            ),
            (
                lambda client: client.list_combiner_model_info(),
                "GET",
                "/api/v3/model-info/combiners",
                None,
                None,
            ),
            (
                lambda client: client.assign_plant_permission("org/1", "plant 1", payload),
                "POST",
                "/api/v3/organizations/org%2F1/plants/plant%201/permissions/grant",
                None,
                payload,
            ),
            (
                lambda client: client.remove_plant_permission("org", "plant", payload),
                "POST",
                "/api/v3/organizations/org/plants/plant/permissions/revoke",
                None,
                payload,
            ),
            (
                lambda client: client.get_plant_list(full=True),
                "GET",
                "/api/v3/plants",
                {"page": None, "size": None, "full": True},
                None,
            ),
            (
                lambda client: client.record_plant_blueprint("plant", payload),
                "POST",
                "/api/v3/plants/plant/blueprints/record",
                None,
                payload,
            ),
            (
                lambda client: client.start_plant_comment_thread("plant", payload),
                "POST",
                "/api/v3/plants/plant/comments/start_thread",
                None,
                payload,
            ),
            (
                lambda client: client.rename_plant_filter("plant", "filter", payload),
                "POST",
                "/api/v3/plants/plant/filters/filter/rename",
                None,
                payload,
            ),
            (
                lambda client: client.get_plant_anomaly_logs(
                    "plant", "2024-01-24", type="hotspot", severity="high"
                ),
                "GET",
                "/api/v3/plants/plant/indicator/anomaly/logs",
                {
                    "date": "2024-01-24",
                    "map_id": None,
                    "map_type": None,
                    "type": "hotspot",
                    "severity": "high",
                },
                None,
            ),
            (
                lambda client: client.get_device_state(
                    "plant", "2024-01-24", fields=["is_relay", "is_rapid_shutdown"]
                ),
                "GET",
                "/api/v3/plants/plant/indicator/device-state",
                {"date": "2024-01-24", "fields": "is_relay,is_rapid_shutdown"},
                None,
            ),
            (
                lambda client: client.register_asset_to_plant("plant", payload),
                "POST",
                "/api/v3/plants/plant/registry/register",
                None,
                payload,
            ),
            (
                lambda client: client.get_plant_weather_observed("plant", "2024-01-24", 3),
                "GET",
                "/api/v3/plants/plant/weather/observed",
                {"date": "2024-01-24", "before": 3},
                None,
            ),
        ]

        for call, method, path, query, json_body in cases:
            with self.subTest(path=path):
                client = StubClient()
                call(client)
                self.assertEqual(len(client.calls), 1)
                self.assertEqual(client.calls[0][:2], (method, path))
                kwargs = client.calls[0][2]
                self.assertEqual(kwargs.get("query"), query)
                self.assertEqual(kwargs.get("json_body"), json_body)

    def test_start_oauth_login_returns_redirect_location(self) -> None:
        client = PatchClientV3(base_url="https://example.com")
        http_error = HTTPError(
            "https://example.com/api/v3/account/login-with-oauth2?provider=google",
            302,
            "found",
            {"Location": "https://accounts.example/auth"},
            BytesIO(b""),
        )
        with patch.object(client._opener, "open", side_effect=http_error):
            redirect_url = client.start_oauth_login("google")
        self.assertEqual(redirect_url, "https://accounts.example/auth")

    def test_merge_headers_preserves_lowercase_bearer_prefix(self) -> None:
        client = PatchClientV3(base_url="https://example.com")
        merged = client._merge_headers(None, "bearer abc.def", None)
        self.assertEqual(merged["Authorization"], "bearer abc.def")

    def test_merge_headers_ignores_whitespace_only_token(self) -> None:
        client = PatchClientV3(base_url="https://example.com")
        merged = client._merge_headers(None, "   ", None)
        self.assertNotIn("Authorization", merged)

    def test_request_raises_patch_client_error_on_url_error(self) -> None:
        client = PatchClientV3(base_url="https://example.com")
        with patch.object(client._opener, "open", side_effect=URLError("boom")):
            with self.assertRaises(PatchClientError) as ctx:
                client.get_account_info()
        self.assertEqual(ctx.exception.status_code, 0)

    def test_http_error_without_headers_is_handled(self) -> None:
        client = PatchClientV3(base_url="https://example.com")
        http_error = HTTPError(
            "https://example.com/api/v3/account/",
            400,
            "bad request",
            None,
            BytesIO(b'{"error":"bad"}'),
        )
        with patch.object(client._opener, "open", side_effect=http_error):
            with self.assertRaises(PatchClientError) as ctx:
                client.get_account_info()
        self.assertEqual(ctx.exception.status_code, 400)

    def test_http_error_with_unreadable_body_preserves_http_status(self) -> None:
        class UnreadableHTTPError(HTTPError):
            def read(self, *_args, **_kwargs):  # type: ignore[override]
                raise OSError("unreadable body")

        client = PatchClientV3(base_url="https://example.com")
        http_error = UnreadableHTTPError(
            "https://example.com/api/v3/account/",
            502,
            "bad gateway",
            {},
            None,
        )
        with patch.object(client._opener, "open", side_effect=http_error):
            with self.assertRaises(PatchClientError) as ctx:
                client.get_account_info()
        self.assertEqual(ctx.exception.status_code, 502)
        self.assertIn("failed to read error response", str(ctx.exception.payload))

    def test_oversized_success_response_preserves_size_error_detail(self) -> None:
        class ResponseStub:
            headers = {}

            def read(self, _limit=None):
                return b"x" * 5

            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, tb):
                return False

        client = PatchClientV3(base_url="https://example.com", max_response_bytes=4)
        with patch.object(client._opener, "open", return_value=ResponseStub()):
            with self.assertRaises(PatchClientError) as ctx:
                client.get_account_info()
        self.assertEqual(ctx.exception.status_code, 0)
        self.assertIn("response exceeded 4 bytes", str(ctx.exception.payload))

    def test_client_module_is_python39_syntax_compatible(self) -> None:
        source_path = Path(__file__).resolve().parents[1] / "patch_client" / "client.py"
        source = source_path.read_text(encoding="utf-8")
        ast.parse(source, filename=str(source_path), feature_version=(3, 9))

    def test_safe_redirect_handler_blocks_cross_origin_redirect(self) -> None:
        from urllib import request

        handler = _SafeRedirectHandler()
        req = request.Request(
            "https://example.com/api/v3/account/",
            headers={"Authorization": "Bearer token", "Account-Type": "manager"},
        )
        redirected = handler.redirect_request(
            req=req,
            fp=None,
            code=302,
            msg="Found",
            headers={"Location": "https://another.example.com/path"},
            newurl="https://another.example.com/path",
        )
        self.assertIsNone(redirected)

    def test_safe_redirect_handler_blocks_https_to_http_downgrade_without_auth_or_body(
        self,
    ) -> None:
        from urllib import request

        handler = _SafeRedirectHandler()
        req = request.Request("https://example.com/api", headers={"Authorization": "Bearer token"})
        redirected = handler.redirect_request(
            req=req,
            fp=None,
            code=302,
            msg="Found",
            headers={"Location": "http://example.com/insecure"},
            newurl="http://example.com/insecure",
        )
        self.assertIsNone(redirected)

    def test_safe_redirect_handler_blocks_https_to_http_downgrade(self) -> None:
        from urllib import request

        handler = _SafeRedirectHandler()
        req = request.Request("https://example.com/api")
        redirected = handler.redirect_request(
            req=req,
            fp=None,
            code=302,
            msg="Found",
            headers={"Location": "http://example.com/insecure"},
            newurl="http://example.com/insecure",
        )
        self.assertIsNone(redirected)

    def test_safe_redirect_handler_blocks_non_http_scheme(self) -> None:
        from urllib import request

        handler = _SafeRedirectHandler()
        req = request.Request("https://example.com/api")
        redirected = handler.redirect_request(
            req=req,
            fp=None,
            code=302,
            msg="Found",
            headers={"Location": "ftp://example.com/file"},
            newurl="ftp://example.com/file",
        )
        self.assertIsNone(redirected)

    def test_safe_redirect_handler_blocks_auth_bearing_redirect_replay(self) -> None:
        from urllib import request

        handler = _SafeRedirectHandler()
        req = request.Request(
            "https://example.com/api/v3/account/",
            headers={"Authorization": "Bearer token"},
        )
        redirected = handler.redirect_request(
            req=req,
            fp=None,
            code=307,
            msg="Temporary Redirect",
            headers={"Location": "https://example.com/next"},
            newurl="https://example.com/next",
        )
        self.assertIsNone(redirected)

    def test_safe_redirect_handler_blocks_body_bearing_redirect_replay(self) -> None:
        from urllib import request

        handler = _SafeRedirectHandler()
        req = request.Request(
            "https://example.com/api/v3/account/auth-with-password",
            data=b'{"password":"pw"}',
            headers={"Content-Type": "application/json"},
        )
        redirected = handler.redirect_request(
            req=req,
            fp=None,
            code=307,
            msg="Temporary Redirect",
            headers={"Location": "https://example.com/next"},
            newurl="https://example.com/next",
        )
        self.assertIsNone(redirected)

    def test_safe_redirect_handler_allows_post_redirect_get(self) -> None:
        from urllib import request

        handler = _SafeRedirectHandler()
        req = request.Request(
            "https://example.com/api/v3/account/auth-with-password",
            data=b'{"password":"pw"}',
            headers={"Content-Type": "application/json"},
        )
        redirected = handler.redirect_request(
            req=req,
            fp=None,
            code=302,
            msg="Found",
            headers={"Location": "https://example.com/next"},
            newurl="https://example.com/next",
        )
        self.assertIsNotNone(redirected)
        assert redirected is not None
        self.assertEqual(redirected.get_method(), "GET")
        self.assertIsNone(redirected.data)

    def test_request_raises_patch_client_error_on_3xx_status(self) -> None:
        class ResponseStub:
            status = 302
            headers = {"Content-Type": "application/json", "Location": "https://example.com/other"}

            def read(self, _limit=None):
                return b'{"detail":"redirected"}'

            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, tb):
                return False

        client = PatchClientV3(base_url="https://example.com")
        with patch.object(client._opener, "open", return_value=ResponseStub()):
            with self.assertRaises(PatchClientError) as ctx:
                client.get_account_info()
        self.assertEqual(ctx.exception.status_code, 302)
        self.assertEqual(ctx.exception.payload, {"detail": "redirected"})


if __name__ == "__main__":
    unittest.main()
