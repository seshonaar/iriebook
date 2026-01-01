# IrieBook Tauri UI

This is a **Tauri**-based frontend for IrieBook, serving as an alternative to the COSMIC/Iced UI.

## Tech Stack
- **Frontend**: React + TypeScript + Vite
- **Backend**: Rust (Tauri)
- **Styling**: CSS (currently default Tauri template)

## Prerequisites (Linux)

To build and run this application on Linux, you need to install system dependencies for Tauri.

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev
```

**Fedora:**
```bash
sudo dnf install webkit2gtk3-devel.x86_64 \
    openssl-devel \
    curl \
    wget \
    file \
    libappindicator-gtk3-devel \
    librsvg2-devel
```

*(See [Tauri Prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites#linux) for other distributions)*

## Development

1.  **Install Frontend Dependencies**:
    ```bash
    cd iriebook-tauri-ui
    npm install
    ```

2.  **Run in Development Mode**:
    ```bash
    npm run tauri dev
    ```

## Building

```bash
npm run tauri build
```
