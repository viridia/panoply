#!/usr/bin/env python3
"""Ask a local Ollama model the same questions about each of several source files.

    scripts/local_sweep.py --model gpt-oss:20b --questions Q.md --out DIR FILE...

Writes one answer per file to DIR/<model>/<file name>.md, headed by the model, the file's size,
the tokens Ollama read and the seconds taken. The source goes to the model with line numbers, and
the model is told to cite them, so a claim can be checked at its line rather than by rereading
the file. Reads only the files given; writes only under DIR.

    --ctx N        context per request, in tokens (default 32768); the server's own setting
                   (OLLAMA_CONTEXT_LENGTH) is not needed
    --think LEVEL  off (default), on, or low / medium / high for models that take a level
    --host URL     Ollama's address (default http://localhost:11434)

A file whose estimated size leaves too little of the context for the answer is refused, since
Ollama drops the start of a prompt that does not fit, without an error. An answer whose prompt
Ollama counted far shorter than estimated is marked TRUNCATED? as well.
"""

import argparse
import json
import re
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

# Characters per token, set low so the estimate runs high. Line-numbered TypeScript measured 2.5
# to 2.8 under qwen3.6, and 3.0 to 3.4 under gpt-oss.
CHARS_PER_TOKEN = 2.2
# Tokens kept free for the answer, and for the reasoning before it when thinking.
RESERVE = 4096
RESERVE_THINKING = 12288
# Below this fraction of the estimate, the prompt was probably cut.
TRUNCATION_RATIO = 0.5

SYSTEM = """You read source code and report facts about it.

Answer each question below about the one file you are given. Rules:
- Cite the line numbers each claim rests on, as (L12) or (L12-L40).
- Report only what this file shows. When it does not answer a question, write \
"Not in this file." and, if the file names where the answer lives (an import, a call), say \
where.
- Do not guess, generalize or recommend. A short answer that is right beats a long one.
- Answer in Markdown, one section per question, headed by the question.

Questions:

"""


def numbered(text):
    return "\n".join(f"{i:>5}| {line}" for i, line in enumerate(text.split("\n"), 1))


def ask(host, model, system, user, ctx, think):
    body = {
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "stream": False,
        "think": think,
        "options": {"num_ctx": ctx, "temperature": 0},
    }
    request = urllib.request.Request(
        f"{host}/api/chat",
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=3600) as response:
        return json.load(response)


def think_value(level):
    return {"off": False, "on": True}.get(level, level)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--model", required=True)
    parser.add_argument("--questions", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--ctx", type=int, default=32768)
    parser.add_argument("--think", default="off", choices=["off", "on", "low", "medium", "high"])
    parser.add_argument("--host", default="http://localhost:11434")
    parser.add_argument("files", nargs="+", type=Path)
    args = parser.parse_args()

    system = SYSTEM + args.questions.read_text(encoding="utf-8")
    reserve = RESERVE if args.think == "off" else RESERVE_THINKING
    out_dir = args.out / re.sub(r"[^A-Za-z0-9._-]+", "_", args.model)
    out_dir.mkdir(parents=True, exist_ok=True)
    failures = 0

    for path in args.files:
        source = path.read_text(encoding="utf-8")
        user = f"File: {path.name}\n\n{numbered(source)}"
        estimate = int((len(system) + len(user)) / CHARS_PER_TOKEN)
        if estimate + reserve > args.ctx:
            print(f"{path}: refused, about {estimate} tokens, over --ctx {args.ctx} less "
                  f"{reserve} for the answer", file=sys.stderr)
            failures += 1
            continue

        start = time.monotonic()
        try:
            reply = ask(args.host, args.model, system, user, args.ctx, think_value(args.think))
        except urllib.error.URLError as err:
            print(f"{path}: {err}", file=sys.stderr)
            failures += 1
            continue
        seconds = time.monotonic() - start

        read = reply.get("prompt_eval_count", 0)
        written = reply.get("eval_count", 0)
        cut = read < estimate * TRUNCATION_RATIO
        thinking = reply["message"].get("thinking") or ""
        header = [
            f"<!-- model: {args.model}, think: {args.think}, ctx: {args.ctx} -->",
            f"<!-- source: {path} ({len(source)} bytes, {source.count(chr(10)) + 1} lines) -->",
            f"<!-- tokens read: {read} (estimated {estimate}), written: {written}, "
            f"thinking: {len(thinking)} chars, seconds: {seconds:.0f} -->",
        ]
        if cut:
            header.append("<!-- TRUNCATED? Ollama read far fewer tokens than were sent. -->")
        answer = path.name + ".md"
        (out_dir / answer).write_text(
            "\n".join(header) + "\n\n" + reply["message"]["content"].strip() + "\n",
            encoding="utf-8",
        )
        flag = "  TRUNCATED?" if cut else ""
        print(f"{path.name}: {seconds:.0f} s, read {read}, wrote {written}{flag}")

    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
