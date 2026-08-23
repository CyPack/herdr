"""Drive the installed herdr the way a person drives it, and read what it paints.

A unit test proves a function. This proves the screen: it starts the *installed*
binary under a pseudo-terminal, in a throwaway XDG home, reads the cells it
paints, sends real key and mouse sequences, and stops the server it started.

Why it exists. Between 2026-08-13 and 2026-08-18 twenty-one one-off scripts were
written to answer exactly this question, each rebuilt from scratch, none of them
versioned and none of them run by any gate. Three defects in that window were
invisible to the Rust suite and visible here in seconds: a bar section that
parsed and drew nothing, a documented popup size that could not fit the tool it
launched, and a reload that reported success while changing nothing on screen.

Why the live environment cannot be touched. Every `HERDR_*` variable is stripped
and every XDG root is redirected into a temporary directory, so the socket, the
config and the session of the person running the tests are out of reach by
construction rather than by care. The harness also stops the server it started,
because a leaked server outlives the test and answers the next one.
"""

from __future__ import annotations

import fcntl
import os
import pty
import re
import select
import shutil
import signal
import struct
import subprocess
import tempfile
import termios
import time
from pathlib import Path

INSTALLED_BINARY = Path.home() / ".local" / "bin" / "herdr"

#: The debug build reads `herdr-dev`, the release build reads `herdr`. Writing
#: both means a harness cannot silently measure a default config while believing
#: it measured the written one — a false pass that cost a full session in
#: August 2026 (the config landed in one home, the binary read the other).
CONFIG_HOMES = ("herdr", "herdr-dev")


def binary_available(binary: Path = INSTALLED_BINARY) -> bool:
    """Whether there is an installed binary to drive."""
    return binary.exists() and os.access(binary, os.X_OK)


class HeadfulSession:
    """One isolated herdr, its screen, and the gestures sent to it."""

    def __init__(
        self,
        config_text: str,
        *,
        binary: Path = INSTALLED_BINARY,
        cols: int = 120,
        rows: int = 40,
        root: Path | None = None,
        cwd: Path | str = "/tmp",
    ) -> None:
        self.binary = binary
        self.cols = cols
        self.rows = rows
        # Where the session — and therefore the file manager — starts. The
        # default is the shared temp dir, but a proof that has to SEE its own
        # fixtures needs a directory nothing else writes to: a crowded listing
        # scrolls them off screen and the assertion reads as a product bug.
        self.cwd = str(cwd)
        self._root = root or Path(tempfile.mkdtemp(prefix="herdr-headful-"))
        self._config_text = config_text
        self._proc: subprocess.Popen | None = None
        self._master: int | None = None
        self._screen = None
        self._stream = None

    # ---- lifecycle -------------------------------------------------------

    def __enter__(self) -> "HeadfulSession":
        self.start()
        return self

    def __exit__(self, *_exc) -> None:
        self.stop()

    def _env(self) -> dict[str, str]:
        env = {k: v for k, v in os.environ.items() if not k.startswith("HERDR")}
        env.update(
            {
                "XDG_CONFIG_HOME": str(self._root / "config"),
                "XDG_STATE_HOME": str(self._root / "state"),
                "XDG_DATA_HOME": str(self._root / "data"),
                "XDG_CACHE_HOME": str(self._root / "cache"),
                "TERM": "xterm-256color",
                "COLORTERM": "truecolor",
                "SHELL": "/bin/sh",
            }
        )
        return env

    def write_config(self, config_text: str) -> None:
        """Write the config into both homes; usable mid-session for reload tests."""
        for home in CONFIG_HOMES:
            directory = self._root / "config" / home
            directory.mkdir(parents=True, exist_ok=True)
            (directory / "config.toml").write_text(config_text)
        self._config_text = config_text

    def start(self) -> None:
        import pyte  # imported here so the module can be imported without it

        self.write_config(self._config_text)
        self._screen = pyte.Screen(self.cols, self.rows)
        self._stream = pyte.ByteStream(self._screen)
        master, slave = pty.openpty()
        # Without this the child believes it has an 80x24 terminal whatever the
        # screen says, and every geometry assertion measures the wrong bar.
        fcntl.ioctl(
            slave, termios.TIOCSWINSZ, struct.pack("HHHH", self.rows, self.cols, 0, 0)
        )
        self._proc = subprocess.Popen(
            [str(self.binary)],
            stdin=slave,
            stdout=slave,
            stderr=slave,
            env=self._env(),
            cwd=self.cwd,
            preexec_fn=os.setsid,
        )
        os.close(slave)
        self._master = master

    def stop(self) -> None:
        if self._proc is not None and self._proc.poll() is None:
            try:
                os.killpg(os.getpgid(self._proc.pid), signal.SIGTERM)
                self._proc.wait(timeout=5)
            except Exception:  # noqa: BLE001 - teardown never fails a test
                pass
        if self._master is not None:
            try:
                os.close(self._master)
            except OSError:
                pass
            self._master = None
        # The client dies with the terminal; the server it spawned does not.
        try:
            subprocess.run(
                [str(self.binary), "server", "stop"],
                env=self._env(),
                capture_output=True,
                timeout=20,
                check=False,
            )
        except Exception:  # noqa: BLE001
            pass
        shutil.rmtree(self._root, ignore_errors=True)

    # ---- reading ---------------------------------------------------------

    def settle(self, seconds: float = 15.0) -> None:
        """Read until the app stops sending, or until `seconds` elapse."""
        assert self._master is not None and self._stream is not None
        end = time.time() + seconds
        while time.time() < end:
            if select.select([self._master], [], [], 0.2)[0]:
                try:
                    data = os.read(self._master, 65536)
                except OSError:
                    break
                if not data:
                    break
                self._stream.feed(data)

    def row(self, index: int) -> str:
        """One screen row as text. Reads `buffer`, not `display`: `display`
        collapses styling the way a screenshot would and has hidden real cells."""
        assert self._screen is not None
        return "".join(
            self._screen.buffer[index][col].data or " " for col in range(self.cols)
        )

    def text(self) -> str:
        return "\n".join(self.row(index) for index in range(self.rows))

    def find(self, needle: str, *, within_rows: int | None = None) -> tuple[int, int] | None:
        """Where `needle` sits, as (row, column), or None."""
        limit = self.rows if within_rows is None else min(within_rows, self.rows)
        for index in range(limit):
            column = self.row(index).find(needle)
            if column >= 0:
                return index, column
        return None

    def find_regex(self, pattern: str, *, within_rows: int | None = None):
        limit = self.rows if within_rows is None else min(within_rows, self.rows)
        for index in range(limit):
            match = re.search(pattern, self.row(index))
            if match:
                return index, match.start()
        return None

    # ---- gestures --------------------------------------------------------

    def send(self, data: bytes) -> None:
        assert self._master is not None
        os.write(self._master, data)

    def click(self, row: int, col: int, *, settle: float = 5.0) -> None:
        """A real press and release at (row, col), in SGR mouse encoding.

        Both halves are sent: a press without a release leaves the app in a drag
        it never sees the end of, and the next gesture measures that instead.
        """
        self.send(f"\x1b[<0;{col + 1};{row + 1}M".encode())
        time.sleep(0.15)
        self.send(f"\x1b[<0;{col + 1};{row + 1}m".encode())
        self.settle(settle)

    def right_click(self, row: int, col: int, *, settle: float = 5.0) -> None:
        """A real secondary press and release at (row, col), SGR-encoded.

        Button 2 is the right button in SGR; the release carries the same
        button code. Sending both halves matters more here than for the left:
        a press without its release leaves the app believing a secondary drag
        is still in flight, and every later gesture measures that instead.
        """
        self.send(f"\x1b[<2;{col + 1};{row + 1}M".encode())
        time.sleep(0.15)
        self.send(f"\x1b[<2;{col + 1};{row + 1}m".encode())
        self.settle(settle)

    def reload_config_via_cli(self) -> subprocess.CompletedProcess:
        """Reload through the same command a person would type."""
        return subprocess.run(
            [str(self.binary), "server", "reload-config"],
            env=self._env(),
            capture_output=True,
            timeout=20,
            text=True,
        )
