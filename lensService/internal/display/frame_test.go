package display

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestFramePublishesCompleteScreenAndPreservesOrdinaryAnswers(t *testing.T) {
	h := New()
	ch, cancel := h.subscribe()
	defer cancel()
	r := httptest.NewRequest(http.MethodPost, "/", strings.NewReader(`{"rows":["Привет, ёж!","","","","",""],"positionMs":12345}`))
	r.RemoteAddr = "127.0.0.1:54321"
	r.Header.Set("Content-Type", "application/json")
	w := httptest.NewRecorder()
	h.Frame(w, r)
	if w.Code != http.StatusNoContent {
		t.Fatalf("frame: %d %s", w.Code, w.Body.String())
	}
	if got := <-ch; got != "\x1eOSF1 12345\nПривет, ёж!\n\n\n\n\n" {
		t.Fatalf("bad wire frame: %q", got)
	}
	h.Publish("Обычный ответ")
	if got := <-ch; got != "Обычный ответ" {
		t.Fatalf("normal answer: %q", got)
	}
}

func TestInvalidFramesDoNotPublish(t *testing.T) {
	cases := []struct {
		name, body, remote, content string
		status                      int
	}{
		{"LAN", `{"rows":["","","","","",""]}`, "192.168.1.104:1", "application/json", 403},
		{"form", `{}`, "127.0.0.1:1", "text/plain", 415},
		{"missing rows", `{"rows":[]}`, "127.0.0.1:1", "application/json", 400},
		{"long row", `{"rows":["` + strings.Repeat("Я", 22) + `","","","","",""]}`, "127.0.0.1:1", "application/json", 400},
		{"newlines", `{"rows":["a\nb","","","","",""]}`, "127.0.0.1:1", "application/json", 400},
		{"two objects", `{"rows":["","","","","",""]} {}`, "127.0.0.1:1", "application/json", 400},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			h := New()
			ch, cancel := h.subscribe()
			defer cancel()
			r := httptest.NewRequest("POST", "/", strings.NewReader(tc.body))
			r.RemoteAddr = tc.remote
			r.Header.Set("Content-Type", tc.content)
			w := httptest.NewRecorder()
			h.Frame(w, r)
			if w.Code != tc.status {
				t.Fatalf("got %d: %s", w.Code, w.Body.String())
			}
			select {
			case <-ch:
				t.Fatal("invalid frame published")
			default:
			}
		})
	}
}
