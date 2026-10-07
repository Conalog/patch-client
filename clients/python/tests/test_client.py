import ast
import json
from email.parser import BytesParser
from email import policy
from pathlib import Path
from http.server import BaseHTTPRequestHandler, HTTPServer
from threading import Thread
import unittest
from io import BytesIO
from unittest.mock import patch
from urllib.error import HTTPError, URLError

from patch_client.client import (
    PatchClientError,
    PatchClientV3,
    _SafeRedirectHandler,
    _decode_response,
    _encode_multipart,
    _encode_path,
    _validate_headers,
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
                "/api/v3/orgs/org%2F1/plants/plant%201/permissions/grant",
                None,
                payload,
            ),
            (
                lambda client: client.remove_plant_permission("org", "plant", payload),
                "POST",
                "/api/v3/orgs/org/plants/plant/permissions/revoke",
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
                lambda client: client.list_plant_blueprints("plant"),
                "GET",
                "/api/v3/plants/plant/blueprints",
                None,
                None,
            ),
            (
                lambda client: client.start_plant_comment_thread("plant", payload),
                "POST",
                "/api/v3/plants/plant/comments/start_thread",
                None,
                payload,
            ),
            (
                lambda client: client.edit_plant_comment("plant", "comment"),
                "POST",
                "/api/v3/plants/plant/comments/comment/edit",
                None,
                None,
            ),
            (
                lambda client: client.reply_plant_comment("plant", "comment", payload),
                "POST",
                "/api/v3/plants/plant/comments/comment/reply",
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
                lambda client: client.filter_plant_registry_logs(
                    "plant", "2024-01-24", asset_id="inv-1", map_type="inverter"
                ),
                "GET",
                "/api/v3/plants/plant/registry/logs/filter",
                {
                    "date": "2024-01-24",
                    "asset_id": "inv-1",
                    "map_id": None,
                    "asset_type": None,
                    "map_type": "inverter",
                },
                None,
            ),
            (
                lambda client: client.get_plant_weather_forecast("plant", days=7),
                "GET",
                "/api/v3/plants/plant/weather/forecast",
                {"days": 7},
                None,
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

    def test_all_openapi_operations_have_a_python_method(self) -> None:
        spec_path = Path(__file__).resolve().parents[3] / "openapi" / "openapi-v3.json"
        spec = __import__("json").loads(spec_path.read_text(encoding="utf-8"))
        methods = {"get", "put", "post", "delete", "patch", "head", "options", "trace"}
        operation_ids = {
            operation["operationId"]
            for path_item in spec["paths"].values()
            for method, operation in path_item.items()
            if method in methods
        }
        self.assertEqual(
            {operation_id for operation_id in operation_ids if not hasattr(PatchClientV3, operation_id)},
            set(),
        )

    def test_fieldwork_commands_require_valid_idempotency_key_before_request(self) -> None:
        client = PatchClientV3(base_url="https://example.com", access_token="token")
        with self.assertRaises(ValueError):
            client.fieldwork_work_create(payload={})
        with self.assertRaises(ValueError):
            client.fieldwork_work_create(payload={}, idempotency_key="bad key")

    def test_participant_session_requires_bearer_authentication(self) -> None:
        client = PatchClientV3(base_url="https://example.com")
        with self.assertRaises(ValueError):
            client.fieldwork_participant_session_create(payload={})

    def test_fieldwork_auth_guard_accepts_default_authorization(self) -> None:
        class StubClient(PatchClientV3):
            def _request(self, *_args, **_kwargs):  # type: ignore[override]
                return None

        client = StubClient(
            base_url="https://example.com",
            default_headers={"Authorization": "Bearer configured-token"},
        )
        self.assertIsNone(client.fieldwork_participant_session_create(payload={}))

    def test_fieldwork_command_uses_json_and_idempotency_header(self) -> None:
        class StubClient(PatchClientV3):
            def _request(self, method, path, **kwargs):  # type: ignore[override]
                self.call = method, path, kwargs
                return None

        client = StubClient(base_url="https://example.com", access_token="token")
        client.fieldwork_work_create(payload={"title": "x"}, idempotency_key="valid-key")
        method, path, kwargs = client.call
        self.assertEqual((method, path), ("POST", "/api/v3/fieldwork/works"))
        self.assertEqual(kwargs["json_body"], {"title": "x"})
        self.assertEqual(kwargs["headers"]["Idempotency-Key"], "valid-key")

    def test_idempotency_header_is_case_insensitive_and_canonical_on_wire(self) -> None:
        class StubClient(PatchClientV3):
            def _request(self, method, path, **kwargs):  # type: ignore[override]
                self.headers = kwargs["headers"]
                return None

        client = StubClient(base_url="https://example.com", access_token="token")
        client.fieldwork_work_create(
            payload={}, headers={"idempotency-key": "valid-key"}
        )
        self.assertEqual(client.headers, {"Authorization": "Bearer token", "Idempotency-Key": "valid-key"})

    def test_multipart_encoder_uses_file_and_form_fields(self) -> None:
        body, content_type = _encode_multipart(
            {"work_id": "work", "quoted": 'a"b'}, {"file": b"data"}
        )
        message = BytesParser(policy=policy.default).parsebytes(
            b"Content-Type: " + content_type.encode() + b"\r\n\r\n" + body
        )
        parts = list(message.iter_parts())
        self.assertEqual(parts[0].get_payload(decode=True), b"work")
        self.assertEqual(parts[1].get_payload(decode=True), b'a"b')
        self.assertEqual(parts[2].get_filename(), "file")
        self.assertEqual(parts[2].get_payload(decode=True), b"data")

    def test_multipart_encoder_bounds_file_read_and_wire_buffer(self) -> None:
        class TooLargeFile:
            def read(self, limit):
                self.limit = limit
                return b"x" * limit

        file = TooLargeFile()
        with self.assertRaises(ValueError):
            _encode_multipart({}, {"file": file}, max_bytes=4)
        self.assertEqual(file.limit, 5)

    def test_multipart_encoder_reads_short_chunks_until_eof(self) -> None:
        class ShortChunkFile:
            name = "chunked"

            def __init__(self):
                self.data = b"abcdef"

            def read(self, _limit):
                chunk, self.data = self.data[:2], self.data[2:]
                return chunk

        body, _ = _encode_multipart({}, {"file": ShortChunkFile()}, max_bytes=1000)
        self.assertIn(b"abcdef", body)

    def test_headers_and_multipart_filename_reject_newlines(self) -> None:
        with self.assertRaises(ValueError):
            _validate_headers({"X-Test": "safe\r\ninjected"})

        class UnsafeFile:
            name = "unsafe\r\nfilename"
            def read(self, _limit):
                return b"data"

        with self.assertRaises(ValueError):
            _encode_multipart({}, {"file": UnsafeFile()})

    def test_path_encoder_rejects_dot_segments(self) -> None:
        for value in (".", ".."):
            with self.subTest(value=value):
                with self.assertRaises(ValueError):
                    _encode_path(value)

    def test_unread_stream_rejects_work_scope_and_omits_account_type(self) -> None:
        client = PatchClientV3(base_url="https://example.com", access_token="participant")
        with self.assertRaises(ValueError):
            client.fieldwork_events(watch="unread", work_id="work")

        class Response:
            status = 200
            headers = {"Content-Type": "text/event-stream"}
            def close(self):
                pass
            def __iter__(self):
                return iter(())

        with patch.object(client._opener, "open", return_value=Response()) as open_mock:
            with client.fieldwork_events(watch="unread"):
                pass
        headers = open_mock.call_args.args[0].headers
        self.assertNotIn("Account-type", headers)
        self.assertEqual(headers["Authorization"], "Bearer participant")

    def test_attachment_download_suppresses_client_credentials_and_returns_bytes(self) -> None:
        class StubClient(PatchClientV3):
            def _request(self, method, path, **kwargs):  # type: ignore[override]
                self.call = method, path, kwargs
                return b'{"still":"bytes"}'

        client = StubClient(
            base_url="https://example.com", access_token="token", account_type="manager"
        )
        result = client.fieldwork_attachment_download("work", "key", "1", "signature")
        self.assertEqual(result, b'{"still":"bytes"}')
        method, path, kwargs = client.call
        self.assertEqual((method, path), ("GET", "/api/v3/fieldwork/attachments/download"))
        self.assertTrue(kwargs["raw_response"])
        self.assertTrue(kwargs["no_redirect"])
        self.assertTrue(kwargs["suppress_credentials"])
        self.assertNotIn("Authorization", kwargs["headers"])
        self.assertNotIn("Account-Type", kwargs["headers"])

    def test_attachment_content_returns_bytes_without_content_type_decoding(self) -> None:
        class StubClient(PatchClientV3):
            def _request(self, method, path, **kwargs):  # type: ignore[override]
                self.call = method, path, kwargs
                return b'{"still":"bytes"}'

        client = StubClient(base_url="https://example.com", access_token="token")
        self.assertEqual(
            client.fieldwork_attachment_content("work", "key"), b'{"still":"bytes"}'
        )
        self.assertTrue(client.call[2]["raw_response"])

    def test_urllib_multipart_and_signed_download_wire_behavior(self) -> None:
        received = []

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_POST(self):
                received.append((self.path, dict(self.headers), self.rfile.read(int(self.headers["Content-Length"]))))
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(b"{}")

            def do_GET(self):
                received.append((self.path, dict(self.headers), b""))
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(b"\xffraw")

        server = HTTPServer(("127.0.0.1", 0), Handler)
        thread = Thread(target=lambda: [server.handle_request() for _ in range(3)])
        thread.start()
        try:
            client = PatchClientV3(
                base_url=f"http://127.0.0.1:{server.server_port}",
                allow_insecure_http=True,
                access_token="token",
                account_type="manager",
                default_headers={"Content-Length": "1", "Content-Type": "text/plain",
                                 "Authorization": "Bearer default", "Account-Type": "manager"},
                timeout=2,
            )
            client.upload_plant_files("plant", b"file-data", name="report")
            self.assertEqual(
                client.fieldwork_attachment_download("work", "key", "1", "signature"),
                b"\xffraw",
            )
            self.assertEqual(
                client.call_operation("fieldwork_attachment_download", work_id="work",
                                      object_key="key", expires="1", signature="signature"),
                b"\xffraw",
            )
        finally:
            thread.join(2)
            server.server_close()
        self.assertFalse(thread.is_alive())
        upload_path, upload_headers, upload_body = received[0]
        self.assertEqual(upload_path, "/api/v3/plants/plant/files")
        self.assertNotEqual(upload_headers["Content-Length"], "1")
        self.assertTrue(upload_headers["Content-Type"].startswith("multipart/form-data;"))
        multipart = BytesParser(policy=policy.default).parsebytes(
            b"Content-Type: " + upload_headers["Content-Type"].encode() + b"\r\n\r\n" + upload_body
        )
        parts = list(multipart.iter_parts())
        self.assertEqual(parts[0].get_payload(decode=True), b"report")
        self.assertEqual(parts[1].get_payload(decode=True), b"file-data")
        for _, download_headers, _ in received[1:]:
            self.assertNotIn("Authorization", download_headers)
            self.assertNotIn("Account-Type", download_headers)

    def test_participant_stream_removes_inherited_account_type(self) -> None:
        received = []

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_GET(self):
                received.append(dict(self.headers))
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream; charset=utf-8")
                self.end_headers()

        server = HTTPServer(("127.0.0.1", 0), Handler)
        thread = Thread(target=server.handle_request)
        thread.start()
        try:
            client = PatchClientV3(
                base_url=f"http://127.0.0.1:{server.server_port}",
                allow_insecure_http=True,
                access_token="participant",
                account_type="manager",
                default_headers={"Account-Type": "manager"},
                timeout=2,
            )
            with client.fieldwork_events(watch="unread", account_type=""):
                pass
        finally:
            thread.join(2)
            server.server_close()
        self.assertFalse(thread.is_alive())
        self.assertNotIn("Account-Type", received[0])
        self.assertEqual(received[0]["Authorization"], "Bearer participant")


if __name__ == "__main__":
    unittest.main()
