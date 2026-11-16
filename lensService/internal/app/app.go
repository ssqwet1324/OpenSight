package app

import (
	"log/slog"
	"net/http"

	"github.com/ssqwet1324/opensight/internal/client"
	"github.com/ssqwet1324/opensight/internal/config"
	"github.com/ssqwet1324/opensight/internal/handler"
	"github.com/ssqwet1324/opensight/internal/usecase"
)

func Run() {
	cfg, err := config.New()
	if err != nil {
		panic("Error load cfg: " + err.Error())
	}
	slog.Info("Config loaded", "cfg", cfg)

	aiClient := client.New(
		cfg.BaseUrl,
		cfg.Model,
	)
	defer aiClient.Close()

	uc := usecase.New(aiClient)
	h := handler.New(uc)

	mux := http.NewServeMux()

	mux.HandleFunc("GET /api/v1/health", h.Health)
	mux.HandleFunc("POST /api/v1/ask", h.Ask)

	slog.Info("OpenSight started on :8080")

	if err := http.ListenAndServe(cfg.ServerPort, mux); err != nil {
		panic("Error starting server: " + err.Error())
	}
}
