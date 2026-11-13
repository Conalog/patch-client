use super::*;

impl Client {
    /// Configure a manager token, or a participant token without Account-Type.
    pub async fn set_access_token(&self, token: &str, account_type: Option<&str>) -> Result<()> {
        let token = token.trim();
        let token = if token
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("bearer "))
        {
            &token[7..]
        } else {
            token
        };
        reqwest::header::HeaderValue::from_str(token)
            .map_err(|_| Error::InvalidPath("invalid access token".into()))?;
        if let Some(account_type) = account_type {
            ensure_allowed(
                account_type,
                &["manager", "viewer", "temporary"],
                "account_type",
            )?;
        }
        *self.auth.write().await = if token.is_empty() {
            None
        } else {
            Some(AuthState {
                token: token.into(),
                account_type: account_type.unwrap_or("").into(),
            })
        };
        Ok(())
    }

    fn validate_command_key(key: &str) -> Result<()> {
        if !(8..=128).contains(&key.len())
            || !key
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        {
            return Err(Error::InvalidPath(
                "command key must be 8–128 ASCII letters, digits, or ._:-".into(),
            ));
        }
        Ok(())
    }

    async fn v3_request(
        &self,
        method: Method,
        url: Url,
        authenticated: bool,
        stream: bool,
    ) -> Result<reqwest::RequestBuilder> {
        // A total response timeout would terminate a healthy, long-lived SSE connection.
        let http = if stream {
            HttpClient::builder()
                .retry(reqwest::retry::never())
                .connect_timeout(self.request_timeout)
                .read_timeout(self.request_timeout)
                .redirect(reqwest::redirect::Policy::none())
                .build()?
        } else {
            self.http.clone()
        };
        let mut req = http.request(method, url).header(
            "Accept",
            if stream {
                "text/event-stream"
            } else {
                "application/json"
            },
        );
        if authenticated {
            if let Some(auth) = self.auth.read().await.clone() {
                req = req.header("Authorization", format!("Bearer {}", auth.token));
                if !auth.account_type.is_empty() {
                    req = req.header("Account-Type", auth.account_type);
                }
            }
        }
        // Commands and multipart uploads are sent once. Callers control replay identity.
        Ok(req)
    }

    async fn v3_error<T>(res: reqwest::Response) -> Result<T> {
        let status = res.status();
        let body = Self::read_body_limited(res).await?;
        Err(Self::api_error(
            status,
            String::from_utf8_lossy(&body).into_owned(),
        ))
    }

    async fn v3_json_response(res: reqwest::Response) -> Result<serde_json::Value> {
        if !res.status().is_success() {
            return Self::v3_error(res).await;
        }
        let body = Self::read_body_limited(res).await?;
        if body.is_empty() {
            Ok(serde_json::Value::Null)
        } else {
            Ok(serde_json::from_slice(&body)?)
        }
    }

    async fn v3_head_response(res: reqwest::Response) -> Result<reqwest::Response> {
        let status = res.status();
        if matches!(
            status,
            StatusCode::OK | StatusCode::PARTIAL_CONTENT | StatusCode::RANGE_NOT_SATISFIABLE
        ) {
            Ok(res)
        } else {
            Self::v3_error(res).await
        }
    }

    /// GET /api/v3/fieldwork/attachments/content
    pub async fn fieldwork_attachment_content_v3(&self, query: &[(&str, &str)]) -> Result<Vec<u8>> {
        let mut url = self.url("api/v3/fieldwork/attachments/content")?;
        if !query.iter().any(|(k, v)| *k == "work_id" && !v.is_empty()) {
            return Err(Error::InvalidPath("missing work_id".into()));
        }
        if !query
            .iter()
            .any(|(k, v)| *k == "object_key" && !v.is_empty())
        {
            return Err(Error::InvalidPath("missing object_key".into()));
        }
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let req = self.v3_request(Method::GET, url, true, false).await?;
        let res = req.send().await?;
        if !res.status().is_success() {
            return Self::v3_error(res).await;
        }
        Self::read_body_limited(res).await
    }

    /// GET /api/v3/fieldwork/attachments/download
    pub async fn fieldwork_attachment_download_v3(
        &self,
        query: &[(&str, &str)],
    ) -> Result<Vec<u8>> {
        let mut url = self.url("api/v3/fieldwork/attachments/download")?;
        if !query.iter().any(|(k, v)| *k == "work_id" && !v.is_empty()) {
            return Err(Error::InvalidPath("missing work_id".into()));
        }
        if !query
            .iter()
            .any(|(k, v)| *k == "object_key" && !v.is_empty())
        {
            return Err(Error::InvalidPath("missing object_key".into()));
        }
        if !query.iter().any(|(k, v)| *k == "expires" && !v.is_empty()) {
            return Err(Error::InvalidPath("missing expires".into()));
        }
        if !query
            .iter()
            .any(|(k, v)| *k == "signature" && !v.is_empty())
        {
            return Err(Error::InvalidPath("missing signature".into()));
        }
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let req = self.v3_request(Method::GET, url, false, false).await?;
        let res = req.send().await?;
        if !res.status().is_success() {
            return Self::v3_error(res).await;
        }
        Self::read_body_limited(res).await
    }

    /// GET /api/v3/fieldwork/command-receipts
    pub async fn fieldwork_receipt_get_v3(
        &self,
        query: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let mut url = self.url("api/v3/fieldwork/command-receipts")?;
        if !query
            .iter()
            .any(|(k, v)| *k == "operation" && !v.is_empty())
        {
            return Err(Error::InvalidPath("missing operation".into()));
        }
        if !query
            .iter()
            .any(|(k, v)| *k == "command_key" && !v.is_empty())
        {
            return Err(Error::InvalidPath("missing command_key".into()));
        }
        Self::validate_command_key(query.iter().find(|(k, _)| *k == "command_key").unwrap().1)?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let req = self.v3_request(Method::GET, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/fieldwork/events
    pub async fn fieldwork_events_v3(&self, query: &[(&str, &str)]) -> Result<reqwest::Response> {
        let mut url = self.url("api/v3/fieldwork/events")?;
        if query.iter().any(|(k, v)| *k == "watch" && *v == "unread") && query.len() != 1 {
            return Err(Error::InvalidPath("watch=unread must be standalone".into()));
        }
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let req = self.v3_request(Method::GET, url, true, true).await?;
        let res = req.send().await?;
        if !res.status().is_success() {
            return Self::v3_error(res).await;
        }
        if !res
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase()
            .split(';')
            .next()
            .is_some_and(|media| media.trim() == "text/event-stream")
        {
            return Err(Error::Api {
                status: res.status().as_u16(),
                message: "expected text/event-stream".into(),
            });
        }
        Ok(res)
    }

    /// GET /api/v3/fieldwork/latest-message-previews
    pub async fn fieldwork_message_previews_v3(
        &self,
        query: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let mut url = self.url("api/v3/fieldwork/latest-message-previews")?;
        if !query.iter().any(|(k, v)| *k == "work_id" && !v.is_empty()) {
            return Err(Error::InvalidPath("missing work_id".into()));
        }
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let req = self.v3_request(Method::GET, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/participant-sessions
    pub async fn fieldwork_participant_session_create_v3(
        &self,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let url = self.url("api/v3/fieldwork/participant-sessions")?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/fieldwork/plants/{plant_id}/member-candidates
    pub async fn fieldwork_plant_member_candidates_v3(
        &self,
        plant_id: &str,
        query: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/plants/{}/member-candidates",
            Self::encode_path_segment(plant_id)
        );
        let mut url = self.url(&path)?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let req = self.v3_request(Method::GET, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/fieldwork/summary
    pub async fn fieldwork_summary_get_v3(
        &self,
        query: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let mut url = self.url("api/v3/fieldwork/summary")?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let req = self.v3_request(Method::GET, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/fieldwork/works
    pub async fn fieldwork_works_list_v3(
        &self,
        query: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let mut url = self.url("api/v3/fieldwork/works")?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let req = self.v3_request(Method::GET, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works
    pub async fn fieldwork_work_create_v3(
        &self,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let url = self.url("api/v3/fieldwork/works")?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/fieldwork/works/{work_id}
    pub async fn fieldwork_work_get_v3(
        &self,
        work_id: &str,
        query: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}",
            Self::encode_path_segment(work_id)
        );
        let mut url = self.url(&path)?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let req = self.v3_request(Method::GET, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/archive
    pub async fn fieldwork_work_archive_v3(
        &self,
        work_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/archive",
            Self::encode_path_segment(work_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/clone
    pub async fn fieldwork_work_clone_v3(
        &self,
        work_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/clone",
            Self::encode_path_segment(work_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/close
    pub async fn fieldwork_work_close_v3(
        &self,
        work_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/close",
            Self::encode_path_segment(work_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/items/{item_id}/complete
    pub async fn fieldwork_item_complete_v3(
        &self,
        work_id: &str,
        item_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/items/{}/complete",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(item_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/items/{item_id}/map-references/add
    pub async fn fieldwork_item_map_reference_add_v3(
        &self,
        work_id: &str,
        item_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/items/{}/map-references/add",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(item_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/items/{item_id}/map-references/sync
    pub async fn fieldwork_item_map_references_sync_v3(
        &self,
        work_id: &str,
        item_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/items/{}/map-references/sync",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(item_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/items/{item_id}/map-references/{map_ref_id}/remove
    pub async fn fieldwork_item_map_reference_remove_v3(
        &self,
        work_id: &str,
        item_id: &str,
        map_ref_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/items/{}/map-references/{}/remove",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(item_id),
            Self::encode_path_segment(map_ref_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/items/{item_id}/mentions/set
    pub async fn fieldwork_item_mentions_set_v3(
        &self,
        work_id: &str,
        item_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/items/{}/mentions/set",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(item_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/items/{item_id}/move
    pub async fn fieldwork_item_move_v3(
        &self,
        work_id: &str,
        item_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/items/{}/move",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(item_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/items/{item_id}/photos/add
    pub async fn fieldwork_item_photos_add_v3(
        &self,
        work_id: &str,
        item_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/items/{}/photos/add",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(item_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/items/{item_id}/photos/{photo_id}/move
    pub async fn fieldwork_item_photo_move_v3(
        &self,
        work_id: &str,
        item_id: &str,
        photo_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/items/{}/photos/{}/move",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(item_id),
            Self::encode_path_segment(photo_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/items/{item_id}/photos/{photo_id}/remove
    pub async fn fieldwork_item_photo_remove_v3(
        &self,
        work_id: &str,
        item_id: &str,
        photo_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/items/{}/photos/{}/remove",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(item_id),
            Self::encode_path_segment(photo_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/items/{item_id}/remove
    pub async fn fieldwork_item_remove_v3(
        &self,
        work_id: &str,
        item_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/items/{}/remove",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(item_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/items/{item_id}/reopen
    pub async fn fieldwork_item_reopen_v3(
        &self,
        work_id: &str,
        item_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/items/{}/reopen",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(item_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/items/{item_id}/update
    pub async fn fieldwork_item_update_v3(
        &self,
        work_id: &str,
        item_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/items/{}/update",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(item_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/members/invite
    pub async fn fieldwork_member_invite_v3(
        &self,
        work_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/members/invite",
            Self::encode_path_segment(work_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/members/self/join
    pub async fn fieldwork_member_join_v3(
        &self,
        work_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/members/self/join",
            Self::encode_path_segment(work_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/members/self/leave
    pub async fn fieldwork_member_leave_v3(
        &self,
        work_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/members/self/leave",
            Self::encode_path_segment(work_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/members/{member_id}/remove
    pub async fn fieldwork_member_remove_v3(
        &self,
        work_id: &str,
        member_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/members/{}/remove",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(member_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/members/{member_id}/responsible/add
    pub async fn fieldwork_member_responsible_add_v3(
        &self,
        work_id: &str,
        member_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/members/{}/responsible/add",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(member_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/members/{member_id}/responsible/remove
    pub async fn fieldwork_member_responsible_remove_v3(
        &self,
        work_id: &str,
        member_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/members/{}/responsible/remove",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(member_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/messages
    pub async fn fieldwork_message_create_v3(
        &self,
        work_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/messages",
            Self::encode_path_segment(work_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/messages/read
    pub async fn fieldwork_messages_read_v3(
        &self,
        work_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/messages/read",
            Self::encode_path_segment(work_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/fieldwork/works/{work_id}/messages/{message_id}
    pub async fn fieldwork_message_get_v3(
        &self,
        work_id: &str,
        message_id: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/messages/{}",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(message_id)
        );
        let url = self.url(&path)?;
        let req = self.v3_request(Method::GET, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/messages/{message_id}/attachments/remove
    pub async fn fieldwork_message_attachment_remove_v3(
        &self,
        work_id: &str,
        message_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/messages/{}/attachments/remove",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(message_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/messages/{message_id}/map-references/add
    pub async fn fieldwork_message_map_reference_add_v3(
        &self,
        work_id: &str,
        message_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/messages/{}/map-references/add",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(message_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/messages/{message_id}/map-references/remove
    pub async fn fieldwork_message_map_reference_remove_v3(
        &self,
        work_id: &str,
        message_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/messages/{}/map-references/remove",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(message_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/messages/{message_id}/map-references/sync
    pub async fn fieldwork_message_map_references_sync_v3(
        &self,
        work_id: &str,
        message_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/messages/{}/map-references/sync",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(message_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/messages/{message_id}/photos/add
    pub async fn fieldwork_message_photos_add_v3(
        &self,
        work_id: &str,
        message_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/messages/{}/photos/add",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(message_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/messages/{message_id}/photos/{photo_id}/move
    pub async fn fieldwork_message_photo_move_v3(
        &self,
        work_id: &str,
        message_id: &str,
        photo_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/messages/{}/photos/{}/move",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(message_id),
            Self::encode_path_segment(photo_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/messages/{message_id}/photos/{photo_id}/remove
    pub async fn fieldwork_message_photo_remove_v3(
        &self,
        work_id: &str,
        message_id: &str,
        photo_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/messages/{}/photos/{}/remove",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(message_id),
            Self::encode_path_segment(photo_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/messages/{message_id}/reaction/remove
    pub async fn fieldwork_reaction_remove_v3(
        &self,
        work_id: &str,
        message_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/messages/{}/reaction/remove",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(message_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/messages/{message_id}/reaction/set
    pub async fn fieldwork_reaction_set_v3(
        &self,
        work_id: &str,
        message_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/messages/{}/reaction/set",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(message_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/messages/{message_id}/remove
    pub async fn fieldwork_message_remove_v3(
        &self,
        work_id: &str,
        message_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/messages/{}/remove",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(message_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/messages/{message_id}/update
    pub async fn fieldwork_message_update_v3(
        &self,
        work_id: &str,
        message_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/messages/{}/update",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(message_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/reopen
    pub async fn fieldwork_work_reopen_v3(
        &self,
        work_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/reopen",
            Self::encode_path_segment(work_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/fieldwork/works/{work_id}/resources
    pub async fn fieldwork_resources_list_v3(
        &self,
        work_id: &str,
        query: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/resources",
            Self::encode_path_segment(work_id)
        );
        let mut url = self.url(&path)?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let req = self.v3_request(Method::GET, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/schedules/{schedule_id}/update
    pub async fn fieldwork_schedule_update_v3(
        &self,
        work_id: &str,
        schedule_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/schedules/{}/update",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(schedule_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/sections/{section_id}/move
    pub async fn fieldwork_section_move_v3(
        &self,
        work_id: &str,
        section_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/sections/{}/move",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(section_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/sections/{section_id}/remove
    pub async fn fieldwork_section_remove_v3(
        &self,
        work_id: &str,
        section_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/sections/{}/remove",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(section_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/sections/{section_id}/rename
    pub async fn fieldwork_section_rename_v3(
        &self,
        work_id: &str,
        section_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/sections/{}/rename",
            Self::encode_path_segment(work_id),
            Self::encode_path_segment(section_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/update
    pub async fn fieldwork_work_update_v3(
        &self,
        work_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/update",
            Self::encode_path_segment(work_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        Self::validate_command_key(command_key)?;
        req = req.header("Idempotency-Key", command_key);
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/orgs/{organization_id}/children
    pub async fn create_child_org_v3(
        &self,
        organization_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/orgs/{}/children",
            Self::encode_path_segment(organization_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/orgs/{organization_id}/members/{member_id}/transfer-ownership
    pub async fn transfer_org_ownership_v3(
        &self,
        organization_id: &str,
        member_id: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/orgs/{}/members/{}/transfer-ownership",
            Self::encode_path_segment(organization_id),
            Self::encode_path_segment(member_id)
        );
        let url = self.url(&path)?;
        let req = self.v3_request(Method::POST, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/orgs/{organization_id}/plants/{plant_id}/move
    pub async fn move_plant_organization_v3(
        &self,
        organization_id: &str,
        plant_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/orgs/{}/plants/{}/move",
            Self::encode_path_segment(organization_id),
            Self::encode_path_segment(plant_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/plants/{plant_id}/comments/{comment_id}
    pub async fn get_plant_comment_v3(
        &self,
        plant_id: &str,
        comment_id: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/comments/{}",
            Self::encode_path_segment(plant_id),
            Self::encode_path_segment(comment_id)
        );
        let url = self.url(&path)?;
        let req = self.v3_request(Method::GET, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/plants/{plant_id}/files
    pub async fn upload_plant_files_v3(
        &self,
        plant_id: &str,
        form: reqwest::multipart::Form,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/files",
            Self::encode_path_segment(plant_id)
        );
        let url = self.url(&path)?;
        let req = self
            .v3_request(Method::POST, url, true, false)
            .await?
            .multipart(form);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/plants/{plant_id}/images
    pub async fn upload_plant_images_v3(
        &self,
        plant_id: &str,
        form: reqwest::multipart::Form,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/images",
            Self::encode_path_segment(plant_id)
        );
        let url = self.url(&path)?;
        let req = self
            .v3_request(Method::POST, url, true, false)
            .await?
            .multipart(form);
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/plants/{plant_id}/logs/device
    pub async fn list_device_transition_logs_v3(
        &self,
        plant_id: &str,
        query: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/logs/device",
            Self::encode_path_segment(plant_id)
        );
        let mut url = self.url(&path)?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let req = self.v3_request(Method::GET, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/plants/{plant_id}/logs/ess
    pub async fn list_ess_transition_logs_v3(
        &self,
        plant_id: &str,
        query: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/logs/ess",
            Self::encode_path_segment(plant_id)
        );
        let mut url = self.url(&path)?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let req = self.v3_request(Method::GET, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/plants/{plant_id}/memos
    pub async fn list_plant_memos_v3(&self, plant_id: &str) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/memos",
            Self::encode_path_segment(plant_id)
        );
        let url = self.url(&path)?;
        let req = self.v3_request(Method::GET, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/plants/{plant_id}/memos/start_thread
    pub async fn start_plant_memo_thread_v3(
        &self,
        plant_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/memos/start_thread",
            Self::encode_path_segment(plant_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/plants/{plant_id}/memos/{comment_id}
    pub async fn get_plant_memo_v3(
        &self,
        plant_id: &str,
        comment_id: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/memos/{}",
            Self::encode_path_segment(plant_id),
            Self::encode_path_segment(comment_id)
        );
        let url = self.url(&path)?;
        let req = self.v3_request(Method::GET, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/plants/{plant_id}/memos/{comment_id}/edit
    pub async fn edit_plant_memo_v3(
        &self,
        plant_id: &str,
        comment_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/memos/{}/edit",
            Self::encode_path_segment(plant_id),
            Self::encode_path_segment(comment_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/plants/{plant_id}/memos/{comment_id}/reply
    pub async fn reply_plant_memo_v3(
        &self,
        plant_id: &str,
        comment_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/memos/{}/reply",
            Self::encode_path_segment(plant_id),
            Self::encode_path_segment(comment_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/plants/{plant_id}/memos/{comment_id}/state
    pub async fn change_plant_memo_state_v3(
        &self,
        plant_id: &str,
        comment_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/memos/{}/state",
            Self::encode_path_segment(plant_id),
            Self::encode_path_segment(comment_id)
        );
        let url = self.url(&path)?;
        let mut req = self.v3_request(Method::POST, url, true, false).await?;
        req = req.json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/plants/{plant_id}/metrics/edge/latest
    pub async fn get_latest_edge_metrics_v3(
        &self,
        plant_id: &str,
        query: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/metrics/edge/latest",
            Self::encode_path_segment(plant_id)
        );
        let mut url = self.url(&path)?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let req = self.v3_request(Method::GET, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// HEAD /api/v3/fieldwork/attachments/content
    pub async fn fieldwork_attachment_content_head_v3(
        &self,
        query: &[(&str, &str)],
        headers: &[(&str, &str)],
    ) -> Result<reqwest::Response> {
        for required in ["work_id", "object_key"] {
            if !query.iter().any(|(k, v)| *k == required && !v.is_empty()) {
                return Err(Error::InvalidPath(format!("missing {required}")));
            }
        }
        let mut url = self.url("api/v3/fieldwork/attachments/content")?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let mut req = self.v3_request(Method::HEAD, url, true, false).await?;
        for (name, value) in headers {
            ensure_allowed(name, &["Range", "If-Range"], "header")?;
            req = req.header(*name, *value);
        }
        Self::v3_head_response(req.send().await?).await
    }

    /// HEAD /api/v3/fieldwork/attachments/download
    pub async fn fieldwork_attachment_download_head_v3(
        &self,
        query: &[(&str, &str)],
        headers: &[(&str, &str)],
    ) -> Result<reqwest::Response> {
        for required in ["work_id", "object_key", "expires", "signature"] {
            if !query.iter().any(|(k, v)| *k == required && !v.is_empty()) {
                return Err(Error::InvalidPath(format!("missing {required}")));
            }
        }
        let mut url = self.url("api/v3/fieldwork/attachments/download")?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let mut req = self.v3_request(Method::HEAD, url, false, false).await?;
        for (name, value) in headers {
            ensure_allowed(name, &["Range", "If-Range"], "header")?;
            req = req.header(*name, *value);
        }
        Self::v3_head_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/messages/read-all
    pub async fn fieldwork_messages_read_all_v3(
        &self,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        Self::validate_command_key(command_key)?;
        let url = self.url("api/v3/fieldwork/messages/read-all")?;
        let req = self
            .v3_request(Method::POST, url, true, false)
            .await?
            .header("Idempotency-Key", command_key)
            .json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/fieldwork/work-items-templates
    pub async fn fieldwork_work_items_templates_list_v3(
        &self,
        query: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let mut url = self.url("api/v3/fieldwork/work-items-templates")?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let req = self.v3_request(Method::GET, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/checklist/entries
    pub async fn fieldwork_checklist_create_v3(
        &self,
        work_id: &str,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        Self::validate_command_key(command_key)?;
        let path = format!(
            "api/v3/fieldwork/works/{}/checklist/entries",
            Self::encode_path_segment(work_id)
        );
        let req = self
            .v3_request(Method::POST, self.url(&path)?, true, false)
            .await?
            .header("Idempotency-Key", command_key)
            .json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/fieldwork/works/{work_id}/latest
    pub async fn fieldwork_latest_get_v3(
        &self,
        work_id: &str,
        scope: &str,
        target_id: Option<&str>,
        include_children: Option<bool>,
    ) -> Result<serde_json::Value> {
        if scope.is_empty() {
            return Err(Error::InvalidPath("missing scope".into()));
        }
        let path = format!(
            "api/v3/fieldwork/works/{}/latest",
            Self::encode_path_segment(work_id)
        );
        let mut query = vec![("scope", scope.to_string())];
        if let Some(value) = target_id.filter(|value| !value.is_empty()) {
            query.push(("target_id", value.to_string()));
        }
        if let Some(value) = include_children {
            query.push(("include_children", value.to_string()));
        }
        let url = self.url_with_query(&path, &query)?;
        let req = self.v3_request(Method::GET, url, true, false).await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/fieldwork/works/{work_id}/members/self/decline
    pub async fn fieldwork_member_decline_v3(
        &self,
        work_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/fieldwork/works/{}/members/self/decline",
            Self::encode_path_segment(work_id)
        );
        let req = self
            .v3_request(Method::POST, self.url(&path)?, true, false)
            .await?
            .json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/files
    pub async fn file_upload_v3(
        &self,
        command_key: &str,
        form: reqwest::multipart::Form,
    ) -> Result<serde_json::Value> {
        Self::validate_command_key(command_key)?;
        let req = self
            .v3_request(Method::POST, self.url("api/v3/files")?, true, false)
            .await?
            .header("Idempotency-Key", command_key)
            .multipart(form);
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/files/upload-sessions
    pub async fn file_upload_start_v3(
        &self,
        body: &serde_json::Value,
        command_key: &str,
    ) -> Result<serde_json::Value> {
        Self::validate_command_key(command_key)?;
        let req = self
            .v3_request(
                Method::POST,
                self.url("api/v3/files/upload-sessions")?,
                true,
                false,
            )
            .await?
            .header("Idempotency-Key", command_key)
            .json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// PATCH /api/v3/files/upload-sessions/{session_id}/bytes
    pub async fn file_upload_bytes_v3(
        &self,
        session_id: &str,
        upload_context: &str,
        upload_offset: u64,
        bytes: &[u8],
    ) -> Result<serde_json::Value> {
        if upload_context.is_empty() {
            return Err(Error::InvalidPath("missing Upload-Context".into()));
        }
        let path = format!(
            "api/v3/files/upload-sessions/{}/bytes",
            Self::encode_path_segment(session_id)
        );
        let req = self
            .v3_request(Method::PATCH, self.url(&path)?, true, false)
            .await?
            .header("Upload-Context", upload_context)
            .header("Upload-Offset", upload_offset.to_string())
            .header("Content-Length", bytes.len().to_string())
            .header("Content-Type", "application/offset+octet-stream")
            .body(bytes.to_vec());
        Self::v3_json_response(req.send().await?).await
    }

    /// POST /api/v3/plants/{plant_id}/drone-inspection/panel-missions
    pub async fn create_panel_photo_mission_v3(
        &self,
        plant_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/drone-inspection/panel-missions",
            Self::encode_path_segment(plant_id)
        );
        let req = self
            .v3_request(Method::POST, self.url(&path)?, true, false)
            .await?
            .json(body);
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/plants/{plant_id}/drone-inspection/panel-missions/{mission_id}
    pub async fn get_panel_photo_mission_v3(
        &self,
        plant_id: &str,
        mission_id: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/drone-inspection/panel-missions/{}",
            Self::encode_path_segment(plant_id),
            Self::encode_path_segment(mission_id)
        );
        let req = self
            .v3_request(Method::GET, self.url(&path)?, true, false)
            .await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/plants/{plant_id}/drone-inspection/panel-missions/{mission_id}/photos
    pub async fn get_panel_photo_results_v3(
        &self,
        plant_id: &str,
        mission_id: &str,
    ) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/drone-inspection/panel-missions/{}/photos",
            Self::encode_path_segment(plant_id),
            Self::encode_path_segment(mission_id)
        );
        let req = self
            .v3_request(Method::GET, self.url(&path)?, true, false)
            .await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/plants/{plant_id}/metrics/equip/latest
    pub async fn get_latest_equip_metrics_v3(&self, plant_id: &str) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/metrics/equip/latest",
            Self::encode_path_segment(plant_id)
        );
        let req = self
            .v3_request(Method::GET, self.url(&path)?, true, false)
            .await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v3/plants/{plant_id}/tasks
    pub async fn get_plant_tasks_v3(&self, plant_id: &str) -> Result<serde_json::Value> {
        let path = format!(
            "api/v3/plants/{}/tasks",
            Self::encode_path_segment(plant_id)
        );
        let req = self
            .v3_request(Method::GET, self.url(&path)?, true, false)
            .await?;
        Self::v3_json_response(req.send().await?).await
    }

    /// GET /api/v4/plants/{plant_id}/blueprint
    pub async fn get_v4_plant_blueprint(
        &self,
        plant_id: &str,
        date: &str,
        conditional_headers: &[(&str, &str)],
    ) -> Result<BlueprintV4Response> {
        if date.is_empty() {
            return Err(Error::InvalidPath("missing date".into()));
        }
        let path = format!(
            "api/v4/plants/{}/blueprint",
            Self::encode_path_segment(plant_id)
        );
        let url = self.url_with_query(&path, &[("date", date.to_string())])?;
        let mut req = self.v3_request(Method::GET, url, true, false).await?;
        for (name, value) in conditional_headers {
            ensure_allowed(
                name,
                &[
                    "If-Match",
                    "If-None-Match",
                    "If-Modified-Since",
                    "If-Unmodified-Since",
                ],
                "header",
            )?;
            req = req.header(*name, *value);
        }
        let res = req.send().await?;
        let status = res.status();
        let etag = res
            .headers()
            .get("ETag")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        if status == StatusCode::NOT_MODIFIED {
            return Ok(BlueprintV4Response {
                status: 304,
                etag,
                body: None,
            });
        }
        if !status.is_success() {
            return Self::v3_error(res).await;
        }
        let body = Self::read_body_limited(res).await?;
        let body = if body.is_empty() {
            None
        } else {
            Some(serde_json::from_slice(&body)?)
        };
        Ok(BlueprintV4Response {
            status: status.as_u16(),
            etag,
            body,
        })
    }
}
