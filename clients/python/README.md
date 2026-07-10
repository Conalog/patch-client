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

OAuth login endpoints are also exposed:

```python
methods = client.list_oauth_methods(provider="google")
redirect_url = client.start_oauth_login("google", redirect_url="https://app.example/callback")
```
