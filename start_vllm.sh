#!/bin/bash
# Start vLLM server with Qwen2.5-Coder optimized for RTX 5080
# 10GB free VRAM allows for larger context and better batching

export CUDA_DEVICE_ORDER=PCI_BUS_ID
export CUDA_VISIBLE_DEVICES=1  # RTX 5080 only

MODEL="Qwen/Qwen2.5-Coder-7B-Instruct"
PORT=8000

echo "Starting vLLM server with Qwen2.5-Coder-7B..."
echo "Model: $MODEL"
echo "Port: $PORT"
echo "VRAM: RTX 5080 (~10GB available)"
echo ""

python3 -m vllm.entrypoints.openai.api_server \
    --model "$MODEL" \
    --host 0.0.0.0 \
    --port $PORT \
    --dtype bfloat16 \
    --max-model-len 32768 \
    --gpu-memory-utilization 0.95 \
    --max-num-seqs 16 \
    --disable-log-requests \
    --trust-remote-code \
    2>&1 | tee vllm.log
