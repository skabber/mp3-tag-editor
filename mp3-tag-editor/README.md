# MP3 Tag Editor

A web-based MP3 ID3 tag editor built with Rust and Dioxus framework. Edit tags, chapter markers, and chapter art directly in your browser.

## Features

- **Load MP3 files** from your local machine or a public URL
- **Edit ID3 tags**: Title, Artist, Album, Year, Genre, Track, Disc, Composer, Comment
- **Chapter Markers**: Add, edit, and remove chapters with precise millisecond timestamps
- **Chapter Art**: Attach images to individual chapters
- **ID3v2.4 Support**: Tags are written in ID3v2.4 format for maximum compatibility

## Prerequisites

- Rust toolchain (latest stable)
- `wasm32-unknown-unknown` target for web builds
- **Nix** (optional, for using the included flake)

## Nix Flake Setup

This project includes a `flake.nix` for reproducible builds with all dependencies.

### Enter the development shell

```sh
nix develop
```

### Build the project

```sh
# With Dioxus CLI
dx build --release
dx serve --release

# Or manually with wasm32 target
cargo build --release --target wasm32-unknown-unknown
```

### Notes on Flake

- The flake provides all build dependencies including Rust, wasm-pack, clang, and OpenSSL dev libraries
- If you're on a non-NixOS system, you may need to enable flakes in your `nix.conf`:

```sh
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf
```

## Quick Start (Without Nix)

### 1. Install Dioxus CLI

```sh
cargo install dioxus-cli
```

### 2. Build for Web

```sh
cd mp3-tag-editor
dx build --release
```

### 3. Serve the Application

```sh
dx serve --release
```

Open your browser to `http://localhost:8080`

## Manual Build (Without Dioxus CLI)

### Install WASM target

```sh
rustup target add wasm32-unknown-unknown
```

### Build release

```sh
cargo build --release --target wasm32-unknown-unknown
```

### Serve with any static file server

```sh
# Using Python
cd dist
python3 -m http.server 8080

# Or using npx
npx serve .
```

## Usage

1. **Load a file**:
   - Click "Select local file" to choose an MP3 from your computer
   - Or paste a public URL and click "Load from URL"

2. **Edit tags**: Modify the fields in the "Basic Tags" section

3. **Add chapters**:
   - Fill in Element ID (e.g., "chap1"), Title, Start Time, and End Time (in milliseconds)
   - Click "Add Chapter"
   - Upload chapter art by clicking the file input within the chapter

4. **Remove chapters**: Click the × button on any chapter

5. **Save**: Click "Save Tags" to write the modified ID3 tags back to the file

## Project Structure

```
mp3-tag-editor/
├── Cargo.toml                # Rust dependencies
├── Cargo.lock                # Dependency lock file
├── Dioxus.toml               # Dioxus configuration
├── index.html                # Web entry point
├── flake.nix                 # Nix development environment
├── README.md                 # This file
├── .gitignore                # Git ignore patterns
├── .github/
│   └── workflows/
│       └── ci.yml           # GitHub Actions CI
└── src/
    └── main.rs              # Application source code
```

## Dependencies

- **dioxus** - UI framework
- **id3** - ID3 tag reading/writing
- **base64** - Image encoding for chapter art
- **reqwest** - HTTP client for URL loading
- **serde** - Serialization

## Notes

- Chapter times are specified in **milliseconds** (e.g., 1 minute = 60000 ms, 30 seconds = 30000 ms)
- Chapter art is embedded directly within each chapter's frames (ID3v2 spec compliant)
- For URL loading, the file must be accessible via a direct download link
- When saving, the modified MP3 file is downloaded to your browser's default download location
- The app preserves chapter art MIME types (PNG, GIF, WebP, JPEG)

## License

MIT