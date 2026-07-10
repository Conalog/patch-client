package patchclient

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync"
	"testing"
)

func TestGetPlantListBuildsV3PathAndHeaders(t *testing.T) {
	var gotAuth string
	var gotAccountType string
	var gotPath string
	var gotQuery string

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotAuth = r.Header.Get("Authorization")
		gotAccountType = r.Header.Get("Account-Type")
		gotPath = r.URL.Path
		gotQuery = r.URL.RawQuery
		_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
	}))
	defer srv.Close()

	client := NewClient(srv.URL)
	client.SetAccessToken("token-value")
	client.SetAccountType(AccountTypeManager)

	_, err := client.GetPlantList(context.Background(), map[string]string{"page": "1", "size": "20"}, nil)
	if err != nil {
		t.Fatalf("GetPlantList returned error: %v", err)
	}

	if gotPath != "/api/v3/plants" {
		t.Fatalf("unexpected path: %s", gotPath)
	}
	if gotQuery != "page=1&size=20" && gotQuery != "size=20&page=1" {
		t.Fatalf("unexpected query: %s", gotQuery)
	}
	if gotAuth != "Bearer token-value" {
		t.Fatalf("unexpected Authorization header: %s", gotAuth)
	}
	if gotAccountType != "manager" {
		t.Fatalf("unexpected Account-Type header: %s", gotAccountType)
	}
}

func TestGetPlantDetailsPreservesEscapedPathSegment(t *testing.T) {
	var gotRequestURI string

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotRequestURI = r.RequestURI
		_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
	}))
	defer srv.Close()

	client := NewClient(srv.URL)
	_, err := client.GetPlantDetails(context.Background(), "unit/a", nil)
	if err != nil {
		t.Fatalf("GetPlantDetails returned error: %v", err)
	}

	if gotRequestURI != "/api/v3/plants/unit%2Fa" {
		t.Fatalf("unexpected request URI: %s", gotRequestURI)
	}
}

func TestSetDefaultHeaderIsApplied(t *testing.T) {
	var gotCustom string

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotCustom = r.Header.Get("X-Custom")
		_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
	}))
	defer srv.Close()

	client := NewClient(srv.URL)
	client.SetDefaultHeader("X-Custom", "yes")

	_, err := client.GetPlantList(context.Background(), nil, nil)
	if err != nil {
		t.Fatalf("GetPlantList returned error: %v", err)
	}

	if gotCustom != "yes" {
		t.Fatalf("unexpected custom header: %s", gotCustom)
	}
}

func TestGetPlantListAcceptsNilContext(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
	}))
	defer srv.Close()

	client := NewClient(srv.URL)
	_, err := client.GetPlantList(nil, nil, nil)
	if err != nil {
		t.Fatalf("GetPlantList returned error with nil context: %v", err)
	}
}

func TestGetPlantListWithNilHTTPClientUsesFallback(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
	}))
	defer srv.Close()

	client := NewClient(srv.URL)
	client.HTTPClient = nil

	_, err := client.GetPlantList(context.Background(), nil, nil)
	if err != nil {
		t.Fatalf("GetPlantList returned error with nil HTTPClient: %v", err)
	}
}

func TestDoJSONParsesProblemJSONContentType(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/problem+json")
		_ = json.NewEncoder(w).Encode(map[string]any{"error": "bad request"})
	}))
	defer srv.Close()

	client := NewClient(srv.URL)
	out, err := client.GetPlantList(context.Background(), nil, nil)
	if err != nil {
		t.Fatalf("GetPlantList returned error: %v", err)
	}

	got, ok := out.(map[string]any)
	if !ok {
		t.Fatalf("expected map response, got %T (%v)", out, out)
	}
	if got["error"] != "bad request" {
		t.Fatalf("unexpected error value: %v", got["error"])
	}
}

func TestDoJSONFailsWhenResponseExceedsLimit(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(strings.Repeat("a", 64)))
	}))
	defer srv.Close()

	client := NewClient(srv.URL)
	client.SetMaxResponseBytes(16)

	_, err := client.GetPlantList(context.Background(), nil, nil)
	if err == nil {
		t.Fatal("expected size limit error, got nil")
	}
	if !strings.Contains(err.Error(), "response body exceeds") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestDoJSONNon2xxKeepsPatchClientErrorWhenBodyExceedsLimit(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte(strings.Repeat("x", 64)))
	}))
	defer srv.Close()

	client := NewClient(srv.URL)
	client.SetMaxResponseBytes(16)

	_, err := client.GetPlantList(context.Background(), nil, nil)
	if err == nil {
		t.Fatal("expected error, got nil")
	}
	patchErr, ok := err.(*PatchClientError)
	if !ok {
		t.Fatalf("expected PatchClientError, got %T (%v)", err, err)
	}
	if patchErr.StatusCode != http.StatusBadRequest {
		t.Fatalf("unexpected status code: %d", patchErr.StatusCode)
	}
	if len(patchErr.Body) != 16 {
		t.Fatalf("expected truncated body length 16, got %d", len(patchErr.Body))
	}
}

func TestReadBodyWithLimitHandlesMaxInt64WithoutOverflow(t *testing.T) {
	payload, overflowed, err := readBodyWithLimit(strings.NewReader("abc"), maxInt64)
	if err != nil {
		t.Fatalf("readBodyWithLimit returned error: %v", err)
	}
	if overflowed {
		t.Fatal("unexpected overflow marker for small body with max limit")
	}
	if string(payload) != "abc" {
		t.Fatalf("unexpected payload: %q", string(payload))
	}
}

func TestLatestV3OperationPaths(t *testing.T) {
	ctx := context.Background()
	payload := map[string]any{"ok": true}
	tests := []struct {
		name   string
		method string
		path   string
		query  string
		call   func(*Client) (any, error)
	}{
		{
			name:   "ListOAuthMethods",
			method: http.MethodGet,
			path:   "/api/v3/account/auth-methods",
			query:  "provider=google",
			call: func(c *Client) (any, error) {
				return c.ListOAuthMethods(ctx, map[string]string{"provider": "google"}, nil)
			},
		},
		{
			name:   "ListCombinerModelInfo",
			method: http.MethodGet,
			path:   "/api/v3/model-info/combiners",
			call: func(c *Client) (any, error) {
				return c.ListCombinerModelInfo(ctx, nil)
			},
		},
		{
			name:   "ListInverterModelInfo",
			method: http.MethodGet,
			path:   "/api/v3/model-info/inverters",
			call: func(c *Client) (any, error) {
				return c.ListInverterModelInfo(ctx, nil)
			},
		},
		{
			name:   "ListModuleModelInfo",
			method: http.MethodGet,
			path:   "/api/v3/model-info/modules",
			call: func(c *Client) (any, error) {
				return c.ListModuleModelInfo(ctx, nil)
			},
		},
		{
			name:   "AssignPlantPermission",
			method: http.MethodPost,
			path:   "/api/v3/organizations/org%2F1/plants/plant%2F1/permissions/grant",
			call: func(c *Client) (any, error) {
				return c.AssignPlantPermission(ctx, "org/1", "plant/1", payload, nil)
			},
		},
		{
			name:   "RemovePlantPermission",
			method: http.MethodPost,
			path:   "/api/v3/organizations/org%2F1/plants/plant%2F1/permissions/revoke",
			call: func(c *Client) (any, error) {
				return c.RemovePlantPermission(ctx, "org/1", "plant/1", payload, nil)
			},
		},
		{
			name:   "ListPlantBlueprints",
			method: http.MethodGet,
			path:   "/api/v3/plants/plant%2F1/blueprints",
			call: func(c *Client) (any, error) {
				return c.ListPlantBlueprints(ctx, "plant/1", nil)
			},
		},
		{
			name:   "RecordPlantBlueprint",
			method: http.MethodPost,
			path:   "/api/v3/plants/plant%2F1/blueprints/record",
			call: func(c *Client) (any, error) {
				return c.RecordPlantBlueprint(ctx, "plant/1", payload, nil)
			},
		},
		{
			name:   "GetPlantBlueprintData",
			method: http.MethodGet,
			path:   "/api/v3/plants/plant%2F1/blueprints/bp%2F1",
			call: func(c *Client) (any, error) {
				return c.GetPlantBlueprintData(ctx, "plant/1", "bp/1", nil)
			},
		},
		{
			name:   "ListPlantComments",
			method: http.MethodGet,
			path:   "/api/v3/plants/plant%2F1/comments",
			call: func(c *Client) (any, error) {
				return c.ListPlantComments(ctx, "plant/1", nil)
			},
		},
		{
			name:   "StartPlantCommentThread",
			method: http.MethodPost,
			path:   "/api/v3/plants/plant%2F1/comments/start_thread",
			call: func(c *Client) (any, error) {
				return c.StartPlantCommentThread(ctx, "plant/1", payload, nil)
			},
		},
		{
			name:   "EditPlantComment",
			method: http.MethodPost,
			path:   "/api/v3/plants/plant%2F1/comments/comment%2F1/edit",
			call: func(c *Client) (any, error) {
				return c.EditPlantComment(ctx, "plant/1", "comment/1", payload, nil)
			},
		},
		{
			name:   "ReplyPlantComment",
			method: http.MethodPost,
			path:   "/api/v3/plants/plant%2F1/comments/comment%2F1/reply",
			call: func(c *Client) (any, error) {
				return c.ReplyPlantComment(ctx, "plant/1", "comment/1", payload, nil)
			},
		},
		{
			name:   "ChangePlantCommentState",
			method: http.MethodPost,
			path:   "/api/v3/plants/plant%2F1/comments/comment%2F1/state",
			call: func(c *Client) (any, error) {
				return c.ChangePlantCommentState(ctx, "plant/1", "comment/1", payload, nil)
			},
		},
		{
			name:   "ListPlantFilters",
			method: http.MethodGet,
			path:   "/api/v3/plants/plant%2F1/filters",
			call: func(c *Client) (any, error) {
				return c.ListPlantFilters(ctx, "plant/1", nil)
			},
		},
		{
			name:   "CreatePlantFilter",
			method: http.MethodPost,
			path:   "/api/v3/plants/plant%2F1/filters/create",
			call: func(c *Client) (any, error) {
				return c.CreatePlantFilter(ctx, "plant/1", payload, nil)
			},
		},
		{
			name:   "DeletePlantFilter",
			method: http.MethodDelete,
			path:   "/api/v3/plants/plant%2F1/filters/filter%2F1",
			call: func(c *Client) (any, error) {
				return c.DeletePlantFilter(ctx, "plant/1", "filter/1", nil)
			},
		},
		{
			name:   "RenamePlantFilter",
			method: http.MethodPost,
			path:   "/api/v3/plants/plant%2F1/filters/filter%2F1/rename",
			call: func(c *Client) (any, error) {
				return c.RenamePlantFilter(ctx, "plant/1", "filter/1", payload, nil)
			},
		},
		{
			name:   "GetPlantAnomalyTimeline",
			method: http.MethodGet,
			path:   "/api/v3/plants/plant%2F1/indicator/anomaly",
			query:  "date=2026-06-15",
			call: func(c *Client) (any, error) {
				return c.GetPlantAnomalyTimeline(ctx, "plant/1", "2026-06-15", nil)
			},
		},
		{
			name:   "GetPlantAnomalyLogs",
			method: http.MethodGet,
			path:   "/api/v3/plants/plant%2F1/indicator/anomaly/logs",
			query:  "date=2026-06-15&severity=high",
			call: func(c *Client) (any, error) {
				return c.GetPlantAnomalyLogs(ctx, "plant/1", "2026-06-15", map[string]string{"severity": "high"}, nil)
			},
		},
		{
			name:   "FilterPlantAnomalyLogs",
			method: http.MethodGet,
			path:   "/api/v3/plants/plant%2F1/indicator/anomaly/logs/filter",
			query:  "date=2026-06-15&type=hotspot",
			call: func(c *Client) (any, error) {
				return c.FilterPlantAnomalyLogs(ctx, "plant/1", "2026-06-15", map[string]string{"type": "hotspot"}, nil)
			},
		},
		{
			name:   "GetPlantAnomalySnapshots",
			method: http.MethodGet,
			path:   "/api/v3/plants/plant%2F1/indicator/anomaly/snapshots",
			query:  "date=2026-06-15&map_id=map1",
			call: func(c *Client) (any, error) {
				return c.GetPlantAnomalySnapshots(ctx, "plant/1", "2026-06-15", map[string]string{"map_id": "map1"}, nil)
			},
		},
		{
			name:   "GetDeviceState",
			method: http.MethodGet,
			path:   "/api/v3/plants/plant%2F1/indicator/device-state",
			query:  "date=2026-06-15&fields=all",
			call: func(c *Client) (any, error) {
				return c.GetDeviceState(ctx, "plant/1", "2026-06-15", map[string]string{"fields": "all"}, nil)
			},
		},
		{
			name:   "GetPlantRegistryTimeline",
			method: http.MethodGet,
			path:   "/api/v3/plants/plant%2F1/registry",
			query:  "date=2024-01-24",
			call: func(c *Client) (any, error) {
				return c.GetPlantRegistryTimeline(ctx, "plant/1", "2024-01-24", nil)
			},
		},
		{
			name:   "GetPlantRegistryLogs",
			method: http.MethodGet,
			path:   "/api/v3/plants/plant%2F1/registry/logs",
			query:  "asset_id=a1&date=2024-01-24",
			call: func(c *Client) (any, error) {
				return c.GetPlantRegistryLogs(ctx, "plant/1", "2024-01-24", map[string]string{"asset_id": "a1"}, nil)
			},
		},
		{
			name:   "FilterPlantRegistryLogs",
			method: http.MethodGet,
			path:   "/api/v3/plants/plant%2F1/registry/logs/filter",
			query:  "asset_type=device",
			call: func(c *Client) (any, error) {
				return c.FilterPlantRegistryLogs(ctx, "plant/1", map[string]string{"asset_type": "device"}, nil)
			},
		},
		{
			name:   "RegisterAssetToPlant",
			method: http.MethodPost,
			path:   "/api/v3/plants/plant%2F1/registry/register",
			call: func(c *Client) (any, error) {
				return c.RegisterAssetToPlant(ctx, "plant/1", payload, nil)
			},
		},
		{
			name:   "GetPlantRegistrySnapshots",
			method: http.MethodGet,
			path:   "/api/v3/plants/plant%2F1/registry/snapshots",
			query:  "date=2024-01-24&map_id=m1",
			call: func(c *Client) (any, error) {
				return c.GetPlantRegistrySnapshots(ctx, "plant/1", "2024-01-24", map[string]string{"map_id": "m1"}, nil)
			},
		},
		{
			name:   "GetPlantRegistryStat",
			method: http.MethodGet,
			path:   "/api/v3/plants/plant%2F1/registry/stat",
			query:  "date=2024-01-24",
			call: func(c *Client) (any, error) {
				return c.GetPlantRegistryStat(ctx, "plant/1", "2024-01-24", nil)
			},
		},
		{
			name:   "UnregisterAssetFromPlant",
			method: http.MethodPost,
			path:   "/api/v3/plants/plant%2F1/registry/unregister",
			call: func(c *Client) (any, error) {
				return c.UnregisterAssetFromPlant(ctx, "plant/1", payload, nil)
			},
		},
		{
			name:   "GetPlantWeatherForecast",
			method: http.MethodGet,
			path:   "/api/v3/plants/plant%2F1/weather/forecast",
			query:  "days=7",
			call: func(c *Client) (any, error) {
				return c.GetPlantWeatherForecast(ctx, "plant/1", map[string]string{"days": "7"}, nil)
			},
		},
		{
			name:   "GetPlantWeatherObserved",
			method: http.MethodGet,
			path:   "/api/v3/plants/plant%2F1/weather/observed",
			query:  "before=1&date=2024-01-24",
			call: func(c *Client) (any, error) {
				return c.GetPlantWeatherObserved(ctx, "plant/1", "2024-01-24", map[string]string{"before": "1"}, nil)
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var gotMethod string
			var gotPath string
			var gotQuery string
			srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				gotMethod = r.Method
				gotPath = r.URL.EscapedPath()
				gotQuery = r.URL.RawQuery
				_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
			}))
			defer srv.Close()

			client := NewClient(srv.URL)
			if _, err := tt.call(client); err != nil {
				t.Fatalf("%s returned error: %v", tt.name, err)
			}
			if gotMethod != tt.method {
				t.Fatalf("unexpected method: got %s want %s", gotMethod, tt.method)
			}
			if gotPath != tt.path {
				t.Fatalf("unexpected path: got %s want %s", gotPath, tt.path)
			}
			if gotQuery != tt.query {
				t.Fatalf("unexpected query: got %s want %s", gotQuery, tt.query)
			}
		})
	}
}

func TestStartOAuthLoginReturnsRedirectLocation(t *testing.T) {
	var targetHits int
	target := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		targetHits++
		_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
	}))
	defer target.Close()

	var gotPath string
	var gotQuery string
	redirectSource := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.Path
		gotQuery = r.URL.RawQuery
		w.Header().Set("Location", target.URL+"/oauth")
		w.WriteHeader(http.StatusFound)
	}))
	defer redirectSource.Close()

	client := NewClient(redirectSource.URL)
	out, err := client.StartOAuthLogin(context.Background(), "google", "app://cb", nil)
	if err != nil {
		t.Fatalf("StartOAuthLogin returned error: %v", err)
	}
	if gotPath != "/api/v3/account/login-with-oauth2" {
		t.Fatalf("unexpected path: %s", gotPath)
	}
	if gotQuery != "provider=google&redirect_url=app%3A%2F%2Fcb" {
		t.Fatalf("unexpected query: %s", gotQuery)
	}
	if out.Location != target.URL+"/oauth" {
		t.Fatalf("unexpected redirect location: %s", out.Location)
	}
	if targetHits != 0 {
		t.Fatalf("expected redirect target not to be called, hits=%d", targetHits)
	}
}

func TestRequestOptionsAcceptHeaderOverridesDefault(t *testing.T) {
	var (
		mu        sync.Mutex
		gotAccept string
	)

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		mu.Lock()
		gotAccept = r.Header.Get("Accept")
		mu.Unlock()
		_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
	}))
	defer srv.Close()

	client := NewClient(srv.URL)
	opts := &RequestOptions{
		Headers: map[string]string{
			"Accept": "text/plain",
		},
	}
	_, err := client.GetPlantList(context.Background(), nil, opts)
	if err != nil {
		t.Fatalf("GetPlantList returned error: %v", err)
	}

	mu.Lock()
	defer mu.Unlock()
	if gotAccept != "text/plain" {
		t.Fatalf("unexpected Accept header: %s", gotAccept)
	}
}

func TestRequestOptionsLowercaseAcceptHeaderOverridesDefaultDeterministically(t *testing.T) {
	var (
		mu      sync.Mutex
		accepts []string
	)
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		mu.Lock()
		accepts = append(accepts, r.Header.Get("Accept"))
		mu.Unlock()
		_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
	}))
	defer srv.Close()

	client := NewClient(srv.URL)
	opts := &RequestOptions{
		Headers: map[string]string{
			"accept": "text/plain",
		},
	}

	for i := 0; i < 20; i++ {
		_, err := client.GetPlantList(context.Background(), nil, opts)
		if err != nil {
			t.Fatalf("GetPlantList returned error: %v", err)
		}
	}

	mu.Lock()
	defer mu.Unlock()
	for _, got := range accepts {
		if got != "text/plain" {
			t.Fatalf("unexpected Accept header: %s", got)
		}
	}
}

func TestDefaultHeadersDuplicateCaseDeterministic(t *testing.T) {
	var (
		mu      sync.Mutex
		accepts []string
	)
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		mu.Lock()
		accepts = append(accepts, r.Header.Get("Accept"))
		mu.Unlock()
		_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
	}))
	defer srv.Close()

	client := NewClient(srv.URL)
	client.SetDefaultHeaders(map[string]string{
		"Accept": "from-upper",
		"accept": "from-lower",
	})

	for i := 0; i < 20; i++ {
		_, err := client.GetPlantList(context.Background(), nil, nil)
		if err != nil {
			t.Fatalf("GetPlantList returned error: %v", err)
		}
	}

	mu.Lock()
	defer mu.Unlock()
	for _, got := range accepts {
		if got != "from-lower" {
			t.Fatalf("unexpected Accept header: %s", got)
		}
	}
}

func TestGetPlantListBlocksInsecureAuthorizationOnNonLoopback(t *testing.T) {
	client := NewClient("http://example.com")
	client.SetAccessToken("token-value")

	_, err := client.GetPlantList(context.Background(), nil, nil)
	if err == nil {
		t.Fatal("expected insecure transport error, got nil")
	}
	if !strings.Contains(err.Error(), "refusing to send request over insecure transport") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestAuthenticateUserBlocksInsecureHTTPWithoutAuthorizationHeader(t *testing.T) {
	client := NewClient("http://example.com")

	_, err := client.AuthenticateUser(context.Background(), map[string]any{
		"type":     "manager",
		"email":    "manager@example.com",
		"password": "pw",
	})
	if err == nil {
		t.Fatal("expected insecure transport error, got nil")
	}
	if !strings.Contains(err.Error(), "refusing to send request over insecure transport") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestAuthenticatedRequestsDoNotFollowRedirects(t *testing.T) {
	var (
		targetHits int
		targetAuth string
	)
	target := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		targetHits++
		targetAuth = r.Header.Get("Authorization")
		_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
	}))
	defer target.Close()

	redirectSource := httptest.NewTLSServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.Redirect(w, r, target.URL+"/from-redirect", http.StatusFound)
	}))
	defer redirectSource.Close()

	client := NewClient(redirectSource.URL)
	client.HTTPClient = redirectSource.Client()
	client.SetAccessToken("token-value")

	_, err := client.GetPlantList(context.Background(), nil, nil)
	if err == nil {
		t.Fatal("expected redirect response error, got nil")
	}
	patchErr, ok := err.(*PatchClientError)
	if !ok {
		t.Fatalf("expected PatchClientError, got %T (%v)", err, err)
	}
	if patchErr.StatusCode != http.StatusFound {
		t.Fatalf("unexpected status code: %d", patchErr.StatusCode)
	}
	if targetHits != 0 {
		t.Fatalf("expected redirect target not to be called, hits=%d auth=%q", targetHits, targetAuth)
	}
}

func TestBodyBearingRequestsDoNotFollowRedirects(t *testing.T) {
	var (
		targetHits int
	)
	target := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		targetHits++
		_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
	}))
	defer target.Close()

	redirectSource := httptest.NewTLSServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.Redirect(w, r, target.URL+"/from-redirect", http.StatusTemporaryRedirect)
	}))
	defer redirectSource.Close()

	client := NewClient(redirectSource.URL)
	client.HTTPClient = redirectSource.Client()

	_, err := client.AuthenticateUser(context.Background(), map[string]any{
		"type":     "manager",
		"email":    "manager@example.com",
		"password": "pw",
	})
	if err == nil {
		t.Fatal("expected redirect response error, got nil")
	}
	patchErr, ok := err.(*PatchClientError)
	if !ok {
		t.Fatalf("expected PatchClientError, got %T (%v)", err, err)
	}
	if patchErr.StatusCode != http.StatusTemporaryRedirect {
		t.Fatalf("unexpected status code: %d", patchErr.StatusCode)
	}
	if targetHits != 0 {
		t.Fatalf("expected redirect target not to be called, hits=%d", targetHits)
	}
}

func TestUnauthenticatedRequestsBlockHTTPSDowngradeRedirect(t *testing.T) {
	redirectSource := httptest.NewTLSServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.Redirect(w, r, "http://example.com/from-redirect", http.StatusFound)
	}))
	defer redirectSource.Close()

	client := NewClient(redirectSource.URL)
	client.HTTPClient = redirectSource.Client()

	_, err := client.GetPlantList(context.Background(), nil, nil)
	if err == nil {
		t.Fatal("expected insecure transport redirect error, got nil")
	}
	if !strings.Contains(err.Error(), "refusing to send request over insecure transport") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestAsBearerAcceptsLowercasePrefix(t *testing.T) {
	got := asBearer("bearer token-value")
	if got != "bearer token-value" {
		t.Fatalf("unexpected bearer token: %q", got)
	}
}

func TestPatchClientErrorOmitsBodyInErrorString(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte("invalid request payload"))
	}))
	defer srv.Close()

	client := NewClient(srv.URL)
	_, err := client.GetPlantList(context.Background(), nil, nil)
	if err == nil {
		t.Fatal("expected error, got nil")
	}

	if strings.Contains(err.Error(), "invalid request payload") {
		t.Fatalf("error message unexpectedly includes response body: %v", err)
	}
}

func TestPatchClientErrorBodySnippetTruncatesByRune(t *testing.T) {
	err := &PatchClientError{Body: "가나다라마바사아자차카타파하"}
	got := err.BodySnippet(5)
	want := "가나다라마..."
	if got != want {
		t.Fatalf("unexpected snippet: got %q want %q", got, want)
	}
}
