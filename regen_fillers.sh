#!/bin/bash
# Regenerate filler phrases using Deepgram Aura

set -e

# Load secrets
if [ -f ~/.config/notclicky/secrets.env ]; then
    source ~/.config/notclicky/secrets.env
else
    echo "secrets.env not found"
    exit 1
fi

if [ -z "$DEEPGRAM_API_KEY" ]; then
    echo "DEEPGRAM_API_KEY not set"
    exit 1
fi

SOUNDS_DIR="$(dirname "$0")/resources/sounds"
mkdir -p "$SOUNDS_DIR"

MODEL="aura-2-arcas-en"

declare -A PHRASES=(
    ["one_moment"]="One moment."
    ["let_me_check"]="Let me check."
    ["checking_now"]="Checking now."
    ["sure_thing"]="Sure thing."
    ["right_away"]="Right away."
)

for name in "${!PHRASES[@]}"; do
    text="${PHRASES[$name]}"
    echo "Generating $name: \"$text\""
    
    curl -s -X POST "https://api.deepgram.com/v1/speak?model=$MODEL&encoding=mp3" \
        -H "Authorization: Token $DEEPGRAM_API_KEY" \
        -H "Content-Type: application/json" \
        -d "{\"text\": \"$text\"}" \
        -o "$SOUNDS_DIR/$name.mp3"
    
    if [ $? -eq 0 ]; then
        echo "  Saved to $SOUNDS_DIR/$name.mp3"
    else
        echo "  Failed!"
    fi
done

echo "All filler phrases regenerated!"