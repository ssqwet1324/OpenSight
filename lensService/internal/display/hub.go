// Package display delivers completed answers to connected glasses.
package display

import (
	"context"
	"log/slog"
	"net/http"
	"sync"
	"time"
	"unicode/utf8"

	"github.com/coder/websocket"
)

// MaxTextBytes matches the message buffer in the ESP32 firmware.
const MaxTextBytes = 4096

type Hub struct {
	mu      sync.Mutex
	clients map[chan string]struct{}
	last    string
	hasLast bool
}

func New() *Hub { return &Hub{clients: make(map[chan string]struct{})} }

// Publish never waits for a device. A slow device receives the latest answer.
func (h *Hub) Publish(answer string) {
	if len(answer) > MaxTextBytes {
		answer = answer[:MaxTextBytes-len("…")]
		for !utf8.ValidString(answer) {
			answer = answer[:len(answer)-1]
		}
		answer += "…"
	}
	h.mu.Lock()
	defer h.mu.Unlock()
	h.last, h.hasLast = answer, true
	for ch := range h.clients {
		select {
		case ch <- answer:
		default:
			select {
			case <-ch:
			default:
			}
			ch <- answer
		}
	}
}

func (h *Hub) subscribe() (chan string, func()) {
	ch := make(chan string, 1)
	h.mu.Lock()
	h.clients[ch] = struct{}{}
	if h.hasLast {
		ch <- h.last
	}
	h.mu.Unlock()
	return ch, func() { h.mu.Lock(); delete(h.clients, ch); h.mu.Unlock() }
}

func (h *Hub) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	conn, err := websocket.Accept(w, r, &websocket.AcceptOptions{
		CompressionMode: websocket.CompressionDisabled,
	})
	if err != nil {
		return
	}
	defer conn.CloseNow()
	ctx := conn.CloseRead(r.Context())
	ch, unsubscribe := h.subscribe()
	defer unsubscribe()
	slog.Info("Display connected", "remote", r.RemoteAddr)
	defer slog.Info("Display disconnected", "remote", r.RemoteAddr)
	ticker := time.NewTicker(20 * time.Second)
	defer ticker.Stop()
	for {
		select {
		case <-ctx.Done():
			return
		case answer := <-ch:
			writeCtx, cancel := context.WithTimeout(ctx, 5*time.Second)
			err := conn.Write(writeCtx, websocket.MessageText, []byte(answer))
			cancel()
			if err != nil {
				return
			}
		case <-ticker.C:
			pingCtx, cancel := context.WithTimeout(ctx, 10*time.Second)
			err := conn.Ping(pingCtx)
			cancel()
			if err != nil {
				return
			}
		}
	}
}

// No additional JSON envelope is required: each WebSocket text message is one answer.
var _ http.Handler = (*Hub)(nil)
