import http.client
from http.server import ThreadingHTTPServer
import json
import threading
import unittest

from observatory.core import Observatory
from observatory.server import handler
from test_core import HOST, RAW


class ServerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.app = Observatory({'hosts': [HOST]}, 'work', lambda h: RAW)
        cls.app.poll(HOST)
        cls.server = ThreadingHTTPServer(('127.0.0.1', 0), handler(cls.app))
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown(); cls.server.server_close(); cls.thread.join()

    def request(self, path, headers=None, method='GET'):
        conn = http.client.HTTPConnection('127.0.0.1', self.server.server_port)
        conn.request(method, path, headers=headers or {})
        result = conn.getresponse(); body = result.read()
        status, response_headers = result.status, dict(result.getheaders())
        conn.close()
        return status, body, response_headers

    def test_query_cannot_unlock_personal(self):
        status, body, headers = self.request('/api/state?profile=personal')
        self.assertEqual(status, 200)
        self.assertEqual(json.loads(body)['profile'], 'work')
        self.assertNotIn(b'PRIVATE', body)
        self.assertEqual(headers['Cache-Control'], 'no-store')

    def test_host_origin_and_mutations_denied(self):
        for headers in ({'Host': 'evil.test'}, {'Origin': 'https://evil.test'}, {'Origin': 'null'}):
            self.assertEqual(self.request('/api/state', headers)[0], 403)
        self.assertEqual(self.request('/api/state', method='POST')[0], 501)
        self.assertEqual(self.request('/../config.local.json')[0], 404)

    def test_assets_and_security_headers(self):
        for path in ('/', '/style.css', '/app.js', '/background.mjs'):
            status, body, headers = self.request(path)
            self.assertEqual(status, 200)
            self.assertGreater(len(body), 100)
            self.assertIn("frame-ancestors 'none'", headers['Content-Security-Policy'])
            self.assertNotIn('Access-Control-Allow-Origin', headers)

    def test_browser_assets_and_retired_endpoint(self):
        status,body,headers=self.request('/vendor/effects.wasm')
        self.assertEqual(status,200)
        self.assertTrue(body.startswith(b'\x00asm'))
        self.assertEqual(headers['Content-Type'],'application/wasm')
        self.assertIn("'wasm-unsafe-eval'",headers['Content-Security-Policy'])
        self.assertNotIn("'unsafe-eval'",headers['Content-Security-Policy'])
        status,font,headers=self.request('/fonts/JetBrainsMonoNerdFont-Regular.ttf')
        self.assertEqual(status,200)
        self.assertTrue(font.startswith(b'\x00\x01\x00\x00'))
        self.assertEqual(headers['Content-Type'],'font/ttf')
        self.assertEqual(self.request('/fonts/OFL.txt')[0],404)
        for path in ('/effects.mjs','/vendor/engine.mjs'):
            self.assertEqual(self.request(path)[0],200)
        for path in ('/api/text-frames','/vendor/../config.local.json','/vendor/manifest.json'):
            self.assertEqual(self.request(path)[0],404)
        self.assertEqual(self.request('/vendor/effects.wasm',{'Origin':'https://evil.test'})[0],403)

    def test_terminal_text_preserves_work_boundary(self):
        status,body,_=self.request('/api/state?profile=personal')
        self.assertEqual(status,200)
        self.assertNotIn(b'PRIVATE',body)
        self.assertIn('terminal_text',json.loads(body))

    def test_music_disabled_and_origin_boundary(self):
        status, body, _ = self.request('/api/music')
        self.assertEqual(status, 200)
        self.assertFalse(json.loads(body)['available'])
        self.assertEqual(self.request('/api/music', {'Origin': 'https://evil.test'})[0], 403)
        self.assertEqual(self.request('/api/music', method='POST')[0], 501)
