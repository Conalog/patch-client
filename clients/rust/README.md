# Rust PATCH API v3 client

The client covers the v3 operations in [the API documentation](https://patch-api.conalog.com/docs).
Existing typed methods keep their names. New methods use the OpenAPI `operationId` plus `_v3`.
New JSON APIs accept and return `serde_json::Value`. Query arguments are slices of `(name, value)` pairs.
Join array query values with commas when the specification uses `explode: false`.

```rust
use patch_client::Client;
use serde_json::json;

let client = Client::new("https://patch-api.conalog.com")?;
// A participant token has no Account-Type. A manager token uses Some("manager").
client.set_access_token("participant-token", None).await?;
let works = client.fieldwork_works_list_v3(&[("status", "active")]).await?;
let result = client.fieldwork_messages_read_v3(
    "work-id", &json!({"through_message_id": "message-id"}), "read-command-0001"
).await?;
```

Fieldwork commands send once. Keep the same body and command key if you explicitly retry.
Use `reqwest::multipart::Form` for the three upload APIs. Attachment forms require the fields listed in the specification.
`fieldwork_events_v3` returns a native `reqwest::Response`; read each chunk with `chunk().await` and drop it to close the stream.
`watch=unread` must be the only query pair. Streams have a connection and idle-read timeout, without a total response deadline.
Binary attachment APIs return bounded `Vec<u8>` values.

Run `cargo test --locked` from the repository root.
