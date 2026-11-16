package client

import (
	"context"
	"fmt"
	"time"

	"github.com/ssqwet1324/opensight/internal/entity"
	"resty.dev/v3"
)

type Client struct {
	httpClient *resty.Client
	model      string
}

func New(baseURL, model string) *Client {
	return &Client{
		httpClient: resty.New().
			SetBaseURL(baseURL).
			SetTimeout(60 * time.Second),
		model: model,
	}
}

func (c *Client) Close() error {
	return c.httpClient.Close()
}

func (c *Client) AskClientAi(
	ctx context.Context,
	req entity.AskRequest,
) (*entity.AskResponse, error) {
	body := chatRequest{
		Model:  c.model,
		Stream: false,
		Messages: []message{
			{
				Role:    "system",
				Content: "Ты AI-помощник для умных очков. Отвечай на русском языке, кратко, максимум 30 слов.",
			},
			{
				Role:    "user",
				Content: req.Prompt,
			},
		},
	}

	var result chatResponse

	resp, err := c.httpClient.R().
		SetContext(ctx).
		SetBody(body).
		SetResult(&result).
		Post("/api/chat")

	if err != nil {
		return nil, fmt.Errorf("ollama request: %w", err)
	}

	if !resp.IsStatusSuccess() {
		return nil, fmt.Errorf(
			"ollama unexpected status: %d",
			resp.StatusCode(),
		)
	}

	return &entity.AskResponse{
		Answer: result.Message.Content,
	}, nil
}
