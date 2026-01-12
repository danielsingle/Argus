#!/bin/bash
# Build script for Argus - The All-Seeing File Search Tool

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}"
echo "     █████╗ ██████╗  ██████╗ ██╗   ██╗███████╗"
echo "    ██╔══██╗██╔══██╗██╔════╝ ██║   ██║██╔════╝"
echo "    ███████║██████╔╝██║  ███╗██║   ██║███████╗"
echo "    ██╔══██║██╔══██╗██║   ██║██║   ██║╚════██║"
echo "    ██║  ██║██║  ██║╚██████╔╝╚██████╔╝███████║"
echo "    ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝  ╚═════╝ ╚══════╝"
echo -e "${NC}"
echo -e "${YELLOW}Build Script${NC}"
echo ""

# Parse arguments
WITH_OCR=false
INSTALL=false

for arg in "$@"; do
    case $arg in
        --ocr)
            WITH_OCR=true
            shift
            ;;
        --install)
            INSTALL=true
            shift
            ;;
        --help|-h)
            echo "Usage: ./build.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --ocr      Build with OCR support (requires Tesseract)"
            echo "  --install  Install to ~/.cargo/bin after building"
            echo "  --help     Show this help message"
            exit 0
            ;;
    esac
done

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Rust/Cargo not found!${NC}"
    echo "Please install Rust from https://rustup.rs"
    exit 1
fi

echo -e "${GREEN}Rust found:${NC} $(rustc --version)"
echo ""

# Check for Tesseract if OCR is enabled
if [ "$WITH_OCR" = true ]; then
    echo -e "${YELLOW}Checking for Tesseract...${NC}"
    if ! command -v tesseract &> /dev/null; then
        echo -e "${RED}Warning: Tesseract not found!${NC}"
        echo "Install with:"
        echo "  Ubuntu/Debian: sudo apt install tesseract-ocr libtesseract-dev libleptonica-dev"
        echo "  Fedora: sudo dnf install tesseract tesseract-devel leptonica-devel"
        echo "  macOS: brew install tesseract"
        echo ""
        read -p "Continue without OCR? [y/N] " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
        WITH_OCR=false
    else
        echo -e "${GREEN}Tesseract found:${NC} $(tesseract --version 2>&1 | head -n 1)"
    fi
    echo ""
fi

# Build
echo -e "${CYAN}Building Argus...${NC}"
echo ""

if [ "$WITH_OCR" = true ]; then
    echo -e "${YELLOW}Building with OCR support...${NC}"
    cargo build --release --features ocr
else
    echo -e "${YELLOW}Building without OCR support...${NC}"
    cargo build --release
fi

# Check if build succeeded
if [ $? -eq 0 ]; then
    echo ""
    echo -e "${GREEN}Build successful!${NC}"

    # Get binary size
    BINARY="target/release/argus"
    if [ -f "$BINARY" ]; then
        SIZE=$(du -h "$BINARY" | cut -f1)
        echo -e "Binary size: ${CYAN}$SIZE${NC}"
        echo -e "Binary location: ${CYAN}$BINARY${NC}"
    fi

    # Install if requested
    if [ "$INSTALL" = true ]; then
        echo ""
        echo -e "${YELLOW}Installing to ~/.cargo/bin...${NC}"
        cargo install --path .
        echo -e "${GREEN}Installed successfully!${NC}"
        echo -e "Run ${CYAN}argus --help${NC} to get started."
    else
        echo ""
        echo "To install, run:"
        echo -e "  ${CYAN}cargo install --path .${NC}"
        echo "Or:"
        echo -e "  ${CYAN}./build.sh --install${NC}"
    fi
else
    echo ""
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}Done!${NC}"
