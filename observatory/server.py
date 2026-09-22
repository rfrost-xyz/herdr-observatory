"""Loopback-only HTTP surface; no control or configuration endpoints."""
import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import signal
from pathlib import Path
from urllib.parse import urlsplit, parse_qs

from .core import Observatory
from .effects import lines_for
import time

WEB = Path(__file__).resolve().parent.parent / 'web'

ASSETS = {'/': ('index.html','text/html'), '/app.js': ('app.js','text/javascript'),
          '/style.css': ('style.css','text/css'), '/effects.mjs': ('effects.mjs','text/javascript'),
          '/fonts/JetBrainsMonoNerdFont-Regular.ttf': ('fonts/JetBrainsMonoNerdFont-Regular.ttf','font/ttf'),
          '/vendor/engine.mjs': ('vendor/engine.mjs','text/javascript'),
          '/vendor/effects.wasm': ('vendor/effects.wasm','application/wasm')}


def handler(observatory):
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            allowed = {f'localhost:{self.server.server_port}', f'127.0.0.1:{self.server.server_port}'}
            origin = self.headers.get('Origin')
            if self.headers.get('Host') not in allowed or (origin and origin not in {'http://' + h for h in allowed}):
                self.send_error(403)
                return
            path = urlsplit(self.path).path
            if path == '/api/state':
                query = parse_qs(urlsplit(self.path).query)
                category = query.get('category', ['all'])[0]
                try:
                    page = int(query.get('page', ['0'])[0])
                    hold = int(query.get('hold', ['120'])[0])
                    if not 0 <= hold <= 300 or not 0 <= page <= 10000 or category not in ('all','work','personal'): raise ValueError()
                except ValueError:
                    self.send_error(400); return
                state=observatory.snapshot()
                state['terminal_text']=lines_for(state,time.time(),page,category,hold)[1]
                content=json.dumps(state,allow_nan=False).encode()
                kind='application/json'
            elif path in ASSETS:
                name,kind = ASSETS[path]
                content = (WEB / name).read_bytes()
            else:
                self.send_error(404)
                return
            self.send_response(200)
            self.send_header('Content-Type', kind if kind in ('application/wasm','font/ttf') else kind + '; charset=utf-8')
            self.send_header('Content-Length', str(len(content)))
            self.send_header('Cache-Control', 'no-store')
            self.send_header('X-Content-Type-Options', 'nosniff')
            self.send_header('Content-Security-Policy', "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'; form-action 'none'")
            self.end_headers()
            self.wfile.write(content)

        def log_message(self, *_args):
            pass
    return Handler


def main():
    parser = argparse.ArgumentParser(description='Passive Herdr fleet dashboard')
    parser.add_argument('--config', default='config.local.json')
    parser.add_argument('--profile', choices=['personal', 'work'], default='work')
    parser.add_argument('--port', type=int, default=8789)
    args = parser.parse_args()
    if not 1 <= args.port <= 65535:
        parser.error('Port must be between 1 and 65535')
    try:
        config = json.loads(Path(args.config).read_text())
        observatory = Observatory(config, args.profile)
    except (OSError, ValueError, TypeError, AttributeError) as error:
        parser.error(str(error))
    server = ThreadingHTTPServer(('127.0.0.1', args.port), handler(observatory))
    def terminate(_signum, _frame):
        raise KeyboardInterrupt

    signal.signal(signal.SIGTERM, terminate)
    observatory.start()
    print(f'Herdr Observatory · {args.profile} · http://127.0.0.1:{args.port}', flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
        observatory.close()


if __name__ == '__main__':
    main()
