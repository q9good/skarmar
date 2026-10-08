"""A local Tencent Docs MCP client. Never prints or persists the Authorization header."""

import argparse
import asyncio
import json
import logging
import os
import re
import sys
from datetime import timedelta
from pathlib import Path

ENDPOINT = "https://docs.qq.com/openapi/mcp"
READ_PREFIX = re.compile(r"^(?:query|get|list)_", re.IGNORECASE)
MUTATION_WORDS = re.compile(r"(?:^|_)(?:create|add|update|delete|remove|modify|set|execute|send|upload|enable|disable)(?:_|$)", re.IGNORECASE)


def credential(path: Path) -> str:
    value = path.read_text(encoding="utf-8-sig").strip()
    value = re.sub(r"(?i)^(?:token|authorization|tencent_docs_token)\s*[:=]\s*", "", value, count=1).strip()
    if len(value) > 1 and value[0] == value[-1] and value[0] in "\"'":
        value = value[1:-1]
    if not value or "\n" in value or "\r" in value:
        raise ValueError("Credential file has an unsupported format")
    return value


def private_output(path: Path) -> Path:
    resolved = path.resolve()
    checkout = Path(__file__).resolve().parents[1]
    if resolved == checkout or checkout in resolved.parents or any(
        (parent / '.git').is_file() or (parent / '.git' / 'HEAD').is_file() for parent in resolved.parents
    ):
        raise ValueError("Save Tencent responses outside the checkout")
    resolved.parent.mkdir(parents=True, exist_ok=True)
    return resolved


def write_private(path: Path, value: object) -> None:
    # An atomic file replacement avoids leaving a partial snapshot after interruption.
    temp = path.with_suffix(path.suffix + ".tmp")
    with open(temp, "w", encoding="utf-8", opener=lambda name, flags: os.open(name, flags, 0o600)) as out:
        json.dump(value, out, ensure_ascii=False, indent=2)
    temp.replace(path)
    path.chmod(0o600)


async def run(args) -> None:
    from mcp import ClientSession
    from mcp.client.streamable_http import streamablehttp_client

    output = private_output(args.output)
    token = credential(args.credential_file)
    # The inherited HTTPS proxy and CA trust stay active. No Bearer prefix is added.
    async with streamablehttp_client(ENDPOINT, headers={"Authorization": token},
                                   timeout=timedelta(seconds=25), sse_read_timeout=timedelta(seconds=60)) as streams:
        async with ClientSession(streams[0], streams[1], read_timeout_seconds=timedelta(seconds=60)) as session:
            initialization = await session.initialize()
            tools = []
            cursor = None
            seen_cursors = set()
            while True:
                page = await session.list_tools(cursor=cursor)
                tools.extend(page.tools)
                if not page.nextCursor:
                    break
                if page.nextCursor in seen_cursors:
                    raise RuntimeError("Tool catalog pagination did not advance")
                seen_cursors.add(page.nextCursor)
                cursor = page.nextCursor
            if args.command == "catalog":
                write_private(output, {"server": initialization.serverInfo.model_dump(mode="json"),
                    "tools": [tool.model_dump(mode="json") for tool in tools]})
                names = [tool.name for tool in tools if READ_PREFIX.search(tool.name) and not MUTATION_WORDS.search(tool.name)]
                print(json.dumps({"connected": True, "tool_count": len(tools), "read_tool_names": names}, ensure_ascii=False))
                return
            tool = next((tool for tool in tools if tool.name == args.tool), None)
            if not tool or not READ_PREFIX.search(tool.name) or MUTATION_WORDS.search(tool.name):
                raise ValueError("Only an inspected query/get/list tool is permitted")
            arguments = json.loads(args.arguments_file.read_text(encoding="utf-8"))
            if not isinstance(arguments, dict):
                raise ValueError("Tool arguments must be a JSON object")
            result = await session.call_tool(tool.name, arguments)
            write_private(output, result.model_dump(mode="json"))
            print(json.dumps({"tool": tool.name, "is_error": bool(result.isError), "output_saved": True}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["catalog", "call"])
    parser.add_argument("--credential-file", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--tool")
    parser.add_argument("--arguments-file", type=Path)
    args = parser.parse_args()
    if args.command == "call" and (not args.tool or not args.arguments_file):
        parser.error("call requires --tool and --arguments-file")
    logging.basicConfig(level=logging.CRITICAL)
    try:
        asyncio.run(run(args))
    except BaseException as error:
        if isinstance(error, KeyboardInterrupt):
            raise SystemExit(130)
        # Exception details may contain request metadata. Report only the type here.
        print(f"Tencent read-only connection failed ({type(error).__name__}); check network policy and credential readiness.", file=sys.stderr)
        raise SystemExit(1)


if __name__ == "__main__":
    main()
