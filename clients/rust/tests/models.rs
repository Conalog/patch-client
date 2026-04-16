use patch_client::model::{
    AuthBody, AuthMethodsBody, AuthOutputV3Body, AuthWithPasswordBody, CreatePlantInput,
    CreateOrgMemberRequest, ErrorModel, ListOutputModuleItemBody, MetricsBody,
    OrgAddPermissionInputBody, OrgAddPermissionOutputBody, OrgInfo, PlantBody,
    RegistryOutputBody, StatPoint,
};
use std::collections::HashMap;

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
fn error_model_rejects_null_scalar_fields_and_unknown_keys() {
    let null_json = r#"{
        "title": null
    }"#;
    serde_json::from_str::<ErrorModel>(null_json).expect_err("null title must fail");

    let unknown_json = r#"{
        "title": "Bad Request",
        "extra": true
    }"#;
    serde_json::from_str::<ErrorModel>(unknown_json).expect_err("unknown keys must fail");
}

#[test]
fn metrics_body_uses_plant_aggregated_variant_for_plant_day_payload() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "plant",
        "source": "device",
        "date": "2026-01-01",
        "interval": "1d",
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
        assert_eq!(v.source, "device");
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
        "interval": "1d",
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
            assert_eq!(v.interval, "1d");
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
        "interval": "1d",
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
            assert_eq!(v.interval, "1d");
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
        "source": "device",
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
            assert_eq!(v.source, "device");
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
        "interval": "1d",
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
        "interval": "1d",
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
        schema: None,
        token: "tok-xyz".to_string(),
        name: "manager".to_string(),
    };
    let auth_dbg = format!("{auth:?}");
    assert!(!auth_dbg.contains("tok-xyz"));
    assert!(auth_dbg.contains("<redacted>"));
}

#[test]
fn auth_body_allows_schema_but_rejects_unknown_keys() {
    let ok_json = r#"{
        "$schema": "https://patch-api.conalog.com/schemas/AuthBody.json",
        "token": "tok-xyz",
        "name": "manager"
    }"#;
    let body: AuthBody = serde_json::from_str(ok_json).expect("$schema should be accepted");
    assert_eq!(body.schema.as_deref(), Some("https://patch-api.conalog.com/schemas/AuthBody.json"));

    let unknown_json = r#"{
        "token": "tok-xyz",
        "name": "manager",
        "extra": true
    }"#;
    serde_json::from_str::<AuthBody>(unknown_json).expect_err("unknown keys must fail");
}

#[test]
fn org_permission_output_rejects_plant_id_alias() {
    let raw = r#"{
        "$schema": "https://patch-api.conalog.com/schemas/OrgAddPermissionOutputBody.json",
        "plantId": "plant-1",
        "type": "viewer",
        "email": "viewer@example.com"
    }"#;
    serde_json::from_str::<OrgAddPermissionOutputBody>(raw)
        .expect_err("v3 output must reject plantId alias");
}

#[test]
fn new_auth_methods_body_deserializes_providers() {
    let json = r#"{
        "authProviders": [
            {
                "name": "google",
                "state": "signed-state",
                "codeChallenge": "challenge",
                "codeChallengeMethod": "S256",
                "authUrl": "https://patch-api.conalog.com/oauth/google"
            }
        ]
    }"#;

    let body: AuthMethodsBody = serde_json::from_str(json).unwrap();
    let providers = body.auth_providers.expect("providers present");
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0].name, "google");
    assert_eq!(providers[0].code_challenge_method, "S256");
}

#[test]
fn auth_output_organizations_reject_org_info_only_fields() {
    let json = r#"{
        "token": "tok",
        "type": "manager",
        "name": "Manager",
        "organizations": [
            {
                "id": "org-1",
                "name": "Org A",
                "owner": "extra"
            }
        ]
    }"#;

    serde_json::from_str::<AuthOutputV3Body>(json)
        .expect_err("organizations must reject fields outside OrganizationBody");
}

#[test]
fn auth_with_password_body_rejects_invalid_account_type_on_serialize() {
    let body = AuthWithPasswordBody {
        account_type: "admin".to_string(),
        password: "pw".to_string(),
        email: Some("manager@example.com".to_string()),
        username: None,
    };

    serde_json::to_string(&body).expect_err("login body must reject unsupported account type");
}

#[test]
fn auth_with_password_body_rejects_missing_required_identity_fields() {
    let manager_missing_email = AuthWithPasswordBody {
        account_type: "manager".to_string(),
        password: "pw".to_string(),
        email: None,
        username: None,
    };
    serde_json::to_string(&manager_missing_email)
        .expect_err("manager login must require email");

    let viewer_missing_username = AuthWithPasswordBody {
        account_type: "viewer".to_string(),
        password: "pw".to_string(),
        email: None,
        username: None,
    };
    serde_json::to_string(&viewer_missing_username)
        .expect_err("viewer login must require username");
}

#[test]
fn new_auth_methods_body_rejects_missing_required_auth_providers() {
    let json = r#"{}"#;
    let err = serde_json::from_str::<AuthMethodsBody>(json).expect_err("missing authProviders");
    assert!(err.to_string().contains("authProviders"));
}

#[test]
fn new_list_output_module_item_body_deserializes_items() {
    let json = r#"{
        "items": [
            {
                "id": "mod-1",
                "model_name": "Model A",
                "manufacturer": "Maker"
            }
        ]
    }"#;

    let body: ListOutputModuleItemBody = serde_json::from_str(json).unwrap();
    let items = body.items.expect("items present");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "mod-1");
    assert_eq!(items[0].model_name.as_deref(), Some("Model A"));
}

#[test]
fn new_list_output_module_item_body_rejects_missing_items() {
    let json = r#"{}"#;
    let err =
        serde_json::from_str::<ListOutputModuleItemBody>(json).expect_err("missing items");
    assert!(err.to_string().contains("items"));
}

#[test]
fn new_list_output_module_item_body_rejects_non_object_cell_specification() {
    let json = r#"{
        "items": [
            {
                "id": "mod-1",
                "cell_specification": ["invalid"]
            }
        ]
    }"#;

    serde_json::from_str::<ListOutputModuleItemBody>(json)
        .expect_err("cell_specification must be an object when present");
}

#[test]
fn new_stat_point_deserializes_registry_stat_payload() {
    let json = r#"{
        "timestamp": "2026-01-01T14:59:59Z",
        "installed_capacity_w": 12000.0,
        "module_models": [
            {"name": "Panel X", "count": 24}
        ],
        "device_models": [
            {"name": "Device Y", "count": 24, "installed_capacity_w": 12000.0}
        ]
    }"#;

    let body: StatPoint = serde_json::from_str(json).unwrap();
    assert_eq!(body.timestamp, "2026-01-01T14:59:59Z");
    assert_eq!(body.installed_capacity_w, 12000.0);
    assert_eq!(body.module_models.as_ref().unwrap()[0].count, 24);
    assert_eq!(body.device_models.as_ref().unwrap()[0].name, "Device Y");
}

#[test]
fn new_stat_point_rejects_missing_required_model_arrays() {
    let json = r#"{
        "timestamp": "2026-01-01T14:59:59Z",
        "installed_capacity_w": 12000.0
    }"#;
    let err = serde_json::from_str::<StatPoint>(json).expect_err("missing model arrays");
    assert!(
        err.to_string().contains("module_models") || err.to_string().contains("device_models")
    );
}

#[test]
fn new_metrics_body_deserializes_sensor_intraday() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "temperature",
        "source": "sensor",
        "date": "2026-01-01",
        "interval": "5m",
        "data": [
            {
                "id": "s1",
                "date": "2026-01-01",
                "timestamp": 1,
                "min": 18.1,
                "max": 22.3,
                "mean": 20.0,
                "median": 20.1
            }
        ]
    }"#;

    let body: MetricsBody = serde_json::from_str(json).unwrap();
    match body {
        MetricsBody::SensorIntraday(v) => {
            assert_eq!(v.unit, "temperature");
            assert_eq!(v.source, "sensor");
            let data = v.data.unwrap();
            assert_eq!(data[0].id, "s1");
            assert_eq!(data[0].mean, Some(20.0));
        }
        _ => panic!("expected SensorIntraday"),
    }
}

#[test]
fn new_sensor_data_rejects_missing_required_nullable_measurements() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "temperature",
        "source": "sensor",
        "date": "2026-01-01",
        "interval": "5m",
        "data": [
            {
                "id": "s1",
                "date": "2026-01-01",
                "timestamp": 1,
                "min": 18.1,
                "max": 22.3,
                "mean": 20.0
            }
        ]
    }"#;

    let err = serde_json::from_str::<MetricsBody>(json).expect_err("missing median key");
    assert!(err.to_string().contains("median"));
}

#[test]
fn new_metrics_body_rejects_missing_required_data_key() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "panel",
        "source": "device",
        "date": "2026-01-01",
        "interval": "5m"
    }"#;

    let err = serde_json::from_str::<MetricsBody>(json).expect_err("missing data key");
    assert!(err.to_string().contains("data"));
}

#[test]
fn new_metrics_body_treats_legacy_day_interval_as_unknown() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "plant",
        "source": "device",
        "date": "2026-01-01",
        "interval": "day",
        "data": [
            {
                "date": "2026-01-01",
                "energy": 42.0
            }
        ]
    }"#;

    let body: MetricsBody = serde_json::from_str(json).unwrap();
    assert!(matches!(body, MetricsBody::Unknown(_)));
}

#[test]
fn new_metrics_body_uses_1d_for_aggregated_variants() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "plant",
        "source": "device",
        "date": "2026-01-01",
        "interval": "1d",
        "data": [
            {
                "id": "daily-1",
                "date": "2026-01-01",
                "energy": 42.0
            }
        ]
    }"#;

    let body: MetricsBody = serde_json::from_str(json).unwrap();
    assert!(matches!(body, MetricsBody::PlantAggregated(_)));
}

#[test]
fn new_registry_output_body_rejects_non_object_asset_model_and_tag() {
    let json = r#"{
        "asset_id": "a1",
        "asset_model": "not-an-object",
        "asset_type": "device",
        "map_id": "m1",
        "map_type": "device",
        "registered": "2026-01-01T00:00:00Z",
        "tag": [],
        "unregistered": ""
    }"#;

    serde_json::from_str::<RegistryOutputBody>(json).expect_err("non-object fields");
}

#[test]
fn new_plant_body_rejects_non_object_metadata() {
    let json = r#"{
        "id": "ask123456789012",
        "name": "Plant A",
        "organization": "org-1",
        "organizationData": {
            "id": "org-1",
            "name": "Org A"
        },
        "created": "2025-01-01 00:00:00.000Z",
        "updated": "2025-01-02 00:00:00.000Z",
        "metadata": [],
        "images": null
    }"#;

    serde_json::from_str::<PlantBody>(json).expect_err("metadata must be an object");
}

#[test]
fn new_org_info_rejects_null_optional_fields_and_unknown_keys() {
    let null_json = r#"{
        "id": "org-1",
        "name": "Org A",
        "icon": null
    }"#;
    serde_json::from_str::<OrgInfo>(null_json).expect_err("icon must not accept null");

    let unknown_json = r#"{
        "id": "org-1",
        "name": "Org A",
        "extra": "nope"
    }"#;
    serde_json::from_str::<OrgInfo>(unknown_json).expect_err("unknown key must fail");
}

#[test]
fn new_plant_body_allows_schema_but_rejects_unknown_keys() {
    let ok_json = r#"{
        "$schema": "https://patch-api.conalog.com/schemas/PlantBody.json",
        "id": "ask123456789012",
        "name": "Plant A",
        "organization": "org-1",
        "organizationData": {
            "id": "org-1",
            "name": "Org A"
        },
        "created": "2025-01-01 00:00:00.000Z",
        "updated": "2025-01-02 00:00:00.000Z",
        "metadata": {},
        "images": []
    }"#;
    let body: PlantBody = serde_json::from_str(ok_json).expect("$schema should be accepted");
    assert_eq!(body.schema.as_deref(), Some("https://patch-api.conalog.com/schemas/PlantBody.json"));

    let unknown_json = r#"{
        "id": "ask123456789012",
        "name": "Plant A",
        "organization": "org-1",
        "organizationData": {
            "id": "org-1",
            "name": "Org A"
        },
        "created": "2025-01-01 00:00:00.000Z",
        "updated": "2025-01-02 00:00:00.000Z",
        "metadata": {},
        "images": [],
        "unexpected": true
    }"#;
    serde_json::from_str::<PlantBody>(unknown_json).expect_err("unknown key must fail");
}

#[test]
fn new_metrics_body_keeps_non_sensor_source_unknown_for_sensor_units() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "temperature",
        "source": "device",
        "date": "2026-01-01",
        "interval": "5m",
        "data": [
            {
                "id": "s1",
                "date": "2026-01-01",
                "timestamp": 1,
                "min": 18.1,
                "max": 22.3,
                "mean": 20.0,
                "median": 20.1
            }
        ]
    }"#;

    let body: MetricsBody = serde_json::from_str(json).unwrap();
    assert!(matches!(body, MetricsBody::Unknown(_)));
}

#[test]
fn new_metrics_body_keeps_sensor_hourly_unknown() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "temperature",
        "source": "sensor",
        "date": "2026-01-01",
        "interval": "1h",
        "data": [
            {
                "id": "s1",
                "date": "2026-01-01",
                "timestamp": 1,
                "min": 18.1,
                "max": 22.3,
                "mean": 20.0,
                "median": 20.1
            }
        ]
    }"#;

    let body: MetricsBody = serde_json::from_str(json).unwrap();
    assert!(matches!(body, MetricsBody::Unknown(_)));
}

#[test]
fn org_permission_output_accepts_snake_case_plant_id() {
    let raw = r#"{
        "plant_id": "plant-2",
        "type": "manager",
        "email": "manager@example.com"
    }"#;
    let model: OrgAddPermissionOutputBody =
        serde_json::from_str(raw).expect("must parse plant_id field");
    assert_eq!(model.plant_id, "plant-2");
    assert_eq!(model.account_type, "manager");
}

#[test]
fn org_permission_output_rejects_null_and_unknown_fields() {
    let null_json = r#"{
        "plantId": "plant-3",
        "type": "viewer",
        "username": null
    }"#;
    serde_json::from_str::<OrgAddPermissionOutputBody>(null_json)
        .expect_err("username must not accept null");

    let unknown_json = r#"{
        "plantId": "plant-3",
        "type": "viewer",
        "extra": true
    }"#;
    serde_json::from_str::<OrgAddPermissionOutputBody>(unknown_json)
        .expect_err("unknown keys must fail");
}

#[test]
fn org_permission_input_rejects_invalid_id_and_account_type_on_serialize() {
    let bad_id = OrgAddPermissionInputBody {
        plant_id: "short-id".to_string(),
        account_type: "viewer".to_string(),
        email: None,
        username: Some("viewer1".to_string()),
    };
    serde_json::to_string(&bad_id).expect_err("plant_id must be exactly 15 characters");

    let bad_type = OrgAddPermissionInputBody {
        plant_id: "pln123456789012".to_string(),
        account_type: "admin".to_string(),
        email: Some("manager@example.com".to_string()),
        username: None,
    };
    serde_json::to_string(&bad_type).expect_err("permission input must reject admin");
}

#[test]
fn create_org_member_request_rejects_missing_identity_fields() {
    let bad_manager = CreateOrgMemberRequest {
        account_type: "manager".to_string(),
        name: "Manager".to_string(),
        email: None,
        username: None,
        metadata: None,
    };
    serde_json::to_string(&bad_manager)
        .expect_err("manager org member request must require email");

    let bad_viewer = CreateOrgMemberRequest {
        account_type: "viewer".to_string(),
        name: "Viewer".to_string(),
        email: None,
        username: None,
        metadata: None,
    };
    serde_json::to_string(&bad_viewer)
        .expect_err("viewer org member request must require username");
}

#[test]
fn create_plant_input_rejects_non_alnum_organization_id_on_serialize() {
    let body = CreatePlantInput {
        name: "Plant".to_string(),
        organization_id: "org-12345678901".to_string(),
        metadata: Some(HashMap::new()),
    };

    serde_json::to_string(&body)
        .expect_err("organization_id must be exactly 15 ASCII alphanumeric characters");
}

#[test]
fn registry_output_rejects_invalid_enum_values() {
    let json = r#"{
        "asset_id": "a1",
        "asset_model": {},
        "asset_type": "panel",
        "map_id": "m1",
        "map_type": "rack",
        "registered": "2026-01-01T00:00:00Z",
        "tag": {},
        "unregistered": ""
    }"#;

    serde_json::from_str::<RegistryOutputBody>(json)
        .expect_err("registry enums must reject unknown values");
}

#[test]
fn metrics_body_inverter_daily_rejects_missing_date() {
    let json = r#"{
        "plant_id": "p1",
        "unit": "inverter",
        "source": "device",
        "date": "2026-01-01",
        "interval": "1d",
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
        "source": "device",
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
