@echo off
REM Build script for Argus - The All-Seeing File Search Tool

setlocal EnableDelayedExpansion

echo.
echo      █████╗ ██████╗  ██████╗ ██╗   ██╗███████╗
echo     ██╔══██╗██╔══██╗██╔════╝ ██║   ██║██╔════╝
echo     ███████║██████╔╝██║  ███╗██║   ██║███████╗
echo     ██╔══██║██╔══██╗██║   ██║██║   ██║╚════██║
echo     ██║  ██║██║  ██║╚██████╔╝╚██████╔╝███████║
echo     ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝  ╚═════╝ ╚══════╝
echo.
echo Build Script for Windows
echo.

REM Parse arguments
set WITH_OCR=false
set INSTALL=false

:parse_args
if "%~1"=="" goto done_parsing
if /i "%~1"=="--ocr" (
    set WITH_OCR=true
    shift
    goto parse_args
)
if /i "%~1"=="--install" (
    set INSTALL=true
    shift
    goto parse_args
)
if /i "%~1"=="--help" goto show_help
if /i "%~1"=="-h" goto show_help
shift
goto parse_args

:show_help
echo Usage: build.bat [OPTIONS]
echo.
echo Options:
echo   --ocr      Build with OCR support (requires Tesseract)
echo   --install  Install to %%USERPROFILE%%\.cargo\bin after building
echo   --help     Show this help message
exit /b 0

:done_parsing

REM Check for Rust
where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo [ERROR] Rust/Cargo not found!
    echo Please install Rust from https://rustup.rs
    exit /b 1
)

for /f "tokens=*" %%i in ('rustc --version') do set RUST_VERSION=%%i
echo [OK] Rust found: %RUST_VERSION%
echo.

REM Check for Tesseract if OCR is enabled
if "%WITH_OCR%"=="true" (
    echo Checking for Tesseract...
    where tesseract >nul 2>nul
    if !errorlevel! neq 0 (
        echo [WARNING] Tesseract not found!
        echo Download from: https://github.com/UB-Mannheim/tesseract/wiki
        echo.
        set /p CONTINUE="Continue without OCR? [y/N] "
        if /i not "!CONTINUE!"=="y" exit /b 1
        set WITH_OCR=false
    ) else (
        echo [OK] Tesseract found
    )
    echo.
)

REM Build
echo Building Argus...
echo.

if "%WITH_OCR%"=="true" (
    echo Building with OCR support...
    cargo build --release --features ocr
) else (
    echo Building without OCR support...
    cargo build --release
)

if %errorlevel% equ 0 (
    echo.
    echo [SUCCESS] Build successful!

    set BINARY=target\release\argus.exe
    if exist "!BINARY!" (
        for %%A in ("!BINARY!") do set SIZE=%%~zA
        set /a SIZE_KB=!SIZE!/1024
        echo Binary size: !SIZE_KB! KB
        echo Binary location: !BINARY!
    )

    if "%INSTALL%"=="true" (
        echo.
        echo Installing to %%USERPROFILE%%\.cargo\bin...
        cargo install --path .
        echo [SUCCESS] Installed successfully!
        echo Run 'argus --help' to get started.
    ) else (
        echo.
        echo To install, run:
        echo   cargo install --path .
        echo Or:
        echo   build.bat --install
    )
) else (
    echo.
    echo [ERROR] Build failed!
    exit /b 1
)

echo.
echo Done!
endlocal
