use serde::de::{self, Deserializer};
use serde::ser::{self, SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

fn deserialize_present_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

fn deserialize_optional_non_null<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)?
        .map(Some)
        .ok_or_else(|| de::Error::custom("null is not allowed for this field"))
}

fn validate_with_message(value: &str, validator: fn(&str) -> bool, expected: &str) -> Result<(), String> {
    if validator(value) {
        Ok(())
    } else {
        Err(format!("invalid value `{value}`, expected {expected}"))
    }
}

fn is_account_type(value: &str) -> bool {
    matches!(value, "manager" | "viewer" | "admin")
}

fn is_member_account_type(value: &str) -> bool {
    matches!(value, "manager" | "viewer")
}

fn is_exact_len_15(value: &str) -> bool {
    value.len() == 15
}

fn is_alnum_len_15(value: &str) -> bool {
    is_exact_len_15(value) && value.chars().all(|c| c.is_ascii_alphanumeric())
}

fn is_registry_asset_type(value: &str) -> bool {
    matches!(value, "device" | "inverter" | "edge")
}

fn is_registry_map_type(value: &str) -> bool {
    matches!(value, "device" | "string" | "edge" | "inverter" | "combiner" | "panel")
}

fn serialize_validated_string<S>(
    value: &str,
    serializer: S,
    validator: fn(&str) -> bool,
    expected: &str,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    validate_with_message(value, validator, expected).map_err(ser::Error::custom)?;
    serializer.serialize_str(value)
}

fn deserialize_validated_string<'de, D>(
    deserializer: D,
    validator: fn(&str) -> bool,
    expected: &str,
) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    validate_with_message(&value, validator, expected).map_err(de::Error::custom)?;
    Ok(value)
}

fn deserialize_account_type<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_validated_string(deserializer, is_account_type, "manager, viewer, or admin")
}

fn deserialize_member_account_type<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_validated_string(deserializer, is_member_account_type, "manager or viewer")
}

fn serialize_alnum_len_15<S>(value: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serialize_validated_string(
        value,
        serializer,
        is_alnum_len_15,
        "exactly 15 ASCII alphanumeric characters",
    )
}

fn deserialize_exact_len_15<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_validated_string(deserializer, is_exact_len_15, "exactly 15 characters")
}

fn deserialize_registry_asset_type<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_validated_string(deserializer, is_registry_asset_type, "device, inverter, or edge")
}

fn deserialize_registry_map_type<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_validated_string(
        deserializer,
        is_registry_map_type,
        "device, string, edge, inverter, combiner, or panel",
    )
}

fn validate_role_identity(
    account_type: &str,
    email: &Option<String>,
    username: &Option<String>,
    context: &str,
) -> Result<(), String> {
    validate_with_message(account_type, is_member_account_type, "manager or viewer")?;
    match account_type {
        "manager" => {
            if email.is_none() || username.is_some() {
                Err(format!("{context} requires email for manager and must not include username"))
            } else {
                Ok(())
            }
        }
        "viewer" => {
            if username.is_none() || email.is_some() {
                Err(format!("{context} requires username for viewer and must not include email"))
            } else {
                Ok(())
            }
        }
        _ => unreachable!(),
    }
}

pub struct AuthWithPasswordBody {
    pub account_type: String,
    pub password: String,
    pub email: Option<String>,
    pub username: Option<String>,
}

impl Serialize for AuthWithPasswordBody {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        validate_role_identity(&self.account_type, &self.email, &self.username, "login body")
            .map_err(ser::Error::custom)?;

        let mut state = serializer.serialize_struct("AuthWithPasswordBody", 4)?;
        state.serialize_field("type", &self.account_type)?;
        state.serialize_field("password", &self.password)?;
        if let Some(email) = &self.email {
            state.serialize_field("email", email)?;
        }
        if let Some(username) = &self.username {
            state.serialize_field("username", username)?;
        }
        state.end()
    }
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

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct OrgInfo {
    pub id: String,
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub icon: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub logo: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub owner: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub updated: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct OrganizationBody {
    pub id: String,
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub icon: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub logo: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct AuthOutputV3Body {
    #[serde(rename = "$schema", default, deserialize_with = "deserialize_optional_non_null")]
    pub schema: Option<String>,
    pub token: String,
    #[serde(rename = "type")]
    #[serde(deserialize_with = "deserialize_account_type")]
    pub account_type: String,
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub email: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub username: Option<String>,
    #[serde(deserialize_with = "deserialize_present_option")]
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
#[serde(deny_unknown_fields)]
pub struct AuthBody {
    #[serde(rename = "$schema", default, deserialize_with = "deserialize_optional_non_null")]
    pub schema: Option<String>,
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
#[serde(deny_unknown_fields)]
pub struct AuthProvider {
    pub name: String,
    pub state: String,
    #[serde(rename = "codeChallenge")]
    pub code_challenge: String,
    #[serde(rename = "codeChallengeMethod")]
    pub code_challenge_method: String,
    #[serde(rename = "authUrl")]
    pub auth_url: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct AuthMethodsBody {
    #[serde(rename = "$schema", default, deserialize_with = "deserialize_optional_non_null")]
    pub schema: Option<String>,
    #[serde(rename = "authProviders")]
    #[serde(deserialize_with = "deserialize_present_option")]
    pub auth_providers: Option<Vec<AuthProvider>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct AccountOutputBody {
    #[serde(rename = "$schema", default, deserialize_with = "deserialize_optional_non_null")]
    pub schema: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    #[serde(deserialize_with = "deserialize_account_type")]
    pub account_type: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub email: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub username: Option<String>,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub organizations: Option<Vec<OrganizationBody>>,
    pub metadata: Option<Value>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct CreateAccountOutputBody {
    #[serde(rename = "$schema", default, deserialize_with = "deserialize_optional_non_null")]
    pub schema: Option<String>,
    pub id: String,
    #[serde(rename = "type")]
    #[serde(deserialize_with = "deserialize_account_type")]
    pub account_type: String,
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub email: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub username: Option<String>,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub organizations: Option<Vec<OrganizationBody>>,
    pub metadata: Option<Value>,
}

#[derive(Debug)]
pub struct CreateOrgMemberRequest {
    pub account_type: String,
    pub name: String,
    pub email: Option<String>,
    pub username: Option<String>,
    pub metadata: Option<Value>,
}

impl Serialize for CreateOrgMemberRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        validate_role_identity(
            &self.account_type,
            &self.email,
            &self.username,
            "organization member request",
        )
        .map_err(ser::Error::custom)?;

        let mut state = serializer.serialize_struct("CreateOrgMemberRequest", 5)?;
        state.serialize_field("type", &self.account_type)?;
        state.serialize_field("name", &self.name)?;
        if let Some(email) = &self.email {
            state.serialize_field("email", email)?;
        }
        if let Some(username) = &self.username {
            state.serialize_field("username", username)?;
        }
        if let Some(metadata) = &self.metadata {
            state.serialize_field("metadata", metadata)?;
        }
        state.end()
    }
}

#[derive(Debug)]
pub struct OrgAddPermissionInputBody {
    pub plant_id: String,
    pub account_type: String,
    pub email: Option<String>,
    pub username: Option<String>,
}

impl Serialize for OrgAddPermissionInputBody {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        validate_with_message(&self.plant_id, is_exact_len_15, "exactly 15 characters")
            .map_err(ser::Error::custom)?;
        validate_role_identity(
            &self.account_type,
            &self.email,
            &self.username,
            "organization permission request",
        )
        .map_err(ser::Error::custom)?;

        let mut state = serializer.serialize_struct("OrgAddPermissionInputBody", 4)?;
        state.serialize_field("plantId", &self.plant_id)?;
        state.serialize_field("type", &self.account_type)?;
        if let Some(email) = &self.email {
            state.serialize_field("email", email)?;
        }
        if let Some(username) = &self.username {
            state.serialize_field("username", username)?;
        }
        state.end()
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct OrgAddPermissionOutputBody {
    #[serde(rename = "$schema", default, deserialize_with = "deserialize_optional_non_null")]
    pub schema: Option<String>,
    #[serde(rename = "plant_id")]
    pub plant_id: String,
    #[serde(rename = "type")]
    #[serde(deserialize_with = "deserialize_member_account_type")]
    pub account_type: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub email: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub username: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct CreatePlantInput {
    pub name: String,
    #[serde(rename = "organizationId")]
    #[serde(serialize_with = "serialize_alnum_len_15")]
    pub organization_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PlantBody {
    #[serde(rename = "$schema", default, deserialize_with = "deserialize_optional_non_null")]
    pub schema: Option<String>,
    #[serde(deserialize_with = "deserialize_exact_len_15")]
    pub id: String,
    pub name: String,
    pub organization: String,
    #[serde(rename = "organizationData")]
    pub organization_data: OrgInfo,
    #[serde(rename = "refPlant", default, deserialize_with = "deserialize_optional_non_null")]
    pub ref_plant: Option<String>,
    pub created: String,
    pub updated: String,
    pub metadata: HashMap<String, Value>,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub images: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PlantBodyV3 {
    #[serde(rename = "$schema", default, deserialize_with = "deserialize_optional_non_null")]
    pub schema: Option<String>,
    #[serde(deserialize_with = "deserialize_exact_len_15")]
    pub id: String,
    pub name: String,
    pub organization: OrgInfo,
    #[serde(rename = "refPlant", default, deserialize_with = "deserialize_optional_non_null")]
    pub ref_plant: Option<String>,
    pub created: String,
    pub updated: String,
    pub metadata: HashMap<String, Value>,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub images: Option<Vec<String>>,
}

impl From<PlantBody> for PlantBodyV3 {
    fn from(value: PlantBody) -> Self {
        Self {
            id: value.id,
            name: value.name,
            organization: value.organization_data,
            ref_plant: value.ref_plant,
            schema: value.schema,
            created: value.created,
            updated: value.updated,
            metadata: value.metadata,
            images: value.images,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PlantsListV3OutputBody {
    #[serde(rename = "$schema", default, deserialize_with = "deserialize_optional_non_null")]
    pub schema: Option<String>,
    #[serde(deserialize_with = "deserialize_present_option")]
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
#[serde(deny_unknown_fields)]
pub struct ListOutputModuleItemBody {
    #[serde(rename = "$schema", default, deserialize_with = "deserialize_optional_non_null")]
    pub schema: Option<String>,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub items: Option<Vec<ModuleItem>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ListOutputInverterItemBody {
    #[serde(rename = "$schema", default, deserialize_with = "deserialize_optional_non_null")]
    pub schema: Option<String>,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub items: Option<Vec<InverterItem>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ListOutputCombinerItemBody {
    #[serde(rename = "$schema", default, deserialize_with = "deserialize_optional_non_null")]
    pub schema: Option<String>,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub items: Option<Vec<CombinerItem>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ModuleItem {
    pub id: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub cancellation_date: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub cell_specification: Option<HashMap<String, Value>>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub certification_date: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub created: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub equipment_code: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub imax_a: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub importer: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub importer_address: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub importer_fax_number: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub importer_phone_number: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub inspection_agency: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub isc_a: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub length_mm: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub manufacturer: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub manufacturer_address: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub manufacturing_country: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub model_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub pmax_w: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub rated_efficiency: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub technical_standard: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub thickness_mm: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub updated: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub vmax_v: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub voc_v: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub vsm_v: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub weight_kg: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub width_mm: Option<f64>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct InverterItem {
    pub id: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub cancellation_date: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub certification_date: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub certification_target_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub cooling_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub created: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub depth_mm: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub efficiency_percent: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub equipment_code: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub frequency_hz: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub height_fuse_mm: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub height_mm: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub importer: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub importer_address: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub importer_fax_number: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub importer_phone_number: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub input_voltage_max_v: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub input_voltage_min_v: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub inspection_agency: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub installation_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub insulation_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub manufacturer: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub manufacturer_address: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub manufacturing_country: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub model_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub mppt_voltage_max_v: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub mppt_voltage_min_v: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub mppt_working_max_v: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub mppt_working_min_v: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub operation_status: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub rated_capacity_w: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub rated_output_voltage_v: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub specification: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub technical_standard: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub updated: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub weight_kg: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub width_mm: Option<f64>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct CombinerItem {
    pub id: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub cancellation_date: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub category: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub certification_date: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub certification_target_type: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub created: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub depth_mm: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub equipment_code: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub has_diode: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub height_fuse_mm: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub height_mm: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub importer: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub importer_address: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub importer_fax_number: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub importer_phone_number: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub inspection_agency: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub install_position: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub ip_rating: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub manufacturer: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub manufacturer_address: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub manufacturing_country: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub max_current_a: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub max_current_per_string_a: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub max_input_voltage_v: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub max_voltage_v: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub model_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub open_circuit_voltage_v: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub operation_status: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub rated_current_a: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub rated_output_power_kva: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub string_count: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub technical_standard: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub updated: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub weight_kg: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub width_mm: Option<f64>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct HealthLevelCategory {
    pub count: i64,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub ids: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct HealthLevelBody {
    pub best: HealthLevelCategory,
    pub caution: HealthLevelCategory,
    pub faulty: HealthLevelCategory,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct InverterLogMessage {
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub ko: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct InverterLogRawElement {
    pub status: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub code: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub lcd: Option<String>,
    pub value: Option<Value>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct InverterLogsResponse {
    #[serde(rename = "$schema", default, deserialize_with = "deserialize_optional_non_null")]
    pub schema: Option<String>,
    #[serde(deserialize_with = "deserialize_present_option")]
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
#[serde(deny_unknown_fields)]
pub struct InverterLatestData {
    #[serde(deserialize_with = "deserialize_present_option")]
    pub logs: Option<Vec<InverterLogItem>>,
    pub state: String,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub daily_energy: Option<f64>,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub total_energy: Option<f64>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct LatestDeviceBodyMetricsStruct {
    pub i_out: f64,
    pub v_in: f64,
    pub v_out: f64,
    pub temp: f64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct RegistryOutputBody {
    pub asset_id: String,
    pub asset_model: HashMap<String, Value>,
    #[serde(deserialize_with = "deserialize_registry_asset_type")]
    pub asset_type: String,
    pub map_id: String,
    #[serde(deserialize_with = "deserialize_registry_map_type")]
    pub map_type: String,
    pub registered: String,
    pub tag: HashMap<String, Value>,
    pub unregistered: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PlantData {
    pub date: String,
    pub energy: f64,
    pub cumulative_energy: f64,
    pub timestamp: i64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PlantDailyData {
    pub energy: f64,
    pub date: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub id: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct PanelDailyData {
    pub id: String,
    pub energy: f64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct InverterData {
    pub id: String,
    pub time: String,
    pub energy: f64,
    pub timestamp: f64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct InverterDailyData {
    pub id: String,
    pub date: String,
    pub energy: f64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BodyPlantData {
    pub plant_id: String,
    pub unit: String,
    pub source: String,
    pub date: String,
    pub interval: String,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub data: Option<Vec<PlantData>>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub before: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BodyPlantDailyData {
    pub plant_id: String,
    pub unit: String,
    pub source: String,
    pub date: String,
    pub interval: String,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub data: Option<Vec<PlantDailyData>>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub before: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BodyPanelData {
    pub plant_id: String,
    pub unit: String,
    pub source: String,
    pub date: String,
    pub interval: String,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub data: Option<Vec<PanelData>>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub before: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BodyPanelDailyData {
    pub plant_id: String,
    pub unit: String,
    pub source: String,
    pub date: String,
    pub interval: String,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub data: Option<Vec<PanelDailyData>>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub before: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BodyInverterData {
    pub plant_id: String,
    pub unit: String,
    pub source: String,
    pub date: String,
    pub interval: String,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub data: Option<Vec<InverterData>>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub before: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BodyInverterDailyData {
    pub plant_id: String,
    pub unit: String,
    pub source: String,
    pub date: String,
    pub interval: String,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub data: Option<Vec<InverterDailyData>>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub before: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SensorData {
    pub id: String,
    pub date: String,
    pub timestamp: i64,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub min: Option<f64>,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub max: Option<f64>,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub mean: Option<f64>,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub median: Option<f64>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BodySensorData {
    pub plant_id: String,
    pub unit: String,
    pub source: String,
    pub date: String,
    pub interval: String,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub data: Option<Vec<SensorData>>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub before: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct StatModelCount {
    pub name: String,
    pub count: i64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct DeviceModelStat {
    pub name: String,
    pub count: i64,
    pub installed_capacity_w: f64,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct StatPoint {
    #[serde(rename = "$schema", default, deserialize_with = "deserialize_optional_non_null")]
    pub schema: Option<String>,
    pub timestamp: String,
    pub installed_capacity_w: f64,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub module_models: Option<Vec<StatModelCount>>,
    #[serde(deserialize_with = "deserialize_present_option")]
    pub device_models: Option<Vec<DeviceModelStat>>,
}

#[derive(Debug, Clone)]
pub enum MetricsBody {
    PanelIntraday(BodyPanelData),
    PanelDaily(BodyPanelDailyData),
    InverterIntraday(BodyInverterData),
    InverterDaily(BodyInverterDailyData),
    PlantIntraday(BodyPlantData),
    PlantAggregated(BodyPlantDailyData),
    SensorIntraday(BodySensorData),
    Unknown(Value),
}

impl<'de> Deserialize<'de> for MetricsBody {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let source = value.get("source").and_then(Value::as_str);
        let unit = value.get("unit").and_then(Value::as_str);
        let interval = value.get("interval").and_then(Value::as_str);

        match (source, unit, interval) {
            (Some("device"), Some("panel"), Some("5m" | "15m" | "1h")) => serde_json::from_value(value)
                .map(MetricsBody::PanelIntraday)
                .map_err(de::Error::custom),
            (Some("device"), Some("panel"), Some("1d")) => serde_json::from_value(value)
                .map(MetricsBody::PanelDaily)
                .map_err(de::Error::custom),
            (Some("device" | "inverter"), Some("inverter"), Some("5m" | "15m" | "1h")) => serde_json::from_value(value)
                .map(MetricsBody::InverterIntraday)
                .map_err(de::Error::custom),
            (Some("device" | "inverter"), Some("inverter"), Some("1d")) => serde_json::from_value(value)
                .map(MetricsBody::InverterDaily)
                .map_err(de::Error::custom),
            (Some("device" | "inverter"), Some("plant"), Some("5m" | "15m" | "1h")) => serde_json::from_value(value)
                .map(MetricsBody::PlantIntraday)
                .map_err(de::Error::custom),
            (Some("device" | "inverter"), Some("plant"), Some("1d" | "1M" | "1y")) => serde_json::from_value(value)
                .map(MetricsBody::PlantAggregated)
                .map_err(de::Error::custom),
            (Some("sensor"), Some("temperature" | "insolation"), Some("5m")) => {
                serde_json::from_value(value)
                    .map(MetricsBody::SensorIntraday)
                    .map_err(de::Error::custom)
            }
            _ => Ok(MetricsBody::Unknown(value)),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ErrorDetail {
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub location: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub message: Option<String>,
    pub value: Option<Value>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ErrorModel {
    #[serde(rename = "$schema", default, deserialize_with = "deserialize_optional_non_null")]
    pub schema: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub status: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub detail: Option<String>,
    pub errors: Option<Vec<ErrorDetail>>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub instance: Option<String>,
    #[serde(rename = "type")]
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub error_type: Option<String>,
}
