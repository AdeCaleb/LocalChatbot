# Private Desktop AI Knowledge Assistant

A local, privacy-focused AI chatbot that answers questions using your documents through a RAG (Retrieval-Augmented Generation) pipeline and local language model.

**No cloud. No data leaves your machine. Single executable.**

## Features

- **Privacy-first** - All processing happens locally on your machine
- **Offline-capable** - No internet required after initial setup
- **Document-grounded** - Answers come strictly from your knowledge base
- **Self-contained** - Ships as a single executable
- **Multiple formats** - Supports PDF, TXT, and Markdown files
- **Source citations** - Know exactly where answers come from

## Architecture

```
┌────────────────────────────────────────────────────┐
│              Tauri Application                      │
├────────────────────────────────────────────────────┤
│  Frontend (React + TypeScript)                      │
│  ┌──────────────────┐  ┌──────────────────┐        │
│  │   Chat Panel     │  │ Document Manager │        │
│  └──────────────────┘  └──────────────────┘        │
├────────────────────────────────────────────────────┤
│  Backend (Rust)                                     │
│  ┌──────────────────────────────────────────┐      │
│  │            RAG Pipeline                   │      │
│  │  Document Loader → Chunker → Embeddings  │      │
│  │         → Vector Store → LLM Engine      │      │
│  └──────────────────────────────────────────┘      │
└────────────────────────────────────────────────────┘
```

## Tech Stack

| Component | Technology |
|-----------|------------|
| Desktop Framework | Tauri 2.0 |
| Frontend | React + TypeScript |
| Backend | Rust |
| Embeddings | Candle + all-MiniLM-L6-v2 |
| Vector Store | usearch |
| LLM Runtime | llama-cpp-rs |
| Target Model | Mistral 7B Q4 |

## System Requirements

**Minimum**
- RAM: 8GB
- Storage: 5GB (app + model)
- OS: Windows 10+, macOS 11+, Linux (glibc 2.31+)

**Recommended**
- RAM: 16GB
- GPU: NVIDIA with 6GB+ VRAM (optional)
- Storage: SSD

## Quick Start

### Prerequisites

1. **Rust** (latest stable)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Node.js** (v18+)

3. **Tauri System Dependencies**

   <details>
   <summary>Linux (Debian/Ubuntu)</summary>

   ```bash
   sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget \
       libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
   ```
   </details>

   <details>
   <summary>Linux (openSUSE)</summary>

   ```bash
   sudo zypper install webkit2gtk3-devel openssl-devel gtk3-devel \
       libsoup-devel librsvg-devel gcc-c++ pkg-config Mesa-dri-intel
   ```
   </details>

   <details>
   <summary>macOS</summary>

   ```bash
   xcode-select --install
   ```
   </details>

   <details>
   <summary>Windows</summary>

   - Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
   - Install [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)
   </details>

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/local-chatbot.git
cd local-chatbot

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

### Download a Model

```bash
mkdir -p models

# Example: Download Mistral 7B Q4
huggingface-cli download TheBloke/Mistral-7B-Instruct-v0.2-GGUF \
    mistral-7b-instruct-v0.2.Q4_K_M.gguf \
    --local-dir ./models
```

## Use Cases

- **Internal company knowledge** - Query documentation privately
- **Consultants** - Chat with client documents securely
- **Legal/Medical** - Handle sensitive documents offline
- **Researchers** - Query paper collections locally
- **Offline environments** - Air-gapped systems, travel

## License

MIT

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.
