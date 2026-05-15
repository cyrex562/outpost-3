#!/usr/bin/env python3
"""
deploy.py — Deploy Outpost 3 to a remote Debian server.

Usage:
    python3 bin/deploy.py [--host HOST] [--user USER] [--key KEY] [--skip-playwright]

Defaults:
    host  : outpost-test-srv-1  (or use 100.88.140.56)
    user  : box-admin
    key   : ~/.ssh/homelab_ssh_key
"""

import argparse
import subprocess
import sys
import os
import textwrap
from pathlib import Path

# ─── Defaults ────────────────────────────────────────────────────────────────

DEFAULT_HOST = "outpost-test-srv-1"
DEFAULT_USER = "box-admin"
DEFAULT_KEY  = str(Path.home() / ".ssh" / "homelab_ssh_key")

REMOTE_DEPLOY_DIR = "/opt/outpost"
REMOTE_SERVICE    = "outpost"
APP_PORT          = 8080
RUST_TARGET       = "x86_64-unknown-linux-gnu"

PROJECT_ROOT = Path(__file__).resolve().parent.parent

# ─── Helpers ─────────────────────────────────────────────────────────────────

def banner(msg: str) -> None:
    print(f"\n\033[1;34m{'─' * 60}\033[0m")
    print(f"\033[1;34m  {msg}\033[0m")
    print(f"\033[1;34m{'─' * 60}\033[0m")


def run(cmd: list[str], cwd: Path | None = None, check: bool = True) -> subprocess.CompletedProcess:
    print(f"  $ {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=cwd, check=check)
    return result


def ssh_opts(key: str) -> list[str]:
    return [
        "-i", key,
        "-o", "StrictHostKeyChecking=accept-new",
        "-o", "BatchMode=yes",
    ]


def ssh(host: str, user: str, key: str, remote_cmd: str) -> None:
    run(["ssh", *ssh_opts(key), f"{user}@{host}", remote_cmd])


def rsync(src: str, dest_user: str, dest_host: str, dest_path: str, key: str,
          extra_args: list[str] | None = None) -> None:
    ssh_arg = f"ssh {' '.join(ssh_opts(key))}"
    cmd = [
        "rsync", "-avz", "--delete",
        *(extra_args or []),
        "-e", ssh_arg,
        src,
        f"{dest_user}@{dest_host}:{dest_path}",
    ]
    run(cmd)


# ─── Steps ───────────────────────────────────────────────────────────────────

def build_release(target: str) -> Path:
    banner("Building release binary")
    run(
        ["cargo", "build", "--release", "--target", target, "-p", "outpost-server"],
        cwd=PROJECT_ROOT,
    )
    binary = PROJECT_ROOT / "target" / target / "release" / "outpost-server"
    if not binary.exists():
        print(f"ERROR: expected binary not found at {binary}", file=sys.stderr)
        sys.exit(1)
    size_mb = binary.stat().st_size / 1_048_576
    print(f"  Binary: {binary}  ({size_mb:.1f} MB)")
    return binary


def provision_server(host: str, user: str, key: str) -> None:
    banner("Provisioning server (directories, packages)")

    setup_script = textwrap.dedent(f"""\
        set -e

        # Create deploy directory owned by the service user
        sudo mkdir -p {REMOTE_DEPLOY_DIR}/data
        sudo chown -R {user}:{user} {REMOTE_DEPLOY_DIR}

        # Install runtime dependencies
        export DEBIAN_FRONTEND=noninteractive
        sudo apt-get update -qq
        sudo apt-get install -y -qq ca-certificates curl rsync
    """)
    ssh(host, user, key, setup_script)


def upload_artifacts(host: str, user: str, key: str, binary: Path) -> None:
    banner("Uploading binary and assets")

    # Stop service first so the running binary isn't locked during overwrite
    ssh(host, user, key,
        f"sudo systemctl stop {REMOTE_SERVICE} 2>/dev/null || true")

    # Binary
    run(["scp", *ssh_opts(key), str(binary),
         f"{user}@{host}:{REMOTE_DEPLOY_DIR}/outpost-server"])

    # Mark executable (scp preserves perms, but be explicit)
    ssh(host, user, key, f"chmod +x {REMOTE_DEPLOY_DIR}/outpost-server")

    # Assets that the server reads from CWD at runtime.
    # content/ lives at the project root; templates/ and static/ live under
    # crates/outpost-server/ (that's the path the binary expects at runtime).
    assets = [
        (PROJECT_ROOT / "content",                              f"{REMOTE_DEPLOY_DIR}/content/"),
        (PROJECT_ROOT / "crates/outpost-server/templates",     f"{REMOTE_DEPLOY_DIR}/crates/outpost-server/templates/"),
        (PROJECT_ROOT / "crates/outpost-server/static",        f"{REMOTE_DEPLOY_DIR}/crates/outpost-server/static/"),
    ]
    for src, dest in assets:
        if src.exists():
            ssh(host, user, key, f"mkdir -p {dest}")
            rsync(str(src) + "/", user, host, dest, key)
        else:
            print(f"  WARNING: {src} not found locally, skipping")


def write_config(host: str, user: str, key: str) -> None:
    banner("Writing config.toml on server")

    config_content = textwrap.dedent(f"""\
        [server]
        host = "0.0.0.0"
        port = {APP_PORT}

        [database]
        path = "{REMOTE_DEPLOY_DIR}/data/outpost3.db"

        [game]
        starting_credits = 10000
        turn_duration_seconds = 60
        tick_rate_ms = 60000
        default_speed_multiplier = 1
        max_speed_multiplier = 10
        idle_safety_enabled = true
        suppress_autopause_in_idle_mode = true
        auto_save_interval_ticks = 10
        save_directory = "{REMOTE_DEPLOY_DIR}/data/saves"
        max_autosaves = 5
    """)

    # Write via heredoc to avoid shell escaping issues
    escaped = config_content.replace("'", "'\\''")
    ssh(host, user, key,
        f"cat > {REMOTE_DEPLOY_DIR}/config.toml << 'EOCONFIG'\n{config_content}\nEOCONFIG")


def install_systemd_service(host: str, user: str, key: str) -> None:
    banner("Installing systemd service")

    unit = textwrap.dedent(f"""\
        [Unit]
        Description=Outpost 3 Game Server
        After=network.target

        [Service]
        Type=simple
        User={user}
        WorkingDirectory={REMOTE_DEPLOY_DIR}
        ExecStart={REMOTE_DEPLOY_DIR}/outpost-server
        Restart=on-failure
        RestartSec=5
        Environment=RUST_LOG=info

        [Install]
        WantedBy=multi-user.target
    """)

    script = textwrap.dedent(f"""\
        set -e
        cat > /tmp/outpost.service << 'EOUNIT'
{unit}
EOUNIT
        sudo mv /tmp/outpost.service /etc/systemd/system/{REMOTE_SERVICE}.service
        sudo systemctl daemon-reload
        sudo systemctl enable {REMOTE_SERVICE}
        sudo systemctl restart {REMOTE_SERVICE}
        sleep 2
        sudo systemctl status {REMOTE_SERVICE} --no-pager
    """)
    ssh(host, user, key, script)


def open_firewall_port(host: str, user: str, key: str) -> None:
    banner(f"Opening firewall port {APP_PORT} (ufw if present)")

    script = textwrap.dedent(f"""\
        set -e
        if command -v ufw >/dev/null 2>&1; then
            sudo ufw allow {APP_PORT}/tcp || true
            echo "ufw: port {APP_PORT} allowed"
        else
            echo "ufw not found, skipping firewall rule (configure manually if needed)"
        fi
    """)
    ssh(host, user, key, script)


def install_playwright(host: str, user: str, key: str) -> None:
    banner("Installing Node.js and Playwright")

    script = textwrap.dedent("""\
        set -e
        export DEBIAN_FRONTEND=noninteractive

        # Install Node.js 20 LTS via NodeSource
        if ! command -v node >/dev/null 2>&1; then
            curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
            sudo apt-get install -y -qq nodejs
        fi

        node --version
        npm --version

        # Install Playwright CLI and browsers system-wide
        sudo npm install -g @playwright/test
        sudo npx playwright install --with-deps chromium

        echo ""
        echo "Playwright ready. Run tests with:"
        echo "  npx playwright test"
    """)
    ssh(host, user, key, script)


def smoke_test(host: str, user: str, key: str) -> None:
    banner("Smoke test — checking HTTP response")

    script = textwrap.dedent(f"""\
        set -e
        sleep 1
        STATUS=$(curl -s -o /dev/null -w "%{{http_code}}" http://localhost:{APP_PORT}/)
        echo "HTTP status: $STATUS"
        if [ "$STATUS" -ge 200 ] && [ "$STATUS" -lt 400 ]; then
            echo "✓ Server is responding"
        else
            echo "✗ Unexpected status $STATUS — check: sudo journalctl -u outpost -n 50"
            exit 1
        fi
    """)
    ssh(host, user, key, script)


# ─── Main ────────────────────────────────────────────────────────────────────

def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Deploy Outpost 3 to a remote Debian server.")
    p.add_argument("--host", default=DEFAULT_HOST,
                   help=f"SSH hostname or IP (default: {DEFAULT_HOST})")
    p.add_argument("--user", default=DEFAULT_USER,
                   help=f"SSH username (default: {DEFAULT_USER})")
    p.add_argument("--key", default=DEFAULT_KEY,
                   help=f"Path to SSH private key (default: {DEFAULT_KEY})")
    p.add_argument("--skip-build", action="store_true",
                   help="Skip cargo build (reuse existing binary)")
    p.add_argument("--skip-playwright", action="store_true",
                   help="Skip Playwright installation")
    p.add_argument("--target", default=RUST_TARGET,
                   help=f"Rust cross-compile target (default: {RUST_TARGET})")
    return p.parse_args()


def main() -> None:
    args = parse_args()

    print(f"\n\033[1;32mOutpost 3 — Deploy to {args.user}@{args.host}\033[0m")
    print(f"  SSH key : {args.key}")
    print(f"  Remote  : {REMOTE_DEPLOY_DIR}")
    print(f"  Port    : {APP_PORT}")
    print(f"  Target  : {args.target}")

    if not Path(args.key).exists():
        print(f"\nERROR: SSH key not found: {args.key}", file=sys.stderr)
        sys.exit(1)

    if args.skip_build:
        binary = PROJECT_ROOT / "target" / args.target / "release" / "outpost-server"
        if not binary.exists():
            print(f"ERROR: --skip-build specified but binary not found at {binary}",
                  file=sys.stderr)
            sys.exit(1)
        banner("Skipping build (--skip-build)")
    else:
        binary = build_release(args.target)

    provision_server(args.host, args.user, args.key)
    upload_artifacts(args.host, args.user, args.key, binary)
    write_config(args.host, args.user, args.key)
    install_systemd_service(args.host, args.user, args.key)
    open_firewall_port(args.host, args.user, args.key)

    if not args.skip_playwright:
        install_playwright(args.host, args.user, args.key)

    smoke_test(args.host, args.user, args.key)

    banner("Deployment complete ✓")
    print(f"\n  App URL : http://{args.host}:{APP_PORT}/")
    print(f"  Logs    : ssh -i {args.key} {args.user}@{args.host} 'sudo journalctl -u {REMOTE_SERVICE} -f'")
    print(f"  Restart : ssh -i {args.key} {args.user}@{args.host} 'sudo systemctl restart {REMOTE_SERVICE}'\n")


if __name__ == "__main__":
    main()
