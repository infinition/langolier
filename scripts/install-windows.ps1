$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not (Get-Command winget -ErrorAction SilentlyContinue)) { throw 'Install App Installer (winget) from Microsoft Store first.' }
$packages = @('Ollama.Ollama', 'Gyan.FFmpeg', 'yt-dlp.yt-dlp', 'Kitware.CMake', 'Git.Git', 'UB-Mannheim.TesseractOCR', 'oschwartz10612.Poppler')
foreach ($package in $packages) {
    winget install --id $package --exact --accept-package-agreements --accept-source-agreements
    if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne -1978335189) { throw "winget failed for $package ($LASTEXITCODE)" }
}
$env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' + [Environment]::GetEnvironmentVariable('Path', 'User')
$work = Join-Path $root 'work'
New-Item -ItemType Directory -Force -Path $work | Out-Null
$whisper = Join-Path $work 'whisper.cpp'
if (-not (Test-Path $whisper)) {
    git clone --depth 1 --branch v1.9.2 https://github.com/ggml-org/whisper.cpp.git $whisper
    if ($LASTEXITCODE -ne 0) { throw 'Could not download whisper.cpp.' }
}
cmake -S $whisper -B "$whisper/build" -DWHISPER_SDL2=OFF -DBUILD_SHARED_LIBS=OFF -DGGML_NATIVE=OFF
if ($LASTEXITCODE -ne 0) { throw 'CMake failed. Install Visual Studio Build Tools with Desktop development with C++.' }
cmake --build "$whisper/build" --config Release --parallel
if ($LASTEXITCODE -ne 0) { throw 'Whisper compilation failed.' }
$toolDir = Join-Path $env:LOCALAPPDATA 'Langolier/tools'
New-Item -ItemType Directory -Force -Path $toolDir | Out-Null
Get-ChildItem "$whisper/build" -Recurse -Filter 'whisper-cli.exe' | Select-Object -First 1 | Copy-Item -Destination $toolDir
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -notlike "*$toolDir*") { [Environment]::SetEnvironmentVariable('Path', "$userPath;$toolDir", 'User') }
$modelDir = Join-Path $env:APPDATA 'local.langolier.studio/models'
New-Item -ItemType Directory -Force -Path $modelDir | Out-Null
$model = Join-Path $modelDir 'ggml-base.bin'
if (-not (Test-Path $model)) {
    Invoke-WebRequest 'https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin' -OutFile "$model.part"
    if ((Get-FileHash "$model.part" -Algorithm SHA256).Hash.ToLower() -ne '60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe') { throw 'Whisper model checksum mismatch.' }
    Move-Item "$model.part" $model
}
Start-Process ollama -ArgumentList 'serve' -WindowStyle Hidden
for ($attempt = 0; $attempt -lt 30; $attempt++) {
    try { Invoke-RestMethod 'http://127.0.0.1:11434/api/version' | Out-Null; break } catch { Start-Sleep 1 }
}
ollama pull qwen3:8b
if ($LASTEXITCODE -ne 0) { throw 'Conversation model download failed.' }
ollama pull embeddinggemma
if ($LASTEXITCODE -ne 0) { throw 'Embedding model download failed.' }
Write-Host 'Ready. Restart the terminal or app to refresh PATH, then use Launch-Windows.bat.'

$tessdata = Join-Path $modelDir 'tessdata'
New-Item -ItemType Directory -Force -Path $tessdata | Out-Null
foreach ($lang in @('eng', 'fra')) {
    $expected = '7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2'
    if ($lang -eq 'fra') { $expected = 'ced037562e8c80c13122dece28dd477d399af80911a28791a66a63ac1e3445ca' }
    $file = Join-Path $tessdata "$lang.traineddata"
    if (-not (Test-Path $file)) {
        Invoke-WebRequest "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/main/$lang.traineddata" -OutFile "$file.part"
        if ((Get-FileHash "$file.part" -Algorithm SHA256).Hash.ToLower() -ne $expected) { throw 'OCR model checksum mismatch.' }
        Move-Item "$file.part" $file
    }
}
