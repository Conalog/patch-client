package patchclient

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
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
