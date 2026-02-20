import ast
from pathlib import Path
import unittest
from io import BytesIO
from unittest.mock import patch
from urllib.error import HTTPError, URLError

from patch_client.client import (
    FilePart,
    PatchClientError,
    PatchClientV3,
    _SafeRedirectHandler,
    _decode_response,
    _encode_multipart,
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

    def test_encode_multipart_rejects_field_name_with_crlf(self) -> None:
        with self.assertRaises(ValueError):
            _encode_multipart({"name\r\nX-Injected: 1": "value"}, {})

    def test_encode_multipart_rejects_content_type_with_crlf(self) -> None:
        with self.assertRaises(ValueError):
            _encode_multipart(
                {},
                {
                    "filename": FilePart(
                        filename="ok.txt",
                        content=b"body",
                        content_type="text/plain\r\nX-Injected: 1",
                    )
                },
            )

    def test_upload_plant_files_requires_at_least_one_file(self) -> None:
        class StubClient(PatchClientV3):
            def __init__(self) -> None:
                super().__init__(base_url="https://example.com")
                self.called = False

            def _request(self, *args, **kwargs):  # type: ignore[override]
                self.called = True
                return None

        client = StubClient()
        with self.assertRaises(ValueError):
            client.upload_plant_files("plant-id", {})
        self.assertFalse(client.called)

    def test_encode_multipart_rejects_when_payload_too_large(self) -> None:
        with self.assertRaises(ValueError):
            _encode_multipart(
                {},
                {"f": FilePart(filename="x.bin", content=b"x" * 64)},
                max_total_bytes=32,
            )

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

    def test_encode_multipart_does_not_over_reject_small_valid_payload(self) -> None:
        content_type, payload = _encode_multipart(
            {},
            {"filename": FilePart(filename="a.txt", content=b"x")},
            max_total_bytes=512,
        )
        self.assertIn("multipart/form-data", content_type)
        self.assertLessEqual(len(payload), 512)

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
