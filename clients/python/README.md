# patch-client (Python)

Handwritten Python client for PATCH Plant Data API v3.

## Installation

```bash
pip install patch-client
```

## Newly Added APIs

- `list_oauth_methods(provider=None, redirect_url=None)`
- `get_oauth2_login_url(provider, redirect_url=None)`
- `list_combiner_model_info()`
- `list_inverter_model_info()`
- `list_module_model_info()`
- `get_device_state(plant_id, date, kind)`
- `get_plant_registry_stat(plant_id, date)`


## Usage

```python
from patch_client import PatchClientV3

client = PatchClientV3(access_token="token", account_type="manager")
plants = client.get_plant_list(page=1, size=20)
```
