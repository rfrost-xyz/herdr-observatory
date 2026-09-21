"""Loopback-only HTTP surface; no control or configuration endpoints."""
import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import gzip
import signal
from pathlib import Path
from urllib.parse import urlsplit, parse_qs

from .core import Observatory
from .effects import TextEffects, lines_for
import time

WEB = Path(__file__).resolve().parent.parent / 'web'


def handler(observatory):
    effects = TextEffects()
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            allowed = {f'localhost:{self.server.server_port}', f'127.0.0.1:{self.server.server_port}'}
            origin = self.headers.get('Origin')
            if self.headers.get('Host') not in allowed or (origin and origin not in {'http://' + h for h in allowed}):
                self.send_error(403)
                return
            path = urlsplit(self.path).path
            if path in ('/api/state','/api/text-frames'):
                query = parse_qs(urlsplit(self.path).query)
                previous = query.get('previous', [''])[0]
                category = query.get('category', ['all'])[0]
                try:
                    page = int(query.get('page', ['0'])[0])
                    hold = int(query.get('hold', ['10'])[0])
                    if not 0 <= hold <= 300 or not 0 <= page <= 10000 or category not in ('all','work','personal') or previous not in ('','decrypt','vhstape','crumble'): raise ValueError()
                except ValueError:
                    self.send_error(400); return
                state=observatory.snapshot()
                if path == '/api/state':
                    state['terminal_text']=lines_for(state,time.time(),page,category,hold)[1]
                    content=json.dumps(state,allow_nan=False).encode()
                else:
                    content=gzip.compress(json.dumps(effects.snapshot(state,previous,page,category,hold),allow_nan=False).encode(),compresslevel=1)
                kind='application/json'
            elif path in ('/', '/app.js', '/style.css'):
                name = {'/': 'index.html', '/app.js': 'app.js', '/style.css': 'style.css'}[path]
                content = (WEB / name).read_bytes()
                kind = {'/': 'text/html', '/app.js': 'text/javascript', '/style.css': 'text/css'}[path]
            else:
                self.send_error(404)
                return
            self.send_response(200)
            self.send_header('Content-Type', kind + '; charset=utf-8')
            if path == '/api/text-frames': self.send_header('Content-Encoding', 'gzip')
            self.send_header('Content-Length', str(len(content)))
            self.send_header('Cache-Control', 'no-store')
            self.send_header('X-Content-Type-Options', 'nosniff')
            self.send_header('Content-Security-Policy', "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'; form-action 'none'")
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
