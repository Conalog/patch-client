package patchclient

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"mime"
	"mime/multipart"
	"net"
	"net/http"
	"net/textproto"
	"net/url"
	"sort"
	"strings"
	"sync"
	"time"
)

type AccountType string

const (
	AccountTypeViewer  AccountType = "viewer"
	AccountTypeManager AccountType = "manager"
	AccountTypeAdmin   AccountType = "admin"
)

type RequestOptions struct {
	AccessToken string
	AccountType AccountType
	Headers     map[string]string
}

type FilePart struct {
	Filename    string
	ContentType string
	Content     []byte
}

type Client struct {
	BaseURL    string
	HTTPClient *http.Client
	mu         sync.RWMutex

	AccessToken string
	AccountType AccountType

	defaultHeaders    map[string]string
	maxResponseBytes  int64
	maxMultipartBytes int64
	allowInsecureHTTP bool
}

type PatchClientError struct {
	Method     string
	URL        string
	StatusCode int
	Body       string
}

const defaultMaxResponseBytes int64 = 10 << 20
const defaultMaxMultipartBytes int64 = 20 << 20
const maxInt64 = int64(^uint64(0) >> 1)

var fallbackHTTPClient = &http.Client{Timeout: 30 * time.Second}

func (e *PatchClientError) Error() string {
	if e.Method != "" && e.URL != "" {
		return fmt.Sprintf("PATCH API request failed: %s %s returned status %d", e.Method, e.URL, e.StatusCode)
	}
	return fmt.Sprintf("PATCH API request failed with status %d", e.StatusCode)
}

func (e *PatchClientError) BodySnippet(maxRunes int) string {
	if maxRunes <= 0 {
		return ""
	}

	body := strings.TrimSpace(e.Body)
	runes := []rune(body)
	if len(runes) <= maxRunes {
		return body
	}
	return string(runes[:maxRunes]) + "..."
}

func NewClient(baseURL string) *Client {
	if baseURL == "" {
		baseURL = "https://patch-api.conalog.com"
	}
	baseURL = normalizeBaseURL(baseURL)
	return &Client{
		BaseURL:           strings.TrimRight(baseURL, "/"),
		HTTPClient:        &http.Client{Timeout: 30 * time.Second},
		defaultHeaders:    map[string]string{},
		maxResponseBytes:  defaultMaxResponseBytes,
		maxMultipartBytes: defaultMaxMultipartBytes,
	}
}

func (c *Client) SetAccessToken(token string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.AccessToken = token
}

func (c *Client) SetAccountType(accountType AccountType) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.AccountType = accountType
}

func (c *Client) SetDefaultHeader(key string, value string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.defaultHeaders == nil {
		c.defaultHeaders = map[string]string{}
	}
	c.defaultHeaders[key] = value
}

func (c *Client) SetDefaultHeaders(headers map[string]string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.defaultHeaders = cloneMap(headers)
}

func (c *Client) GetDefaultHeaders() map[string]string {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return cloneMap(c.defaultHeaders)
}

func (c *Client) SetMaxResponseBytes(limit int64) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if limit <= 0 {
		c.maxResponseBytes = defaultMaxResponseBytes
		return
	}
	c.maxResponseBytes = limit
}

func (c *Client) SetMaxMultipartBytes(limit int64) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if limit <= 0 {
		c.maxMultipartBytes = defaultMaxMultipartBytes
		return
	}
	c.maxMultipartBytes = limit
}

func (c *Client) SetAllowInsecureHTTP(allow bool) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.allowInsecureHTTP = allow
}

func (c *Client) AuthenticateUser(ctx context.Context, payload any) (any, error) {
	return c.doJSON(ctx, http.MethodPost, "/api/v3/account/auth-with-password", nil, payload, nil, nil)
}

func (c *Client) RefreshUserToken(ctx context.Context, opts *RequestOptions) (any, error) {
	return c.doJSON(ctx, http.MethodPost, "/api/v3/account/refresh-token", nil, nil, nil, opts)
}

func (c *Client) GetAccountInfo(ctx context.Context, opts *RequestOptions) (any, error) {
	return c.doJSON(ctx, http.MethodGet, "/api/v3/account/", nil, nil, nil, opts)
}

func (c *Client) CreateOrganizationMember(ctx context.Context, organizationID string, payload any, opts *RequestOptions) (any, error) {
	path := fmt.Sprintf("/api/v3/organizations/%s/members", encodePath(organizationID))
	return c.doJSON(ctx, http.MethodPost, path, nil, payload, nil, opts)
}

func (c *Client) AssignPlantPermission(ctx context.Context, organizationID string, payload any, opts *RequestOptions) (any, error) {
	path := fmt.Sprintf("/api/v3/organizations/%s/permissions", encodePath(organizationID))
	return c.doJSON(ctx, http.MethodPost, path, nil, payload, nil, opts)
}

func (c *Client) GetPlantList(ctx context.Context, query map[string]string, opts *RequestOptions) (any, error) {
	return c.doJSON(ctx, http.MethodGet, "/api/v3/plants", query, nil, nil, opts)
}

func (c *Client) CreatePlant(ctx context.Context, payload any, opts *RequestOptions) (any, error) {
	return c.doJSON(ctx, http.MethodPost, "/api/v3/plants", nil, payload, nil, opts)
}

func (c *Client) GetPlantDetails(ctx context.Context, plantID string, opts *RequestOptions) (any, error) {
	path := fmt.Sprintf("/api/v3/plants/%s", encodePath(plantID))
	return c.doJSON(ctx, http.MethodGet, path, nil, nil, nil, opts)
}

func (c *Client) GetPlantBlueprint(ctx context.Context, plantID string, date string, opts *RequestOptions) (any, error) {
	path := fmt.Sprintf("/api/v3/plants/%s/blueprint", encodePath(plantID))
	return c.doJSON(ctx, http.MethodGet, path, map[string]string{"date": date}, nil, nil, opts)
}

func (c *Client) UploadPlantFiles(ctx context.Context, plantID string, fields map[string]string, files map[string]FilePart, opts *RequestOptions) (any, error) {
	path := fmt.Sprintf("/api/v3/plants/%s/files", encodePath(plantID))
	normalizedFields, normalizedFiles, err := normalizeUploadPayload(fields, files)
	if err != nil {
		return nil, err
	}
	contentType, payload, err := encodeMultipart(normalizedFields, normalizedFiles, c.multipartLimit())
	if err != nil {
		return nil, err
	}
	return c.doJSON(ctx, http.MethodPost, path, nil, nil, payload, withContentType(opts, contentType))
}

func (c *Client) GetAssetHealthLevel(ctx context.Context, plantID string, unit string, date string, view string, opts *RequestOptions) (any, error) {
	path := fmt.Sprintf("/api/v3/plants/%s/indicator/health-level/%s", encodePath(plantID), encodePath(unit))
	query := map[string]string{"date": date}
	if view != "" {
		query["view"] = view
	}
	return c.doJSON(ctx, http.MethodGet, path, query, nil, nil, opts)
}

func (c *Client) GetPanelSeqnum(ctx context.Context, plantID string, date string, opts *RequestOptions) (any, error) {
	path := fmt.Sprintf("/api/v3/plants/%s/indicator/seqnum", encodePath(plantID))
	return c.doJSON(ctx, http.MethodGet, path, map[string]string{"date": date}, nil, nil, opts)
}

func (c *Client) ListInverterLogs(ctx context.Context, plantID string, query map[string]string, opts *RequestOptions) (any, error) {
	path := fmt.Sprintf("/api/v3/plants/%s/logs/inverter", encodePath(plantID))
	return c.doJSON(ctx, http.MethodGet, path, query, nil, nil, opts)
}

func (c *Client) ListInverterLogsByID(ctx context.Context, plantID string, inverterID string, query map[string]string, opts *RequestOptions) (any, error) {
	path := fmt.Sprintf("/api/v3/plants/%s/logs/inverters/%s", encodePath(plantID), encodePath(inverterID))
	return c.doJSON(ctx, http.MethodGet, path, query, nil, nil, opts)
}

func (c *Client) GetLatestDeviceMetrics(ctx context.Context, plantID string, query map[string]string, opts *RequestOptions) (any, error) {
	path := fmt.Sprintf("/api/v3/plants/%s/metrics/device/latest", encodePath(plantID))
	return c.doJSON(ctx, http.MethodGet, path, query, nil, nil, opts)
}

func (c *Client) GetLatestInverterMetrics(ctx context.Context, plantID string, opts *RequestOptions) (any, error) {
	path := fmt.Sprintf("/api/v3/plants/%s/metrics/inverter/latest", encodePath(plantID))
	return c.doJSON(ctx, http.MethodGet, path, nil, nil, nil, opts)
}

func (c *Client) GetMetricsByDate(
	ctx context.Context,
	plantID string,
	source string,
	unit string,
	interval string,
	date string,
	query map[string]string,
	opts *RequestOptions,
) (any, error) {
	path := fmt.Sprintf(
		"/api/v3/plants/%s/metrics/%s/%s-%s",
		encodePath(plantID),
		encodePath(source),
		encodePath(unit),
		encodePath(interval),
	)
	q := cloneMap(query)
	q["date"] = date
	return c.doJSON(ctx, http.MethodGet, path, q, nil, nil, opts)
}

func (c *Client) GetAssetRegistrationOnPlant(
	ctx context.Context,
	plantID string,
	recordType string,
	date string,
	query map[string]string,
	opts *RequestOptions,
) (any, error) {
	path := fmt.Sprintf("/api/v3/plants/%s/registry/%s", encodePath(plantID), encodePath(recordType))
	q := cloneMap(query)
	q["date"] = date
	return c.doJSON(ctx, http.MethodGet, path, q, nil, nil, opts)
}

func (c *Client) doJSON(
	ctx context.Context,
	method string,
	path string,
	query map[string]string,
	jsonBody any,
	rawBody []byte,
	opts *RequestOptions,
) (any, error) {
	target, err := c.buildURL(path, query)
	if err != nil {
		return nil, err
	}

	var body io.Reader
	contentType := ""
	hasBody := false
	if jsonBody != nil {
		encoded, marshalErr := json.Marshal(jsonBody)
		if marshalErr != nil {
			return nil, marshalErr
		}
		body = bytes.NewReader(encoded)
		contentType = "application/json"
		hasBody = true
	} else if rawBody != nil {
		body = bytes.NewReader(rawBody)
		hasBody = true
	}

	req, err := http.NewRequestWithContext(nonNilContext(ctx), method, target, body)
	if err != nil {
		return nil, err
	}

	headers := c.mergeHeaders(opts)
	if headers["Accept"] == "" {
		headers["Accept"] = "application/json"
	}
	if contentType != "" {
		headers["Content-Type"] = contentType
	}

	for k, v := range headers {
		req.Header.Set(k, v)
	}

	if c.shouldBlockInsecureRequest(target) {
		return nil, fmt.Errorf("refusing to send request over insecure transport")
	}

	client := c.httpClient()
	if shouldDisableRedirects(headers, hasBody) {
		client = withRedirectsDisabled(client)
	} else {
		client = withRedirectSecurityChecks(client, c.shouldBlockInsecureRequest)
	}
	resp, err := client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	limit := c.responseLimit()
	payload, overflowed, err := readBodyWithLimit(resp.Body, limit)
	if err != nil {
		return nil, err
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, &PatchClientError{
			Method:     method,
			URL:        target,
			StatusCode: resp.StatusCode,
			Body:       string(payload),
		}
	}
	if overflowed {
		return nil, fmt.Errorf("response body exceeds %d bytes", limit)
	}

	if len(payload) == 0 {
		return nil, nil
	}

	if isJSONContentType(resp.Header.Get("Content-Type")) {
		var out any
		if err := json.Unmarshal(payload, &out); err != nil {
			return nil, err
		}
		return out, nil
	}

	return string(payload), nil
}

func (c *Client) buildURL(path string, query map[string]string) (string, error) {
	target, err := url.Parse(strings.TrimRight(c.BaseURL, "/") + path)
	if err != nil {
		return "", err
	}

	params := target.Query()
	for k, v := range query {
		if v != "" {
			params.Set(k, v)
		}
	}
	target.RawQuery = params.Encode()
	return target.String(), nil
}

func (c *Client) mergeHeaders(opts *RequestOptions) map[string]string {
	c.mu.RLock()
	defaultHeaders := cloneMap(c.defaultHeaders)
	token := c.AccessToken
	accountType := c.AccountType
	c.mu.RUnlock()

	headers := map[string]string{}
	for _, k := range sortedHeaderKeys(defaultHeaders) {
		v := defaultHeaders[k]
		ck := canonicalHeaderKey(k)
		if ck != "" {
			headers[ck] = v
		}
	}

	if opts != nil && opts.AccessToken != "" {
		token = opts.AccessToken
	}
	if token != "" {
		headers["Authorization"] = asBearer(token)
	}

	if opts != nil && opts.AccountType != "" {
		accountType = opts.AccountType
	}
	if accountType != "" {
		headers["Account-Type"] = string(accountType)
	}

	if opts != nil {
		for _, k := range sortedHeaderKeys(opts.Headers) {
			v := opts.Headers[k]
			ck := canonicalHeaderKey(k)
			if ck != "" {
				headers[ck] = v
			}
		}
	}

	return headers
}

func withContentType(opts *RequestOptions, contentType string) *RequestOptions {
	if opts == nil {
		return &RequestOptions{Headers: map[string]string{"Content-Type": contentType}}
	}
	out := &RequestOptions{
		AccessToken: opts.AccessToken,
		AccountType: opts.AccountType,
		Headers:     cloneMap(opts.Headers),
	}
	if out.Headers == nil {
		out.Headers = map[string]string{}
	}
	for k := range out.Headers {
		if strings.EqualFold(k, "Content-Type") {
			delete(out.Headers, k)
		}
	}
	out.Headers["Content-Type"] = contentType
	return out
}

func normalizeUploadPayload(fields map[string]string, files map[string]FilePart) (map[string]string, map[string]FilePart, error) {
	outFields := cloneMap(fields)
	outFiles := cloneFileMap(files)

	// OpenAPI schema defines multipart keys as "name" and "filename".
	if _, ok := outFiles["filename"]; !ok {
		if len(outFiles) != 1 {
			return nil, nil, fmt.Errorf("upload files must include 'filename' field")
		}
		for _, file := range outFiles {
			outFiles = map[string]FilePart{"filename": file}
			break
		}
	}

	if _, ok := outFields["name"]; !ok {
		if file, ok := outFiles["filename"]; ok && file.Filename != "" {
			outFields["name"] = file.Filename
		} else {
			outFields["name"] = "file"
		}
	}

	return outFields, outFiles, nil
}

func encodeMultipart(fields map[string]string, files map[string]FilePart, limit int64) (string, []byte, error) {
	var buf bytes.Buffer
	writer := multipart.NewWriter(&buf)
	if limit <= 0 {
		limit = defaultMaxMultipartBytes
	}

	for k, v := range fields {
		safeName, err := rejectCRLF(k, "multipart field name")
		if err != nil {
			return "", nil, err
		}
		if err := writer.WriteField(safeName, v); err != nil {
			return "", nil, err
		}
		if int64(buf.Len()) > limit {
			return "", nil, fmt.Errorf("multipart payload exceeds %d bytes", limit)
		}
	}

	for fieldName, filePart := range files {
		safeFieldName, err := rejectCRLF(fieldName, "multipart file field name")
		if err != nil {
			return "", nil, err
		}
		safeFilename, err := rejectCRLF(filePart.Filename, "multipart filename")
		if err != nil {
			return "", nil, err
		}
		contentType := filePart.ContentType
		if contentType == "" {
			contentType = "application/octet-stream"
		}
		safeContentType, err := rejectCRLF(contentType, "multipart content type")
		if err != nil {
			return "", nil, err
		}
		header := textproto.MIMEHeader{}
		header.Set(
			"Content-Disposition",
			fmt.Sprintf(`form-data; name="%s"; filename="%s"`, escapeQuotes(safeFieldName), escapeQuotes(safeFilename)),
		)
		header.Set("Content-Type", safeContentType)

		part, err := writer.CreatePart(header)
		if err != nil {
			return "", nil, err
		}
		if int64(buf.Len())+int64(len(filePart.Content)) > limit {
			return "", nil, fmt.Errorf("multipart payload exceeds %d bytes", limit)
		}
		if _, err := part.Write(filePart.Content); err != nil {
			return "", nil, err
		}
		if int64(buf.Len()) > limit {
			return "", nil, fmt.Errorf("multipart payload exceeds %d bytes", limit)
		}
	}

	if err := writer.Close(); err != nil {
		return "", nil, err
	}
	if int64(buf.Len()) > limit {
		return "", nil, fmt.Errorf("multipart payload exceeds %d bytes", limit)
	}

	return writer.FormDataContentType(), buf.Bytes(), nil
}

func asBearer(token string) string {
	if len(token) >= len("Bearer ") && strings.EqualFold(token[:len("Bearer ")], "Bearer ") {
		return token
	}
	return "Bearer " + token
}

func encodePath(v string) string {
	return url.PathEscape(v)
}

func cloneMap(in map[string]string) map[string]string {
	if len(in) == 0 {
		return map[string]string{}
	}
	out := make(map[string]string, len(in))
	for k, v := range in {
		out[k] = v
	}
	return out
}

func cloneFileMap(in map[string]FilePart) map[string]FilePart {
	if len(in) == 0 {
		return map[string]FilePart{}
	}
	out := make(map[string]FilePart, len(in))
	for k, v := range in {
		out[k] = v
	}
	return out
}

func escapeQuotes(v string) string {
	replacer := strings.NewReplacer("\\", "\\\\", "\"", "\\\"")
	return replacer.Replace(v)
}

func canonicalHeaderKey(k string) string {
	k = strings.TrimSpace(k)
	if k == "" {
		return ""
	}
	return textproto.CanonicalMIMEHeaderKey(k)
}

func sortedHeaderKeys(m map[string]string) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
}

func (c *Client) httpClient() *http.Client {
	c.mu.RLock()
	client := c.HTTPClient
	c.mu.RUnlock()
	if client == nil {
		return fallbackHTTPClient
	}
	return client
}

func (c *Client) responseLimit() int64 {
	c.mu.RLock()
	limit := c.maxResponseBytes
	c.mu.RUnlock()
	if limit <= 0 {
		return defaultMaxResponseBytes
	}
	return limit
}

func (c *Client) multipartLimit() int64 {
	c.mu.RLock()
	limit := c.maxMultipartBytes
	c.mu.RUnlock()
	if limit <= 0 {
		return defaultMaxMultipartBytes
	}
	return limit
}

func (c *Client) shouldBlockInsecureRequest(target string) bool {
	c.mu.RLock()
	allowInsecure := c.allowInsecureHTTP
	c.mu.RUnlock()
	if allowInsecure {
		return false
	}
	u, err := url.Parse(target)
	if err != nil {
		return true
	}
	if strings.EqualFold(u.Scheme, "https") {
		return false
	}
	host := u.Hostname()
	if host == "" {
		return true
	}
	if ip := net.ParseIP(host); ip != nil {
		return ip.IsLoopback() == false
	}
	return !strings.EqualFold(host, "localhost")
}

func shouldDisableRedirects(headers map[string]string, hasBody bool) bool {
	if hasBody || hasAuthorizationHeader(headers) {
		return true
	}
	for k, v := range headers {
		if strings.TrimSpace(v) == "" {
			continue
		}
		if strings.EqualFold(k, "Accept") || strings.EqualFold(k, "Content-Type") {
			continue
		}
		return true
	}
	return false
}

func normalizeBaseURL(raw string) string {
	parsed, err := url.Parse(raw)
	if err != nil || parsed.Scheme == "" || parsed.Host == "" {
		return strings.TrimRight(raw, "/")
	}
	parsed.RawQuery = ""
	parsed.Fragment = ""
	return strings.TrimRight(parsed.String(), "/")
}

func hasAuthorizationHeader(headers map[string]string) bool {
	for k, v := range headers {
		if strings.EqualFold(k, "Authorization") && strings.TrimSpace(v) != "" {
			return true
		}
	}
	return false
}

func withRedirectsDisabled(in *http.Client) *http.Client {
	out := *in
	out.CheckRedirect = func(req *http.Request, via []*http.Request) error {
		return http.ErrUseLastResponse
	}
	return &out
}

func withRedirectSecurityChecks(in *http.Client, shouldBlock func(string) bool) *http.Client {
	out := *in
	previous := in.CheckRedirect
	out.CheckRedirect = func(req *http.Request, via []*http.Request) error {
		// Match net/http default redirect ceiling.
		if len(via) >= 10 {
			return fmt.Errorf("stopped after 10 redirects")
		}
		if shouldBlock(req.URL.String()) {
			return fmt.Errorf("refusing to send request over insecure transport")
		}
		if previous != nil {
			return previous(req, via)
		}
		return nil
	}
	return &out
}

func nonNilContext(ctx context.Context) context.Context {
	if ctx == nil {
		return context.Background()
	}
	return ctx
}

func readBodyWithLimit(body io.Reader, limit int64) ([]byte, bool, error) {
	readLimit := limit
	canDetectOverflow := limit < maxInt64
	if canDetectOverflow {
		readLimit = limit + 1
	}
	reader := io.LimitReader(body, readLimit)
	payload, err := io.ReadAll(reader)
	if err != nil {
		return nil, false, err
	}
	if canDetectOverflow && int64(len(payload)) > limit {
		return payload[:limit], true, nil
	}
	return payload, false, nil
}

func rejectCRLF(value string, fieldName string) (string, error) {
	if strings.ContainsAny(value, "\r\n") {
		return "", fmt.Errorf("%s must not contain CR or LF characters", fieldName)
	}
	return value, nil
}

func isJSONContentType(contentType string) bool {
	mediaType, _, err := mime.ParseMediaType(contentType)
	if err != nil {
		mediaType = strings.TrimSpace(strings.Split(contentType, ";")[0])
	}
	mediaType = strings.ToLower(mediaType)
	return mediaType == "application/json" || strings.HasSuffix(mediaType, "+json")
}
