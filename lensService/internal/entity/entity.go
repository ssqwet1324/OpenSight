package entity

// AskRequest - запрос
type AskRequest struct {
	Prompt string `json:"prompt"`
}

// AskResponse - ответ
type AskResponse struct {
	Answer string `json:"answer"`
}
