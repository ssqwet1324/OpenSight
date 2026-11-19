package display

import (
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"strings"
	"unicode/utf8"
)

const framePrefix = "\x1eOSF1 "

// Frame is a complete screen, so dropped frames and seeks cannot leave stale letters.
type Frame struct {
	Rows       []string `json:"rows"`
	PositionMS uint32   `json:"positionMs"`
}

func (h *Hub) Frame(w http.ResponseWriter, r *http.Request) {
	// Only the local player proxy controls the screen; no new LAN control API.
	host, _, err := net.SplitHostPort(r.RemoteAddr)
	if err != nil || !net.ParseIP(host).IsLoopback() {
		http.Error(w, "local player only", http.StatusForbidden)
		return
	}
	if strings.Split(r.Header.Get("Content-Type"), ";")[0] != "application/json" {
		http.Error(w, "expected application/json", http.StatusUnsupportedMediaType)
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 2048)
	decoder := json.NewDecoder(r.Body)
	decoder.DisallowUnknownFields()
	var frame Frame
	if err := decoder.Decode(&frame); err != nil {
		http.Error(w, "invalid frame", http.StatusBadRequest)
		return
	}
	if decoder.Decode(new(any)) != io.EOF || len(frame.Rows) != 6 || frame.PositionMS > 86400000 {
		http.Error(w, "expected six screen rows and media time", http.StatusBadRequest)
		return
	}
	for _, row := range frame.Rows {
		if !utf8.ValidString(row) || utf8.RuneCountInString(row) > 21 || strings.ContainsAny(row, "\r\n\x00\x1e") {
			http.Error(w, "invalid screen row", http.StatusBadRequest)
			return
		}
	}
	h.Publish(fmt.Sprintf("%s%d\n%s", framePrefix, frame.PositionMS, strings.Join(frame.Rows, "\n")))
	w.WriteHeader(http.StatusNoContent)
}

func (h *Hub) Status(w http.ResponseWriter, r *http.Request) {
	h.mu.Lock()
	clients := len(h.clients)
	h.mu.Unlock()
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Cache-Control", "no-store")
	_ = json.NewEncoder(w).Encode(struct {
		Clients int `json:"clients"`
	}{clients})
}
