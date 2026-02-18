import unittest

from patch_client.client import FilePart, PatchClientV3, _decode_response, _encode_multipart


class ClientSafetyTests(unittest.TestCase):
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


if __name__ == "__main__":
    unittest.main()
