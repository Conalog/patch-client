# patch-client (Go)

PATCH Plant Data API v3용 수작업 Go 클라이언트입니다.

## 사용

```go
client := patchclient.NewClient("https://patch-api.conalog.com")
client.SetAccessToken("token")
client.SetAccountType(patchclient.AccountTypeManager)

plants, err := client.GetPlantList(ctx, map[string]string{"page": "0", "size": "20"}, nil)
```
