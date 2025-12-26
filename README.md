# Private Desktop AI Knowledge Assistant

A local, privacy-focused AI chatbot that answers questions using your documents through a RAG (Retrieval-Augmented Generation) pipeline and local language model.

**No cloud. No data leaves your machine. Single executable.**

---

## Overview

This application allows users to chat with their own documents using an AI model that runs entirely on their machine. Unlike cloud-based chatbots, this assistant is:

- **Privacy-first**: All processing happens locally
- **Offline-capable**: No internet required after initial setup
- **Document-grounded**: Answers come strictly from your knowledge base
- **Self-contained**: Ships as a single executable

---

## Features

### Core Functionality
- Natural language chat interface
- Document-based Q&A with RAG
- Support for PDF, TXT, and Markdown files
- Source citations for answers
- Persistent local knowledge base

### Technical Highlights
- Single binary distribution (no runtime dependencies)
- 8GB RAM target (runs on consumer hardware)
- Streaming responses
- Efficient vector similarity search

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Tauri Application                     │
│                   (Single Executable)                    │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │              Frontend (React + TypeScript)       │    │
│  │  ┌─────────────────┐  ┌─────────────────────┐   │    │
│  │  │   Chat Panel    │  │  Document Manager   │   │    │
│  │  │  - Messages     │  │  - File upload      │   │    │
│  │  │  - Input        │  │  - Index status     │   │    │
│  │  │  - Citations    │  │  - File list        │   │    │
│  │  └─────────────────┘  └─────────────────────┘   │    │
│  └─────────────────────────────────────────────────┘    │
│                           │                              │
│                    Tauri IPC Bridge                      │
│                           │                              │
│  ┌─────────────────────────────────────────────────┐    │
│  │              Backend (Rust)                      │    │
│  │                                                  │    │
│  │  ┌──────────────┐  ┌──────────────────────┐     │    │
│  │  │   Commands   │  │    RAG Pipeline      │     │    │
│  │  │  - chat      │  │  ┌────────────────┐  │     │    │
│  │  │  - upload    │  │  │ Doc Loader     │  │     │    │
│  │  │  - index     │  │  │ (PDF/TXT/MD)   │  │     │    │
│  │  │  - search    │  │  ├────────────────┤  │     │    │
│  │  └──────────────┘  │  │ Text Chunker   │  │     │    │
│  │                    │  │ (overlap)      │  │     │    │
│  │                    │  ├────────────────┤  │     │    │
│  │                    │  │ Embeddings     │  │     │    │
│  │                    │  │ (all-MiniLM)   │  │     │    │
│  │                    │  ├────────────────┤  │     │    │
│  │                    │  │ Vector Store   │  │     │    │
│  │                    │  │ (usearch)      │  │     │    │
│  │                    │  ├────────────────┤  │     │    │
│  │                    │  │ LLM Engine     │  │     │    │
│  │                    │  │ (llama.cpp)    │  │     │    │
│  │                    │  └────────────────┘  │     │    │
│  │                    └──────────────────────┘     │    │
│  └─────────────────────────────────────────────────┘    │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

---

## Tech Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| **Desktop Framework** | Tauri 2.0 | Native app shell, IPC, bundling |
| **Frontend** | React + TypeScript | Chat UI, document management |
| **Backend** | Rust | Core logic, RAG pipeline |
| **PDF Parsing** | `pdf-extract` | Document text extraction |
| **Embeddings** | `candle` + all-MiniLM-L6-v2 | Semantic text vectors |
| **Vector Store** | `usearch` | Similarity search |
| **LLM Runtime** | `llama-cpp-rs` | Local model inference |
| **Target Model** | Mistral 7B Q4 | ~4GB RAM, good quality |

### Why This Stack?

**Tauri over Electron**
- 10-15MB bundle vs 150MB+
- Uses system webview (no Chromium)
- Rust backend = no Python runtime needed

**Full Rust over Python Sidecar**
- Single binary output
- Faster startup (<1s vs 2-5s)
- Lower memory footprint
- No IPC overhead
- True parallelism (no GIL)

**Candle over PyTorch**
- Native Rust ML framework
- No Python dependency
- Optimized for inference

---

## System Requirements

### Minimum
- **RAM**: 8GB
- **Storage**: 5GB (app + model)
- **OS**: Windows 10+, macOS 11+, Linux (glibc 2.31+)

### Recommended
- **RAM**: 16GB
- **GPU**: NVIDIA with 6GB+ VRAM (optional, for faster inference)
- **Storage**: SSD for faster document indexing

---

## Project Structure

```
localChatbot/
├── src/                    # React frontend
│   ├── components/         # UI components
│   │   ├── Chat/          # Chat interface
│   │   └── Documents/     # Document manager
│   ├── hooks/             # Custom React hooks
│   ├── lib/               # Utilities
│   ├── App.tsx            # Main app component
│   └── main.tsx           # Entry point
│
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── main.rs        # Tauri entry point
│   │   ├── commands/      # IPC command handlers
│   │   ├── rag/           # RAG pipeline
│   │   │   ├── loader.rs      # Document loading
│   │   │   ├── chunker.rs     # Text chunking
│   │   │   ├── embeddings.rs  # Vector embeddings
│   │   │   ├── store.rs       # Vector store
│   │   │   └── pipeline.rs    # Orchestration
│   │   ├── llm/           # LLM integration
│   │   │   ├── engine.rs      # Inference engine
│   │   │   └── streaming.rs   # Response streaming
│   │   └── state.rs       # Application state
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
│
├── models/                # LLM model files (gitignored)
├── data/                  # User documents & index (gitignored)
│
├── package.json           # Node dependencies
├── tsconfig.json          # TypeScript config
├── vite.config.ts         # Vite bundler config
└── README.md              # This file
```

---

## Development Setup

### Prerequisites

1. **Rust** (latest stable)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Node.js** (v18+)
   ```bash
   # Using package manager (example: openSUSE)
   sudo zypper install nodejs npm

   # Or using nvm
   curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
   nvm install --lts
   ```

3. **Tauri System Dependencies**

   **Linux (Debian/Ubuntu)**
   ```bash
   sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget \
       libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
   ```

   **Linux (openSUSE)**
   ```bash
   sudo zypper install webkit2gtk3-devel openssl-devel gtk3-devel \
       libsoup-devel librsvg-devel gcc-c++ pkg-config
   ```

   **macOS**
   ```bash
   xcode-select --install
   ```

   **Windows**
   - Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
   - Install [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

4. **Tauri CLI**
   ```bash
   cargo install create-tauri-app --locked
   cargo install tauri-cli --locked
   ```

### Running Locally

```bash
# Install frontend dependencies
npm install

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

### Downloading the Model

The application requires a Mistral 7B Q4 quantized model:

```bash
# Create models directory
mkdir -p models

# Download model (example using huggingface-cli)
huggingface-cli download TheBloke/Mistral-7B-Instruct-v0.2-GGUF \
    mistral-7b-instruct-v0.2.Q4_K_M.gguf \
    --local-dir ./models
```

---

## RAG Pipeline Flow

```
User Question
      │
      ▼
┌─────────────┐
│  Embedding  │ ──► Convert question to vector
└─────────────┘
      │
      ▼
┌─────────────┐
│   Search    │ ──► Find top-k similar chunks
└─────────────┘
      │
      ▼
┌─────────────┐
│  Context    │ ──► Build prompt with retrieved chunks
│  Assembly   │
└─────────────┘
      │
      ▼
┌─────────────┐
│     LLM     │ ──► Generate grounded response
└─────────────┘
      │
      ▼
   Response + Citations
```

---

## Implementation Phases

### Phase 1: Foundation ✅
- [x] Project documentation
- [ ] Tauri + React scaffold
- [ ] Basic Rust backend structure
- [ ] Simple chat UI shell

### Phase 2: Document Pipeline
- [ ] PDF text extraction
- [ ] TXT/Markdown loading
- [ ] Text chunking with overlap
- [ ] Document metadata tracking

### Phase 3: Intelligence Layer
- [ ] Embedding model integration
- [ ] Vector store setup
- [ ] Similarity search

### Phase 4: LLM Integration
- [ ] llama-cpp-rs bindings
- [ ] Model loading
- [ ] Streaming responses

### Phase 5: RAG & Polish
- [ ] Full RAG pipeline
- [ ] Source citations in UI
- [ ] Error handling
- [ ] Single binary packaging

---

## Use Cases

- **Internal company knowledge** - Query internal documentation privately
- **Consultants** - Chat with client documents without cloud exposure
- **Legal/Medical** - Handle sensitive documents offline
- **Researchers** - Query paper collections locally
- **Offline environments** - Air-gapped systems, travel

---

## License

[To be determined]

---

## Contributing

[Contribution guidelines to be added]
