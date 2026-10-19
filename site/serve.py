"""Serve the built site locally for browser tests, including parallel viewer module loads."""
import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


class SiteServer(ThreadingHTTPServer):
    # The viewer imports many modules at once. The default five-connection backlog can refuse
    # requests when both Playwright workers open an iframe, particularly on Windows.
    request_queue_size = 128


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=8767)
    parser.add_argument("--directory", type=Path, default=Path(__file__).resolve().parent / "dist")
    args = parser.parse_args()
    handler = partial(SimpleHTTPRequestHandler, directory=str(args.directory.resolve()))
    with SiteServer(("127.0.0.1", args.port), handler) as server:
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass
