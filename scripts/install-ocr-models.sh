#!/usr/bin/env bash
set -euo pipefail
model_dir="$1/tessdata"
mkdir -p "$model_dir"
for lang in eng fra; do
  expected='7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2'
  if [ "$lang" = fra ]; then expected='ced037562e8c80c13122dece28dd477d399af80911a28791a66a63ac1e3445ca'; fi
  if [ ! -f "$model_dir/$lang.traineddata" ]; then
    curl -fL --retry 3 "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/main/$lang.traineddata" -o "$model_dir/$lang.traineddata.part"
    printf '%s  %s\n' "$expected" "$model_dir/$lang.traineddata.part" | shasum -a 256 -c -
    mv "$model_dir/$lang.traineddata.part" "$model_dir/$lang.traineddata"
  fi
done
