package patchclient

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
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

func TestEscapeQuotesUsesSingleBackslashForDoubleQuotes(t *testing.T) {
	got := escapeQuotes("a\"b\\c")
	want := "a\\\"b\\\\c"
	if got != want {
		t.Fatalf("unexpected escaped value: got %q want %q", got, want)
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
