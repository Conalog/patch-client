package patchclient

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"regexp"
	"strings"
	"time"
)

var idempotencyKeyPattern = regexp.MustCompile(`^[A-Za-z0-9._:-]{8,128}$`)

type FieldworkAttachment struct {
	WorkID, ParentKind, ParentID, Operation, CommandKey, Filename string
	Ordinal                                                       int
	File                                                          io.Reader
}

func (c *Client) fieldworkCommand(ctx context.Context, path string, payload any, opts *RequestOptions) (any, error) {
	key := c.mergeHeaders(opts)["Idempotency-Key"]
	if !idempotencyKeyPattern.MatchString(key) {
		return nil, fmt.Errorf("Idempotency-Key must match %s", idempotencyKeyPattern.String())
	}
	if payload == nil {
		return nil, fmt.Errorf("command body is required")
	}
	return c.doJSONFieldwork(ctx, http.MethodPost, path, nil, payload, opts)
}

func (c *Client) fieldworkPost(ctx context.Context, path string, payload any, opts *RequestOptions) (any, error) {
	return c.doJSONFieldwork(ctx, http.MethodPost, path, nil, payload, opts)
}

func (c *Client) fieldworkGet(ctx context.Context, path string, query map[string]string, opts *RequestOptions) (any, error) {
	return c.doJSONFieldwork(ctx, http.MethodGet, path, query, nil, opts)
}

func (c *Client) FieldworkUploadAttachment(ctx context.Context, attachment FieldworkAttachment, opts *RequestOptions) (any, error) {
	if attachment.WorkID == "" || attachment.ParentKind == "" || attachment.ParentID == "" || attachment.Operation == "" || !idempotencyKeyPattern.MatchString(attachment.CommandKey) || attachment.Ordinal < 0 || attachment.Ordinal > 99 {
		return nil, fmt.Errorf("invalid fieldwork attachment")
	}
	return c.doMultipart(ctx, "/api/v3/fieldwork/attachments", map[string]string{"work_id": attachment.WorkID, "parent_kind": attachment.ParentKind, "parent_id": attachment.ParentID, "operation": attachment.Operation, "command_key": attachment.CommandKey, "ordinal": fmt.Sprint(attachment.Ordinal)}, "file", attachment.Filename, attachment.File, opts, true)
}

func (c *Client) FieldworkAttachmentContent(ctx context.Context, workID, objectKey string, opts *RequestOptions) ([]byte, error) {
	return c.fieldworkBytes(ctx, "/api/v3/fieldwork/attachments/content", map[string]string{"work_id": workID, "object_key": objectKey}, opts, true)
}
func (c *Client) FieldworkAttachmentDownload(ctx context.Context, workID, objectKey, expires, signature string, opts *RequestOptions) ([]byte, error) {
	if workID == "" || objectKey == "" || expires == "" || signature == "" {
		return nil, fmt.Errorf("work_id, object_key, expires, and signature are required")
	}
	return c.fieldworkBytes(ctx, "/api/v3/fieldwork/attachments/download", map[string]string{"work_id": workID, "object_key": objectKey, "expires": expires, "signature": signature}, opts, false)
}
func (c *Client) FieldworkReceiptGet(ctx context.Context, operation, commandKey string, query map[string]string, opts *RequestOptions) (any, error) {
	if operation == "" || !idempotencyKeyPattern.MatchString(commandKey) {
		return nil, fmt.Errorf("operation and a valid command_key are required")
	}
	q := cloneMap(query)
	q["operation"], q["command_key"] = operation, commandKey
	return c.fieldworkGet(ctx, "/api/v3/fieldwork/command-receipts", q, opts)
}

func (c *Client) fieldworkBytes(ctx context.Context, path string, query map[string]string, opts *RequestOptions, authenticated bool) ([]byte, error) {
	if authenticated {
		return c.doBytesWithAuth(ctx, http.MethodGet, path, query, opts, true, true)
	}
	return c.doBytesWithAuth(ctx, http.MethodGet, path, query, opts, false, false)
}
func (c *Client) FieldworkMessagePreviews(ctx context.Context, workIDs []string, opts *RequestOptions) (any, error) {
	return c.fieldworkGet(ctx, "/api/v3/fieldwork/latest-message-previews", map[string]string{"work_id": strings.Join(workIDs, ",")}, opts)
}
func (c *Client) FieldworkNotificationsList(ctx context.Context, query map[string]string, opts *RequestOptions) (any, error) {
	return c.fieldworkGet(ctx, "/api/v3/fieldwork/notifications", query, opts)
}
func (c *Client) FieldworkNotificationsSummary(ctx context.Context, query map[string]string, opts *RequestOptions) (any, error) {
	return c.fieldworkGet(ctx, "/api/v3/fieldwork/notifications/summary", query, opts)
}
func (c *Client) FieldworkNotificationArchive(ctx context.Context, notificationID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkPost(ctx, "/api/v3/fieldwork/notifications/"+encodePath(notificationID)+"/archive", payload, opts)
}
func (c *Client) FieldworkNotificationRead(ctx context.Context, notificationID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkPost(ctx, "/api/v3/fieldwork/notifications/"+encodePath(notificationID)+"/read", payload, opts)
}
func (c *Client) FieldworkParticipantSessionCreate(ctx context.Context, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkPost(ctx, "/api/v3/fieldwork/participant-sessions", payload, opts)
}
func (c *Client) FieldworkPlantMemberCandidates(ctx context.Context, plantID string, query map[string]string, opts *RequestOptions) (any, error) {
	return c.fieldworkGet(ctx, "/api/v3/fieldwork/plants/"+encodePath(plantID)+"/member-candidates", query, opts)
}
func (c *Client) FieldworkSummaryGet(ctx context.Context, query map[string]string, opts *RequestOptions) (any, error) {
	return c.fieldworkGet(ctx, "/api/v3/fieldwork/summary", query, opts)
}
func (c *Client) FieldworkTemplatesList(ctx context.Context, query map[string]string, opts *RequestOptions) (any, error) {
	return c.fieldworkGet(ctx, "/api/v3/fieldwork/templates", query, opts)
}
func (c *Client) FieldworkWorksList(ctx context.Context, query map[string]string, opts *RequestOptions) (any, error) {
	return c.fieldworkGet(ctx, "/api/v3/fieldwork/works", query, opts)
}
func (c *Client) FieldworkWorkGet(ctx context.Context, workID string, query map[string]string, opts *RequestOptions) (any, error) {
	return c.fieldworkGet(ctx, "/api/v3/fieldwork/works/"+encodePath(workID), query, opts)
}
func (c *Client) FieldworkResourcesList(ctx context.Context, workID string, query map[string]string, opts *RequestOptions) (any, error) {
	return c.fieldworkGet(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/resources", query, opts)
}

func (c *Client) FieldworkWorkCreate(ctx context.Context, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkCommand(ctx, "/api/v3/fieldwork/works", payload, opts)
}
func (c *Client) FieldworkWorkArchive(ctx context.Context, workID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkCommand(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/archive", payload, opts)
}
func (c *Client) FieldworkWorkClone(ctx context.Context, workID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkCommand(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/clone", payload, opts)
}
func (c *Client) FieldworkWorkClose(ctx context.Context, workID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkCommand(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/close", payload, opts)
}
func (c *Client) FieldworkWorkReopen(ctx context.Context, workID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkCommand(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/reopen", payload, opts)
}
func (c *Client) FieldworkWorkUpdate(ctx context.Context, workID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkCommand(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/update", payload, opts)
}
func (c *Client) FieldworkWorkSeen(ctx context.Context, workID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkPost(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/seen", payload, opts)
}
func (c *Client) FieldworkItemCreate(ctx context.Context, workID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkCommand(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/items", payload, opts)
}
func (c *Client) FieldworkItemComplete(ctx context.Context, workID, itemID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkItemCommand(ctx, workID, itemID, "/complete", payload, opts)
}
func (c *Client) FieldworkItemMapReferenceAdd(ctx context.Context, workID, itemID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkItemCommand(ctx, workID, itemID, "/map-references/add", payload, opts)
}
func (c *Client) FieldworkItemMapReferencesSync(ctx context.Context, workID, itemID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkItemCommand(ctx, workID, itemID, "/map-references/sync", payload, opts)
}
func (c *Client) FieldworkItemMapReferenceRemove(ctx context.Context, workID, itemID, mapRefID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkItemCommand(ctx, workID, itemID, "/map-references/"+encodePath(mapRefID)+"/remove", payload, opts)
}
func (c *Client) FieldworkItemMentionsSet(ctx context.Context, workID, itemID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkItemCommand(ctx, workID, itemID, "/mentions/set", payload, opts)
}
func (c *Client) FieldworkItemMove(ctx context.Context, workID, itemID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkItemCommand(ctx, workID, itemID, "/move", payload, opts)
}
func (c *Client) FieldworkItemPhotosAdd(ctx context.Context, workID, itemID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkItemCommand(ctx, workID, itemID, "/photos/add", payload, opts)
}
func (c *Client) FieldworkItemPhotoMove(ctx context.Context, workID, itemID, photoID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkItemCommand(ctx, workID, itemID, "/photos/"+encodePath(photoID)+"/move", payload, opts)
}
func (c *Client) FieldworkItemPhotoRemove(ctx context.Context, workID, itemID, photoID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkItemCommand(ctx, workID, itemID, "/photos/"+encodePath(photoID)+"/remove", payload, opts)
}
func (c *Client) FieldworkItemRemove(ctx context.Context, workID, itemID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkItemCommand(ctx, workID, itemID, "/remove", payload, opts)
}
func (c *Client) FieldworkItemReopen(ctx context.Context, workID, itemID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkItemCommand(ctx, workID, itemID, "/reopen", payload, opts)
}
func (c *Client) FieldworkItemUpdate(ctx context.Context, workID, itemID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkItemCommand(ctx, workID, itemID, "/update", payload, opts)
}
func (c *Client) fieldworkItemCommand(ctx context.Context, workID, itemID, suffix string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkCommand(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/items/"+encodePath(itemID)+suffix, payload, opts)
}

func (c *Client) FieldworkMemberInvite(ctx context.Context, workID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkPost(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/members/invite", payload, opts)
}
func (c *Client) FieldworkMemberJoin(ctx context.Context, workID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkPost(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/members/self/join", payload, opts)
}
func (c *Client) FieldworkMemberLeave(ctx context.Context, workID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkPost(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/members/self/leave", payload, opts)
}
func (c *Client) FieldworkMemberRemove(ctx context.Context, workID, memberID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkPost(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/members/"+encodePath(memberID)+"/remove", payload, opts)
}
func (c *Client) FieldworkMemberResponsibleAdd(ctx context.Context, workID, memberID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkPost(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/members/"+encodePath(memberID)+"/responsible/add", payload, opts)
}
func (c *Client) FieldworkMemberResponsibleRemove(ctx context.Context, workID, memberID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkPost(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/members/"+encodePath(memberID)+"/responsible/remove", payload, opts)
}

func (c *Client) FieldworkMessageCreate(ctx context.Context, workID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkCommand(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/messages", payload, opts)
}
func (c *Client) FieldworkMessagesRead(ctx context.Context, workID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkCommand(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/messages/read", payload, opts)
}
func (c *Client) FieldworkMessageGet(ctx context.Context, workID, messageID string, opts *RequestOptions) (any, error) {
	return c.fieldworkGet(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/messages/"+encodePath(messageID), nil, opts)
}
func (c *Client) FieldworkMessageAttachmentRemove(ctx context.Context, workID, messageID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkMessageCommand(ctx, workID, messageID, "/attachments/remove", payload, opts)
}
func (c *Client) FieldworkMessageMapReferenceAdd(ctx context.Context, workID, messageID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkMessageCommand(ctx, workID, messageID, "/map-references/add", payload, opts)
}
func (c *Client) FieldworkMessageMapReferenceRemove(ctx context.Context, workID, messageID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkMessageCommand(ctx, workID, messageID, "/map-references/remove", payload, opts)
}
func (c *Client) FieldworkMessageMapReferencesSync(ctx context.Context, workID, messageID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkMessageCommand(ctx, workID, messageID, "/map-references/sync", payload, opts)
}
func (c *Client) FieldworkMessagePhotosAdd(ctx context.Context, workID, messageID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkMessageCommand(ctx, workID, messageID, "/photos/add", payload, opts)
}
func (c *Client) FieldworkMessagePhotoMove(ctx context.Context, workID, messageID, photoID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkMessageCommand(ctx, workID, messageID, "/photos/"+encodePath(photoID)+"/move", payload, opts)
}
func (c *Client) FieldworkMessagePhotoRemove(ctx context.Context, workID, messageID, photoID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkMessageCommand(ctx, workID, messageID, "/photos/"+encodePath(photoID)+"/remove", payload, opts)
}
func (c *Client) FieldworkReactionRemove(ctx context.Context, workID, messageID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkMessageCommand(ctx, workID, messageID, "/reaction/remove", payload, opts)
}
func (c *Client) FieldworkReactionSet(ctx context.Context, workID, messageID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkMessageCommand(ctx, workID, messageID, "/reaction/set", payload, opts)
}
func (c *Client) FieldworkMessageRemove(ctx context.Context, workID, messageID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkMessageCommand(ctx, workID, messageID, "/remove", payload, opts)
}
func (c *Client) FieldworkMessageUpdate(ctx context.Context, workID, messageID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkMessageCommand(ctx, workID, messageID, "/update", payload, opts)
}
func (c *Client) fieldworkMessageCommand(ctx context.Context, workID, messageID, suffix string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkCommand(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/messages/"+encodePath(messageID)+suffix, payload, opts)
}

func (c *Client) FieldworkScheduleUpdate(ctx context.Context, workID, scheduleID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkCommand(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/schedules/"+encodePath(scheduleID)+"/update", payload, opts)
}
func (c *Client) FieldworkSectionCreate(ctx context.Context, workID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkCommand(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/sections", payload, opts)
}
func (c *Client) FieldworkSectionMove(ctx context.Context, workID, sectionID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkSectionCommand(ctx, workID, sectionID, "/move", payload, opts)
}
func (c *Client) FieldworkSectionRemove(ctx context.Context, workID, sectionID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkSectionCommand(ctx, workID, sectionID, "/remove", payload, opts)
}
func (c *Client) FieldworkSectionRename(ctx context.Context, workID, sectionID string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkSectionCommand(ctx, workID, sectionID, "/rename", payload, opts)
}
func (c *Client) fieldworkSectionCommand(ctx context.Context, workID, sectionID, suffix string, payload any, opts *RequestOptions) (any, error) {
	return c.fieldworkCommand(ctx, "/api/v3/fieldwork/works/"+encodePath(workID)+"/sections/"+encodePath(sectionID)+suffix, payload, opts)
}

func (c *Client) FieldworkEvents(ctx context.Context, query map[string]string, opts *RequestOptions) (io.ReadCloser, error) {
	if query["watch"] == "unread" && (query["work_id"] != "" || query["surface"] != "") {
		return nil, fmt.Errorf("watch=unread cannot include work_id or surface")
	}
	target, err := c.buildURL("/api/v3/fieldwork/events", query)
	if err != nil {
		return nil, err
	}
	streamCtx, cancel := context.WithCancel(nonNilContext(ctx))
	timer := time.AfterFunc(c.streamSetupTimeout(), cancel)
	fail := func(err error) (io.ReadCloser, error) { timer.Stop(); cancel(); return nil, err }
	req, err := http.NewRequestWithContext(streamCtx, http.MethodGet, target, nil)
	if err != nil {
		return fail(err)
	}
	headers := c.mergeHeaders(opts)
	if opts != nil && opts.OmitAccountType {
		delete(headers, "Account-Type")
	}
	headers["Accept"] = "text/event-stream"
	for k, v := range headers {
		req.Header.Set(k, v)
	}
	if c.shouldBlockInsecureRequest(target) {
		return fail(fmt.Errorf("refusing to send request over insecure transport"))
	}
	resp, err := withStreamTimeoutDisabled(c.httpClient()).Do(req)
	if err != nil {
		return fail(err)
	}
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		payload, _, readErr := readBodyWithLimit(resp.Body, c.responseLimit())
		resp.Body.Close()
		if readErr != nil {
			return fail(readErr)
		}
		return fail(&PatchClientError{Method: http.MethodGet, URL: target, StatusCode: resp.StatusCode, Body: string(payload)})
	}
	if !isEventStreamContentType(resp.Header.Get("Content-Type")) {
		resp.Body.Close()
		return fail(fmt.Errorf("expected text/event-stream response"))
	}
	if !timer.Stop() {
		resp.Body.Close()
		return fail(fmt.Errorf("SSE setup timed out"))
	}
	return &cancellableReadCloser{ReadCloser: resp.Body, cancel: cancel}, nil
}

type cancellableReadCloser struct {
	io.ReadCloser
	cancel context.CancelFunc
}

func (body *cancellableReadCloser) Read(p []byte) (int, error) {
	n, err := body.ReadCloser.Read(p)
	if err != nil {
		body.cancel()
	}
	return n, err
}

func (body *cancellableReadCloser) Close() error {
	body.cancel()
	return body.ReadCloser.Close()
}

func (c *Client) streamSetupTimeout() time.Duration {
	timeout := c.httpClient().Timeout
	if timeout <= 0 {
		return 30 * time.Second
	}
	return timeout
}
