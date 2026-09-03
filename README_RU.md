# GameAP Files Plugin

Плагин управления FTP/SFTP для панели [GameAP](https://gameap.com).
Устанавливает и настраивает FTP/SFTP-демон
[gameap-files](https://github.com/gameap/gameap-files) на нодах, управляет
FTP-пользователями игровых серверов, правилами доступа, виртуальными путями и
SSH-ключами.

Переписан на Rust с исходного Go-плагина (`plugin-gameap-files`). Данные в
storage, YAML-файлы на нодах и HTTP API полностью совместимы — существующие
установки продолжают работать после замены.

*Read this in other languages: [English](README.md)*

## Возможности

- Установка gameap-files на Linux- и Windows-ноды в один клик (цепочка
  daemon-тасков, отслеживание статуса через события с fallback-поллингом и
  таймаутом) и кнопка **Обновить**, которая повторно запускает установщик с
  сохранёнными настройками для обновления установленной ноды
- Настройка FTP/SFTP per-node (`config.yaml` патчится на месте — чужие ключи
  сохраняются); сервис перезапускается через системный юнит, user-юнит
  rootless-демона или службу Windows
- FTP/SFTP-пользователи по серверам: создание/изменение/удаление, Argon2id-хэши
  через crypto-сервис панели, одноразовый показ сгенерированного пароля
- Правила доступа по путям (`read` / `write` / `delete` / `list`),
  виртуальные пути, SSH-ключи
- Синхронизация пользователей на ноды YAML-файлами в
  `<work_path>/.plugins/files/users.d/` — сервисный каталог плагина внутри
  рабочего каталога демона, единственное место, куда gameap-daemon разрешает
  писать плагину панели (относительные домашние каталоги тоже привязываются
  к нему); демон перечитывает файлы на лету
- Админ-страницы: список нод со статусами установки, все пользователи с
  группировкой нода → сервер и фильтрами
- Права сервера `ftp-users-view` / `ftp-users-manage` для не-админов

## Архитектура

| Слой | Модуль | Ответственность |
|---|---|---|
| ABI | `src/lib.rs` | Реализация `Plugin`, `register_plugin!`, вшитый фронтенд |
| Транспорт | `src/http.rs`, `src/router.rs` | JSON-модель ошибок, таблица маршрутов, диспетчеризация |
| Хендлеры | `src/handlers/*` | Логика маршрутов и DTO, обработка событий |
| Сервисы | `src/services/*` | Пользователи, синхронизация, оркестрация установки, патч YAML, админ-агрегация |
| Домен | `src/domain/*` | Wire-совместимая модель, валидация |
| Хост-шов | `src/host_api.rs` | Трейт `HostApi` + `WasmHost` (wasm) / `MockHost` (тесты) |

Бизнес-логика не трогает ABI хоста напрямую — всё идёт через трейт `HostApi`,
поэтому весь стек роутер + хендлеры гоняется нативным `cargo test` на
in-memory моке хоста.

### Хранение данных

KV-хранилище панели (совместимо с Go-версией):

| Scope | Ключ | Содержимое |
|---|---|---|
| node | `ftp:setup_status` | Статус установки, id тасков, таймстампы |
| node | `ftp:node_config` | Настройки FTP/SFTP |
| server | `ftp:users_list` | Индекс имён (JSON-массив) |
| server | `ftp:user:{username}` | Полный документ пользователя (JSON) |

### События

- `SERVER_DELETED` — удаляет пользователей сервера из storage и их YAML с ноды
  (id ноды берётся из payload события)
- `DAEMON_TASK_COMPLETED` / `DAEMON_TASK_FAILED` — сопоставляются с id
  install/download-тасков из сохранённого статуса установки; завершённая
  установка один раз пересинхронизирует всех пользователей ноды
  (`synced_after_install` в сохранённом статусе) и удаляет ошибочные файлы
  `<work_path>/etc/gameap-files/users.d/*.yaml`, которые писали старые релизы

### Раскладка на ноде

gameap-daemon ограничивает файловые операции плагина панели рабочим
каталогом ноды, поэтому все пути плагина относительны к нему:

| Путь на ноде (относительно `work_path`) | Содержимое |
|---|---|
| `.plugins/files/config.yaml` | конфигурация gameap-files, патчится из диалога настроек |
| `.plugins/files/users.d/<user>.yaml` | по одному файлу на FTP-пользователя |
| `tools/install-files-linux.sh`, `tools/install-files-windows.ps1` | установщики, скачанные через `get-tool` |
| `tools/gameap-files/` (Windows) | бинарник и служба Windows-установки |

### Обновление с 0.7.x

Ноды, установленные прежним релизом плагина, продолжают читать
`/etc/gameap-files`, пока один раз не нажать **Обновить**: установщик
переносит этот каталог в `<work_path>/.plugins/files`, плагин
пересинхронизирует пользователей ноды и удаляет файлы, которые старый релиз
оставил в `<work_path>/etc/gameap-files`. До этого **Настройки** на такой
ноде падают с ошибкой "failed to download config", а созданные в панели
пользователи не видны gameap-files.

## Сборка

Path-зависимости требуют раскладки соседними каталогами:

```
gameap-api/      # gameap/gameap — web/plugin-sdk для фронтенда
gameap-proto/    # gameap/gameap-proto — rust/gameap-plugin-sdk
plugin-files/    # этот репозиторий
```

Требования: Rust (закреплён в `rust-toolchain.toml`, цель `wasm32-wasip1`),
Node.js 22+, опционально [binaryen](https://github.com/WebAssembly/binaryen)
для `wasm-opt`.

```bash
make build   # фронтенд (npm ci + vite) → cargo build → wasm-opt → files.wasm
make test    # cargo test + vitest фронтенда
make lint    # clippy (обе цели) + vue-tsc
```

Цикл разработки:

```bash
cd frontend && npm run dev     # пересборка фронтенда при изменениях
cargo build --target wasm32-wasip1 --release   # пересборка wasm
cd frontend && npm run debug   # UI отдельно от панели на MSW-моках
```

## Установка

Загрузите `files.wasm` через **Администрирование → Плагины** либо положите в
каталог плагинов панели и перезапустите GameAP.

## API

Все маршруты живут под `/api/plugins/files`. Полная спецификация — в
[`openapi/openapi.yaml`](openapi/openapi.yaml): установка/статус/конфиг нод,
CRUD пользователей, правила доступа, виртуальные пути, SSH-ключи и
админ-эндпоинты.

## Релиз

1. Поднимите `version` в `Cargo.toml` **и** `frontend/package.json`
   (должны совпадать).
2. Смержите, создайте GitHub-релиз с тегом `v<version>`.
3. Release-workflow соберёт, подпишет GPG и опубликует wasm на
   plugins.gameap.dev (нужны секреты `GPG_SIGNING_KEY` /
   `GAMEAP_DEPLOY_TOKEN` и переменная репозитория `GAMEAP_PLUGIN_ID`).

## Лицензия

MIT
