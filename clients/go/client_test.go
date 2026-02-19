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

func TestUploadPlantFilesContentTypeOverrideIsCaseInsensitive(t *testing.T) {
	var gotContentType string
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotContentType = r.Header.Get("Content-Type")
		_ = json.NewEncoder(w).Encode(map[string]any{"ok": true})
	}))
	defer srv.Close()

	client := NewClient(srv.URL)
	_, err := client.UploadPlantFiles(
		context.Background(),
		"plant-1",
		map[string]string{"name": "file.txt"},
		map[string]FilePart{
			"filename": {
				Filename:    "file.txt",
				ContentType: "text/plain",
				Content:     []byte("hello"),
			},
		},
		&RequestOptions{
			Headers: map[string]string{
				"content-type": "application/json",
			},
		},
	)
	if err != nil {
		t.Fatalf("UploadPlantFiles returned error: %v", err)
	}
	if !strings.HasPrefix(strings.ToLower(gotContentType), "multipart/form-data; boundary=") {
		t.Fatalf("unexpected content type: %s", gotContentType)
	}
}

func TestEncodeMultipartRejectsCRLFInFieldName(t *testing.T) {
	_, _, err := encodeMultipart(map[string]string{"name\r\nX:1": "v"}, nil, 1024)
	if err == nil {
		t.Fatal("expected CRLF validation error, got nil")
	}
}

func TestEncodeMultipartRejectsCRLFInFilename(t *testing.T) {
	_, _, err := encodeMultipart(
		map[string]string{"name": "f"},
		map[string]FilePart{
			"filename": {
				Filename:    "x\r\nInjected: 1",
				ContentType: "text/plain",
				Content:     []byte("hello"),
			},
		},
		1024,
	)
	if err == nil {
		t.Fatal("expected CRLF validation error, got nil")
	}
}

func TestUploadPlantFilesRejectsPayloadAboveConfiguredLimit(t *testing.T) {
	client := NewClient("https://example.com")
	client.SetMaxMultipartBytes(32)

	_, err := client.UploadPlantFiles(
		context.Background(),
		"plant-1",
		map[string]string{"name": "file.txt"},
		map[string]FilePart{
			"filename": {
				Filename:    "file.txt",
				ContentType: "application/octet-stream",
				Content:     []byte(strings.Repeat("a", 128)),
			},
		},
		nil,
	)
	if err == nil {
		t.Fatal("expected multipart size limit error, got nil")
	}
	if !strings.Contains(err.Error(), "multipart payload exceeds") {
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
