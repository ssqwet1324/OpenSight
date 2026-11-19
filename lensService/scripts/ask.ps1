param(
    [Parameter(Position = 0)]
    [string]$Prompt,
    [string]$Server = 'http://localhost:8080'
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Prompt)) {
    $Prompt = Read-Host 'Question for Qwen'
}
if ([string]::IsNullOrWhiteSpace($Prompt)) {
    throw 'Enter a question.'
}

$opensightBody = @{ prompt = $Prompt } | ConvertTo-Json -Compress
$opensightReply = Invoke-RestMethod -Method Post `
    -Uri ($Server.TrimEnd('/') + '/api/v1/ask') `
    -ContentType 'application/json; charset=utf-8' `
    -Body ([Text.Encoding]::UTF8.GetBytes($opensightBody))

$opensightReply.answer
