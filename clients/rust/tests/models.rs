use patch_client::model::{
    AuthBody, AuthWithPasswordBody, ErrorModel, MetricsBody, OrgAddPermissionOutputBody,
};

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
            assert_eq!(v.unit, "panel");
            assert_eq!(v.source, "device");
            let data = v.data.unwrap();
            assert_eq!(data.len(), 1);
            assert_eq!(data[0].id, "a1");
            assert_eq!(data[0].timestamp, 1);
            assert_eq!(data[0].energy, 1.0);
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

#[test]
fn metrics_body_uses_plant_aggregated_variant_for_plant_day_payload() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "plant",
        "source": "summary",
        "date": "2026-01-01",
        "interval": "day",
        "data": [
            {
                "id": "daily-1",
                "date": "2026-01-01",
                "energy": 42.0
            }
        ]
    }"#;

    let body: MetricsBody = serde_json::from_str(json).unwrap();
    assert!(
        matches!(body, MetricsBody::PlantAggregated(_)),
        "plant/day payload should deserialize to PlantAggregated"
    );
    if let MetricsBody::PlantAggregated(v) = body {
        assert_eq!(v.unit, "plant");
        assert_eq!(v.source, "summary");
        let data = v.data.unwrap();
        assert_eq!(data[0].id.as_deref(), Some("daily-1"));
        assert_eq!(data[0].date, "2026-01-01");
        assert_eq!(data[0].energy, 42.0);
    }
}

#[test]
fn metrics_body_preserves_unknown_discriminants() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "unknown",
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

    let body =
        serde_json::from_str::<MetricsBody>(json).expect("must preserve unknown unit/interval");
    match body {
        MetricsBody::Unknown(raw) => {
            assert_eq!(
                raw.get("unit").and_then(serde_json::Value::as_str),
                Some("unknown")
            );
            assert_eq!(
                raw.get("interval").and_then(serde_json::Value::as_str),
                Some("5m")
            );
        }
        _ => panic!("expected Unknown metrics variant"),
    }
}

#[test]
fn metrics_body_deserializes_panel_daily() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "panel",
        "source": "device",
        "date": "2026-01-01",
        "interval": "day",
        "data": [
            {
                "id": "panel-1",
                "energy": 12.5
            }
        ]
    }"#;

    let body: MetricsBody = serde_json::from_str(json).unwrap();
    match body {
        MetricsBody::PanelDaily(v) => {
            assert_eq!(v.plant_id, "p1");
            assert_eq!(v.unit, "panel");
            assert_eq!(v.source, "device");
            assert_eq!(v.date, "2026-01-01");
            assert_eq!(v.interval, "day");
            let data = v.data.unwrap();
            assert_eq!(data.len(), 1);
            assert_eq!(data[0].id, "panel-1");
            assert_eq!(data[0].energy, 12.5);
        }
        _ => panic!("expected PanelDaily"),
    }
}

#[test]
fn metrics_body_deserializes_inverter_intraday() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "inverter",
        "source": "device",
        "date": "2026-01-01",
        "interval": "5m",
        "data": [
            {
                "id": "inv-1",
                "time": "10:00",
                "energy": 3.2,
                "timestamp": 1.0
            }
        ]
    }"#;

    let body: MetricsBody = serde_json::from_str(json).unwrap();
    match body {
        MetricsBody::InverterIntraday(v) => {
            assert_eq!(v.unit, "inverter");
            assert_eq!(v.source, "device");
            assert_eq!(v.date, "2026-01-01");
            assert_eq!(v.interval, "5m");
            let data = v.data.unwrap();
            assert_eq!(data.len(), 1);
            assert_eq!(data[0].id, "inv-1");
            assert_eq!(data[0].time, "10:00");
            assert_eq!(data[0].energy, 3.2);
            assert_eq!(data[0].timestamp, 1.0);
        }
        _ => panic!("expected InverterIntraday"),
    }
}

#[test]
fn metrics_body_deserializes_inverter_daily() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "inverter",
        "source": "device",
        "date": "2026-01-01",
        "interval": "day",
        "data": [
            {
                "id": "inv-1",
                "date": "2026-01-01",
                "energy": 9.8
            }
        ]
    }"#;

    let body: MetricsBody = serde_json::from_str(json).unwrap();
    match body {
        MetricsBody::InverterDaily(v) => {
            assert_eq!(v.unit, "inverter");
            assert_eq!(v.source, "device");
            assert_eq!(v.date, "2026-01-01");
            assert_eq!(v.interval, "day");
            let data = v.data.unwrap();
            assert_eq!(data.len(), 1);
            assert_eq!(data[0].id, "inv-1");
            assert_eq!(data[0].date, "2026-01-01");
            assert_eq!(data[0].energy, 9.8);
        }
        _ => panic!("expected InverterDaily"),
    }
}

#[test]
fn metrics_body_deserializes_plant_intraday() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "plant",
        "source": "summary",
        "date": "2026-01-01",
        "interval": "5m",
        "data": [
            {
                "date": "2026-01-01",
                "energy": 4.4,
                "cumulative_energy": 8.8,
                "timestamp": 1
            }
        ]
    }"#;

    let body: MetricsBody = serde_json::from_str(json).unwrap();
    match body {
        MetricsBody::PlantIntraday(v) => {
            assert_eq!(v.unit, "plant");
            assert_eq!(v.source, "summary");
            assert_eq!(v.date, "2026-01-01");
            assert_eq!(v.interval, "5m");
            let data = v.data.unwrap();
            assert_eq!(data.len(), 1);
            assert_eq!(data[0].date, "2026-01-01");
            assert_eq!(data[0].energy, 4.4);
            assert_eq!(data[0].cumulative_energy, 8.8);
            assert_eq!(data[0].timestamp, 1);
        }
        _ => panic!("expected PlantIntraday"),
    }
}

#[test]
fn metrics_body_panel_daily_allows_null_data() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "panel",
        "source": "device",
        "date": "2026-01-01",
        "interval": "day",
        "data": null
    }"#;

    let body: MetricsBody = serde_json::from_str(json).unwrap();
    match body {
        MetricsBody::PanelDaily(v) => assert!(v.data.is_none()),
        _ => panic!("expected PanelDaily"),
    }
}

#[test]
fn metrics_body_panel_daily_rejects_missing_energy() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "panel",
        "source": "device",
        "date": "2026-01-01",
        "interval": "day",
        "data": [
            {
                "id": "panel-1"
            }
        ]
    }"#;

    let err = serde_json::from_str::<MetricsBody>(json).expect_err("missing energy must fail");
    assert!(err.to_string().contains("energy"));
}

#[test]
fn metrics_body_inverter_intraday_rejects_string_timestamp() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "inverter",
        "source": "device",
        "date": "2026-01-01",
        "interval": "5m",
        "data": [
            {
                "id": "inv-1",
                "time": "10:00",
                "energy": 3.2,
                "timestamp": "1.0"
            }
        ]
    }"#;

    let err = serde_json::from_str::<MetricsBody>(json).expect_err("string timestamp must fail");
    assert!(err.to_string().contains("invalid type"));
}

#[test]
fn auth_models_redact_secrets_in_debug_output() {
    let login = AuthWithPasswordBody {
        account_type: "manager".to_string(),
        password: "pw-123".to_string(),
        email: Some("manager@example.com".to_string()),
        username: None,
    };
    let login_dbg = format!("{login:?}");
    assert!(!login_dbg.contains("pw-123"));
    assert!(login_dbg.contains("<redacted>"));

    let auth = AuthBody {
        token: "tok-xyz".to_string(),
        name: "manager".to_string(),
    };
    let auth_dbg = format!("{auth:?}");
    assert!(!auth_dbg.contains("tok-xyz"));
    assert!(auth_dbg.contains("<redacted>"));
}

#[test]
fn org_permission_output_accepts_plant_id_alias() {
    let raw = r#"{
        "plantId": "plant-1",
        "type": "viewer",
        "email": "viewer@example.com",
        "username": null
    }"#;
    let model: OrgAddPermissionOutputBody =
        serde_json::from_str(raw).expect("must parse plantId alias");
    assert_eq!(model.plant_id, "plant-1");
    assert_eq!(model.account_type, "viewer");
}

#[test]
fn org_permission_output_accepts_snake_case_plant_id() {
    let raw = r#"{
        "plant_id": "plant-2",
        "type": "manager",
        "email": "manager@example.com",
        "username": null
    }"#;
    let model: OrgAddPermissionOutputBody =
        serde_json::from_str(raw).expect("must parse plant_id field");
    assert_eq!(model.plant_id, "plant-2");
    assert_eq!(model.account_type, "manager");
}

#[test]
fn metrics_body_inverter_daily_rejects_missing_date() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "inverter",
        "source": "device",
        "date": "2026-01-01",
        "interval": "day",
        "data": [
            {
                "id": "inv-1",
                "energy": 9.8
            }
        ]
    }"#;

    let err = serde_json::from_str::<MetricsBody>(json).expect_err("missing date must fail");
    assert!(err.to_string().contains("date"));
}

#[test]
fn metrics_body_plant_intraday_rejects_missing_cumulative_energy() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "plant",
        "source": "summary",
        "date": "2026-01-01",
        "interval": "5m",
        "data": [
            {
                "date": "2026-01-01",
                "energy": 4.4,
                "timestamp": 1
            }
        ]
    }"#;

    let err =
        serde_json::from_str::<MetricsBody>(json).expect_err("missing cumulative_energy must fail");
    assert!(err.to_string().contains("cumulative_energy"));
}
