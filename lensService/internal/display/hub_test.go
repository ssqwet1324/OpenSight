package display

import (
	"context"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
	"unicode/utf8"

	"github.com/coder/websocket"
)

func TestPublishKeepsLatestWithoutBlocking(t *testing.T) {
	h := New()
	ch, cancel := h.subscribe()
	defer cancel()
	done := make(chan struct{})
	go func() {
		for i := 0; i < 1000; i++ {
			h.Publish("old")
		}
		h.Publish("latest")
		close(done)
	}()
	select {
	case <-done:
	case <-time.After(time.Second):
		t.Fatal("slow display blocked publishing")
	}
	if got := <-ch; got != "latest" {
		t.Fatalf("got %q", got)
	}
	other, unsubscribe := h.subscribe()
	defer unsubscribe()
	if got := <-other; got != "latest" {
		t.Fatalf("reconnect replay got %q", got)
	}
}

func TestLongAnswerKeepsValidUTF8(t *testing.T) {
	h := New()
	ch, cancel := h.subscribe()
	defer cancel()
	h.Publish(strings.Repeat("Я", MaxTextBytes))
	got := <-ch
	if len(got) > MaxTextBytes || !utf8.ValidString(got) || !strings.HasSuffix(got, "…") {
		t.Fatal("invalid bounded display text")
	}
}

func TestWebSocketReplaysAndReceivesLiveAnswer(t *testing.T) {
	h := New()
	h.Publish("Первый ответ")
	s := httptest.NewServer(h)
	defer s.Close()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	c, _, err := websocket.Dial(ctx, "ws"+strings.TrimPrefix(s.URL, "http"), nil)
	if err != nil {
		t.Fatal(err)
	}
	defer c.CloseNow()
	_, got, err := c.Read(ctx)
	if err != nil || string(got) != "Первый ответ" {
		t.Fatalf("replay: %q %v", got, err)
	}
	h.Publish("Новый ответ")
	_, got, err = c.Read(ctx)
	if err != nil || string(got) != "Новый ответ" {
		t.Fatalf("live: %q %v", got, err)
	}
}
