#!/usr/bin/env python3
# GROUP: 42
# MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
"""
Secure Chat Protocol - Testing CLI

A comprehensive tool for testing the secure chat protocol (servers and clients).
Provides server management capabilities and will be extended to support client testing.
"""

import argparse
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Dict, Optional

import psutil
from rich.console import Console
from rich.table import Table
from rich.panel import Panel


class TestManager:
    """Manages testing for the secure chat protocol (servers and clients)."""

    def __init__(self):
        self.console = Console()
        self.test_dir = Path(__file__).parent
        self.project_root = self.test_dir.parent
        self.server_dir = self.project_root / "server"
        self.logs_dir = self.test_dir / "logs"
        self.configs_dir = self.test_dir / "configs"
        self.keys_dir = self.test_dir / "keys"

        # Server configurations
        self.servers = {
            1: {
                "port": 3001,
                "name": "Bootstrap Server",
                "config": "server1_config.yaml",
            },
            2: {
                "port": 3002,
                "name": "Client Server 1",
                "config": "server2_config.yaml",
            },
            3: {
                "port": 3003,
                "name": "Client Server 2",
                "config": "server3_config.yaml",
            },
        }

        # Ensure directories exist
        self.logs_dir.mkdir(exist_ok=True)

    def build_server(self) -> bool:
        """Build the server binary."""
        self.console.print("Building server...", style="yellow")

        try:
            result = subprocess.run(
                ["cargo", "build"],
                cwd=self.server_dir,
                capture_output=False,  # Show cargo output
                text=True,
                timeout=120,
            )

            if result.returncode == 0:
                self.console.print("Server built successfully", style="green")
                return True
            else:
                self.console.print(f"Build failed:\n{result.stderr}", style="red")
                return False

        except subprocess.TimeoutExpired:
            self.console.print("Build timed out", style="red")
            return False
        except Exception as e:
            self.console.print(f"Build error: {e}", style="red")
            return False

    def is_port_in_use(self, port: int) -> bool:
        """Check if a port is in use."""
        for conn in psutil.net_connections():
            if conn.laddr.port == port and conn.status == psutil.CONN_LISTEN:
                return True
        return False

    def get_server_pid(self, server_num: int) -> Optional[int]:
        """Get the PID of a running server."""
        pid_file = self.logs_dir / f"server{server_num}.pid"
        if pid_file.exists():
            try:
                pid = int(pid_file.read_text().strip())
                if psutil.pid_exists(pid):
                    return pid
            except (ValueError, OSError):
                pass
        return None

    def save_server_pid(self, server_num: int, pid: int):
        """Save server PID to file."""
        pid_file = self.logs_dir / f"server{server_num}.pid"
        pid_file.write_text(str(pid))

    def get_server_status(self) -> Dict[int, Dict]:
        """Get status of all servers."""
        status = {}
        for server_num, config in self.servers.items():
            port = config["port"]
            pid = self.get_server_pid(server_num)

            status[server_num] = {
                "name": config["name"],
                "port": port,
                "pid": pid,
                "running": self.is_port_in_use(port),
                "config": config["config"],
            }

        return status

    def launch_server(self, server_num: int, background: bool = False) -> bool:
        """Launch a specific server."""
        if server_num not in self.servers:
            self.console.print(f"Invalid server number: {server_num}", style="red")
            return False

        config = self.servers[server_num]
        port = config["port"]

        # Check if already running
        if self.is_port_in_use(port):
            self.console.print(
                f"  Server {server_num} is already running on port {port}",
                style="yellow",
            )
            return True

        # Set up environment
        env = os.environ.copy()
        env.update(
            {
                "CONFIG_FILE": str(self.configs_dir / config["config"]),
                "PRIVATE_KEY_FILE": str(
                    self.keys_dir / f"server{server_num}_private_key.pem"
                ),
                "HOST": "127.0.0.1",
                "PORT": str(port),
            }
        )

        log_file = self.logs_dir / f"server{server_num}.log"

        self.console.print(
            f"Starting Server {server_num} ({config['name']}) on port {port}..."
        )

        try:
            if background:
                # Start in background
                with open(log_file, "w") as f:
                    process = subprocess.Popen(
                        ["cargo", "run"],
                        cwd=self.server_dir,
                        env=env,
                        stdout=f,
                        stderr=subprocess.STDOUT,
                        start_new_session=True,
                    )

                # Wait a moment and check if it started successfully
                # NOTE: might need adjusting if server is failing to start
                time.sleep(5)
                if self.is_port_in_use(port):
                    self.save_server_pid(server_num, process.pid)
                    self.console.print(
                        f"  Server {server_num} started (PID: {process.pid})",
                        style="green",
                    )
                    return True
                else:
                    self.console.print(
                        f"Server {server_num} failed to start", style="red"
                    )
                    return False
            else:
                # Start in foreground with live output
                with open(log_file, "w") as f:
                    process = subprocess.Popen(
                        ["cargo", "run"],
                        cwd=self.server_dir,
                        env=env,
                        stdout=subprocess.PIPE,
                        stderr=subprocess.STDOUT,
                        text=True,
                        bufsize=1,
                    )

                # Stream output to both console and log file
                try:
                    if process.stdout is None:
                        self.console.print(
                            "Failed to capture server output", style="red"
                        )
                        return process.returncode == 0
                    for line in process.stdout:
                        print(line, end="")
                        f.write(line)
                        f.flush()
                except KeyboardInterrupt:
                    process.terminate()
                    self.console.print(
                        f"\n  Server {server_num} stopped", style="yellow"
                    )

                return process.returncode == 0

        except Exception as e:
            self.console.print(f"Failed to start server {server_num}: {e}", style="red")
            return False

    def stop_server(self, server_num: int) -> bool:
        """Stop a specific server."""
        if server_num not in self.servers:
            self.console.print(f"Invalid server number: {server_num}", style="red")
            return False

        config = self.servers[server_num]
        port = config["port"]

        # Try to stop by PID first
        pid = self.get_server_pid(server_num)
        if pid:
            try:
                process = psutil.Process(pid)
                process.terminate()

                # Wait for graceful shutdown
                try:
                    process.wait(timeout=5)
                except psutil.TimeoutExpired:
                    process.kill()

                # Remove PID file
                pid_file = self.logs_dir / f"server{server_num}.pid"
                if pid_file.exists():
                    pid_file.unlink()

                self.console.print(
                    f"  Server {server_num} stopped (PID: {pid})", style="green"
                )
                return True

            except psutil.NoSuchProcess:
                pass

        # Fallback: kill by port
        for conn in psutil.net_connections():
            if conn.laddr.port == port and conn.status == psutil.CONN_LISTEN:
                try:
                    process = psutil.Process(conn.pid)
                    process.terminate()
                    self.console.print(
                        f"  Server {server_num} stopped (port {port})", style="green"
                    )
                    return True
                except psutil.NoSuchProcess:
                    pass

        self.console.print(f"  Server {server_num} was not running", style="yellow")
        return True

    def stop_all_servers(self):
        """Stop all running servers."""
        self.console.print("Stopping all test servers...", style="yellow")

        stopped = 0
        for server_num in self.servers:
            if self.stop_server(server_num):
                stopped += 1

        # Also kill any remaining cargo processes
        for proc in psutil.process_iter(["pid", "name", "cmdline"]):
            try:
                if "cargo" in proc.name() and "run" in proc.cmdline():
                    proc.terminate()
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                pass

        self.console.print(f"Stopped {stopped} servers", style="green")

    def launch_all_servers(self, delay: int = 5) -> bool:
        """Launch all servers in sequence."""
        self.console.print("Launching all test servers...", style="blue")

        # Stop any existing servers first
        self.stop_all_servers()
        time.sleep(1)

        success_count = 0

        for server_num in sorted(self.servers.keys()):
            if self.launch_server(server_num, background=True):
                success_count += 1
                if server_num < max(self.servers.keys()):
                    self.console.print(
                        f"  Waiting {delay}s before starting next server..."
                    )
                    time.sleep(delay)
            else:
                self.console.print(f"Failed to start Server {server_num}", style="red")
                break

        self.console.print(
            f"Successfully started {success_count}/{len(self.servers)} servers",
            style="green",
        )
        return success_count == len(self.servers)

    def show_status(self):
        """Show status of all servers."""
        status = self.get_server_status()

        table = Table(title="Server Status")
        table.add_column("Server", style="cyan")
        table.add_column("Name", style="blue")
        table.add_column("Port", style="magenta")
        table.add_column("Status", style="bold")
        table.add_column("PID", style="dim")
        table.add_column("Config", style="dim")

        for server_num, info in status.items():
            status_text = "Running" if info["running"] else "Stopped"
            status_style = "green" if info["running"] else "red"

            table.add_row(
                f"Server {server_num}",
                info["name"],
                str(info["port"]),
                f"[{status_style}]{status_text}[/{status_style}]",
                str(info["pid"]) if info["pid"] else "-",
                info["config"],
            )

        self.console.print(table)

    def show_logs(self, server_num: int, follow: bool = False, lines: int = 20):
        """Show server logs."""
        if server_num not in self.servers:
            self.console.print(f"Invalid server number: {server_num}", style="red")
            return

        log_file = self.logs_dir / f"server{server_num}.log"

        if not log_file.exists():
            self.console.print(
                f"No log file found for Server {server_num}", style="red"
            )
            return

        if follow:
            # Follow log file (like tail -f)
            try:
                subprocess.run(["tail", "-f", str(log_file)])
            except KeyboardInterrupt:
                pass
        else:
            # Show last N lines
            try:
                result = subprocess.run(
                    ["tail", "-n", str(lines), str(log_file)],
                    capture_output=True,
                    text=True,
                )

                if result.stdout:
                    panel = Panel(
                        result.stdout,
                        title=f"Server {server_num} Logs (last {lines} lines)",
                        border_style="blue",
                    )
                    self.console.print(panel)
                else:
                    self.console.print("Log file is empty", style="yellow")

            except Exception as e:
                self.console.print(f"Error reading logs: {e}", style="red")

    def generate_keys(self):
        """Generate new RSA keys for all servers."""
        self.console.print("Generating new RSA keys...", style="yellow")

        try:
            result = subprocess.run(
                ["cargo", "run", "--bin", "generate_keys"],
                cwd=self.test_dir,
                capture_output=True,
                text=True,
            )

            if result.returncode == 0:
                if result.stdout:
                    self.console.print(result.stdout)
                self.console.print("Keys generated successfully", style="green")
                return True
            else:
                self.console.print(
                    f"Key generation failed:\n{result.stderr}", style="red"
                )
                return False

        except Exception as e:
            self.console.print(f"Key generation error: {e}", style="red")
            return False

    def demo_bootstrap(self):
        """Run a quick bootstrap demo."""
        self.console.print("Running bootstrap demonstration...", style="blue")

        # Stop any existing servers
        self.stop_all_servers()
        time.sleep(1)

        # Start Server 1 (bootstrap server)
        self.console.print("  1. Starting bootstrap server (Server 1)...")
        if not self.launch_server(1, background=True):
            return False

        time.sleep(3)

        # Start Server 2 (will bootstrap from Server 1)
        self.console.print("  2. Starting client server (Server 2)...")
        if not self.launch_server(2, background=True):
            return False

        time.sleep(2)

        # Show status
        self.console.print(
            "Demo servers started! Check status and logs.", style="green"
        )
        self.show_status()

        return True


def main():
    """Main CLI entry point."""
    manager = TestManager()

    parser = argparse.ArgumentParser(
        description="Secure Chat Protocol - Testing CLI",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s status             Show server status
  %(prog)s build              Build the server
  %(prog)s start 1            Start server 1
  %(prog)s start-all          Start all servers
  %(prog)s stop 2             Stop server 2
  %(prog)s stop-all           Stop all servers
  %(prog)s logs 1 --follow    Follow server 1 logs
  %(prog)s demo               Run bootstrap demo
  %(prog)s generate-keys      Generate new RSA keys
        """,
    )

    subparsers = parser.add_subparsers(dest="command", help="Available commands")

    # Status command
    subparsers.add_parser("status", help="Show server status")

    # Build command
    subparsers.add_parser("build", help="Build the server")

    # Start command
    start_parser = subparsers.add_parser("start", help="Start a specific server")
    start_parser.add_argument(
        "server", type=int, choices=[1, 2, 3], help="Server number to start"
    )
    start_parser.add_argument(
        "--background", "-b", action="store_true", help="Start in background"
    )

    # Start all command
    start_all_parser = subparsers.add_parser("start-all", help="Start all servers")
    start_all_parser.add_argument(
        "--delay",
        "-d",
        type=int,
        default=3,
        help="Delay between server starts (seconds)",
    )

    # Stop command
    stop_parser = subparsers.add_parser("stop", help="Stop a specific server")
    stop_parser.add_argument(
        "server", type=int, choices=[1, 2, 3], help="Server number to stop"
    )

    # Stop all command
    subparsers.add_parser("stop-all", help="Stop all servers")

    # Logs command
    logs_parser = subparsers.add_parser("logs", help="Show server logs")
    logs_parser.add_argument(
        "server", type=int, choices=[1, 2, 3], help="Server number"
    )
    logs_parser.add_argument(
        "--follow", "-f", action="store_true", help="Follow log output"
    )
    logs_parser.add_argument(
        "--lines", "-n", type=int, default=20, help="Number of lines to show"
    )

    # Demo command
    subparsers.add_parser("demo", help="Run bootstrap demonstration")

    # Generate keys command
    subparsers.add_parser("generate-keys", help="Generate new RSA keys")

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        return

    try:
        if args.command == "status":
            manager.show_status()

        elif args.command == "build":
            manager.build_server()

        elif args.command == "start":
            manager.launch_server(args.server, background=args.background)

        elif args.command == "start-all":
            manager.launch_all_servers(delay=args.delay)

        elif args.command == "stop":
            manager.stop_server(args.server)

        elif args.command == "stop-all":
            manager.stop_all_servers()

        elif args.command == "logs":
            manager.show_logs(args.server, follow=args.follow, lines=args.lines)

        elif args.command == "demo":
            manager.demo_bootstrap()

        elif args.command == "generate-keys":
            manager.generate_keys()

    except KeyboardInterrupt:
        manager.console.print("\nInterrupt signal received. Goodbye!", style="yellow")
    except Exception as e:
        manager.console.print(f"Error: {e}", style="red")
        sys.exit(1)


if __name__ == "__main__":
    main()