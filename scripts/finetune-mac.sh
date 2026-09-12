#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [ "$(uname -s)" != Darwin ] || [ "$(uname -m)" != arm64 ]; then
  echo 'This recipe requires Apple Silicon and MLX.'
  exit 1
fi
if [ "$#" -lt 1 ]; then
  echo 'Usage: scripts/finetune-mac.sh /path/to/reviewed-export.jsonl [mlx-model-id]'
  exit 1
fi
training_model="${2:-mlx-community/Qwen3-4B-4bit}"
python3 -m venv work/mlx-env
work/mlx-env/bin/pip install -r scripts/requirements-mlx.txt
work/mlx-env/bin/python scripts/prepare_dataset.py "$1" work/training-data
work/mlx-env/bin/python -m mlx_lm.lora --model "$training_model" --train --data work/training-data --batch-size 1 --num-layers 8 --iters 200 --learning-rate 1e-5 --max-seq-length 2048 --adapter-path work/adapters
work/mlx-env/bin/python -m mlx_lm.lora --model "$training_model" --test --data work/training-data --adapter-path work/adapters
work/mlx-env/bin/pip freeze > work/training-data/environment.lock.txt
echo 'Adapter saved under work/adapters. Compare held-out results before deployment.'
