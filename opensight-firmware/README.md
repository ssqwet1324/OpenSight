# Прошивка OpenSight

Rust, ESP32-C3 Super Mini, Wi-Fi, WebSocket и SSD1306-совместимый OLED 128×64.

## Инструменты

Rust stable с `rust-src` и целью `riscv32imc-unknown-none-elf` задаётся в `rust-toolchain.toml`. При подготовке проекта использовались `esp-generate 2.0.0`, `espflash 4.7.0`, `esp-hal 1.2.2`. Генерировать проект заново не требуется: исходники и `Cargo.lock` уже сохранены.

```powershell
rustup target add riscv32imc-unknown-none-elf
rustup component add rust-src
cargo install espflash --locked
```

Собирать из этой папки. Зависимости зафиксированы в `Cargo.lock`.

## Локальные настройки

```powershell
Copy-Item wifi-ssid.example.txt wifi-ssid.txt
Copy-Item wifi-password.example.txt wifi-password.txt
Copy-Item src/server_config.example.rs src/server_config.rs
```

Заполни `wifi-ssid.txt` названием сети, `wifi-password.txt` паролем, а `HOST` в `src/server_config.rs` — IPv4-адресом компьютера с Go-сервисом. `PORT` по умолчанию `8080`. ESP и компьютер должны быть в одной доступной друг другу сети. Используемая конфигурация подключения — WPA2 Personal; ESP32-C3 работает с Wi-Fi 2,4 ГГц.

Эти три файла игнорируются Git. Они читаются во время сборки, поэтому после смены настроек нужна повторная сборка и прошивка.

## OLED

`GND → GND`, `3V3 → VDD/VCC`, `GPIO4 → SDA`, `GPIO5 → SCK/SCL`. Перед подключением проверить подписи на своих платах и отключить питание. Используется I²C 100 кГц; адреса проверяются `0x3C` и `0x3D`.

После старта выводится тестовая надпись OpenSight / OLED ready. Далее экран получает данные от Go-сервиса.

## Сборка и прошивка

```powershell
cargo build --locked
espflash list-ports
espflash flash --port COM8 --before usb-reset --after watchdog-reset .\target\riscv32imc-unknown-none-elf\debug\opensight-firmware
```

`COM8` — порт проверенного стенда; на другом компьютере выбери свой. Если плата не входит в загрузчик автоматически: удерживать BOOT, кратко нажать RST, отпустить BOOT. После записи отпускать BOOT и запускать обычную загрузку. Один только найденный COM-порт ещё не подтверждает успешную запись прошивки.

На проверенном Windows-стенде встроенный монитор espflash вызывал трудности с USB Serial/JTAG. Для чтения журнала используется обычный последовательный порт:

```powershell
& .\scripts\monitor.ps1 -Port COM8
```

Перед новой прошивкой закрыть монитор через Ctrl+C. Начальный вывод: `OpenSight started!`, затем адрес Wi-Fi и статус WebSocket.

## Два вида сообщений

Обычное UTF-8 сообщение WebSocket — готовый ответ AI, максимум 4096 байт. Он печатается примерно с задержкой 45 мс на символ. Экран: 21 символ в строке, шесть строк, перенос по словам, прокрутка вверх, русский шрифт ISO 8859-5. Некоторые знаки пунктуации упрощаются под шрифт; неподдерживаемые символы заменяются `?`.

Сообщение `\x1eOSF1 <positionMs>\n<row1>\n…\n<row6>` — полный текстовый кадр внешнего источника. Каждая строка до 21 символа. Он рисуется сразу, обновляя только изменившиеся буквы. Это позволяет внешнему плееру управлять печатью по времени, останавливать её на паузе и восстанавливать после перемотки. Транспорт описан в [README сервиса](../lensService/README.md).

Задача вывода отделена от чтения WebSocket: анимация не останавливает обработку ping/pong. При новом сообщении незавершённый вывод заменяется свежим.

## Тесты без платы

Чтобы Cargo не выбрал embedded-цель для этих тестов, они запускаются через `rustc` из PowerShell на компьютере:

```powershell
rustc --edition=2024 --test src/text_layout.rs -o "$env:TEMP/opensight-layout-tests.exe"
& "$env:TEMP/opensight-layout-tests.exe"
rustc --edition=2024 --test src/screen_frame.rs -o "$env:TEMP/opensight-frame-tests.exe"
& "$env:TEMP/opensight-frame-tests.exe"
```

Тесты проверяют кириллицу, границы строк, прокрутку, длинные слова и валидацию кадров. Они не заменяют проверку I²C, Wi-Fi и USB на физической плате.
