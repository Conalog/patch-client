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
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;

#[derive(Clone, Debug)]
struct AuthState {
    token: String,
    account_type: String,
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
        let base_url = Url::parse(base_url)?;
        Ok(Self {
            base_url,
            http: HttpClient::new(),
            auth: Arc::new(RwLock::new(None)),
        })
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

        let auth: AuthOutputV3Body = self
            .execute_json(
                Method::POST,
                self.url("api/v3/account/auth-with-password")?,
                Some(&body),
            )
            .await?;
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
        let auth: AuthBody = self
            .execute_json(
                Method::POST,
                self.url("api/v2/manager/auth-with-password")?,
                Some(&body),
            )
            .await?;
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
        let auth: AuthBody = self
            .execute_json(
                Method::POST,
                self.url("api/v2/viewer/auth-with-password")?,
                Some(&body),
            )
            .await?;
        let mut lock = self.auth.write().await;
        *lock = Some(AuthState {
            token: auth.token.clone(),
            account_type: "viewer".to_string(),
        });
        Ok(auth)
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
            let new_auth: AuthBody = res.json().await?;
            let mut lock = self.auth.write().await;
            if let Some(auth) = &mut *lock {
                auth.token = new_auth.token;
            }
            Ok(())
        } else {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
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
            message: body,
        }
    }

    async fn execute_json<T: serde::de::DeserializeOwned, B: serde::Serialize>(
        &self,
        method: Method,
        url: Url,
        body: Option<&B>,
    ) -> Result<T> {
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

            if let Some(b) = body {
                req = req.json(b);
            }

            let res = req.send().await?;
            let status = res.status();

            if status == StatusCode::UNAUTHORIZED && retries > 0 && authed {
                retries -= 1;
                if self.refresh_token().await.is_ok() {
                    continue;
                }
                return Err(Error::Unauthorized);
            }

            if status.is_success() {
                return Ok(res.json().await?);
            }

            let body = res.text().await.unwrap_or_default();
            return Err(Self::api_error(status, body));
        }
    }

    async fn execute_no_content(&self, method: Method, url: Url) -> Result<()> {
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
                if self.refresh_token().await.is_ok() {
                    continue;
                }
                return Err(Error::Unauthorized);
            }

            if status.is_success() {
                return Ok(());
            }

            let body = res.text().await.unwrap_or_default();
            return Err(Self::api_error(status, body));
        }
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

    pub async fn create_plant_v3(&self, input: &CreatePlantInput) -> Result<PlantBody> {
        self.execute_json(Method::POST, self.url("api/v3/plants")?, Some(input))
            .await
    }

    pub async fn get_blueprint_text_v3(&self, plant_id: &str, date: &str) -> Result<String> {
        let path = format!(
            "api/v3/plants/{}/blueprint",
            Self::encode_path_segment(plant_id)
        );
        let url = self.url_with_query(&path, &[("date", date.to_string())])?;
        self.execute_json(Method::GET, url, Option::<&()>::None)
            .await
    }

    pub async fn get_blueprint_text_v2(&self, plant_id: &str, date: &str) -> Result<String> {
        let path = format!(
            "api/v2/blueprint/plants/{}",
            Self::encode_path_segment(plant_id)
        );
        let url = self.url_with_query(&path, &[("date", date.to_string())])?;
        self.execute_json(Method::GET, url, Option::<&()>::None)
            .await
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
            MetricsBody::PanelIntraday(body) => Ok(PanelIntradayMetrics {
                data: body.data.unwrap_or_default(),
                plant_id: body.plant_id,
                date: body.date,
            }),
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
        if let Some(fs) = fields {
            for f in fs {
                q.push(("fields", f.to_string()));
            }
        }
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
    ) -> Result<()> {
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
        if let Some(fs) = fields {
            for f in fs {
                q.push(("fields", f.to_string()));
            }
        }
        let url = self.url_with_query(&path, &q)?;
        self.execute_no_content(Method::GET, url).await
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
        if let Some(fs) = fields {
            for f in fs {
                q.push(("fields", f.to_string()));
            }
        }
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
                if self.refresh_token().await.is_ok() {
                    continue;
                }
                return Err(Error::Unauthorized);
            }

            if status.is_success() {
                return Ok(res.json().await?);
            }

            let body = res.text().await.unwrap_or_default();
            return Err(Self::api_error(status, body));
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

    pub async fn get_panel_seqnum_v3(&self, plant_id: &str, date: &str) -> Result<()> {
        let path = format!(
            "api/v3/plants/{}/indicator/seqnum",
            Self::encode_path_segment(plant_id)
        );
        let url = self.url_with_query(&path, &[("date", date.to_string())])?;
        self.execute_no_content(Method::GET, url).await
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
}
