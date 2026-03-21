use acp_cli::queue::messages::{QueueRequest, QueueResponse};

#[test]
fn prompt_request_roundtrip() {
    let msg = QueueRequest::Prompt {
        messages: vec!["hello".into(), "world".into()],
        reply_id: "r1".into(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: QueueRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, parsed);
}

#[test]
fn cancel_request_roundtrip() {
    let msg = QueueRequest::Cancel;
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: QueueRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, parsed);
}

#[test]
fn status_request_roundtrip() {
    let msg = QueueRequest::Status;
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: QueueRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, parsed);
}

#[test]
fn all_response_types_roundtrip() {
    let responses: Vec<QueueResponse> = vec![
        QueueResponse::PromptResult {
            reply_id: "r1".into(),
            content: "answer".into(),
            stop_reason: "end_turn".into(),
        },
        QueueResponse::Event {
            kind: "progress".into(),
            data: "50%".into(),
        },
        QueueResponse::StatusResponse {
            state: "idle".into(),
            queue_depth: 0,
        },
        QueueResponse::Error {
            message: "something went wrong".into(),
        },
        QueueResponse::Queued {
            reply_id: "r2".into(),
            position: 3,
        },
    ];

    for msg in &responses {
        let json = serde_json::to_string(msg).unwrap();
        let parsed: QueueResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(*msg, parsed);
    }
}

#[test]
fn prompt_request_json_shape() {
    let msg = QueueRequest::Prompt {
        messages: vec!["hi".into()],
        reply_id: "r1".into(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let val: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(val["type"], "prompt");
    assert_eq!(val["reply_id"], "r1");
    assert!(val["messages"].is_array());
}

#[test]
fn cancel_request_json_shape() {
    let msg = QueueRequest::Cancel;
    let json = serde_json::to_string(&msg).unwrap();
    let val: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(val["type"], "cancel");
}

#[tokio::test]
async fn socket_send_recv() {
    use acp_cli::queue::ipc::{cleanup_socket, recv_message, send_message, start_ipc_server};
    use tokio::io::BufReader;
    use tokio::net::UnixStream;

    // Use a unique key based on test name + pid to avoid collisions.
    let session_key = format!("test_ipc_{}", std::process::id());

    let listener = start_ipc_server(&session_key).await.unwrap();

    // Spawn a task that accepts one connection and echoes a response.
    let key_clone = session_key.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();

        // Wrap in BufReader to read the request.
        let mut reader = BufReader::new(stream);
        let req: Option<QueueRequest> = recv_message(&mut reader).await.unwrap();
        assert!(req.is_some());
        let req = req.unwrap();
        match &req {
            QueueRequest::Status => {}
            _ => panic!("expected Status request"),
        }

        // Recover the underlying stream to send a response.
        let mut stream = reader.into_inner();
        let resp = QueueResponse::StatusResponse {
            state: "idle".into(),
            queue_depth: 0,
        };
        send_message(&mut stream, &resp).await.unwrap();

        cleanup_socket(&key_clone);
    });

    // Client side: connect and send a request.
    let socket_path = acp_cli::queue::ipc::socket_path(&session_key);
    let mut client = UnixStream::connect(&socket_path).await.unwrap();

    let req = QueueRequest::Status;
    send_message(&mut client, &req).await.unwrap();

    // Read response.
    let mut reader = BufReader::new(client);
    let resp: Option<QueueResponse> = recv_message(&mut reader).await.unwrap();
    assert!(resp.is_some());
    let resp = resp.unwrap();
    assert_eq!(
        resp,
        QueueResponse::StatusResponse {
            state: "idle".into(),
            queue_depth: 0,
        }
    );

    server.await.unwrap();
}
