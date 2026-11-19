package handler

import (
	"encoding/json"
	"net/http"
	"strings"

	"github.com/ssqwet1324/opensight/internal/entity"
	"github.com/ssqwet1324/opensight/internal/usecase"
)

type Handler struct {
	uc      usecase.UseCaseProvider
	publish func(string)
}

func New(uc usecase.UseCaseProvider, publish func(string)) *Handler {
	return &Handler{
		uc:      uc,
		publish: publish,
	}
}

func (h *Handler) Health(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/plain; charset=utf-8")
	w.WriteHeader(http.StatusOK)

	_, _ = w.Write([]byte("OK\n"))
}

func (h *Handler) Ask(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()

	r.Body = http.MaxBytesReader(w, r.Body, 1<<20)

	var req entity.AskRequest

	decoder := json.NewDecoder(r.Body)
	decoder.DisallowUnknownFields()

	if err := decoder.Decode(&req); err != nil {
		http.Error(w, "invalid JSON", http.StatusBadRequest)
		return
	}

	if strings.TrimSpace(req.Prompt) == "" {
		http.Error(w, "prompt is required", http.StatusBadRequest)
		return
	}

	resp, err := h.uc.Ask(ctx, req)
	if err != nil {
		http.Error(w, "internal server error", http.StatusInternalServerError)
		return
	}

	if resp == nil {
		http.Error(w, "empty response from service", http.StatusInternalServerError)
		return
	}
	if h.publish != nil {
		h.publish(resp.Answer)
	}

	w.Header().Set("Content-Type", "application/json")

	if err := json.NewEncoder(w).Encode(resp); err != nil {
		return
	}
}
