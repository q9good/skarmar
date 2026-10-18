#!/usr/bin/env python3
"""Build a process-private hosts file when a local proxy uses fake-IP DNS.

This does not change /etc/hosts or the host's DNS/proxy settings. Use the output
only in the connector's filesystem namespace; see the deployment runbook.
"""

import argparse
import ipaddress
import json
import os
from pathlib import Path
import tempfile
import urllib.request

EDGE_NAMES = ("region1.v2.argotunnel.com", "region2.v2.argotunnel.com")


def resolve_edges(proxy):
    opener = urllib.request.build_opener(
        urllib.request.ProxyHandler({"https": proxy} if proxy else {})
    )
    result = {}
    for name in EDGE_NAMES:
        request = urllib.request.Request(
            "https://cloudflare-dns.com/dns-query?name=" + name + "&type=A",
            headers={"Accept": "application/dns-json"},
        )
        with opener.open(request, timeout=20) as response:
            answer = json.load(response)
        if answer.get("Status") != 0:
            raise RuntimeError("DNS lookup failed for " + name)
        addresses = []
        for record in answer.get("Answer", []):
            if record.get("type") != 1:
                continue
            address = ipaddress.IPv4Address(record["data"])
            if not address.is_global:
                raise RuntimeError("DNS returned a non-public edge address")
            if str(address) not in addresses:
                addresses.append(str(address))
        if not addresses:
            raise RuntimeError("DNS returned no IPv4 edge addresses for " + name)
        result[name] = addresses
    return result


def hosts_content(original, edges):
    lines = []
    for line in original.splitlines():
        fields = line.split("#", 1)[0].split()
        if any(name in fields[1:] for name in EDGE_NAMES):
            continue
        lines.append(line)
    lines.append("# Process-private Cloudflare edge addresses; regenerated on start.")
    for name in EDGE_NAMES:
        lines.extend(address + " " + name for address in edges[name])
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--proxy", help="Existing HTTPS forward proxy, e.g. http://127.0.0.1:7890")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    edges = resolve_edges(args.proxy)
    content = hosts_content(Path("/etc/hosts").read_text(), edges)
    args.output.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w", dir=args.output.parent, prefix=".edge-hosts-", delete=False
        ) as stream:
            temporary = stream.name
            os.chmod(temporary, 0o600)
            stream.write(content)
        os.replace(temporary, args.output)
        temporary = None
    finally:
        if temporary:
            os.unlink(temporary)
    print("Refreshed private Cloudflare edge DNS for " + str(len(edges)) + " regions")


if __name__ == "__main__":
    main()
