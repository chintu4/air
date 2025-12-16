# AIR - Your Personal AI Agent

**Privacy-First | Local Intelligence | Cloud Power**

AIR is a powerful, personal AI agent designed to run on your own machine. It prioritizes your privacy and speed by running models locally (on your CPU/GPU) while offering seamless access to advanced cloud models (like Gemini) when you need them.

Whether you need to chat, analyze documents, capture screenshots, or browse the web, AIR is your all-in-one assistant.

---

## 🚀 Why Use AIR?

*   **🔒 Privacy First**: Runs open-source models (like TinyLlama, Mistral) directly on your computer. Your basic chats stay on your device.
*   **⚡ High Performance**: Low-latency responses for everyday tasks using optimized local inference.
*   **🧠 Smart Hybrid Mode**: Automatically switches to powerful cloud models (Gemini) for complex reasoning or coding tasks.
*   **🛠️ Real Tools**: AIR isn't just a chatbot. It can:
    *   **Read & Write Files** on your system.
    *   **See Your Screen** to help with UI tasks.
    *   **Browse the Web** to fetch live content.
    *   **Speak** to you with text-to-speech.
    *   **Remember** information from your documents (RAG).

---

## 📦 Installation
You can check out [air](https://github.com/chintu4/air) 's release section to download latest version for desktop type
( or )

AIR is available for installation via Rust's package manager, `cargo`.

### Prerequisites
*   **Rust**: Install from [rustup.rs](https://rustup.rs)
*   **C++ Build Tools**: (Required for compiling model engines)
    *   *Windows*: Visual Studio Build Tools with C++ workload.
    *   *Linux*: `build-essential`, `pkg-config`, `libssl-dev`.
    *   *macOS*: Xcode Command Line Tools.

### Install AIR
```bash
# Clone the repository
git clone https://github.com/chintu4/air.git
cd air

# Build and install
cargo install --path .
```
*Note: The first build may take a few minutes as it compiles the AI inference engine.*

---

## 🏁 Quick Start

Once installed, follow these steps to get your personal agent ready.

### 1. Setup Local Models (Recommended)
Download a small, fast model to run on your computer. This allows AIR to work offline and preserves privacy.
```bash
air setup --local
```
*This will download a verified GGUF model (e.g., TinyLlama) to your `~/.air/models` directory.*

### 2. Connect Cloud AI (Optional)
For smarter responses and vision capabilities, connect a cloud provider (currently supports Google Gemini).
```bash
air login
```
*Follow the prompts to get your free API key from Google AI Studio.*

### 3. Start Chatting!
Launch the interactive agent:
```bash
air
```

---

## 💡 Features & Usage

### 💬 Interactive Chat
Just type `air` to enter the interactive mode. You can ask questions, ask for help with files, or just chat.
```text
You: Create a new file called notes.txt and write "Meeting at 5pm" in it.
AIR: I've created the file 'notes.txt' with your content.
```

### ⚡ Smart Routing (Default)
AIR decides whether to use your local model (fast/free) or the cloud (smart/costly) based on your query.
```bash
# Basic query (Uses Local Model)
air -p "What time is it?"

# Complex query (Falls back to Cloud)
air -p "Analyze the market trends for AI in 2024"
```

### 🏠 Local-Only Mode
Force AIR to use *only* your local model. No data leaves your machine.
```bash
air -p "Draft a private email" --local-only
```
*Or use the `-l` flag.*

### 🧠 Memory & Knowledge (RAG)
Teach AIR about your personal documents so you can chat with them.
```bash
# Add a file to AIR's knowledge base
air memory add --path ./my-project-docs.pdf

# Then ask about it in chat
air -p "Summarize the project docs I just added"
```

### 🛠️ Integrated Tools
AIR can use tools to help you. It will ask for permission before performing sensitive actions (like deleting files).

*   **📂 File System**: "Read the config file", "Create a python script".
*   **🌐 Web Access**: "Fetch the content of rust-lang.org".
*   **📸 Vision & Screenshots**: "Take a screenshot" (Can also analyze them if Cloud is enabled).
*   **🗣️ Voice**: "Say 'Hello World'" (Text-to-Speech).

---

## ⚙️ Configuration

Want to change the local model, adjust timeouts, or enable/disable providers? Use the interactive config menu:

```bash
air config
```
*   **Local Model**: Select which `.gguf` file to use.
*   **Timeouts**: Adjust how long to wait for local generation.
*   **Providers**: Enable/Disable Cloud fallback.

---

## 👩‍💻 For Developers

### Architecture
AIR uses a **ReAct** (Reasoning + Acting) loop:
1.  **Think**: The LLM analyzes the user request.
2.  **Act**: The LLM decides to call a tool (e.g., `fs.read`, `web.fetch`) or answer directly.
3.  **Observe**: The tool output is fed back to the LLM.
4.  **Response**: The LLM formulates the final answer.

### Tech Stack
*   **Core**: Rust 🦀
*   **Local Inference**: `mistralrs` / `candle-core` (runs GGUF models locally).
*   **Cloud**: HTTP Clients for Gemini/OpenAI/Anthropic.
*   **Vector Database**: `sqlite` + `langchain-rust` for RAG.

### Building for Production
```bash
cargo build --release
```

### License
MIT
