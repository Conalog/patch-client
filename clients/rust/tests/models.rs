use patch_client::model::{ErrorModel, MetricsBody};

#[test]
fn metrics_body_deserializes_panel_intraday() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "panel",
        "source": "device",
        "date": "2026-01-01",
        "interval": "5m",
        "data": [
            {
                "id": "a1",
                "date": "2026-01-01",
                "timestamp": 1,
                "energy": 1.0,
                "cumulative_energy": 2.0,
                "i_out": 3.0,
                "p": 4.0,
                "v_in": 5.0,
                "v_out": 6.0,
                "temp": 7.0
            }
        ]
    }"#;

    let body: MetricsBody = serde_json::from_str(json).unwrap();
    match body {
        MetricsBody::PanelIntraday(v) => {
            assert_eq!(v.plant_id, "p1");
            assert_eq!(v.date, "2026-01-01");
            assert!(v.data.unwrap().len() == 1);
        }
        _ => panic!("expected PanelIntraday"),
    }
}

#[test]
fn error_model_deserializes_problem_json() {
    let json = r#"{
        "title": "Bad Request",
        "status": 400,
        "detail": "invalid input",
        "type": "https://example.com/problem",
        "errors": [{"location": "body.email", "message": "required"}]
    }"#;

    let model: ErrorModel = serde_json::from_str(json).unwrap();
    assert_eq!(model.title.as_deref(), Some("Bad Request"));
    assert_eq!(model.status, Some(400));
    assert_eq!(model.detail.as_deref(), Some("invalid input"));
    assert!(model.errors.as_ref().unwrap().len() == 1);
}
