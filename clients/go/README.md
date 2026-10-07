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

## Fieldwork

Fieldwork command methods require an `Idempotency-Key` header. It must match
`[A-Za-z0-9._:-]{8,128}`. Pass it through `RequestOptions.Headers`.

```go
opts := &patchclient.RequestOptions{Headers: map[string]string{
    "Idempotency-Key": "command-20261008",
}}
work, err := client.FieldworkWorkCreate(ctx, payload, opts)
```

Fieldwork requests use the client's configured `Account-Type`. For participant
tokens, set `OmitAccountType: true` in `RequestOptions`.

Uploads stream from the supplied `io.Reader`. The caller owns that reader. If a
custom reader can block, close or unblock it after context cancellation.

`FieldworkEvents` returns the raw SSE body. Read it as a stream and close it.

```go
stream, err := client.FieldworkEvents(ctx, map[string]string{"watch": "unread"}, nil)
if err != nil { /* handle error */ }
defer stream.Close()
```

## Redirect Policy

The client intentionally disables redirect following for auth-bearing, body-bearing,
or custom-header requests (anything beyond `Accept`/`Content-Type`).
This is stricter than the default `net/http` behavior to reduce credential/context replay risk.
