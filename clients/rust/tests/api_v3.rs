use patch_client::{Client, Error};
use serde_json::json;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn read_request(stream: &mut TcpStream) -> String {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut data = Vec::new();
    let mut buf = [0; 4096];
    loop {
        let n = stream.read(&mut buf).unwrap();
        assert!(n > 0, "request closed before headers");
        data.extend_from_slice(&buf[..n]);
        if let Some(end) = data.windows(4).position(|x| x == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&data[..end]).to_ascii_lowercase();
            let length = headers
                .lines()
                .find_map(|line| {
                    line.strip_prefix("content-length: ")
                        .and_then(|n| n.parse::<usize>().ok())
                })
                .unwrap_or(0);
            if data.len() >= end + 4 + length {
                break;
            }
        }
    }
    String::from_utf8(data).unwrap()
}

fn server(
    check: impl FnOnce(String) + Send + 'static,
    status: u16,
    body: &'static str,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        check(read_request(&mut stream));
        write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    });
    (url, task)
}

#[tokio::test]
async fn commands_keep_identity_auth_and_do_not_replay() {
    let (url, task) = server(
        |req| {
            assert!(req.starts_with("POST /api/v3/fieldwork/works/work%20one/messages/read "));
            let headers = req.to_ascii_lowercase();
            assert!(headers.contains("authorization: bearer participant-token\r\n"));
            assert!(!headers.contains("account-type:"));
            assert!(headers.contains("idempotency-key: read-key-0001\r\n"));
            assert!(req.ends_with("{\"through_message_id\":\"message-1\"}"));
        },
        401,
        "{\"title\":\"Unauthorized\"}",
    );
    let client = Client::new(&url).unwrap();
    client
        .set_access_token("bearer participant-token", None)
        .await
        .unwrap();
    let err = client
        .fieldwork_messages_read_v3(
            "work one",
            &json!({"through_message_id":"message-1"}),
            "read-key-0001",
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        Error::Api { status: 401, .. } | Error::ApiProblem { status: 401, .. }
    ));
    task.join().unwrap();
}

#[tokio::test]
async fn signed_download_omits_credentials_and_preserves_json_file_bytes() {
    let (url, task) = server(
        |req| {
            assert!(req.starts_with("GET /api/v3/fieldwork/attachments/download?"));
            let headers = req.to_ascii_lowercase();
            assert!(!headers.contains("authorization:"));
            assert!(!headers.contains("account-type:"));
            assert!(req.contains("object_key=file%2Fdata.json"));
        },
        200,
        "{\"file\":true}",
    );
    let client = Client::new(&url).unwrap();
    client
        .set_access_token("manager-token", Some("manager"))
        .await
        .unwrap();
    let bytes = client
        .fieldwork_attachment_download_v3(&[
            ("work_id", "work-1"),
            ("object_key", "file/data.json"),
            ("expires", "1234567890"),
            ("signature", "signed-value"),
        ])
        .await
        .unwrap();
    assert_eq!(bytes, b"{\"file\":true}");
    task.join().unwrap();
}

#[tokio::test]
async fn required_parameters_fail_before_network() {
    let client = Client::new("http://127.0.0.1:1").unwrap();
    for key in ["short", "bad key with spaces", "invalid\r\nheader"] {
        assert!(matches!(
            client.fieldwork_work_create_v3(&json!({}), key).await,
            Err(Error::InvalidPath(_))
        ));
    }
    assert!(matches!(
        client
            .fieldwork_receipt_get_v3(&[("operation", "work.create")])
            .await,
        Err(Error::InvalidPath(_))
    ));
    assert!(matches!(
        client
            .fieldwork_events_v3(&[("watch", "unread"), ("work_id", "w")])
            .await,
        Err(Error::InvalidPath(_))
    ));
    assert!(matches!(
        client
            .fieldwork_events_v3(&[("watch", "unread"), ("watch", "inbox")])
            .await,
        Err(Error::InvalidPath(_))
    ));
    assert!(matches!(
        client.fieldwork_work_get_v3("..", &[]).await,
        Err(Error::InvalidPath(_))
    ));
    assert!(matches!(
        client
            .set_access_token("token\r\nHeader: injected", None)
            .await,
        Err(Error::InvalidPath(_))
    ));
}

#[tokio::test]
async fn native_multipart_upload_uses_form_body() {
    let (url, task) = server(
        |req| {
            assert!(req.starts_with("POST /api/v3/plants/plant-1/images "));
            assert!(req
                .to_ascii_lowercase()
                .contains("content-type: multipart/form-data; boundary="));
            assert!(req
                .to_ascii_lowercase()
                .contains("account-type: manager\r\n"));
            assert!(req.contains("name=\"filename\"; filename=\"photo.jpg\""));
            assert!(req.contains("photo-bytes"));
        },
        200,
        "{\"id\":\"image-1\"}",
    );
    let client = Client::new(&url).unwrap();
    client
        .set_access_token("manager-token", Some("manager"))
        .await
        .unwrap();
    let form = reqwest::multipart::Form::new().text("name", "photo").part(
        "filename",
        reqwest::multipart::Part::bytes(b"photo-bytes".to_vec()).file_name("photo.jpg"),
    );
    assert_eq!(
        client
            .upload_plant_images_v3("plant-1", form)
            .await
            .unwrap(),
        json!({"id":"image-1"})
    );
    task.join().unwrap();
}

#[tokio::test]
async fn events_return_before_body_completion_and_close_on_drop() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (finish_tx, finish_rx) = mpsc::channel();
    let task = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let req = read_request(&mut stream);
        assert!(req.starts_with("GET /api/v3/fieldwork/events?watch=unread "));
        assert!(req
            .to_ascii_lowercase()
            .contains("accept: text/event-stream\r\n"));
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\nevent: ready\ndata: {}\n\n").unwrap();
        finish_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    });
    let client = Client::new(&url).unwrap();
    client.set_access_token("participant", None).await.unwrap();
    let mut res = tokio::time::timeout(
        Duration::from_secs(3),
        client.fieldwork_events_v3(&[("watch", "unread")]),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        res.chunk().await.unwrap().unwrap().as_ref(),
        b"event: ready\ndata: {}\n\n"
    );
    drop(res);
    finish_tx.send(()).unwrap();
    task.join().unwrap();
}
