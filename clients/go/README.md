# patch-client (Go)

Handwritten Go client for PATCH Plant Data API v3.

## Usage

```go
client := patchclient.NewClient("https://patch-api.conalog.com")
client.SetAccessToken("token")
client.SetAccountType(patchclient.AccountTypeManager)

plants, err := client.GetPlantList(ctx, map[string]string{"page": "0", "size": "20"}, nil)
blueprints, err := client.ListPlantBlueprints(ctx, "unw4id41ud2p0wt", nil)
weather, err := client.GetPlantWeatherForecast(ctx, "unw4id41ud2p0wt", map[string]string{"days": "7"}, nil)
redirect, err := client.StartOAuthLogin(ctx, "google", "myscheme://callback", nil)
```

Most methods return decoded JSON as `any`; `StartOAuthLogin` returns the 302
`Location` header without following the redirect.

## Redirect Policy

The client intentionally disables redirect following for auth-bearing, body-bearing,
or custom-header requests (anything beyond `Accept`/`Content-Type`).
This is stricter than the default `net/http` behavior to reduce credential/context replay risk.
