package usecase

import (
	"context"
	"fmt"

	"github.com/ssqwet1324/opensight/internal/entity"
)

type AIClient interface {
	AskClientAi(ctx context.Context, req entity.AskRequest) (*entity.AskResponse, error)
}

type UseCaseProvider interface {
	Ask(ctx context.Context, req entity.AskRequest) (*entity.AskResponse, error)
}

type Usecase struct {
	ai AIClient
}

func New(ai AIClient) UseCaseProvider {
	return &Usecase{
		ai: ai,
	}
}

func (uc *Usecase) Ask(ctx context.Context, req entity.AskRequest) (*entity.AskResponse, error) {
	resp, err := uc.ai.AskClientAi(ctx, req)
	if err != nil {
		return nil, fmt.Errorf("ask AI: %w", err)
	}

	return resp, nil
}
