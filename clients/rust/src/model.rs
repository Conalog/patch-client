use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

#[derive(Serialize)]
pub struct AuthWithPasswordBody {
    #[serde(rename = "type")]
    pub account_type: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

impl fmt::Debug for AuthWithPasswordBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthWithPasswordBody")
            .field("account_type", &self.account_type)
            .field("password", &"<redacted>")
            .field("email", &self.email)
            .field("username", &self.username)
            .finish()
    }
}

#[derive(Serialize)]
pub struct AuthAccountBody {
    pub account: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

impl fmt::Debug for AuthAccountBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthAccountBody")
            .field("account", &self.account)
            .field("password", &"<redacted>")
            .finish()
    }
}

#[derive(Serialize)]
pub struct AuthEmailBody {
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

impl fmt::Debug for AuthEmailBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthEmailBody")
            .field("email", &self.email)
            .field("password", &"<redacted>")
            .finish()
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct OrgInfo {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub logo: Option<String>,
    pub owner: Option<String>,
}

pub type OrganizationBody = OrgInfo;

#[derive(Deserialize, Clone)]
pub struct AuthOutputV3Body {
    pub token: String,
    #[serde(rename = "type")]
    pub account_type: String,
    pub name: String,
    pub email: Option<String>,
    pub username: Option<String>,
    pub organizations: Option<Vec<OrganizationBody>>,
    pub metadata: Option<Value>,
}

impl fmt::Debug for AuthOutputV3Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthOutputV3Body")
            .field("token", &"<redacted>")
            .field("account_type", &self.account_type)
            .field("name", &self.name)
            .field("email", &self.email)
            .field("username", &self.username)
            .field("organizations", &self.organizations)
            .field("metadata", &self.metadata)
            .finish()
    }
}

#[derive(Deserialize, Clone)]
pub struct AuthBody {
    pub token: String,
    pub name: String,
}

impl fmt::Debug for AuthBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthBody")
            .field("token", &"<redacted>")
            .field("name", &self.name)
            .finish()
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct AccountOutputBody {
    pub name: String,
    #[serde(rename = "type")]
    pub account_type: String,
    pub email: Option<String>,
    pub username: Option<String>,
    pub organizations: Option<Vec<OrganizationBody>>,
    pub metadata: Option<Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CreateAccountOutputBody {
    pub id: String,
    #[serde(rename = "type")]
    pub account_type: String,
    pub name: String,
    pub email: Option<String>,
    pub username: Option<String>,
    pub organizations: Option<Vec<OrganizationBody>>,
    pub metadata: Option<Value>,
}

#[derive(Serialize, Debug)]
pub struct CreateOrgMemberRequest {
    #[serde(rename = "type")]
    pub account_type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[allow(non_camel_case_types)]
pub type Create_org_memberRequest = CreateOrgMemberRequest;

#[derive(Serialize)]
pub struct CreateUserAccountRequest {
    #[serde(rename = "type")]
    pub account_type: String,
    pub name: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl fmt::Debug for CreateUserAccountRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateUserAccountRequest")
            .field("account_type", &self.account_type)
            .field("name", &self.name)
            .field("password", &"<redacted>")
            .field("email", &self.email)
            .field("username", &self.username)
            .field("metadata", &self.metadata)
            .finish()
    }
}

#[allow(non_camel_case_types)]
pub type Create_user_accountRequest = CreateUserAccountRequest;

#[derive(Serialize, Debug)]
pub struct OrgAddPermissionInputBody {
    #[serde(rename = "plantId")]
    pub plant_id: String,
    #[serde(rename = "type")]
    pub account_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct OrgAddPermissionOutputBody {
    #[serde(rename = "plant_id", alias = "plantId")]
    pub plant_id: String,
    #[serde(rename = "type")]
    pub account_type: String,
    pub email: Option<String>,
    pub username: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct CreatePlantInput {
    pub name: String,
    #[serde(rename = "organizationId")]
    pub organization_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlantBody {
    pub id: String,
    pub name: String,
    pub organization: String,
    #[serde(rename = "organizationData")]
    pub organization_data: OrgInfo,
    pub created: String,
    pub updated: String,
    pub metadata: Value,
    pub images: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlantBodyV3 {
    pub id: String,
    pub name: String,
    pub organization: OrgInfo,
    pub created: String,
    pub updated: String,
    pub metadata: Value,
    pub images: Option<Vec<String>>,
}

impl From<PlantBody> for PlantBodyV3 {
    fn from(value: PlantBody) -> Self {
        Self {
            id: value.id,
            name: value.name,
            organization: value.organization_data,
            created: value.created,
            updated: value.updated,
            metadata: value.metadata,
            images: value.images,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlantsListV3OutputBody {
    pub items: Option<Vec<PlantBodyV3>>,
    pub page: i64,
    #[serde(rename = "perPage")]
    pub per_page: i64,
    #[serde(rename = "totalItems")]
    pub total_items: i64,
    #[serde(rename = "totalPages")]
    pub total_pages: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FileUploadResponse {
    pub id: String,
    pub plant_id: String,
    pub filename: String,
    pub size: i64,
    pub created: String,
    pub updated: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HealthLevelCategory {
    pub count: i64,
    pub ids: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HealthLevelBody {
    pub best: HealthLevelCategory,
    pub caution: HealthLevelCategory,
    pub faulty: HealthLevelCategory,
}

#[derive(Deserialize, Debug, Clone)]
pub struct InverterLogMessage {
    pub ko: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct InverterLogRawElement {
    pub status: String,
    pub code: Option<String>,
    pub lcd: Option<String>,
    pub value: Option<Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct InverterLogItem {
    #[serde(rename = "plantId")]
    pub plant_id: String,
    pub level: String,
    #[serde(rename = "inverterId")]
    pub inverter_id: String,
    pub timestamp: String,
    pub message: InverterLogMessage,
    pub raw: InverterLogRawElement,
}

#[derive(Deserialize, Debug, Clone)]
pub struct InverterLogsResponse {
    pub items: Option<Vec<InverterLogItem>>,
    pub page: i64,
    #[serde(rename = "perPage")]
    pub per_page: i64,
    #[serde(rename = "totalPages")]
    pub total_pages: i64,
    #[serde(rename = "totalSizes")]
    pub total_sizes: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct InverterLatestData {
    pub logs: Option<Vec<InverterLogItem>>,
    pub state: String,
    pub daily_energy: Option<f64>,
    pub total_energy: Option<f64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct InverterDataBody {
    pub timestamp: String,
    pub asset_id: String,
    pub asset_type: String,
    pub map_id: String,
    pub map_type: String,
    pub edge_id: String,
    pub plant_id: String,
    pub data: InverterLatestData,
    pub model: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LatestDeviceBodyMetricsStruct {
    pub i_out: f64,
    pub v_in: f64,
    pub v_out: f64,
    pub temp: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LatestDeviceBody {
    pub timestamp: String,
    pub asset_id: String,
    pub asset_type: String,
    pub map_id: String,
    pub map_type: String,
    pub plant_id: String,
    pub edge_id: String,
    pub metrics: LatestDeviceBodyMetricsStruct,
    pub state: HashMap<String, bool>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RegistryOutputBody {
    pub asset_id: String,
    pub asset_type: String,
    pub map_id: String,
    pub map_type: String,
    pub registered: String,
    pub tag: Value,
    pub unregistered: String,
}

#[derive(Serialize, Debug)]
pub struct RegisterBody {
    pub asset_id: String,
    pub asset_type: String,
    pub map_id: String,
    pub map_type: String,
    pub registered: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered_meta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct UnregisterBody {
    pub asset_id: String,
    pub asset_type: String,
    pub map_id: String,
    pub map_type: String,
    pub unregistered: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unregistered_meta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlantData {
    pub date: String,
    pub energy: f64,
    pub cumulative_energy: f64,
    pub timestamp: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlantDailyData {
    pub energy: f64,
    pub date: String,
    pub id: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PanelData {
    pub id: String,
    pub date: String,
    pub timestamp: i64,
    pub energy: f64,
    pub cumulative_energy: f64,
    pub i_out: f64,
    pub p: f64,
    pub v_in: f64,
    pub v_out: f64,
    pub temp: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PanelDailyData {
    pub id: String,
    pub energy: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct InverterData {
    pub id: String,
    pub time: String,
    pub energy: f64,
    pub timestamp: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct InverterDailyData {
    pub id: String,
    pub date: String,
    pub energy: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BodyPlantData {
    pub plant_id: String,
    pub unit: String,
    pub source: String,
    pub date: String,
    pub interval: String,
    pub data: Option<Vec<PlantData>>,
    pub before: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BodyPlantDailyData {
    pub plant_id: String,
    pub unit: String,
    pub source: String,
    pub date: String,
    pub interval: String,
    pub data: Option<Vec<PlantDailyData>>,
    pub before: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BodyPanelData {
    pub plant_id: String,
    pub unit: String,
    pub source: String,
    pub date: String,
    pub interval: String,
    pub data: Option<Vec<PanelData>>,
    pub before: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BodyPanelDailyData {
    pub plant_id: String,
    pub unit: String,
    pub source: String,
    pub date: String,
    pub interval: String,
    pub data: Option<Vec<PanelDailyData>>,
    pub before: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BodyInverterData {
    pub plant_id: String,
    pub unit: String,
    pub source: String,
    pub date: String,
    pub interval: String,
    pub data: Option<Vec<InverterData>>,
    pub before: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BodyInverterDailyData {
    pub plant_id: String,
    pub unit: String,
    pub source: String,
    pub date: String,
    pub interval: String,
    pub data: Option<Vec<InverterDailyData>>,
    pub before: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum MetricsBody {
    PanelIntraday(BodyPanelData),
    PanelDaily(BodyPanelDailyData),
    InverterIntraday(BodyInverterData),
    InverterDaily(BodyInverterDailyData),
    PlantIntraday(BodyPlantData),
    PlantAggregated(BodyPlantDailyData),
    Unknown(Value),
}

impl<'de> Deserialize<'de> for MetricsBody {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let unit = value.get("unit").and_then(Value::as_str);
        let interval = value.get("interval").and_then(Value::as_str);

        match (unit, interval) {
            (Some("panel"), Some("5m")) => serde_json::from_value(value)
                .map(MetricsBody::PanelIntraday)
                .map_err(de::Error::custom),
            (Some("panel"), Some("day")) => serde_json::from_value(value)
                .map(MetricsBody::PanelDaily)
                .map_err(de::Error::custom),
            (Some("inverter"), Some("5m")) => serde_json::from_value(value)
                .map(MetricsBody::InverterIntraday)
                .map_err(de::Error::custom),
            (Some("inverter"), Some("day")) => serde_json::from_value(value)
                .map(MetricsBody::InverterDaily)
                .map_err(de::Error::custom),
            (Some("plant"), Some("5m")) => serde_json::from_value(value)
                .map(MetricsBody::PlantIntraday)
                .map_err(de::Error::custom),
            (Some("plant"), Some("day")) => serde_json::from_value(value)
                .map(MetricsBody::PlantAggregated)
                .map_err(de::Error::custom),
            _ => Ok(MetricsBody::Unknown(value)),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ErrorDetail {
    pub location: Option<String>,
    pub message: Option<String>,
    pub value: Option<Value>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ErrorModel {
    pub title: Option<String>,
    pub status: Option<i64>,
    pub detail: Option<String>,
    pub errors: Option<Vec<ErrorDetail>>,
    pub instance: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PanelIntradayMetrics {
    pub data: Vec<PanelData>,
    pub plant_id: String,
    pub date: String,
}
