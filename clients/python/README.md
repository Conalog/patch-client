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
plants = client.get_plant_list(page=1, size=20)
```
