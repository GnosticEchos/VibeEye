# Linux Native Installation Guide (Non-Container Setup)

This guide walks through installing **VibeEye** (`vibe-eye` CLI and `vibeeye-mcp` server) as a native Linux user program backed by user-level `systemd` services for **SurrealDB** and **Ollama** — without Docker or root-owned containers.

> **Note on example credentials:** This guide uses `Vibe` / `Sparkles!` as demo credentials and `/home/User` as the example home directory. Replace these with your own username, paths, and a strong password for any non-local or sensitive deployment.

---

## Why Run SurrealDB as a User Service?

While VibeEye supports embedded `surrealkv://` file storage directly, an embedded file lock can only be held by **one process at a time**. Running SurrealDB as a lightweight `systemd --user` background service over WebSocket (`ws://127.0.0.1:8003`) allows:

- **Concurrent access:** Your AI agent host can keep `vibeeye-mcp` open continuously while you simultaneously run `vibe-eye crawl` or `vibe-eye db` commands in your terminal.
- **Zero root permission friction:** Data files live in your home directory (e.g., `~/Surreal_Data/playground1.db`) and run under your own Linux user account.
- **Automatic startup on login/boot:** Managed cleanly by `systemd --user` and `loginctl enable-linger`.

---

## 1. Prerequisites

Ensure the following tools are installed on your Linux machine:

1. **Rust 1.88+** (`rustup update`)
2. **SurrealDB CLI/Server (v3.x)** — e.g., installed at `/usr/local/bin/surreal`:
   ```bash
   surreal db start --help
   ```
3. **Ollama** (or another OpenAI-compatible `/v1/embeddings` server, such as `gufo`, `llama-server`, or `FLM`) — e.g., installed at `/usr/bin/ollama`.
4. **`~/.local/bin` on your `PATH`**:
   ```bash
   mkdir -p ~/.local/bin
   export PATH="$HOME/.local/bin:$PATH"
   ```

---

## 2. Build & Install VibeEye Binaries

Clone the repository, compile with both `surrealdb` and `embeddings` features enabled, and install both binaries to `~/.local/bin`:

```bash
git clone https://github.com/GnosticEchos/VibeEye.git
cd VibeEye

cargo build --release --features "surrealdb embeddings"

install -m 755 target/release/vibe-eye target/release/vibeeye-mcp ~/.local/bin/
```

Verify the binaries are in your `PATH`:

```bash
which vibe-eye vibeeye-mcp
vibe-eye --version
```

---

## 3. Prepare Your Embedding Model (Ollama Example)

Start Ollama (or if already running, pull your preferred embedding model). This guide uses `qwen3-embedding:0.6b` (`1024` dimensions, `32768` context length):

```bash
ollama pull qwen3-embedding:0.6b
ollama show qwen3-embedding:0.6b
```

Expected model details:
```text
  Model
    architecture        qwen3
    parameters          595.78M
    context length      32768
    embedding length    1024
    quantization        Q8_0
```

*(If you have manual foreground instances of `ollama serve` or `surreal db start` running in terminal tabs, stop them before proceeding to Step 4 so `systemd` can bind their ports cleanly.)*

---

## 4. Set Up User-Level `systemd` Services

### 4.1 Create directories and SurrealDB credentials file

Keep your SurrealDB credentials in a `0600` environment file so they are not exposed in `ps aux` process arguments:

```bash
mkdir -p ~/.config/vibe-eye
mkdir -p ~/.config/systemd/user
mkdir -p ~/Surreal_Data

cat << 'EOF' > ~/.config/vibe-eye/surrealdb.env
SURREAL_USER=Vibe
SURREAL_PASS=Sparkles!
EOF

chmod 600 ~/.config/vibe-eye/surrealdb.env
```

### 4.2 Create `~/.config/systemd/user/surrealdb.service`

> Replace `/home/User` with your actual home directory path (`echo $HOME`).

```ini
[Unit]
Description=SurrealDB Server (User)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
EnvironmentFile=/home/User/.config/vibe-eye/surrealdb.env
ExecStart=/usr/local/bin/surreal db start --no-banner -A --bind 0.0.0.0:8003 surrealkv:///home/User/Surreal_Data/playground1.db -l info
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

> **Tip:** Use `--bind 127.0.0.1:8003` if you only want local connections on the same machine, or `--bind 0.0.0.0:8003` if you also connect from other machines on your LAN.

### 4.3 Create `~/.config/systemd/user/ollama.service`

Running Ollama as a user unit ensures it reads models directly from `~/.ollama` under your user account:

```ini
[Unit]
Description=Ollama Service (User)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
Environment="HOME=/home/User"
Environment="PATH=/opt/rocm/bin:/usr/local/bin:/usr/bin:/bin"
ExecStart=/usr/bin/ollama serve
Restart=on-failure
RestartSec=3

[Install]
WantedBy=default.target
```

*(Note: If a system-wide `/usr/lib/systemd/system/ollama.service` exists, keep it disabled so it does not conflict with your user unit on port `11434`.)*

### 4.4 Enable, start, and enable boot linger

```bash
# Reload user systemd unit files
systemctl --user daemon-reload

# Enable on login and start immediately
systemctl --user enable --now surrealdb.service ollama.service

# Enable lingering so user services also start at system boot before login
loginctl enable-linger "$USER"
```

Check that both services are `active (running)` and `Linger=yes`:

```bash
systemctl --user status surrealdb.service ollama.service
loginctl show-user "$USER" -p Linger
```

---

## 5. Configure VibeEye (`~/.config/vibe-eye/crawl.toml`)

Create `~/.config/vibe-eye/crawl.toml` pointing to your local SurrealDB instance (`ws://Vibe:Sparkles!@127.0.0.1:8003`) and Ollama embedding endpoint (`http://127.0.0.1:11434/v1/embeddings`):

```toml
# VibeEye crawl configuration (~/.config/vibe-eye/crawl.toml)
#
# Merge order: [global] → [domain."example.com"] → [subdomain."docs.example.com"]

[global]
max_depth = 2
max_pages = 100
format = "markdown"
respect_robots = false
requests_per_second = 2.0
concurrency = 4
same_origin = true
timeout = 15
sitemap = false

# ── SurrealDB ────────────────────────────────────────────────────────────────
db_url = "ws://Vibe:Sparkles!@127.0.0.1:8003"
surrealdb_ns = "vibeeye"
surrealdb_db = "crawl"

# ── Embedding provider (used with --embed flag & db vector / db hybrid) ──────
[global.embeddings]
provider = "openai-compatible"
endpoint = "http://127.0.0.1:11434/v1/embeddings"
model = "qwen3-embedding:0.6b"
dimensions = 1024
context_window = 32768
chunk_size = 512
chunk_overlap = 50
embed_concurrency = 4

# ── CLI theming (help-tree output styling) ───────────────────────────────────
[cli.help_tree.command]
style = "bold"
color = "#7ee7e6"

[cli.help_tree.options]
style = "normal"
color = "#ff69b4"

[cli.help_tree.description]
style = "italic"
color = "#90a2af"
```

---

## 6. Verify the Installation End-to-End

### 6.1 Verify Ollama's embedding endpoint

```bash
curl -s http://127.0.0.1:11434/v1/embeddings \
  -H "Content-Type: application/json" \
  -d '{"model": "qwen3-embedding:0.6b", "input": ["hello world"]}' \
  | jq '{model: .model, dim: (.data[0].embedding | length)}'
```

Expected output:
```json
{
  "model": "qwen3-embedding:0.6b",
  "dim": 1024
}
```

### 6.2 Bootstrap SurrealDB schema & test vector search

On a brand-new SurrealDB datastore, the `vibeeye/crawl` schema (`page`, `discovered`, `chunk`, `db_metadata`, and the 1024-dim HNSW index) is automatically bootstrapped on the first `--surrealdb` crawl. You can bootstrap and verify the entire pipeline in seconds using the built-in test stub:

```bash
# 1. Run a 1-page stub crawl with SurrealDB + embeddings to bootstrap schema & HNSW index
VIBEYE_TEST_STUB=1 vibe-eye crawl https://example.com --max-depth 0 --max-pages 1 --surrealdb --embed

# 2. Check group status
vibe-eye db status example_com

# 3. Run a vector similarity search
vibe-eye db vector "Navigated to example" -g example_com

# 4. Clean up the test group
vibe-eye db reset example_com

# 5. Verify clean group list
vibe-eye db list
```

---

## 7. Configure MCP Hosts (`vibeeye-mcp`)

Because `vibeeye-mcp` automatically loads `~/.config/vibe-eye/crawl.toml` via `resolve_db_url()`, your MCP client configuration (Claude Desktop, Cursor, Antigravity, etc.) only needs to invoke the installed binary:

```json
{
  "mcpServers": {
    "vibeeye": {
      "command": "/home/User/.local/bin/vibeeye-mcp",
      "args": []
    }
  }
}
```

---

## 8. Managing the Services

```bash
# View live logs
journalctl --user -u surrealdb.service -f
journalctl --user -u ollama.service -f

# Restart a service
systemctl --user restart surrealdb.service
systemctl --user restart ollama.service

# Stop services (if switching to another inference engine like gufo)
systemctl --user stop ollama.service
systemctl --user disable ollama.service
```
