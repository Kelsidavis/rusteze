#!/bin/bash
# Create qwen3-30b-aider:32k model with 32K context support

cd "$(dirname "$0")"

echo "Creating qwen3-30b-aider:32k model..."
ollama create qwen3-30b-aider:32k -f Modelfile.qwen3-32k

if [ $? -eq 0 ]; then
    echo "✓ Model created successfully!"
    echo "You can now restart dev.sh to use the new model."
else
    echo "✗ Failed to create model."
    exit 1
fi
