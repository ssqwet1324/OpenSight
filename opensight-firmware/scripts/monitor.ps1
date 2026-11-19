param(
    [string]$Port = 'COM8',
    [int]$BaudRate = 115200
)

$ErrorActionPreference = 'Stop'
$opensightPort = [System.IO.Ports.SerialPort]::new($Port, $BaudRate)
$opensightPort.Encoding = [System.Text.Encoding]::UTF8

try {
    $opensightPort.Open()
    Write-Host "OpenSight USB monitor: $Port. Press Ctrl+C to stop."
    Write-Host 'Waiting for messages; firmware prints its IP every 10 seconds.'

    while ($true) {
        $opensightText = $opensightPort.ReadExisting()
        if ($opensightText) {
            Write-Host -NoNewline $opensightText
        }
        Start-Sleep -Milliseconds 100
    }
}
finally {
    $opensightPort.Dispose()
}
