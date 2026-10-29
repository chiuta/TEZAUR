# Chat bots

Both run on the same machine as the library (or on a server that has it) and call the `tezaur` CLI.

    export TEZAUR_BIN=/usr/bin/tezaur
    export TEZAUR_ARGS="--index /data/tezaur.idx --api ollama --model llama3.1:8b"
    # Telegram
    TELEGRAM_TOKEN=… ALLOWED_CHAT_IDS=12345 python3 telegram_bot.py
    # Matrix (unencrypted rooms)
    MATRIX_HOMESERVER=https://matrix.example.org MATRIX_TOKEN=… MATRIX_ROOMS='!abc:example.org' python3 matrix_bot.py

Privacy: questions and answers pass through the chat service. Telegram bots are never end-to-end encrypted; Matrix is
only if the room is — and this bot cannot join encrypted rooms. Both refuse chats/rooms not on the allow-list.
Tested against mock Telegram/Matrix servers, not the real services.
