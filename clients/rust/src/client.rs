use crate::error::{Error, Result};
use crate::model::{
    AccountOutputBody, AuthAccountBody, AuthBody, AuthEmailBody, AuthOutputV3Body,
    AuthWithPasswordBody, CreateAccountOutputBody, CreateOrgMemberRequest, CreatePlantInput,
    ErrorModel, FileUploadResponse, HealthLevelBody, InverterDataBody, InverterLogsResponse,
    LatestDeviceBody, MetricsBody, OrgAddPermissionInputBody, OrgAddPermissionOutputBody,
    PanelIntradayMetrics, PlantBody, PlantBodyV3, PlantsListV3OutputBody, RegistryOutputBody,
};
use percent_encoding::{percent_decode_str, percent_encode_byte};
use reqwest::multipart::{Form, Part};
use reqwest::{Client as HttpClient, Method, StatusCode};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use url::Url;

const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_MAX_RESPONSE_BYTES: usize = 10 << 20;

#[derive(Clone)]
struct AuthState {
    token: String,
    account_type: String,
}

impl std::fmt::Debug for AuthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthState")
            .field("token", &"<redacted>")
            .field("account_type", &self.account_type)
            .finish()
    }
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum CreatePlantV3Response {
    V3(PlantBodyV3),
    Legacy(PlantBody),
}

impl CreatePlantV3Response {
    fn into_v3(self) -> PlantBodyV3 {
        match self {
            Self::V3(v3) => v3,
            Self::Legacy(v2) => v2.into(),
        }
    }
}

#[derive(Clone)]
pub struct Client {
    base_url: Url,
    http: HttpClient,
    auth: Arc<RwLock<Option<AuthState>>>,
}

impl Client {
    fn encode_path_segment(segment: &str) -> String {
        let mut out = String::with_capacity(segment.len());
        for &b in segment.as_bytes() {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'~') {
                out.push(b as char);
            } else {
                out.push_str(percent_encode_byte(b));
            }
        }
        out
    }

    pub fn new(base_url: &str) -> Result<Self> {
        // Keep default timeout aligned with Go/Python clients.
        Self::new_with_timeout(base_url, DEFAULT_HTTP_TIMEOUT)
    }

    pub fn new_with_timeout(base_url: &str, timeout: Duration) -> Result<Self> {
        let mut base_url = Url::parse(base_url)?;
        Self::validate_base_url(&base_url)?;
        Self::normalize_base_url(&mut base_url);
        let http = HttpClient::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self {
            base_url,
            http,
            auth: Arc::new(RwLock::new(None)),
        })
    }

    fn validate_base_url(base_url: &Url) -> Result<()> {
        if base_url.query().is_some() || base_url.fragment().is_some() {
            return Err(Error::InvalidPath(
                "base_url must not include query or fragment".to_string(),
            ));
        }
        let host = base_url.host_str().unwrap_or_default();
        if host.is_empty() {
            return Err(Error::InvalidPath("base_url must include host".to_string()));
        }
        match base_url.scheme() {
            "https" => Ok(()),
            "http" if Self::is_loopback_host(base_url.host_str()) => Ok(()),
            _ => Err(Error::InsecureBaseUrl(base_url.to_string())),
        }
    }

    fn is_loopback_host(host: Option<&str>) -> bool {
        match host {
            Some(h) if h.eq_ignore_ascii_case("localhost") => true,
            Some(h) => h
                .parse::<IpAddr>()
                .map(|ip| ip.is_loopback())
                .unwrap_or(false),
            None => false,
        }
    }

    fn normalize_base_url(base_url: &mut Url) {
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
    }

    fn url(&self, path: &str) -> Result<Url> {
        let path = path.trim_start_matches('/');
        if path.contains("://") {
            return Err(Error::InvalidPath(path.to_string()));
        }
        if path.split('/').any(|segment| {
            let decoded = percent_decode_str(segment).decode_utf8_lossy();
            matches!(decoded.as_ref(), "." | "..")
                || decoded.contains('/')
                || decoded.contains('\\')
        }) {
            return Err(Error::InvalidPath(path.to_string()));
        }
        Ok(self.base_url.join(path)?)
    }

    pub async fn login(&self, account: &str, password: &str) -> Result<AuthOutputV3Body> {
        let body = if account.contains('@') {
            AuthWithPasswordBody {
                account_type: "manager".to_string(),
                email: Some(account.to_string()),
                username: None,
                password: password.to_string(),
            }
        } else {
            AuthWithPasswordBody {
                account_type: "viewer".to_string(),
                email: None,
                username: Some(account.to_string()),
                password: password.to_string(),
            }
        };

        let auth: AuthOutputV3Body = match self
            .execute_json_unauth_no_refresh(
                Method::POST,
                self.url("api/v3/account/auth-with-password")?,
                Some(&body),
            )
            .await
        {
            Ok(v) => v,
            Err(err) => {
                self.clear_auth_on_login_failure(&err).await;
                return Err(err);
            }
        };
        let mut lock = self.auth.write().await;
        *lock = Some(AuthState {
            token: auth.token.clone(),
            account_type: auth.account_type.clone(),
        });
        Ok(auth)
    }

    pub async fn login_v2_manager(&self, email: &str, password: Option<&str>) -> Result<AuthBody> {
        let body = AuthEmailBody {
            email: email.to_string(),
            password: password.map(|s| s.to_string()),
        };
        let auth: AuthBody = match self
            .execute_json_unauth_no_refresh(
                Method::POST,
                self.url("api/v2/manager/auth-with-password")?,
                Some(&body),
            )
            .await
        {
            Ok(v) => v,
            Err(err) => {
                self.clear_auth_on_login_failure(&err).await;
                return Err(err);
            }
        };
        let mut lock = self.auth.write().await;
        *lock = Some(AuthState {
            token: auth.token.clone(),
            account_type: "manager".to_string(),
        });
        Ok(auth)
    }

    pub async fn login_v2_viewer(&self, account: &str, password: Option<&str>) -> Result<AuthBody> {
        let body = AuthAccountBody {
            account: account.to_string(),
            password: password.map(|s| s.to_string()),
        };
        let auth: AuthBody = match self
            .execute_json_unauth_no_refresh(
                Method::POST,
                self.url("api/v2/viewer/auth-with-password")?,
                Some(&body),
            )
            .await
        {
            Ok(v) => v,
            Err(err) => {
                self.clear_auth_on_login_failure(&err).await;
                return Err(err);
            }
        };
        let mut lock = self.auth.write().await;
        *lock = Some(AuthState {
            token: auth.token.clone(),
            account_type: "viewer".to_string(),
        });
        Ok(auth)
    }

    async fn clear_auth_on_login_failure(&self, err: &Error) {
        let should_clear = match err {
            Error::Api { status, .. } | Error::ApiProblem { status, .. } => {
                *status == 401 || *status == 403
            }
            Error::Unauthorized => true,
            _ => false,
        };
        if should_clear {
            let mut lock = self.auth.write().await;
            *lock = None;
        }
    }

    pub async fn refresh_token(&self) -> Result<()> {
        let (token, account_type) = {
            let lock = self.auth.read().await;
            if let Some(auth) = &*lock {
                (auth.token.clone(), auth.account_type.clone())
            } else {
                return Err(Error::Unauthorized);
            }
        };

        let url = self.url("api/v3/account/refresh-token")?;
        let res = self
            .http
            .post(url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Account-Type", account_type)
            .send()
            .await?;

        if res.status().is_success() {
            let bytes = Self::read_body_limited(res).await?;
            let new_auth: AuthBody = serde_json::from_slice(&bytes)?;
            let mut lock = self.auth.write().await;
            if let Some(auth) = &mut *lock {
                auth.token = new_auth.token;
            }
            Ok(())
        } else {
            let status = res.status();
            if matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
                return Err(Error::Unauthorized);
            }
            let body = String::from_utf8_lossy(&Self::read_body_limited(res).await?).into_owned();
            Err(Self::api_error(status, body))
        }
    }

    fn api_error(status: StatusCode, body: String) -> Error {
        if let Ok(problem) = serde_json::from_str::<ErrorModel>(&body) {
            return Error::ApiProblem {
                status: status.as_u16(),
                title: problem
                    .title
                    .clone()
                    .unwrap_or_else(|| "API Error".to_string()),
                detail: problem.detail.clone(),
                error: Box::new(problem),
            };
        }
        Error::Api {
            status: status.as_u16(),
            message: "upstream error body omitted".to_string(),
        }
    }

    async fn execute_json<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        method: Method,
        url: Url,
        body: Option<&B>,
    ) -> Result<T> {
        self.execute_json_internal(method, url, body, true, true)
            .await
    }

    async fn execute_json_unauth_no_refresh<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        method: Method,
        url: Url,
        body: Option<&B>,
    ) -> Result<T> {
        self.execute_json_internal(method, url, body, false, false)
            .await
    }

    async fn execute_json_internal<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        method: Method,
        url: Url,
        body: Option<&B>,
        allow_refresh_on_401: bool,
        include_auth: bool,
    ) -> Result<T> {
        let mut retries = 1;
        loop {
            let mut req = self.http.request(method.clone(), url.clone());

            let (auth, authed) = if include_auth {
                let lock = self.auth.read().await;
                ((*lock).clone(), lock.is_some())
            } else {
                (None, false)
            };
            if let Some(auth) = auth {
                req = req
                    .header("Authorization", format!("Bearer {}", auth.token))
                    .header("Account-Type", &auth.account_type);
            }

            if let Some(b) = body {
                req = req.json(b);
            }

            let res = req.send().await?;
            let status = res.status();

            if status == StatusCode::UNAUTHORIZED && retries > 0 && authed && allow_refresh_on_401 {
                retries -= 1;
                self.refresh_token().await?;
                continue;
            }

            let body_bytes = Self::read_body_limited(res).await?;
            if status.is_success() {
                return Ok(serde_json::from_slice::<T>(&body_bytes)?);
            }

            let body = String::from_utf8_lossy(&body_bytes).into_owned();
            return Err(Self::api_error(status, body));
        }
    }

    async fn execute_text(
        &self,
        method: Method,
        url: Url,
        decode_json_string: bool,
    ) -> Result<String> {
        let mut retries = 1;
        loop {
            let mut req = self.http.request(method.clone(), url.clone());

            let (auth, authed) = {
                let lock = self.auth.read().await;
                ((*lock).clone(), lock.is_some())
            };
            if let Some(auth) = auth {
                req = req
                    .header("Authorization", format!("Bearer {}", auth.token))
                    .header("Account-Type", &auth.account_type);
            }

            let res = req.send().await?;
            let status = res.status();
            if status == StatusCode::UNAUTHORIZED && retries > 0 && authed {
                retries -= 1;
                self.refresh_token().await?;
                continue;
            }
            let content_type = res
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_ascii_lowercase();
            let body = Self::read_body_limited(res).await?;
            if status.is_success() {
                if decode_json_string
                    && (content_type.contains("application/json") || content_type.contains("+json"))
                {
                    if let Ok(text) = serde_json::from_slice::<String>(&body) {
                        return Ok(text);
                    }
                }
                return Ok(String::from_utf8_lossy(&body).into_owned());
            }
            let body_str = String::from_utf8_lossy(&body).into_owned();
            return Err(Self::api_error(status, body_str));
        }
    }

    async fn read_body_limited(mut res: reqwest::Response) -> Result<Vec<u8>> {
        let mut body = Vec::new();
        while let Some(chunk) = res.chunk().await? {
            if body.len() + chunk.len() > DEFAULT_MAX_RESPONSE_BYTES {
                return Err(Error::ResponseTooLarge(DEFAULT_MAX_RESPONSE_BYTES));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }

    fn url_with_query(&self, path: &str, query: &[(&str, String)]) -> Result<Url> {
        let mut url = self.url(path)?;
        {
            let mut qp = url.query_pairs_mut();
            for (k, v) in query {
                qp.append_pair(k, v);
            }
        }
        Ok(url)
    }

    fn push_fields_csv_query(q: &mut Vec<(&str, String)>, fields: Option<&[String]>) {
        if let Some(fs) = fields {
            if !fs.is_empty() {
                q.push(("fields", fs.join(",")));
            }
        }
    }

    pub async fn get_account(&self) -> Result<AccountOutputBody> {
        self.execute_json(
            Method::GET,
            self.url("api/v3/account/")?,
            Option::<&()>::None,
        )
        .await
    }

    pub async fn list_plants(&self) -> Result<PlantsListV3OutputBody> {
        self.execute_json(Method::GET, self.url("api/v3/plants")?, Option::<&()>::None)
            .await
    }

    pub async fn list_plants_v3(
        &self,
        page: Option<u32>,
        size: Option<u32>,
    ) -> Result<PlantsListV3OutputBody> {
        let mut q = Vec::new();
        if let Some(v) = page {
            q.push(("page", v.to_string()));
        }
        if let Some(v) = size {
            q.push(("size", v.to_string()));
        }
        let url = self.url_with_query("api/v3/plants", &q)?;
        self.execute_json(Method::GET, url, Option::<&()>::None)
            .await
    }

    pub async fn list_plants_v2(
        &self,
        page: Option<u32>,
        size: Option<u32>,
    ) -> Result<Option<Vec<PlantBody>>> {
        let mut q = Vec::new();
        if let Some(v) = page {
            q.push(("page", v.to_string()));
        }
        if let Some(v) = size {
            q.push(("size", v.to_string()));
        }
        let url = self.url_with_query("api/v2/information/plants", &q)?;
        self.execute_json(Method::GET, url, Option::<&()>::None)
            .await
    }

    pub async fn get_plant(&self, plant_id: &str) -> Result<PlantBodyV3> {
        self.get_plant_v3(plant_id).await
    }

    pub async fn get_plant_v3(&self, plant_id: &str) -> Result<PlantBodyV3> {
        let path = format!("api/v3/plants/{}", Self::encode_path_segment(plant_id));
        self.execute_json(Method::GET, self.url(&path)?, Option::<&()>::None)
            .await
    }

    pub async fn get_plant_v2(&self, plant_id: &str) -> Result<PlantBody> {
        let path = format!(
            "api/v2/information/plants/{}",
            Self::encode_path_segment(plant_id)
        );
        self.execute_json(Method::GET, self.url(&path)?, Option::<&()>::None)
            .await
    }

    pub async fn create_plant_v3(&self, input: &CreatePlantInput) -> Result<PlantBodyV3> {
        let out: CreatePlantV3Response = self
            .execute_json(Method::POST, self.url("api/v3/plants")?, Some(input))
            .await?;
        Ok(out.into_v3())
    }

    pub async fn get_blueprint_text_v3(&self, plant_id: &str, date: &str) -> Result<String> {
        let path = format!(
            "api/v3/plants/{}/blueprint",
            Self::encode_path_segment(plant_id)
        );
        let url = self.url_with_query(&path, &[("date", date.to_string())])?;
        self.execute_text(Method::GET, url, true).await
    }

    pub async fn get_blueprint_text_v2(&self, plant_id: &str, date: &str) -> Result<String> {
        let path = format!(
            "api/v2/blueprint/plants/{}",
            Self::encode_path_segment(plant_id)
        );
        let url = self.url_with_query(&path, &[("date", date.to_string())])?;
        self.execute_text(Method::GET, url, false).await
    }

    pub async fn get_blueprint(&self, plant_id: &str, date: &str) -> Result<serde_json::Value> {
        let s = self.get_blueprint_text_v3(plant_id, date).await?;
        Ok(serde_json::Value::String(s))
    }

    pub async fn get_registry(
        &self,
        plant_id: &str,
        date: &str,
    ) -> Result<Vec<RegistryOutputBody>> {
        let res = self
            .get_registry_v3(plant_id, "snapshots", date, None, None)
            .await?;
        Ok(res.unwrap_or_default())
    }

    pub async fn get_registry_v3(
        &self,
        plant_id: &str,
        record_type: &str,
        date: &str,
        asset_id: Option<&str>,
        map_id: Option<&str>,
    ) -> Result<Option<Vec<RegistryOutputBody>>> {
        let path = format!(
            "api/v3/plants/{}/registry/{}",
            Self::encode_path_segment(plant_id),
            Self::encode_path_segment(record_type)
        );
        let mut q = vec![("date", date.to_string())];
        if let Some(v) = asset_id {
            q.push(("asset_id", v.to_string()));
        }
        if let Some(v) = map_id {
            q.push(("map_id", v.to_string()));
        }
        let url = self.url_with_query(&path, &q)?;
        self.execute_json(Method::GET, url, Option::<&()>::None)
            .await
    }

    pub async fn get_registry_v2(
        &self,
        plant_id: &str,
        record_type: &str,
        date: &str,
        asset_id: Option<&str>,
        map_id: Option<&str>,
    ) -> Result<Option<Vec<RegistryOutputBody>>> {
        let path = format!(
            "api/v2/registry/plants/{}/{}",
            Self::encode_path_segment(plant_id),
            Self::encode_path_segment(record_type)
        );
        let mut q = vec![("date", date.to_string())];
        if let Some(v) = asset_id {
            q.push(("asset_id", v.to_string()));
        }
        if let Some(v) = map_id {
            q.push(("map_id", v.to_string()));
        }
        let url = self.url_with_query(&path, &q)?;
        self.execute_json(Method::GET, url, Option::<&()>::None)
            .await
    }

    pub async fn get_panel_metrics(
        &self,
        plant_id: &str,
        date: &str,
    ) -> Result<PanelIntradayMetrics> {
        let metrics = self
            .get_metrics_by_date_v3(plant_id, "device", "panel", "5m", date, None, None)
            .await?;

        match metrics {
            MetricsBody::PanelIntraday(body) => {
                let data = body.data.ok_or_else(|| Error::Api {
                    status: 500,
                    message: "missing metrics data in panel metrics response".to_string(),
                })?;
                Ok(PanelIntradayMetrics {
                    data,
                    plant_id: body.plant_id,
                    date: body.date,
                })
            }
            _ => Err(Error::Api {
                status: 500,
                message: "unexpected metrics body variant".to_string(),
            }),
        }
    }

    pub async fn get_latest_device_metrics_v3(
        &self,
        plant_id: &str,
        include_state: Option<bool>,
        ago: Option<i64>,
    ) -> Result<Option<Vec<LatestDeviceBody>>> {
        let path = format!(
            "api/v3/plants/{}/metrics/device/latest",
            Self::encode_path_segment(plant_id)
        );
        let mut q: Vec<(&str, String)> = Vec::new();
        if let Some(v) = include_state {
            q.push(("includeState", v.to_string()));
        }
        if let Some(v) = ago {
            q.push(("ago", v.to_string()));
        }
        let url = self.url_with_query(&path, &q)?;
        self.execute_json(Method::GET, url, Option::<&()>::None)
            .await
    }

    pub async fn get_latest_inverter_metrics_v3(
        &self,
        plant_id: &str,
    ) -> Result<Option<Vec<InverterDataBody>>> {
        let path = format!(
            "api/v3/plants/{}/metrics/inverter/latest",
            Self::encode_path_segment(plant_id)
        );
        self.execute_json(Method::GET, self.url(&path)?, Option::<&()>::None)
            .await
    }

    pub async fn get_latest_device_metrics_v2(
        &self,
        plant_id: &str,
        include_state: Option<bool>,
        ago: Option<i64>,
    ) -> Result<Option<Vec<LatestDeviceBody>>> {
        let path = format!(
            "api/v2/metrics/plants/{}/device/latest",
            Self::encode_path_segment(plant_id)
        );
        let mut q: Vec<(&str, String)> = Vec::new();
        if let Some(v) = include_state {
            q.push(("includeState", v.to_string()));
        }
        if let Some(v) = ago {
            q.push(("ago", v.to_string()));
        }
        let url = self.url_with_query(&path, &q)?;
        self.execute_json(Method::GET, url, Option::<&()>::None)
            .await
    }

    pub async fn get_latest_inverter_metrics_v2(
        &self,
        plant_id: &str,
    ) -> Result<Option<Vec<InverterDataBody>>> {
        let path = format!(
            "api/v2/metrics/plants/{}/inverter/latest",
            Self::encode_path_segment(plant_id)
        );
        self.execute_json(Method::GET, self.url(&path)?, Option::<&()>::None)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn get_metrics_by_date_v3(
        &self,
        plant_id: &str,
        source: &str,
        unit: &str,
        interval: &str,
        date: &str,
        before: Option<i64>,
        fields: Option<&[String]>,
    ) -> Result<MetricsBody> {
        let path = format!(
            "api/v3/plants/{}/metrics/{}/{}-{}",
            Self::encode_path_segment(plant_id),
            Self::encode_path_segment(source),
            Self::encode_path_segment(unit),
            Self::encode_path_segment(interval)
        );

        let mut q: Vec<(&str, String)> = vec![("date", date.to_string())];
        if let Some(v) = before {
            q.push(("before", v.to_string()));
        }
        Self::push_fields_csv_query(&mut q, fields);
        let url = self.url_with_query(&path, &q)?;
        self.execute_json(Method::GET, url, Option::<&()>::None)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn get_metrics_by_date_v2(
        &self,
        plant_id: &str,
        source: &str,
        unit: &str,
        interval: &str,
        date: &str,
        before: Option<i64>,
        fields: Option<&[String]>,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v2/metrics/plants/{}/{}/{}-{}",
            Self::encode_path_segment(plant_id),
            Self::encode_path_segment(source),
            Self::encode_path_segment(unit),
            Self::encode_path_segment(interval)
        );
        let mut q: Vec<(&str, String)> = vec![("date", date.to_string())];
        if let Some(v) = before {
            q.push(("before", v.to_string()));
        }
        Self::push_fields_csv_query(&mut q, fields);
        let url = self.url_with_query(&path, &q)?;
        self.execute_json(Method::GET, url, Option::<&()>::None)
            .await
    }

    /// v2 metrics endpoint (typed).
    ///
    /// Note: OpenAPI currently omits the 200-schema for this operation.
    /// Use this helper when the server returns the same metrics payload shapes as v3.
    #[allow(clippy::too_many_arguments)]
    pub async fn get_metrics_by_date_v2_typed(
        &self,
        plant_id: &str,
        source: &str,
        unit: &str,
        interval: &str,
        date: &str,
        before: Option<i64>,
        fields: Option<&[String]>,
    ) -> Result<MetricsBody> {
        let path = format!(
            "api/v2/metrics/plants/{}/{}/{}-{}",
            Self::encode_path_segment(plant_id),
            Self::encode_path_segment(source),
            Self::encode_path_segment(unit),
            Self::encode_path_segment(interval)
        );

        let mut q: Vec<(&str, String)> = vec![("date", date.to_string())];
        if let Some(v) = before {
            q.push(("before", v.to_string()));
        }
        Self::push_fields_csv_query(&mut q, fields);
        let url = self.url_with_query(&path, &q)?;
        self.execute_json(Method::GET, url, Option::<&()>::None)
            .await
    }

    #[cfg(test)]
    fn build_metrics_path_v2(plant_id: &str, source: &str, unit: &str, interval: &str) -> String {
        format!(
            "api/v2/metrics/plants/{}/{}/{}-{}",
            Self::encode_path_segment(plant_id),
            Self::encode_path_segment(source),
            Self::encode_path_segment(unit),
            Self::encode_path_segment(interval)
        )
    }

    #[cfg(test)]
    fn build_metrics_path_v3(plant_id: &str, source: &str, unit: &str, interval: &str) -> String {
        format!(
            "api/v3/plants/{}/metrics/{}/{}-{}",
            Self::encode_path_segment(plant_id),
            Self::encode_path_segment(source),
            Self::encode_path_segment(unit),
            Self::encode_path_segment(interval)
        )
    }

    pub async fn list_inverter_logs_v3(
        &self,
        plant_id: &str,
        page: Option<u32>,
        size: Option<u32>,
    ) -> Result<InverterLogsResponse> {
        let path = format!(
            "api/v3/plants/{}/logs/inverter",
            Self::encode_path_segment(plant_id)
        );
        let mut q = Vec::new();
        if let Some(v) = page {
            q.push(("page", v.to_string()));
        }
        if let Some(v) = size {
            q.push(("size", v.to_string()));
        }
        let url = self.url_with_query(&path, &q)?;
        self.execute_json(Method::GET, url, Option::<&()>::None)
            .await
    }

    pub async fn list_inverter_logs_by_id_v3(
        &self,
        plant_id: &str,
        inverter_id: &str,
        page: Option<u32>,
        size: Option<u32>,
    ) -> Result<InverterLogsResponse> {
        let path = format!(
            "api/v3/plants/{}/logs/inverters/{}",
            Self::encode_path_segment(plant_id),
            Self::encode_path_segment(inverter_id)
        );
        let mut q = Vec::new();
        if let Some(v) = page {
            q.push(("page", v.to_string()));
        }
        if let Some(v) = size {
            q.push(("size", v.to_string()));
        }
        let url = self.url_with_query(&path, &q)?;
        self.execute_json(Method::GET, url, Option::<&()>::None)
            .await
    }

    pub async fn list_inverter_logs_v2(
        &self,
        plant_id: &str,
        page: Option<u32>,
        size: Option<u32>,
    ) -> Result<InverterLogsResponse> {
        let path = format!(
            "api/v2/logs/plants/{}/inverter",
            Self::encode_path_segment(plant_id)
        );
        let mut q = Vec::new();
        if let Some(v) = page {
            q.push(("page", v.to_string()));
        }
        if let Some(v) = size {
            q.push(("size", v.to_string()));
        }
        let url = self.url_with_query(&path, &q)?;
        self.execute_json(Method::GET, url, Option::<&()>::None)
            .await
    }

    pub async fn upload_plant_file_v3(
        &self,
        plant_id: &str,
        name: &str,
        filename: &str,
        bytes: Vec<u8>,
    ) -> Result<FileUploadResponse> {
        let path = format!(
            "api/v3/plants/{}/files",
            Self::encode_path_segment(plant_id)
        );
        let url = self.url(&path)?;

        let mut retries = 1;
        loop {
            let mut req = self.http.request(Method::POST, url.clone());

            let (auth, authed) = {
                let lock = self.auth.read().await;
                ((*lock).clone(), lock.is_some())
            };
            if let Some(auth) = auth {
                req = req
                    .header("Authorization", format!("Bearer {}", auth.token))
                    .header("Account-Type", &auth.account_type);
            }

            let form = Form::new().text("name", name.to_string()).part(
                "filename",
                Part::bytes(bytes.clone()).file_name(filename.to_string()),
            );

            let res = req.multipart(form).send().await?;
            let status = res.status();

            if status == StatusCode::UNAUTHORIZED && retries > 0 && authed {
                retries -= 1;
                self.refresh_token().await?;
                continue;
            }

            let body = Self::read_body_limited(res).await?;
            if status.is_success() {
                return Ok(serde_json::from_slice::<FileUploadResponse>(&body)?);
            }

            let body_str = String::from_utf8_lossy(&body).into_owned();
            return Err(Self::api_error(status, body_str));
        }
    }

    pub async fn get_health_level_v3(
        &self,
        plant_id: &str,
        unit: &str,
        date: &str,
        view: Option<&str>,
    ) -> Result<HealthLevelBody> {
        let path = format!(
            "api/v3/plants/{}/indicator/health-level/{}",
            Self::encode_path_segment(plant_id),
            Self::encode_path_segment(unit)
        );
        let mut q: Vec<(&str, String)> = vec![("date", date.to_string())];
        if let Some(v) = view {
            q.push(("view", v.to_string()));
        }
        let url = self.url_with_query(&path, &q)?;
        self.execute_json(Method::GET, url, Option::<&()>::None)
            .await
    }

    pub async fn get_panel_seqnum_v3(
        &self,
        plant_id: &str,
        date: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/indicator/seqnum",
            Self::encode_path_segment(plant_id)
        );
        let url = self.url_with_query(&path, &[("date", date.to_string())])?;
        self.execute_json(Method::GET, url, Option::<&()>::None)
            .await
    }

    pub async fn create_org_member_v3(
        &self,
        organization_id: &str,
        body: &CreateOrgMemberRequest,
    ) -> Result<CreateAccountOutputBody> {
        let path = format!(
            "api/v3/organizations/{}/members",
            Self::encode_path_segment(organization_id)
        );
        self.execute_json(Method::POST, self.url(&path)?, Some(body))
            .await
    }

    pub async fn assign_plant_permission_v3(
        &self,
        organization_id: &str,
        body: &OrgAddPermissionInputBody,
    ) -> Result<OrgAddPermissionOutputBody> {
        let path = format!(
            "api/v3/organizations/{}/permissions",
            Self::encode_path_segment(organization_id)
        );
        self.execute_json(Method::POST, self.url(&path)?, Some(body))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::{Duration, Instant};

    const TEST_ACCEPT_TIMEOUT: Duration = Duration::from_secs(30);

    struct MockStep {
        method: &'static str,
        path_prefix: &'static str,
        status: u16,
        content_type: &'static str,
        body: &'static str,
        stall_before_response: Option<Duration>,
    }

    struct MockServer {
        base_url: String,
        handle: thread::JoinHandle<()>,
    }

    fn reason_phrase(status: u16) -> &'static str {
        match status {
            200 => "OK",
            401 => "Unauthorized",
            403 => "Forbidden",
            500 => "Internal Server Error",
            _ => "Unknown",
        }
    }

    fn accept_with_timeout(listener: &TcpListener, timeout: Duration) -> TcpStream {
        let deadline = Instant::now() + timeout;
        loop {
            match listener.accept() {
                Ok((stream, _)) => return stream,
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        panic!("timed out waiting for test client connection");
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(err) => panic!("accept request: {err}"),
            }
        }
    }

    fn spawn_mock_server(steps: Vec<MockStep>) -> MockServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        listener
            .set_nonblocking(true)
            .expect("set nonblocking listener");
        let addr = listener.local_addr().expect("read local addr");
        let handle = thread::spawn(move || {
            for step in steps {
                let mut stream = accept_with_timeout(&listener, TEST_ACCEPT_TIMEOUT);
                stream.set_nonblocking(false).expect("set blocking stream");
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .expect("set read timeout");
                let mut req_buf = [0_u8; 8192];
                let n = stream.read(&mut req_buf).expect("read request");
                let req = String::from_utf8_lossy(&req_buf[..n]);
                let req_line = req.lines().next().unwrap_or_default();
                let expected = format!("{} {}", step.method, step.path_prefix);
                assert!(
                    req_line.starts_with(&expected),
                    "unexpected request line. expected prefix `{expected}`, got `{req_line}`"
                );

                if let Some(d) = step.stall_before_response {
                    thread::sleep(d);
                }

                let response = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    step.status,
                    reason_phrase(step.status),
                    step.content_type,
                    step.body.len(),
                    step.body
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
                stream.flush().expect("flush response");
            }
        });
        MockServer {
            base_url: format!("http://{addr}"),
            handle,
        }
    }

    fn spawn_hanging_server(hang_for: Duration) -> MockServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        listener
            .set_nonblocking(true)
            .expect("set nonblocking listener");
        let addr = listener.local_addr().expect("read local addr");
        let handle = thread::spawn(move || {
            let mut stream = accept_with_timeout(&listener, TEST_ACCEPT_TIMEOUT);
            stream.set_nonblocking(false).expect("set blocking stream");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("set read timeout");
            let mut req_buf = [0_u8; 1024];
            let _ = stream.read(&mut req_buf);
            thread::sleep(hang_for);
        });
        MockServer {
            base_url: format!("http://{addr}"),
            handle,
        }
    }

    #[test]
    fn metrics_path_uses_compound_unit_interval_segment() {
        let p2 = Client::build_metrics_path_v2("p1", "device", "panel", "5m");
        assert_eq!(p2, "api/v2/metrics/plants/p1/device/panel-5m");

        let p3 = Client::build_metrics_path_v3("p1", "device", "panel", "5m");
        assert_eq!(p3, "api/v3/plants/p1/metrics/device/panel-5m");
    }

    #[test]
    fn metrics_path_encodes_untrusted_segments() {
        let p3 = Client::build_metrics_path_v3("../admin", "device/solar", "..", "5m");
        assert_eq!(
            p3,
            "api/v3/plants/%2E%2E%2Fadmin/metrics/device%2Fsolar/%2E%2E-5m"
        );
    }

    #[test]
    fn api_error_parses_problem_json_when_possible() {
        let body = r#"{"title":"Bad Request","status":400,"detail":"invalid input"}"#;
        let err = Client::api_error(StatusCode::BAD_REQUEST, body.to_string());
        match err {
            Error::ApiProblem {
                status,
                title,
                detail,
                ..
            } => {
                assert_eq!(status, 400);
                assert_eq!(title, "Bad Request");
                assert_eq!(detail.as_deref(), Some("invalid input"));
            }
            _ => panic!("expected ApiProblem"),
        }
    }

    #[test]
    fn url_join_invalid_input_does_not_panic() {
        use std::panic::AssertUnwindSafe;

        let client = Client::new("https://example.com/").expect("valid base");
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| client.url("http://[:::1]")));
        assert!(result.is_ok(), "url join should not panic on invalid path");
    }

    #[test]
    fn new_rejects_non_loopback_http_base_url() {
        match Client::new("http://example.com") {
            Err(Error::InsecureBaseUrl(_)) => {}
            _ => panic!("non-loopback http must be rejected"),
        }
    }

    #[test]
    fn new_rejects_hostless_https_base_url() {
        assert!(
            Client::new("https:///").is_err(),
            "hostless https base URL must be rejected"
        );
    }

    #[test]
    fn base_url_path_prefix_is_preserved_without_trailing_slash() {
        let client = Client::new("https://example.com/proxy").expect("valid base");
        let url = client
            .url("api/v3/account/")
            .expect("url should include configured prefix");
        assert_eq!(url.as_str(), "https://example.com/proxy/api/v3/account/");
    }

    #[test]
    fn url_rejects_parent_path_segments() {
        let client = Client::new("https://example.com/").expect("valid base");
        let err = client
            .url("api/v3/plants/../admin")
            .expect_err("must reject path traversal segments");
        assert!(matches!(err, Error::InvalidPath(_)));
    }

    #[test]
    fn url_rejects_percent_encoded_parent_path_segments() {
        let client = Client::new("https://example.com/").expect("valid base");
        let err = client
            .url("api/v3/plants/%2E%2E/admin")
            .expect_err("must reject encoded path traversal segments");
        assert!(matches!(err, Error::InvalidPath(_)));
    }

    #[test]
    fn url_rejects_lowercase_percent_encoded_parent_path_segments() {
        let client = Client::new("https://example.com/").expect("valid base");
        let err = client
            .url("api/v3/plants/%2e%2e/admin")
            .expect_err("must reject encoded path traversal segments");
        assert!(matches!(err, Error::InvalidPath(_)));
    }

    #[test]
    fn url_rejects_percent_encoded_path_separators() {
        let client = Client::new("https://example.com/").expect("valid base");
        let err = client
            .url("api/v3/plants/%2Fadmin")
            .expect_err("must reject encoded path separators");
        assert!(matches!(err, Error::InvalidPath(_)));
    }

    #[test]
    fn encode_path_segment_escapes_dot_segments() {
        assert_eq!(Client::encode_path_segment(".."), "%2E%2E");
        assert_eq!(Client::encode_path_segment("./x"), "%2E%2Fx");
    }

    #[test]
    fn api_error_redacts_raw_error_body_for_non_problem_json() {
        let raw = "secret=very-sensitive-token";
        let err = Client::api_error(StatusCode::INTERNAL_SERVER_ERROR, raw.to_string());
        match err {
            Error::Api { status, message } => {
                assert_eq!(status, 500);
                assert!(
                    !message.contains(raw),
                    "error message should not include raw upstream payload"
                );
            }
            _ => panic!("expected Api error"),
        }
    }

    #[tokio::test]
    async fn get_metrics_by_date_v3_serializes_fields_as_csv_query() {
        let server = spawn_mock_server(vec![MockStep {
            method: "GET",
            path_prefix:
                "/api/v3/plants/p1/metrics/device/panel-5m?date=2026-01-01&fields=i_out%2Cp",
            status: 200,
            content_type: "application/json",
            body: r#"{
                "plant_id":"p1",
                "unit":"panel",
                "source":"device",
                "date":"2026-01-01",
                "interval":"5m",
                "data":[
                  {
                    "id":"x1",
                    "date":"2026-01-01",
                    "timestamp":1,
                    "energy":1.0,
                    "cumulative_energy":2.0,
                    "i_out":3.0,
                    "p":4.0,
                    "v_in":5.0,
                    "v_out":6.0,
                    "temp":7.0
                  }
                ]
            }"#,
            stall_before_response: None,
        }]);

        let client = Client::new(&server.base_url).expect("create client");
        let fields = vec!["i_out".to_string(), "p".to_string()];
        let out = client
            .get_metrics_by_date_v3(
                "p1",
                "device",
                "panel",
                "5m",
                "2026-01-01",
                None,
                Some(&fields),
            )
            .await
            .expect("metrics request should succeed");
        assert!(matches!(out, MetricsBody::PanelIntraday(_)));
        server.handle.join().expect("join mock server");
    }

    #[tokio::test]
    async fn get_panel_seqnum_v3_returns_payload() {
        let server = spawn_mock_server(vec![MockStep {
            method: "GET",
            path_prefix: "/api/v3/plants/p1/indicator/seqnum?date=2026-01-01",
            status: 200,
            content_type: "application/json",
            body: r#"{"items":[{"timestamp":1,"seq_num":3}]}"#,
            stall_before_response: None,
        }]);

        let client = Client::new(&server.base_url).expect("create client");
        let out = client
            .get_panel_seqnum_v3("p1", "2026-01-01")
            .await
            .expect("seqnum request should succeed");
        assert_eq!(
            out.get("items")
                .and_then(serde_json::Value::as_array)
                .map(Vec::len),
            Some(1)
        );
        server.handle.join().expect("join mock server");
    }

    #[tokio::test]
    async fn get_panel_metrics_rejects_missing_data() {
        let server = spawn_mock_server(vec![MockStep {
            method: "GET",
            path_prefix: "/api/v3/plants/p1/metrics/device/panel-5m?date=2026-01-01",
            status: 200,
            content_type: "application/json",
            body: r#"{
                "plant_id":"p1",
                "unit":"panel",
                "source":"device",
                "date":"2026-01-01",
                "interval":"5m",
                "data": null
            }"#,
            stall_before_response: None,
        }]);

        let client = Client::new(&server.base_url).expect("create client");
        let err = client
            .get_panel_metrics("p1", "2026-01-01")
            .await
            .expect_err("missing data must be treated as an error");
        match err {
            Error::Api { status, message } => {
                assert_eq!(status, 500);
                assert!(message.contains("missing metrics data"));
            }
            _ => panic!("expected Api error for missing panel metrics data"),
        }
        server.handle.join().expect("join mock server");
    }

    #[tokio::test]
    async fn login_does_not_refresh_on_login_endpoint_unauthorized() {
        let server = spawn_mock_server(vec![
            MockStep {
                method: "POST",
                path_prefix: "/api/v3/account/auth-with-password",
                status: 200,
                content_type: "application/json",
                body: r#"{
                    "token":"old-token",
                    "type":"manager",
                    "name":"manager",
                    "email":"manager@example.com",
                    "username":null,
                    "organizations":null,
                    "metadata":null
                }"#,
                stall_before_response: None,
            },
            MockStep {
                method: "POST",
                path_prefix: "/api/v3/account/auth-with-password",
                status: 401,
                content_type: "application/json",
                body: r#"{"title":"invalid credentials","detail":"wrong password"}"#,
                stall_before_response: None,
            },
        ]);

        let client = Client::new(&server.base_url).expect("create client");
        client
            .login("manager@example.com", "pw")
            .await
            .expect("first login should succeed");
        let err = client
            .login("manager@example.com", "wrong")
            .await
            .expect_err("second login should fail with login endpoint error");
        match err {
            Error::ApiProblem { status, title, .. } => {
                assert_eq!(status, 401);
                assert_eq!(title, "invalid credentials");
            }
            _ => panic!("expected 401 login error without refresh"),
        }
        let auth_lock = client.auth.read().await;
        assert!(
            auth_lock.is_none(),
            "failed login must clear stale authentication context"
        );
        server.handle.join().expect("join mock server");
    }

    #[tokio::test]
    async fn login_endpoints_do_not_send_stale_authorization_header() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        listener
            .set_nonblocking(true)
            .expect("set nonblocking listener");
        let addr = listener.local_addr().expect("read local addr");
        let handle = thread::spawn(move || {
            let bodies = [
                r#"{"token":"first","type":"manager","name":"m","email":"m@example.com","username":null,"organizations":null,"metadata":null}"#,
                r#"{"token":"second","type":"manager","name":"m2","email":"m2@example.com","username":null,"organizations":null,"metadata":null}"#,
            ];
            for body in bodies {
                let mut stream = accept_with_timeout(&listener, TEST_ACCEPT_TIMEOUT);
                stream.set_nonblocking(false).expect("set blocking stream");
                let mut req_buf = [0_u8; 8192];
                let n = stream.read(&mut req_buf).expect("read request");
                let req = String::from_utf8_lossy(&req_buf[..n]);
                let req_line = req.lines().next().unwrap_or_default();
                assert!(
                    req_line.starts_with("POST /api/v3/account/auth-with-password"),
                    "unexpected request line `{req_line}`"
                );
                let req_lower = req.to_ascii_lowercase();
                assert!(
                    !req_lower.contains("\nauthorization:"),
                    "login request must not carry stale Authorization header"
                );
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
                stream.flush().expect("flush response");
            }
        });

        let client = Client::new(&format!("http://{addr}")).expect("create client");
        client
            .login("m@example.com", "pw")
            .await
            .expect("first login should succeed");
        client
            .login("m2@example.com", "pw2")
            .await
            .expect("second login should succeed");

        handle.join().expect("join mock server");
    }

    #[tokio::test]
    async fn get_blueprint_text_accepts_plain_text_response() {
        let server = spawn_mock_server(vec![MockStep {
            method: "GET",
            path_prefix: "/api/v3/plants/p1/blueprint?date=2026-01-01",
            status: 200,
            content_type: "text/plain",
            body: "raw-blueprint-content",
            stall_before_response: None,
        }]);

        let client = Client::new(&server.base_url).expect("create client");
        let text = client
            .get_blueprint_text_v3("p1", "2026-01-01")
            .await
            .expect("plain text blueprint should be accepted");
        assert_eq!(text, "raw-blueprint-content");
        server.handle.join().expect("join mock server");
    }

    #[tokio::test]
    async fn get_blueprint_text_decodes_json_string_payload() {
        let server = spawn_mock_server(vec![MockStep {
            method: "GET",
            path_prefix: "/api/v3/plants/p1/blueprint?date=2026-01-01",
            status: 200,
            content_type: "application/json",
            body: r#""aGVsbG8=""#,
            stall_before_response: None,
        }]);

        let client = Client::new(&server.base_url).expect("create client");
        let text = client
            .get_blueprint_text_v3("p1", "2026-01-01")
            .await
            .expect("json string blueprint should be decoded");
        assert_eq!(text, "aGVsbG8=");
        server.handle.join().expect("join mock server");
    }

    #[tokio::test]
    async fn create_plant_v3_accepts_legacy_response_shape() {
        let server = spawn_mock_server(vec![MockStep {
            method: "POST",
            path_prefix: "/api/v3/plants",
            status: 200,
            content_type: "application/json",
            body: r#"{
                "id":"p1",
                "name":"Plant One",
                "organization":"org-1",
                "organizationData":{"id":"org-1","name":"Org One","icon":null,"logo":null,"owner":null},
                "created":"2026-01-01T00:00:00Z",
                "updated":"2026-01-01T00:00:00Z",
                "metadata":{},
                "images":null
            }"#,
            stall_before_response: None,
        }]);

        let client = Client::new(&server.base_url).expect("create client");
        let created = client
            .create_plant_v3(&CreatePlantInput {
                name: "Plant One".to_string(),
                organization_id: "org-1".to_string(),
                metadata: None,
            })
            .await
            .expect("legacy create response should deserialize");
        assert_eq!(created.id, "p1");
        assert_eq!(created.organization.id, "org-1");
        server.handle.join().expect("join mock server");
    }

    #[tokio::test]
    async fn unauthorized_request_keeps_refresh_failure_cause() {
        let server = spawn_mock_server(vec![
            MockStep {
                method: "POST",
                path_prefix: "/api/v3/account/auth-with-password",
                status: 200,
                content_type: "application/json",
                body: r#"{
                    "token":"old-token",
                    "type":"manager",
                    "name":"manager",
                    "email":"manager@example.com",
                    "username":null,
                    "organizations":null,
                    "metadata":null
                }"#,
                stall_before_response: None,
            },
            MockStep {
                method: "GET",
                path_prefix: "/api/v3/account/",
                status: 401,
                content_type: "text/plain",
                body: "unauthorized",
                stall_before_response: None,
            },
            MockStep {
                method: "POST",
                path_prefix: "/api/v3/account/refresh-token",
                status: 500,
                content_type: "application/json",
                body: r#"{"title":"refresh failed","detail":"backend down"}"#,
                stall_before_response: None,
            },
        ]);

        let client = Client::new(&server.base_url).expect("create client");
        client
            .login("manager@example.com", "pw")
            .await
            .expect("login should succeed");
        let err = client
            .get_account()
            .await
            .expect_err("refresh failure should not collapse to Unauthorized");
        match err {
            Error::ApiProblem { status, title, .. } => {
                assert_eq!(status, 500);
                assert_eq!(title, "refresh failed");
            }
            _ => panic!("expected refresh API failure cause to be preserved"),
        }
        server.handle.join().expect("join mock server");
    }

    #[tokio::test]
    async fn unauthorized_refresh_maps_to_unauthorized_error() {
        let server = spawn_mock_server(vec![
            MockStep {
                method: "POST",
                path_prefix: "/api/v3/account/auth-with-password",
                status: 200,
                content_type: "application/json",
                body: r#"{
                    "token":"old-token",
                    "type":"manager",
                    "name":"manager",
                    "email":"manager@example.com",
                    "username":null,
                    "organizations":null,
                    "metadata":null
                }"#,
                stall_before_response: None,
            },
            MockStep {
                method: "GET",
                path_prefix: "/api/v3/account/",
                status: 401,
                content_type: "text/plain",
                body: "unauthorized",
                stall_before_response: None,
            },
            MockStep {
                method: "POST",
                path_prefix: "/api/v3/account/refresh-token",
                status: 401,
                content_type: "text/plain",
                body: "refresh unauthorized",
                stall_before_response: None,
            },
        ]);

        let client = Client::new(&server.base_url).expect("create client");
        client
            .login("manager@example.com", "pw")
            .await
            .expect("login should succeed");
        let err = client
            .get_account()
            .await
            .expect_err("must return unauthorized");
        assert!(matches!(err, Error::Unauthorized));
        server.handle.join().expect("join mock server");
    }

    #[tokio::test]
    async fn new_client_with_timeout_enforces_request_deadline() {
        let server = spawn_hanging_server(Duration::from_millis(300));
        let client = Client::new_with_timeout(&server.base_url, Duration::from_millis(50))
            .expect("create client");

        let result = tokio::time::timeout(Duration::from_millis(500), client.get_account())
            .await
            .expect("request should terminate via client timeout");
        let err = result.expect_err("request should fail with timeout");
        match err {
            Error::Request(req_err) => {
                assert!(req_err.is_timeout(), "request error should be timeout");
            }
            _ => panic!("expected request timeout error"),
        }
        server.handle.join().expect("join hanging server");
    }
}
