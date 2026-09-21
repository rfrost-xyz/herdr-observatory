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
        for path in ('/', '/style.css', '/app.js'):
            status, body, headers = self.request(path)
            self.assertEqual(status, 200)
            self.assertGreater(len(body), 100)
            self.assertIn("frame-ancestors 'none'", headers['Content-Security-Policy'])
            self.assertNotIn('Access-Control-Allow-Origin', headers)
