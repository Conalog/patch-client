# patch-client (Python)

Handwritten Python client for PATCH Plant Data API v3.

## Installation

```bash
pip install patch-client
```

## Usage

```python
from patch_client import PatchClientV3

client = PatchClientV3(access_token="token", account_type="manager")
plants = client.get_plant_list(page=1, size=20, full=True)
health = client.get_asset_health_level("plant-id", "inverter", "2026-04-13")

client.assign_plant_permission(
    "organization-id",
    "plant-id",
    {"username": "user-id", "type": "viewer"},
)
client.start_plant_comment_thread("plant-id", {"text": "Check inverter 1"})
```

The client covers all v3 operations. New JSON methods use the snake_case
operation ID and accept named path and query values plus payload. Fieldwork command methods
require idempotency_key with 8-128 safe characters.

```python
client.fieldwork_work_create(
    payload={"title": "Inspection"},
    idempotency_key="work-create-20261008",
)

with client.fieldwork_events(watch="unread") as events:
    for line in events:
        print(line.decode("utf-8").rstrip())
```

Fieldwork participant tokens omit account_type. Managers can provide it.
Event streams keep the urllib timeout for each blocking read and must be closed.
fieldwork_events with watch="unread" cannot include work_id or surface.
Use fieldwork_upload_attachment, upload_plant_files, and upload_plant_images
for multipart uploads. They accept bytes or a binary file object.
Multipart requests buffer at most 10 MiB of encoded wire data. Add streaming
uploads if larger files become necessary.
If a client was configured for a manager, call set_account_type(None) before
using a participant token.
For one participant request with inherited manager headers, pass account_type=""
to remove Account-Type from the final request.

OAuth login endpoints are also exposed:

```python
methods = client.list_oauth_methods(provider="google")
redirect_url = client.start_oauth_login("google", redirect_url="https://app.example/callback")
```
