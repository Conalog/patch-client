# patch-client (Python)

PATCH Plant Data API v3용 수작업 Python 클라이언트입니다.

## 설치

```bash
pip install patch-client
```

## 사용

```python
from patch_client import PatchClientV3

client = PatchClientV3(access_token="token", account_type="manager")
plants = client.get_plant_list(page=0, size=20)
```
