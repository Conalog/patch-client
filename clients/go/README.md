# patch-client (Go)

Handwritten Go client for PATCH Plant Data API v3.

## Usage

```go
client := patchclient.NewClient("https://patch-api.conalog.com")
client.SetAccessToken("token")
client.SetAccountType(patchclient.AccountTypeManager)

plants, err := client.GetPlantList(ctx, map[string]string{"page": "0", "size": "20"}, nil)
```

## Newly Added APIs

- `ListOAuthMethods(ctx, query, opts)`
- `GetOAuth2LoginURL(ctx, provider, redirectURL)`
- `ListCombinerModelInfo(ctx, opts)`
- `ListInverterModelInfo(ctx, opts)`
- `ListModuleModelInfo(ctx, opts)`
- `GetDeviceState(ctx, plantID, date, kind, opts)`
- `GetPlantRegistryStat(ctx, plantID, date, opts)`

## Redirect Policy

The client intentionally disables redirect following for auth-bearing, body-bearing,
or custom-header requests (anything beyond `Accept`/`Content-Type`).
This is stricter than the default `net/http` behavior to reduce credential/context replay risk.
