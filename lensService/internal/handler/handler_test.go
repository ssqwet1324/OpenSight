package handler

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/coder/websocket"
	"github.com/ssqwet1324/opensight/internal/display"
	"github.com/ssqwet1324/opensight/internal/entity"
)

type fakeUsecase struct {
	answer *entity.AskResponse
	err    error
}

func (f fakeUsecase) Ask(context.Context, entity.AskRequest) (*entity.AskResponse, error) {
	return f.answer, f.err
}

func TestAskReturnsJSONAndDeliversSameAnswer(t *testing.T) {
	hub := display.New()
	h := New(fakeUsecase{answer: &entity.AskResponse{Answer: "Очки получили ответ."}}, hub.Publish)
	mux := http.NewServeMux()
	mux.HandleFunc("POST /api/v1/ask", h.Ask)
	mux.Handle("GET /api/v1/ws", hub)
	s := httptest.NewServer(mux)
	defer s.Close()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	ws, _, err := websocket.Dial(ctx, "ws"+strings.TrimPrefix(s.URL, "http")+"/api/v1/ws", nil)
	if err != nil {
		t.Fatal(err)
	}
	defer ws.CloseNow()
	r, err := http.Post(s.URL+"/api/v1/ask", "application/json", strings.NewReader(`{"prompt":"Проверка"}`))
	if err != nil {
		t.Fatal(err)
	}
	defer r.Body.Close()
	var reply entity.AskResponse
	if err = json.NewDecoder(r.Body).Decode(&reply); err != nil {
		t.Fatal(err)
	}
	if r.StatusCode != http.StatusOK || reply.Answer != "Очки получили ответ." {
		t.Fatalf("unexpected response: %d %+v", r.StatusCode, reply)
	}
	_, text, err := ws.Read(ctx)
	if err != nil || string(text) != reply.Answer {
		t.Fatalf("WebSocket: %q %v", text, err)
	}
}

func TestAskDoesNotPublishInvalidOrFailedRequests(t *testing.T) {
	cases := []struct {
		name, body string
		uc         fakeUsecase
		status     int
	}{
		{"invalid JSON", "{", fakeUsecase{}, 400},
		{"empty prompt", `{"prompt":" "}`, fakeUsecase{}, 400},
		{"upstream failure", `{"prompt":"hello"}`, fakeUsecase{err: errors.New("offline")}, 500},
		{"nil answer", `{"prompt":"hello"}`, fakeUsecase{}, 500},
	}
	for _, tt := range cases {
		t.Run(tt.name, func(t *testing.T) {
			published := false
			h := New(tt.uc, func(string) { published = true })
			w := httptest.NewRecorder()
			h.Ask(w, httptest.NewRequest("POST", "/api/v1/ask", strings.NewReader(tt.body)))
			if w.Code != tt.status || published {
				t.Fatalf("status=%d published=%t", w.Code, published)
			}
		})
	}
}
