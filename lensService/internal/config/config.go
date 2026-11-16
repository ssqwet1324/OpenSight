package config

import (
	"fmt"

	"github.com/ilyakaznacheev/cleanenv"
)

type Config struct {
	BaseUrl    string `env:"BASE_URL" env-required:"true"`
	Model      string `env:"MODEL" env-required:"true"`
	ServerPort string `env:"SERVER_PORT" env-required:"true"`
}

func New() (*Config, error) {
	var cfg Config

	if err := cleanenv.ReadConfig(".env", &cfg); err != nil {
		return nil, fmt.Errorf("failed to read config: %w", err)
	}

	return &cfg, nil
}
